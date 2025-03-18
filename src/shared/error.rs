use axum::{http::StatusCode, response::IntoResponse};
use validator::ValidationErrors;

#[derive(Debug)]
pub enum AppError {
    Validation(ValidationErrors),
    Unauthorized(String),
    InternalServerError(String),
    BadRequest(String),
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Validation(err) => (
                StatusCode::BAD_REQUEST,
                format!("Validation error: {:?}", err),
            ),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.to_string()),
            Self::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string()),
            Self::BadRequest(msg) => {
                // 处理自定义的错误消息
                (StatusCode::BAD_REQUEST, msg)
            }
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
        }
        .into_response()
    }
}
