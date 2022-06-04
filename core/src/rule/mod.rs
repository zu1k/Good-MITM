use hyper::{header, header::HeaderValue, Body, Request, Response, StatusCode};
use log::*;
use std::vec::Vec;

use crate::{cache, mitm::RequestOrResponse};
pub use action::Action;
pub use filter::Filter;

mod action;
mod filter;

#[derive(Debug, Clone)]
pub struct Rule {
    pub filters: Vec<Filter>,
    pub actions: Vec<Action>,

    pub url: Option<String>,
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
                                let target = cache::get_regex(&re)
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
                        Some(new_req) => tmp_req = new_req,
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

                Action::Js(code) => {
                    info!("[LogRequest] {}", url);
                    if let Ok(req) = action::js::modify_req(code, tmp_req).await {
                        return RequestOrResponse::Request(req);
                    } else {
                        return RequestOrResponse::Response(
                            Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::default())
                                .unwrap(),
                        );
                    }
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
                    tmp_res = modify.modify_res(tmp_res).await
                }
                Action::LogRes => {
                    info!("[LogResponse] {}", url);
                    action::log_res(&tmp_res).await;
                }

                Action::Js(code) => {
                    info!("[LogResponse] {}", url);
                    if let Ok(res) = action::js::modify_res(code, tmp_res).await {
                        return res;
                    } else {
                        return Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::default())
                            .unwrap();
                    }
                }
                _ => {}
            }
        }

        tmp_res
    }
}
