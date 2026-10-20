mod common;

use common::task;
use rtatr::query::parse;
use rtatr::task::{Prop, Task};

fn t(tags: &[&str]) -> Task {
    task("20240101-120000", tags, 100, "")
}

#[test]
fn empty_query_is_true() {
    assert!(parse("").unwrap().matches(&t(&[])));
}

#[test]
fn tag_match() {
    let q = parse(":a").unwrap();
    assert!(q.matches(&t(&["a"])));
    assert!(!q.matches(&t(&["b"])));
    assert!(!q.matches(&t(&[])));
}

#[test]
fn tag_with_special_chars() {
    let task = t(&["foo.bar/baz", "@user"]);
    assert!(parse(":foo.bar/baz").unwrap().matches(&task));
    assert!(parse(":@user").unwrap().matches(&task));
}

#[test]
fn empty_tag_errors() {
    assert!(parse(":").is_err());
}

#[test]
fn not_operator() {
    let q = parse("not :a").unwrap();
    assert!(!q.matches(&t(&["a"])));
    assert!(q.matches(&t(&["b"])));
    assert!(q.matches(&t(&[])));
}

#[test]
fn and_or_precedence() {
    // :a and :b or :c -> (:a and :b) or :c
    let q = parse(":a and :b or :c").unwrap();
    assert!(q.matches(&t(&["c"])));
    assert!(q.matches(&t(&["a", "b"])));
    assert!(!q.matches(&t(&["a"])));
    assert!(!q.matches(&t(&["b"])));
}

#[test]
fn brackets_grouping() {
    let q = parse(":a and [:b or :c]").unwrap();
    assert!(q.matches(&t(&["a", "c"])));
    assert!(!q.matches(&t(&["c"])));
    assert!(!q.matches(&t(&["a"])));
}

#[test]
fn nested_brackets() {
    let q = parse("[[[:a or :b] and :c] or :d]").unwrap();
    assert!(q.matches(&t(&["d"])));
    assert!(q.matches(&t(&["a", "c"])));
    assert!(!q.matches(&t(&["a"])));
}

#[test]
fn parens_rejected() {
    assert!(parse("(:a)").is_err());
    assert!(parse(":a and (:b or :c)").is_err());
}

#[test]
fn parens_error_hints_at_brackets() {
    let err = parse("(:a)").unwrap_err();
    assert!(err.contains('[') && err.contains(']'), "error: {err}");
}

#[test]
fn unbalanced_brackets() {
    assert!(parse("[:a").is_err());
    assert!(parse(":a]").is_err());
}

#[test]
fn trailing_operator() {
    assert!(parse(":a and").is_err());
    assert!(parse(":a or").is_err());
    assert!(parse("and :a").is_err());
}

#[test]
fn keyword_any() {
    let q = parse("any").unwrap();
    assert!(q.matches(&t(&[])));
    assert!(q.matches(&t(&["x"])));
}

#[test]
fn keyword_tagged() {
    let q = parse("tagged").unwrap();
    assert!(!q.matches(&t(&[])));
    assert!(q.matches(&t(&["x"])));
}

#[test]
fn keyword_priority_alone_errors() {
    assert!(parse("priority").is_err());
}

#[test]
fn integer_alone_errors() {
    assert!(parse("50").is_err());
    assert!(parse("-5").is_err());
}

#[test]
fn comparison_priority_vs_literal() {
    let low = task("20240101-120000", &["x"], 50, "");
    let high = task("20240101-120000", &["x"], 200, "");

    for (src, ok_low, ok_high) in [
        ("priority lt 100", true, false),
        ("priority le 50", true, false),
        ("priority gt 100", false, true),
        ("priority ge 200", false, true),
        ("priority eq 50", true, false),
        ("priority ne 50", false, true),
        ("100 gt priority", true, false),
        ("50 eq priority", true, false),
    ] {
        let q = parse(src).unwrap();
        assert_eq!(q.matches(&low), ok_low, "{src} vs low");
        assert_eq!(q.matches(&high), ok_high, "{src} vs high");
    }
}

#[test]
fn comparison_literal_vs_literal() {
    assert!(parse("5 lt 10").unwrap().matches(&t(&[])));
    assert!(!parse("10 lt 5").unwrap().matches(&t(&[])));
}

#[test]
fn comparison_chain_errors() {
    assert!(parse("1 lt 2 lt 3").is_err());
}

#[test]
fn comparison_on_bool_errors() {
    assert!(parse(":a lt :b").is_err());
    assert!(parse("tagged lt 0").is_err());
    assert!(parse("any eq tagged").is_err());
}

#[test]
fn not_on_integer_errors() {
    assert!(parse("not priority").is_err());
    assert!(parse("not 5").is_err());
}

#[test]
fn huid_primary() {
    let task = task("20240101-120000", &[], 100, "");
    assert!(parse("20240101-120000").unwrap().matches(&task));
    assert!(!parse("20240101-120001").unwrap().matches(&task));
}

#[test]
fn huid_primary_with_suffix() {
    let task = task("20240101-120000-rexim", &[], 100, "");
    assert!(parse("20240101-120000-rexim").unwrap().matches(&task));
    assert!(!parse("20240101-120000-other").unwrap().matches(&task));
}

#[test]
fn huid_primary_lexical_only() {
    let q = parse("20249999-999999").unwrap();
    assert!(matches!(q, rtatr::query::Query::HuidMatch(_)));

    assert!(!q.matches(&task("20240101-120000", &[], 100, "")));
}
#[test]
fn comparison_precedence_with_and_or() {
    // priority lt 50 and :urgent or :done
    // -> (priority lt 50 and :urgent) or :done
    let q = parse("priority lt 50 and :urgent or :done").unwrap();
    assert!(q.matches(&task("20240101-120000", &["done"], 999, "")));
    assert!(q.matches(&task("20240101-120000", &["urgent"], 10, "")));
    assert!(!q.matches(&task("20240101-120000", &["urgent"], 100, "")));
    assert!(!q.matches(&task("20240101-120000", &[], 10, "")));
}

#[test]
fn text_search_quoted() {
    let task = task("20240101-120000", &[], 100, "the login bug is back");
    assert!(parse("\"login bug\"").unwrap().matches(&task));
    assert!(!parse("\"logout\"").unwrap().matches(&task));
}

#[test]
fn text_search_bare() {
    let task = task("20240101-120000", &[], 100, "the login bug is back");
    assert!(parse("login").unwrap().matches(&task));
    assert!(!parse("logout").unwrap().matches(&task));
}

#[test]
fn text_search_case_insensitive() {
    let task = task("20240101-120000", &[], 100, "Login Bug");
    assert!(parse("login").unwrap().matches(&task));
    assert!(parse("LOGIN").unwrap().matches(&task));
}

#[test]
fn text_search_ignores_markdown() {
    let task = task("20240101-120000", &[], 100, "# Heading\n- item\n");
    assert!(parse("Heading").unwrap().matches(&task));
    assert!(parse("item").unwrap().matches(&task));
}

#[test]
fn prop_filter_equality() {
    let task = task("20240101-120000", &["urgent"], 100, "");
    assert!(parse("PRIORITY=100").unwrap().matches(&task));
    assert!(!parse("PRIORITY=50").unwrap().matches(&task));
    assert!(parse("STATUS=OPEN").unwrap().matches(&task));
    assert!(!parse("STATUS=CLOSED").unwrap().matches(&task));
}

#[test]
fn prop_filter_tags() {
    let task_ = task("20240101-120000", &["urgent", "work"], 100, "");

    assert!(parse("TAGS=\"urgent,work\"").unwrap().matches(&task_));
    assert!(!parse("TAGS=\"work,urgent\"").unwrap().matches(&task_));

    let single = task("20240101-120000", &["urgent"], 100, "");
    assert!(parse("TAGS=urgent").unwrap().matches(&single));
    assert!(!parse("TAGS=urgent").unwrap().matches(&task_));
}

#[test]
fn prop_filter_other_prop() {
    let mut task = task("20240101-120000", &[], 100, "");
    task.props
        .push(Prop::Other("DUE".into(), "2024-12-31".into()));
    assert!(parse("DUE=2024-12-31").unwrap().matches(&task));
    assert!(!parse("DUE=2025-01-01").unwrap().matches(&task));
}

#[test]
fn prop_filter_missing_does_not_match() {
    let task = task("20240101-120000", &[], 100, "");
    assert!(!parse("DUE=anything").unwrap().matches(&task));
}

#[test]
#[ignore = "known incompatibility: upstream accepts +N via strtol, rtatr does not yet"]
fn positive_integer_accepted() {
    assert!(parse("priority lt +5").is_ok());
}
