mod common;

use common::{init_tasks_dir, parse_huid, task, task_path, temp_store, write_task_md};
use rtatr::path::Paths;
use rtatr::store::Store;

#[test]
fn empty_store_returns_empty() {
    let (_dir, store) = temp_store();
    assert!(store.all().unwrap().is_empty());
}

#[test]
fn missing_tasks_dir_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Paths::from(dir.path().to_path_buf()));
    assert!(store.all().unwrap().is_empty());
}

#[test]
fn save_creates_task_directory() {
    let (_dir, store) = temp_store();
    let h = parse_huid("20240101-120000");
    let mut t = task("20240101-120000", &["x"], 100, "hello");
    store.save(&mut t).unwrap();

    let task_md = store.paths().task_path(&h);
    assert!(task_md.is_file());
    assert!(task_md.ends_with("20240101-120000/TASK.md"));
    assert!(store.paths().task_dir(&h).is_dir());
}

#[test]
fn save_updates_existing() {
    let (_dir, store) = temp_store();
    let mut t = task("20240101-120000", &["x"], 100, "hello");
    store.save(&mut t).unwrap();
    t.title = "changed".into();
    store.save(&mut t).unwrap();
    let reloaded = store.get(&parse_huid("20240101-120000")).unwrap();
    assert_eq!(reloaded.title, "changed");
}

#[test]
fn get_missing_errors() {
    let (_dir, store) = temp_store();
    assert!(store.get(&parse_huid("20240101-120000")).is_err());
}

#[test]
fn all_reads_all_tasks() {
    let (dir, store) = temp_store();
    init_tasks_dir(dir.path());
    write_task_md(
        dir.path(),
        "20240101-120000",
        "# T1\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n",
    );
    write_task_md(
        dir.path(),
        "20240102-120000",
        "# T2\n\n- STATUS: OPEN\n- PRIORITY: 50\n- TAGS:\n",
    );

    let all = store.all().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].title, "T1");
    assert_eq!(all[1].title, "T2");
}

#[test]
fn all_skips_dir_without_task_md() {
    let (dir, store) = temp_store();
    init_tasks_dir(dir.path());
    std::fs::create_dir_all(dir.path().join("tasks/20240101-120000")).unwrap();
    write_task_md(
        dir.path(),
        "20240102-120000",
        "# T2\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n",
    );

    let all = store.all().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].title, "T2");
}

#[test]
fn all_skips_non_huid_dirs() {
    let (dir, store) = temp_store();
    init_tasks_dir(dir.path());
    std::fs::create_dir_all(dir.path().join("tasks/not-a-huid")).unwrap();
    std::fs::write(
        dir.path().join("tasks/not-a-huid/TASK.md"),
        "# T\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n",
    )
    .unwrap();
    assert!(store.all().unwrap().is_empty());
}

#[test]
fn all_skips_dotfiles() {
    let (dir, store) = temp_store();
    init_tasks_dir(dir.path());
    std::fs::create_dir_all(dir.path().join("tasks/.hidden")).unwrap();
    assert!(store.all().unwrap().is_empty());
}

#[test]
fn all_errors_on_huid_file() {
    let (dir, store) = temp_store();
    init_tasks_dir(dir.path());
    std::fs::write(dir.path().join("tasks/20240101-120000"), "").unwrap();
    assert!(store.all().is_err());
}

#[test]
fn all_skips_plain_files() {
    let (dir, store) = temp_store();
    init_tasks_dir(dir.path());
    std::fs::write(dir.path().join("tasks/README.md"), "hi").unwrap();
    assert!(store.all().unwrap().is_empty());
}

#[test]
fn delete_removes_directory() {
    let (_dir, store) = temp_store();
    let mut t = task("20240101-120000", &[], 100, "");
    store.save(&mut t).unwrap();
    let task_dir = store.paths().task_dir(&parse_huid("20240101-120000"));
    assert!(task_dir.is_dir());

    store.delete(&parse_huid("20240101-120000")).unwrap();
    assert!(!task_dir.exists());
    assert!(store.get(&parse_huid("20240101-120000")).is_err());
}

#[test]
fn delete_missing_errors() {
    let (_dir, store) = temp_store();
    assert!(store.delete(&parse_huid("20240101-120000")).is_err());
}

#[test]
fn delete_removes_attachments() {
    let (_dir, store) = temp_store();
    let mut t = task("20240101-120000", &[], 100, "");
    store.save(&mut t).unwrap();

    let task_dir = store.paths().task_dir(&parse_huid("20240101-120000"));
    std::fs::write(task_dir.join("screenshot.png"), "fake").unwrap();
    assert!(task_dir.join("screenshot.png").exists());

    store.delete(&parse_huid("20240101-120000")).unwrap();
    assert!(!task_dir.exists());
}

#[test]
fn next_huid_no_collision() {
    let (dir, store) = temp_store();
    init_tasks_dir(dir.path());
    let h = rtatr::huid::Huid::now();
    std::fs::create_dir_all(store.paths().task_dir(&h)).unwrap();

    let h2 = store.next_huid().unwrap();
    assert_ne!(h.to_string(), h2.to_string());
}

#[test]
fn tasks_sorted_by_huid() {
    let (dir, store) = temp_store();
    init_tasks_dir(dir.path());
    write_task_md(
        dir.path(),
        "20240102-000000",
        "# T2\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n",
    );
    write_task_md(
        dir.path(),
        "20240101-000000",
        "# T1\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS:\n",
    );

    let all = store.all().unwrap();
    assert_eq!(all[0].huid.to_string(), "20240101-000000");
    assert_eq!(all[1].huid.to_string(), "20240102-000000");
}

#[test]
fn path_helpers() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::from(dir.path().to_path_buf());
    let h = parse_huid("20240101-120000");
    assert_eq!(paths.tasks_dir(), dir.path().join("tasks"));
    assert_eq!(paths.task_dir(&h), dir.path().join("tasks/20240101-120000"));
    assert_eq!(
        paths.task_path(&h),
        dir.path().join("tasks/20240101-120000/TASK.md")
    );

    paths.ensure_task_dir(&h).unwrap();
    assert!(paths.task_dir(&h).is_dir());

    std::fs::write(paths.task_path(&h), "").unwrap();
    assert_eq!(
        task_path(dir.path(), "20240101-120000"),
        paths.task_path(&h)
    );
}

#[test]
fn next_huid_creates_dir_name() {
    let (_dir, store) = temp_store();
    let h = store.next_huid().unwrap();
    let s = h.to_string();
    assert!(s.len() >= 15);
    assert!(s.parse::<rtatr::huid::Huid>().is_ok());
    assert!(!store.paths().task_dir(&h).exists());
}
