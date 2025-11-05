use std::{borrow::Cow, collections::BTreeMap};

use itertools::Itertools;
use semver::Version;

pub struct Configuration<'a> {
    pub name: String,
    pub version: Version,
    pub features: BTreeMap<Cow<'a, str>, bool>,
}

impl<'a> Configuration<'a> {
    pub fn new(name: String, version: Version, features: BTreeMap<Cow<'a, str>, bool>) -> Self {
        Self { name, version, features }
    }

    pub fn is_enabled(&self, feature: &str) -> bool {
        *self.features.get(&Cow::Borrowed(feature)).unwrap_or(&false)
    }

    pub fn from_csv(name: String, version: Version, content: &'a str) -> Option<Configuration<'a>> {
        let features = content.lines()
            .map(|l| l.split_once(','))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .map(|(l, r)| (Cow::Borrowed(l.trim_matches('"')), r == "True"))
            .collect::<BTreeMap<_, _>>();
        Some(Configuration::new(name, version, features))
    }

    pub fn from_csv_owned(name: String, version: Version, content: &str) -> Option<Configuration<'static>> {
        let features = content.lines()
            .map(|l| l.split_once(','))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .map(|(l, r)| (Cow::Owned(l.trim_matches('"').to_string()), r == "True"))
            .collect::<BTreeMap<_, _>>();
        Some(Configuration::new(name, version, features))
    }

    pub fn to_csv(&self) -> String {
        self.features.iter()
            .map(|(feature, &enabled)| format!("\"{feature}\",{}", if enabled { "True" } else { "False" }))
            .join("\n")
    }
}

