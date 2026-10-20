use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtatrError {
    #[error("IO error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error: {0}")]
    Parse(String),
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("invalid huid: {0}")]
    InvalidHuid(String),
    #[error("query error: {0}")]
    Query(String),
}

pub type Result<T> = std::result::Result<T, RtatrError>;