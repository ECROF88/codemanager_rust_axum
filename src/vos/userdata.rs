#[derive(serde::Serialize, Debug)]
pub struct UserData {
    pub username: String,
    pub email: String,
    // 其他需要的字段
    pub avatar: Option<String>,
}
