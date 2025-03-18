use crate::shared::setting;
use chrono::{Duration, Utc};
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{DecodingKey, Validation, decode};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // 用户ID
    pub exp: u64,    // 过期时间
    pub iat: u64,    // 签发时间
}
// static JWT_SECRET: &[u8] = b"your_secret_key";
pub fn generate_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let expires_at = now + Duration::hours(24);

    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp() as u64,
        exp: expires_at.timestamp() as u64,
    };
    let setting = setting::get_config();

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&setting.jwt.jwt_secret),
    )
}

#[derive(Debug)]
pub enum TokenError {
    Expired,
    InvalidSignature,
    InvalidToken,
    TooEarly,
    Other(String),
}

// // 转换 jsonwebtoken 错误到自定义错误
impl From<jsonwebtoken::errors::Error> for TokenError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            ErrorKind::ExpiredSignature => TokenError::Expired,
            ErrorKind::InvalidSignature => TokenError::InvalidSignature,
            ErrorKind::InvalidToken => TokenError::InvalidToken,
            _ => TokenError::Other(err.to_string()),
        }
    }
}

pub fn validate_token(token: &str) -> Result<Claims, TokenError> {
    let setting = setting::get_config();

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&setting.jwt.jwt_secret),
        &Validation::default(),
    )?;

    let now = Utc::now().timestamp() as u64;
    if token_data.claims.iat > now {
        return Err(TokenError::TooEarly);
    }
    Ok(token_data.claims)
}
