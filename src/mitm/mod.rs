mod decoder;
mod proxy;
mod rewind;

use crate::{error::Error, rule::Rule};
use hyper::{
    client::HttpConnector,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Client, Request, Response, Server, Uri,
};
use hyper_proxy::{Proxy as UpstreamProxy, ProxyConnector};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use proxy::Proxy;
use std::{future::Future, net::SocketAddr, sync::Arc};

pub use crate::ca::CertificateAuthority;
pub use decoder::decode_response;
pub use hyper;
pub use hyper_proxy;
pub(crate) use rewind::Rewind;
pub use tokio_rustls::rustls;
pub use tokio_tungstenite::tungstenite;

#[derive(Clone)]
pub enum MaybeProxyClient {
    Proxy(Client<ProxyConnector<HttpsConnector<HttpConnector>>>),
    Https(Client<HttpsConnector<HttpConnector>>),
}

/// Enum representing either an HTTP request or response.
#[derive(Debug)]
pub enum RequestOrResponse {
    Request(Request<Body>),
    Response(Response<Body>),
}

/// Context for HTTP requests and responses.
#[derive(Clone, Debug)]
pub struct HttpContext {
    /// Address of the client that is sending the request.
    pub client_addr: SocketAddr,

    pub uri: Option<Uri>,

    pub should_modify_response: bool,
    pub rule: Vec<Rule>,
}

/// Context for websocket messages.
#[derive(Clone, Debug)]
pub struct MessageContext {
    /// Address of the client.
    pub client_addr: SocketAddr,
    /// URI of the server.
    pub server_uri: Uri,
}

/// Configuration for the proxy server.
///
/// The proxy server can be configured with a number of options.
#[derive(Clone)]
pub struct ProxyConfig<F: Future<Output = ()>> {
    /// The address to listen on.
    pub listen_addr: SocketAddr,
    /// A future that once resolved will cause the proxy server to shut down.
    pub shutdown_signal: F,
    /// The certificate authority to use.
    pub ca: CertificateAuthority,
    pub upstream_proxy: Option<UpstreamProxy>,
}

/// Attempts to start a proxy server using the provided configuration options.
///
/// This will fail if the proxy server is unable to be started.
pub async fn start_proxy<F>(
    ProxyConfig {
        listen_addr,
        shutdown_signal,
        ca,
        upstream_proxy,
    }: ProxyConfig<F>,
) -> Result<(), Error>
where
    F: Future<Output = ()>,
{
    let client = gen_client(upstream_proxy);
    let ca = Arc::new(ca);

    let make_service = make_service_fn(move |conn: &AddrStream| {
        let client = client.clone();
        let ca = Arc::clone(&ca);
        let client_addr = conn.remote_addr();

        async move {
            Ok::<_, Error>(service_fn(move |req| {
                Proxy {
                    ca: Arc::clone(&ca),
                    client: client.clone(),
                    client_addr,
                }
                .proxy(req)
            }))
        }
    });

    Server::bind(&listen_addr)
        .http1_preserve_header_case(true)
        .http1_title_case_headers(true)
        .serve(make_service)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|err| err.into())
}

fn gen_client(upstream_proxy: Option<UpstreamProxy>) -> MaybeProxyClient {
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

        return MaybeProxyClient::Proxy(
            Client::builder()
                .http1_title_case_headers(true)
                .http1_preserve_header_case(true)
                .build(connector),
        );
    } else {
        MaybeProxyClient::Https(
            Client::builder()
                .http1_title_case_headers(true)
                .http1_preserve_header_case(true)
                .build(https),
        )
    }
}
