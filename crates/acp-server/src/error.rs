use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::types::{AcError, AcErrorCode};

#[derive(Debug, thiserror::Error)]
pub enum AcServerError {
    #[error("server error: {0}")]
    Internal(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("not found: {0}")]
    NotFound(String),
}

impl AcServerError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }

    fn error_code(&self) -> AcErrorCode {
        match self {
            Self::Internal(_) => AcErrorCode::ServerError,
            Self::InvalidInput(_) => AcErrorCode::InvalidInput,
            Self::NotFound(_) => AcErrorCode::NotFound,
        }
    }
}

impl IntoResponse for AcServerError {
    fn into_response(self) -> Response {
        let body = AcError {
            code: self.error_code(),
            message: self.to_string(),
            data: None,
        };
        (self.status_code(), axum::Json(body)).into_response()
    }
}

impl From<&str> for AcServerError {
    fn from(s: &str) -> Self {
        Self::Internal(s.to_string())
    }
}
