mod common;

use chrono::Timelike;
use common::parse_huid;
use rtatr::huid::Huid;
use rtatr::path::Paths;

#[test]
fn parse_plain() {
    let h = parse_huid("20240101-120000");
    assert_eq!(h.to_string(), "20240101-120000");
    assert_eq!(h.suffix(), None);
}

#[test]
fn parse_with_numeric_suffix() {
    let h = parse_huid("20240101-120000-01");
    assert_eq!(h.suffix(), Some("01"));
    assert_eq!(h.to_string(), "20240101-120000-01");
}

#[test]
fn parse_with_alnum_suffix() {
    for s in [
        "20240101-120000-rexim",
        "20240101-120000-01a",
        "20240101-120000-A-B-C",
    ] {
        let h = parse_huid(s);
        assert_eq!(h.to_string(), s);
    }
}

#[test]
fn parse_with_empty_suffix() {
    let h = parse_huid("20240101-120000-");
    assert_eq!(h.suffix(), Some(""));
    assert_eq!(h.to_string(), "20240101-120000-");
}

#[test]
fn parse_rejects_too_short() {
    assert!("2024".parse::<Huid>().is_err());
    assert!("20240101-1200".parse::<Huid>().is_err());
}

#[test]
fn parse_rejects_bad_separator() {
    assert!("20240101_120000".parse::<Huid>().is_err());
    assert!("202401011-12000".parse::<Huid>().is_err());
}

#[test]
fn parse_rejects_bad_suffix_char() {
    assert!("20240101-120000-a_b".parse::<Huid>().is_err());
    assert!("20240101-120000-a b".parse::<Huid>().is_err());
    assert!("20240101-120000-a.b".parse::<Huid>().is_err());
}

#[test]
fn parse_rejects_non_digit_body() {
    assert!("abcdefgh-120000".parse::<Huid>().is_err());
    assert!("20240101-abcdef".parse::<Huid>().is_err());
}

#[test]
fn ordering_is_lexicographic() {
    let pairs = [
        ("20240101-120000", "20240101-120001"),
        ("20240101-120000", "20240101-120000-01"),
        ("20240101-120000-01", "20240101-120000-02"),
        ("20240101-120000-09", "20240101-120000-10"),
        ("20240101-120000-rexim", "20240101-120000-z"),
    ];
    for (a, b) in pairs {
        assert!(parse_huid(a) < parse_huid(b), "{a} < {b}");
        assert_eq!(a.cmp(b), parse_huid(a).cmp(&parse_huid(b)));
    }
}

#[test]
fn now_truncates_nanos() {
    let h = Huid::now();
    assert_eq!(h.timestamp().nanosecond(), 0);
}

#[test]
fn now_has_no_suffix() {
    assert_eq!(Huid::now().suffix(), None);
}

#[test]
fn counter_produces_zero_padded_suffix() {
    assert_eq!(Huid::now_with_counter(7).suffix(), Some("07"));
    assert_eq!(Huid::now_with_counter(42).suffix(), Some("42"));
    assert_eq!(Huid::now_with_counter(0).suffix(), None);
}

#[test]
fn roundtrip_via_display() {
    for s in [
        "20240101-120000",
        "20240101-120000-01",
        "20240101-120000-rexim",
        "20240101-120000-",
    ] {
        assert_eq!(parse_huid(s).to_string(), s);
    }
}

#[test]
fn trunc_to_secs_public_reexport() {
    let now = rtatr::trunc_to_secs(chrono::Utc::now());
    assert_eq!(now.nanosecond(), 0);
}

#[test]
fn huid_from_task_dir() {
    let paths = Paths::from(std::path::PathBuf::from("/root"));
    assert!(paths
        .huid_from_task_dir(std::path::Path::new("/root/tasks/20240101-120000"))
        .is_some());
    assert!(paths
        .huid_from_task_dir(std::path::Path::new("/root/tasks/20240101-120000-rexim"))
        .is_some());
    assert!(paths
        .huid_from_task_dir(std::path::Path::new("/root/tasks/not-a-huid"))
        .is_none());
}
