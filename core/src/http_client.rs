use std::{sync::Arc, time::SystemTime};

use hyper::{client::HttpConnector, Client};
use hyper_proxy::{Proxy as UpstreamProxy, ProxyConnector};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use rustls::{
    client::{ServerCertVerified, ServerCertVerifier},
    ClientConfig,
};

#[derive(Clone)]
pub enum HttpClient {
    Proxy(Client<ProxyConnector<HttpsConnector<HttpConnector>>>),
    Https(Client<HttpsConnector<HttpConnector>>),
}

pub fn gen_client(upstream_proxy: Option<UpstreamProxy>) -> HttpClient {
    let https = HttpsConnectorBuilder::new()
        .with_tls_config({
            let cert_resolver = Arc::new(TrustAllCertVerifier::default());
            ClientConfig::builder()
                .with_safe_defaults()
                .with_custom_certificate_verifier(cert_resolver)
                .with_no_client_auth()
        })
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .build();

    if let Some(proxy) = upstream_proxy {
        let connector = ProxyConnector::from_proxy_unsecured(https, proxy);
        return HttpClient::Proxy(
            Client::builder()
                .http1_title_case_headers(true)
                .http1_preserve_header_case(true)
                .build(connector),
        );
    } else {
        HttpClient::Https(
            Client::builder()
                .http1_title_case_headers(true)
                .http1_preserve_header_case(true)
                .build(https),
        )
    }
}

#[derive(Default)]
struct TrustAllCertVerifier;

impl ServerCertVerifier for TrustAllCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _n_ow: SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}
