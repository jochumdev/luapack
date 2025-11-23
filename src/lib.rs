#![allow(clippy::collapsible_if)]
mod bundle;
mod config;
mod graph;
mod normalize;
mod options;
mod replace;
mod resolve;
mod scan;
mod transform;
mod vendor;

pub use bundle::BindRequire as _BindRequireExport;
pub use bundle::{generate_bundle, lua_quote, BindRequire, BundleCtx};
pub use config::{load_config, resolve_path_like, resolve_pathbuf, BundleConfig, LoadedConfig};
pub use graph::ModuleGraph;
pub use normalize::infer_suffixes;
pub use options::{BundleOptions, NameNormalizer};
pub use replace::{matches_replace, parse_replace_rules, ArgMode, MatchKind, ReplaceRule};
pub use resolve::ModuleResolver;
pub use scan::{find_literal_requires, RequireMatch};
pub use transform::transform_requires;
pub use vendor::{collect_vendor_modules, parse_vendor_specs, to_glob_and_root, VendorSpec};
