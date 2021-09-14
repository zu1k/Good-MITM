use crate::action::Action;
use crate::rule;

use hudsucker::{
    async_trait::async_trait,
    hyper::{header, Body, Request, Response},
    HttpContext, HttpHandler, RequestOrResponse,
};

#[derive(Clone, Default)]
pub struct MitmHandler {
    should_modify_response: bool,
    action: Option<Action>,
}

#[async_trait]
impl HttpHandler for MitmHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        println!("{:?}", req.uri().to_string());

        // remove accept-encoding to avoid encoded body
        let mut req = req;
        req.headers_mut().remove(header::ACCEPT_ENCODING);

        if let Some(rule) = rule::match_rule(&req) {
            self.should_modify_response = true;
            let rt = rule.action.do_req(req).await;
            self.action = Some(rule.action);
            return rt;
        }

        RequestOrResponse::Request(req)
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        if !self.should_modify_response || self.action.is_none() {
            return res;
        }
        let action = self.action.clone().unwrap();
        action.do_res(res).await
    }
}
