use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, TimeZone, Timelike, Utc};

/// Human-Unique ID: `YYYYMMDD-HHMMSS` with optional `-NN` collision suffix.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Huid {
    timestamp: DateTime<Utc>,
    counter: u32,
}

impl Huid {
    pub fn now() -> Self {
        Self {
            timestamp: trunc_to_secs(Utc::now()),
            counter: 0,
        }
    }

    pub fn now_with_counter(counter: u32) -> Self {
        Self {
            timestamp: trunc_to_secs(Utc::now()),
            counter,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    pub fn counter(&self) -> u32 {
        self.counter
    }
}

fn trunc_to_secs(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_nanosecond(0).expect("nanosecond 0 is always valid")
}

impl fmt::Display for Huid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ts = self.timestamp.format("%Y%m%d-%H%M%S");
        if self.counter == 0 {
            write!(f, "{ts}")
        } else {
            write!(f, "{ts}-{:02}", self.counter)
        }
    }
}

impl FromStr for Huid {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.len() < 15 {
            return Err(format!("huid too short: {s}"));
        }
        let date_part = s.get(0..15).ok_or_else(|| format!("huid not ASCII: {s}"))?;
        let naive = NaiveDateTime::parse_from_str(date_part, "%Y%m%d-%H%M%S")
            .map_err(|e| format!("invalid huid timestamp '{date_part}': {e}"))?;
        let timestamp = Utc.from_utc_datetime(&naive);

        let counter = match s.get(15..).unwrap_or("") {
            "" => 0,
            rest => {
                let digits = rest
                    .strip_prefix('-')
                    .ok_or_else(|| format!("invalid huid suffix: {rest}"))?;
                digits
                    .parse::<u32>()
                    .map_err(|e| format!("invalid counter: {e}"))?
            }
        };

        Ok(Self { timestamp, counter })
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
