use anyhow::{anyhow, Result};
use http::{header::HeaderName, Response};
use hyper::{body::*, Request};
use quick_js::{console::LogConsole, Context, JsValue};
use std::collections::HashMap;

pub async fn modify_req(code: &str, req: Request<Body>) -> Result<Request<Body>> {
    let (mut parts, body) = req.into_parts();
    let mut body_bytes = Bytes::default();

    let req_js = {
        let mut req_js = HashMap::new();

        // headers
        let headers = {
            let mut headers = HashMap::new();
            for (name, value) in &parts.headers {
                headers.insert(
                    name.to_string(),
                    JsValue::String(value.to_str().unwrap_or_default().to_owned()),
                );
            }
            headers
        };
        req_js.insert("headers".to_owned(), JsValue::Object(headers));

        // body text
        match to_bytes(body).await {
            Ok(content) => {
                body_bytes = content.clone();
                if let Ok(text) = String::from_utf8(content.to_vec()) {
                    req_js.insert("body".to_owned(), JsValue::String(text));
                } else {
                    req_js.insert("body".to_owned(), JsValue::Undefined);
                }
            }
            Err(_) => {
                req_js.insert("body".to_owned(), JsValue::Undefined);
            }
        }
        req_js.insert(
            "method".to_owned(),
            JsValue::String(parts.method.to_string()),
        );
        req_js.insert("url".to_owned(), JsValue::String(parts.uri.to_string()));

        JsValue::Object(req_js)
    };

    let context = Context::builder().console(LogConsole).build()?;

    let data = {
        let mut req = HashMap::new();
        req.insert("request".to_owned(), req_js);
        JsValue::Object(req)
    };
    context.set_global("data", data)?;

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
                return Ok(Request::from_parts(parts, Body::from(body)));
            } else {
                return Err(anyhow!("can not get js eval ret"));
            };
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn modify_res(code: &str, res: Response<Body>) -> Result<Response<Body>> {
    let (mut parts, body) = res.into_parts();
    let mut body_bytes = Bytes::default();

    let res_js = {
        let mut res_js = HashMap::new();

        // headers
        let headers = {
            let mut headers = HashMap::new();
            for (name, value) in &parts.headers {
                headers.insert(
                    name.to_string(),
                    JsValue::String(value.to_str().unwrap_or_default().to_owned()),
                );
            }
            headers
        };
        res_js.insert("headers".to_owned(), JsValue::Object(headers));

        // body text
        match to_bytes(body).await {
            Ok(content) => {
                body_bytes = content.clone();
                if let Ok(text) = String::from_utf8(content.to_vec()) {
                    res_js.insert("body".to_owned(), JsValue::String(text));
                } else {
                    res_js.insert("body".to_owned(), JsValue::Undefined);
                }
            }
            Err(_) => {
                res_js.insert("body".to_owned(), JsValue::Undefined);
            }
        }

        JsValue::Object(res_js)
    };

    let context = Context::builder().console(LogConsole).build()?;
    let data = {
        let mut req = HashMap::new();
        req.insert("response".to_owned(), res_js);
        JsValue::Object(req)
    };
    context.set_global("data", data)?;

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
                return Ok(Response::from_parts(parts, Body::from(body)));
            } else {
                return Err(anyhow!("can not get js eval ret"));
            };
        }
        Err(err) => Err(err.into()),
    }
}
