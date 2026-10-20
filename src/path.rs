use std::path::{Component, Path, PathBuf};

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

    /// If `cwd` is exactly `.../tasks/<HUID>`, return that HUID.
    /// Mirrors upstream `id_run` (which checks `path_relative(cwd, tasks/) == ".."`).
    pub fn current_task_huid(&self) -> Option<Huid> {
        let cwd = std::env::current_dir().ok()?;
        let name = cwd.file_name()?.to_str()?;
        let huid: Huid = name.parse().ok()?;
        let tasks = cwd.parent()?;
        if tasks.file_name()? != "tasks" {
            return None;
        }
        Some(huid)
    }

    pub fn discover() -> Option<Self> {
        if let Ok(root) = std::env::var("RTATR_ROOT") {
            let p = PathBuf::from(root);
            if p.join("tasks").is_dir() {
                return Some(Self { root: p });
            }
        }
        let cwd = std::env::current_dir().ok()?;
        let mut cur: Option<&Path> = Some(&cwd);
        while let Some(dir) = cur {
            if dir.join("tasks").is_dir() {
                return Some(Self {
                    root: dir.to_path_buf(),
                });
            }
            cur = dir.parent();
        }
        None
    }
    pub fn current_task_id(&self) -> Option<String> {
        let cwd = std::env::current_dir().ok()?;
        let name = cwd.file_name()?.to_str()?.to_string();
        let parent = cwd.parent()?;
        if parent.file_name()? != "tasks" {
            return None;
        }
        Some(name)
    }
}

impl From<PathBuf> for Paths {
    fn from(root: PathBuf) -> Self {
        Self { root }
    }
}

pub fn relpath_from_canonical(base: &Path, target: &Path) -> PathBuf {
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    let base_parts: Vec<Component> = base.components().collect();
    let targ_parts: Vec<Component> = target.components().collect();
    let mut i = 0;
    while i < base_parts.len() && i < targ_parts.len() && base_parts[i] == targ_parts[i] {
        i += 1;
    }
    let mut result = PathBuf::new();
    for _ in i..base_parts.len() {
        result.push("..");
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    for comp in &targ_parts[i..] {
        result.push(comp.as_os_str());
    }
    result
}
