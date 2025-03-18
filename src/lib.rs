use axum::{
    Router,
    http::{HeaderValue, Method},
    middleware,
    routing::{get, post},
};
use tower::ServiceBuilder;
use tower_http::{cors::Any, cors::CorsLayer, trace::TraceLayer};

// 模块声明
pub mod dtos;
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
pub fn create_router() -> Router {
    // 初始化服务
    let auth_service = AuthService::new();
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any);
    // .allow_credentials(true);
    // 创建主路由
    // let app = Router::new()
    //     .route("/public", get(todo!()))
    //     .route("/internal", get(todo!()))
    //     .layer(CorsLayer::permissive());
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
                        .layer(middleware::from_fn(auth_middleware::auth_middleware)),
                ),
        )
        .layer(CorsLayer::permissive())
        .with_state(auth_service)
}
