use serde::{Deserialize, Serialize};
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

// todo();
pub struct CommitRepoRequest {}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct GetReopDiffRequest {
    #[validate(required(message = "Repository name is required"))]
    pub repo_name: Option<String>,

    #[validate(required(message = "Commit ID is required"))]
    pub commit_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct GetRepoFilesRequest {
    #[validate(required(message = "Repository name is required"))]
    pub repo_name: Option<String>,

    // 要浏览的目录路径，默认为根目录
    pub path: Option<String>,

    // 分支名称，默认使用当前分支
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct GetFileContentRequest {
    #[validate(required(message = "Repository name is required"))]
    pub repo_name: Option<String>,

    #[validate(required(message = "File path is required"))]
    pub file_path: Option<String>,

    // 可选的分支名称
    pub branch: Option<String>,
}
