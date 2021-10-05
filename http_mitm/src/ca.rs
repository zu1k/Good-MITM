use crate::Error;
use chrono::{Duration, Utc};
use http::uri::Authority;
use moka::future::Cache;
use rcgen::{DistinguishedName, DnType, KeyPair, RcgenError, SanType};
use std::sync::Arc;
use tokio_rustls::rustls::{self, NoClientAuth, ServerConfig};

/// Issues certificates for use when communicating with clients.
///
/// Issues certificates for communicating with clients over TLS. Certificates are cached in memory
/// up to a max size that is provided when creating the authority. Clients should be configured to
/// either trust the provided root certificate, or to ignore certificate errors.
#[derive(Clone)]
pub struct CertificateAuthority {
    private_key: rustls::PrivateKey,
    ca_cert: rustls::Certificate,
    cache: Cache<Authority, Arc<ServerConfig>>,
}

impl CertificateAuthority {
    /// Attempts to create a new certificate authority.
    ///
    /// This will fail if the provided key or certificate is invalid, or if the key does not match
    /// the certificate.
    pub fn new(
        private_key: rustls::PrivateKey,
        ca_cert: rustls::Certificate,
        cache_size: usize,
    ) -> Result<CertificateAuthority, Error> {
        let ca = CertificateAuthority {
            private_key,
            ca_cert,
            cache: Cache::new(cache_size),
        };

        ca.validate()?;
        Ok(ca)
    }

    pub(crate) async fn gen_server_config(&self, authority: &Authority) -> Arc<ServerConfig> {
        if let Some(server_cfg) = self.cache.get(authority) {
            return server_cfg;
        }

        let mut server_cfg = ServerConfig::new(NoClientAuth::new());
        let certs = vec![self.gen_cert(authority)];

        server_cfg
            .set_single_cert(certs, self.private_key.clone())
            .expect("Failed to set certificate");
        server_cfg.set_protocols(&[b"http/1.1".to_vec()]);

        let server_cfg = Arc::new(server_cfg);

        self.cache
            .insert(authority.clone(), Arc::clone(&server_cfg))
            .await;

        server_cfg
    }

    fn gen_cert(&self, authority: &Authority) -> rustls::Certificate {
        let now = Utc::now();
        let mut params = rcgen::CertificateParams::default();
        params.not_before = now - Duration::weeks(104);
        params.not_after = now + Duration::weeks(104);
        params
            .subject_alt_names
            .push(SanType::DnsName(authority.host().to_string()));
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, authority.host());
        params.distinguished_name = distinguished_name;

        let key_pair = KeyPair::from_der(&self.private_key.0).expect("Failed to parse private key");
        params.alg = key_pair
            .compatible_algs()
            .next()
            .expect("Failed to find compatible algorithm");
        params.key_pair = Some(key_pair);

        let key_pair = KeyPair::from_der(&self.private_key.0).expect("Failed to parse private key");

        let ca_cert_params = rcgen::CertificateParams::from_ca_cert_der(&self.ca_cert.0, key_pair)
            .expect("Failed to parse CA certificate");
        let ca_cert = rcgen::Certificate::from_params(ca_cert_params)
            .expect("Failed to generate CA certificate");

        let cert = rcgen::Certificate::from_params(params).expect("Failed to generate certificate");
        rustls::Certificate(
            cert.serialize_der_with_signer(&ca_cert)
                .expect("Failed to serialize certificate"),
        )
    }

    fn validate(&self) -> Result<(), RcgenError> {
        let key_pair = rcgen::KeyPair::from_der(&self.private_key.0)?;
        rcgen::CertificateParams::from_ca_cert_der(&self.ca_cert.0, key_pair)?;
        Ok(())
    }
}
