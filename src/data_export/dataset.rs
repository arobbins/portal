use time::OffsetDateTime;

pub struct Dataset {
    pub file: &'static str,
    pub about: &'static str,
    pub tables: &'static [&'static str],
    pub columns: &'static [&'static str],
    pub rows: Vec<Vec<String>>,
}

pub fn utc(at: OffsetDateTime) -> String {
    jiff::Timestamp::from_second(at.unix_timestamp())
        .map(|at| at.to_string())
        .unwrap_or_default()
}

pub fn maybe_utc(at: Option<OffsetDateTime>) -> String {
    at.map(utc).unwrap_or_default()
}

pub fn yes_no(value: bool) -> String {
    if value { "yes" } else { "no" }.to_string()
}
