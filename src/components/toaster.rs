use topcoat::{
    Result,
    context::Cx,
    runtime::Signal,
    view::{Child, View, component, view},
};

use super::notice::notice;
use super::tone::Tone;

pub const NO_TOAST: &str = "";

#[component]
pub async fn toaster(
    cx: &Cx,
    toast: &Signal<String>,
    name: &str,
    #[default] tone: Tone,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let name = name.to_string();
    let none = NO_TOAST.to_string();

    Ok(view! {
        <div
            role="status"
            :class=$(if toast.get() == name {
                "a-toaster a-toaster--is-open"
            } else {
                "a-toaster"
            })
            @click=$(|_e| toast.set(none.clone()))
            @animationend=$(|_e| toast.set(none.clone()))
        >
            notice(tone: tone, (child))
        </div>
    })
}
