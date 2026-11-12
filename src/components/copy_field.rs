use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use super::button::{ButtonVariant, button};
use super::form::{form_group, form_label};

#[component]
pub async fn copy_field(cx: &Cx, label: &str, id: &str, value: &str) -> Result<impl View> {
    let copied = signal(cx, || false);
    let copy_value = value.to_string();
    let copy_ok: std::result::Result<bool, bool> = Ok(true);
    let copy_failed: std::result::Result<bool, bool> = Err(false);

    Ok(view! {
        form_group(
            form_label(for_id: id, (label))
            <div class="a-copy-field">
                <input
                    class="a-form-input a-copy-field__input"
                    id=(id)
                    type="text"
                    value=(value)
                    readonly=(true)
                >
                button(
                    variant: ButtonVariant::Secondary,
                    attrs: attributes! {
                        @click=$(async |_e: Event| {
                            let result = raw!(
                                "await navigator.clipboard.writeText(String(${copy_value})).then(() => ${copy_ok}, () => ${copy_failed})",
                                copy_failed,
                            );
                            if result.is_ok() {
                                copied.set(true);
                            }
                        })
                    },
                    $(if copied.get() { "Copied" } else { "Copy" })
                )
                <span class="u-visually-hidden" role="status">
                    $(if copied.get() { "Copied to clipboard" } else { "" })
                </span>
            </div>
        )
    })
}
