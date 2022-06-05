use crate::error::Error;
use cookie::time::OffsetDateTime;
use http::uri::Authority;
use moka::future::Cache;
use rand::{thread_rng, Rng};
use rcgen::{
    DistinguishedName, DnType, ExtendedKeyUsagePurpose, KeyPair, KeyUsagePurpose, RcgenError,
    SanType,
};
use std::sync::Arc;
use time::ext::NumericalDuration;
use tokio_rustls::rustls::{self, ServerConfig};

const CERT_TTL_DAYS: u64 = 365;
const CERT_CACHE_TTL_SECONDS: u64 = CERT_TTL_DAYS * 24 * 60 * 60 / 2;

/// Issues certificates for use when communicating with clients.
///
/// Issues certificates for communicating with clients over TLS. Certificates are cached in memory
/// up to a max size that is provided when creating the authority. Clients should be configured to
/// either trust the provided root certificate, or to ignore certificate errors.
#[derive(Clone)]
pub struct CertificateAuthority {
    private_key: rustls::PrivateKey,
    ca_cert: rustls::Certificate,
    ca_cert_string: String,
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
        ca_cert_string: String,
        cache_size: u64,
    ) -> Result<CertificateAuthority, Error> {
        let ca = CertificateAuthority {
            private_key,
            ca_cert,
            ca_cert_string,
            cache: Cache::builder()
                .max_capacity(cache_size)
                .time_to_live(std::time::Duration::from_secs(CERT_CACHE_TTL_SECONDS))
                .build(),
        };

        ca.validate()?;
        Ok(ca)
    }

    pub(crate) async fn gen_server_config(&self, authority: &Authority) -> Arc<ServerConfig> {
        if let Some(server_cfg) = self.cache.get(authority) {
            return server_cfg;
        }

        let certs = vec![self.gen_cert(authority)];

        let server_cfg = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, self.private_key.clone())
            .expect("Failed to set certificate");
        let server_cfg = Arc::new(server_cfg);

        self.cache
            .insert(authority.clone(), Arc::clone(&server_cfg))
            .await;

        server_cfg
    }

    fn gen_cert(&self, authority: &Authority) -> rustls::Certificate {
        let mut params = rcgen::CertificateParams::default();

        params.serial_number = Some(thread_rng().gen::<u64>());
        params.not_before = OffsetDateTime::now_utc().saturating_sub(1.days());
        params.not_after = OffsetDateTime::now_utc().saturating_add((CERT_TTL_DAYS as i64).days());
        params
            .subject_alt_names
            .push(SanType::DnsName(authority.host().to_string()));
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, authority.host());
        params.distinguished_name = distinguished_name;

        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

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

    pub fn get_cert(&self) -> String {
        self.ca_cert_string.clone()
    }
}
