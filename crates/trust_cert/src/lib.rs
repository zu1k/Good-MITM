use rcgen::Certificate;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[allow(unused_variables)]
pub fn trust_cert(cert: Certificate) {
    #[cfg(windows)]
    return windows::install_cert(cert);
    #[cfg(target_os = "linux")]
    return linux::install_cert(cert);
}
