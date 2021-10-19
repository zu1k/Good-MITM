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

impl<T> IntoIterator for SingleOrMulti<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = SingleOrMultiIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        SingleOrMultiIter {
            data: self,
            index: 0,
        }
    }
}

pub struct SingleOrMultiIter<T>
where
    T: Clone,
{
    data: SingleOrMulti<T>,
    index: usize,
}

impl<T> Iterator for SingleOrMultiIter<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.data {
            SingleOrMulti::Single(v) => {
                if self.index == 0 {
                    self.index += 1;
                    Some(v.clone())
                } else {
                    None
                }
            }
            SingleOrMulti::Multi(sm) => {
                let idx = self.index;
                self.index += 1;
                if sm.len() > idx {
                    Some(sm[idx].clone())
                } else {
                    None
                }
            }
        }
    }
}
