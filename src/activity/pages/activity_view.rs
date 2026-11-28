use crate::activity::actions::ActivityView;
use crate::components::property_list::PropertyTag;
use crate::components::tone::Tone;
use crate::time_zone::Zone;

pub(super) struct ActivityRow {
    pub(super) key: String,
    pub(super) summary: String,
    pub(super) detail: String,
    pub(super) tags: Vec<PropertyTag>,
}

impl ActivityRow {
    pub(super) fn of(index: usize, item: &ActivityView, zone: &Zone) -> Self {
        let mut parts = vec![zone.date_time(item.at)];
        parts.extend(item.device.clone());
        parts.extend(item.ip.clone());
        let tags = item
            .by
            .as_deref()
            .map(|by| PropertyTag::new(by, Tone::Neutral))
            .into_iter()
            .collect();

        Self {
            key: index.to_string(),
            summary: item.summary.clone(),
            detail: parts.join(" · "),
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(device: Option<&str>, ip: Option<&str>, by: Option<&str>) -> ActivityView {
        ActivityView {
            summary: "Signed in".to_string(),
            device: device.map(str::to_string),
            ip: ip.map(str::to_string),
            by: by.map(str::to_string),
            at: time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
        }
    }

    #[test]
    fn the_detail_joins_the_time_device_and_address() {
        let zone = Zone::utc();
        let row = ActivityRow::of(
            3,
            &view(Some("Firefox on macOS"), Some("203.0.113.9"), None),
            &zone,
        );

        assert_eq!(row.key, "3");
        assert_eq!(row.summary, "Signed in");
        assert_eq!(
            row.detail,
            format!(
                "{} · Firefox on macOS · 203.0.113.9",
                zone.date_time(view(None, None, None).at)
            )
        );
        assert!(row.tags.is_empty());
    }

    #[test]
    fn who_did_it_becomes_a_tag() {
        let row = ActivityRow::of(0, &view(None, None, Some("Claude")), &Zone::utc());

        assert_eq!(row.tags.len(), 1);
        assert_eq!(row.detail, Zone::utc().date_time(view(None, None, None).at));
    }
}
