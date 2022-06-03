use fancy_regex::Regex;

pub fn get_regex(re: &str) -> Regex {
    fancy_regex::Regex::new(re).unwrap()
}
