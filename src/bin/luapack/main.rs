#![allow(unexpected_cfgs)]
#![allow(clippy::collapsible_if)]
use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use luapack::{
    BundleCtx, ModuleGraph, ModuleResolver, _BindRequireExport as BindRequire,
    collect_vendor_modules, generate_bundle, infer_suffixes, load_config, parse_replace_rules,
    parse_vendor_specs, resolve_pathbuf, BundleOptions, NameNormalizer,
};

/// luapack: Lua bundler (Rust) — CLI
#[derive(Parser, Debug)]
#[command(
    name = "luapack",
    version,
    about = "Lua bundler with replace/vendor modes"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Bundle a Lua project according to paths/replaces/vendor
    Bundle(BundleCmd),
}

#[derive(Parser, Debug)]
struct BundleCmd {
    /// Entry Lua source file (e.g., lua/main.lua)
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Lua version (informational for now)
    #[arg(long)]
    lua: Option<String>,

    /// First-party bundle roots (Lua-style search paths)
    #[arg(long = "path", value_name = "PATTERN", action = ArgAction::Append)]
    paths: Vec<String>,

    /// Prelude files injected before modules execute (order preserved)
    #[arg(long = "prelude", value_name = "FILE", action = ArgAction::Append)]
    preludes: Vec<PathBuf>,

    /// Replace rules (syntax per docs; parsed later)
    #[arg(long = "replace", value_name = "RULE", action = ArgAction::Append)]
    replace: Vec<String>,

    /// Vendor include roots (syntax per docs; parsed later)
    #[arg(long = "vendor", value_name = "SPEC", action = ArgAction::Append)]
    vendor: Vec<String>,

    /// Output bundle file path
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output: Option<PathBuf>,

    /// Execute this module from the bundle at the end (e.g., core.main)
    #[arg(long = "entry", value_name = "MODULE")]
    entry: Option<String>,

    /// How to bind `require` inside bundled modules: router (default) or global
    #[arg(long = "bind-require", value_enum)]
    bind_require: Option<BindRequire>,

    /// Print extra information about parsing
    #[arg(long = "diagnostics", action = ArgAction::SetTrue, default_value_t = false)]
    diagnostics: bool,

    /// Path to config file; if omitted, auto-discovers luapack.{toml,yaml,yml,json}
    #[arg(long = "config", value_name = "FILE")]
    config: Option<PathBuf>,

    /// Base directory to redact absolute paths in bundle comments
    #[arg(long = "redact-base", value_name = "DIR")]
    redact_base: Option<PathBuf>,
}

fn main() -> ExitCode {
    match real_main() {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {:#}", err);
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Bundle(cmd) => run_bundle(cmd),
    }?;

    // TODO: future steps — watch mode
    Ok(())
}

fn run_bundle(cli: BundleCmd) -> Result<()> {
    // Load configuration (explicit or auto-discovered)
    let loaded = load_config(cli.config.as_deref())?;
    let base = loaded.dir.as_deref();

    // Read entry source
    let code = fs::read_to_string(&cli.input)
        .with_context(|| format!("failed to read input: {}", cli.input.display()))?;

    // Effective options: config < env < CLI (CLI overrides). For booleans, CLI true wins; false doesn't cancel config.
    let lua_ver = cli
        .lua
        .clone()
        .or(loaded.cfg.lua.clone())
        .unwrap_or_else(|| "5.1".to_string());

    // Warn if selected dialect likely unsupported by this build
    {
        let v = lua_ver.to_ascii_lowercase();
        let needs = if v.contains("luajit") {
            Some(("luajit", cfg!(feature = "luajit")))
        } else if v.starts_with("5.4") || v == "54" {
            Some(("lua54", cfg!(feature = "lua54")))
        } else if v.starts_with("5.3") || v == "53" {
            Some(("lua53", cfg!(feature = "lua53")))
        } else if v.starts_with("5.2") || v == "52" {
            Some(("lua52", cfg!(feature = "lua52")))
        } else {
            None // 5.1 baseline
        };
        if let Some((feat, built)) = needs {
            if !built {
                eprintln!(
                    "warning: requested Lua dialect requires feature '{feat}', but this binary was built without it.\n         Rebuild with: cargo build --features {feat}"
                );
            }
        }
    }

    let paths: Vec<String> = if !cli.paths.is_empty() {
        cli.paths.clone()
    } else {
        loaded
            .cfg
            .paths
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|p| {
                // resolve relative to config file dir
                resolve_pathbuf(base, &p).to_string_lossy().to_string()
            })
            .collect()
    };

    let preludes: Vec<PathBuf> = if !cli.preludes.is_empty() {
        cli.preludes.clone()
    } else {
        loaded
            .cfg
            .preludes
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|p| resolve_pathbuf(base, &p))
            .collect()
    };

    let replaces_vec: Vec<String> = if !cli.replace.is_empty() {
        cli.replace.clone()
    } else {
        loaded.cfg.replace.clone().unwrap_or_default()
    };

    let vendors_vec: Vec<String> = if !cli.vendor.is_empty() {
        cli.vendor.clone()
    } else {
        loaded.cfg.vendors.clone().unwrap_or_default()
    };

    let output_path: Option<PathBuf> = if let Some(o) = &cli.output {
        Some(o.clone())
    } else {
        loaded
            .cfg
            .output
            .as_deref()
            .map(|s| resolve_pathbuf(base, s))
    };

    let entry_mod = cli.entry.clone().or(loaded.cfg.entry.clone());

    let bind_mode = if let Some(b) = cli.bind_require {
        b
    } else if let Some(s) = loaded.cfg.bind_require.as_deref() {
        match s.to_ascii_lowercase().as_str() {
            "router" => BindRequire::Router,
            "global" => BindRequire::Global,
            _ => BindRequire::Router,
        }
    } else {
        BindRequire::Router
    };

    let diagnostics = cli.diagnostics || loaded.cfg.diagnostics.unwrap_or(false);

    // Determine redaction base path
    let redact_base_path: Option<PathBuf> = if let Some(b) = &cli.redact_base {
        Some(b.clone())
    } else {
        loaded
            .cfg
            .redact_base
            .as_deref()
            .map(|s| resolve_pathbuf(base, s))
    };

    // Parse replace/vendor flags up-front; use them later for bundling
    let parsed_replaces = parse_replace_rules(&replaces_vec).unwrap_or_default();
    let parsed_vendors = parse_vendor_specs(&vendors_vec).unwrap_or_default();
    // Compute suffix normalization early
    let vendor_paths: Vec<String> = parsed_vendors
        .iter()
        .flat_map(|v| v.paths.iter().cloned())
        .collect();
    let vendor_suffixes: Vec<String> = parsed_vendors
        .iter()
        .flat_map(|v| v.suffixes.iter().cloned())
        .collect();
    let suffixes = infer_suffixes(&paths, &vendor_paths, &vendor_suffixes, &parsed_replaces);
    let normalizer = NameNormalizer::new(suffixes);

    let bundle_opts = BundleOptions {
        lua: lua_ver.clone(),
        paths: paths.clone(),
        preludes: preludes.clone(),
        replaces: parsed_replaces.clone(),
        vendor_specs: parsed_vendors.clone(),
        entry: entry_mod.clone(),
        bind: bind_mode,
        diagnostics,
        redact_base: redact_base_path.clone(),
        normalizer: normalizer.clone(),
    };

    // Diagnostics (optional): show parsed info and simple resolution
    if diagnostics {
        eprintln!("parsed ok: {} (lua={})", cli.input.display(), lua_ver);
        if !paths.is_empty() {
            eprintln!("paths: {}", paths.join(", "));
        }
        if !preludes.is_empty() {
            eprintln!(
                "preludes: {}",
                preludes
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        match &parse_replace_rules(&replaces_vec) {
            Ok(rules) if !rules.is_empty() => {
                eprintln!("replace rules ({}):", rules.len());
                for r in rules {
                    eprintln!(
                        "  match={:?} old={} new={} name={:?} prefix={:?} paths={:?} arg={:?}",
                        r.match_kind, r.old, r.new, r.name, r.prefix, r.paths, r.arg
                    );
                }
            }
            Err(e) => eprintln!("warning: replace parse error: {e}"),
            _ => {}
        }
        match &parse_vendor_specs(&vendors_vec) {
            Ok(specs) if !specs.is_empty() => {
                eprintln!("vendor specs ({}):", specs.len());
                for v in specs {
                    eprintln!(
                        "  paths={:?} exclude_name={:?} exclude_prefix={:?}",
                        v.paths, v.exclude_names, v.exclude_prefixes
                    );
                }
            }
            Err(e) => eprintln!("warning: vendor parse error: {e}"),
            _ => {}
        }
        if let Some(out) = &output_path {
            eprintln!("output: {}", out.display());
        }
        let resolver = ModuleResolver::new(paths.clone());
        let requires = luapack::find_literal_requires(&code);
        if !requires.is_empty() {
            eprintln!("require literals found ({}):", requires.len());
            for r in requires {
                eprintln!(
                    "  {}:{}:{} -> {}",
                    cli.input.display(),
                    r.line,
                    r.col,
                    r.module
                );
                match resolver.resolve(&r.module) {
                    Some(path) => eprintln!("    resolved: {}", path.display()),
                    None => eprintln!("    unresolved with given --path templates"),
                }
            }
            let graph = ModuleGraph::build_from_entry_code(&code, &resolver, &normalizer);
            eprintln!(
                "graph: first_party={} unresolved={}",
                graph.first_party.len(),
                graph.unresolved.len()
            );
            if !graph.unresolved.is_empty() {
                eprintln!("unresolved modules (unique):");
                for m in graph.unresolved.iter() {
                    eprintln!("  - {}", m);
                }
            }
        }
    }

    // If output is requested, emit a minimal bundle: runtime loader + first-party + vendor + root + preludes
    if let Some(out_path) = &output_path {
        let resolver = ModuleResolver::new(paths.clone());
        let graph = ModuleGraph::build_from_entry_code(&code, &resolver, &normalizer);
        let (vendor_mods, vendor_dups) =
            collect_vendor_modules(&parsed_vendors, &parsed_replaces, &normalizer)?;
        if diagnostics && !vendor_dups.is_empty() {
            eprintln!("vendor duplicate module names ({}):", vendor_dups.len());
            for n in vendor_dups {
                eprintln!("  - {}", n);
            }
        }
        if diagnostics {
            eprintln!("vendor included modules: {}", vendor_mods.len());
        }
        let ctx = BundleCtx {
            preludes: &bundle_opts.preludes,
            entry: bundle_opts.entry.as_deref(),
            replaces: &bundle_opts.replaces,
            vendor_mods: &vendor_mods,
            entry_source: &code,
            entry_path: &cli.input,
            bind: bundle_opts.bind,
            resolver: Some(&resolver),
            redact_base: bundle_opts.redact_base.clone(),
            normalizer: &bundle_opts.normalizer,
        };
        let (bundle, total_rewrites) = generate_bundle(&graph, ctx)?;
        if diagnostics {
            eprintln!("bundle literal rewrites: {}", total_rewrites);
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(out_path, bundle)
            .with_context(|| format!("failed to write bundle to {}", out_path.display()))?;
    }

    Ok(())
}
