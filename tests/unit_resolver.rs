use luapack::*;

#[test]
fn dotted_to_path_and_resolve() {
    assert_eq!(ModuleResolver::dotted_to_path("a.b.c"), "a/b/c");

    // Use fixtures tree to test resolve
    let resolver = ModuleResolver::new(vec![
        format!(
            "{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            "tests/fixtures/replace_path/lua/?.lua"
        ),
        format!(
            "{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            "tests/fixtures/replace_path/lua/?/init.lua"
        ),
    ]);
    let p = resolver.resolve("x.y");
    assert!(
        p.is_some(),
        "expected to resolve x.y in fixtures/replace_path"
    );
    assert!(p
        .unwrap()
        .to_string_lossy()
        .ends_with("tests/fixtures/replace_path/lua/x/y.lua"));

    // Negative case
    let none = resolver.resolve("not.present");
    assert!(none.is_none());
}
