use crate::action;
use crate::filter;
use crate::filter::Filter;
use hudsucker::hyper::Body;
use hudsucker::hyper::Request;
use std::path::Path;
use std::sync::RwLock;
use std::vec::Vec;
mod file;

lazy_static! {
    static ref RULES: RwLock<Vec<Rule>> = RwLock::from(Vec::new());
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub filter: filter::Filter,
    pub action: action::Action,
}

impl From<file::Rule> for Rule {
    fn from(rule: file::Rule) -> Self {
        let filter = match rule.filter {
            file::Filter::Domain(s) => Filter::new_domain(s.as_str()),
            file::Filter::DomainKeyword(s) => Filter::new_domain_keyword(s.as_str()),
            file::Filter::DomainPrefix(s) => Filter::new_domain_prefix(s.as_str()),
            file::Filter::DomainSuffix(s) => Filter::new_domain_suffix(s.as_str()),
            file::Filter::UrlRegex(re) => Filter::new_url_regex(re.as_str()),
        };
        Self {
            filter,
            action: rule.action,
        }
    }
}

pub fn match_rule(req: &Request<Body>) -> Option<Rule> {
    let rules = RULES.read().unwrap();
    for rule in rules.iter() {
        if rule.filter.is_match_req(req) {
            return Some(rule.clone());
        }
    }
    None
}

#[allow(dead_code)]
pub fn add_rule_examples_internal() {
    let mut rules = RULES.write().unwrap();
    rules.push(Rule {
        filter: filter::Filter::new_url_regex(r"^https:?/\/ddrk.me.*"),
        action: action::Action::Modify(action::Modify::Body(action::BodyModify {
            origin: "<head>".to_string(),
            new: include_str!("../../assets/body/ddrk.html").to_string(),
        })),
    });
    rules.push(Rule {
        filter: filter::Filter::new_url_regex(
            r"(nfmovies)(?!.*?(\.css|\.js|\.jpeg|\.png|\.gif)).*",
        ),
        action: action::Action::Modify(action::Modify::Body(action::BodyModify {
            origin: "<head>".to_string(),
            new: include_str!("../../assets/body/nfmovies.html").to_string(),
        })),
    });
}

pub fn add_rule_file<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let mut rules = RULES.write().unwrap();
    match file::read_rules_from_file(path) {
        Ok(rules_config) => {
            for rule in rules_config {
                rules.push(Rule::from(rule));
            }
            Ok(())
        }
        Err(err) => Err(err),
    }
}
