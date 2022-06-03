use hyper::{header, Body, Request, Response};
use log::info;
use std::sync::{Arc, RwLock};
use wildmatch::WildMatch;

use crate::{
    mitm::{HttpContext, RequestOrResponse},
    rule::Rule,
};

#[derive(Clone)]
pub struct HttpHandler {
    rules: Arc<Vec<Rule>>,
}

impl HttpHandler {
    pub fn new(rules: Arc<Vec<Rule>>) -> Self {
        Self { rules }
    }

    fn match_rules(&self, req: &Request<Body>) -> Vec<Rule> {
        let mut matched = vec![];
        for rule in self.rules.iter() {
            for filter in &rule.filters {
                if filter.is_match_req(req) {
                    matched.push(rule.clone());
                }
            }
        }
        matched
    }

    pub async fn handle_request(
        &self,
        ctx: &mut HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        ctx.uri = Some(req.uri().clone());

        // remove accept-encoding to avoid encoded body
        let mut req = req;
        req.headers_mut().remove(header::ACCEPT_ENCODING);

        let rules = self.match_rules(&req);
        if !rules.is_empty() {
            ctx.should_modify_response = true;
        }

        for mut rule in rules {
            ctx.rule.push(rule.clone());
            let rt = rule.do_req(req).await;
            if let RequestOrResponse::Request(r) = rt {
                req = r;
            } else {
                return rt;
            }
        }

        RequestOrResponse::Request(req)
    }

    pub async fn handle_response(
        &self,
        ctx: &mut HttpContext,
        res: Response<Body>,
    ) -> Response<Body> {
        if !ctx.should_modify_response || ctx.rule.is_empty() {
            return res;
        }
        let uri = ctx.uri.as_ref().unwrap();
        let content_type = match res.headers().get(header::CONTENT_TYPE) {
            Some(content_type) => content_type.to_str().unwrap_or_default(),
            None => "unknown",
        };
        info!(
            "[Response] {} {} {}",
            res.status(),
            uri.host().unwrap_or_default(),
            content_type
        );

        let mut res = res;
        let rules = ctx.rule.clone();
        for rule in rules {
            res = rule.do_res(res).await;
        }
        res
    }
}

#[derive(Clone, Default)]
pub struct MitmFilter {
    filters: Arc<RwLock<Vec<WildMatch>>>,
}

impl MitmFilter {
    pub fn new(filters: Arc<RwLock<Vec<WildMatch>>>) -> Self {
        Self { filters }
    }

    pub async fn filter(&self, _ctx: &HttpContext, req: &Request<Body>) -> bool {
        let host = req.uri().host().unwrap_or_default();
        let list = self.filters.read().unwrap();
        for m in list.iter() {
            if m.matches(host) {
                return true;
            }
        }
        false
    }
}
