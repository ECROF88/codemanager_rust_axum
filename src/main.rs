use std::env;

use axum::{Router, routing::get};
mod dtos;
// mod error;
// mod handlers;
// mod models;
// mod routes;
// mod services;
use code_management_backend::{create_router, shared::setting};
// use handlers::handler;
// use shared::error;
mod shared;

use jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() {
    unsafe {
        env::set_var("RUST_BACKTRACE", "1");
    }
    // let config = setting::load_config();
    // build our application with a single route
    // let app = Router::new().route("/", get(|| async { "Hello, World!" }));
    let app = create_router();
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
