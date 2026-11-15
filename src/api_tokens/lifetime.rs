use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
pub enum TokenLifetime {
    #[serde(rename = "7")]
    Week,
    #[serde(rename = "30")]
    Month,
    #[serde(rename = "90")]
    Quarter,
    #[serde(rename = "365")]
    Year,
}

impl TokenLifetime {
    pub const ALL: [Self; 4] = [Self::Week, Self::Month, Self::Quarter, Self::Year];
    pub const DEFAULT: Self = Self::Quarter;

    pub fn days(self) -> i64 {
        match self {
            Self::Week => 7,
            Self::Month => 30,
            Self::Quarter => 90,
            Self::Year => 365,
        }
    }

    pub fn value(self) -> &'static str {
        match self {
            Self::Week => "7",
            Self::Month => "30",
            Self::Quarter => "90",
            Self::Year => "365",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Week => "7 days",
            Self::Month => "30 days",
            Self::Quarter => "90 days",
            Self::Year => "1 year",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_label_matches_the_lifetime() {
        for lifetime in TokenLifetime::ALL {
            let days = lifetime.days();
            let expected = if days == 365 {
                "1 year".to_string()
            } else {
                format!("{days} days")
            };
            assert_eq!(lifetime.label(), expected);
        }
    }

    #[test]
    fn lifetimes_parse_from_their_form_values() {
        for lifetime in TokenLifetime::ALL {
            let parsed: TokenLifetime =
                serde_json::from_value(serde_json::json!(lifetime.value())).unwrap();
            assert_eq!(parsed, lifetime);
            assert_eq!(lifetime.value(), lifetime.days().to_string());
        }
    }
}
