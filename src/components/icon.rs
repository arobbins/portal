use topcoat::{
    Result,
    context::Cx,
    view::{NodeViewParts, PartsWriter, View, component, view},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum IconName {
    Checkmark,
    ChevronDown,
    CreditCard,
    Edit,
    ErrorCircle,
    ExternalLink,
    Info,
    Loading,
    Logout,
    Menu,
    Monitor,
    Moon,
    Pinwheel,
    Settings,
    Sun,
    User,
    UserSquare,
    Warning,
}

impl IconName {
    fn svg(self) -> &'static str {
        match self {
            Self::Checkmark => include_str!("../../assets/icons/checkmark.svg"),
            Self::ChevronDown => include_str!("../../assets/icons/chevron-down.svg"),
            Self::CreditCard => include_str!("../../assets/icons/credit-card.svg"),
            Self::Edit => include_str!("../../assets/icons/edit.svg"),
            Self::ErrorCircle => include_str!("../../assets/icons/error-circle.svg"),
            Self::ExternalLink => include_str!("../../assets/icons/external-link.svg"),
            Self::Info => include_str!("../../assets/icons/info.svg"),
            Self::Loading => include_str!("../../assets/icons/loading.svg"),
            Self::Menu => include_str!("../../assets/icons/menu.svg"),
            Self::Logout => include_str!("../../assets/icons/logout.svg"),
            Self::Monitor => include_str!("../../assets/icons/monitor.svg"),
            Self::Moon => include_str!("../../assets/icons/moon.svg"),
            Self::Pinwheel => include_str!("../../assets/icons/pinwheel.svg"),
            Self::Settings => include_str!("../../assets/icons/settings.svg"),
            Self::Sun => include_str!("../../assets/icons/sun.svg"),
            Self::User => include_str!("../../assets/icons/user.svg"),
            Self::UserSquare => include_str!("../../assets/icons/user-square.svg"),
            Self::Warning => include_str!("../../assets/icons/warning.svg"),
        }
    }
}

impl NodeViewParts for IconName {
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        parts.push_static_str_unescaped(self.svg());
    }
}

#[component]
pub async fn icon(cx: &Cx, name: IconName) -> Result<impl View> {
    Ok(view! { <span class="a-icon">(name)</span> })
}
