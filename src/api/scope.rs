use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::user::model::{Role, User};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ApiScope {
    #[serde(rename = "profile:read")]
    ProfileRead,
    #[serde(rename = "profile:write")]
    ProfileWrite,
}

impl ApiScope {
    pub const ALL: [Self; 2] = [Self::ProfileRead, Self::ProfileWrite];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProfileRead => "profile:read",
            Self::ProfileWrite => "profile:write",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|scope| scope.as_str() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ProfileRead => "Read your profile",
            Self::ProfileWrite => "Read and update your profile",
        }
    }

    pub fn choice_label(self) -> &'static str {
        match self {
            Self::ProfileRead => "Read your profile (profile:read)",
            Self::ProfileWrite => "Read and update your profile (profile:write)",
        }
    }

    pub fn implies(self, needed: Self) -> bool {
        self == needed || matches!((self, needed), (Self::ProfileWrite, Self::ProfileRead))
    }

    fn requires_admin(self) -> bool {
        match self {
            Self::ProfileRead | Self::ProfileWrite => false,
        }
    }

    pub fn available_to(self, user: &User) -> bool {
        !self.requires_admin() || user.role == Role::Admin
    }
}

pub fn parse_scopes(values: &[String]) -> Vec<ApiScope> {
    values
        .iter()
        .filter_map(|value| ApiScope::from_str(value))
        .collect()
}

pub fn grants(scopes: &[ApiScope], needed: ApiScope) -> bool {
    scopes.iter().any(|scope| scope.implies(needed))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenAudience {
    Api,
}

impl TokenAudience {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scope_round_trips_through_its_name() {
        for scope in ApiScope::ALL {
            assert_eq!(ApiScope::from_str(scope.as_str()), Some(scope));
        }
        assert_eq!(ApiScope::from_str("profile:*"), None);
        assert_eq!(ApiScope::from_str(""), None);
    }

    #[test]
    fn every_choice_label_names_its_scope() {
        for scope in ApiScope::ALL {
            assert!(scope.choice_label().contains(scope.as_str()));
            assert!(scope.choice_label().starts_with(scope.label()));
        }
    }

    #[test]
    fn write_implies_read_but_not_the_other_way() {
        assert!(grants(&[ApiScope::ProfileWrite], ApiScope::ProfileRead));
        assert!(grants(&[ApiScope::ProfileRead], ApiScope::ProfileRead));
        assert!(!grants(&[ApiScope::ProfileRead], ApiScope::ProfileWrite));
        assert!(!grants(&[], ApiScope::ProfileRead));
    }

    #[test]
    fn serde_names_match_the_scope_names() {
        for scope in ApiScope::ALL {
            assert_eq!(serde_json::to_value(scope).unwrap(), scope.as_str());
        }
    }

    #[test]
    fn unknown_scope_names_grant_nothing() {
        let scopes = parse_scopes(&["admin:*".to_string(), "profile:read".to_string()]);

        assert_eq!(scopes, vec![ApiScope::ProfileRead]);
    }
}
