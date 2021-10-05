use fancy_regex::Regex;
use http_mitm::hyper::{Body, Request};
mod mitm_filter;
pub use mitm_filter::*;

#[derive(Debug, Clone)]
pub enum Filter {
    Domain(String),
    DomainKeyword(String),
    DomainPrefix(String),
    DomainSuffix(String),
    UrlRegex(fancy_regex::Regex),
}

#[allow(dead_code)]
impl Filter {
    pub fn new_domain(s: &str) -> Self {
        Self::Domain(s.to_lowercase())
    }

    pub fn new_domain_keyword(s: &str) -> Self {
        Self::DomainKeyword(s.to_lowercase())
    }

    pub fn new_domain_prefix(s: &str) -> Self {
        Self::DomainPrefix(s.to_lowercase())
    }

    pub fn new_domain_suffix(s: &str) -> Self {
        Self::DomainSuffix(s.to_lowercase())
    }

    pub fn new_url_regex(s: &str) -> Self {
        let r = Regex::new(s).unwrap();
        Self::UrlRegex(r)
    }

    pub fn is_match_req(&self, req: &Request<Body>) -> bool {
        let host = req.uri().host().unwrap_or_default().to_lowercase();
        match self {
            Self::Domain(target) => host == *target,
            Self::DomainKeyword(target) => host.contains(target),
            Self::DomainPrefix(target) => host.starts_with(target),
            Self::DomainSuffix(target) => host.ends_with(target),
            Self::UrlRegex(re) => {
                let url = req.uri().to_string();
                re.is_match(&url).unwrap()
            }
        }
    }
}
