use std::{borrow::Cow, collections::BTreeSet};

use semver::Version;

pub type FeatureSet<'a> = BTreeSet<Cow<'a, str>>;

pub struct Configuration<'a> {
    pub name: String,
    pub version: Version,
    pub enabled: FeatureSet<'a>,
    pub disabled: FeatureSet<'a>,
}

impl<'a> Configuration<'a> {
    pub fn new(name: String, version: Version, enabled: FeatureSet<'a>, disabled: FeatureSet<'a>) -> Self {
        Self { name, version, enabled, disabled }
    }

    pub fn is_enabled(&self, feature: &str) -> bool {
        self.enabled.contains(&Cow::Borrowed(feature))
    }

    pub fn is_feature(&self, feature: &str) -> bool {
        self.enabled.contains(&Cow::Borrowed(feature)) || self.disabled.contains(&Cow::Borrowed(feature))
    }

    pub fn features(&self) -> impl Iterator<Item = &str> {
        let enabled = self.enabled.iter().map(|f| f.as_ref());
        let disabled = self.disabled.iter().map(|f| f.as_ref());
        enabled.chain(disabled)
    }
}

