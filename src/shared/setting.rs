use lazy_static::lazy_static;
use serde::Deserialize;
use std::sync::RwLock;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub jwt: JwtConfig,
    pub git_path: GitPathConfig,
}

#[derive(Debug, Deserialize)]
pub struct JwtConfig {
    #[serde(deserialize_with = "deserialize")]
    pub jwt_secret: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct GitPathConfig {
    #[serde(deserialize_with = "deserialize")]
    pub repositories_path: Vec<u8>,
}

fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    Ok(s.into_bytes())
}

pub fn load_config() -> Settings {
    config::Config::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap()
}
pub fn get_config() -> std::sync::RwLockReadGuard<'static, Settings> {
    CONFIG.read().unwrap()
}
lazy_static! {
    static ref CONFIG: RwLock<Settings> = RwLock::new(load_config());
}
