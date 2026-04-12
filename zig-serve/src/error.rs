use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use zig_core::error::ZigError;

/// API error type that wraps `ZigError` and maps to HTTP status codes.
pub struct ServeError {
    status: StatusCode,
    code: String,
    message: String,
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

impl ServeError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found".into(),
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized".into(),
            message: "invalid or missing bearer token".into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_request".into(),
            message: message.into(),
        }
    }
}

impl From<ZigError> for ServeError {
    fn from(err: ZigError) -> Self {
        let message = err.to_string();
        match &err {
            ZigError::Parse(_) => Self {
                status: StatusCode::BAD_REQUEST,
                code: "parse_error".into(),
                message,
            },
            ZigError::Validation(_) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "validation_error".into(),
                message,
            },
            ZigError::Io(msg) if msg.contains("not found") => Self {
                status: StatusCode::NOT_FOUND,
                code: "not_found".into(),
                message,
            },
            ZigError::Io(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "io_error".into(),
                message,
            },
            ZigError::Serialize(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "serialize_error".into(),
                message,
            },
            ZigError::Zag(_) => Self {
                status: StatusCode::BAD_GATEWAY,
                code: "zag_error".into(),
                message,
            },
            ZigError::Execution(_) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: "execution_error".into(),
                message,
            },
        }
    }
}

impl IntoResponse for ServeError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error: ErrorDetail {
                code: self.code,
                message: self.message,
            },
        };
        (self.status, axum::Json(body)).into_response()
    }
}
