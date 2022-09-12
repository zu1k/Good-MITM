use hyper::{Body, Request};
use serde::{Deserialize, Serialize};

use crate::cache::get_regex;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Filter {
    All,
    Domain(String),
    DomainKeyword(String),
    DomainPrefix(String),
    DomainSuffix(String),
    UrlRegex(String),
}

impl Filter {
    pub fn init(&self) -> Self {
        match self {
            Filter::All => self.to_owned(),
            Filter::Domain(d) => Self::Domain(d.to_lowercase()),
            Filter::DomainKeyword(d) => Self::DomainKeyword(d.to_lowercase()),
            Filter::DomainPrefix(d) => Self::DomainPrefix(d.to_lowercase()),
            Filter::DomainSuffix(d) => Self::DomainSuffix(d.to_lowercase()),
            Filter::UrlRegex(re) => Self::UrlRegex(re.to_owned()),
        }
    }

    pub fn is_match_req(&self, req: &Request<Body>) -> bool {
        let host = req.uri().host().unwrap_or_default().to_lowercase();
        match self {
            Self::All => true,
            Self::Domain(target) => host == *target,
            Self::DomainKeyword(target) => host.contains(target),
            Self::DomainPrefix(target) => host.starts_with(target),
            Self::DomainSuffix(target) => host.ends_with(target),
            Self::UrlRegex(target) => {
                let url = req.uri().to_string();
                get_regex(target).is_match(&url).unwrap()
            }
        }
    }

    pub fn mitm_filtter_pattern(&self) -> Option<String> {
        match self {
            Self::All => Some("*".to_owned()),
            Self::Domain(d) => Some(d.to_owned()),
            Self::DomainKeyword(d) => Some(format!("*{}*", d)),
            Self::DomainPrefix(d) => Some(format!("{}*", d)),
            Self::DomainSuffix(d) => Some(format!("*{}", d)),
            _ => None,
        }
    }
}
