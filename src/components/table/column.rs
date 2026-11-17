#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ColumnFormat {
    #[default]
    Text,
    Number,
    Date,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ColumnPriority {
    #[default]
    Always,
    Wide,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TableDensity {
    #[default]
    Comfortable,
    Compact,
}

impl TableDensity {
    pub(super) fn modifier(self) -> &'static str {
        match self {
            Self::Comfortable => "a-table--comfortable",
            Self::Compact => "a-table--compact",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TableLayout {
    #[default]
    Scroll,
    Stacked,
}

impl TableLayout {
    pub(super) fn modifier(self) -> &'static str {
        match self {
            Self::Scroll => "a-table--scroll",
            Self::Stacked => "a-table--stacked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column {
    pub label: &'static str,
    pub sort_key: Option<&'static str>,
    pub format: ColumnFormat,
    pub priority: ColumnPriority,
    pub truncate: bool,
}

impl Column {
    pub const fn new(label: &'static str) -> Self {
        Self {
            label,
            sort_key: None,
            format: ColumnFormat::Text,
            priority: ColumnPriority::Always,
            truncate: false,
        }
    }

    pub const fn sortable(self, key: &'static str) -> Self {
        Self {
            sort_key: Some(key),
            ..self
        }
    }

    pub const fn number(self) -> Self {
        Self {
            format: ColumnFormat::Number,
            ..self
        }
    }

    pub const fn date(self) -> Self {
        Self {
            format: ColumnFormat::Date,
            ..self
        }
    }

    pub const fn wide_only(self) -> Self {
        Self {
            priority: ColumnPriority::Wide,
            ..self
        }
    }

    pub const fn truncated(self) -> Self {
        Self {
            truncate: true,
            ..self
        }
    }

    pub(super) fn classes(self, base: &str) -> String {
        let mut classes = vec![base.to_string()];
        match self.format {
            ColumnFormat::Text => {}
            ColumnFormat::Number => classes.push(format!("{base}--number")),
            ColumnFormat::Date => classes.push(format!("{base}--date")),
        }
        if self.priority == ColumnPriority::Wide {
            classes.push(format!("{base}--wide"));
        }
        if self.truncate {
            classes.push(format!("{base}--truncate"));
        }
        classes.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_column_adds_a_modifier_for_each_option() {
        let plain = Column::new("Plan");
        let joined = Column::new("Joined").date();
        let amount = Column::new("Amount").number().wide_only().truncated();

        assert_eq!(plain.classes("a-table__cell"), "a-table__cell");
        assert_eq!(
            joined.classes("a-table__cell"),
            "a-table__cell a-table__cell--date"
        );
        assert_eq!(
            amount.classes("a-table__cell"),
            "a-table__cell a-table__cell--number a-table__cell--wide a-table__cell--truncate"
        );
    }
}
