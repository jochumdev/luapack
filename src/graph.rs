use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;

use crate::normalize::normalize_module_name;
use crate::options::NameNormalizer;
use crate::resolve::ModuleResolver;
use crate::scan::find_literal_requires;

pub struct ModuleGraph {
    pub first_party: HashMap<String, PathBuf>,
    pub unresolved: HashSet<String>,
}

impl ModuleGraph {
    pub fn build_from_entry_code(
        entry_code: &str,
        resolver: &ModuleResolver,
        normalizer: &NameNormalizer,
    ) -> Self {
        let mut first_party: HashMap<String, PathBuf> = HashMap::new();
        let mut unresolved: HashSet<String> = HashSet::new();
        let mut visited_paths: HashSet<PathBuf> = HashSet::new();
        let mut q: VecDeque<(String, PathBuf)> = VecDeque::new();

        for r in find_literal_requires(entry_code) {
            let n = normalize_module_name(&r.module, normalizer);
            if let Some(path) = resolver.resolve(&r.module) {
                q.push_back((n, path));
            } else {
                unresolved.insert(n);
            }
        }

        while let Some((mod_name, path)) = q.pop_front() {
            if visited_paths.contains(&path) {
                continue;
            }
            visited_paths.insert(path.clone());
            first_party.entry(mod_name.clone()).or_insert(path.clone());

            if let Ok(code) = fs::read_to_string(&path) {
                for r in find_literal_requires(&code) {
                    let n = normalize_module_name(&r.module, normalizer);
                    if let Some(p) = resolver.resolve(&r.module) {
                        if !visited_paths.contains(&p) {
                            q.push_back((n, p));
                        }
                    } else {
                        unresolved.insert(n);
                    }
                }
            }
        }

        Self {
            first_party,
            unresolved,
        }
    }
}
