use crate::{
    mitm::hyper::{body::*, header, Body, HeaderMap, Request, Response, StatusCode},
    utils::{cache, SingleOrMulti},
};
use cookie::{Cookie, CookieJar};
use enum_dispatch::enum_dispatch;
use fancy_regex::Regex;
use http::HeaderValue;
use serde::{Deserialize, Serialize};

#[enum_dispatch]
pub trait Replacer {
    fn replace(&self, origin: &str) -> String;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Replace {
    pub origin: Option<String>,
    pub new: String,
}

impl Replacer for Replace {
    fn replace(&self, origin: &str) -> String {
        match self.origin.clone() {
            Some(ref o) => origin.replace(o, &self.new),
            None => self.new.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RegexReplace {
    pub re: String,
    pub new: String,
}

impl Replacer for RegexReplace {
    fn replace(&self, origin: &str) -> String {
        if let Some(re) = cache::REGEX.read().unwrap().get(&self.re) {
            return re.replace_all(origin, &self.new).to_string();
        }
        let re = Regex::new(&self.re).unwrap();
        cache::REGEX
            .write()
            .unwrap()
            .insert(self.re.clone(), re.clone());
        re.replace(origin, &self.new).to_string()
    }
}

#[enum_dispatch(Replacer)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type")]
pub enum TextModify {
    #[serde(rename = "plain")]
    Replace,
    #[serde(rename = "regex")]
    RegexReplace,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CookieModify {
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub remove: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Modify {
    Header(TextModify),
    #[serde(alias = "cookie")]
    Cookies(SingleOrMulti<CookieModify>),
    Body(TextModify),
}

impl Modify {
    pub async fn modify_req(&self, req: Request<Body>) -> Option<Request<Body>> {
        match self {
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
                                let text = bm.replace(&text);
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
            Modify::Cookies(cookies_mod) => {
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

                for c in cookies_mod.clone().into_iter() {
                    if c.remove {
                        cookies_jar.remove(Cookie::named(c.name))
                    } else {
                        cookies_jar.add(Cookie::new(c.name, c.value))
                    }
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
            Self::Body(bm) => {
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
                                let text = bm.replace(&text);
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
            Modify::Cookies(cookies_mod) => {
                let mut res = res;

                let mut cookies_jar = CookieJar::new();
                if let Some(cookies) = res.headers().get(header::COOKIE) {
                    let cookies = cookies.to_str().unwrap().to_string();
                    let cookies: Vec<String> = cookies.split("; ").map(String::from).collect();
                    for c in cookies {
                        if let Ok(c) = Cookie::parse(c) {
                            cookies_jar.add(c);
                        }
                    }
                }

                let mut set_cookies_jar = CookieJar::new();
                let set_cookies = res.headers().get_all(header::SET_COOKIE);
                for sc in set_cookies {
                    let sc = sc.to_str().unwrap().to_string();
                    if let Ok(c) = Cookie::parse(sc) {
                        set_cookies_jar.add(c)
                    }
                }

                for c in cookies_mod.clone().into_iter() {
                    if c.remove {
                        cookies_jar.remove(Cookie::named(c.name.clone()));
                        set_cookies_jar.remove(Cookie::named(c.name));
                    } else {
                        let c = Cookie::new(c.name, c.value);
                        cookies_jar.add(c.clone());
                        set_cookies_jar.add(c.clone());
                    }
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
        }
    }

    fn modify_header(&self, header: &mut HeaderMap, md: &TextModify) {
        for value in header.values_mut() {
            let text = value.to_str().unwrap().to_string();
            let new = md.replace(&text);
            if new != text {
                *value = header::HeaderValue::from_str(new.as_str()).unwrap();
            }
        }
    }
}
