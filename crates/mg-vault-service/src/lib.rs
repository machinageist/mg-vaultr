//! Owner-only local IPC service boundary.

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use mg_vault_client::{
    Operation, PROTOCOL_VERSION, ProtocolError, Request, Response, ResponseData, ServiceStatus,
    read_frame, write_frame,
};
use mg_vault_core::VaultRegistry;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("service I/O error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("service protocol error: {0}")]
    Protocol(#[from] mg_vault_client::ClientError),
    #[error("service registry error: {0}")]
    Registry(#[from] mg_vault_core::Error),
    #[error("unsafe service runtime path {}: {reason}", path.display())]
    UnsafeRuntime { path: PathBuf, reason: String },
    #[error("service peer UID {peer} does not match owner UID {owner}")]
    PeerUidMismatch { owner: u32, peer: u32 },
}

pub type Result<T> = std::result::Result<T, ServiceError>;

pub struct Server {
    listener: UnixListener,
    socket_path: PathBuf,
    registry_path: PathBuf,
    owner_uid: u32,
}

impl Server {
    /// Bind an owner-only service endpoint.
    ///
    /// # Errors
    /// Refuses unsafe runtime directories and existing socket paths.
    pub fn bind(socket_path: &Path, registry_path: &Path) -> Result<Self> {
        let registry = VaultRegistry::load(registry_path)?;
        if registry
            .list()
            .iter()
            .any(|vault| socket_path.starts_with(&vault.path))
        {
            return Err(ServiceError::UnsafeRuntime {
                path: socket_path.to_path_buf(),
                reason: "service endpoint must be outside every vault".to_owned(),
            });
        }
        let parent = socket_path
            .parent()
            .ok_or_else(|| ServiceError::UnsafeRuntime {
                path: socket_path.to_path_buf(),
                reason: "socket path has no parent".to_owned(),
            })?;
        let owner_uid = rustix::process::geteuid().as_raw();
        ensure_private_directory(parent, owner_uid)?;
        if fs::symlink_metadata(socket_path).is_ok() {
            return Err(ServiceError::UnsafeRuntime {
                path: socket_path.to_path_buf(),
                reason: "endpoint already exists; refusing to replace it".to_owned(),
            });
        }
        let listener = UnixListener::bind(socket_path).map_err(|source| io(socket_path, source))?;
        fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))
            .map_err(|source| io(socket_path, source))?;
        Ok(Self {
            listener,
            socket_path: socket_path.to_path_buf(),
            registry_path: registry_path.to_path_buf(),
            owner_uid,
        })
    }

    /// Serve connections until the process is terminated.
    ///
    /// # Errors
    /// Returns on listener, peer-credential, protocol, or registry failure.
    pub fn serve(&self) -> Result<()> {
        loop {
            let _ = self.serve_one();
        }
    }

    /// Serve one negotiated client connection.
    ///
    /// # Errors
    /// Returns on listener, authorization, protocol, or registry failure.
    pub fn serve_one(&self) -> Result<()> {
        let (mut stream, _) = self
            .listener
            .accept()
            .map_err(|source| io(&self.socket_path, source))?;
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .map_err(|source| io(&self.socket_path, source))?;
        stream
            .set_write_timeout(Some(std::time::Duration::from_secs(5)))
            .map_err(|source| io(&self.socket_path, source))?;
        self.authorize(&stream)?;
        self.handle_connection(&mut stream)
    }

    fn authorize(&self, stream: &UnixStream) -> Result<()> {
        let credentials = rustix::net::sockopt::socket_peercred(stream)
            .map_err(|source| io(&self.socket_path, source.into()))?;
        authorize_uid(self.owner_uid, credentials.uid.as_raw())
    }

    fn handle_connection(&self, stream: &mut UnixStream) -> Result<()> {
        let hello: Request = read_frame(stream)?;
        let Operation::Hello {
            client_versions, ..
        } = hello.operation
        else {
            write_protocol_error(
                stream,
                hello.request_id,
                "handshake_required",
                "first operation must be hello",
            )?;
            return Ok(());
        };
        if !client_versions.contains(&PROTOCOL_VERSION) {
            write_protocol_error(
                stream,
                hello.request_id,
                "protocol_incompatible",
                "no supported protocol version overlap",
            )?;
            return Ok(());
        }
        write_frame(
            stream,
            &Response {
                protocol_version: PROTOCOL_VERSION,
                request_id: hello.request_id,
                ok: true,
                service_version: env!("CARGO_PKG_VERSION").to_owned(),
                data: Some(ResponseData::Hello {
                    selected_version: PROTOCOL_VERSION,
                }),
                error: None,
            },
        )?;

        let request: Request = read_frame(stream)?;
        if request.protocol_version != PROTOCOL_VERSION {
            write_protocol_error(
                stream,
                request.request_id,
                "protocol_incompatible",
                "request protocol does not match negotiated protocol",
            )?;
            return Ok(());
        }
        match request.operation {
            Operation::ServiceStatus => {
                let registered_vaults =
                    u64::try_from(VaultRegistry::load(&self.registry_path)?.list().len())
                        .unwrap_or(u64::MAX);
                write_frame(
                    stream,
                    &Response {
                        protocol_version: PROTOCOL_VERSION,
                        request_id: request.request_id,
                        ok: true,
                        service_version: env!("CARGO_PKG_VERSION").to_owned(),
                        data: Some(ResponseData::ServiceStatus(ServiceStatus {
                            state: "available".to_owned(),
                            registered_vaults,
                        })),
                        error: None,
                    },
                )?;
            }
            Operation::Hello { .. } => write_protocol_error(
                stream,
                request.request_id,
                "unexpected_hello",
                "hello is valid only as the first operation",
            )?,
        }
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

/// Enforce the same-user local IPC boundary.
///
/// # Errors
/// Returns [`ServiceError::PeerUidMismatch`] when identities differ.
pub fn authorize_uid(owner: u32, peer: u32) -> Result<()> {
    if owner == peer {
        Ok(())
    } else {
        Err(ServiceError::PeerUidMismatch { owner, peer })
    }
}

fn ensure_private_directory(path: &Path, owner_uid: u32) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => {
                return Err(ServiceError::UnsafeRuntime {
                    path: current,
                    reason: "runtime path contains a non-directory component".to_owned(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|source| io(&current, source))?;
            }
            Err(source) => return Err(io(&current, source)),
        }
    }
    let metadata = fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if !metadata.file_type().is_dir() || metadata.uid() != owner_uid {
        return Err(ServiceError::UnsafeRuntime {
            path: path.to_path_buf(),
            reason: "runtime path must be a directory owned by the service user".to_owned(),
        });
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|source| io(path, source))
}

fn write_protocol_error(
    stream: &mut UnixStream,
    request_id: String,
    code: &str,
    message: &str,
) -> Result<()> {
    write_frame(
        stream,
        &Response {
            protocol_version: PROTOCOL_VERSION,
            request_id,
            ok: false,
            service_version: env!("CARGO_PKG_VERSION").to_owned(),
            data: None,
            error: Some(ProtocolError {
                code: code.to_owned(),
                message: message.to_owned(),
            }),
        },
    )?;
    Ok(())
}

fn io(path: &Path, source: std::io::Error) -> ServiceError {
    ServiceError::Io {
        path: path.to_path_buf(),
        source,
    }
}
