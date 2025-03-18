use crate::services::service;
use crate::shared::error::AppError;
use crate::shared::response::ApiResponse;
use crate::vos::userdata::UserData;
use crate::{dtos::request, shared::jwt::Claims};
use axum::{Extension, Json, extract::State, http::StatusCode};
use validator::{Validate, ValidationErrors};

#[axum::debug_handler]
pub async fn register(
    State(service): State<service::AuthService>,
    Json(payload): Json<request::RegisterRequest>,
) -> Result<ApiResponse<()>, AppError> {
    println!("Registering user: {}", payload.username);
    service.register(payload).await?;

    Ok(ApiResponse::success("Registration successful"))
}

#[axum::debug_handler]
pub async fn login(
    State(service): State<service::AuthService>,
    Json(payload): Json<request::LoginRequest>,
) -> Result<ApiResponse<String>, AppError> {
    let token = service.login(payload).await?;

    Ok(ApiResponse::success_data(token))
}

#[axum::debug_handler]
pub async fn get_user_data(
    Extension(claims): Extension<Claims>,
    State(service): State<service::AuthService>,
) -> Result<ApiResponse<UserData>, AppError> {
    println!("get user data handler");
    // 从 JWT claims 中获取用户 ID
    let user_id = claims.sub;
    println!("claims.sub={}", user_id);
    // 获取用户数据
    let user_data = service.get_user_data(user_id).await?;

    Ok(ApiResponse::success_data(user_data))
}

// 将验证错误转换为字符串
fn format_validation_errors(errors: ValidationErrors) -> String {
    errors
        .field_errors()
        .iter()
        .map(|(field, errors)| {
            format!(
                "{}: {}",
                field,
                errors
                    .iter()
                    .map(|e| e
                        .message
                        .as_ref()
                        .map_or("Unknown error", |v| v)
                        .to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
