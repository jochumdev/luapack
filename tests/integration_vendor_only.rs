mod common;

#[test]
fn bundle_vendor_only_includes_vendor_module() {
    let (bundle, _rewrites) = common::bundle_for(
        "tests/fixtures/vendor_only/lua/main.lua",
        &[],
        common::BundleOptions {
            entry_override: None,
            preludes: &[],
            replaces: &[],
            vendor_specs: &["path=tests/fixtures/vendor_only/vendor/lua/?/init.lua".to_string()],
        },
    );

    assert!(
        bundle.contains("__B_MODULES['foo']"),
        "expected vendor module 'foo' present\n{}",
        bundle
    );
    assert!(
        bundle.contains("__B_MODULES['__root']"),
        "expected root module present"
    );

    // Snapshot the entire bundle for regression coverage
    insta::assert_snapshot!(bundle);
}
