#![allow(dead_code)]

use std::path::PathBuf;

use luapack::*;

pub fn manifest_path(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.join(rel).to_string_lossy().into_owned()
}

pub fn mk_resolver(paths: Vec<String>) -> ModuleResolver {
    ModuleResolver::new(paths)
}

pub fn build_graph(
    entry_path: &str,
    paths: &[&str],
    normalizer: &NameNormalizer,
) -> (PathBuf, String, ModuleResolver, ModuleGraph) {
    let entry = PathBuf::from(manifest_path(entry_path));
    let code = std::fs::read_to_string(&entry).expect("read entry");
    let resolver = mk_resolver(paths.iter().map(|p| manifest_path(p)).collect());
    let graph = ModuleGraph::build_from_entry_code(&code, &resolver, normalizer);
    (entry, code, resolver, graph)
}

pub struct BundleOptions<'a> {
    pub entry_override: Option<&'a str>,
    pub preludes: &'a [PathBuf],
    pub replaces: &'a [ReplaceRule],
    pub vendor_specs: &'a [String],
}

pub fn bundle_for(entry_path: &str, paths: &[&str], opts: BundleOptions<'_>) -> (String, usize) {
    // Parse vendor specs and compute suffix normalizer
    let abs_vendor_specs: Vec<String> = opts
        .vendor_specs
        .iter()
        .map(|s| {
            // Absolutize all path=... occurrences within the spec string
            let mut out_parts: Vec<String> = Vec::new();
            for part in s.split(',') {
                if let Some((k, v)) = part.split_once('=') {
                    if k.trim() == "path" {
                        out_parts.push(format!("path={}", manifest_path(v.trim())));
                    } else {
                        out_parts.push(part.to_string());
                    }
                } else {
                    // bare value, treat as path
                    out_parts.push(manifest_path(part.trim()));
                }
            }
            out_parts.join(",")
        })
        .collect();
    let vendor_specs = parse_vendor_specs(&abs_vendor_specs).expect("parse vendor specs");
    let vendor_paths: Vec<String> = vendor_specs.iter().flat_map(|v| v.paths.clone()).collect();
    let vendor_suffixes: Vec<String> = vendor_specs
        .iter()
        .flat_map(|v| v.suffixes.clone())
        .collect();
    let suffixes = infer_suffixes(&[], &vendor_paths, &vendor_suffixes, opts.replaces);
    let normalizer = NameNormalizer::new(suffixes);

    let (entry, code, resolver, graph) = build_graph(entry_path, paths, &normalizer);
    // Stable path redaction for snapshots
    let redact_base = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let (vendor_mods, _dups) = collect_vendor_modules(&vendor_specs, opts.replaces, &normalizer)
        .expect("collect vendor modules");
    let ctx = BundleCtx {
        preludes: opts.preludes,
        entry: opts.entry_override,
        replaces: opts.replaces,
        vendor_mods: &vendor_mods,
        entry_source: &code,
        entry_path: &entry,
        bind: _BindRequireExport::Router,
        resolver: Some(&resolver),
        redact_base,
        normalizer: &normalizer,
    };
    generate_bundle(&graph, ctx).expect("bundle")
}
