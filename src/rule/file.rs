use super::action;
use good_mitm::utils::SingleOrMulti;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs, io::BufReader, path::Path};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub name: String,
    #[serde(alias = "filter")]
    pub filters: SingleOrMulti<Filter>,
    #[serde(alias = "action")]
    pub actions: SingleOrMulti<action::Action>,
}

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

pub fn read_rules_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Rule>, Box<dyn Error>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let rules = serde_yaml::from_reader(reader)?;
    Ok(rules)
}

pub fn read_rules_from_dir<P: AsRef<Path>>(path: P) -> Result<Vec<Rule>, Box<dyn Error>> {
    let mut rules = vec![];
    let dir = fs::read_dir(path).expect("Not a valid dir");
    for entry in dir.flatten() {
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_file() {
                if let Ok(ref mut rules_part) = read_rules_from_file(entry.path()) {
                    rules.append(rules_part);
                }
            }
        }
    }
    Ok(rules)
}

pub fn read_rules_from_firl_or_dir<P: AsRef<Path>>(path: P) -> Result<Vec<Rule>, Box<dyn Error>> {
    let m = fs::metadata(&path).expect("Not a valid path");
    if m.file_type().is_dir() {
        read_rules_from_dir(path)
    } else {
        read_rules_from_file(path)
    }
}
