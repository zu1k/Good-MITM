use crate::action::Modify;
use crate::filter::Filter;

use hudsucker::{
    async_trait::async_trait,
    hyper::{header, Body, Request, Response},
    HttpContext, HttpHandler, RequestOrResponse,
};

#[derive(Clone, Default)]
pub struct MitmHandler {
    should_modify_response: bool,
}

#[async_trait]
impl HttpHandler for MitmHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        println!("{:?}", req.uri().to_string());
        let filter = Filter::new_domain("www.nfmovies.com");
        if filter.is_match_req(&req) {
            self.should_modify_response = true;
            // let action = Action::Redirect("https://lgf.im/".to_string());
            // return action.do_req(req);
        }
        let mut req = req;
        req.headers_mut().remove(header::ACCEPT_ENCODING);
        RequestOrResponse::Request(req)
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        if !self.should_modify_response {
            return res;
        }

        let origin_body = "</body>";
        let new_body = include_str!("../../assets/bb.html");
        let modifier = Modify::new_modify_body(origin_body, new_body);

        // println!("{:?}", res);
        modifier.modify_res(res).await
    }
}
