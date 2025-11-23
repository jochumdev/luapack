mod common;

#[test]
fn bundle_includes_init_module() {
    let (bundle, _rewrites) = common::bundle_for(
        "tests/fixtures/init_search/lua/main.lua",
        &[
            "tests/fixtures/init_search/lua/?.lua",
            "tests/fixtures/init_search/lua/?/init.lua",
        ],
        common::BundleOptions {
            entry_override: None,
            preludes: &[],
            replaces: &[],
            vendor_specs: &[],
        },
    );

    assert!(
        bundle.contains("__B_MODULES['pkg']"),
        "expected module from init search present\n{}",
        bundle
    );

    // Snapshot the entire bundle for regression coverage
    insta::assert_snapshot!(bundle);
}
