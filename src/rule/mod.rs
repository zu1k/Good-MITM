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
    pub filters: Vec<Filter>,
    pub actions: Vec<Action>,

    url: Option<String>,
}

impl From<file::Rule> for Rule {
    fn from(rule: file::Rule) -> Self {
        let filters: Vec<Filter> = rule.filters.into_iter().map(Filter::init).collect();

        {
            // append mitm list
            let mitm_list: Vec<String> = filters
                .iter()
                .filter_map(Filter::mitm_filtter_pattern)
                .collect();
            filter::mitm_list_append(mitm_list);

            let mitm_list = match rule.mitm_list {
                Some(s) => s.into_iter().collect(),
                None => vec![],
            };
            filter::mitm_list_append(mitm_list);
        }

        Self {
            filters,
            actions: rule.actions.to_vec(),
            url: None,
        }
    }
}

impl Rule {
    pub async fn do_req(&mut self, req: Request<Body>) -> RequestOrResponse {
        let url = req.uri().to_string();
        self.url = Some(url.clone());
        let mut tmp_req = req;

        for action in &self.actions {
            match action {
                Action::Reject => {
                    info!("[Reject] {}", url);
                    let res = Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(Body::default())
                        .unwrap();

                    return RequestOrResponse::Response(res);
                }

                Action::Redirect(target) => {
                    if target.contains('$') {
                        for filter in self.filters.clone() {
                            if let Filter::UrlRegex(re) = filter {
                                let target = filter::REGEX_CACHE
                                    .read()
                                    .unwrap()
                                    .get(&re)
                                    .unwrap()
                                    .replace(tmp_req.uri().to_string().as_str(), target.as_str())
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
                    }
                    if let Ok(target_url) = HeaderValue::from_str(target.as_str()) {
                        let mut res = Response::builder()
                            .status(StatusCode::FOUND)
                            .body(Body::default())
                            .unwrap();
                        res.headers_mut().insert(header::LOCATION, target_url);
                        info!("[Redirect] {} -> {}", url, target);
                        return RequestOrResponse::Response(res);
                    };
                }

                Action::ModifyRequest(modify) => {
                    info!("[ModifyRequest] {}", url);
                    match modify.modify_req(tmp_req).await {
                        Some(req) => tmp_req = req,
                        None => {
                            return RequestOrResponse::Response(
                                Response::builder()
                                    .status(StatusCode::BAD_REQUEST)
                                    .body(Body::default())
                                    .unwrap(),
                            );
                        }
                    }
                }

                Action::LogReq => {
                    info!("[LogRequest] {}", url);
                    action::log_req(&tmp_req).await;
                }

                _ => {}
            }
        }

        RequestOrResponse::Request(tmp_req)
    }

    pub async fn do_res(&self, res: Response<Body>) -> Response<Body> {
        let url = self.url.clone().unwrap_or_default();
        let mut tmp_res = res;

        for action in &self.actions {
            match action {
                Action::ModifyResponse(modify) => {
                    info!("[ModifyResponse] {}", url);
                    if tmp_res.headers().get(header::CONTENT_ENCODING).is_some() {
                        tmp_res = decode_response(tmp_res).unwrap()
                    };
                    tmp_res = modify.modify_res(tmp_res).await
                }
                Action::LogRes => {
                    info!("[LogResponse] {}", url);
                    action::log_res(&tmp_res).await;
                }
                _ => {}
            }
        }

        tmp_res
    }
}

pub fn match_rules(req: &Request<Body>) -> Vec<Rule> {
    let mut matched = vec![];
    let rules = RULES.read().unwrap();
    for rule in rules.iter() {
        for filter in &rule.filters {
            if filter.is_match_req(req) {
                matched.push(rule.clone());
            }
        }
    }
    matched
}

pub fn add_rules_from_fs<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let mut rules = RULES.write().unwrap();
    match file::read_rules_from_fs(path) {
        Ok(rules_config) => {
            let mut rules_tmp = rules_config.into_iter().map(Rule::from).collect();
            rules.append(&mut rules_tmp);

            Ok(())
        }
        Err(err) => Err(err),
    }
}
