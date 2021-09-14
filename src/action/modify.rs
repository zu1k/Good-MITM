use hudsucker::hyper::{body::*, header, Body, Request, Response, StatusCode};

#[derive(Debug, Clone)]
pub struct BodyModify {
    pub origin: String,
    pub new: String,
}

impl BodyModify {
    pub fn new(origin: &str, new: &str) -> Self {
        Self {
            origin: String::from(origin),
            new: String::from(new),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Modify {
    Header,
    Cookie,
    Body(BodyModify),
}

impl Modify {
    pub fn new_modify_body(origin: &str, new: &str) -> Self {
        Self::Body(BodyModify::new(origin, new))
    }

    pub async fn modify_req(&self, req: Request<Body>) -> Request<Body> {
        match self {
            Modify::Body(bm) => req,
            _ => req,
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
            _ => res,
        }
    }
}
