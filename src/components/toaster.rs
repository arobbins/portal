use topcoat::{
    Result,
    runtime::Signal,
    view::{Child, View, component, view},
};

use super::notice::{NoticeKind, notice};

#[component]
pub async fn toaster(
    open: &Signal<bool>,
    #[default] kind: NoticeKind,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            :class=$(if open.get() {
                "a-toaster a-toaster--is-open"
            } else {
                "a-toaster"
            })
            @click=$(|_e| open.set(false))
            @animationend=$(|_e| open.set(false))
        >
            notice(kind: kind, (child))
        </div>
    })
}
