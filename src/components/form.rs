mod choices;
mod dialog;
mod layout;
mod submit;
mod text;
mod types;

pub use choices::{checkbox_group, radio_group, select_field};
pub use dialog::{DialogForm, dialog_error_toast, dialog_form_error, dialog_input, page_toast};
pub use layout::{form, form_actions, form_col, form_error, form_group, form_label, form_row};
pub use submit::{ModalSubmit, modal_form};
pub use text::{checkbox, code_field, form_field, form_input, textarea_field};
#[cfg(test)]
pub use types::FORM_ERROR_ID;
pub use types::{Autocomplete, Choice, FormVariant, InputType, LabelLink};
