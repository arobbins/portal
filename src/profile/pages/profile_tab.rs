use topcoat::{
    Result,
    context::{Cx, try_context},
    router::href,
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use super::delete_account_card::{DeleteDrafts, delete_account_dialog, delete_account_row};
use super::delete_routes::delete_account_problem;
use super::email_card::{EmailDrafts, change_email_dialog, email_row};
use super::email_routes::email_change_problem;
use super::profile_routes::{ProfileUpdateFailed, save_profile_name, update_profile_submit};
use crate::action::ActionProblem;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{Autocomplete, form_field};
use crate::components::form::{
    DialogForm, ModalSubmit, dialog_error_toast, dialog_input, modal_form, page_toast,
};
use crate::components::icon::{IconName, icon};
use crate::components::property_list::{property, property_list};
use crate::data_export::pages::export_card::{ExportDrafts, export_dialog, export_row};
use crate::data_export::pages::export_routes::export_problem;
use crate::flash::{Flash, flash_toasts, take_flash};
use crate::user::model::User;
use crate::user::name::NAME_FIELD;

const MODAL_ID: &str = "edit-profile";
const UNREACHABLE: &str = "We couldn't save your profile. Please try again.";
const FLASHES: &[Flash] = &[
    Flash::ProfileSaved,
    Flash::EmailConfirmed,
    Flash::EmailChangeSent,
    Flash::EmailChanged,
    Flash::PasswordReset,
];

struct NameDrafts {
    value: Signal<String>,
    draft: Signal<String>,
}

#[component]
pub async fn profile_tab(cx: &Cx, user: &User, display_name: &Signal<String>) -> Result<impl View> {
    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<ProfileUpdateFailed>(cx);
    let name = failed.map_or_else(|| user.name.clone(), |failed| failed.name.clone());
    let name_failure = failed.map(|failed| failed.problem.spec());
    let email_failure = email_change_problem(cx).map(ActionProblem::spec);
    let delete_failure = delete_account_problem(cx).map(ActionProblem::spec);
    let export_failure = export_problem(cx).map(ActionProblem::spec);
    let failure = name_failure
        .or(email_failure)
        .or(export_failure)
        .or(delete_failure);
    let form = DialogForm::new(cx, name_failure);
    let email_form = DialogForm::new(cx, email_failure);
    let delete_form = DialogForm::new(cx, delete_failure);
    let export_form = DialogForm::new(cx, export_failure);
    let export_drafts = ExportDrafts::new(cx);
    let toast = page_toast(cx, failure.is_some(), flash);
    let email_drafts = EmailDrafts::new(cx);
    let delete_drafts = DeleteDrafts::new(cx);
    let drafts = NameDrafts {
        value: signal(cx, || name.clone()),
        draft: signal(cx, || name.clone()),
    };

    Ok(view! {
        <div class="p-account__sub-page-content">
            details_card(
                display_name: display_name,
                email: &user.email,
                forms: (&form, &email_form),
                drafts: (&drafts, &email_drafts)
            )
            data_card(
                forms: (&export_form, &delete_form),
                drafts: (&export_drafts, &delete_drafts)
            )
            edit_profile_dialog(
                form: &form,
                drafts: &drafts,
                display_name: display_name,
                toast: &toast
            )
            change_email_dialog(
                form: &email_form,
                email: &user.email,
                drafts: &email_drafts,
                toast: &toast
            )
            delete_account_dialog(
                form: &delete_form,
                drafts: &delete_drafts,
                toast: &toast
            )
            export_dialog(form: &export_form, drafts: &export_drafts, toast: &toast)
            flash_toasts(toast: &toast, flashes: FLASHES)
            dialog_error_toast(failure: failure, toast: &toast)
        </div>
    })
}

#[component]
async fn details_card(
    cx: &Cx,
    display_name: &Signal<String>,
    email: &str,
    forms: (&DialogForm, &DialogForm),
    drafts: (&NameDrafts, &EmailDrafts),
) -> Result<impl View> {
    let (form, email_form) = forms;
    let (drafts, email_drafts) = drafts;

    Ok(view! {
        card(
            id: Some("your-details"),
            card_header(title: "Your details")
            card_body(
                property_list(
                    name_row(display_name: display_name, form: form, drafts: drafts)
                    email_row(email: email, form: email_form, drafts: email_drafts)
                )
            )
        )
    })
}

#[component]
async fn data_card(
    cx: &Cx,
    forms: (&DialogForm, &DialogForm),
    drafts: (&ExportDrafts, &DeleteDrafts),
) -> Result<impl View> {
    let (export_form, delete_form) = forms;
    let (export_drafts, delete_drafts) = drafts;

    Ok(view! {
        card(
            id: Some("your-data"),
            card_header(title: "Your data")
            card_body(
                property_list(
                    export_row(form: export_form, drafts: export_drafts)
                    delete_account_row(form: delete_form, drafts: delete_drafts)
                )
            )
        )
    })
}

#[component]
async fn name_row(
    cx: &Cx,
    display_name: &Signal<String>,
    form: &DialogForm,
    drafts: &NameDrafts,
) -> Result<impl View> {
    let NameDrafts { value, draft } = drafts;
    let error_message = &form.error_message;

    Ok(view! {
        property(
            name: "Name",
            live_description: Some(display_name),
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    command="show-modal"
                    commandfor=(MODAL_ID)
                    @click=$(|_e: Event| {
                        value.set(display_name.get());
                        draft.set(display_name.get());
                        error_message.set("".to_owned());
                    })
                },
                icon(name: IconName::Edit)
                "Edit profile"
            )
        )
    })
}

#[component]
async fn edit_profile_dialog(
    cx: &Cx,
    form: &DialogForm,
    drafts: &NameDrafts,
    display_name: &Signal<String>,
    toast: &Signal<String>,
) -> Result<impl View> {
    let NameDrafts { value, draft } = drafts;
    let submit = ModalSubmit {
        procedure: save_profile_name,
        action: href!(update_profile_submit).resolve(cx),
        label: "Save profile",
        success_toast: Flash::ProfileSaved.name(),
        unreachable: UNREACHABLE,
    };

    Ok(view! {
        modal_form(
            id: MODAL_ID,
            heading: "Edit profile",
            form: form,
            toast: toast,
            submit: submit,
            args: $((draft.get(),)),
            saved: Some(display_name),
            form_field(
                label: "Name",
                id: "edit-profile-name",
                autocomplete: Some(Autocomplete::Name),
                name: NAME_FIELD,
                required: true,
                attrs: dialog_input(cx, form, NAME_FIELD, value, draft)
            )
        )
    })
}
