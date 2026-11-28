use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::user::model::{Role, User};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum ApiScope {
    #[serde(rename = "profile:read")]
    #[sqlx(rename = "profile:read")]
    ProfileRead,
    #[serde(rename = "profile:write")]
    #[sqlx(rename = "profile:write")]
    ProfileWrite,
    #[serde(rename = "billing:read")]
    #[sqlx(rename = "billing:read")]
    BillingRead,
    #[serde(rename = "billing:write")]
    #[sqlx(rename = "billing:write")]
    BillingWrite,
    #[cfg(test)]
    #[serde(rename = "test:admin")]
    #[sqlx(rename = "test:admin")]
    TestAdmin,
}

impl ApiScope {
    pub const ALL: [Self; 4] = [
        Self::ProfileRead,
        Self::ProfileWrite,
        Self::BillingRead,
        Self::BillingWrite,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProfileRead => "profile:read",
            Self::ProfileWrite => "profile:write",
            Self::BillingRead => "billing:read",
            Self::BillingWrite => "billing:write",
            #[cfg(test)]
            Self::TestAdmin => "test:admin",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        #[cfg(test)]
        if value == Self::TestAdmin.as_str() {
            return Some(Self::TestAdmin);
        }
        Self::ALL.into_iter().find(|scope| scope.as_str() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ProfileRead => "Read your profile",
            Self::ProfileWrite => "Read and update your profile",
            Self::BillingRead => "Read your billing",
            Self::BillingWrite => "Read your billing and start checkouts",
            #[cfg(test)]
            Self::TestAdmin => "Administer the test",
        }
    }

    pub fn choice_label(self) -> &'static str {
        match self {
            Self::ProfileRead => "Read your profile (profile:read)",
            Self::ProfileWrite => "Read and update your profile (profile:write)",
            Self::BillingRead => "Read your billing (billing:read)",
            Self::BillingWrite => "Read your billing and start checkouts (billing:write)",
            #[cfg(test)]
            Self::TestAdmin => "Administer the test (test:admin)",
        }
    }

    pub fn implies(self, needed: Self) -> bool {
        self == needed
            || matches!(
                (self, needed),
                (Self::ProfileWrite, Self::ProfileRead) | (Self::BillingWrite, Self::BillingRead)
            )
    }

    fn requires_admin(self) -> bool {
        match self {
            Self::ProfileRead | Self::ProfileWrite | Self::BillingRead | Self::BillingWrite => {
                false
            }
            #[cfg(test)]
            Self::TestAdmin => true,
        }
    }

    pub fn can_write(self) -> bool {
        match self {
            Self::ProfileRead | Self::BillingRead => false,
            Self::ProfileWrite | Self::BillingWrite => true,
            #[cfg(test)]
            Self::TestAdmin => true,
        }
    }

    pub fn offered_by_sign_in(self) -> bool {
        !self.requires_admin()
    }

    pub fn available_to(self, user: &User) -> bool {
        !self.requires_admin() || user.role == Role::Admin
    }
}

pub fn scope_names(scopes: &[ApiScope]) -> String {
    let names: Vec<&str> = scopes.iter().map(|scope| scope.as_str()).collect();
    names.join(" ")
}

pub fn grants(scopes: &[ApiScope], needed: ApiScope) -> bool {
    scopes.iter().any(|scope| scope.implies(needed))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum TokenAudience {
    #[serde(rename = "api")]
    Api,
    #[serde(rename = "mcp")]
    Mcp,
}

impl TokenAudience {
    pub const ALL: [Self; 2] = [Self::Api, Self::Mcp];
    pub const DEFAULT: Self = Self::Api;

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Mcp => "mcp",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|audience| audience.as_str() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Api => "Scripts and apps (JSON API)",
            Self::Mcp => "AI assistants (MCP)",
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
        assert!(grants(&[ApiScope::BillingWrite], ApiScope::BillingRead));
        assert!(!grants(&[ApiScope::BillingRead], ApiScope::BillingWrite));
        assert!(!grants(&[ApiScope::ProfileWrite], ApiScope::BillingRead));
    }

    #[test]
    fn serde_names_match_the_scope_names() {
        for scope in ApiScope::ALL {
            assert_eq!(serde_json::to_value(scope).unwrap(), scope.as_str());
        }
    }

    #[test]
    fn scope_names_are_space_separated() {
        let scopes = [ApiScope::ProfileRead, ApiScope::BillingWrite];

        assert_eq!(scope_names(&scopes), "profile:read billing:write");
        assert_eq!(scope_names(&[]), "");
    }
}
