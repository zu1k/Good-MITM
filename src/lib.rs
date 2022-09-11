pub mod ca;
pub mod error;
pub mod file;

pub use core;
pub use hyper_proxy;
pub use trust_cert;

pub async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}
