/// Very small helper used for full-text search. Strips common
/// markdown syntax so text search matches user-facing words.
pub fn strip_markdown(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for line in input.lines() {
        let line = line.trim_start();
        let line = line.trim_start_matches('#').trim_start();
        let line = line.trim_start_matches("- ").trim_start();
        let line = line.trim_start_matches("* ").trim_start();
        out.push_str(line);
        out.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_headers_and_bullets() {
        let s = strip_markdown("# Title\n- one\n* two\nbody");
        assert!(s.contains("Title"));
        assert!(s.contains("one"));
        assert!(s.contains("body"));
        assert!(!s.contains('#'));
    }
}