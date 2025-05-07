use axum::{
    Router,
    http::{HeaderValue, Method},
    middleware,
    routing::{get, post},
};
use gitmodule::structs::WebSocketManager;
use services::service::{AppState, GitService};
use shared::error::AppError;
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{cors::Any, cors::CorsLayer, trace::TraceLayer};

// 模块声明
pub mod db;
pub mod dtos;
pub mod gitmodule;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;
pub mod shared;
pub mod vos;
// 类型导入
use crate::{handlers::handler, shared::auth_middleware};
/*
let app = Router::new()
  .route("...", ...)
  .layer(3)
  .layer(2)
  .layer(1); */
/// 创建应用路由
/// struct AppState {

pub async fn create_router() -> Result<Router, AppError> {
    // 初始化服务
    let app_state = AppState::init_app().await?;

    Ok(Router::new()
        // API 路由组
        .nest(
            "/api",
            Router::new()
                .route("/ws/{token}", get(handler::websocket_handler))
                // 认证路由
                .nest(
                    "/auth",
                    Router::new()
                        .route("/register", post(handler::register))
                        .route("/login", post(handler::login)),
                )
                // 需要认证的路由组
                .nest(
                    "/protected",
                    Router::new()
                        .route("/user/userdata", get(handler::get_user_data))
                        .route("/user/update", post(handler::update_user_data))
                        .route(
                            "/repo/commithistories",
                            get(handler::get_repo_commit_histories),
                        )
                        .route("/repo/repos", get(handler::get_repos))
                        .route("/repo/clone", post(handler::clone_repo_for_user))
                        .route("/repo/files", get(handler::get_repo_files_tree))
                        .route("/repo/filecontent", get(handler::get_repo_file_content))
                        .route("/repo/getdiff", get(handler::get_repo_commit_diff))
                        .route("/repo/update", post(handler::update_repo_data))
                        .route("/repo/del", post(handler::del_repo_for_user))
                        .layer(middleware::from_fn(auth_middleware::auth_middleware)),
                ),
        )
        .layer(CorsLayer::permissive())
        .with_state(app_state))
}
