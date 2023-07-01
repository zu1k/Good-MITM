use cookie::{Cookie, CookieJar};
use http::{header::HeaderName, HeaderValue, Uri};
use hyper::{body::*, header, Body, HeaderMap, Request, Response, StatusCode};
use log::error;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::cache::get_regex;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TextModify {
    Set(String),
    Complex(TextModifyComplex),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TextModifyComplex {
    pub origin: Option<String>,
    pub re: Option<String>,
    pub new: String,
}

impl TextModify {
    fn exec_action(&self, text: &str) -> String {
        match self {
            TextModify::Set(new) => new.to_string(),
            TextModify::Complex(md) => {
                if let Some(ref origin) = md.origin {
                    return text.replace(origin, &md.new);
                }

                if let Some(ref re) = md.re {
                    return get_regex(re).replace_all(text, &md.new).to_string();
                }

                md.new.clone()
            }
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MapModify {
    pub key: String,
    #[serde(default)]
    pub value: Option<TextModify>,
    #[serde(default)]
    pub remove: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Modify {
    Url(TextModify),
    Header(MapModify),
    Cookie(MapModify),
    Body(TextModify),
}

impl Modify {
    pub async fn modify_req(&self, mut req: Request<Body>) -> Option<Request<Body>> {
        match self {
            Modify::Url(md) => {
                let origin = req.uri().to_string();
                let new_url = md.exec_action(&origin);
                match Uri::from_str(&new_url) {
                    Ok(uri) => *req.uri_mut() = uri,
                    Err(err) => error!("url modify error: {}", err),
                }
                Some(req)
            }
            Modify::Body(bm) => {
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
                                let text = bm.exec_action(&text);
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
            Modify::Cookie(md) => {
                let mut req = req;
                let mut cookies_jar = CookieJar::new();

                if let Some(cookies) = req.headers().get(header::COOKIE) {
                    let cookies = cookies.to_str().unwrap().to_string();
                    let cookies: Vec<String> = cookies.split("; ").map(String::from).collect();
                    for c in cookies {
                        if let Ok(c) = Cookie::parse(c) {
                            cookies_jar.add(c);
                        }
                    }
                }

                if md.remove {
                    cookies_jar.remove(Cookie::named(md.key.clone()))
                } else {
                    let new_cookie_value = md
                        .value
                        .to_owned()
                        .map(|text_md| {
                            let origin_cookie_value = cookies_jar
                                .get(&md.key)
                                .map(|c| c.value().to_string())
                                .unwrap_or_default();
                            text_md.exec_action(&origin_cookie_value)
                        })
                        .unwrap_or_default();
                    cookies_jar.add(Cookie::new(md.key.clone(), new_cookie_value))
                }

                let cookies: Vec<String> = cookies_jar.iter().map(|c| c.to_string()).collect();
                let cookies = cookies.join("; ");
                req.headers_mut()
                    .insert(header::COOKIE, HeaderValue::from_str(&cookies).unwrap());

                Some(req)
            }
        }
    }

    pub async fn modify_res(&self, res: Response<Body>) -> Response<Body> {
        match self {
            Modify::Body(bm) => {
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
                                let text = bm.exec_action(&text);
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
            Modify::Header(md) => {
                let mut res = res;
                self.modify_header(res.headers_mut(), md);
                res
            }
            Modify::Cookie(md) => {
                let mut res = res;

                let mut cookies_jar = CookieJar::new();
                if let Some(cookies) = res.headers().get(header::COOKIE) {
                    let cookies = String::from_utf8_lossy(cookies.as_bytes()).to_string();
                    Cookie::split_parse(cookies)
                        .filter_map(Result::ok)
                        .for_each(|cookie| cookies_jar.add(cookie));
                }

                let mut set_cookies_jar = CookieJar::new();
                let set_cookies = res.headers().get_all(header::SET_COOKIE);
                for sc in set_cookies {
                    if let Ok(c) = Cookie::parse(String::from_utf8_lossy(sc.as_bytes()).to_string())
                    {
                        set_cookies_jar.add(c);
                    }
                }

                if md.remove {
                    cookies_jar.remove(Cookie::named(md.key.clone()));
                    set_cookies_jar.remove(Cookie::named(md.key.clone()));
                } else {
                    let new_cookie_value = md
                        .value
                        .to_owned()
                        .map(|text_md| {
                            let origin_cookie_value = cookies_jar
                                .get(&md.key)
                                .map(|c| c.value().to_string())
                                .or_else(|| {
                                    set_cookies_jar.get(&md.key).map(|c| c.value().to_string())
                                })
                                .unwrap_or_default();
                            text_md.exec_action(&origin_cookie_value)
                        })
                        .unwrap_or_default();

                    let c = Cookie::new(md.key.clone(), new_cookie_value);
                    cookies_jar.add(c.clone());
                    set_cookies_jar.add(c.clone());
                }

                let cookies: Vec<String> = cookies_jar.iter().map(|c| c.to_string()).collect();
                let cookies = cookies.join("; ");
                let header = res.headers_mut();
                header.insert(header::COOKIE, HeaderValue::from_str(&cookies).unwrap());

                header.remove(header::SET_COOKIE);
                for sc in set_cookies_jar.iter() {
                    header.append(
                        header::SET_COOKIE,
                        HeaderValue::from_str(&sc.to_string()).unwrap(),
                    );
                }

                res
            }
            Modify::Url(_) => {
                error!("modify response url not supported");
                res
            }
        }
    }

    fn modify_header<'a>(&self, header: &mut HeaderMap, md: &'a MapModify) {
        if md.remove {
            header.remove(&md.key);
        } else if let Some(ref text_md) = md.value {
            if let Some(h) = header.get_mut(&md.key) {
                let new_header_value = text_md.exec_action(h.to_str().unwrap_or_default());
                *h = header::HeaderValue::from_str(new_header_value.as_str()).unwrap();
            } else {
                let new_header_value = text_md.exec_action("");
                header.append(
                    HeaderName::from_str(&md.key).unwrap(),
                    header::HeaderValue::from_str(new_header_value.as_str()).unwrap(),
                );
            }
        }
    }
}
