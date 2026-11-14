use crate::api::scope::ApiScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ViewProfile,
    EditProfile,
    ManagePassword,
    ManagePasskeys,
    ManageApiTokens,
}

impl Permission {
    pub const ALL: [Self; 5] = [
        Self::ViewProfile,
        Self::EditProfile,
        Self::ManagePassword,
        Self::ManagePasskeys,
        Self::ManageApiTokens,
    ];

    pub fn api_scope(self) -> Option<ApiScope> {
        match self {
            Self::ViewProfile => Some(ApiScope::ProfileRead),
            Self::EditProfile => Some(ApiScope::ProfileWrite),
            Self::ManagePassword | Self::ManagePasskeys | Self::ManageApiTokens => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRole {
    Owner,
}

impl WorkspaceRole {
    pub fn grants(self, permission: Permission) -> bool {
        match self {
            Self::Owner => Permission::ALL.contains(&permission),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_settings_never_have_an_api_scope() {
        for permission in [
            Permission::ManagePassword,
            Permission::ManagePasskeys,
            Permission::ManageApiTokens,
        ] {
            assert_eq!(permission.api_scope(), None);
        }
    }

    #[test]
    fn every_api_scope_belongs_to_a_permission() {
        for scope in ApiScope::ALL {
            assert!(
                Permission::ALL
                    .into_iter()
                    .any(|permission| permission.api_scope() == Some(scope)),
                "{}",
                scope.as_str()
            );
        }
    }

    #[test]
    fn the_owner_can_do_everything() {
        for permission in Permission::ALL {
            assert!(WorkspaceRole::Owner.grants(permission));
        }
    }
}
