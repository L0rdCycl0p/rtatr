use std::path::PathBuf;

use chrono::Timelike as _;
use rtatr::path::Paths;
use rtatr::query::parse;
use rtatr::store::Store;
use rtatr::task::Task;
use rtatr::{query, Huid};

fn temp_store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Paths::from(dir.path().to_path_buf()));
    (dir, store)
}

#[test]
fn end_to_end_add_query_done() {
    let (_dir, store) = temp_store();

    let h1 = store.next_huid().unwrap();
    let mut t1 = Task {
        huid: h1.clone(),
        tags: ["work"].iter().map(|s| s.to_string()).collect(),
        props: Default::default(),
        body: "First task".to_string(),
        path: PathBuf::new(),
    };
    store.save(&mut t1).unwrap();

    let h2 = store.next_huid().unwrap();
    let mut t2 = Task {
        huid: h2.clone(),
        tags: ["work", "done"].iter().map(|s| s.to_string()).collect(),
        props: Default::default(),
        body: "Second task".to_string(),
        path: PathBuf::new(),
    };
    store.save(&mut t2).unwrap();

    let open = query::parse(":work and not :done").unwrap();
    let results = store.query(&open).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].huid, h1);
}

#[test]
fn persists_across_store_instances() {
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::new(Paths::from(dir.path().to_path_buf()));
        let huid = store.next_huid().unwrap();
        let mut t = Task {
            huid: huid.clone(),
            tags: ["x"].iter().map(|s| s.to_string()).collect(),
            props: Default::default(),
            body: "hello".to_string(),
            path: PathBuf::new(),
        };
        store.save(&mut t).unwrap();
    }
    let store2 = Store::new(Paths::from(dir.path().to_path_buf()));
    assert_eq!(store2.all().unwrap().len(), 1);
}

#[test]
fn now_has_no_subsecond_precision() {
    let h = Huid::now();
    assert_eq!(h.timestamp().nanosecond(), 0);
    assert_eq!(h.to_string().parse::<Huid>().unwrap(), h);
}

#[test]
fn trailing_operator_errors() {
    assert!(parse(":a and").is_err());
    assert!(parse(":a or").is_err());
    assert!(parse("and :a").is_err());
}

#[test]
fn unbalanced_parens_errors() {
    assert!(parse("(:a").is_err());
    assert!(parse(":a)").is_err());
}


#[test]
fn store_writes_task_directory() {
    let (_dir, store) = temp_store();
    let h = store.next_huid().unwrap();
    let mut t = Task {
        huid: h.clone(),
        tags: ["x"].into_iter().map(String::from).collect(),
        props: Default::default(),
        body: "hello".into(),
        path: PathBuf::new(),
    };
    store.save(&mut t).unwrap();

    let task_md = store.paths().task_path(&h);
    assert!(task_md.is_file(), "TASK.md missing at {task_md:?}");
    assert!(task_md.ends_with(format!("{h}/TASK.md")));
    assert!(store.paths().task_dir(&h).is_dir());

    assert_eq!(store.all().unwrap().len(), 1);
}

#[test]
fn huid_accepts_upstream_suffix() {
    let h: Huid = "20240101-120000-rexim".parse().unwrap();
    assert_eq!(h.to_string(), "20240101-120000-rexim");
    let h: Huid = "20240101-120000-01a".parse().unwrap();
    assert_eq!(h.to_string(), "20240101-120000-01a");

    assert!("20240101-120000".parse::<Huid>().unwrap()
          < "20240101-120000-01".parse::<Huid>().unwrap());
    assert!("20240101-120000-01".parse::<Huid>().unwrap()
          < "20240101-120000-02".parse::<Huid>().unwrap());
    assert!("20240101-120000-09".parse::<Huid>().unwrap()
          < "20240101-120000-10".parse::<Huid>().unwrap());
}

#[test]
fn valid_huid_file_is_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("tasks")).unwrap();
    std::fs::write(dir.path().join("tasks/20240101-120000"), "").unwrap();
    let store = Store::new(Paths::from(dir.path().to_path_buf()));
    assert!(store.all().is_err());
}