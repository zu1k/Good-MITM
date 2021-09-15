use crate::rule::{self, Rule};
use hudsucker::{
    async_trait::async_trait,
    hyper::{Body, Request, Response},
    HttpContext, HttpHandler, RequestOrResponse,
};

#[derive(Clone, Default)]
pub struct MitmHandler {
    should_modify_response: bool,
    rule: Option<Rule>,
}

#[async_trait]
impl HttpHandler for MitmHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        // remove accept-encoding to avoid encoded body
        // let mut req = req;
        // req.headers_mut().remove(header::ACCEPT_ENCODING);

        if let Some(mut rule) = rule::match_rule(&req) {
            self.should_modify_response = true;
            let rt = rule.do_req(req).await;
            self.rule = Some(rule);
            return rt;
        }

        RequestOrResponse::Request(req)
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        if !self.should_modify_response || self.rule.is_none() {
            return res;
        }
        let rule = self.rule.clone().unwrap();
        rule.do_res(res).await
    }
}
