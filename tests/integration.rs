use rtatr::{query::parse, Task};

fn task_with(tags: &[&str], priority: i32, body: &str) -> Task {
    let mut t = Task::new(
        "20240101-120000".parse().unwrap(),
        "t".into(),
        priority,
        tags.iter().map(|s| s.to_string()).collect(),
    );
    t.body = body.into();
    t
}

// --- Upstream query syntax ----------------------------------------------

#[test]
fn upstream_any_matches_everything() {
    let q = parse("any").unwrap();
    assert!(q.matches(&task_with(&[], 100, "")));
    assert!(q.matches(&task_with(&["x"], 100, "hi")));
}

#[test]
fn upstream_tagged() {
    let q = parse("tagged").unwrap();
    assert!(!q.matches(&task_with(&[], 100, "")));
    assert!(q.matches(&task_with(&["a"], 100, "")));
}

#[test]
fn upstream_priority_comparisons() {
    let low = task_with(&["x"], 50, "");
    let high = task_with(&["x"], 200, "");
    for (q, ok_low, ok_high) in [
        ("priority lt 100", true, false),
        ("priority le 50", true, false),
        ("priority gt 100", false, true),
        ("priority ge 200", false, true),
        ("priority eq 50", true, false),
        ("priority ne 50", false, true),
        ("100 lt priority", false, true),
        ("50 eq priority", true, false),
    ] {
        let q = parse(q).unwrap();
        assert_eq!(q.matches(&low), ok_low, "query {q:?} vs low-priority task");
        assert_eq!(
            q.matches(&high),
            ok_high,
            "query {q:?} vs high-priority task"
        );
    }
}

#[test]
fn upstream_huid_primary() {
    let t = task_with(&[], 100, "");
    let h = t.huid.to_string();
    assert!(parse(&h).unwrap().matches(&t));
    assert!(!parse("20240101-120001").unwrap().matches(&t));
    let t2 = Task::new(
        "20240101-120000-rexim".parse().unwrap(),
        "x".into(),
        100,
        vec![],
    );
    assert!(parse("20240101-120000-rexim").unwrap().matches(&t2));
}

#[test]
fn upstream_square_brackets() {
    let q = parse("[:a or :b] and :c").unwrap();
    assert!(q.matches(&task_with(&["a", "c"], 100, "")));
    assert!(!q.matches(&task_with(&["a"], 100, "")));
}

#[test]
fn upstream_parens_rejected() {
    assert!(parse("(:a)").is_err());
    assert!(parse(":a and (:b or :c)").is_err());
}

#[test]
fn upstream_not_on_int_errors() {
    assert!(parse("not priority").is_err());
    assert!(parse("not 50").is_err());
}

#[test]
fn upstream_cmp_on_bools_errors() {
    assert!(parse(":a lt :b").is_err());
    assert!(parse("tagged gt 0").is_err());
}

#[test]
fn upstream_int_alone_errors() {
    assert!(parse("50").is_err());
    assert!(parse("priority").is_err());
}

#[test]
fn upstream_comparison_precedence() {
    // and binds tighter than or, cmp tighter than and.
    let q = parse("priority lt 50 and :urgent or :done").unwrap();
    // (priority<50 AND urgent) OR done
    assert!(q.matches(&task_with(&["done"], 999, "")));
    assert!(q.matches(&task_with(&["urgent"], 10, "")));
    assert!(!q.matches(&task_with(&["urgent"], 100, "")));
    assert!(!q.matches(&task_with(&[], 10, "")));
}

// --- rtatr extensions still work ----------------------------------------

#[test]
fn fork_text_search_still_works() {
    let t = task_with(&[], 100, "the login bug is back");
    assert!(parse("\"login bug\"").unwrap().matches(&t));
    assert!(parse("login").unwrap().matches(&t));
    assert!(!parse("logout").unwrap().matches(&t));
}

#[test]
fn fork_prop_filter_still_works() {
    let t = task_with(&[], 100, "");
    assert!(parse("PRIORITY=100").unwrap().matches(&t));
    assert!(!parse("PRIORITY=50").unwrap().matches(&t));
    assert!(parse("TAGS=urgent")
        .unwrap()
        .matches(&task_with(&["urgent"], 100, "")));
    assert!(parse("hello")
        .unwrap()
        .matches(&task_with(&[], 100, "hello world")));
    assert!(!parse("hello").unwrap().matches(&t));
}
#[test]
fn fork_tag_syntax_unchanged() {
    let q = parse(":a and not :b").unwrap();
    assert!(q.matches(&task_with(&["a"], 100, "")));
    assert!(!q.matches(&task_with(&["a", "b"], 100, "")));
}
