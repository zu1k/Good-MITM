use serde::{Deserialize, Serialize};

use super::SingleOrMulti;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub name: String,
    #[serde(alias = "mitm")]
    pub mitm_list: Option<Vec<String>>,
    #[serde(alias = "filter")]
    pub filters: SingleOrMulti<core::rule::Filter>,
    #[serde(alias = "action")]
    pub actions: SingleOrMulti<core::rule::Action>,
}

impl From<Rule> for (core::rule::Rule, Vec<String>) {
    fn from(rule: Rule) -> Self {
        let filters: Vec<core::rule::Filter> = rule
            .filters
            .into_vec()
            .iter()
            .map(core::rule::Filter::init)
            .collect();

        let mut mitm_filters: Vec<String> = filters
            .iter()
            .filter_map(core::rule::Filter::mitm_filtter_pattern)
            .collect();

        let mut mitm_list_2 = match rule.mitm_list {
            Some(s) => s.into_iter().collect(),
            None => vec![],
        };
        mitm_filters.append(&mut mitm_list_2);

        let rule = core::rule::Rule {
            filters,
            actions: rule.actions.into_vec(),
            url: None,
        };

        (rule, mitm_filters)
    }
}
