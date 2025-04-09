use super::{
    error::AppError,
    jwt::{Claims, validate_token},
};
use crate::shared::setting;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
// const JWT_SECRET: &[u8] = b"your-secret-key";
// const JWT_SECRET: &[u8] = b"your-secret-key";
// #[derive(Debug, Serialize, Deserialize, Clone)]
// // pub struct Claims {
// //     pub sub: String, // 用户ID
// //     pub exp: usize,  // 过期时间
// // }

pub async fn auth_middleware(
    // State(state): State<()>, // You likely need a real state type here
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // 从请求头中获取token
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| {
            if auth_str.starts_with("Bearer ") {
                Some(auth_str[7..].to_string())
            } else {
                None
            }
        })
        .ok_or(AppError::Unauthorized("can not get token".to_string()))?;
    // 验证token
    let claims = validate_token(&token)
        .map_err(|_| AppError::Unauthorized("token validate failed".to_string()))?;

    // 将用户信息注入请求扩展中
    req.extensions_mut().insert(claims);

    // 继续处理请求
    Ok(next.run(req).await)
}
