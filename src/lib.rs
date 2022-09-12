pub mod ca;
pub mod error;
pub mod file;

pub use hyper_proxy;
pub use mitm_core;
pub use trust_cert;

pub async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}
