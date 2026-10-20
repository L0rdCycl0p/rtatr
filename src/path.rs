use std::path::{Path, PathBuf};

use crate::huid::Huid;

pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    
    pub fn tasks_dir(&self) -> PathBuf {
        self.root.join("tasks")
    }

    /// `tasks/<HUID>/`
    pub fn task_dir(&self, huid: &Huid) -> PathBuf {
        self.tasks_dir().join(huid.to_string())
    }

    /// `tasks/<HUID>/TASK.md`
    pub fn task_path(&self, huid: &Huid) -> PathBuf {
        self.task_dir(huid).join("TASK.md")
    }

    pub fn ensure_tasks_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.tasks_dir())
    }

    pub fn ensure_task_dir(&self, huid: &Huid) -> std::io::Result<()> {
        std::fs::create_dir_all(self.task_dir(huid))
    }

    /// Parse a `tasks/<HUID>` directory name into a HUID.
    pub fn huid_from_task_dir(&self, path: &Path) -> Option<Huid> {
        path.file_name()?.to_str()?.parse().ok()
    }
}


impl From<PathBuf> for Paths {
    fn from(root: PathBuf) -> Self {
        Self { root }
    }
}