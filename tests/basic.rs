use std::collections::HashMap;
use std::path::PathBuf;

use luapack::*;

#[test]
fn transform_require_prefix_rewrite_integration() {
    let code = "local t = require('bar.common.tablex')\n";
    let rules = vec![ReplaceRule {
        match_kind: MatchKind::Prefix,
        old: "require".into(),
        new: "bar_require".into(),
        name: None,
        prefix: Some("bar.".into()),
        paths: vec![],
        arg: ArgMode::Rest,
    }];
    let normalizer = NameNormalizer::new(Default::default());
    let (out, n) = transform_requires(code, &rules, None, None, &normalizer);
    assert!(n >= 1, "expected at least 1 rewrite, got {}", n);
    assert!(
        out.contains("bar_require('common.tablex')")
            || out.contains("bar_require(\"common.tablex\")"),
        "output was: {}",
        out
    );
}

#[test]
fn bundle_simple_smoke_integration() {
    // Use fixtures/replace main
    let entry = PathBuf::from(format!(
        "{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        "tests/fixtures/replace/lua/main.lua"
    ));
    let code = std::fs::read_to_string(&entry).expect("read entry");
    let resolver = ModuleResolver::new(vec![
        format!(
            "{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            "tests/fixtures/replace/lua/?.lua"
        ),
        format!(
            "{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            "tests/fixtures/replace/lua/?/init.lua"
        ),
    ]);
    let normalizer = NameNormalizer::new(Default::default());
    let graph = ModuleGraph::build_from_entry_code(&code, &resolver, &normalizer);
    let ctx = BundleCtx {
        preludes: &[],
        entry: None,
        replaces: &[],
        vendor_mods: &HashMap::new(),
        entry_source: &code,
        entry_path: &entry,
        bind: _BindRequireExport::Router,
        resolver: Some(&resolver),
        redact_base: None,
        normalizer: &normalizer,
    };
    let (bundle, _rewrites) = generate_bundle(&graph, ctx).expect("bundle");
    assert!(
        bundle.contains("__B_MODULES['__root']"),
        "bundle missing root module"
    );
    assert!(
        bundle.contains("require(\"bar.common.tablex\")")
            || bundle.contains("require('bar.common.tablex')"),
        "root require should remain when no replaces, got: {}",
        &bundle
    );
}
