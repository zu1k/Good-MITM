use crate::{
    ca::CertificateAuthority,
    handler::{HttpHandler, MitmFilter},
    http_client::HttpClient,
    rule::Rule,
};
use http::{header, uri::PathAndQuery, HeaderValue, Uri};
use hyper::{
    body::HttpBody, server::conn::Http, service::service_fn, upgrade::Upgraded, Body, Method,
    Request, Response,
};
use log::*;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;

/// Enum representing either an HTTP request or response.
#[derive(Debug)]
pub enum RequestOrResponse {
    Request(Request<Body>),
    Response(Response<Body>),
}

/// Context for HTTP requests and responses.
#[derive(Clone, Debug)]
pub struct HttpContext {
    pub uri: Option<Uri>,

    pub should_modify_response: bool,
    pub rule: Vec<Rule>,
}

#[derive(Clone)]
pub(crate) struct MitmProxy {
    pub ca: Arc<CertificateAuthority>,
    pub client: HttpClient,

    pub http_handler: Arc<HttpHandler>,
    pub mitm_filter: Arc<MitmFilter>,
}

impl MitmProxy {
    pub(crate) async fn proxy(self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let res = if req.method() == Method::CONNECT {
            self.process_connect(req).await
        } else {
            self.process_request(req).await
        };

        match res {
            Ok(mut res) => {
                allow_all_cros(&mut res);
                Ok(res)
            }
            Err(err) => Err(err),
        }
    }

    async fn process_request(self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let mut ctx = HttpContext {
            uri: None,
            should_modify_response: false,
            rule: vec![],
        };

        if req.uri().path().starts_with("/mitm/cert")
            || req
                .headers()
                .get(http::header::HOST)
                .unwrap()
                .to_str()
                .unwrap_or_default()
                .contains("cert.mitm")
        {
            return Ok(self.get_cert_res());
        }

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
            rule: vec![],
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
                        let server_config = self.ca.gen_server_config(&authority).await;
                        let stream = TlsAcceptor::from(server_config)
                            .accept(upgraded)
                            .await
                            .expect("Failed to establish TLS connection with client");

                        if let Err(e) = self.serve_https(stream).await {
                            let e_string = e.to_string();
                            if !e_string.starts_with("error shutting down connection") {
                                debug!("res:: {}", e);
                            }
                        }
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

    async fn serve_https(
        self,
        stream: tokio_rustls::server::TlsStream<Upgraded>,
    ) -> Result<(), hyper::Error> {
        let service = service_fn(|mut req| {
            if req.version() == http::Version::HTTP_10 || req.version() == http::Version::HTTP_11 {
                let authority = req
                    .headers()
                    .get(http::header::HOST)
                    .expect("Host is a required header")
                    .to_str()
                    .expect("Failed to convert host to str");

                let uri = http::uri::Builder::new()
                    .scheme(http::uri::Scheme::HTTPS)
                    .authority(authority)
                    .path_and_query(
                        req.uri()
                            .path_and_query()
                            .unwrap_or(&PathAndQuery::from_static("/"))
                            .to_owned(),
                    )
                    .build()
                    .expect("Failed to build URI");

                let (mut parts, body) = req.into_parts();
                parts.uri = uri;
                req = Request::from_parts(parts, body)
            };

            self.clone().process_request(req)
        });

        Http::new()
            .serve_connection(stream, service)
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
