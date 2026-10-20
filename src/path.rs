use std::path::{Path, PathBuf};

use crate::huid::Huid;

pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn tasks_dir(&self) -> PathBuf {
        self.root.join("tasks")
    }

    pub fn task_path(&self, huid: &Huid) -> PathBuf {
        self.tasks_dir().join(format!("{huid}.md"))
    }

    pub fn ensure_tasks_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.tasks_dir())
    }

    pub fn task_huid_from_path(&self, path: &Path) -> Option<Huid> {
        path.file_stem()?.to_str()?.parse().ok()
    }
}
