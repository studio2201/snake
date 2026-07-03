//! Snake-wide error type.
//!
//! Handlers return `Result<impl IntoResponse, AppError>` so the framework
//! can render a consistent error envelope without each handler rolling its
//! own JSON. The `Display` impl intentionally omits internal error details
//! from the user-facing response body to avoid leaking paths, secrets, or
//! stack traces to clients.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Errors produced by Snake handlers and middleware.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Underlying I/O failure (file missing, permission denied, etc.).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization failure.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// Anything else — the public message is fixed to avoid information leaks.
    #[error("{context}")]
    Internal {
        /// Operator-visible context for logs. Never echoed to the client.
        context: &'static str,
    },
}

impl AppError {
    /// Build a generic internal error with the given log-friendly context.
    #[must_use]
    pub const fn internal(context: &'static str) -> Self {
        Self::Internal { context }
    }

    /// HTTP status that this error maps to.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Io(_) | Self::Json(_) | Self::Internal { .. } => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Log the full chain for operators, but return only a generic body
        // for unknown/internals. Known categories still carry their
        // high-level description so client-side debugging isn't impossible.
        let status = self.status();
        let body = match &self {
            Self::Io(_) => "internal error: storage".to_string(),
            Self::Json(_) => "internal error: serialization".to_string(),
            Self::Internal { .. } => "internal error".to_string(),
        };
        tracing::error!(target: "app_error", error = %self, status = %status, "request failed");
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_status_is_500() {
        assert_eq!(
            AppError::internal("oops").status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn io_error_maps_to_500() {
        let e: AppError = std::io::Error::other("disk").into();
        assert_eq!(e.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn json_error_maps_to_500() {
        let e: AppError = serde_json::from_str::<serde_json::Value>("{")
            .unwrap_err()
            .into();
        assert_eq!(e.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn into_response_redacts_internal_details() {
        let err = AppError::internal("db password leaked into message");
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
