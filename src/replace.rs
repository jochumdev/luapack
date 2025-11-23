use crate::normalize::normalize_module_name;
use crate::options::NameNormalizer;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchKind {
    Exact,
    Prefix,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgMode {
    Rest,
    Full,
}

#[derive(Debug, Clone)]
pub struct ReplaceRule {
    pub match_kind: MatchKind,
    pub old: String,
    pub new: String,
    pub name: Option<String>,
    pub prefix: Option<String>,
    pub paths: Vec<String>,
    pub arg: ArgMode,
}

pub fn parse_replace_rules(flags: &[String]) -> Result<Vec<ReplaceRule>> {
    let mut out = Vec::new();
    for raw in flags {
        let mut match_kind: Option<MatchKind> = None;
        let mut old: Option<String> = None;
        let mut newc: Option<String> = None;
        let mut name: Option<String> = None;
        let mut prefix: Option<String> = None;
        let mut paths: Vec<String> = Vec::new();
        let mut arg = ArgMode::Full;

        for part in raw.split(',') {
            let (k, v) = match part.split_once('=') {
                Some(kv) => kv,
                None => continue,
            };
            match k.trim() {
                "match" => match v.trim() {
                    "exact" => match_kind = Some(MatchKind::Exact),
                    "prefix" => match_kind = Some(MatchKind::Prefix),
                    "path" => match_kind = Some(MatchKind::Path),
                    other => return Err(anyhow::anyhow!("unknown match kind: {}", other)),
                },
                "old" => old = Some(v.trim().to_string()),
                "new" => newc = Some(v.trim().to_string()),
                "name" => name = Some(v.trim().to_string()),
                "prefix" => prefix = Some(v.trim().to_string()),
                "path" => paths.push(v.trim().to_string()),
                "arg" => match v.trim() {
                    "{rest}" => arg = ArgMode::Rest,
                    "{full}" => arg = ArgMode::Full,
                    other => return Err(anyhow::anyhow!("unknown arg mode: {}", other)),
                },
                _ => {}
            }
        }

        let rule = ReplaceRule {
            match_kind: match_kind
                .ok_or_else(|| anyhow::anyhow!("replace rule requires 'match='"))?,
            old: old.unwrap_or_else(|| "require".to_string()),
            new: newc.ok_or_else(|| anyhow::anyhow!("replace rule requires 'new='"))?,
            name,
            prefix,
            paths,
            arg,
        };
        out.push(rule);
    }
    Ok(out)
}

pub fn matches_replace(name: &str, replaces: &[ReplaceRule], normalizer: &NameNormalizer) -> bool {
    let name = normalize_module_name(name, normalizer);
    for r in replaces {
        match r.match_kind {
            MatchKind::Exact => {
                if let Some(ref n) = r.name {
                    if n == &name {
                        return true;
                    }
                }
            }
            MatchKind::Prefix => {
                if let Some(ref p) = r.prefix {
                    if name.starts_with(p) {
                        return true;
                    }
                }
            }
            MatchKind::Path => {}
        }
    }
    false
}
