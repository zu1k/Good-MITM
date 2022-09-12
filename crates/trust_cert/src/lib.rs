use rcgen::Certificate;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[allow(unused_variables)]
pub fn trust_cert(cert: Certificate) {
    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            windows::install_cert(cert);
        } else  if #[cfg(target_os = "linux")]  {
            linux::install_cert(cert);
        } else {
            panic!("not implemented on this target")
        }
    }
}
