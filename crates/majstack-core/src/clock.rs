use chrono::{DateTime, Utc};

pub fn now_ts() -> i64 {
    Utc::now().timestamp()
}

pub fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub fn parse_iso(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

pub fn elapsed_seconds(start: DateTime<Utc>) -> i64 {
    (Utc::now() - start).num_seconds()
}
