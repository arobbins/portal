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

    let is_dismissing = signal(cx, || false);
    let is_dismissed = signal(cx, || false);

    let class = format!("a-notice {} a-notice--is-dismissible", kind.modifier());
    let dismissing_class = format!("{class} a-notice--is-dismissing");
    let dismissed_class = format!("{dismissing_class} a-notice--is-dismissed");

    Ok(view! {
        <div
            :class=$(if is_dismissed.get() {
                dismissed_class.clone()
            } else if is_dismissing.get() {
                dismissing_class.clone()
            } else {
                class.clone()
            })
            @click=$(|_e| is_dismissing.set(true))
            @transitionend=$(|_e| {
                if is_dismissing.get() {
                    is_dismissed.set(true);
                }
            })
        >
            (child)
        </div>
    }
    .boxed())
}
