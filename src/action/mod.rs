mod modify;
pub use modify::*;

use hudsucker::hyper::{header, header::HeaderValue, Body, Request, Response, StatusCode};
use hudsucker::RequestOrResponse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Reject,
    Redirect(String),
    Modify(Modify),
}

impl Action {
    pub async fn do_req(&self, req: Request<Body>) -> RequestOrResponse {
        match self {
            Action::Reject => {
                let res = Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::default())
                    .unwrap();
                RequestOrResponse::Response(res)
            }
            Action::Redirect(target) => match HeaderValue::from_str(target.as_str()) {
                Ok(target) => {
                    let mut res = Response::builder()
                        .status(StatusCode::FOUND)
                        .body(Body::default())
                        .unwrap();
                    res.headers_mut().insert(header::LOCATION, target);
                    RequestOrResponse::Response(res)
                }
                Err(_) => RequestOrResponse::Request(req),
            },
            Action::Modify(modify) => RequestOrResponse::Request(modify.modify_req(req).await),
        }
    }

    pub async fn do_res(&self, res: Response<Body>) -> Response<Body> {
        match self {
            Action::Modify(modify) => modify.modify_res(res).await,
            _ => res,
        }
    }
}
