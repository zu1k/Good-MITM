use fancy_regex::Regex;
use std::{collections::HashMap, sync::RwLock};

lazy_static! {
    pub static ref REGEX: RwLock<HashMap<String, Regex>> = RwLock::from(HashMap::default());
}
