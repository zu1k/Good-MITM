use windows::Win32::{
    Foundation::*,
    Security::Cryptography::{
        CertAddEncodedCertificateToStore, CertCloseStore, CertOpenSystemStoreA,
        CERT_STORE_ADD_REPLACE_EXISTING, PKCS_7_ASN_ENCODING, X509_ASN_ENCODING,
    },
};

pub fn install_cert(cert: Vec<u8>) {
    let mut cert = cert;
    unsafe {
        // get root store
        let store = CertOpenSystemStoreA(0, PSTR(String::from("ROOT\0").as_mut_ptr()));

        // add cert
        if !CertAddEncodedCertificateToStore(
            store,
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
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
