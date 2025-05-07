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
use tracing::info;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
// #[cfg(target_os = "linux")]
// fn print_jemalloc_stats() {
//     unsafe {
//         let mut epoch: u64 = 1;
//         let epoch_ptr = &mut epoch as *mut _ as *mut std::ffi::c_void;
//         jemalloc_sys::mallctl(
//             b"epoch\0".as_ptr() as *const i8,
//             std::ptr::null_mut(),
//             std::ptr::null_mut(),
//             epoch_ptr,
//             std::mem::size_of::<u64>(),
//         );
//         let mut allocated = 0usize;
//         let mut sz = std::mem::size_of::<usize>();
//         jemalloc_sys::mallctl(
//             b"stats.allocated\0".as_ptr() as *const i8,
//             &mut allocated as *mut _ as *mut std::ffi::c_void,
//             &mut sz,
//             std::ptr::null_mut(),
//             0,
//         );
//         println!("Jemalloc allocated: {} bytes", allocated);
//     }
// }
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    unsafe {
        env::set_var("RUST_BACKTRACE", "1");
    }

    // print_jemalloc_stats();
    // let config = setting::load_config();
    // build our application with a single route
    // let app = Router::new().route("/", get(|| async { "Hello, World!" }));
    let app = create_router().await.expect("Fail to create Router");
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3003));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("start to run!{}", addr);
    axum::serve(listener, app).await.unwrap();
}
