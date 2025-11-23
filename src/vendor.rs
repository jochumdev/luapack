use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glob::glob;

use crate::normalize::normalize_module_name;
use crate::options::NameNormalizer;
use crate::replace::{matches_replace, MatchKind, ReplaceRule};

#[derive(Debug, Clone)]
pub struct VendorSpec {
    pub paths: Vec<String>,
    pub exclude_names: Vec<String>,
    pub exclude_prefixes: Vec<String>,
    pub suffixes: Vec<String>,
}

fn path_rule_matches(replaces: &[ReplaceRule], path: &Path) -> bool {
    let p = path.to_string_lossy().replace('\\', "/");
    for r in replaces {
        if r.match_kind == MatchKind::Path {
            for g in &r.paths {
                if let Ok(pat) = glob::Pattern::new(g) {
                    if pat.matches(&p) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn parse_vendor_specs(flags: &[String]) -> Result<Vec<VendorSpec>> {
    let mut out = Vec::new();
    for raw in flags {
        let mut paths: Vec<String> = Vec::new();
        let mut exclude_names: Vec<String> = Vec::new();
        let mut exclude_prefixes: Vec<String> = Vec::new();
        let mut suffixes: Vec<String> = Vec::new();
        for part in raw.split(',') {
            let s = part.trim();
            if s.is_empty() {
                continue;
            }
            if let Some((k, v)) = s.split_once('=') {
                match k.trim() {
                    "path" => paths.push(v.trim().to_string()),
                    "exclude" => {
                        if let Some((kind, val)) = v.split_once(':') {
                            match kind.trim() {
                                "name" => exclude_names.push(val.trim().to_string()),
                                "prefix" => exclude_prefixes.push(val.trim().to_string()),
                                _ => {}
                            }
                        }
                    }
                    "suffix" => suffixes.push(v.trim().to_string()),
                    _ => {}
                }
            } else {
                // Bare value: treat as a path template directly
                paths.push(s.to_string());
            }
        }
        out.push(VendorSpec {
            paths,
            exclude_names,
            exclude_prefixes,
            suffixes,
        });
    }
    Ok(out)
}

pub fn collect_vendor_modules(
    specs: &[VendorSpec],
    replaces: &[crate::replace::ReplaceRule],
    normalizer: &NameNormalizer,
) -> Result<(HashMap<String, PathBuf>, HashSet<String>)> {
    let mut out: HashMap<String, PathBuf> = HashMap::new();
    let mut dups: HashSet<String> = HashSet::new();
    for spec in specs {
        for t in &spec.paths {
            let (glob_pat, root_prefix, init_mode) = to_glob_and_root(t);
            for path in glob(&glob_pat)
                .with_context(|| format!("bad glob pattern: {}", glob_pat))?
                .flatten()
            {
                if !path.is_file() {
                    continue;
                }
                let rel = match path.strip_prefix(&root_prefix) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let rel_str = rel.to_string_lossy();
                let raw_name = if init_mode {
                    rel_str
                        .trim_end_matches("/init.lua")
                        .trim_end_matches("\\init.lua")
                        .replace(['\\', '/'], ".")
                } else {
                    rel_str.trim_end_matches(".lua").replace(['\\', '/'], ".")
                };
                let name = normalize_module_name(&raw_name, normalizer);
                if spec.exclude_names.iter().any(|n| n == &name) {
                    continue;
                }
                if spec.exclude_prefixes.iter().any(|p| name.starts_with(p)) {
                    continue;
                }
                if matches_replace(&name, replaces, normalizer)
                    || path_rule_matches(replaces, &path)
                {
                    continue;
                }
                if out.contains_key(&name) {
                    dups.insert(name.clone());
                }
                out.entry(name).or_insert(path);
            }
        }
    }
    Ok((out, dups))
}

pub fn to_glob_and_root(t: &str) -> (String, PathBuf, bool) {
    let init_mode = t.contains("?/init.lua");
    let idx = t.find('?').unwrap_or(t.len());
    let root = &t[..idx];
    let glob_pat = if init_mode {
        t.replace("?/init.lua", "**/init.lua")
    } else {
        t.replace("?.lua", "**/*.lua")
    };
    (glob_pat, PathBuf::from(root), init_mode)
}
