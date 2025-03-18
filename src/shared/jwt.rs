use crate::shared::setting;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, DecodingKey, Validation};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // 用户ID
    pub exp: usize,  // 过期时间
    pub iat: usize,  // 签发时间
}
// static JWT_SECRET: &[u8] = b"your_secret_key";
pub fn generate_token(user_id: String) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let expires_at = now + Duration::hours(24);

    let claims = Claims {
        sub: user_id,
        iat: now.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };
    let setting = setting::load_config();
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&setting.jwt.jwt_secret),
    )
}

pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let setting = setting::load_config();
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(&setting.jwt.jwt_secret),
        &Validation::default(),
    )
    .map(|data| data.claims)
}
