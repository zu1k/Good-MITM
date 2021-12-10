mod decoder;
mod proxy;
mod rewind;

use crate::error::Error;
use hyper::{
    client::HttpConnector,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Client, Request, Response, Server, Uri,
};
use hyper_proxy::{Proxy as UpstreamProxy, ProxyConnector};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use proxy::Proxy;
use std::{convert::Infallible, future::Future, net::SocketAddr, sync::Arc};
use tokio_tungstenite::tungstenite::Message;

pub(crate) use rewind::Rewind;

pub use crate::ca::CertificateAuthority;
pub use decoder::decode_response;
pub use hyper;
pub use hyper_proxy;
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
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HttpContext {
    /// Address of the client that is sending the request.
    pub client_addr: SocketAddr,
}

/// Context for websocket messages.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MessageContext {
    /// Address of the client.
    pub client_addr: SocketAddr,
    /// URI of the server.
    pub server_uri: Uri,
}

/// Handler for HTTP requests and responses.
///
/// Each request/response pair is passed to the same instance of the handler.
#[async_trait::async_trait]
pub trait HttpHandler: Clone + Send + Sync + 'static {
    /// The handler will be called for each HTTP request. It can either return a modified request,
    /// or a response. If a request is returned, it will be sent to the upstream server. If a
    /// response is returned, it will be sent to the client.
    async fn handle_request(
        &mut self,
        context: &HttpContext,
        request: Request<Body>,
    ) -> RequestOrResponse;

    /// The handler will be called for each HTTP response. It can modify a response before it is
    /// forwarded to the client.
    async fn handle_response(
        &mut self,
        context: &HttpContext,
        response: Response<Body>,
    ) -> Response<Body>;
}

/// Handler for websocket messages.
///
/// Messages sent over the same websocket stream are passed to the same instance of the handler.
#[async_trait::async_trait]
pub trait MessageHandler: Clone + Send + Sync + 'static {
    /// The handler will be called for each websocket message. It can return an optional modified
    /// message. If None is returned the message will not be forwarded.
    async fn handle_message(
        &mut self,
        context: &MessageContext,
        message: Message,
    ) -> Option<Message>;
}

#[async_trait::async_trait]
pub trait MitmFilter: Clone + Send + Sync + 'static {
    async fn filter(&mut self, context: &HttpContext, request: &Request<Body>) -> bool;
}

/// Configuration for the proxy server.
///
/// The proxy server can be configured with a number of options.
#[derive(Clone)]
pub struct ProxyConfig<F: Future<Output = ()>, H, M, MF>
where
    H: HttpHandler,
    M: MessageHandler,
    MF: MitmFilter,
{
    /// The address to listen on.
    pub listen_addr: SocketAddr,
    /// A future that once resolved will cause the proxy server to shut down.
    pub shutdown_signal: F,
    /// The certificate authority to use.
    pub ca: CertificateAuthority,
    /// A handler for HTTP requests and responses.
    pub http_handler: H,
    /// A handler for websocket messages.
    pub message_handler: M,
    pub mitm_filter: MF,
    /// The upstream proxy to use.
    pub upstream_proxy: Option<UpstreamProxy>,
}

/// Attempts to start a proxy server using the provided configuration options.
///
/// This will fail if the proxy server is unable to be started.
pub async fn start_proxy<F, H, M, MF>(
    ProxyConfig {
        listen_addr,
        shutdown_signal,
        ca,
        http_handler,
        message_handler,
        mitm_filter,
        upstream_proxy,
    }: ProxyConfig<F, H, M, MF>,
) -> Result<(), Error>
where
    F: Future<Output = ()>,
    H: HttpHandler,
    M: MessageHandler,
    MF: MitmFilter,
{
    let client = gen_client(upstream_proxy);
    let ca = Arc::new(ca);

    let make_service = make_service_fn(move |conn: &AddrStream| {
        let client = client.clone();
        let ca = Arc::clone(&ca);
        let http_handler = http_handler.clone();
        let message_handler = message_handler.clone();
        let mitm_filter = mitm_filter.clone();
        let client_addr = conn.remote_addr();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                Proxy {
                    ca: Arc::clone(&ca),
                    client: client.clone(),
                    http_handler: http_handler.clone(),
                    message_handler: message_handler.clone(),
                    mitm_filter: mitm_filter.clone(),
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
