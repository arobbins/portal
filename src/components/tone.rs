#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Neutral,
    Info,
    Success,
    Warning,
    Error,
}

impl Tone {
    pub const ALL: [Self; 5] = [
        Self::Neutral,
        Self::Info,
        Self::Success,
        Self::Warning,
        Self::Error,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Neutral => "Neutral",
            Self::Info => "Info",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }

    pub fn class(self, block: &str) -> String {
        format!("{block} {block}--{}", self.slug())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tone_adds_a_modifier_to_its_block() {
        assert_eq!(Tone::Warning.class("a-badge"), "a-badge a-badge--warning");
    }
}
