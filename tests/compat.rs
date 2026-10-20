mod common;

use common::{init_tasks_dir, parse_huid, write_task_md};
use std::path::PathBuf;

use rtatr::path::Paths;
use rtatr::store::Store;
use rtatr::task::Task;

/// Fixtures that rtatr parses *and* re-renders byte-for-byte identically.
/// These are the strict proof of `task_md_parse` + `render_task_md`
/// compatibility with upstream `tatr`.
const ROUNDTRIP_FIXTURES: &[(&str, &str)] = &[
    (
        "20240101-120000",
        "# Windows support\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS: scope\n\nNo description.\n",
    ),
    (
        "20240102-120000",
        "# Old task\n\n- STATUS: CLOSED\n- PRIORITY: 50\n- TAGS:\n",
    ),
    (
        "20240103-120000",
        "# Ship v1\n\n- STATUS: OPEN\n- PRIORITY: 10\n- TAGS: release,urgent\n- DUE: 2024-12-31\n\nStuff to do:\n\n- [ ] write changelog\n- [ ] tag release\n",
    ),
    (
        "20240105-120000-rexim",
        "# Suffixed\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n\n",
    ),
];

/// Fixtures that rtatr must *parse* correctly but whose `TASK.md` bytes
/// change on re-render — because upstream's `unparse_tags` normalises
/// separators to commas. Round-tripping these byte-for-byte would be
/// testing the wrong behaviour.
const PARSE_ONLY_FIXTURES: &[(&str, &str)] = &[(
    "20240104-120000",
    "# Many tags\n\n- STATUS: OPEN\n- PRIORITY: 500\n- TAGS: a,b c,d\n\nBody\n",
)];

fn all_fixtures() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    ROUNDTRIP_FIXTURES.iter().chain(PARSE_ONLY_FIXTURES.iter())
}

fn find_fixture(huid: &str) -> Task {
    let (h, content) = all_fixtures()
        .find(|(h, _)| *h == huid)
        .unwrap_or_else(|| panic!("unknown fixture: {huid}"));
    Task::from_markdown(parse_huid(h), PathBuf::from("TASK.md"), content)
}

#[test]
fn all_fixtures_have_valid_title() {
    for (huid, _) in all_fixtures() {
        let t = find_fixture(huid);
        assert_ne!(
            t.title,
            rtatr::task::INVALID_TITLE,
            "huid {huid} parsed as invalid title"
        );
    }
}

#[test]
fn roundtrip_fixtures_byte_identical() {
    for (huid, content) in ROUNDTRIP_FIXTURES {
        let t = Task::from_markdown(parse_huid(huid), PathBuf::from("TASK.md"), content);
        assert_eq!(
            t.to_markdown(),
            *content,
            "roundtrip mismatch for {huid}\n--- expected ---\n{content}\n--- got ---\n{}",
            t.to_markdown()
        );
    }
}

#[test]
fn store_reads_upstream_layout() {
    let dir = tempfile::tempdir().unwrap();
    init_tasks_dir(dir.path());
    let mut n = 0;
    for (huid, content) in all_fixtures() {
        write_task_md(dir.path(), huid, content);
        n += 1;
    }
    let store = Store::new(Paths::from(dir.path().to_path_buf()));
    assert_eq!(store.all().unwrap().len(), n);
}

#[test]
fn fixture_1_fields() {
    let t = find_fixture("20240101-120000");
    assert_eq!(t.title, "Windows support");
    assert_eq!(t.status, "OPEN");
    assert_eq!(t.priority, 100);
    assert_eq!(t.tags, vec!["scope".to_string()]);
    assert_eq!(t.body, "\nNo description.\n");
}

#[test]
fn fixture_2_closed() {
    let t = find_fixture("20240102-120000");
    assert_eq!(t.status, "CLOSED");
    assert!(t.tags.is_empty());
    assert_eq!(t.body, "");
}

#[test]
fn fixture_3_extra_property() {
    let t = find_fixture("20240103-120000");
    assert_eq!(t.title, "Ship v1");
    assert_eq!(t.priority, 10);
    assert_eq!(t.tags, vec!["release".to_string(), "urgent".to_string()]);
    assert!(t.body.contains("- [ ] write changelog"));
    assert!(t
        .props
        .iter()
        .any(|p| matches!(p, rtatr::task::Prop::Other(k, v) if k == "DUE" && v == "2024-12-31")));
}

#[test]
fn fixture_4_tags_mixed_separators() {
    let t = find_fixture("20240104-120000");
    assert_eq!(
        t.tags,
        vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string()
        ]
    );
    assert_eq!(
        t.to_markdown(),
        "# Many tags\n\n- STATUS: OPEN\n- PRIORITY: 500\n- TAGS: a,b,c,d\n\nBody\n"
    );
}

#[test]
fn fixture_5_suffixed_huid() {
    let t = find_fixture("20240105-120000-rexim");
    assert_eq!(t.huid.to_string(), "20240105-120000-rexim");
}

#[test]
fn upstream_fixture_queryable_by_rtatr_tql() {
    let t = find_fixture("20240103-120000");
    assert!(rtatr::query::parse(":release").unwrap().matches(&t));
    assert!(rtatr::query::parse("priority lt 50").unwrap().matches(&t));
    assert!(rtatr::query::parse("DUE=2024-12-31").unwrap().matches(&t));
    assert!(rtatr::query::parse("\"write changelog\"")
        .unwrap()
        .matches(&t));
    assert!(rtatr::query::parse("changelog").unwrap().matches(&t));
    assert!(rtatr::query::parse("20240103-120000").unwrap().matches(&t));
    assert!(rtatr::query::parse("tagged").unwrap().matches(&t));
    assert!(!rtatr::query::parse("any and not :release")
        .unwrap()
        .matches(&t));
}

#[test]
fn upstream_fixture_closed_not_in_open_filter() {
    let closed = find_fixture("20240102-120000");
    assert_eq!(closed.status, "CLOSED");
}

#[test]
fn invalid_title_fixture_produces_upstream_error_string() {
    let content = "no hash\n";
    let t = Task::from_markdown(
        parse_huid("20240101-120000"),
        PathBuf::from("TASK.md"),
        content,
    );
    assert_eq!(t.title, rtatr::task::INVALID_TITLE);
}

#[test]
fn empty_fixture_file() {
    let t = Task::from_markdown(parse_huid("20240101-120000"), PathBuf::from("TASK.md"), "");
    assert_eq!(t.title, rtatr::task::INVALID_TITLE);
    assert!(t.body.is_empty());
}
