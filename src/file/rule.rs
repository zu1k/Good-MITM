use serde::{Deserialize, Serialize};

use crate::{rule::{filter::Filter, action::Action}, handler::mitm_list_append};

use super::SingleOrMulti;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub name: String,
    #[serde(alias = "mitm")]
    pub mitm_list: Option<Vec<String>>,
    #[serde(alias = "filter")]
    pub filters: SingleOrMulti<Filter>,
    #[serde(alias = "action")]
    pub actions: SingleOrMulti<Action>,
}

impl Into<crate::rule::Rule> for Rule {
    fn into(self) -> crate::rule::Rule {
        let filters: Vec<Filter> = self.filters.to_vec().iter().map(|f| Filter::init).collect();

        {
            // append mitm list
            let mitm_list: Vec<String> = filters
                .iter()
                .filter_map(Filter::mitm_filtter_pattern)
                .collect();
            mitm_list_append(mitm_list);

            let mitm_list = match self.mitm_list {
                Some(s) => s.into_iter().collect(),
                None => vec![],
            };
            mitm_list_append(mitm_list);
        }

        crate::rule::Rule {
            filters,
            actions: self.actions.to_vec(),
            url: None,
        }
    }
}
