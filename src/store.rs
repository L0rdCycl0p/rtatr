use std::fs;

use crate::error::{Result, RtatrError};
use crate::huid::Huid;
use crate::path::Paths;
use crate::query::Query;
use crate::task::Task;

pub struct Store {
    paths: Paths,
}

impl Store {
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn all(&self) -> Result<Vec<Task>> {
        let dir = self.paths.tasks_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut tasks = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|e| RtatrError::Io {
            path: dir.clone(),
            source: e,
        })? {
            let entry = entry.map_err(|e| RtatrError::Io {
                path: dir.clone(),
                source: e,
            })?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let Some(huid) = self.paths.task_huid_from_path(&path) else {
                continue;
            };
            let content = fs::read_to_string(&path).map_err(|e| RtatrError::Io {
                path: path.clone(),
                source: e,
            })?;
            tasks.push(Task::from_markdown(huid, path, &content));
        }
        tasks.sort_by(|a, b| a.huid.cmp(&b.huid));
        Ok(tasks)
    }

    pub fn query(&self, q: &Query) -> Result<Vec<Task>> {
        Ok(self.all()?.into_iter().filter(|t| q.matches(t)).collect())
    }

    pub fn get(&self, huid: &Huid) -> Result<Task> {
        let path = self.paths.task_path(huid);
        if !path.exists() {
            return Err(RtatrError::TaskNotFound(huid.to_string()));
        }
        let content = fs::read_to_string(&path).map_err(|e| RtatrError::Io {
            path: path.clone(),
            source: e,
        })?;
        Ok(Task::from_markdown(huid.clone(), path, &content))
    }

    pub fn save(&self, task: &mut Task) -> Result<()> {
        self.paths.ensure_tasks_dir().map_err(|e| RtatrError::Io {
            path: self.paths.tasks_dir(),
            source: e,
        })?;
        if task.path.as_os_str().is_empty() {
            task.path = self.paths.task_path(&task.huid);
        }
        fs::write(&task.path, task.to_markdown()).map_err(|e| RtatrError::Io {
            path: task.path.clone(),
            source: e,
        })?;
        Ok(())
    }

    pub fn delete(&self, huid: &Huid) -> Result<()> {
        let task = self.get(huid)?;
        fs::remove_file(&task.path).map_err(|e| RtatrError::Io {
            path: task.path.clone(),
            source: e,
        })?;
        Ok(())
    }

    /// Generate a fresh HUID that doesn't collide with an existing file.
    pub fn next_huid(&self) -> Result<Huid> {
        for counter in 0..1000 {
            let h = Huid::now_with_counter(counter);
            if !self.paths.task_path(&h).exists() {
                return Ok(h);
            }
        }
        Err(RtatrError::Parse("huid counter exhausted".into()))
    }
}
