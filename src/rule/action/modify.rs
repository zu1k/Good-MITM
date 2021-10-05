use http_mitm::hyper::{body::*, header, Body, HeaderMap, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BodyModify {
    pub origin: String,
    pub new: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct HeaderModify {
    pub origin: String,
    pub new: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Modify {
    Header(HeaderModify),
    Cookie,
    Body(BodyModify),
}

impl Modify {
    pub async fn modify_req(&self, req: Request<Body>) -> Option<Request<Body>> {
        match self {
            Modify::Body(bm) => {
                let origin = bm.origin.as_str();
                let new = bm.new.as_str();

                let (parts, body) = req.into_parts();
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
                                let text = text.replace(origin, new);
                                Some(Request::from_parts(parts, Body::from(text)))
                            }
                            Err(_) => Some(Request::from_parts(parts, Body::from(content))),
                        },
                        // req body read failed
                        Err(_) => None,
                    }
                } else {
                    Some(Request::from_parts(parts, body))
                }
            }
            Modify::Header(hm) => {
                let mut req = req;
                self.modify_header(req.headers_mut(), hm);
                Some(req)
            }
            _ => Some(req),
        }
    }

    pub async fn modify_res(&self, res: Response<Body>) -> Response<Body> {
        match self {
            Self::Body(bm) => {
                let origin = bm.origin.as_str();
                let new = bm.new.as_str();

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
                                let text = text.replace(origin, new);
                                Response::from_parts(parts, Body::from(text))
                            }
                            Err(_) => Response::from_parts(parts, Body::from(content)),
                        },
                        Err(err) => Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .body(Body::from(err.to_string()))
                            .unwrap(),
                    }
                } else {
                    Response::from_parts(parts, body)
                }
            }
            Modify::Header(hm) => {
                let mut res = res;
                self.modify_header(res.headers_mut(), hm);
                res
            }
            _ => res,
        }
    }

    fn modify_header(&self, header: &mut HeaderMap, hm: &HeaderModify) {
        let origin = hm.origin.as_str();
        let new = hm.new.as_str();
        for value in header.values_mut() {
            let text = value.to_str().unwrap().to_string();
            if text.contains(origin) {
                let text = text.replace(origin, new);
                *value = header::HeaderValue::from_str(text.as_str()).unwrap();
            }
        }
    }
}
