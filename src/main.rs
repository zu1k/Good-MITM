#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

mod ca;
mod handler;
mod rule;

use clap::{App, Arg, SubCommand};
use http_mitm::*;
use log::*;
use rustls_pemfile as pemfile;
use std::fs;

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
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
        incoming_message_handler: handler::NoopMessageHandler {},
        outgoing_message_handler: handler::NoopMessageHandler {},
        mitm_filter: rule::filter::MitmFilter {},
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
        .subcommand(
            SubCommand::with_name("run")
                .about("start to run")
                .display_order(1)
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
                        .help("rule file or dir")
                        .long_help("load rules from file or dir")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("bind")
                        .short("b")
                        .long("bind")
                        .help("bind address")
                        .long_help("bind address")
                        .default_value("127.0.0.1:34567")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("genca")
                .display_order(2)
                .about("generate your own ca private key and certificate"),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some("run") => {
            let matches = matches.subcommand_matches("run").unwrap();
            let rule_file_or_dir = matches
                .value_of("rule")
                .expect("rule file path should not be none");
            let bind = matches
                .value_of("bind")
                .expect("bind address should not be none");
            if let Err(err) = rule::add_rules_from_file_or_dir(rule_file_or_dir) {
                error!("parse rule file failed, err: {}", err);
                std::process::exit(3);
            }

            let key_path = matches
                .value_of("key")
                .expect("need root ca private key file");

            let cert_path = matches.value_of("cert").expect("need root ca cert file");

            run(key_path, cert_path, bind)
        }
        Some("genca") => ca::gen_ca(),
        _ => {
            println!("subcommand not valid!")
        }
    }
}
