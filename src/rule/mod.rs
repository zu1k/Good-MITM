mod action;
mod file;
mod filter;

use action::Action;
use filter::Filter;
use hudsucker::hyper::Body;
use hudsucker::hyper::Request;
use hudsucker::hyper::{header, header::HeaderValue, Response, StatusCode};
use hudsucker::RequestOrResponse;
use log::*;
use std::path::Path;
use std::sync::RwLock;
use std::vec::Vec;

lazy_static! {
    static ref RULES: RwLock<Vec<Rule>> = RwLock::from(Vec::new());
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub filter: Filter,
    pub action: Action,
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
        }
    }
}

impl Rule {
    pub async fn do_req(&self, req: Request<Body>) -> RequestOrResponse {
        let url = req.uri().to_string();
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
            Action::Modify(modify) => {
                info!("[Modify] {}", url);
                RequestOrResponse::Request(modify.modify_req(req).await)
            }
        }
    }

    pub async fn do_res(&self, res: Response<Body>) -> Response<Body> {
        match self.action.clone() {
            Action::Modify(modify) => modify.modify_res(res).await,
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
