use crate::{rule, HttpContext, RequestOrResponse};
use hyper::{header, Body, Request, Response};
use log::info;

mod mitm_filter;
pub use mitm_filter::*;

#[derive(Clone, Default)]
pub struct HttpHandler {}

impl HttpHandler {
    pub async fn handle_request(ctx: &mut HttpContext, req: Request<Body>) -> RequestOrResponse {
        ctx.uri = Some(req.uri().clone());

        // remove accept-encoding to avoid encoded body
        let mut req = req;
        req.headers_mut().remove(header::ACCEPT_ENCODING);

        let rules = rule::match_rules(&req);
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

    pub async fn handle_response(ctx: &mut HttpContext, res: Response<Body>) -> Response<Body> {
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
