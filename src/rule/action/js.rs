use anyhow::{anyhow, Result};
use http::{header::HeaderName, Response};
use hyper::{
    body::{to_bytes, Body, Bytes},
    Request,
};
use quick_js::{console::LogConsole, Context, JsValue};
use std::collections::HashMap;

macro_rules! to_js_value_map {
    ($parts:ident, $body_bytes:ident) => {{
        let mut req_js = HashMap::new();

        // headers
        let headers = {
            let mut headers = HashMap::new();
            for (name, value) in &$parts.headers {
                headers.insert(
                    name.to_string(),
                    JsValue::String(value.to_str().unwrap_or_default().to_owned()),
                );
            }
            headers
        };
        req_js.insert("headers".to_owned(), JsValue::Object(headers));

        // body text
        if let Ok(text) = String::from_utf8($body_bytes.to_vec()) {
            req_js.insert("body".to_owned(), JsValue::String(text));
        } else {
            req_js.insert("body".to_owned(), JsValue::Undefined);
        }

        req_js
    }};
}

pub async fn modify_req(code: &str, req: Request<Body>) -> Result<Request<Body>> {
    let (mut parts, body) = req.into_parts();
    let body_bytes = to_bytes(body).await.unwrap_or_default();
    let mut req_js = to_js_value_map!(parts, body_bytes);
    req_js.insert(
        "method".to_owned(),
        JsValue::String(parts.method.to_string()),
    );
    req_js.insert("url".to_owned(), JsValue::String(parts.uri.to_string()));
    let mut data = HashMap::new();
    data.insert("request".to_owned(), JsValue::Object(req_js));

    let context = Context::builder().console(LogConsole).build()?;
    context.set_global("data", JsValue::Object(data))?;
    match context.eval(code) {
        Ok(req_js) => {
            if let JsValue::Object(req_js) = req_js {
                if let Some(JsValue::Object(headers)) = req_js.get("headers") {
                    for (key, value) in headers {
                        if let JsValue::String(value) = value {
                            parts.headers.insert(
                                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                                value.parse().unwrap(),
                            );
                        }
                    }
                }

                if let Some(JsValue::String(url)) = req_js.get("url") {
                    parts.uri = url.parse().unwrap();
                }

                let body = if let Some(JsValue::String(body)) = req_js.get("body") {
                    Bytes::from(body.to_owned())
                } else {
                    body_bytes
                };
                Ok(Request::from_parts(parts, Body::from(body)))
            } else {
                Err(anyhow!("can not get js eval ret"))
            }
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn modify_res(code: &str, res: Response<Body>) -> Result<Response<Body>> {
    let (mut parts, body) = res.into_parts();
    let body_bytes = to_bytes(body).await.unwrap_or_default();
    let res_js = to_js_value_map!(parts, body_bytes);
    let mut data = HashMap::new();
    data.insert("response".to_owned(), JsValue::Object(res_js));

    let context = Context::builder().console(LogConsole).build()?;
    context.set_global("data", JsValue::Object(data))?;
    match context.eval(code) {
        Ok(req_js) => {
            if let JsValue::Object(req_js) = req_js {
                if let Some(JsValue::Object(headers)) = req_js.get("headers") {
                    for (key, value) in headers {
                        if let JsValue::String(value) = value {
                            parts.headers.insert(
                                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                                value.parse().unwrap(),
                            );
                        }
                    }
                }

                let body = if let Some(JsValue::String(body)) = req_js.get("body") {
                    Bytes::from(body.to_owned())
                } else {
                    body_bytes
                };
                Ok(Response::from_parts(parts, Body::from(body)))
            } else {
                Err(anyhow!("can not get js eval ret"))
            }
        }
        Err(err) => Err(err.into()),
    }
}
