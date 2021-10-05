mod action;
mod file;
pub mod filter;

use action::Action;
use filter::Filter;
use http_mitm::{
    decode_response,
    hyper::{header, header::HeaderValue, Body, Request, Response, StatusCode},
    RequestOrResponse,
};
use log::*;
use std::{path::Path, sync::RwLock, vec::Vec};

lazy_static! {
    static ref RULES: RwLock<Vec<Rule>> = RwLock::from(Vec::new());
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub filter: Filter,
    pub action: Action,

    url: Option<String>,
}

impl From<file::Rule> for Rule {
    fn from(rule: file::Rule) -> Self {
        let filter = match rule.filter {
            file::Filter::Domain(s) => Filter::new_domain(s.as_str()),
            file::Filter::DomainKeyword(s) => Filter::new_domain_keyword(s.as_str()),
            file::Filter::DomainPrefix(s) => Filter::new_domain_prefix(s.as_str()),
            file::Filter::DomainSuffix(s) => Filter::new_domain_suffix(s.as_str()),
            file::Filter::UrlRegex(re) => Filter::new_url_regex(re.as_str()),
        };
        Self {
            filter,
            action: rule.action,
            url: None,
        }
    }
}

impl Rule {
    pub async fn do_req(&mut self, req: Request<Body>) -> RequestOrResponse {
        let url = req.uri().to_string();
        self.url = Some(url.clone());
        match self.action.clone() {
            Action::Reject => {
                info!("[Reject] {}", url);
                let res = Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::default())
                    .unwrap();
                RequestOrResponse::Response(res)
            }
            Action::Redirect(target) => {
                if target.contains('$') {
                    if let Filter::UrlRegex(re) = self.filter.clone() {
                        let target = re
                            .replace(req.uri().to_string().as_str(), target.as_str())
                            .to_string();
                        if let Ok(target_url) = HeaderValue::from_str(target.as_str()) {
                            let mut res = Response::builder()
                                .status(StatusCode::FOUND)
                                .body(Body::default())
                                .unwrap();
                            res.headers_mut().insert(header::LOCATION, target_url);
                            info!("[Redirect] {} -> {}", url, target);
                            return RequestOrResponse::Response(res);
                        }
                    }
                }
                return match HeaderValue::from_str(target.as_str()) {
                    Ok(target_url) => {
                        let mut res = Response::builder()
                            .status(StatusCode::FOUND)
                            .body(Body::default())
                            .unwrap();
                        res.headers_mut().insert(header::LOCATION, target_url);
                        info!("[Redirect] {} -> {}", url, target);
                        RequestOrResponse::Response(res)
                    }
                    Err(_) => RequestOrResponse::Request(req),
                };
            }
            Action::ModifyRequest(modify) => {
                info!("[ModifyRequest] {}", url);
                match modify.modify_req(req).await {
                    Some(req) => RequestOrResponse::Request(req),
                    None => RequestOrResponse::Response(
                        Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::default())
                            .unwrap(),
                    ),
                }
            }
            _ => RequestOrResponse::Request(req),
        }
    }

    pub async fn do_res(&self, res: Response<Body>) -> Response<Body> {
        let url = self.url.clone().unwrap_or_default();
        match self.action.clone() {
            Action::ModifyResponse(modify) => {
                info!("[ModifyResponse] {}", url);
                let res = if res.headers().get(header::CONTENT_ENCODING).is_some() {
                    decode_response(res).unwrap()
                } else {
                    res
                };
                modify.modify_res(res).await
            }
            _ => res,
        }
    }
}

pub fn match_rule(req: &Request<Body>) -> Option<Rule> {
    let rules = RULES.read().unwrap();
    for rule in rules.iter() {
        if rule.filter.is_match_req(req) {
            return Some(rule.clone());
        }
    }
    None
}

pub fn add_rule_file<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let mut rules = RULES.write().unwrap();
    match file::read_rules_from_file(path) {
        Ok(rules_config) => {
            for rule in rules_config {
                rules.push(Rule::from(rule));
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}
