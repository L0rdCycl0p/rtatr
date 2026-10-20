//! Shell-completion support for `rtatr`.
//!
//! Provides two things:
//!
//! - Value completers for HUIDs, tags, update operators, and TQL primaries,
//!   registered on clap args via `ArgValueCompleter`. Used by the dynamic
//!   completion hook (`COMPLETE=bash rtatr` …).
//! - `find_tasks_dir`, which locates the store the same way the completers
//!   do, so completions follow the working directory even when `--root` is
//!   not passed explicitly.
//!
//! Dynamic completion does **not** see `--root` on the command line being
//! completed — clap_complete's engine does not expose other parsed args to
//! the completer. Users with a store outside the project tree should export
//! `RTATR_ROOT` so completion finds it.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use clap_complete::engine::CompletionCandidate;

use crate::huid::Huid;
use crate::task::split_tags;

// ---------------------------------------------------------------------------
// Store discovery
// ---------------------------------------------------------------------------

/// Find the tasks directory for completion. Lookup order:
///
/// 1. `RTATR_ROOT` environment variable + `/tasks`
/// 2. walk up from `cwd`, first `tasks/` directory found
/// 3. `./tasks` if it exists
pub fn find_tasks_dir() -> Option<PathBuf> {
    if let Ok(root) = std::env::var("RTATR_ROOT") {
        let p = PathBuf::from(root).join("tasks");
        if p.is_dir() {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur: Option<&Path> = Some(&cwd);
        while let Some(dir) = cur {
            let candidate = dir.join("tasks");
            if candidate.is_dir() {
                return Some(candidate);
            }
            cur = dir.parent();
        }
    }
    let fallback = PathBuf::from("tasks");
    fallback.is_dir().then_some(fallback)
}

// ---------------------------------------------------------------------------
// Data collection
// ---------------------------------------------------------------------------

/// Sorted list of HUID directory names under `dir`.
pub fn all_huids_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .filter(|name| name.parse::<Huid>().is_ok())
        .collect();
    out.sort();
    out
}

pub fn all_huids() -> Vec<String> {
    find_tasks_dir()
        .map(|d| all_huids_in(&d))
        .unwrap_or_default()
}

/// Sorted, deduplicated list of tags from `tasks/tags` and every `TASK.md`'s
/// `- TAGS:` line.
pub fn all_tags_in(dir: &Path) -> Vec<String> {
    let mut set = BTreeSet::new();

    if let Ok(content) = std::fs::read_to_string(dir.join("tags")) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let tag = line
                .split(|c: char| c.is_whitespace() || c == ',')
                .next()
                .unwrap_or("");
            if !tag.is_empty() {
                set.insert(tag.to_string());
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let Ok(content) = std::fs::read_to_string(entry.path().join("TASK.md")) else {
                continue;
            };
            // The header block ends at the first blank line *after* at least
            // one property line. Everything before that is metadata.
            let mut seen_prop = false;
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("- TAGS:") {
                    seen_prop = true;
                    for tag in split_tags(rest) {
                        set.insert(tag);
                    }
                } else if line.trim().is_empty() && seen_prop {
                    break;
                }
            }
        }
    }

    set.into_iter().collect()
}

pub fn all_tags() -> Vec<String> {
    find_tasks_dir()
        .map(|d| all_tags_in(&d))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// `ArgValueCompleter` callbacks
// ---------------------------------------------------------------------------

/// Complete a HUID. Used on `find`, `done`, `rm`, `show`, `update`, `ref`.
pub fn complete_huids(current: &OsStr) -> Vec<CompletionCandidate> {
    let prefix = current.to_string_lossy();
    all_huids()
        .into_iter()
        .filter(|h| h.starts_with(prefix.as_ref()))
        .map(CompletionCandidate::new)
        .collect()
}

/// Complete the last element of a comma- and whitespace-separated tag list.
pub fn complete_tag_list(current: &OsStr) -> Vec<CompletionCandidate> {
    let s = current.to_string_lossy();
    let split_at = s
        .rfind(|c: char| c == ',' || c.is_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    let (prefix, last) = s.split_at(split_at);

    all_tags()
        .into_iter()
        .filter(|t| t.starts_with(last))
        .map(|t| CompletionCandidate::new(format!("{prefix}{t}")))
        .collect()
}

/// Complete the last element of a comma-separated update-ops string.
/// Offers `open`, `close`, `+<tag>`, `-<tag>`.
pub fn complete_update_ops(current: &OsStr) -> Vec<CompletionCandidate> {
    let s = current.to_string_lossy();
    let split_at = s.rfind(',').map(|i| i + 1).unwrap_or(0);
    let (prefix, last) = s.split_at(split_at);

    let mut out: Vec<CompletionCandidate> = Vec::new();
    let tags = all_tags();

    if !last.starts_with('+') && !last.starts_with('-') {
        for op in ["open", "close"] {
            if op.starts_with(last) {
                out.push(CompletionCandidate::new(format!("{prefix}{op}")));
            }
        }
    }

    if let Some(stripped) = last.strip_prefix('+') {
        for tag in &tags {
            if tag.starts_with(stripped) {
                out.push(CompletionCandidate::new(format!("{prefix}+{tag}")));
            }
        }
    } else if let Some(stripped) = last.strip_prefix('-') {
        for tag in &tags {
            if tag.starts_with(stripped) {
                out.push(CompletionCandidate::new(format!("{prefix}-{tag}")));
            }
        }
    } else {
        for tag in &tags {
            if tag.starts_with(last) {
                out.push(CompletionCandidate::new(format!("{prefix}+{tag}")));
                out.push(CompletionCandidate::new(format!("{prefix}-{tag}")));
            }
        }
    }

    out
}

/// Complete a TQL query prefix. Completes the last whitespace-separated
/// token so `:bug and ` behaves the same as an empty prefix.
pub fn complete_query(current: &OsStr) -> Vec<CompletionCandidate> {
    let s = current.to_string_lossy();
    let split_at = s.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
    let (prefix, last) = s.split_at(split_at);

    let mut out: Vec<CompletionCandidate> = Vec::new();

    if !last.starts_with(':') {
        for kw in ["any", "tagged", "priority", "not"] {
            if kw.starts_with(last) {
                out.push(CompletionCandidate::new(format!("{prefix}{kw}")));
            }
        }
    }

    let tag_prefix = last.strip_prefix(':').unwrap_or(last);
    for tag in all_tags() {
        if tag.starts_with(tag_prefix) {
            out.push(CompletionCandidate::new(format!("{prefix}:{tag}")));
        }
    }

    out
}

/// Best-effort file completion for `todo`. Matches the last path component
/// of `current` against the containing directory. Used as a fallback when
/// the shell does not fall through to native file completion.
pub fn complete_files(current: &OsStr) -> Vec<CompletionCandidate> {
    let s = current.to_string_lossy();
    let (dir, prefix) = match s.rfind('/') {
        Some(i) => (Path::new(&s[..=i]), &s[i + 1..]),
        None => (Path::new("."), &s[..]),
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let dir_prefix = &s[..s.len() - prefix.len()];
    entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .filter(|name| name.starts_with(prefix))
        .map(|name| CompletionCandidate::new(format!("{dir_prefix}{name}")))
        .collect()
}

/// Subcommand names for `rtatr help <TAB>`. Keep in sync with the
/// `Command` enum in `main.rs` — a mismatch is a silent completion miss,
/// not a compile error.
pub fn complete_subcommands(current: &OsStr) -> Vec<CompletionCandidate> {
    let prefix = current.to_string_lossy();
    const NAMES: &[&str] = &[
        "init", "ls", "new", "add", "todo", "find", "id", "ref", "graph",
        "summary", "update", "done", "rm", "show", "tag", "untag",
        "completions", "help", "version",
    ];
    NAMES
        .iter()
        .filter(|n| n.starts_with(prefix.as_ref()))
        .map(|n| CompletionCandidate::new(*n))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_root<T>(root: &Path, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let old = std::env::var("RTATR_ROOT").ok();
        std::env::set_var("RTATR_ROOT", root);
        let result = f();
        match old {
            Some(v) => std::env::set_var("RTATR_ROOT", v),
            None => std::env::remove_var("RTATR_ROOT"),
        }
        result
    }

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let tasks = dir.path().join("tasks");
        std::fs::create_dir_all(tasks.join("20240101-120000")).unwrap();
        std::fs::create_dir_all(tasks.join("20240102-120000-rexim")).unwrap();
        std::fs::create_dir_all(tasks.join("20240103-140000-01")).unwrap();
        std::fs::write(
            tasks.join("20240101-120000/TASK.md"),
            "# A\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS: bug,urgent\n",
        )
        .unwrap();
        std::fs::write(
            tasks.join("20240102-120000-rexim/TASK.md"),
            "# B\n\n- STATUS: OPEN\n- PRIORITY: 50\n- TAGS: feature\n",
        )
        .unwrap();
        std::fs::write(
            tasks.join("20240103-140000-01/TASK.md"),
            "# C\n\n- STATUS: OPEN\n- PRIORITY: 20\n- TAGS: bug,feature\n",
        )
        .unwrap();
        std::fs::write(tasks.join("tags"), "bug Bugs\nfeature Features\n").unwrap();
        dir
    }

    #[test]
    fn huids_in_store() {
        let dir = setup();
        let huids = all_huids_in(&dir.path().join("tasks"));
        assert_eq!(
            huids,
            vec![
                "20240101-120000",
                "20240102-120000-rexim",
                "20240103-140000-01",
            ]
        );
    }

    #[test]
    fn tags_in_store_dedup_and_sorted() {
        let dir = setup();
        let tags = all_tags_in(&dir.path().join("tasks"));
        for t in ["bug", "urgent", "feature"] {
            assert!(tags.contains(&t.to_string()), "missing {t}: {tags:?}");
        }
        let mut sorted = tags.clone();
        sorted.sort();
        assert_eq!(tags, sorted);
    }

    #[test]
    fn complete_huids_prefix_filters() {
        let dir = setup();
        with_root(dir.path(), || {
            assert_eq!(complete_huids(OsStr::new("20240101")).len(), 1);
            assert_eq!(complete_huids(OsStr::new("20240102")).len(), 1);
            assert_eq!(complete_huids(OsStr::new("202401")).len(), 3);
            assert_eq!(complete_huids(OsStr::new("zzz")).len(), 0);
        });
    }

    #[test]
    fn complete_tag_list_handles_comma_and_space() {
        let dir = setup();
        with_root(dir.path(), || {
            // Prefix "bug,ur" suggests "bug,urgent".
            let c = complete_tag_list(OsStr::new("bug,ur"));
            assert_eq!(c.len(), 1);
            // Prefix "bug " (space) also triggers completion.
            let c = complete_tag_list(OsStr::new("bug "));
            assert!(c.len() >= 2); // feature, urgent
            // Empty prefix offers every tag.
            let c = complete_tag_list(OsStr::new(""));
            assert!(c.len() >= 3);
        });
    }

    #[test]
    fn complete_update_ops_variants() {
        let dir = setup();
        with_root(dir.path(), || {
            // `cl` → only `close`.
            let c = complete_update_ops(OsStr::new("cl"));
            assert_eq!(c.len(), 1);
            // `+bu` → `+bug`.
            let c = complete_update_ops(OsStr::new("+bu"));
            assert_eq!(c.len(), 1);
            // `close,-ur` → `close,-urgent`.
            let c = complete_update_ops(OsStr::new("close,-ur"));
            assert_eq!(c.len(), 1);
            // Empty prefix → both `+tag` and `-tag` for every tag, plus
            // `open`/`close` since they both start with "".
            let c = complete_update_ops(OsStr::new(""));
            assert!(c.len() >= 6); // 2 ops + (2 per tag) * 3 tags
        });
    }

    #[test]
    fn complete_query_keywords_and_tags() {
        let dir = setup();
        with_root(dir.path(), || {
            // Empty → keywords + tags.
            let c = complete_query(OsStr::new(""));
            assert!(c.len() >= 4);
            // `:bu` → `:bug`.
            let c = complete_query(OsStr::new(":bu"));
            assert_eq!(c.len(), 1);
            // `an` → `any`.
            let c = complete_query(OsStr::new("an"));
            assert_eq!(c.len(), 1);
            // `:bug and :` → every tag after "and ".
            let c = complete_query(OsStr::new(":bug and :"));
            assert!(c.len() >= 3);
        });
    }

    #[test]
    fn complete_subcommands_filters() {
        assert_eq!(complete_subcommands(OsStr::new("new")).len(), 1);
        assert_eq!(complete_subcommands(OsStr::new("u")).len(), 2); // update, untag
        assert!(complete_subcommands(OsStr::new("")).len() >= 15);
    }

    #[test]
    fn find_tasks_dir_respects_root_env() {
        let dir = setup();
        with_root(dir.path(), || {
            let found = find_tasks_dir().expect("should find store via RTATR_ROOT");
            assert!(found.ends_with("tasks"));
        });
    }
}