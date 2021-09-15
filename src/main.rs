#[macro_use]
extern crate lazy_static;

mod action;
mod filter;
mod handler;
mod rule;

use clap::{App, Arg};
use hudsucker::{rustls::internal::pemfile, *};
use log::*;
use std::fs;
use std::net::SocketAddr;

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tokio::main]
async fn run(key_path: &str, cert_path: &str) {
    let private_key_bytes = fs::read(key_path).expect("ca private key file path not valid!");
    let ca_cert_bytes = fs::read(cert_path).expect("ca cert file path not valid!");

    let private_key = pemfile::pkcs8_private_keys(&mut private_key_bytes.as_slice())
        .expect("Failed to parse private key")
        .remove(0);
    let ca_cert = pemfile::certs(&mut ca_cert_bytes.as_slice())
        .expect("Failed to parse CA certificate")
        .remove(0);

    let ca = CertificateAuthority::new(private_key, ca_cert, 1_000)
        .expect("Failed to create Certificate Authority");

    let proxy_config = ProxyConfig {
        listen_addr: SocketAddr::from(([127, 0, 0, 1], 34567)),
        shutdown_signal: shutdown_signal(),
        http_handler: handler::MitmHandler::default(),
        incoming_message_handler: handler::NoopMessageHandler {},
        outgoing_message_handler: handler::NoopMessageHandler {},
        upstream_proxy: None,
        ca,
    };

    if let Err(e) = start_proxy(proxy_config).await {
        error!("{}", e);
    }
}

fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();

    let matches = App::new("Good Man in the Middle")
        .author("zu1k <i@lgf.im>")
        .about("Use MITM technology to provide features like rewrite, redirect.")
        .arg(
            Arg::with_name("key")
                .short("k")
                .long("key")
                .alias("private")
                .help("private key file path")
                .long_help("private key file path")
                .default_value("ca/private.key")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("cert")
                .short("c")
                .long("cert")
                .help("cert file path")
                .long_help("cert file path")
                .default_value("ca/cert.crt")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("rule")
                .short("r")
                .long("rule")
                .help("rule file")
                .long_help("load rules from file")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let rule_file = matches
        .value_of("rule")
        .expect("rule file path should not be none");
    if let Err(err) = rule::add_rule_file(rule_file) {
        error!("parse rule file failed, err: {}", err);
        std::process::exit(3);
    }

    let key_path = matches
        .value_of("key")
        .expect("need root ca private key file");
    let cert_path = matches.value_of("cert").expect("need root ca cert file");

    run(key_path, cert_path)
}
