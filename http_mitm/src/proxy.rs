use crate::{
    CertificateAuthority, HttpContext, HttpHandler, MaybeProxyClient, MessageContext,
    MessageHandler, MitmFilter, RequestOrResponse, Rewind,
};
use futures::{Sink, SinkExt, Stream, StreamExt};
use http::{uri::PathAndQuery, HeaderValue};
use hyper::{
    server::conn::Http, service::service_fn, upgrade::Upgraded, Body, Method, Request, Response,
    Uri,
};
use log::*;
use std::{net::SocketAddr, sync::Arc};
use tokio::io::AsyncReadExt;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::{connect_async, tungstenite, tungstenite::Message, WebSocketStream};

#[derive(Clone)]
pub(crate) struct Proxy<H, M1, M2, F1>
where
    H: HttpHandler,
    M1: MessageHandler,
    M2: MessageHandler,
    F1: MitmFilter,
{
    pub ca: Arc<CertificateAuthority>,
    pub client: MaybeProxyClient,
    pub http_handler: H,
    pub incoming_message_handler: M1,
    pub outgoing_message_handler: M2,
    pub mitm_filter: F1,
    pub client_addr: SocketAddr,
}

impl<H, M1, M2, F1> Proxy<H, M1, M2, F1>
where
    H: HttpHandler,
    M1: MessageHandler,
    M2: MessageHandler,
    F1: MitmFilter,
{
    pub(crate) async fn proxy(self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        match if req.method() == Method::CONNECT {
            // https
            self.process_connect(req).await
        } else {
            self.process_request(req).await
        } {
            Ok(resp) => Ok(allow_all_cros(resp)),
            Err(e) => Err(e),
        }
    }

    async fn process_request(
        mut self,
        mut req: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        let ctx = HttpContext {
            client_addr: self.client_addr,
        };

        if let Some(host) = req.headers().get(http::header::HOST) {
            if host.to_str().unwrap_or_default() == "good-mitm.com" {
                return Ok(Response::builder()
                    .header(
                        http::header::CONTENT_DISPOSITION,
                        "attachment; filename=good-mitm.crt",
                    )
                    .header(http::header::CONTENT_TYPE, "application/octet-stream")
                    .status(http::StatusCode::OK)
                    .body(Body::from(self.ca.clone().get_cert()))
                    .unwrap());
            }
        }

        req.headers_mut().remove(http::header::HOST);
        req.headers_mut().remove(http::header::ACCEPT_ENCODING);

        let req = match self.http_handler.handle_request(&ctx, req).await {
            RequestOrResponse::Request(req) => req,
            RequestOrResponse::Response(res) => return Ok(res),
        };

        if hyper_tungstenite::is_upgrade_request(&req) {
            let scheme = if req.uri().scheme().unwrap_or(&http::uri::Scheme::HTTP)
                == &http::uri::Scheme::HTTP
            {
                "ws"
            } else {
                "wss"
            };

            let uri = http::uri::Builder::new()
                .scheme(scheme)
                .authority(
                    req.uri()
                        .authority()
                        .expect("Authority not included in request")
                        .to_owned(),
                )
                .path_and_query(
                    req.uri()
                        .path_and_query()
                        .unwrap_or(&PathAndQuery::from_static("/"))
                        .to_owned(),
                )
                .build()
                .expect("Failed to build URI for websocket connection");

            let (res, websocket) =
                hyper_tungstenite::upgrade(req, None).expect("Request has missing headers");

            tokio::spawn(async move {
                let server_socket = websocket.await.unwrap_or_else(|_| {
                    panic!("Failed to upgrade websocket connection for {}", uri)
                });
                self.handle_websocket(server_socket, uri).await;
            });

            return Ok(res);
        }

        let res = match self.client {
            MaybeProxyClient::Proxy(client) => client.request(req).await?,
            MaybeProxyClient::Https(client) => client.request(req).await?,
        };

        Ok(self.http_handler.handle_response(&ctx, res).await)
    }

    async fn process_connect(mut self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let ctx = HttpContext {
            client_addr: self.client_addr,
        };
        if self.mitm_filter.filter(&ctx, &req).await {
            tokio::task::spawn(async move {
                let authority = req
                    .uri()
                    .authority()
                    .expect("URI does not contain authority")
                    .clone();

                match hyper::upgrade::on(req).await {
                    Ok(mut upgraded) => {
                        let mut buffer = [0; 4];
                        let bytes_read = upgraded
                            .read(&mut buffer)
                            .await
                            .expect("Failed to read from upgraded connection");

                        let upgraded = Rewind::new_buffered(
                            upgraded,
                            bytes::Bytes::copy_from_slice(buffer[..bytes_read].as_ref()),
                        );

                        if bytes_read == 4 && buffer == *b"GET " {
                            if let Err(e) = self.serve_websocket(upgraded).await {
                                debug!("https connect error for {}: {}", authority, e);
                            }
                        } else {
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
                    }
                    Err(e) => debug!("upgrade error for {}: {}", authority, e),
                };
            });
        } else {
            tokio::task::spawn(async move {});
        }
        Ok(Response::new(Body::empty()))
    }

    async fn handle_websocket(self, server_socket: WebSocketStream<Upgraded>, uri: Uri) {
        let (client_socket, _) = connect_async(&uri)
            .await
            .unwrap_or_else(|_| panic!("Failed to open websocket connection to {}", uri));

        let (server_sink, server_stream) = server_socket.split();
        let (client_sink, client_stream) = client_socket.split();

        let Proxy {
            incoming_message_handler,
            outgoing_message_handler,
            ..
        } = self;

        spawn_message_forwarder(
            server_stream,
            client_sink,
            incoming_message_handler,
            self.client_addr,
            uri.clone(),
        );

        spawn_message_forwarder(
            client_stream,
            server_sink,
            outgoing_message_handler,
            self.client_addr,
            uri,
        );
    }

    async fn serve_websocket(self, stream: Rewind<Upgraded>) -> Result<(), hyper::Error> {
        let service = service_fn(|req| {
            let authority = req
                .headers()
                .get(http::header::HOST)
                .expect("Host is a required header")
                .to_str()
                .expect("Failed to convert host to str");

            let uri = http::uri::Builder::new()
                .scheme(http::uri::Scheme::HTTP)
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
            let req = Request::from_parts(parts, body);
            self.clone().process_request(req)
        });

        Http::new()
            .serve_connection(stream, service)
            .with_upgrades()
            .await
    }

    async fn serve_https(
        self,
        stream: tokio_rustls::server::TlsStream<Rewind<Upgraded>>,
    ) -> Result<(), hyper::Error> {
        let service = service_fn(|mut req| {
            if req.version() == http::Version::HTTP_11 {
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
}

fn spawn_message_forwarder(
    mut stream: impl Stream<Item = Result<Message, tungstenite::Error>> + Unpin + Send + 'static,
    mut sink: impl Sink<Message, Error = tungstenite::Error> + Unpin + Send + 'static,
    mut handler: impl MessageHandler,
    client_addr: SocketAddr,
    uri: Uri,
) {
    let ctx = MessageContext {
        client_addr,
        server_uri: uri,
    };

    tokio::spawn(async move {
        while let Some(message) = stream.next().await {
            match message {
                Ok(message) => {
                    let message = match handler.handle_message(&ctx, message).await {
                        Some(message) => message,
                        None => continue,
                    };

                    match sink.send(message).await {
                        Err(tungstenite::Error::ConnectionClosed) => (),
                        Err(e) => error!("websocket send error: {}", e),
                        _ => (),
                    }
                }
                Err(e) => error!("websocket message error: {}", e),
            }
        }
    });
}

fn allow_all_cros(resp: Response<Body>) -> Response<Body> {
    let mut resp = resp;
    let header = resp.headers_mut();
    let all = HeaderValue::from_str("*").unwrap();
    header.insert(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, all.clone());
    header.insert(http::header::ACCESS_CONTROL_ALLOW_METHODS, all.clone());
    header.insert(http::header::ACCESS_CONTROL_ALLOW_METHODS, all);
    resp
}
