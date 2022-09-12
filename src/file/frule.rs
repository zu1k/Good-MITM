use serde::{Deserialize, Serialize};

use super::SingleOrMulti;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub name: String,
    #[serde(alias = "mitm")]
    pub mitm_list: Option<SingleOrMulti<String>>,
    #[serde(alias = "filter")]
    pub filters: SingleOrMulti<rule::Filter>,
    #[serde(alias = "action")]
    pub actions: SingleOrMulti<rule::Action>,
}

impl From<Rule> for (rule::Rule, Vec<String>) {
    fn from(rule: Rule) -> Self {
        let filters: Vec<rule::Filter> = rule
            .filters
            .into_vec()
            .iter()
            .map(rule::Filter::init)
            .collect();

        let mut mitm_filters: Vec<String> = filters
            .iter()
            .filter_map(rule::Filter::mitm_filtter_pattern)
            .collect();

        let mut mitm_list_2 = match rule.mitm_list {
            Some(s) => s.into_vec().into_iter().collect(),
            None => vec![],
        };
        mitm_filters.append(&mut mitm_list_2);

        let rule = rule::Rule {
            filters,
            actions: rule.actions.into_vec(),
            url: None,
        };

        (rule, mitm_filters)
    }
}
