use rcgen::Certificate;
use windows::{
    w,
    Win32::Security::Cryptography::{
        CertAddEncodedCertificateToStore, CertCloseStore, CertOpenSystemStoreW,
        CERT_STORE_ADD_REPLACE_EXISTING, HCRYPTPROV_LEGACY, PKCS_7_ASN_ENCODING, X509_ASN_ENCODING,
    },
};

pub fn install_cert(cert: Certificate) {
    let mut cert = cert.serialize_der().unwrap();
    unsafe {
        // get root store
        let store = CertOpenSystemStoreW(HCRYPTPROV_LEGACY(0), w!("ROOT"))
            .expect("open system root ca store");

        // add cert
        if !CertAddEncodedCertificateToStore(
            store,
            X509_ASN_ENCODING.0 | PKCS_7_ASN_ENCODING.0,
            cert.as_mut_ptr(),
            cert.len() as u32,
            CERT_STORE_ADD_REPLACE_EXISTING,
            0 as _,
        )
        .as_bool()
        {
            panic!("CertAddEncodedCertificateToStore failed")
        }

        if !CertCloseStore(store, 0).as_bool() {
            panic!("CertCloseStore failed")
        }
    }
}
