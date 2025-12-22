use std::string::FromUtf8Error;

use rmcp::service::ClientInitializeError;

#[derive(Debug, thiserror::Error)]
pub enum TmcpError {
    /// Error from reqwest HTTP client
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    /// Error from JSON serialization/deserialization
    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    /// Error from invalid HTTP header value
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error("Vid error: {0}")]
    VidError(#[from] tsp_sdk::vid::VidError),
    #[error("Tsp error: {0}")]
    TspError(#[from] tsp_sdk::Error),
    #[error("TSP CESR error: {0}")]
    TspCesrError(#[from] tsp_sdk::cesr::error::DecodeError),
    #[error("Base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Tmcp error: {0}")]
    TmcpError(String),
    #[error("UTF-8 error: {0}")]
    StringError(#[from] FromUtf8Error),
    #[error("Client initialization error: {0}")]
    RmcpClientInitializeError(#[from] ClientInitializeError),
    #[error("Client service error: {0}")]
    RmcpClientServiceError(#[from] rmcp::service::ServiceError),
}
