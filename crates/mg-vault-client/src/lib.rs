//! Typed local IPC client and protocol for the index service.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Request {
    pub protocol_version: u16,
    pub request_id: String,
    pub operation: Operation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Operation {
    Hello {
        client_versions: Vec<u16>,
        capabilities: Vec<String>,
    },
    ServiceStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Response {
    pub protocol_version: u16,
    pub request_id: String,
    pub ok: bool,
    pub service_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ProtocolError>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseData {
    Hello { selected_version: u16 },
    ServiceStatus(ServiceStatus),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServiceStatus {
    pub state: String,
    pub registered_vaults: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("IPC I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("IPC JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IPC frame length {length} exceeds limit {limit}")]
    FrameTooLarge { length: usize, limit: usize },
    #[error("IPC protocol error {code}: {message}")]
    Protocol { code: String, message: String },
    #[error("invalid IPC response: {0}")]
    InvalidResponse(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;

pub struct Client {
    stream: UnixStream,
    negotiated_version: u16,
    service_version: String,
}

impl Client {
    /// Connect and negotiate the highest mutually supported protocol version.
    ///
    /// # Errors
    /// Returns an I/O, framing, serialization, or typed protocol error.
    pub fn connect(
        socket: &Path,
        supported_versions: &[u16],
        request_id: impl Into<String>,
    ) -> Result<Self> {
        let mut stream = UnixStream::connect(socket)?;
        let request_id = request_id.into();
        write_frame(
            &mut stream,
            &Request {
                protocol_version: 0,
                request_id: request_id.clone(),
                operation: Operation::Hello {
                    client_versions: supported_versions.to_vec(),
                    capabilities: Vec::new(),
                },
            },
        )?;
        let response: Response = read_frame(&mut stream)?;
        validate_response_shape(&response, PROTOCOL_VERSION)?;
        validate_response_id(&response, &request_id)?;
        if let Some(error) = response.error {
            return Err(ClientError::Protocol {
                code: error.code,
                message: error.message,
            });
        }
        let Some(ResponseData::Hello { selected_version }) = response.data else {
            return Err(ClientError::InvalidResponse(
                "hello response did not contain negotiation data".to_owned(),
            ));
        };
        if !supported_versions.contains(&selected_version) {
            return Err(ClientError::InvalidResponse(format!(
                "server selected unsupported protocol {selected_version}"
            )));
        }
        Ok(Self {
            stream,
            negotiated_version: selected_version,
            service_version: response.service_version,
        })
    }

    #[must_use]
    pub const fn protocol_version(&self) -> u16 {
        self.negotiated_version
    }

    #[must_use]
    pub fn service_version(&self) -> &str {
        &self.service_version
    }

    /// Request current process-level service health.
    ///
    /// # Errors
    /// Returns an I/O, framing, serialization, typed protocol, or response-shape error.
    pub fn service_status(&mut self, request_id: impl Into<String>) -> Result<ServiceStatus> {
        let request_id = request_id.into();
        write_frame(
            &mut self.stream,
            &Request {
                protocol_version: self.negotiated_version,
                request_id: request_id.clone(),
                operation: Operation::ServiceStatus,
            },
        )?;
        let response: Response = read_frame(&mut self.stream)?;
        validate_response_shape(&response, self.negotiated_version)?;
        validate_response_id(&response, &request_id)?;
        if let Some(error) = response.error {
            return Err(ClientError::Protocol {
                code: error.code,
                message: error.message,
            });
        }
        match response.data {
            Some(ResponseData::ServiceStatus(status)) => Ok(status),
            _ => Err(ClientError::InvalidResponse(
                "status response did not contain service status".to_owned(),
            )),
        }
    }
}

fn validate_response_id(response: &Response, expected: &str) -> Result<()> {
    if response.request_id == expected {
        Ok(())
    } else {
        Err(ClientError::InvalidResponse(format!(
            "request ID mismatch: expected {expected:?}, received {:?}",
            response.request_id
        )))
    }
}

fn validate_response_shape(response: &Response, expected_version: u16) -> Result<()> {
    if response.protocol_version != expected_version {
        return Err(ClientError::InvalidResponse(format!(
            "protocol version mismatch: expected {expected_version}, received {}",
            response.protocol_version
        )));
    }
    if response.service_version.is_empty()
        || response.ok == response.error.is_some()
        || (response.ok && response.data.is_none())
        || (!response.ok && response.data.is_some())
    {
        return Err(ClientError::InvalidResponse(
            "response status and payload are inconsistent".to_owned(),
        ));
    }
    Ok(())
}

/// Write one big-endian length-prefixed JSON frame.
///
/// # Errors
/// Returns an I/O, serialization, or frame-size error.
pub fn write_frame<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(ClientError::FrameTooLarge {
            length: bytes.len(),
            limit: MAX_FRAME_BYTES,
        });
    }
    let length = u32::try_from(bytes.len()).map_err(|_| ClientError::FrameTooLarge {
        length: bytes.len(),
        limit: MAX_FRAME_BYTES,
    })?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

/// Read one bounded big-endian length-prefixed JSON frame.
///
/// # Errors
/// Returns before allocating the payload when the declared frame exceeds the limit.
pub fn read_frame<R: Read, T: DeserializeOwned>(reader: &mut R) -> Result<T> {
    let mut prefix = [0_u8; 4];
    reader.read_exact(&mut prefix)?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(ClientError::FrameTooLarge {
            length,
            limit: MAX_FRAME_BYTES,
        });
    }
    let mut bytes = vec![0_u8; length];
    reader.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_round_trips_and_preserves_request_id() {
        let request = Request {
            protocol_version: 1,
            request_id: "req-7".to_owned(),
            operation: Operation::ServiceStatus,
        };
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &request).unwrap();
        let decoded: Request = read_frame(&mut bytes.as_slice()).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn oversized_declared_frame_is_rejected_before_payload_read() {
        let length = u32::try_from(MAX_FRAME_BYTES + 1).unwrap();
        let error = read_frame::<_, Request>(&mut length.to_be_bytes().as_slice()).unwrap_err();
        assert!(matches!(error, ClientError::FrameTooLarge { .. }));
    }
}
