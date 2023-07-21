use async_trait::async_trait;
use hyper::{Body, Request, Response};
use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};
use wildmatch::WildMatch;

use crate::mitm::{HttpContext, RequestOrResponse};

pub trait CustomContextData: Clone + Default + Send + Sync + 'static {}

#[async_trait]
pub trait HttpHandler<D: CustomContextData>: Clone + Send + Sync + 'static {
    async fn handle_request(
        &self,
        _ctx: &mut HttpContext<D>,
        req: Request<Body>,
    ) -> RequestOrResponse {
        RequestOrResponse::Request(req)
    }

    async fn handle_response(
        &self,
        _ctx: &mut HttpContext<D>,
        res: Response<Body>,
    ) -> Response<Body> {
        res
    }
}

#[derive(Clone, Default)]
pub struct MitmFilter<D: CustomContextData> {
    filters: Arc<RwLock<Vec<WildMatch>>>,

    _custom_contex_data: PhantomData<D>,
}

impl<D: CustomContextData> MitmFilter<D> {
    pub fn new(filters: Vec<String>) -> Self {
        let filters = filters.iter().map(|f| WildMatch::new(f)).collect();
        Self {
            filters: Arc::new(RwLock::new(filters)),
            ..Default::default()
        }
    }

    pub async fn filter_req(&self, _ctx: &HttpContext<D>, req: &Request<Body>) -> bool {
        let host = req.uri().host().unwrap_or_default();
        let list = self.filters.read().unwrap();
        for m in list.iter() {
            if m.matches(host) {
                return true;
            }
        }
        false
    }

    pub async fn filter(&self, host: &str) -> bool {
        let list = self.filters.read().unwrap();
        for m in list.iter() {
            if m.matches(host) {
                return true;
            }
        }
        false
    }
}
