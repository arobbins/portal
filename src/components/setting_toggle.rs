use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Signal, Surrogated, TypedProcedure, signal},
    view::{View, attributes, component, view},
};

use crate::action::FieldError;
use crate::components::toaster::{NO_TOAST, toaster};
use crate::components::toggle::toggle;
use crate::components::tone::Tone;

pub struct InlineSave<'a, P: TypedProcedure> {
    pub procedure: P,
    pub success_toast: &'a str,
    pub unreachable: &'a str,
}

#[component]
pub async fn setting_toggle<P>(
    cx: &Cx,
    label: &str,
    id: &str,
    name: &str,
    checked: bool,
    toast: &Signal<String>,
    save: InlineSave<'_, P>,
    #[default] description: Option<&str>,
) -> Result<impl View>
where
    P: TypedProcedure<Args = (bool,), Output = std::result::Result<String, FieldError>>
        + Surrogated
        + Copy,
    <P as Surrogated>::Surrogate: serde::Serialize,
{
    let InlineSave {
        procedure,
        success_toast,
        unreachable,
    } = save;
    let saving = signal(cx, || false);
    let success_toast = success_toast.to_string();
    let failed_toast = format!("{id}-failed");
    let input_id = id.to_string();
    let failed: std::result::Result<String, FieldError> = Err(FieldError::general(unreachable));

    Ok(view! {
        <div
            class="a-setting-toggle"
            :aria-busy=$(if saving.get() { "true" } else { "false" })
        >
            toggle(
                label: label,
                id: id,
                name: name,
                description: description,
                checked: checked,
                attrs: attributes! {
                    :disabled=$(saving.get())
                    @change=$(async |e: Event| {
                        let _on = e.current_target.checked;
                        saving.set(true);
                        toast.set(NO_TOAST.to_owned());
                        let outcome = raw!(
                            "await Promise.resolve(${procedure}.call(${_on})).catch(() => ${failed})",
                            failed.clone(),
                        );
                        saving.set(false);
                        if outcome.is_ok() {
                            toast.set(success_toast.clone());
                        } else {
                            toast.set(failed_toast.clone());
                            raw!(
                                "document.getElementById(String(${input_id})).checked = String(${_on}) !== 'true'",
                                (),
                            )
                        }
                    })
                }
            )
        </div>
        toaster(toast: toast, name: &failed_toast, tone: Tone::Error, (unreachable))
    })
}
