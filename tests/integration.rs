use std::path::PathBuf;

use rtatr::path::Paths;
use rtatr::query;
use rtatr::store::Store;
use rtatr::task::Task;

fn temp_store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Paths::new(dir.path().to_path_buf()));
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
        let store = Store::new(Paths::new(dir.path().to_path_buf()));
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
    let store2 = Store::new(Paths::new(dir.path().to_path_buf()));
    assert_eq!(store2.all().unwrap().len(), 1);
}