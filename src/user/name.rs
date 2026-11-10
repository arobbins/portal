const MAX_NAME_LENGTH: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    Missing,
    TooLong,
}

impl NameProblem {
    pub fn message(self) -> &'static str {
        match self {
            Self::Missing => "Name is required.",
            Self::TooLong => "Name must be 100 characters or fewer.",
        }
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
        assert_eq!(check_name(&"é".repeat(101)), Err(NameProblem::TooLong));
        assert!(check_name(&"é".repeat(100)).is_ok());
    }
}
