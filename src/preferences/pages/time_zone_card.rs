use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use super::time_zone_routes::{save_time_zone, update_time_zone_submit};
use crate::account::tabs::AccountSection;
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{
    Choice, DialogForm, ModalSubmit, dialog_input, modal_form, select_field,
};
use crate::components::icon::{IconName, icon};
use crate::flash::Flash;
use crate::preferences::actions::TIME_ZONE_FIELD;
use crate::time_zone::Zone;

const MODAL_ID: &str = "change-time-zone";
const UNREACHABLE: &str = "We couldn't save your time zone. Please try again.";

pub(super) struct ZoneDrafts {
    saved: Signal<String>,
    value: Signal<String>,
    draft: Signal<String>,
}

impl ZoneDrafts {
    pub fn new(cx: &Cx, current: &str) -> Self {
        let current = current.to_string();
        Self {
            saved: signal(cx, {
                let current = current.clone();
                move || current
            }),
            value: signal(cx, {
                let current = current.clone();
                move || current
            }),
            draft: signal(cx, move || current),
        }
    }
}

#[component]
pub(super) async fn time_zone_card(
    cx: &Cx,
    form: &DialogForm,
    drafts: &ZoneDrafts,
) -> Result<impl View> {
    let ZoneDrafts {
        saved,
        value,
        draft,
    } = drafts;
    let (error_field, error_message) = (&form.error_field, &form.error_message);

    Ok(view! {
        card(
            id: Some(AccountSection::TimeZone.anchor()),
            card_header(
                title: AccountSection::TimeZone.label(),
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        command="show-modal"
                        commandfor=(MODAL_ID)
                        @click=$(|_e: Event| {
                            value.set(saved.get());
                            draft.set(saved.get());
                            error_field.set("".to_owned());
                            error_message.set("".to_owned());
                        })
                    },
                    icon(name: IconName::Edit)
                    "Change time zone"
                )
            )
            card_body(
                <p>
                    "Dates and times are shown in "
                    $(saved.get())
                    "."
                </p>
            )
        )
    })
}

#[component]
pub(super) async fn time_zone_dialog(
    cx: &Cx,
    form: &DialogForm,
    drafts: &ZoneDrafts,
    current: &str,
    toast: &Signal<String>,
) -> Result<impl View> {
    let ZoneDrafts {
        saved,
        value,
        draft,
    } = drafts;
    let choices: Vec<Choice<'static>> = Zone::names()
        .iter()
        .map(|name| Choice {
            value: name.as_str(),
            label: name.as_str(),
        })
        .collect();
    let submit = ModalSubmit {
        procedure: save_time_zone,
        action: href!(update_time_zone_submit).resolve(cx),
        label: "Save time zone",
        success_toast: Flash::TimeZoneSaved.name(),
        unreachable: UNREACHABLE,
    };

    Ok(view! {
        modal_form(
            id: MODAL_ID,
            heading: "Change time zone",
            form: form,
            toast: toast,
            submit: submit,
            args: $((draft.get(),)),
            saved: Some(saved),
            select_field(
                label: "Time zone",
                id: "time-zone-select",
                name: TIME_ZONE_FIELD,
                options: &choices,
                selected: Some(current),
                attrs: dialog_input(cx, form, TIME_ZONE_FIELD, value, draft)
            )
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                button_type: ButtonType::Button,
                attrs: attributes! {
                    @click=$(|_e: Event| {
                        let detected = raw!(
                            "Intl.DateTimeFormat().resolvedOptions().timeZone",
                            String::new(),
                        );
                        draft.set(detected);
                        value.set(draft.get());
                    })
                },
                "Use this device's time zone"
            )
        )
    })
}
