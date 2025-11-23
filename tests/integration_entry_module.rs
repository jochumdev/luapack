mod common;

#[test]
fn bundle_returns_specified_entry_module() {
    let (bundle, _rewrites) = common::bundle_for(
        "tests/fixtures/entry_module/lua/main.lua",
        &[
            "tests/fixtures/entry_module/lua/?.lua",
            "tests/fixtures/entry_module/lua/?/init.lua",
        ],
        common::BundleOptions {
            entry_override: Some("core.runner"),
            preludes: &[],
            replaces: &[],
            vendor_specs: &[],
        },
    );

    assert!(bundle.contains("__B_MODULES['core.runner']"));
    assert!(
        bundle
            .trim_end()
            .ends_with("return __B_REQUIRE('core.runner')"),
        "unexpected tail: {}",
        bundle
    );

    // Snapshot the entire bundle for regression coverage
    insta::assert_snapshot!(bundle);
}
