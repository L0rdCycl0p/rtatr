use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use crate::error::{Result, RtatrError};
use crate::store::Store;

/// Mirror of upstream `graph_run` in `tatr.c`: emit a Graphviz graph of
/// task-to-task references and try to render it with `neato`.
pub fn run(store: &Store) -> Result<()> {
    let tasks = store.all()?;
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for t in &tasks {
        let from = t.huid.to_string();
        let content = t.to_markdown();
        let mut tos = Vec::new();
        for other in &tasks {
            if other.huid == t.huid {
                continue;
            }
            let to = other.huid.to_string();
            if content.contains(&to) {
                tos.push(to);
            }
        }
        if !tos.is_empty() {
            graph.insert(from, tos);
        }
    }

    let mut out = String::from("digraph {\n");
    for (from, tos) in &graph {
        for to in tos {
            out.push_str(&format!("    \"{from}\" -> \"{to}\";\n"));
        }
    }
    out.push_str("}\n");

    let dot_path = Path::new("graph.dot");
    std::fs::write(dot_path, &out).map_err(|e| RtatrError::Io {
        path: dot_path.to_path_buf(),
        source: e,
    })?;
    println!("Generated graph.dot");

    let status = Command::new("neato")
        .args(["-Goverlap=scale", "-Tsvg", "graph.dot", "-ograph.svg"])
        .status();
    match status {
        Ok(s) if s.success() => println!("Generated graph.svg"),
        Ok(s) => {
            eprintln!("warning: neato exited with {s:?}");
            return Err(RtatrError::Parse(format!("neato failed: {s:?}")));
        }
        Err(e) => {
            eprintln!("warning: failed to run neato: {e}");
            return Err(RtatrError::Io {
                path: std::path::PathBuf::from("neato"),
                source: e,
            });
        }
    }
    Ok(())
}