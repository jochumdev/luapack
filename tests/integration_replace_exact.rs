use luapack::parse_replace_rules;

mod common;

#[test]
fn bundle_rewrites_exact_in_root() {
    let flags = vec!["match=exact,name=core.greet,new=greet_require".to_string()];
    let rules = parse_replace_rules(&flags).expect("parse");

    let (bundle, rewrites) = common::bundle_for(
        "tests/fixtures/replace_exact/lua/main.lua",
        &[],
        common::BundleOptions {
            entry_override: None,
            preludes: &[],
            replaces: &rules,
            vendor_specs: &[],
        },
    );

    assert!(rewrites >= 1, "expected at least one rewrite");
    assert!(
        bundle.contains("greet_require('core.greet')")
            || bundle.contains("greet_require(\"core.greet\")"),
        "bundle: {}",
        bundle
    );

    // Snapshot the entire bundle for regression coverage
    insta::assert_snapshot!(bundle);
}
