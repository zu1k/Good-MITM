use hyper::{client::HttpConnector, Client};
use hyper_proxy::{Proxy as UpstreamProxy, ProxyConnector};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};

#[derive(Clone)]
pub enum HttpClient {
    Proxy(Client<ProxyConnector<HttpsConnector<HttpConnector>>>),
    Https(Client<HttpsConnector<HttpConnector>>),
}

pub fn gen_client(upstream_proxy: Option<UpstreamProxy>) -> HttpClient {
    let https = HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .build();

    if let Some(proxy) = upstream_proxy {
        // The following can only panic when using the "rustls" hyper_proxy feature
        let connector = ProxyConnector::from_proxy(https, proxy)
            .expect("Failed to create upstream proxy connector");

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
