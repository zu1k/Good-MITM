use handler::{HttpHandler, MitmFilter};
use http_client::gen_client;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Server,
};
use hyper_proxy::Proxy as UpstreamProxy;
use mitm::MitmProxy;
use rule::Rule;
use std::{future::Future, net::SocketAddr, sync::Arc};
use typed_builder::TypedBuilder;

pub use ca::CertificateAuthority;
use error::Error;

mod ca;
mod cache;
mod error;
mod handler;
mod http_client;
mod mitm;
pub mod rule;

#[derive(TypedBuilder)]
pub struct Proxy<F: Future<Output = ()>> {
    /// The address to listen on.
    pub listen_addr: SocketAddr,
    /// A future that once resolved will cause the proxy server to shut down.
    pub shutdown_signal: F,
    /// The certificate authority to use.
    pub ca: CertificateAuthority,
    pub upstream_proxy: Option<UpstreamProxy>,

    pub rules: Vec<Rule>,
    pub mitm_filters: Vec<String>,
}

impl<F> Proxy<F>
where
    F: Future<Output = ()>,
{
    pub async fn start_proxy(self) -> Result<(), Error> {
        let client = gen_client(self.upstream_proxy);
        let ca = Arc::new(self.ca);

        let rules = Arc::new(self.rules);
        let http_handler = Arc::new(HttpHandler::new(rules));
        let mitm_filter = Arc::new(MitmFilter::new(self.mitm_filters));

        let make_service = make_service_fn(move |_conn: &AddrStream| {
            let client = client.clone();
            let ca = Arc::clone(&ca);
            let http_handler = Arc::clone(&http_handler);
            let mitm_filter = Arc::clone(&mitm_filter);

            async move {
                Ok::<_, Error>(service_fn(move |req| {
                    MitmProxy {
                        ca: Arc::clone(&ca),
                        client: client.clone(),

                        http_handler: Arc::clone(&http_handler),
                        mitm_filter: Arc::clone(&mitm_filter),
                    }
                    .proxy(req)
                }))
            }
        });

        Server::bind(&self.listen_addr)
            .http1_preserve_header_case(true)
            .http1_title_case_headers(true)
            .serve(make_service)
            .with_graceful_shutdown(self.shutdown_signal)
            .await
            .map_err(|err| err.into())
    }
}
