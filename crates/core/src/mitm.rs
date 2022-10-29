use crate::{
    ca::CertificateAuthority,
    handler::{CustomContextData, HttpHandler, MitmFilter},
    http_client::HttpClient,
};
use http::{header, uri::Scheme, HeaderValue, Uri};
use hyper::{
    body::HttpBody, server::conn::Http, service::service_fn, upgrade::Upgraded, Body, Method,
    Request, Response,
};
use log::*;
use std::{marker::PhantomData, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_rustls::TlsAcceptor;

/// Enum representing either an HTTP request or response.
#[derive(Debug)]
pub enum RequestOrResponse {
    Request(Request<Body>),
    Response(Response<Body>),
}

/// Context for HTTP requests and responses.
#[derive(Default, Debug)]
pub struct HttpContext<D: Default + Send + Sync> {
    pub uri: Option<Uri>,

    pub should_modify_response: bool,
    pub custom_data: D,
}

#[derive(Clone)]
pub(crate) struct MitmProxy<H, D>
where
    H: HttpHandler<D>,
    D: CustomContextData,
{
    pub ca: Arc<CertificateAuthority>,
    pub client: HttpClient,

    pub http_handler: Arc<H>,
    pub mitm_filter: Arc<MitmFilter<D>>,

    pub custom_contex_data: PhantomData<D>,
}

impl<H, D> MitmProxy<H, D>
where
    H: HttpHandler<D>,
    D: CustomContextData,
{
    pub(crate) async fn proxy_req(
        self,
        req: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        let res = if req.method() == Method::CONNECT {
            self.process_connect(req).await
        } else {
            self.process_request(req, Scheme::HTTP).await
        };

        match res {
            Ok(mut res) => {
                allow_all_cros(&mut res);
                Ok(res)
            }
            Err(err) => {
                error!("proxy request failed: {err:?}");
                Err(err)
            }
        }
    }

    async fn process_request(
        self,
        mut req: Request<Body>,
        scheme: Scheme,
    ) -> Result<Response<Body>, hyper::Error> {
        if req.uri().path().starts_with("/mitm/cert") {
            return Ok(self.get_cert_res());
        }

        let mut ctx = HttpContext {
            uri: None,
            should_modify_response: false,
            ..Default::default()
        };

        // if req.uri().authority().is_none() {
        if req.version() == http::Version::HTTP_10 || req.version() == http::Version::HTTP_11 {
            let (mut parts, body) = req.into_parts();

            if let Some(Ok(authority)) = parts
                .headers
                .get(http::header::HOST)
                .map(|host| host.to_str())
            {
                let mut uri = parts.uri.into_parts();
                uri.scheme = Some(scheme.clone());
                uri.authority = authority.try_into().ok();
                parts.uri = Uri::from_parts(uri).expect("build uri");
            }

            req = Request::from_parts(parts, body);
        };
        // }

        let mut req = match self.http_handler.handle_request(&mut ctx, req).await {
            RequestOrResponse::Request(req) => req,
            RequestOrResponse::Response(res) => return Ok(res),
        };

        {
            let header_mut = req.headers_mut();
            header_mut.remove(http::header::HOST);
            header_mut.remove(http::header::ACCEPT_ENCODING);
            header_mut.remove(http::header::CONTENT_LENGTH);
        }

        let res = match self.client {
            HttpClient::Proxy(client) => client.request(req).await?,
            HttpClient::Https(client) => client.request(req).await?,
        };

        let mut res = self.http_handler.handle_response(&mut ctx, res).await;
        let length = res.size_hint().lower();

        {
            let header_mut = res.headers_mut();

            if let Some(content_length) = header_mut.get_mut(http::header::CONTENT_LENGTH) {
                *content_length = HeaderValue::from_str(&length.to_string()).unwrap();
            }

            // Remove `Strict-Transport-Security` to avoid HSTS
            // See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security
            header_mut.remove(header::STRICT_TRANSPORT_SECURITY);
        }

        Ok(res)
    }

    async fn process_connect(self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let ctx = HttpContext {
            uri: None,
            should_modify_response: false,
            ..Default::default()
        };

        if self.mitm_filter.filter(&ctx, &req).await {
            tokio::task::spawn(async move {
                let authority = req
                    .uri()
                    .authority()
                    .expect("URI does not contain authority")
                    .clone();

                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        self.serve_tls(upgraded).await;
                    }
                    Err(e) => debug!("upgrade error for {}: {}", authority, e),
                };
            });
        } else {
            tokio::task::spawn(async move {
                let remote_addr = host_addr(req.uri()).unwrap();
                let upgraded = hyper::upgrade::on(req).await.unwrap();
                tunnel(upgraded, remote_addr).await
            });
        }
        Ok(Response::new(Body::empty()))
    }

    pub async fn serve_tls<IO: AsyncRead + AsyncWrite + Unpin + Send + 'static>(self, stream: IO) {
        let server_config = self.ca.clone().gen_server_config();

        match TlsAcceptor::from(server_config).accept(stream).await {
            Ok(stream) => {
                if let Err(e) = Http::new()
                    .http1_preserve_header_case(true)
                    .http1_title_case_headers(true)
                    .serve_connection(
                        stream,
                        service_fn(|req| self.clone().process_request(req, Scheme::HTTPS)),
                    )
                    .with_upgrades()
                    .await
                {
                    let e_string = e.to_string();
                    if !e_string.starts_with("error shutting down connection") {
                        debug!("res:: {}", e);
                    }
                }
            }
            Err(err) => {
                error!("Tls accept failed: {err}")
            }
        }
    }

    pub async fn serve_stream<S>(self, stream: S) -> Result<(), hyper::Error>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        Http::new()
            .http1_preserve_header_case(true)
            .http1_title_case_headers(true)
            .serve_connection(stream, service_fn(|req| self.clone().proxy_req(req)))
            .with_upgrades()
            .await
    }

    fn get_cert_res(&self) -> hyper::Response<Body> {
        Response::builder()
            .header(
                http::header::CONTENT_DISPOSITION,
                "attachment; filename=good-mitm.crt",
            )
            .header(http::header::CONTENT_TYPE, "application/octet-stream")
            .status(http::StatusCode::OK)
            .body(Body::from(self.ca.clone().get_cert()))
            .unwrap()
    }
}

fn allow_all_cros(res: &mut Response<Body>) {
    let header_mut = res.headers_mut();
    let all = HeaderValue::from_str("*").unwrap();
    header_mut.insert(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, all.clone());
    header_mut.insert(http::header::ACCESS_CONTROL_ALLOW_METHODS, all.clone());
    header_mut.insert(http::header::ACCESS_CONTROL_ALLOW_METHODS, all);
}

fn host_addr(uri: &http::Uri) -> Option<String> {
    uri.authority().map(|auth| auth.to_string())
}

async fn tunnel(mut upgraded: Upgraded, addr: String) -> std::io::Result<()> {
    let mut server = TcpStream::connect(addr).await?;
    tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;
    Ok(())
}
