use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoticeKind {
    Success,
    Warning,
    Error,
    #[default]
    Info,
}

impl NoticeKind {
    pub fn modifier(self) -> &'static str {
        match self {
            Self::Success => "a-notice--success",
            Self::Warning => "a-notice--warning",
            Self::Error => "a-notice--error",
            Self::Info => "a-notice--info",
        }
    }
}

#[component]
pub fn Notice(
    #[prop(optional)] kind: NoticeKind,
    #[prop(optional)] dismissible: bool,
    children: Children,
) -> impl IntoView {
    if dismissible {
        view! { <DismissibleNotice kind=kind>{children()}</DismissibleNotice> }.into_any()
    } else {
        view! { <div class=format!("a-notice {}", kind.modifier())>{children()}</div> }.into_any()
    }
}

#[island]
fn DismissibleNotice(kind: NoticeKind, children: Children) -> impl IntoView {
    let is_dismissing = RwSignal::new(false);
    let is_dismissed = RwSignal::new(false);

    let class = move || {
        let mut class = format!("a-notice {} a-notice--is-dismissible", kind.modifier());
        if is_dismissing.get() {
            class.push_str(" a-notice--is-dismissing");
        }
        if is_dismissed.get() {
            class.push_str(" a-notice--is-dismissed");
        }
        class
    };

    view! {
        <div
            class=class
            on:click=move |_| is_dismissing.set(true)
            on:transitionend=move |_| {
                if is_dismissing.get_untracked() {
                    is_dismissed.set(true);
                }
            }
        >
            {children()}
        </div>
    }
}
