use crate::gitmodule::CommitInfo;
use crate::services::service;
use crate::shared::error::AppError;
use crate::shared::response::ApiResponse;
use crate::vos::ReposVo;
use crate::vos::userdata::UserData;
use crate::{dtos::request, shared::jwt::Claims};
use axum::extract::Query;
use axum::{Extension, Json, extract::State, http::StatusCode};
use validator::{Validate, ValidationErrors};

#[axum::debug_handler]
pub async fn register(
    State(service): State<service::AppState>,
    Json(payload): Json<request::RegisterRequest>,
) -> Result<ApiResponse<()>, AppError> {
    println!("Registering user: {}", payload.username);
    service.auth_service.register(payload).await?;

    Ok(ApiResponse::success("Registration successful"))
}

#[axum::debug_handler]
pub async fn login(
    State(service): State<service::AppState>,
    Json(payload): Json<request::LoginRequest>,
) -> Result<ApiResponse<String>, AppError> {
    let token = service.auth_service.login(payload).await?;

    Ok(ApiResponse::success_data(token))
}

#[axum::debug_handler]
pub async fn get_user_data(
    Extension(claims): Extension<Claims>,
    State(service): State<service::AppState>,
) -> Result<ApiResponse<UserData>, AppError> {
    println!("get user data handler");
    // 从 JWT claims 中获取用户 ID
    let user_id = claims.sub;
    println!("claims.sub={}", user_id);
    // 获取用户数据
    let user_data = service.auth_service.get_user_data(user_id).await?;

    Ok(ApiResponse::success_data(user_data))
}

#[axum::debug_handler]
pub async fn get_repo_commit_histories(
    Extension(claims): Extension<Claims>,
    State(service): State<service::AppState>,
    // Json(payload): Json<request::RepoRequest>,
    Query(params): Query<request::RepoRequest>,
) -> Result<ApiResponse<Vec<CommitInfo>>, AppError> {
    println!("Get repository commit data handler");

    let user_id = claims.sub;
    println!("User ID: {}", user_id);

    // 获取仓库提交历史
    let limit = params.limit.unwrap_or(50); // 默认获取50条提交记录
    // let commit_history = service.get_repo_history(&payload.repo_name, limit).await?;
    if let Some(repo_name) = params.repo_name {
        let commit_history = service
            .git_service
            .get_repo_commit_histories(&user_id, &repo_name, limit)
            .await?;

        Ok(ApiResponse::success_data(commit_history))
    } else {
        Err(AppError::NotFound(format!("repo_name is required")))
    }
}

#[axum::debug_handler]
pub async fn get_repos(
    Extension(claims): Extension<Claims>,
    State(service): State<service::AppState>,
    // Json(payload): Json<request::RepoRequest>,
) -> Result<ApiResponse<Vec<ReposVo>>, AppError> {
    let user_id = claims.sub;
    println!("User ID: {}", user_id);

    let repos_data = service
        .git_service
        .get_repos_data_for_users(&user_id)
        .await?;

    Ok(ApiResponse::success_data(repos_data))
}

#[axum::debug_handler]
pub async fn clone_repo_for_user(
    Extension(claims): Extension<Claims>,
    State(service): State<service::AppState>,
    Json(payload): Json<request::CloneRepoRequest>,
) -> Result<ApiResponse<()>, AppError> {
    let user_id = claims.sub;

    let repo_url = match &payload.repo_url {
        Some(url) if url.ends_with(".git") => url,
        _ => return Err(AppError::BadRequest("Invalid Git repository URL".into())),
    };

    let repo_name = match &payload.repo_name {
        Some(name) if !name.trim().is_empty() => name,
        _ => return Err(AppError::BadRequest("Repository name is required".into())),
    };

    println!("Cloning repository {} for user {}", repo_name, user_id);

    // 调用服务层进行仓库克隆
    let repo_path = service
        .git_service
        .clone_repo_for_user(&user_id, repo_url, repo_name)
        .await?;
    println!("handler get cloned path {}", repo_path);
    Ok(ApiResponse::success("success cloned"))
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
