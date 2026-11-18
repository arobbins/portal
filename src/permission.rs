use crate::api::scope::ApiScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ViewProfile,
    EditProfile,
    ManagePassword,
    ManageEmail,
    ConfirmIdentity,
    ManagePasskeys,
    ManageTwoFactor,
    ManageDevices,
    ViewActivity,
    ManageApiTokens,
    DeleteAccount,
    AdministerUsers,
}

impl Permission {
    pub const ALL: [Self; 12] = [
        Self::ViewProfile,
        Self::EditProfile,
        Self::ManagePassword,
        Self::ManageEmail,
        Self::ConfirmIdentity,
        Self::ManagePasskeys,
        Self::ManageTwoFactor,
        Self::ManageDevices,
        Self::ViewActivity,
        Self::ManageApiTokens,
        Self::DeleteAccount,
        Self::AdministerUsers,
    ];

    pub fn api_scope(self) -> Option<ApiScope> {
        match self {
            Self::ViewProfile => Some(ApiScope::ProfileRead),
            Self::EditProfile => Some(ApiScope::ProfileWrite),
            Self::ManagePassword
            | Self::ManageEmail
            | Self::ConfirmIdentity
            | Self::ManagePasskeys
            | Self::ManageTwoFactor
            | Self::ManageDevices
            | Self::ViewActivity
            | Self::ManageApiTokens
            | Self::DeleteAccount
            | Self::AdministerUsers => None,
        }
    }

    pub fn admins_only(self) -> bool {
        matches!(self, Self::AdministerUsers)
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
            Permission::ManageEmail,
            Permission::ConfirmIdentity,
            Permission::ManagePasskeys,
            Permission::ManageTwoFactor,
            Permission::ManageDevices,
            Permission::ViewActivity,
            Permission::ManageApiTokens,
            Permission::DeleteAccount,
            Permission::AdministerUsers,
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
