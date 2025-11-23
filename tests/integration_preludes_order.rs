use std::path::PathBuf;

mod common;

#[test]
fn preludes_appear_in_order() {
    let preludes = &[
        PathBuf::from(common::manifest_path(
            "tests/fixtures/preludes_order/prelude1.lua",
        )),
        PathBuf::from(common::manifest_path(
            "tests/fixtures/preludes_order/prelude2.lua",
        )),
    ];

    let (bundle, _rewrites) = common::bundle_for(
        "tests/fixtures/preludes_order/lua/main.lua",
        &[],
        common::BundleOptions {
            entry_override: None,
            preludes,
            replaces: &[],
            vendor_specs: &[],
        },
    );

    // Paths in bundle comments are redacted to be relative to the base; search by filename
    let p1 = bundle.find("prelude1.lua").unwrap();
    let p2 = bundle.find("prelude2.lua").unwrap();
    assert!(p1 < p2, "preludes not in order: {}", bundle);
}
