use crate::Rule;
use async_trait::async_trait;
use hyper::{header, Body, Request, Response};
use log::info;
use mitm_core::{
    handler::{CustomContextData, HttpHandler},
    mitm::{HttpContext, RequestOrResponse},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct RuleHttpHandler {
    rules: Arc<Vec<Rule>>,
}

#[derive(Default, Clone)]
pub struct RuleHandlerCtx {
    rules: Vec<Rule>,
}

impl CustomContextData for RuleHandlerCtx {}

impl RuleHttpHandler {
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
}

#[async_trait]
impl HttpHandler<RuleHandlerCtx> for RuleHttpHandler {
    async fn handle_request(
        &self,
        ctx: &mut HttpContext<RuleHandlerCtx>,
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
            ctx.custom_data.rules.push(rule.clone());
            let rt = rule.do_req(req).await;
            if let RequestOrResponse::Request(r) = rt {
                req = r;
            } else {
                return rt;
            }
        }

        RequestOrResponse::Request(req)
    }

    async fn handle_response(
        &self,
        ctx: &mut HttpContext<RuleHandlerCtx>,
        res: Response<Body>,
    ) -> Response<Body> {
        if !ctx.should_modify_response || ctx.custom_data.rules.is_empty() {
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
        for rule in &ctx.custom_data.rules {
            res = rule.do_res(res).await;
        }
        res
    }
}
