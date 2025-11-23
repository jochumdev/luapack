use luapack::*;

#[test]
fn no_rewrite_when_require_shadowed() {
    let code = r#"
        local require = function(x) return x end
        local x = require('bar.z')
    "#;
    let rules = vec![ReplaceRule {
        match_kind: MatchKind::Prefix,
        old: "require".into(),
        new: "bar_require".into(),
        name: None,
        prefix: Some("bar.".into()),
        paths: vec![],
        arg: ArgMode::Rest,
    }];
    let nrm = NameNormalizer::new(std::collections::HashSet::new());
    let (_out, n) = transform_requires(code, &rules, None, None, &nrm);
    assert_eq!(n, 0, "should not rewrite shadowed require");
}

#[test]
fn no_rewrite_with_extra_args() {
    let code = "require('bar.z', 123)";
    let rules = vec![ReplaceRule {
        match_kind: MatchKind::Prefix,
        old: "require".into(),
        new: "bar_require".into(),
        name: None,
        prefix: Some("bar.".into()),
        paths: vec![],
        arg: ArgMode::Rest,
    }];
    let nrm = NameNormalizer::new(std::collections::HashSet::new());
    let (_out, n) = transform_requires(code, &rules, None, None, &nrm);
    assert_eq!(n, 0, "should not rewrite call with extra args");
}

#[test]
fn rewrite_string_arg_form() {
    let code = "return require 'bar.alpha'";
    let rules = vec![ReplaceRule {
        match_kind: MatchKind::Prefix,
        old: "require".into(),
        new: "bar_require".into(),
        name: None,
        prefix: Some("bar.".into()),
        paths: vec![],
        arg: ArgMode::Rest,
    }];
    let nrm = NameNormalizer::new(std::collections::HashSet::new());
    let (out, n) = transform_requires(code, &rules, None, None, &nrm);
    assert!(n >= 1);
    assert!(out.contains("bar_require"), "output: {}", out);
    assert!(
        !out.contains("require('bar.alpha')") && !out.contains("require 'bar.alpha'"),
        "output: {}",
        out
    );
}
