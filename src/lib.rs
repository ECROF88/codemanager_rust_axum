use axum::{
    Router,
    http::{HeaderValue, Method},
    middleware,
    routing::{get, post},
};
use services::service::{AppState, GitService};
use tower::ServiceBuilder;
use tower_http::{cors::Any, cors::CorsLayer, trace::TraceLayer};

// 模块声明
pub mod dtos;
pub mod gitmodule;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;
pub mod shared;
pub mod vos;
// 类型导入
use crate::{handlers::handler, services::service::AuthService, shared::auth_middleware};
/*
let app = Router::new()
  .route("...", ...)
  .layer(3)
  .layer(2)
  .layer(1); */
/// 创建应用路由
/// struct AppState {

pub fn create_router() -> Router {
    // 初始化服务
    let app_state = AppState {
        auth_service: AuthService::new(),
        git_service: GitService::new(),
    };

    Router::new()
        // API 路由组
        .nest(
            "/api",
            Router::new()
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
                        // .route("/gitclone", post(handler::get_user_data))
                        // .route("/gitcommit", post(handler::get_user_data))
                        .route(
                            "/repo/commithistories",
                            get(handler::get_repo_commit_histories),
                        )
                        .route("/repo/repos", get(handler::get_repos))
                        .route("/repo/gitclone", post(handler::clone_repo_for_user))
                        // .route("/repo/commit", post(handler::commit_for_user_repo))
                        .route("/repo/getdiff", get(handler::get_repo_commit_diff))
                        .layer(middleware::from_fn(auth_middleware::auth_middleware)),
                ),
        )
        .layer(CorsLayer::permissive())
        .with_state(app_state)
}
