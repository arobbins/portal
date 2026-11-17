use topcoat::{
    Result,
    context::Cx,
    runtime::signal,
    view::{Child, View, ViewExt, component, view},
};

use super::tone::Tone;

const SHOWN: &str = "shown";
const DISMISSING: &str = "dismissing";
const DISMISSED: &str = "dismissed";

#[component]
pub async fn notice(
    cx: &Cx,
    #[default] tone: Tone,
    #[default] dismissible: bool,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    if !dismissible {
        let class = tone.class("a-notice");
        return Ok(view! { <div class=(class)>(child)</div> }.boxed());
    }

    let stage = signal(cx, || SHOWN.to_string());
    let dismissing = DISMISSING.to_string();
    let dismissed = DISMISSED.to_string();

    let class = format!("{} a-notice--is-dismissible", tone.class("a-notice"));
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
            <button type="button" class="a-notice__close">"Close"</button>
        </div>
    }
    .boxed())
}
