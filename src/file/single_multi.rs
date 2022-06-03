use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SingleOrMulti<T>
where
    T: Clone,
{
    Single(T),
    Multi(Vec<T>),
}

impl<T> SingleOrMulti<T>
where
    T: Clone,
{
    pub fn to_vec(self) -> Vec<T> {
        self.into()
    }
}

impl<T> From<SingleOrMulti<T>> for Vec<T>
where
    T: Clone,
{
    fn from(sm: SingleOrMulti<T>) -> Vec<T> {
        match sm {
            SingleOrMulti::Single(v) => vec![v],
            SingleOrMulti::Multi(mv) => mv,
        }
    }
}
