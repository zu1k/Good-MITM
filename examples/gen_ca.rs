extern crate rcgen;
use rcgen::*;

fn main() {
    let subject_alt_names = vec!["*".to_string()];
    let mut param = CertificateParams::new(subject_alt_names);
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, "MITM");
    param.distinguished_name = distinguished_name;
    let cert = Certificate::from_params(param).unwrap();
    println!("{}", cert.serialize_pem().unwrap());
    println!("{}", cert.serialize_private_key_pem());
}
