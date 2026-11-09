const MIN_PASSWORD_LENGTH: usize = 8;
const MAX_PASSWORD_LENGTH: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordProblem {
    TooShort,
    TooLong,
    Mismatch,
}

impl PasswordProblem {
    pub fn message(self) -> &'static str {
        match self {
            Self::TooShort => "Password must be at least 8 characters.",
            Self::TooLong => "Password must be 256 characters or fewer.",
            Self::Mismatch => "Passwords do not match.",
        }
    }
}

pub fn check_new_password(password: &str, confirmation: &str) -> Result<(), PasswordProblem> {
    let length = password.chars().count();

    if length < MIN_PASSWORD_LENGTH {
        return Err(PasswordProblem::TooShort);
    }

    if length > MAX_PASSWORD_LENGTH {
        return Err(PasswordProblem::TooLong);
    }

    if password != confirmation {
        return Err(PasswordProblem::Mismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_matching_password_of_a_sensible_length() {
        assert_eq!(check_new_password("long-enough", "long-enough"), Ok(()));
    }

    #[test]
    fn rejects_short_long_and_mismatched_passwords() {
        assert_eq!(
            check_new_password("short", "short"),
            Err(PasswordProblem::TooShort)
        );
        let long = "a".repeat(257);
        assert_eq!(
            check_new_password(&long, &long),
            Err(PasswordProblem::TooLong)
        );
        assert_eq!(
            check_new_password("long-enough", "different"),
            Err(PasswordProblem::Mismatch)
        );
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert_eq!(
            check_new_password("ééééééé", "ééééééé"),
            Err(PasswordProblem::TooShort)
        );
    }
}
