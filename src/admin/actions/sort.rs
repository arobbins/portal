use schemars::JsonSchema;
use serde::Deserialize;

use crate::listing::{SortDirection, SortKey};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserSort {
    Name,
    Email,
    Status,
    Role,
    #[default]
    Joined,
    LastActive,
}

impl SortKey for UserSort {
    const ALL: &'static [Self] = &[
        Self::Name,
        Self::Email,
        Self::Status,
        Self::Role,
        Self::Joined,
        Self::LastActive,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Email => "email",
            Self::Status => "status",
            Self::Role => "role",
            Self::Joined => "joined",
            Self::LastActive => "last_active",
        }
    }

    fn first_direction(self) -> SortDirection {
        match self {
            Self::Name | Self::Email | Self::Status | Self::Role => SortDirection::Ascending,
            Self::Joined | Self::LastActive => SortDirection::Descending,
        }
    }
}
