use luapack::parse_replace_rules;

mod common;

#[test]
fn bundle_rewrites_prefix_in_root() {
    let flags = vec!["match=prefix,prefix=bar.,new=bar_require,arg={rest}".to_string()];
    let rules = parse_replace_rules(&flags).expect("parse");

    let (bundle, rewrites) = common::bundle_for(
        "tests/fixtures/replace_prefix/lua/main.lua",
        &[
            "tests/fixtures/replace_prefix/lua/?.lua",
            "tests/fixtures/replace_prefix/lua/?/init.lua",
        ],
        common::BundleOptions {
            entry_override: None,
            preludes: &[],
            replaces: &rules,
            vendor_specs: &[],
        },
    );

    assert!(rewrites >= 1, "expected at least one rewrite");
    assert!(
        bundle.contains("bar_require('common.tablex')")
            || bundle.contains("bar_require(\"common.tablex\")"),
        "bundle: {}",
        bundle
    );

    // Snapshot the entire bundle for regression coverage
    insta::assert_snapshot!(bundle);
}
