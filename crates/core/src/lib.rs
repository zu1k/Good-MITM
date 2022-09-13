use error::Error;
use handler::{CustomContextData, HttpHandler, MitmFilter};
use http_client::gen_client;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Server,
};
use hyper_proxy::Proxy as UpstreamProxy;
use mitm::MitmProxy;
use std::{future::Future, marker::PhantomData, net::SocketAddr, sync::Arc};
use typed_builder::TypedBuilder;

pub use ca::CertificateAuthority;
pub use hyper;
pub use rcgen;
pub use tokio_rustls;

mod ca;
mod error;
pub mod handler;
mod http_client;
pub mod mitm;

#[derive(TypedBuilder)]
pub struct Proxy<F, H, D>
where
    F: Future<Output = ()>,
    H: HttpHandler<D>,
    D: CustomContextData,
{
    /// The address to listen on.
    pub listen_addr: SocketAddr,
    /// A future that once resolved will cause the proxy server to shut down.
    pub shutdown_signal: F,
    /// The certificate authority to use.
    pub ca: CertificateAuthority,
    pub upstream_proxy: Option<UpstreamProxy>,

    pub mitm_filters: Vec<String>,
    pub handler: H,

    #[builder(default)]
    _custom_contex_data: PhantomData<D>,
}

impl<F, H, D> Proxy<F, H, D>
where
    F: Future<Output = ()>,
    H: HttpHandler<D>,
    D: CustomContextData,
{
    pub async fn start_proxy(self) -> Result<(), Error> {
        let client = gen_client(self.upstream_proxy)?;
        let ca = Arc::new(self.ca);

        let http_handler = Arc::new(self.handler);
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

                        custom_contex_data: Default::default(),
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
            .map_err(Error::from)
    }
}
