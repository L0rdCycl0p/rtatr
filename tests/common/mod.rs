#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use rtatr::huid::Huid;
use rtatr::path::Paths;
use rtatr::store::Store;
use rtatr::task::Task;

pub fn temp_store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Paths::from(dir.path().to_path_buf()));
    (dir, store)
}

pub fn task(huid: &str, tags: &[&str], priority: i32, body: &str) -> Task {
    let mut t = Task::new(
        huid.parse().unwrap(),
        "t".into(),
        priority,
        tags.iter().map(|s| s.to_string()).collect(),
    );
    t.body = body.into();
    t
}

pub fn rtatr(root: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_rtatr"));
    c.arg("--root").arg(root);
    c
}

pub fn run_ok(cmd: &mut Command) -> String {
    let out: Output = cmd.output().expect("spawn");
    assert!(
        out.status.success(),
        "command failed: {:?}\nstderr: {}",
        cmd,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

pub fn write_task_md(root: &Path, huid: &str, content: &str) {
    let dir = root.join("tasks").join(huid);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("TASK.md"), content).unwrap();
}

pub fn init_tasks_dir(root: &Path) {
    std::fs::create_dir_all(root.join("tasks")).unwrap();
}

pub fn parse_huid(s: &str) -> Huid {
    s.parse().unwrap()
}

pub fn task_path(root: &Path, huid: &str) -> PathBuf {
    root.join("tasks").join(huid).join("TASK.md")
}
