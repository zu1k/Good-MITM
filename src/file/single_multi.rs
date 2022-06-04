use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SingleOrMulti<T> {
    Single(T),
    Multi(Vec<T>),
}

impl<T> SingleOrMulti<T> {
    pub fn into_vec(self) -> Vec<T> {
        self.into()
    }
}

impl<T> From<SingleOrMulti<T>> for Vec<T> {
    fn from(sm: SingleOrMulti<T>) -> Vec<T> {
        match sm {
            SingleOrMulti::Single(v) => vec![v],
            SingleOrMulti::Multi(mv) => mv,
        }
    }
}
