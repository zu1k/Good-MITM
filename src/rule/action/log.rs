use http_mitm::hyper::{Body, Request, Response};
use log::info;
use std::fmt::Write;

pub async fn log_req(req: &Request<Body>) {
    let headers = req.headers();
    let mut header_formated = String::new();
    for (key, value) in headers {
        let v = match value.to_str() {
            Ok(v) => v.to_string(),
            Err(_) => {
                format!("[u8]; {}", value.len())
            }
        };
        write!(
            &mut header_formated,
            "\t{:<20}{}\r\n",
            format!("{}:", key.as_str()),
            v
        )
        .unwrap();
    }

    info!(
        "{} {}
Headers:
{}",
        req.method(),
        req.uri().to_string(),
        header_formated
    )
}

pub async fn log_res(res: &Response<Body>) {
    let headers = res.headers();
    let mut header_formated = String::new();
    for (key, value) in headers {
        let v = match value.to_str() {
            Ok(v) => v.to_string(),
            Err(_) => {
                format!("[u8]; {}", value.len())
            }
        };
        write!(
            &mut header_formated,
            "\t{:<20}{}\r\n",
            format!("{}:", key.as_str()),
            v
        )
        .unwrap();
    }

    info!(
        "{} {:?}
Headers:
{}",
        res.status(),
        res.version(),
        header_formated
    )
}
