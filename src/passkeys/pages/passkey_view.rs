use crate::passkeys::queries::Passkey;
use crate::utils::format_date;

pub(super) struct PasskeyRow {
    pub id: String,
    pub name: String,
    pub synced: bool,
    pub summary: String,
}

impl PasskeyRow {
    pub fn from_passkey(passkey: &Passkey) -> Self {
        let last_used = passkey.last_used_at.map_or_else(
            || "never used".to_string(),
            |at| format!("last used {}", format_date(at)),
        );

        Self {
            id: passkey.public_id.clone(),
            name: passkey.name.clone(),
            synced: passkey.backed_up,
            summary: format!("Added {} · {last_used}", format_date(passkey.created_at)),
        }
    }
}
