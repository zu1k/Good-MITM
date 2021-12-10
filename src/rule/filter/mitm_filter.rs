use crate::HttpContext;
use async_trait::async_trait;
use hyper::{Body, Request};
use std::{collections::HashMap, sync::RwLock};
use wildmatch::WildMatch;

lazy_static! {
    static ref MITM_LIST: RwLock<Vec<String>> = RwLock::from(Vec::new());
}

pub fn mitm_list_append(list: Vec<String>) {
    let mut list = list;
    MITM_LIST.write().unwrap().append(&mut list);
}

#[derive(Clone, Default)]
pub struct MitmFilter {
    matches: HashMap<String, WildMatch>,
}

#[async_trait]
impl crate::mitm::MitmFilter for MitmFilter {
    async fn filter(&mut self, _ctx: &HttpContext, req: &Request<Body>) -> bool {
        let host = req.uri().host().unwrap_or_default();
        let list = MITM_LIST.read().unwrap();
        for p in list.iter() {
            if self
                .matches
                .entry(p.clone())
                .or_insert_with(|| WildMatch::new(p))
                .matches(host)
            {
                return true;
            }
        }
        false
    }
}
