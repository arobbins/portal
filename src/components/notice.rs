use topcoat::{
    Result,
    context::Cx,
    runtime::signal,
    view::{Child, View, ViewExt, component, view},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

const SHOWN: &str = "shown";
const DISMISSING: &str = "dismissing";
const DISMISSED: &str = "dismissed";

#[component]
pub async fn notice(
    cx: &Cx,
    #[default] kind: NoticeKind,
    #[default] dismissible: bool,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    if !dismissible {
        let class = format!("a-notice {}", kind.modifier());
        return Ok(view! { <div class=(class)>(child)</div> }.boxed());
    }

    let stage = signal(cx, || SHOWN.to_string());
    let dismissing = DISMISSING.to_string();
    let dismissed = DISMISSED.to_string();

    let class = format!("a-notice {} a-notice--is-dismissible", kind.modifier());
    let dismissing_class = format!("{class} a-notice--is-dismissing");
    let dismissed_class = format!("{dismissing_class} a-notice--is-dismissed");

    Ok(view! {
        <div
            :class=$(if stage.get() == dismissed {
                dismissed_class.clone()
            } else if stage.get() == dismissing {
                dismissing_class.clone()
            } else {
                class.clone()
            })
            @click=$(|_e| stage.set(dismissing.clone()))
            @transitionend=$(|_e| {
                if stage.get() == dismissing {
                    stage.set(dismissed.clone());
                }
            })
        >
            (child)
        </div>
    }
    .boxed())
}
