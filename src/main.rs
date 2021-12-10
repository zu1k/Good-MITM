#[macro_use]
extern crate lazy_static;

mod ca;
mod handler;
mod mitm;
mod rule;

use clap::Parser;
use log::*;
use mitm::*;
use rustls_pemfile as pemfile;
use std::fs;

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[derive(Parser)]
#[clap(name = "Good Man in the Middle", version, about, author)]
struct AppOpts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    /// run proxy serve
    Run(Run),
    /// gen your own ca cert and private key
    Genca,
}

#[derive(Parser)]
struct Run {
    #[clap(
        short,
        long,
        default_value = "ca/private.key",
        help = "private key file path"
    )]
    key: String,
    #[clap(short, long, default_value = "ca/cert.crt", help = "cert file path")]
    cert: String,
    #[clap(short, long, help = "load rules from file or dir")]
    rule: String,
    #[clap(short, long, default_value = "127.0.0.1:34567", help = "bind address")]
    bind: String,
}

fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();

    let opts = AppOpts::parse();
    match opts.subcmd {
        SubCommand::Run(opts) => {
            if let Err(err) = rule::add_rules_from_fs(opts.rule) {
                error!("parse rule file failed, err: {}", err);
                std::process::exit(3);
            }
            run(&opts.key, &opts.cert, &opts.bind);
        }
        SubCommand::Genca => ca::gen_ca(),
    }
}

#[tokio::main]
async fn run(key_path: &str, cert_path: &str, bind: &str) {
    let private_key_bytes = fs::read(key_path).expect("ca private key file path not valid!");
    let ca_cert_bytes = fs::read(cert_path).expect("ca cert file path not valid!");

    let private_key = pemfile::pkcs8_private_keys(&mut private_key_bytes.as_slice())
        .expect("Failed to parse private key");

    let private_key = rustls::PrivateKey(private_key[0].clone());
    let ca_cert =
        pemfile::certs(&mut ca_cert_bytes.as_slice()).expect("Failed to parse CA certificate");
    let ca_cert = rustls::Certificate(ca_cert[0].clone());

    let ca = CertificateAuthority::new(
        private_key,
        ca_cert,
        String::from_utf8(ca_cert_bytes).unwrap(),
        1_000,
    )
    .expect("Failed to create Certificate Authority");

    let proxy_config = ProxyConfig {
        listen_addr: bind.parse().expect("bind address not valid!"),
        shutdown_signal: shutdown_signal(),
        http_handler: handler::MitmHandler::default(),
        message_handler: crate::mitm::NoopMessageHandler::new(),
        mitm_filter: rule::filter::MitmFilter::default(),
        upstream_proxy: None,
        ca,
    };

    if let Err(e) = start_proxy(proxy_config).await {
        error!("{}", e);
    }
}
