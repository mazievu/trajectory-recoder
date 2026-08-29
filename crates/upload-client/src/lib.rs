//! Resumable HTTP chunked uploader client with backoff and throttling.

pub mod client;

pub use client::{
    CompleteSessionResponse, HeartbeatRequest, InitiateSessionRequest, InitiateSessionResponse,
    RegisterMachineRequest, RegisterMachineResponse, SessionStatusResponse, UploadClient,
    UploadClientConfig, UploadError,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_client_builder() {
        let client = UploadClient::new("http://127.0.0.1:8080");
        assert_eq!(client.server_url(), "http://127.0.0.1:8080");
    }
}
