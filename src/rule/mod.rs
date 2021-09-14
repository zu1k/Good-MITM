use crate::action;
use crate::filter;
use hudsucker::hyper::Body;
use hudsucker::hyper::Request;
use std::sync::RwLock;
use std::vec::Vec;

lazy_static! {
    static ref RULES: RwLock<Vec<Rule>> = { RwLock::from(Vec::new()) };
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub filter: filter::Filter,
    pub action: action::Action,
}

pub fn match_rule(req: &Request<Body>) -> Option<Rule> {
    let rules = RULES.read().unwrap();
    for rule in rules.iter() {
        if rule.filter.is_match_req(req) {
            return Some(rule.clone());
        }
    }
    return None;
}

pub fn add_rule_examples() {
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
