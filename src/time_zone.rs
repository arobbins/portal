use jiff::tz::TimeZone;
use time::OffsetDateTime;

pub const UTC_NAME: &str = "UTC";

#[derive(Debug, Clone)]
pub struct Zone {
    name: String,
    zone: TimeZone,
}

fn all_names() -> Vec<String> {
    let mut names: Vec<String> = jiff::tz::db()
        .available()
        .map(|name| name.as_str().to_string())
        .filter(|name| name.contains('/') || name == UTC_NAME)
        .filter(|name| !name.starts_with("Etc/") && !name.starts_with("SystemV/"))
        .collect();
    names.sort();
    names.dedup();
    names
}

impl Zone {
    pub fn utc() -> Self {
        Self {
            name: UTC_NAME.to_string(),
            zone: TimeZone::UTC,
        }
    }

    pub fn named(name: Option<&str>) -> Self {
        name.and_then(Self::parse).unwrap_or_else(Self::utc)
    }

    pub fn parse(name: &str) -> Option<Self> {
        let name = name.trim();
        if name.is_empty() || name.len() > 64 {
            return None;
        }
        let zone = jiff::tz::db().get(name).ok()?;
        let name = zone.iana_name().unwrap_or(name).to_string();
        Some(Self { name, zone })
    }

    pub fn names() -> &'static [String] {
        static NAMES: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(all_names);
        &NAMES
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn date(&self, at: OffsetDateTime) -> String {
        self.zoned(at).strftime("%b %-d, %Y").to_string()
    }

    pub fn iso_date(&self, at: OffsetDateTime) -> String {
        self.zoned(at).strftime("%Y-%m-%d").to_string()
    }

    pub fn date_time(&self, at: OffsetDateTime) -> String {
        self.zoned(at)
            .strftime("%b %-d, %Y at %H:%M %Z")
            .to_string()
    }

    fn zoned(&self, at: OffsetDateTime) -> jiff::Zoned {
        let timestamp = jiff::Timestamp::from_second(at.unix_timestamp())
            .unwrap_or(jiff::Timestamp::UNIX_EPOCH);
        timestamp.to_zoned(self.zone.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn times_are_shown_in_the_zone_with_its_abbreviation() {
        let at = datetime!(2026-11-17 02:30 UTC);

        assert_eq!(Zone::utc().date_time(at), "Nov 17, 2026 at 02:30 UTC");
        let pacific = Zone::named(Some("America/Los_Angeles"));
        assert_eq!(pacific.date_time(at), "Nov 16, 2026 at 18:30 PST");
        assert_eq!(pacific.date(at), "Nov 16, 2026");
    }

    #[test]
    fn an_unknown_or_missing_zone_falls_back_to_utc() {
        assert_eq!(Zone::named(None).name(), "UTC");
        assert_eq!(Zone::named(Some("Mars/Olympus_Mons")).name(), "UTC");
        assert!(Zone::parse("").is_none());
        assert!(Zone::parse(&"A/".repeat(40)).is_none());
        assert_eq!(
            Zone::parse(" Europe/Paris ").unwrap().name(),
            "Europe/Paris"
        );
    }

    #[test]
    fn the_choices_are_real_place_names() {
        let names = Zone::names();

        assert!(names.contains(&"Europe/Paris".to_string()));
        assert!(names.contains(&"UTC".to_string()));
        assert!(!names.iter().any(|name| name.starts_with("Etc/")));
        assert!(names.iter().all(|name| Zone::parse(name).is_some()));
    }
}
