//! API error types and JSON error responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ports::{ArtifactStoreError, EventBusError};
use serde::Serialize;

/// API error type with structured JSON responses.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Database connection or query error.
    #[error("database error: {0}")]
    DatabaseError(String),

    /// Event bus operation failed.
    #[error("event bus error: {0}")]
    EventBusError(#[from] EventBusError),

    /// Artifact store operation failed.
    #[error("artifact store error: {0}")]
    ArtifactStoreError(#[from] ArtifactStoreError),

    /// Invalid request payload or parameters.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Requested resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Internal server error.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Structured error response for JSON API.
#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

/// Error detail with code and message.
#[derive(Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database_error"),
            ApiError::EventBusError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "event_bus_error"),
            ApiError::ArtifactStoreError(e) => match e {
                ArtifactStoreError::NotFound(_) => (StatusCode::NOT_FOUND, "artifact_not_found"),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "artifact_store_error"),
            },
            ApiError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let message = self.to_string();

        let body = Json(ErrorResponse {
            error: ErrorDetail {
                code: code.to_string(),
                message,
            },
        });

        (status, body).into_response()
    }
}

impl From<db::DbError> for ApiError {
    fn from(err: db::DbError) -> Self {
        ApiError::DatabaseError(err.to_string())
    }
}
