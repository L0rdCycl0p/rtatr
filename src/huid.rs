use std::{fmt, str::FromStr};

use chrono::{DateTime, NaiveDateTime, TimeZone, Timelike, Utc};

/// Truncate a UTC timestamp to whole seconds (nanoseconds zeroed).
///
/// `Huid::now()` uses this so that IDs have stable, second-precision
/// timestamps matching upstream `tatr` (`time()`/`gmtime()` in `huid.c`).
pub fn trunc_to_secs(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_nanosecond(0).expect("nanosecond 0 is always valid")
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Huid {
    timestamp: DateTime<Utc>,
    /// `None` = no suffix. `Some("")` = trailing dash (upstream accepts this).
    /// `Some("rexim")` = `-rexim`.
    suffix: Option<String>,
}

impl Huid {
    pub fn now() -> Self {
        Self::now_with_counter(0)
    }

    pub fn now_with_counter(counter: u32) -> Self {
        Self {
            timestamp: trunc_to_secs(Utc::now()),
            suffix: if counter == 0 {
                None
            } else {
                Some(format!("{counter:02}"))
            },
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    pub fn suffix(&self) -> Option<&str> {
        self.suffix.as_deref()
    }
    /// Construct with an explicit suffix (upstream `-s`). No collision
    /// handling — the caller decides what to do if the directory exists.
    pub fn now_with_suffix(suffix: Option<String>) -> Self {
        Self {
            timestamp: trunc_to_secs(Utc::now()),
            suffix,
        }
    }
}

impl fmt::Display for Huid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ts = self.timestamp.format("%Y%m%d-%H%M%S");
        match &self.suffix {
            None => write!(f, "{ts}"),
            Some(s) => write!(f, "{ts}-{s}"),
        }
    }
}

impl FromStr for Huid {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.len() < 15 {
            return Err(format!("huid too short: {s}"));
        }
        let (date_part, rest) = s.split_at(15);
        let naive = NaiveDateTime::parse_from_str(date_part, "%Y%m%d-%H%M%S")
            .map_err(|e| format!("invalid huid timestamp '{date_part}': {e}"))?;
        let timestamp = Utc.from_utc_datetime(&naive);

        let suffix = if rest.is_empty() {
            None
        } else {
            let body = rest
                .strip_prefix('-')
                .ok_or_else(|| format!("invalid huid suffix: {rest}"))?;
            for c in body.chars() {
                if !(c.is_ascii_alphanumeric() || c == '-') {
                    return Err(format!("invalid huid suffix character: {c:?}"));
                }
            }
            Some(body.to_string())
        };

        Ok(Self { timestamp, suffix })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_plain() {
        let h: Huid = "20240101-120000".parse().unwrap();
        assert_eq!(h.to_string(), "20240101-120000");
    }

    #[test]
    fn roundtrip_with_counter() {
        let h = Huid::now_with_counter(7);
        let s = h.to_string();
        let h2: Huid = s.parse().unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn ordering() {
        let a: Huid = "20240101-120000".parse().unwrap();
        let b: Huid = "20240101-120001".parse().unwrap();
        assert!(a < b);
    }
}
