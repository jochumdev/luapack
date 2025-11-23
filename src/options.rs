use std::borrow::Cow;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::bundle::BindRequire;
use crate::replace::ReplaceRule;
use crate::vendor::VendorSpec;

#[derive(Debug, Clone, Default)]
pub struct NameNormalizer {
    suffixes: HashSet<String>,
}

impl NameNormalizer {
    pub fn new(suffixes: HashSet<String>) -> Self {
        Self { suffixes }
    }

    pub fn normalize<'a>(&self, name: &'a str) -> Cow<'a, str> {
        if self.suffixes.is_empty() {
            return Cow::Borrowed(name);
        }
        let mut out = name.to_string();
        loop {
            let mut changed = false;
            for suf in &self.suffixes {
                let suffixed = format!(".{}", suf);
                if out.ends_with(&suffixed) {
                    let newlen = out.len() - suffixed.len();
                    out.truncate(newlen);
                    changed = true;
                    break;
                }
            }
            if !changed {
                break;
            }
        }
        Cow::Owned(out)
    }
}

#[derive(Debug, Clone)]
pub struct BundleOptions {
    pub lua: String,
    pub paths: Vec<String>,
    pub preludes: Vec<PathBuf>,
    pub replaces: Vec<ReplaceRule>,
    pub vendor_specs: Vec<VendorSpec>,
    pub entry: Option<String>,
    pub bind: BindRequire,
    pub diagnostics: bool,
    pub redact_base: Option<PathBuf>,
    pub normalizer: NameNormalizer,
}
