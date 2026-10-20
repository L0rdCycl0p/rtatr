mod common;

use common::parse_huid;
use std::path::PathBuf;

use rtatr::task::{Prop, Task, DEFAULT_STATUS, INVALID_TITLE, MISSING_PRIORITY, NEW_TASK_PRIORITY};

fn p() -> PathBuf {
    PathBuf::from("TASK.md")
}

fn parse(content: &str) -> Task {
    Task::from_markdown(parse_huid("20240101-120000"), p(), content)
}

#[test]
fn parse_minimal_upstream() {
    let t = parse("# Title\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS: a,b\n\nbody\n");
    assert_eq!(t.title, "Title");
    assert_eq!(t.status, "OPEN");
    assert_eq!(t.priority, 100);
    assert_eq!(t.tags, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(t.body, "\nbody\n");
}

#[test]
fn roundtrip_minimal() {
    let content = "# Title\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS: a,b\n\nbody\n";
    assert_eq!(parse(content).to_markdown(), content);
}

#[test]
fn roundtrip_empty_body() {
    let content = "# Title\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n";
    let t = parse(content);
    assert_eq!(t.body, "");
    assert_eq!(t.to_markdown(), content);
}

#[test]
fn roundtrip_extra_property() {
    let content =
        "# Title\n\n- STATUS: OPEN\n- PRIORITY: 50\n- TAGS: a\n- DUE: 2024-12-31\n\nBody\n";
    assert_eq!(parse(content).to_markdown(), content);
}

#[test]
fn missing_status_appends_at_end() {
    let content = "# Title\n\n- PRIORITY: 100\n- TAGS:\n\nBody\n";
    let t = parse(content);
    assert_eq!(t.status, DEFAULT_STATUS);
    assert_eq!(
        t.to_markdown(),
        "# Title\n\n- PRIORITY: 100\n- TAGS:\n- STATUS: OPEN\n\nBody\n"
    );
}

#[test]
fn missing_priority_defaults() {
    let t = parse("# T\n\n- STATUS: OPEN\n- TAGS:\n\nB\n");
    assert_eq!(t.priority, MISSING_PRIORITY);
}

#[test]
fn missing_tags_defaults() {
    let t = parse("# T\n\n- STATUS: OPEN\n- PRIORITY: 100\n\nB\n");
    assert!(t.tags.is_empty());
}

#[test]
fn all_props_missing_defaults_and_appends() {
    let t = parse("# Title\n\nBody\n");
    assert_eq!(t.status, DEFAULT_STATUS);
    assert_eq!(t.priority, MISSING_PRIORITY);
    assert!(t.tags.is_empty());
    assert_eq!(
        t.to_markdown(),
        "# Title\n\n- STATUS: OPEN\n- PRIORITY: 999999\n- TAGS:\nBody\n"
    );
}

#[test]
fn invalid_title_matches_upstream() {
    let t = parse("no heading here\n\n- STATUS: OPEN\n");
    assert_eq!(t.title, INVALID_TITLE);
    assert_eq!(t.status, DEFAULT_STATUS);
    assert_eq!(t.priority, MISSING_PRIORITY);
    assert!(t.tags.is_empty());
    assert_eq!(t.body, "");
}

#[test]
fn new_renders_upstream_format() {
    let t = Task::new(
        parse_huid("20240101-120000"),
        "Fix it".into(),
        NEW_TASK_PRIORITY,
        vec!["a".into(), "b".into()],
    );
    assert_eq!(
        t.to_markdown(),
        "# Fix it\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS: a,b\n"
    );
}

#[test]
fn tags_split_various() {
    let cases: &[(&str, &[&str])] = &[
        ("a,b", &["a", "b"]),
        ("a, b", &["a", "b"]),
        ("a b", &["a", "b"]),
        ("a,,,b", &["a", "b"]),
        ("a,,, b", &["a", "b"]),
        ("", &[]),
        ("   ", &[]),
        ("a", &["a"]),
    ];
    for (input, expected) in cases {
        let content = format!("# T\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS: {input}\n");
        let t = parse(&content);
        let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        assert_eq!(t.tags, expected, "input: {input:?}");
    }
}

#[test]
fn tags_keep_duplicates() {
    let t = parse("# T\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS: a,a,b\n");
    assert_eq!(
        t.tags,
        vec!["a".to_string(), "a".to_string(), "b".to_string()]
    );
}

#[test]
fn priority_parsed_via_atoi() {
    for (raw, expected) in [
        ("100", 100),
        ("-5", -5),
        ("+5", 5),
        ("100abc", 100),
        ("abc", 0),
        ("", 0),
    ] {
        let content = format!("# T\n\n- STATUS: OPEN\n- PRIORITY: {raw}\n- TAGS:\n");
        assert_eq!(parse(&content).priority, expected, "input: {raw:?}");
    }
}

#[test]
fn duplicate_property_last_wins() {
    let content = "# T\n\n- DUE: first\n- STATUS: OPEN\n- DUE: second\n- PRIORITY: 100\n- TAGS:\n";
    let t = parse(content);

    let due_entries: Vec<_> = t
        .props
        .iter()
        .filter(|p| matches!(p, Prop::Other(k, _) if k == "DUE"))
        .collect();
    assert_eq!(due_entries.len(), 1);

    match due_entries[0] {
        Prop::Other(_, v) => assert_eq!(v, "second"),
        _ => unreachable!(),
    }

    assert!(matches!(t.props[0], Prop::Status));
    assert!(matches!(&t.props[1], Prop::Other(k, _) if k == "DUE"));
    assert!(matches!(t.props[2], Prop::Priority));
    assert!(matches!(t.props[3], Prop::Tags));

    assert_eq!(
        t.to_markdown(),
        "# T\n\n- STATUS: OPEN\n- DUE: second\n- PRIORITY: 100\n- TAGS:\n"
    );
}

#[test]
fn body_preserved_verbatim() {
    let content =
        "# T\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n\n## Sub\n\n- one\n- two\n\nTrailing.\n";
    let t = parse(content);
    assert_eq!(t.body, "\n## Sub\n\n- one\n- two\n\nTrailing.\n");
    assert_eq!(t.to_markdown(), content);
}

#[test]
fn title_with_trailing_spaces_preserved() {
    let t = parse("# Title   \n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n");
    assert_eq!(t.title, "Title   ");
}

#[test]
fn title_empty() {
    let t = parse("#\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n");
    assert_eq!(t.title, "");
}

#[test]
fn leading_whitespace_before_title() {
    let t = parse("   # Title\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n");
    assert_eq!(t.title, "Title");
}
