use std::sync::Arc;

use crate::gitmodule::structs::{CommitDetail, CommitInfo, GitFileEntry, WebSocketMsg};
// use crate::gitmodule::{CommitInfo, structs::CommitDetail};
use crate::services::service;
use crate::shared::error::AppError;
use crate::shared::response::ApiResponse;
use crate::vos::ReposVo;
use crate::vos::userdata::UserData;
use crate::{dtos::request, shared::jwt::Claims};
use axum::extract::{Path, Query, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::{Extension, Json, extract::State, http::StatusCode};
use tokio::sync::mpsc;
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
        .clone_repo_for_user(&user_id, repo_url, repo_name, &service.ws_manager)
        .await?;
    println!("handler get cloned {}", repo_path);

    Ok(ApiResponse::success("success started!"))
}

pub async fn commit_for_user_repo(
    Extension(claims): Extension<Claims>,
    State(service): State<service::AppState>,
    Json(payload): Json<request::CloneRepoRequest>,
) -> Result<ApiResponse<()>, AppError> {
    let user_id = claims.sub;
    todo!()
}

#[axum::debug_handler]
pub async fn get_repo_commit_diff(
    Extension(claims): Extension<Claims>,
    State(service): State<service::AppState>,
    Query(params): Query<request::GetReopDiffRequest>,
) -> Result<ApiResponse<CommitDetail>, AppError> {
    let user_id = claims.sub;

    if params.commit_id.is_none() {
        return Err(AppError::BadRequest("Commit ID is required".into()));
    }

    if params.repo_name.is_none() {
        return Err(AppError::BadRequest("Repo Name is required".into()));
    }

    // let repo_name = params
    //     .repo_name
    //     .as_ref()
    //     .ok_or_else(|| AppError::BadRequest("Repository name is required".into()))?;

    // let commit_id = params
    //     .commit_id
    //     .as_ref()
    //     .ok_or_else(|| AppError::BadRequest("Commit ID is required".into()))?;
    // println!(
    //     "Getting diff for commit {} in repo {}",
    //     params.commit_id.unwrap(),
    //     params.repo_name.unwrap()
    // );

    let repo_name = params.repo_name.as_ref().unwrap();
    let commit_id = params.commit_id.as_ref().unwrap();
    let commit_diff_details = service
        .git_service
        .get_repo_commit_diff(&user_id, repo_name, commit_id)
        .await?;

    // todo!()
    Ok(ApiResponse::success_data(commit_diff_details))
}

#[axum::debug_handler]
pub async fn get_repo_files_tree(
    Extension(claims): Extension<Claims>,
    State(service): State<service::AppState>,
    Query(params): Query<request::GetRepoFilesRequest>,
) -> Result<ApiResponse<Vec<GitFileEntry>>, AppError> {
    let user_id = claims.sub;

    // 获取仓库名
    let repo_name = params
        .repo_name
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Repository name is required".into()))?;

    // 获取可选参数
    let directory_path = params.path.as_deref();
    let branch = params.branch.as_deref();

    println!(
        "Getting files tree for repo {} (path: {:?}, branch: {:?})",
        repo_name, directory_path, branch
    );

    // 调用服务层获取文件树
    let files = service
        .git_service
        .list_repository_files(&user_id, repo_name, directory_path, branch)
        .await?;

    Ok(ApiResponse::success_data(files))
}

#[axum::debug_handler]
pub async fn get_repo_file_content(
    Extension(claims): Extension<Claims>,
    State(service): State<service::AppState>,
    Query(params): Query<request::GetFileContentRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let user_id = claims.sub;
    let repo_name = params
        .repo_name
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Repository name is required".into()))?;

    let file_path = params
        .file_path
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("File path is required".into()))?;

    // 获取可选的分支名
    let branch = params.branch.as_deref();

    println!(
        "Getting file content: {}/{} (branch: {:?})",
        repo_name, file_path, branch
    );

    // 调用服务获取文件内容
    let content = service
        .git_service
        .get_file_content(&user_id, repo_name, file_path, branch)
        .await?;

    // 推断内容类型
    let content_type = infer_content_type(file_path);

    // 直接返回文件内容，而不是封装在ApiResponse中
    // 这样更适合前端直接处理文本内容
    Ok((
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, content_type)],
        content,
    ))
}

// 根据文件扩展名推断内容类型
fn infer_content_type(file_path: &str) -> &'static str {
    if let Some(ext) = file_path.split('.').last() {
        match ext.to_lowercase().as_str() {
            "html" => "text/html; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "js" => "application/javascript; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "md" => "text/markdown; charset=utf-8",
            "rs" => "text/plain; charset=utf-8",   // Rust源码
            "go" => "text/plain; charset=utf-8",   // Go源码
            "py" => "text/plain; charset=utf-8",   // Python源码
            "java" => "text/plain; charset=utf-8", // Java源码
            "c" | "cpp" | "h" => "text/plain; charset=utf-8", // C/C++源码
            "txt" => "text/plain; charset=utf-8",
            // 添加更多文件类型...
            _ => "text/plain; charset=utf-8", // 默认为纯文本
        }
    } else {
        "text/plain; charset=utf-8"
    }
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(user_id): Path<String>,
    State(service): State<service::AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, user_id, service))
}

async fn handle_socket(
    socket: axum::extract::ws::WebSocket,
    user_id: String,
    app_state: service::AppState,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};

    // 分割 WebSocket 为 sender 和 receiver
    let (mut sender, mut receiver) = socket.split();

    // 创建一个通道，用于从应用状态发送消息到 WebSocket
    let (tx, mut rx) = mpsc::unbounded_channel::<WebSocketMsg>();

    // 注册连接
    app_state
        .ws_manager
        .register_connection(&user_id, tx.clone())
        .await;

    // 将消息从通道接收并转发到 WebSocket
    let _ = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let text = serde_json::to_string(&message).unwrap_or_else(|_| "{}".to_string());
            if sender.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    })
    .await;

    // // 从 WebSocket 接收消息
    // let user_id_clone = user_id.clone();
    // // let app_state_clone = app_state.clone();
    // let tx_clone = tx.clone();
    // let mut recv_task = tokio::spawn(async move {
    //     while let Some(Ok(message)) = receiver.next().await {
    //         match message {
    //             Message::Text(text) => {
    //                 // 处理接收到的消息
    //                 println!("Received message from {}: {}", user_id_clone, text);
    //             }
    //             Message::Close(_) => {
    //                 break;
    //             }
    //             _ => {}
    //         }
    //     }

    //     // WebSocket 已关闭，取消注册
    //     app_state
    //         .ws_manager
    //         .unregister_connection(&user_id_clone, &tx_clone)
    //         .await;
    // });

    // 等待任何一个任务完成
    // tokio::select! {
    //     _ = (&mut send_task) => recv_task.abort(),
    //     _ = (&mut recv_task) => send_task.abort(),
    // }
}
