mod common;

use common::{init_tasks_dir, rtatr, run_ok, task_path, write_task_md};
use std::fs;

#[test]
fn init_creates_tasks() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    assert!(dir.path().join("tasks").is_dir());
}

#[test]
fn init_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    run_ok(rtatr(dir.path()).arg("init"));
    assert!(dir.path().join("tasks").is_dir());
}

#[test]
fn new_prints_huid() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let out = run_ok(rtatr(dir.path()).args(["new", "Fix it"]));
    let huid = out.trim();
    assert!(huid.len() >= 15);
    assert!(task_path(dir.path(), huid).is_file());
}

#[test]
fn new_with_tags() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let huid = run_ok(rtatr(dir.path()).args(["new", "-t", "work,urgent", "x"]))
        .trim()
        .to_string();
    let content = fs::read_to_string(task_path(dir.path(), &huid)).unwrap();
    assert!(
        content.contains("- TAGS: work,urgent"),
        "content: {content}"
    );
}

#[test]
fn new_with_priority() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let huid = run_ok(rtatr(dir.path()).args(["new", "-p", "50", "x"]))
        .trim()
        .to_string();
    let content = fs::read_to_string(task_path(dir.path(), &huid)).unwrap();
    assert!(content.contains("- PRIORITY: 50"));
}

#[test]
fn new_with_suffix() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let huid = run_ok(rtatr(dir.path()).args(["new", "-s", "rexim", "x"]))
        .trim()
        .to_string();
    assert!(huid.ends_with("-rexim"), "huid: {huid}");
}

#[test]
fn new_with_multiple_positional_words() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let huid = run_ok(rtatr(dir.path()).args(["new", "Fix", "the", "login", "bug"]))
        .trim()
        .to_string();
    let content = fs::read_to_string(task_path(dir.path(), &huid)).unwrap();
    assert!(content.starts_with("# Fix the login bug\n"));
}

#[test]
fn new_without_title_defaults() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let huid = run_ok(rtatr(dir.path()).arg("new")).trim().to_string();
    let content = fs::read_to_string(task_path(dir.path(), &huid)).unwrap();
    assert!(content.starts_with("# New Task\n"));
}

#[test]
fn add_is_alias_for_new() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let huid = run_ok(rtatr(dir.path()).args(["add", "Via alias"]))
        .trim()
        .to_string();
    let content = fs::read_to_string(task_path(dir.path(), &huid)).unwrap();
    assert!(content.starts_with("# Via alias\n"));
}

#[test]
fn ls_empty() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let out = run_ok(rtatr(dir.path()).arg("ls"));
    assert!(out.contains("No tasks were found"));
}

#[test]
fn ls_lists_open_only_by_default() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    run_ok(rtatr(dir.path()).args(["new", "Open one"]));
    let h2 = run_ok(rtatr(dir.path()).args(["new", "Closed one"]))
        .trim()
        .to_string();
    run_ok(rtatr(dir.path()).args(["done", &h2]));

    let out = run_ok(rtatr(dir.path()).arg("ls"));
    assert!(out.contains("Open one"));
    assert!(!out.contains("Closed one"));
}

#[test]
fn ls_c_shows_closed() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "Closed one"]))
        .trim()
        .to_string();
    run_ok(rtatr(dir.path()).args(["done", &h]));

    let out = run_ok(rtatr(dir.path()).args(["ls", "-c"]));
    assert!(out.contains("Closed one"));
}

#[test]
fn ls_by_query() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    run_ok(rtatr(dir.path()).args(["new", "-t", "work", "Work item"]));
    run_ok(rtatr(dir.path()).args(["new", "-t", "personal", "Home item"]));

    let out = run_ok(rtatr(dir.path()).args(["ls", ":work"]));
    assert!(out.contains("Work item"));
    assert!(!out.contains("Home item"));
}

#[test]
fn ls_priority_descending_default() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    run_ok(rtatr(dir.path()).args(["new", "-p", "100", "Low"]));
    run_ok(rtatr(dir.path()).args(["new", "-p", "50", "High"]));

    let out = run_ok(rtatr(dir.path()).arg("ls"));
    assert!(out.find("Low").unwrap() < out.find("High").unwrap());
}

#[test]
fn ls_ascending() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    run_ok(rtatr(dir.path()).args(["new", "-p", "100", "Low"]));
    run_ok(rtatr(dir.path()).args(["new", "-p", "50", "High"]));

    let out = run_ok(rtatr(dir.path()).args(["ls", "-a"]));
    assert!(out.find("High").unwrap() < out.find("Low").unwrap());
}

#[test]
fn ls_by_id_descending() {
    let dir = tempfile::tempdir().unwrap();
    init_tasks_dir(dir.path());
    write_task_md(
        dir.path(),
        "20240101-000000",
        "# Old\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n",
    );
    write_task_md(
        dir.path(),
        "20240102-000000",
        "# New\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n",
    );

    let out = run_ok(rtatr(dir.path()).args(["ls", "--id"]));
    assert!(out.find("New").unwrap() < out.find("Old").unwrap());

    let out = run_ok(rtatr(dir.path()).args(["ls", "--id", "-a"]));
    assert!(out.find("Old").unwrap() < out.find("New").unwrap());
}

#[test]
fn find_prints_report() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "-t", "work", "-p", "50", "Fix it"]))
        .trim()
        .to_string();

    let out = run_ok(rtatr(dir.path()).args(["find", &h]));
    assert!(
        out.contains("OPEN [PRIORITY:  50] [work] Fix it"),
        "out: {out}"
    );
}

#[test]
fn find_path_only() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "x"]))
        .trim()
        .to_string();
    let out = run_ok(rtatr(dir.path()).args(["find", &h, "--path-only"]));
    assert!(out.trim().ends_with(&format!("{h}/TASK.md")));
}

#[test]
fn find_closed_task() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "x"]))
        .trim()
        .to_string();
    run_ok(rtatr(dir.path()).args(["done", &h]));
    let out = run_ok(rtatr(dir.path()).args(["find", &h]));
    assert!(out.contains("CLOSED"));
}

#[test]
fn find_missing_errors() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let out = rtatr(dir.path())
        .args(["find", "20240101-120000"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn update_add_remove_tag() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "x"]))
        .trim()
        .to_string();
    run_ok(rtatr(dir.path()).args(["update", &h, "--add-tag", "bug", "--add-tag", "urgent"]));

    let content = fs::read_to_string(task_path(dir.path(), &h)).unwrap();
    assert!(content.contains("- TAGS: bug,urgent"));

    run_ok(rtatr(dir.path()).args(["update", &h, "--remove-tag", "bug"]));
    let content = fs::read_to_string(task_path(dir.path(), &h)).unwrap();
    assert!(content.contains("- TAGS: urgent"));
}

#[test]
fn done_sets_closed() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "x"]))
        .trim()
        .to_string();
    run_ok(rtatr(dir.path()).args(["done", &h]));
    let content = fs::read_to_string(task_path(dir.path(), &h)).unwrap();
    assert!(content.contains("- STATUS: CLOSED"));
}

#[test]
fn rm_deletes_task() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "x"]))
        .trim()
        .to_string();
    assert!(task_path(dir.path(), &h).exists());
    run_ok(rtatr(dir.path()).args(["rm", &h]));
    assert!(!task_path(dir.path(), &h).exists());
    assert!(!dir.path().join("tasks").join(&h).exists());
}

#[test]
fn show_prints_details() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "-t", "work", "-p", "50", "Fix it"]))
        .trim()
        .to_string();

    let out = run_ok(rtatr(dir.path()).args(["show", &h]));
    assert!(out.contains(&format!("HUID:     {h}")));
    assert!(out.contains("Title:    Fix it"));
    assert!(out.contains("Status:   OPEN"));
    assert!(out.contains("Priority: 50"));
    assert!(out.contains("Tags:     work"));
}

#[test]
fn summary_counts() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    run_ok(rtatr(dir.path()).args(["new", "-t", "bug", "a"]));
    run_ok(rtatr(dir.path()).args(["new", "-t", "bug,feat", "b"]));
    run_ok(rtatr(dir.path()).args(["new", "c"]));

    fs::write(
        dir.path().join("tasks/tags"),
        "bug Bugs found during testing\nfeat New features\n",
    )
    .unwrap();

    let out = run_ok(rtatr(dir.path()).arg("summary"));
    assert!(out.contains("STATUS:   OPEN"), "summary:\n{out}");
    assert!(out.contains("TOTAL:    3"), "summary:\n{out}");
    assert!(out.contains("UNTAGGED: 1"), "summary:\n{out}");
    let has_bug = out
        .lines()
        .any(|l| l.contains("bug") && l.contains("Bugs found"));
    assert!(has_bug, "summary:\n{out}");
    let has_feat = out
        .lines()
        .any(|l| l.contains("feat") && l.contains("New features"));
    assert!(has_feat, "summary:\n{out}");
}

#[test]
fn summary_closed() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "x"]))
        .trim()
        .to_string();
    run_ok(rtatr(dir.path()).args(["done", &h]));

    let out = run_ok(rtatr(dir.path()).args(["summary", "-c"]));
    assert!(out.contains("STATUS:   CLOSED"));
    assert!(out.contains("TOTAL:    1"));
}

#[test]
fn version_prints() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_ok(rtatr(dir.path()).arg("version"));
    assert!(out.starts_with("rtatr "));
}

#[test]
fn id_inside_task_dir() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(rtatr(dir.path()).arg("init"));
    let h = run_ok(rtatr(dir.path()).args(["new", "x"]))
        .trim()
        .to_string();

    let task_dir = dir.path().join("tasks").join(&h);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_rtatr"))
        .arg("id")
        .current_dir(&task_dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), h);
}

#[test]
fn id_outside_task_dir_errors() {
    let dir = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_rtatr"))
        .arg("id")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
}
