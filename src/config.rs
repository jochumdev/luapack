use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use config as cfg;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize, Clone)]
pub struct BundleConfig {
    pub lua: Option<String>,
    pub paths: Option<Vec<String>>,
    pub preludes: Option<Vec<String>>,
    pub replace: Option<Vec<String>>,
    pub vendors: Option<Vec<String>>,
    pub output: Option<String>,
    pub entry: Option<String>,
    pub bind_require: Option<String>,
    pub diagnostics: Option<bool>,
    pub redact_base: Option<String>,
}

#[derive(Debug, Default, Deserialize, Clone)]
struct RootConfig {
    pub bundle: Option<BundleConfig>,
}

pub struct LoadedConfig {
    pub cfg: BundleConfig,
    pub dir: Option<PathBuf>,
}

pub fn load_config(explicit: Option<&Path>) -> Result<LoadedConfig> {
    if let Some(p) = explicit {
        return load_from_path(p);
    }
    let cwd = std::env::current_dir()?;
    let candidates = [
        "luapack.toml",
        "luapack.yaml",
        "luapack.yml",
        "luapack.json",
    ];
    for name in &candidates {
        let path = cwd.join(name);
        if path.is_file() {
            return load_from_path(&path);
        }
    }
    Ok(LoadedConfig {
        cfg: BundleConfig::default(),
        dir: None,
    })
}

fn load_from_path(path: &Path) -> Result<LoadedConfig> {
    let builder = cfg::Config::builder().add_source(cfg::File::from(path));
    // Optional environment overlay: LUAPACK_BUNDLE_KEYS
    let builder = builder.add_source(cfg::Environment::with_prefix("LUAPACK").separator("_"));
    let conf = builder.build()?;
    let root = conf
        .try_deserialize::<RootConfig>()
        .with_context(|| format!("failed to parse config at {}", path.display()))?;
    let cfg = root.bundle.unwrap_or_default();
    let dir = path.parent().map(|p| p.to_path_buf());
    Ok(LoadedConfig { cfg, dir })
}

pub fn resolve_path_like(base: Option<&Path>, value: &str) -> String {
    let p = Path::new(value);
    if p.is_absolute() {
        value.to_string()
    } else if let Some(b) = base {
        b.join(p).to_string_lossy().to_string()
    } else {
        value.to_string()
    }
}

pub fn resolve_pathbuf(base: Option<&Path>, value: &str) -> PathBuf {
    let p = Path::new(value);
    if p.is_absolute() {
        p.to_path_buf()
    } else if let Some(b) = base {
        b.join(p)
    } else {
        p.to_path_buf()
    }
}
