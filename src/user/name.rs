use crate::action::ActionProblem;

pub const MAX_NAME_LENGTH: usize = 100;
pub const NAME_FIELD: &str = "name";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    Missing,
    TooLong,
}

impl NameProblem {
    pub const ALL: [Self; 2] = [Self::Missing, Self::TooLong];
}

impl ActionProblem for NameProblem {
    fn code(self) -> &'static str {
        match self {
            Self::Missing => "name_missing",
            Self::TooLong => "name_too_long",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Missing => "Name is required.",
            Self::TooLong => "Name must be 100 characters or fewer.",
        }
    }

    fn field(self) -> Option<&'static str> {
        Some(NAME_FIELD)
    }
}

pub fn check_name(name: &str) -> Result<&str, NameProblem> {
    let name = name.trim();

    if name.is_empty() {
        return Err(NameProblem::Missing);
    }

    if name.chars().count() > MAX_NAME_LENGTH {
        return Err(NameProblem::TooLong);
    }

    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_accepts_a_normal_name() {
        assert_eq!(check_name("  Ada Lovelace "), Ok("Ada Lovelace"));
    }

    #[test]
    fn rejects_blank_and_over_long_names() {
        assert_eq!(check_name("   "), Err(NameProblem::Missing));
        assert_eq!(
            check_name(&"é".repeat(MAX_NAME_LENGTH + 1)),
            Err(NameProblem::TooLong)
        );
        assert!(check_name(&"é".repeat(MAX_NAME_LENGTH)).is_ok());
    }

    #[test]
    fn the_length_message_matches_the_limit() {
        assert!(
            NameProblem::TooLong
                .message()
                .contains(&format!("{MAX_NAME_LENGTH} characters"))
        );
    }
}
