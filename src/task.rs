use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::huid::Huid;

/// A task, backed by a single markdown file.
///
/// File format:
/// ```text
/// tags: :work :urgent
/// due: 2024-12-31
/// status: open
///
/// # Task body as markdown...
/// ```
///
/// The header block ends at the first blank line. Everything after is `body`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub huid: Huid,
    pub tags: BTreeSet<String>,
    pub props: BTreeMap<String, String>,
    pub body: String,
    pub path: PathBuf,
}

impl Task {
    pub fn from_markdown(huid: Huid, path: PathBuf, content: &str) -> Self {
        let (raw_props, body) = parse_content(content);

        let tags = raw_props
            .get("tags")
            .map(|s| {
                s.split_whitespace()
                    .map(|t| t.trim_start_matches(':').to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let props = raw_props.into_iter().filter(|(k, _)| k != "tags").collect();

        Self {
            huid,
            tags,
            props,
            body,
            path,
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("tags:");
        for tag in &self.tags {
            out.push_str(&format!(" :{tag}"));
        }
        out.push('\n');
        for (k, v) in &self.props {
            out.push_str(&format!("{k}: {v}\n"));
        }
        out.push('\n');
        out.push_str(self.body.trim_end_matches('\n'));
        out.push('\n');
        out
    }

    pub fn primary_tag(&self) -> &str {
        self.tags
            .iter()
            .next()
            .map(String::as_str)
            .unwrap_or("task")
    }

    pub fn title(&self) -> String {
        self.body
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim_start_matches('#').trim().to_string())
            .unwrap_or_default()
    }
}

fn parse_content(content: &str) -> (BTreeMap<String, String>, String) {
    let (header, body) = match content.find("\n\n") {
        Some(pos) => (&content[..pos], &content[pos + 2..]),
        None => (content, ""),
    };

    let mut props = BTreeMap::new();
    for line in header.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            props.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    (props, body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let original = Task {
            huid: "20240101-120000".parse().unwrap(),
            tags: ["work", "urgent"].iter().map(|s| s.to_string()).collect(),
            props: [("due".to_string(), "2024-12-31".to_string())]
                .into_iter()
                .collect(),
            body: "# Fix login\n\nDetails here.\n".to_string(),
            path: PathBuf::from("x.md"),
        };
        let md = original.to_markdown();
        let parsed = Task::from_markdown(original.huid.clone(), original.path.clone(), &md);
        assert_eq!(parsed.tags, original.tags);
        assert_eq!(parsed.props, original.props);
        assert_eq!(parsed.body.trim(), original.body.trim());
    }
}
