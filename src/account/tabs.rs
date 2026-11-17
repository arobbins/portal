use crate::routes::ACCOUNT_PATH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountTab {
    Profile,
    Security,
    Billing,
    Settings,
    Developer,
}

impl AccountTab {
    pub const ALL: [Self; 5] = [
        Self::Profile,
        Self::Security,
        Self::Billing,
        Self::Settings,
        Self::Developer,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Security => "security",
            Self::Billing => "billing",
            Self::Settings => "settings",
            Self::Developer => "developer",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.slug() == slug)
    }

    pub fn heading(self) -> &'static str {
        match self {
            Self::Profile => "Profile",
            Self::Security => "Security",
            Self::Billing => "Billing",
            Self::Settings => "Settings",
            Self::Developer => "Developer",
        }
    }

    pub fn path(self) -> String {
        format!("{ACCOUNT_PATH}/{}", self.slug())
    }

    pub fn sections(self) -> &'static [AccountSection] {
        match self {
            Self::Profile => &[AccountSection::DeleteAccount],
            Self::Security => &[
                AccountSection::Password,
                AccountSection::Passkeys,
                AccountSection::Devices,
                AccountSection::Activity,
                AccountSection::TwoFactor,
                AccountSection::SingleSignOn,
            ],
            Self::Billing => &[
                AccountSection::Subscriptions,
                AccountSection::Invoices,
                AccountSection::PaymentMethods,
            ],
            Self::Settings => &[AccountSection::Notifications],
            Self::Developer => &[AccountSection::ApiTokens],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountSection {
    Password,
    Passkeys,
    TwoFactor,
    SingleSignOn,
    Devices,
    Activity,
    Subscriptions,
    Invoices,
    PaymentMethods,
    Notifications,
    ApiTokens,
    DeleteAccount,
}

impl AccountSection {
    #[cfg(test)]
    pub const ALL: [Self; 12] = [
        Self::Password,
        Self::Passkeys,
        Self::TwoFactor,
        Self::SingleSignOn,
        Self::Devices,
        Self::Activity,
        Self::Subscriptions,
        Self::Invoices,
        Self::PaymentMethods,
        Self::Notifications,
        Self::ApiTokens,
        Self::DeleteAccount,
    ];

    pub fn tab(self) -> AccountTab {
        AccountTab::ALL
            .into_iter()
            .find(|tab| tab.sections().contains(&self))
            .expect("every section belongs to a tab")
    }

    pub fn anchor(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Passkeys => "passkeys",
            Self::TwoFactor => "two-factor",
            Self::SingleSignOn => "single-sign-on",
            Self::Devices => "devices",
            Self::Activity => "activity",
            Self::Subscriptions => "subscriptions",
            Self::Invoices => "invoices",
            Self::PaymentMethods => "payment-methods",
            Self::Notifications => "notifications",
            Self::ApiTokens => "api-tokens",
            Self::DeleteAccount => "delete-account",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Password => "Password",
            Self::Passkeys => "Passkeys",
            Self::TwoFactor => "Two-factor authentication",
            Self::SingleSignOn => "Single sign-on",
            Self::Devices => "Devices",
            Self::Activity => "Recent activity",
            Self::Subscriptions => "Subscriptions",
            Self::Invoices => "Invoices",
            Self::PaymentMethods => "Payment methods",
            Self::Notifications => "Notifications",
            Self::ApiTokens => "API tokens",
            Self::DeleteAccount => "Delete account",
        }
    }

    pub fn path(self) -> String {
        format!("{}#{}", self.tab().path(), self.anchor())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tab_round_trips_through_its_slug() {
        for tab in AccountTab::ALL {
            assert_eq!(AccountTab::from_slug(tab.slug()), Some(tab));
        }
        assert_eq!(AccountTab::from_slug("nope"), None);
    }

    #[test]
    fn every_section_belongs_to_exactly_one_tab() {
        let sections: Vec<AccountSection> = AccountTab::ALL
            .into_iter()
            .flat_map(|tab| tab.sections().iter().copied())
            .collect();
        for section in &sections {
            assert_eq!(sections.iter().filter(|other| *other == section).count(), 1);
            assert!(section.tab().sections().contains(section));
        }
    }
}
