#![allow(dead_code)]

use anyhow::{Ok, Result};
use clap::Parser;
use core::{CertificateAuthority, Proxy};
use hyper_proxy::Intercept;
use log::*;
use rustls_pemfile as pemfile;
use std::fs;

mod ca;
mod error;
mod file;

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
    Genca(Genca),
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
    #[clap(short, long, help = "upstream proxy")]
    proxy: Option<String>,
}

#[derive(Parser)]
struct Genca {
    #[clap(short, long, help = "install cert on your trust zone")]
    trust: bool,
}

fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();

    let opts = AppOpts::parse();
    match opts.subcmd {
        SubCommand::Run(opts) => {
            run(&opts).unwrap();
        }
        SubCommand::Genca(opts) => {
            let cert = ca::gen_ca();
            if opts.trust {
                trust_cert::trust_cert(cert);
            }
        }
    }
}

#[tokio::main]
async fn run(opts: &Run) -> Result<()> {
    info!("CA Private key use: {}", opts.key);
    let private_key_bytes = fs::read(&opts.key).expect("ca private key file path not valid!");
    let private_key = pemfile::pkcs8_private_keys(&mut private_key_bytes.as_slice())
        .expect("Failed to parse private key");
    let private_key = rustls::PrivateKey(private_key[0].clone());

    info!("CA Certificate use: {}", opts.cert);
    let ca_cert_bytes = fs::read(&opts.cert).expect("ca cert file path not valid!");
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

    info!("Http Proxy listen on: http://{}", opts.bind);

    let (rules, mitm_filters) = file::load_rules_amd_mitm_filters(&opts.rule)?;

    let proxy = Proxy::builder()
        .ca(ca)
        .listen_addr(opts.bind.parse().expect("bind address not valid!"))
        .upstream_proxy(
            opts.proxy
                .clone()
                .map(|proxy| hyper_proxy::Proxy::new(Intercept::All, proxy.parse().unwrap())),
        )
        .shutdown_signal(shutdown_signal())
        .mitm_filters(mitm_filters)
        .rules(rules)
        .build();
    proxy.start_proxy().await?;
    Ok(())
}
