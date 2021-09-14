use crate::action;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use std::{fs::File, io::BufReader};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub name: String,
    pub filter: Filter,
    pub action: action::Action,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Filter {
    Domain(String),
    DomainKeyword(String),
    DomainPrefix(String),
    DomainSuffix(String),
    UrlRegex(String),
}

pub fn read_rules_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Rule>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let rules = serde_yaml::from_reader(reader)?;
    Ok(rules)
}
