use axum::body::Body;
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(message: impl Into<String>) -> ApiResponse<T> {
        ApiResponse {
            code: 0,
            message: message.into(),
            data: None,
        }
    }
    pub fn success_data(data: T) -> ApiResponse<T> {
        ApiResponse {
            code: 0,
            message: "success".to_string(),
            data: Some(data),
        }
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        // 构造 HTTP 响应
        let status = if self.code == 0 {
            StatusCode::OK // 成功时返回 200 OK
        } else {
            StatusCode::BAD_REQUEST // 错误时返回 400 Bad Request
        };

        // 构造 JSON 响应体
        let body = Json(serde_json::json!({
            "code": self.code,
            "message": self.message,
            "data": self.data,
        }));

        // 返回完整的 HTTP 响应
        (status, body).into_response()
    }
}
