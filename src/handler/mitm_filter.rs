use crate::HttpContext;
use hyper::{Body, Request};
use std::sync::RwLock;
use wildmatch::WildMatch;

lazy_static! {
    static ref MATCHES: RwLock<Vec<WildMatch>> = RwLock::from(Vec::new()); // TODO: init this
}

pub fn mitm_list_append(list: Vec<String>) {
    let mut list = list.iter().map(|m| WildMatch::new(m)).collect();
    MATCHES.write().unwrap().append(&mut list);
}

#[derive(Clone, Default)]
pub struct MitmFilter;

impl MitmFilter {
    pub async fn filter(_ctx: &HttpContext, req: &Request<Body>) -> bool {
        let host = req.uri().host().unwrap_or_default();
        let list = MATCHES.read().unwrap();
        for m in list.iter() {
            if m.matches(host) {
                return true;
            }
        }
        false
    }
}
