use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{procedure, signal},
    view::{View, attributes, component, view},
};

use super::{styleguide_page, styleguide_section};
use crate::action::FieldError;
use crate::components::button::button;
use crate::components::form::{DialogForm, ModalSubmit, dialog_input, form_field, modal_form};
use crate::components::toaster::{NO_TOAST, toaster};
use crate::components::tone::Tone;

const MODAL_ID: &str = "styleguide-rename-project";
const NAME_FIELD: &str = "project_name";
const RENAMED_TOAST: &str = "styleguide-project-renamed";
const BLANK_NAME: &str = "Enter a project name.";

#[procedure]
async fn rename_styleguide_project(
    cx: &Cx,
    name: String,
) -> topcoat::Result<std::result::Result<String, FieldError>> {
    Ok(project_name(&name))
}

fn project_name(name: &str) -> std::result::Result<String, FieldError> {
    match name.trim() {
        "" => Err(FieldError {
            field: NAME_FIELD.to_string(),
            message: BLANK_NAME.to_string(),
        }),
        name => Ok(name.to_string()),
    }
}

#[component]
pub async fn modal_forms_section(cx: &Cx) -> Result<impl View> {
    let form = DialogForm::new(cx, None);
    let toast = signal(cx, || NO_TOAST.to_string());
    let project = signal(cx, || "Orion".to_string());
    let value = signal(cx, || "Orion".to_string());
    let draft = signal(cx, || "Orion".to_string());
    let submit = ModalSubmit {
        procedure: rename_styleguide_project,
        action: href!(styleguide_page).resolve(cx),
        label: "Save name",
        success_toast: RENAMED_TOAST,
        unreachable: "We couldn't rename the project. Please try again.",
    };

    Ok(view! {
        styleguide_section(
            title: "Modal forms",
            <p>
                "Project: "
                $(project.get())
            </p>
            <div class="p-styleguide__row">
                button(
                    attrs: attributes! { command="show-modal" commandfor=(MODAL_ID) },
                    "Rename project"
                )
            </div>
            modal_form(
                id: MODAL_ID,
                heading: "Rename project",
                form: &form,
                toast: &toast,
                submit: submit,
                args: $((draft.get(),)),
                saved: Some(&project),
                form_field(
                    label: "Project name",
                    id: "styleguide-project-name",
                    name: NAME_FIELD,
                    attrs: dialog_input(cx, &form, NAME_FIELD, &value, &draft)
                )
            )
            toaster(
                toast: &toast,
                name: RENAMED_TOAST,
                tone: Tone::Success,
                "Project renamed."
            )
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_project_name_is_a_field_error() {
        assert_eq!(project_name("  ").unwrap_err().field, NAME_FIELD);
        assert_eq!(project_name(" Atlas ").unwrap(), "Atlas");
    }
}
