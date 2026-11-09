use topcoat::{
    Result,
    runtime::Signal,
    view::{Child, View, component, view},
};

use super::notice::{NoticeKind, notice};

pub const NO_TOAST: &str = "";

#[component]
pub async fn toaster(
    toast: &Signal<String>,
    name: &str,
    #[default] kind: NoticeKind,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let name = name.to_string();
    let none = NO_TOAST.to_string();

    Ok(view! {
        <div
            :class=$(if toast.get() == name {
                "a-toaster a-toaster--is-open"
            } else {
                "a-toaster"
            })
            @click=$(|_e| toast.set(none.clone()))
            @animationend=$(|_e| toast.set(none.clone()))
        >
            notice(kind: kind, (child))
        </div>
    })
}
