use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(required(message = "Identity field is required"))]
    pub identity: Option<String>,
    // pub identity : String,
    #[validate(required(message = "Password is required"))]
    pub password: Option<String>,
}
/*
required 验证器专门用于 Option<T> 类型
length 验证器用于 String 类型
*/

#[derive(Debug, Deserialize, Validate)]
pub struct RepoRequest {
    #[validate(required(message = "Repository name is required"))]
    pub repo_name: Option<String>,

    // 可选参数：限制获取的提交记录数量
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CloneRepoRequest {
    pub repo_url: Option<String>,
    pub repo_name: Option<String>,
}
