use anyhow::Result;
use std::{fs, io::BufReader, path::Path};

use single_multi::SingleOrMulti;

pub mod rule;
mod single_multi;

pub(crate) fn load_rules_amd_mitm_filters<P: AsRef<Path>>(
    path: P,
) -> Result<(Vec<core::rule::Rule>, Vec<String>)> {
    let m = fs::metadata(&path).expect("Not a valid path");
    if m.file_type().is_dir() {
        load_rules_amd_mitm_filters_from_dir(path)
    } else {
        load_rules_amd_mitm_filters_from_file(path)
    }
}

fn load_rules_amd_mitm_filters_from_file<P: AsRef<Path>>(
    path: P,
) -> Result<(Vec<core::rule::Rule>, Vec<String>)> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let rules: Vec<rule::Rule> = serde_yaml::from_reader(reader)?;

    let (rules, filters) = rules
        .into_iter()
        .fold((vec![], vec![]), |(mut a, mut b), r| {
            let (rule, mut filters) = r.into();
            a.push(rule);
            b.append(&mut filters);
            (a, b)
        });

    Ok((rules, filters))
}

fn load_rules_amd_mitm_filters_from_dir<P: AsRef<Path>>(
    path: P,
) -> Result<(Vec<core::rule::Rule>, Vec<String>)> {
    let dir = fs::read_dir(path).expect("Not a valid dir");

    let (rules, filters) = dir
        .flatten()
        .filter(|f| f.file_type().is_ok())
        .filter(|f| f.file_type().ok().unwrap().is_file())
        .map(|f| load_rules_amd_mitm_filters_from_file(f.path()))
        .filter_map(|r| r.ok())
        .fold(
            (vec![], vec![]),
            |(mut a, mut b), (mut rule, mut filters)| {
                a.append(&mut rule);
                b.append(&mut filters);
                (a, b)
            },
        );

    Ok((rules, filters))
}
