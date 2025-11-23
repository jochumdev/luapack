use std::collections::HashSet;

use crate::options::NameNormalizer;
use crate::replace::ReplaceRule;

pub fn normalize_module_name(name: &str, n: &NameNormalizer) -> String {
    n.normalize(name).into_owned()
}

pub fn infer_suffixes(
    paths: &[String],
    vendor_paths: &[String],
    vendor_suffixes: &[String],
    replaces: &[ReplaceRule],
) -> HashSet<String> {
    let mut s: HashSet<String> = HashSet::new();

    // Infer from --path templates
    for p in paths {
        if p.contains("?/init.lua") {
            s.insert("init".to_string());
        }
    }

    // Infer from vendor paths and explicit suffixes
    for p in vendor_paths {
        if p.contains("?/init.lua") {
            s.insert("init".to_string());
        }
    }
    for suf in vendor_suffixes {
        if !suf.is_empty() {
            s.insert(suf.clone());
        }
    }

    // Infer from replace path rules
    for r in replaces {
        for p in &r.paths {
            if p.contains("?/init.lua") {
                s.insert("init".to_string());
            }
        }
    }

    s
}
