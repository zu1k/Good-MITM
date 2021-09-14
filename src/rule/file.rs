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

#[test]
fn test_gen_yaml() {
    let rules = vec![
        Rule {
            name: "demo rule 1".into(),
            filter: Filter::Domain("domain.com".into()),
            action: action::Action::Modify(action::Modify::new_modify_body(
                "origin body",
                "new body",
            )),
        },
        Rule {
            name: "demo rule 2".into(),
            filter: Filter::UrlRegex(r".*\.domain.com".into()),
            action: action::Action::Modify(action::Modify::new_modify_body(
                "origin body",
                "new body",
            )),
        },
    ];

    let s = serde_yaml::to_string(&rules).expect("serde yaml failed!");
    println!("{}", s);

    let f = include_str!("../../assets/rules/demo.yaml");
    let rules: Vec<Rule> = serde_yaml::from_str(f).expect("parse failed!");
    println!("{:#?}", rules);
    if let Filter::UrlRegex(r) = rules[0].clone().filter {
        println!("{}", r)
    }

    // match read_rules_from_file("assets/rules/demo.yaml") {
    //     Ok(rules) => println!("{:#?}", rules),
    //     Err(err) => panic!("{}", err),
    // }
}
