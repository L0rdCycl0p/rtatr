use crate::task::Task;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UOp {
    Open,
    Close,
    Tag(String),
    Untag(String),
}

/// Parse an update-language string. Tokens split on whitespace and commas.
/// Mirrors upstream `compile_update` in `update.c`.
pub fn parse_update(s: &str) -> Result<Vec<UOp>, String> {
    let mut ops = Vec::new();
    for tok in s.split(|c: char| c.is_whitespace() || c == ',') {
        if tok.is_empty() {
            continue;
        }
        if tok == "open" {
            ops.push(UOp::Open);
        } else if tok == "close" {
            ops.push(UOp::Close);
        } else if let Some(tag) = tok.strip_prefix('+') {
            if tag.is_empty() {
                return Err(format!("empty tag in update: '{tok}'"));
            }
            ops.push(UOp::Tag(tag.to_string()));
        } else if let Some(tag) = tok.strip_prefix('-') {
            if tag.is_empty() {
                return Err(format!("empty tag in update: '{tok}'"));
            }
            ops.push(UOp::Untag(tag.to_string()));
        } else {
            return Err(format!("unknown update action: '{tok}'"));
        }
    }
    Ok(ops)
}

/// Apply updates to a task in place. Returns `true` if anything changed.
/// Mirrors upstream `update_task` in `update.c`.
pub fn apply_update(task: &mut Task, ops: &[UOp]) -> bool {
    let mut changed = false;
    for op in ops {
        match op {
            UOp::Open => {
                if task.status != "OPEN" {
                    task.status = "OPEN".into();
                    changed = true;
                }
            }
            UOp::Close => {
                if task.status != "CLOSED" {
                    task.status = "CLOSED".into();
                    changed = true;
                }
            }
            UOp::Tag(t) => {
                if !task.tags.contains(t) {
                    task.tags.push(t.clone());
                    changed = true;
                }
            }
            UOp::Untag(t) => {
                let before = task.tags.len();
                task.tags.retain(|x| x != t);
                if task.tags.len() != before {
                    changed = true;
                }
            }
        }
    }
    changed
}