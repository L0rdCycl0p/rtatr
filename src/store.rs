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

            // `path.is_dir()` follows symlinks, matching upstream's `stat`-based
            // `get_file_type`. A valid-HUID *file* is an error, same as upstream.
            if path.is_dir() {
                let Some(huid) = self.paths.huid_from_task_dir(&path) else {
                    continue;
                };
                let task_md = path.join("TASK.md");
                if !task_md.is_file() {
                    continue;
                }
                let content = fs::read_to_string(&task_md).map_err(|e| RtatrError::Io {
                    path: task_md.clone(),
                    source: e,
                })?;
                tasks.push(Task::from_markdown(huid, task_md, &content));
            } else if self.paths.huid_from_task_dir(&path).is_some() {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                return Err(RtatrError::Parse(format!("{name} is not a directory")));
            }
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
        self.paths
            .ensure_task_dir(&task.huid)
            .map_err(|e| RtatrError::Io {
                path: self.paths.task_dir(&task.huid),
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
        let _ = self.get(huid)?;
        let dir = self.paths.task_dir(huid);
        let extra = fs::read_dir(&dir)
            .map_err(|e| RtatrError::Io {
                path: dir.clone(),
                source: e,
            })?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() != "TASK.md")
            .count();
        if extra > 0 {
            eprintln!("warning: deleting task {huid} along with {extra} attachment(s)");
        }
        fs::remove_dir_all(&dir).map_err(|e| RtatrError::Io {
            path: dir,
            source: e,
        })?;
        Ok(())
    }

    pub fn next_huid(&self) -> Result<Huid> {
        for counter in 0..1000 {
            let h = Huid::now_with_counter(counter);
            if !self.paths.task_dir(&h).exists() {
                return Ok(h);
            }
        }
        Err(RtatrError::Parse("huid counter exhausted".into()))
    }
}
