extern crate rcgen;
use log::error;
use rcgen::*;
use std::fs;

pub fn gen_ca() {
    let subject_alt_names = vec!["*".to_string()];
    let mut param = CertificateParams::new(subject_alt_names);
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, "MITM");
    param.distinguished_name = distinguished_name;
    let cert = Certificate::from_params(param).unwrap();
    let cert_crt = cert.serialize_pem().unwrap();

    println!("{}", cert_crt);
    if let Err(err) = fs::write("cert.crt", cert_crt) {
        error!("cert file write failed: {}", err);
    }

    let private_key = cert.serialize_private_key_pem();
    println!("{}", private_key);
    if let Err(err) = fs::write("private.key", private_key) {
        error!("private key file write failed: {}", err);
    }
}
