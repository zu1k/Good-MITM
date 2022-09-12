use log::error;
use mitm_core::rcgen::*;
use std::fs;

pub fn gen_ca() -> Certificate {
    let cert = mitm_core::CertificateAuthority::gen_ca().expect("mitm-core generate cert");
    let cert_crt = cert.serialize_pem().unwrap();

    fs::create_dir("ca").unwrap();

    println!("{}", cert_crt);
    if let Err(err) = fs::write("ca/cert.crt", cert_crt) {
        error!("cert file write failed: {}", err);
    }

    let private_key = cert.serialize_private_key_pem();
    println!("{}", private_key);
    if let Err(err) = fs::write("ca/private.key", private_key) {
        error!("private key file write failed: {}", err);
    }

    cert
}
