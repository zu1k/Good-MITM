use hudsucker::{
    async_trait::async_trait,
    hyper::{body::*, header, Body, Request, Response, StatusCode},
    rustls::internal::pemfile,
    tungstenite::Message,
    *,
};

use fancy_regex::Regex;
use log::*;
use std::net::SocketAddr;

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[derive(Clone, Default)]
struct MitmHandler {
    should_modify_response: bool,
    regex: Option<String>,
}

#[async_trait]
impl HttpHandler for MitmHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        let re = Regex::new(r"(nfmovies)(?!.*?(\.css|\.js|\.jpeg|\.png|\.gif)).*").unwrap();
        println!("{:?}", req.uri().to_string());
        let u = req.uri().to_string();
        if re.is_match(&u).unwrap() {
            self.should_modify_response = true;
        }
        let mut req = req;
        req.headers_mut().remove(header::ACCEPT_ENCODING);
        RequestOrResponse::Request(req)
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        if !self.should_modify_response {
            return res;
        }

        let origin_body = "</body>";
        let new_body = include_str!("../assets/bb.html");

        // println!("{:?}", res);
        let (parts, body) = res.into_parts();
        if match parts.headers.get(header::CONTENT_TYPE) {
            Some(content_type) => {
                let content_type = content_type.to_str().unwrap_or_default();
                content_type.contains("text") || content_type.contains("javascript")
            }
            None => false,
        } {
            match to_bytes(body).await {
                Ok(content) => match String::from_utf8(content.to_vec()) {
                    Ok(text) => {
                        let text = text.replace(origin_body, new_body);
                        return Response::from_parts(parts, Body::from(text));
                    }
                    Err(_) => {
                        return Response::from_parts(parts, Body::from(content));
                    }
                },
                Err(err) => {
                    return Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(Body::from(err.to_string()))
                        .unwrap();
                }
            }
        } else {
            return Response::from_parts(parts, body);
        }
    }
}

#[derive(Clone)]
struct NoopMessageHandler {}

#[async_trait]
impl MessageHandler for NoopMessageHandler {
    async fn handle_message(&mut self, _ctx: &MessageContext, msg: Message) -> Option<Message> {
        Some(msg)
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut private_key_bytes: &[u8] = include_bytes!("../assets/ca/private.key");
    let mut ca_cert_bytes: &[u8] = include_bytes!("../assets/ca/cert.crt");
    let private_key = pemfile::pkcs8_private_keys(&mut private_key_bytes)
        .expect("Failed to parse private key")
        .remove(0);
    let ca_cert = pemfile::certs(&mut ca_cert_bytes)
        .expect("Failed to parse CA certificate")
        .remove(0);

    let ca = CertificateAuthority::new(private_key, ca_cert, 1_000)
        .expect("Failed to create Certificate Authority");

    let proxy_config = ProxyConfig {
        listen_addr: SocketAddr::from(([127, 0, 0, 1], 34567)),
        shutdown_signal: shutdown_signal(),
        http_handler: MitmHandler::default(),
        incoming_message_handler: NoopMessageHandler {},
        outgoing_message_handler: NoopMessageHandler {},
        upstream_proxy: None,
        ca,
    };

    if let Err(e) = start_proxy(proxy_config).await {
        error!("{}", e);
    }
}
