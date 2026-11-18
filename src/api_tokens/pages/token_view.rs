use time::OffsetDateTime;

use crate::api::scope::TokenAudience;
use crate::api_tokens::queries::ApiToken;
use crate::api_tokens::token::display_hint;
use crate::components::property_list::PropertyTag;
use crate::components::tone::Tone;
use crate::time_zone::Zone;

pub(super) struct TokenRow {
    pub id: String,
    pub name: String,
    pub tags: Vec<PropertyTag>,
    pub detail: String,
}

impl TokenRow {
    pub fn new(token: &ApiToken, now: OffsetDateTime, zone: &Zone) -> Self {
        let expired = token.expires_at <= now;
        Self {
            id: token.public_id.clone(),
            name: token.name.clone(),
            tags: tags(token, expired),
            detail: detail(token, expired, zone),
        }
    }
}

fn tags(token: &ApiToken, expired: bool) -> Vec<PropertyTag> {
    let scopes = token
        .scopes
        .iter()
        .map(|scope| PropertyTag::new(scope.as_str(), Tone::Neutral));
    let mcp = (token.audience == TokenAudience::Mcp).then(|| PropertyTag::new("MCP", Tone::Info));
    let expiry = expired.then(|| PropertyTag::new("Expired", Tone::Error));

    scopes.chain(mcp).chain(expiry).collect()
}

fn detail(token: &ApiToken, expired: bool, zone: &Zone) -> String {
    let verb = if expired { "expired" } else { "expires" };
    let last_used = token.last_used_at.map_or_else(
        || "never used".to_string(),
        |at| format!("last used {}", zone.date(at)),
    );
    format!(
        "{} · Created {} · {verb} {} · {last_used}",
        display_hint(&token.last_four),
        zone.date(token.created_at),
        zone.date(token.expires_at),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::scope::ApiScope;

    fn token(expires_in: time::Duration) -> ApiToken {
        let created = OffsetDateTime::from_unix_timestamp(1_793_700_000).unwrap();
        ApiToken {
            public_id: "tok_aaaaaaaaaaaaaaaaaaaa".to_string(),
            name: "Deploy".to_string(),
            last_four: "abcd".to_string(),
            scopes: vec![ApiScope::ProfileRead],
            audience: TokenAudience::Mcp,
            created_at: created,
            expires_at: created + expires_in,
            last_used_at: None,
        }
    }

    #[test]
    fn a_row_tags_scopes_audience_and_expiry() {
        let token = token(time::Duration::days(1));
        let row = TokenRow::new(
            &token,
            token.expires_at + time::Duration::seconds(1),
            &Zone::utc(),
        );
        let labels: Vec<&str> = row.tags.iter().map(|tag| tag.label.as_str()).collect();

        assert_eq!(labels, vec!["profile:read", "MCP", "Expired"]);
        assert!(row.detail.contains("expired Nov 4, 2026"));
        assert!(row.detail.ends_with("never used"));
    }
}
