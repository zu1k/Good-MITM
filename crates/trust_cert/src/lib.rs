use rcgen::Certificate;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[allow(unused_variables)]
pub fn trust_cert(cert: Certificate) {
    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            windows::install_cert(cert.serialize_der().unwrap());
        } else  if #[cfg(target_os = "linux")]  {
            todo!()
            // linux::install_cert();
        } else {
            panic!("not implemented on this target")
        }
    }
}
