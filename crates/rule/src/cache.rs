use cached::{proc_macro::cached, SizedCache};
use fancy_regex::Regex;

#[cached(
    type = "SizedCache<String, Regex>",
    create = "{ SizedCache::with_size(100) }",
    convert = r#"{ re.to_string() }"#
)]
pub fn get_regex(re: &str) -> Regex {
    fancy_regex::Regex::new(re).unwrap()
}
