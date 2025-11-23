use luapack::*;

#[test]
fn parse_replace_rules_prefix_and_exact() {
    let flags = vec![
        "match=prefix,prefix=bar.,new=bar_require,arg={rest}".to_string(),
        "match=exact,name=core.greet,new=greet_require,arg={full}".to_string(),
    ];
    let rules = parse_replace_rules(&flags).expect("parse");
    assert_eq!(rules.len(), 2);

    // prefix
    let r0 = &rules[0];
    assert!(matches!(r0.match_kind, MatchKind::Prefix));
    assert_eq!(r0.prefix.as_deref(), Some("bar."));

    // exact
    let r1 = &rules[1];
    assert!(matches!(r1.match_kind, MatchKind::Exact));
    assert_eq!(r1.name.as_deref(), Some("core.greet"));
}

#[test]
fn matches_replace_basic() {
    let rules = vec![
        ReplaceRule {
            match_kind: MatchKind::Exact,
            old: "require".into(),
            new: "g".into(),
            name: Some("core.greet".into()),
            prefix: None,
            paths: vec![],
            arg: ArgMode::Full,
        },
        ReplaceRule {
            match_kind: MatchKind::Prefix,
            old: "require".into(),
            new: "b".into(),
            name: None,
            prefix: Some("bar.".into()),
            paths: vec![],
            arg: ArgMode::Rest,
        },
    ];
    let normalizer = NameNormalizer::new(Default::default());
    assert!(matches_replace("core.greet", &rules, &normalizer));
    assert!(matches_replace("bar.x", &rules, &normalizer));
    assert!(!matches_replace("baz", &rules, &normalizer));
}

#[test]
fn parse_replace_rules_invalid_match_kind() {
    let flags = vec!["match=unknown,new=x".to_string()];
    let err = parse_replace_rules(&flags).unwrap_err();
    assert!(format!("{err}").contains("unknown match kind"));
}
