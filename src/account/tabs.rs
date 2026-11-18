use crate::admin::pages::admin_shell::AdminSection;
use crate::routes::ACCOUNT_PATH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountArea {
    Page(AccountPage),
    Admin(AdminSection),
}

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
        self.pages()[0].path()
    }

    pub fn pages(self) -> &'static [AccountPage] {
        match self {
            Self::Profile => &[AccountPage::Profile],
            Self::Security => &[AccountPage::SignIn, AccountPage::Devices],
            Self::Billing => &[AccountPage::Plan, AccountPage::Invoices],
            Self::Settings => &[AccountPage::Notifications, AccountPage::TimeZone],
            Self::Developer => &[AccountPage::Developer],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountPage {
    Profile,
    SignIn,
    Devices,
    Plan,
    Invoices,
    Notifications,
    TimeZone,
    Developer,
}

impl AccountPage {
    pub const ALL: [Self; 8] = [
        Self::Profile,
        Self::SignIn,
        Self::Devices,
        Self::Plan,
        Self::Invoices,
        Self::Notifications,
        Self::TimeZone,
        Self::Developer,
    ];

    pub fn tab(self) -> AccountTab {
        AccountTab::ALL
            .into_iter()
            .find(|tab| tab.pages().contains(&self))
            .expect("every page belongs to a tab")
    }

    pub fn slug(self) -> Option<&'static str> {
        match self {
            Self::Profile | Self::Developer => None,
            Self::SignIn => Some("authentication"),
            Self::Devices => Some("devices"),
            Self::Plan => Some("plan"),
            Self::Invoices => Some("invoices"),
            Self::Notifications => Some("notifications"),
            Self::TimeZone => Some("time-zone"),
        }
    }

    pub fn from_slugs(tab: &str, page: Option<&str>) -> Option<Self> {
        let tab = AccountTab::from_slug(tab)?;
        tab.pages()
            .iter()
            .copied()
            .find(|candidate| candidate.slug() == page)
    }

    pub fn heading(self) -> &'static str {
        match self {
            Self::Profile => "Profile",
            Self::SignIn => "Authentication",
            Self::Devices => "Devices and activity",
            Self::Plan => "Plan and payment",
            Self::Invoices => "Invoices",
            Self::Notifications => "Notifications",
            Self::TimeZone => "Time zone",
            Self::Developer => "Developer",
        }
    }

    pub fn path(self) -> String {
        match (self, self.slug()) {
            (Self::Profile, _) => ACCOUNT_PATH.to_string(),
            (_, Some(slug)) => format!("{ACCOUNT_PATH}/{}/{slug}", self.tab().slug()),
            (_, None) => format!("{ACCOUNT_PATH}/{}", self.tab().slug()),
        }
    }

    pub fn sections(self) -> &'static [AccountSection] {
        match self {
            Self::Profile => &[AccountSection::DeleteAccount],
            Self::SignIn => &[
                AccountSection::Password,
                AccountSection::Passkeys,
                AccountSection::TwoFactor,
                AccountSection::SingleSignOn,
            ],
            Self::Devices => &[AccountSection::Devices, AccountSection::Activity],
            Self::Plan => &[
                AccountSection::Subscriptions,
                AccountSection::PaymentMethods,
            ],
            Self::Invoices => &[AccountSection::Invoices],
            Self::Notifications => &[AccountSection::Notifications],
            Self::TimeZone => &[AccountSection::TimeZone],
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
    TimeZone,
    ApiTokens,
    DeleteAccount,
}

impl AccountSection {
    #[cfg(test)]
    pub const ALL: [Self; 13] = [
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
        Self::TimeZone,
        Self::ApiTokens,
        Self::DeleteAccount,
    ];

    pub fn page(self) -> AccountPage {
        AccountPage::ALL
            .into_iter()
            .find(|page| page.sections().contains(&self))
            .expect("every section belongs to a page")
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
            Self::TimeZone => "time-zone",
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
            Self::TimeZone => "Time zone",
            Self::ApiTokens => "API tokens",
            Self::DeleteAccount => "Delete account",
        }
    }

    pub fn path(self) -> String {
        format!("{}#{}", self.page().path(), self.anchor())
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
    fn every_page_belongs_to_one_tab_and_every_section_to_one_page() {
        for page in AccountPage::ALL {
            let homes = AccountTab::ALL
                .into_iter()
                .filter(|tab| tab.pages().contains(&page))
                .count();
            assert_eq!(homes, 1, "{page:?}");
        }
        for section in AccountSection::ALL {
            let homes = AccountPage::ALL
                .into_iter()
                .filter(|page| page.sections().contains(&section))
                .count();
            assert_eq!(homes, 1, "{section:?}");
        }
    }

    #[test]
    fn a_page_has_its_own_path_only_when_its_tab_has_several() {
        assert_eq!(AccountPage::Profile.path(), "/account");
        assert_eq!(AccountPage::Devices.path(), "/account/security/devices");
        assert_eq!(
            AccountTab::Security.path(),
            "/account/security/authentication"
        );
        assert_eq!(
            AccountSection::Activity.path(),
            "/account/security/devices#activity"
        );
        for page in AccountPage::ALL {
            assert_eq!(page.slug().is_some(), page.tab().pages().len() > 1);
            assert_eq!(
                AccountPage::from_slugs(page.tab().slug(), page.slug()),
                Some(page)
            );
        }
        assert_eq!(AccountPage::from_slugs("security", None), None);
        assert_eq!(AccountPage::from_slugs("profile", Some("x")), None);
    }
}
