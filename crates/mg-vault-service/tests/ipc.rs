use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::thread;

use mg_vault_client::{Client, ClientError, MAX_FRAME_BYTES};
use mg_vault_core::VaultRegistry;
use mg_vault_service::{Server, ServiceError, authorize_uid};

#[test]
fn real_server_negotiates_and_reports_health_over_owner_only_socket() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = temp.path().join("runtime/mg-vault");
    let socket = runtime.join("indexd.sock");
    let registry = temp.path().join("config/vaults.json");
    let server = Server::bind(&socket, &registry).unwrap();

    assert_eq!(
        fs::metadata(&runtime).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let handle = thread::spawn(move || server.serve_one().unwrap());
    let mut client = Client::connect(&socket, &[1], "hello-1").unwrap();
    assert_eq!(client.protocol_version(), 1);
    assert!(!client.service_version().is_empty());
    let status = client.service_status("status-1").unwrap();
    assert_eq!(status.state, "available");
    assert_eq!(status.registered_vaults, 0);
    handle.join().unwrap();
}

#[test]
fn incompatible_protocol_fails_closed_before_status() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("runtime/indexd.sock");
    let registry = temp.path().join("config/vaults.json");
    let server = Server::bind(&socket, &registry).unwrap();
    let handle = thread::spawn(move || server.serve_one().unwrap());

    let Err(error) = Client::connect(&socket, &[99], "hello-old") else {
        panic!("unsupported protocol unexpectedly negotiated");
    };
    assert!(matches!(
        error,
        ClientError::Protocol { ref code, .. } if code == "protocol_incompatible"
    ));
    handle.join().unwrap();
}

#[test]
fn peer_uid_mismatch_is_rejected() {
    let error = authorize_uid(1000, 1001).unwrap_err();
    assert!(error.to_string().contains("does not match"));
}

#[test]
fn endpoint_inside_a_registered_vault_is_rejected_without_creating_a_socket() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    fs::create_dir(&vault).unwrap();
    let registry_path = temp.path().join("config/vaults.json");
    let mut registry = VaultRegistry::load(&registry_path).unwrap();
    registry.register("notes", &vault).unwrap();
    let socket = vault.join("indexd.sock");

    let Err(error) = Server::bind(&socket, &registry_path) else {
        panic!("service endpoint inside a vault was accepted");
    };
    assert!(matches!(error, ServiceError::UnsafeRuntime { .. }));
    assert!(!socket.exists());
}

#[test]
fn oversized_frame_is_rejected_before_payload_read() {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("runtime/indexd.sock");
    let registry = temp.path().join("config/vaults.json");
    let server = Server::bind(&socket, &registry).unwrap();
    let handle = thread::spawn(move || server.serve_one());

    let mut stream = UnixStream::connect(&socket).unwrap();
    let declared = u32::try_from(MAX_FRAME_BYTES + 1).unwrap();
    stream.write_all(&declared.to_be_bytes()).unwrap();
    let error = handle.join().unwrap().unwrap_err();
    assert!(matches!(
        error,
        ServiceError::Protocol(ClientError::FrameTooLarge { .. })
    ));
}
