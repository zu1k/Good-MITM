use hudsucker::{
    async_trait::async_trait,
    hyper::{header, Body, Request, Response},
    rustls::internal::pemfile,
    tungstenite::Message,
    *,
};

mod action;
mod filter;
use action::Action;
use filter::Filter;
use log::*;
use std::net::SocketAddr;

use crate::action::Modify;

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[derive(Clone, Default)]
struct MitmHandler {
    should_modify_response: bool,
}

#[async_trait]
impl HttpHandler for MitmHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        println!("{:?}", req.uri().to_string());
        let filter = Filter::new_domain("www.nfmovies.com");
        if filter.is_match_req(&req) {
            self.should_modify_response = true;
            // let action = Action::Redirect("https://lgf.im/".to_string());
            // return action.do_req(req);
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
        let modifier = Modify::new_modify_body(origin_body, new_body);

        // println!("{:?}", res);
        modifier.modify_res(res).await
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
