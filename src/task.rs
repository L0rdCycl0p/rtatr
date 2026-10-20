use std::path::PathBuf;

use crate::huid::Huid;

pub const DEFAULT_STATUS: &str = "OPEN";
/// Priority applied when a `TASK.md` has no `- PRIORITY:` line.
/// Matches `load_tasks` in upstream `task.c` ("Unset priority is super high
/// so you don't forget to set it").
pub const MISSING_PRIORITY: i32 = 999_999;
/// Priority applied when creating a fresh task. Matches `DEFAULT_PRIORITY`
/// in upstream `main.c` used by `new_run`.
pub const NEW_TASK_PRIORITY: i32 = 100;
pub const INVALID_TITLE: &str = "!!! INVALID: TASK TITLE MUST START WITH # !!!";

/// One property line of a `TASK.md` header. Order is preserved so we can
/// round-trip upstream files byte-for-byte. `STATUS` / `PRIORITY` / `TAGS`
/// carry no inline value — their content lives in `Task::status` /
/// `Task::priority` / `Task::tags`, exactly like upstream's `Property` list
/// (upstream's `render_task_md` ignores the stored value for those keys).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prop {
    Status,
    Priority,
    Tags,
    Other(String, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub huid: Huid,
    pub title: String,
    pub status: String,
    pub priority: i32,
    /// Ordered; upstream never de-duplicates tags within a task.
    pub tags: Vec<String>,
    /// Order of property lines as they will be rendered. Always contains
    /// `Status`, `Priority`, and `Tags` (appended at the end if missing
    /// from the source file, matching upstream's `properties_put` fallback).
    pub props: Vec<Prop>,
    pub body: String,
    pub path: PathBuf,
}

impl Task {
    /// Build a fresh task the way upstream `create_new_task` does.
    pub fn new(huid: Huid, title: String, priority: i32, tags: Vec<String>) -> Self {
        Self {
            huid,
            title,
            status: DEFAULT_STATUS.into(),
            priority,
            tags,
            props: vec![Prop::Status, Prop::Priority, Prop::Tags],
            body: String::new(),
            path: PathBuf::new(),
        }
    }

    /// Parse a `TASK.md`. Mirrors `load_tasks` + `task_md_parse` +
    /// `task_md_extract_title` + `task_md_extract_properties`.
    pub fn from_markdown(huid: Huid, path: PathBuf, content: &str) -> Self {
        let bytes = content.as_bytes();
        let (title, title_ok, after_title) = extract_title(bytes);

        if !title_ok {
            // Upstream: `if (task_md_extract_title(...)) { ...body... }` — when the
            // title is invalid, properties and body are left at their defaults
            // (`{0}` = empty) and the three fallback properties are still appended.
            return Self {
                huid,
                title,
                status: DEFAULT_STATUS.into(),
                priority: MISSING_PRIORITY,
                tags: Vec::new(),
                props: vec![Prop::Status, Prop::Priority, Prop::Tags],
                body: String::new(),
                path,
            };
        }

        let (raw_props, body_start) = extract_props(bytes, after_title);
        let body = String::from_utf8_lossy(&bytes[body_start..]).into_owned();

        let mut status: Option<String> = None;
        let mut priority: Option<i32> = None;
        let mut tags: Option<Vec<String>> = None;
        let mut props: Vec<Prop> = Vec::new();

        for (key, value) in raw_props {
            match key.as_str() {
                "STATUS" => {
                    status = Some(value);
                    props.push(Prop::Status);
                }
                "PRIORITY" => {
                    priority = Some(c_atoi(&value));
                    props.push(Prop::Priority);
                }
                "TAGS" => {
                    tags = Some(split_tags(&value));
                    props.push(Prop::Tags);
                }
                _ => props.push(Prop::Other(key, value)),
            }
        }

        let status = status.unwrap_or_else(|| {
            props.push(Prop::Status);
            DEFAULT_STATUS.into()
        });
        let priority = priority.unwrap_or_else(|| {
            props.push(Prop::Priority);
            MISSING_PRIORITY
        });
        let tags = tags.unwrap_or_else(|| {
            props.push(Prop::Tags);
            Vec::new()
        });

        Self {
            huid,
            title,
            status,
            priority,
            tags,
            props,
            body,
            path,
        }
    }

    /// Render back to `TASK.md`, byte-compatible with upstream `render_task_md`.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# ");
        out.push_str(&self.title);
        out.push('\n');
        out.push('\n');
        for p in &self.props {
            match p {
                Prop::Status => {
                    out.push_str("- STATUS: ");
                    out.push_str(&self.status);
                    out.push('\n');
                }
                Prop::Priority => {
                    out.push_str("- PRIORITY: ");
                    out.push_str(&self.priority.to_string());
                    out.push('\n');
                }
                Prop::Tags => {
                    out.push_str("- TAGS:");
                    if !self.tags.is_empty() {
                        out.push(' ');
                        out.push_str(&self.tags.join(","));
                    }
                    out.push('\n');
                }
                Prop::Other(k, v) => {
                    out.push_str("- ");
                    out.push_str(k);
                    out.push_str(": ");
                    out.push_str(v);
                    out.push('\n');
                }
            }
        }
        out.push_str(&self.body);
        out
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers — port of `md.c` / `task.c`
// ---------------------------------------------------------------------------

fn is_c_whitespace(b: u8) -> bool {
    // `isspace` in the C locale.
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_c_whitespace_no_newline(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | 0x0b | 0x0c)
}

fn trim_c_whitespace(s: &[u8]) -> String {
    let mut start = 0;
    let mut end = s.len();
    while start < end && is_c_whitespace(s[start]) {
        start += 1;
    }
    while end > start && is_c_whitespace(s[end - 1]) {
        end -= 1;
    }
    String::from_utf8_lossy(&s[start..end]).into_owned()
}

/// C `atoi`. Returns 0 if there are no leading digits; accepts a leading
/// sign; stops at the first non-digit.
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return 0;
    }
    let n: i32 = std::str::from_utf8(&bytes[start..i])
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(0);
    sign * n
}

/// Split a `TAGS` value on commas and/or whitespace (`unparsed_tags_next`).
pub fn split_tags(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c == ',' || ((c as u32) < 128 && is_c_whitespace(c as u8)) {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn extract_title(s: &[u8]) -> (String, bool, usize) {
    let mut i = 0;
    while i < s.len() && is_c_whitespace(s[i]) {
        i += 1;
    }
    if i >= s.len() || s[i] != b'#' {
        return (INVALID_TITLE.to_string(), false, i);
    }
    i += 1;
    while i < s.len() && is_c_whitespace_no_newline(s[i]) {
        i += 1;
    }
    let start = i;
    while i < s.len() && s[i] != b'\n' {
        i += 1;
    }
    let title = String::from_utf8_lossy(&s[start..i]).into_owned();
    (title, true, i)
}

fn extract_props(s: &[u8], mut i: usize) -> (Vec<(String, String)>, usize) {
    let mut props: Vec<(String, String)> = Vec::new();

    // `md_trim_spaces` before the loop — consumes the newline after the title
    // and any blank lines up to the first property (or body) line.
    while i < s.len() && is_c_whitespace(s[i]) {
        i += 1;
    }

    loop {
        let saved = i;
        while i < s.len() && is_c_whitespace_no_newline(s[i]) {
            i += 1;
        }
        if i >= s.len() || s[i] != b'-' {
            return (props, saved);
        }
        i += 1;
        while i < s.len() && is_c_whitespace_no_newline(s[i]) {
            i += 1;
        }
        let key_start = i;
        while i < s.len() && s[i].is_ascii_alphanumeric() {
            i += 1;
        }
        let key = &s[key_start..i];
        while i < s.len() && is_c_whitespace_no_newline(s[i]) {
            i += 1;
        }
        if key.is_empty() || i >= s.len() || s[i] != b':' {
            return (props, saved);
        }
        i += 1;
        let val_start = i;
        while i < s.len() && s[i] != b'\n' {
            i += 1;
        }
        let value = trim_c_whitespace(&s[val_start..i]);

        // `properties_put`: overwrite-by-key, new entry goes to the end.
        let key_str = String::from_utf8_lossy(key).into_owned();
        if let Some(idx) = props.iter().position(|(k, _)| k == &key_str) {
            props.remove(idx);
        }
        props.push((key_str, value));

        if i < s.len() {
            i += 1; // consume '\n'
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h() -> Huid {
        "20240101-120000".parse().unwrap()
    }

    #[test]
    fn roundtrip_upstream_format() {
        let content =
            "# Fix login bug\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS: urgent,work\n\nBody text.\n";
        let t = Task::from_markdown(h(), PathBuf::from("TASK.md"), content);
        assert_eq!(t.title, "Fix login bug");
        assert_eq!(t.status, "OPEN");
        assert_eq!(t.priority, 100);
        assert_eq!(t.tags, vec!["urgent".to_string(), "work".to_string()]);
        assert_eq!(t.body, "\nBody text.\n");
        assert_eq!(t.to_markdown(), content);
    }

    #[test]
    fn roundtrip_with_extra_property() {
        let content =
            "# Title\n\n- STATUS: OPEN\n- PRIORITY: 50\n- TAGS: a\n- DUE: 2024-12-31\n\nBody\n";
        let t = Task::from_markdown(h(), PathBuf::from("TASK.md"), content);
        assert_eq!(t.to_markdown(), content);
    }

    #[test]
    fn missing_priority_defaults_and_appends() {
        let content = "# Title\n\n- STATUS: OPEN\n- TAGS: a\nBody\n";
        let t = Task::from_markdown(h(), PathBuf::from("TASK.md"), content);
        assert_eq!(t.priority, 999_999);
        assert_eq!(
            t.to_markdown(),
            "# Title\n\n- STATUS: OPEN\n- TAGS: a\n- PRIORITY: 999999\nBody\n"
        );
    }

    #[test]
    fn missing_status_defaults_and_appends() {
        let content = "# Title\n\n- PRIORITY: 100\n\nBody\n";
        let t = Task::from_markdown(h(), PathBuf::from("TASK.md"), content);
        assert_eq!(t.status, "OPEN");
        assert_eq!(
            t.to_markdown(),
            "# Title\n\n- PRIORITY: 100\n- STATUS: OPEN\n- TAGS:\n\nBody\n"
        );
    }

    #[test]
    fn invalid_title_matches_upstream() {
        let content = "no hash title\n";
        let t = Task::from_markdown(h(), PathBuf::from("TASK.md"), content);
        assert_eq!(t.title, INVALID_TITLE);
        assert_eq!(t.body, "");
        assert_eq!(
            t.to_markdown(),
            "# !!! INVALID: TASK TITLE MUST START WITH # !!!\n\n- STATUS: OPEN\n- PRIORITY: 999999\n- TAGS:\n"
        );
    }

    #[test]
    fn new_task_renders_upstream_format() {
        let t = Task::new(
            h(),
            "Fix login bug".into(),
            NEW_TASK_PRIORITY,
            vec!["a".into(), "b".into()],
        );
        assert_eq!(
            t.to_markdown(),
            "# Fix login bug\n\n- STATUS: OPEN\n- PRIORITY: 100\n- TAGS: a,b\n"
        );
    }

    #[test]
    fn atoi_matches_c() {
        assert_eq!(c_atoi(""), 0);
        assert_eq!(c_atoi("abc"), 0);
        assert_eq!(c_atoi("42"), 42);
        assert_eq!(c_atoi("-5"), -5);
        assert_eq!(c_atoi("+5"), 5);
        assert_eq!(c_atoi("100abc"), 100);
    }

    #[test]
    fn tags_split_on_comma_and_whitespace() {
        assert_eq!(
            split_tags("a,b c  d"),
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string()
            ]
        );
        assert_eq!(split_tags(""), Vec::<String>::new());
        assert_eq!(split_tags("  ,,  "), Vec::<String>::new());
    }
}
