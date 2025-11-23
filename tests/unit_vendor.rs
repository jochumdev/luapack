use luapack::*;

mod common;

#[test]
fn parse_vendor_specs_bare_and_exclude() {
    let flags = vec![
        "path=vendor/?.lua,exclude=name:foo,exclude=prefix:bar.".to_string(),
        "vendor/?/init.lua".to_string(),
    ];
    let specs = parse_vendor_specs(&flags).expect("parse");
    assert_eq!(specs.len(), 2);
    assert_eq!(specs[0].paths.len(), 1);
    assert_eq!(specs[0].exclude_names, vec!["foo"]);
    assert_eq!(specs[0].exclude_prefixes, vec!["bar."]);
    assert_eq!(specs[1].paths, vec!["vendor/?/init.lua".to_string()]);
}

#[test]
fn vendor_suffix_normalization_dedups_init() {
    // Build absolute vendor specs pointing at the example vendor tree
    let p1 = format!(
        "path={}",
        common::manifest_path("examples/simple/vendor/lua/?.lua")
    );
    let p2 = format!(
        "path={}",
        common::manifest_path("examples/simple/vendor/lua/?/init.lua")
    );
    let flags = vec![p1, p2];
    let specs = parse_vendor_specs(&flags).expect("parse vendor specs");

    // Infer suffixes from templates (should infer 'init') and build normalizer
    let vendor_paths: Vec<String> = specs.iter().flat_map(|v| v.paths.clone()).collect();
    let vendor_suffixes: Vec<String> = specs.iter().flat_map(|v| v.suffixes.clone()).collect();
    let suffixes = infer_suffixes(&[], &vendor_paths, &vendor_suffixes, &[]);
    let nrm = NameNormalizer::new(suffixes);

    // Collect modules and ensure 'mock_recoil' appears only once without '.init'
    let (mods, _dups) = collect_vendor_modules(&specs, &[], &nrm).expect("collect vendor");
    assert!(
        mods.contains_key("mock_recoil"),
        "expected normalized name present"
    );
    assert!(
        !mods.contains_key("mock_recoil.init"),
        "did not expect unnormalized name"
    );
}

#[test]
fn to_glob_and_root_cases() {
    let (g1, root1, init1) = to_glob_and_root("a/b/?.lua");
    assert_eq!(g1, "a/b/**/*.lua");
    assert_eq!(root1.to_string_lossy(), "a/b/");
    assert!(!init1);

    let (g2, root2, init2) = to_glob_and_root("a/b/?/init.lua");
    assert_eq!(g2, "a/b/**/init.lua");
    assert_eq!(root2.to_string_lossy(), "a/b/");
    assert!(init2);
}
