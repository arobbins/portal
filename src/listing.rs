use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, JsonSchema)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Ascending,
    #[default]
    #[serde(rename = "desc")]
    Descending,
}

impl SortDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ascending => "asc",
            Self::Descending => "desc",
        }
    }

    pub fn aria(self) -> &'static str {
        match self {
            Self::Ascending => "ascending",
            Self::Descending => "descending",
        }
    }

    pub fn reversed(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    fn parse(value: &str) -> Option<Self> {
        [Self::Ascending, Self::Descending]
            .into_iter()
            .find(|direction| direction.as_str() == value)
    }
}

pub trait SortKey: Copy + PartialEq + Default + 'static {
    const ALL: &'static [Self];

    fn as_str(self) -> &'static str;

    fn first_direction(self) -> SortDirection;

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|key| key.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListQuery<S> {
    pub q: String,
    pub sort: S,
    pub direction: SortDirection,
    pub page: u32,
}

impl<S: SortKey> Default for ListQuery<S> {
    fn default() -> Self {
        Self {
            q: String::new(),
            sort: S::default(),
            direction: S::default().first_direction(),
            page: 0,
        }
    }
}

impl<S: SortKey> ListQuery<S> {
    pub fn parse(query: Option<&str>) -> Self {
        let mut list = Self::default();
        let mut direction = None;
        for (name, value) in url::form_urlencoded::parse(query.unwrap_or("").as_bytes()) {
            match name.as_ref() {
                "q" => list.q = value.into_owned(),
                "sort" => list.sort = S::parse(&value).unwrap_or(list.sort),
                "dir" => direction = SortDirection::parse(&value),
                "page" => list.page = value.parse().unwrap_or(0),
                _ => {}
            }
        }
        list.direction = direction.unwrap_or(list.sort.first_direction());
        list
    }

    pub fn href(&self, base: &str) -> String {
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("q", &self.q)
            .append_pair("sort", self.sort.as_str())
            .append_pair("dir", self.direction.as_str())
            .append_pair("page", &self.page.to_string())
            .finish();
        format!("{base}?{query}")
    }

    pub fn page_href(&self, base: &str, page: u32) -> String {
        Self {
            page,
            ..self.clone()
        }
        .href(base)
    }

    pub fn sorted_by(&self, sort: S) -> Self {
        let direction = if sort == self.sort {
            self.direction.reversed()
        } else {
            sort.first_direction()
        };
        Self {
            q: self.q.clone(),
            sort,
            direction,
            page: 0,
        }
    }

    pub fn sorting(&self, base: &str) -> TableSorting {
        TableSorting {
            key: self.sort.as_str(),
            direction: self.direction,
            links: S::ALL
                .iter()
                .map(|key| (key.as_str(), self.sorted_by(*key).href(base)))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSorting {
    pub key: &'static str,
    pub direction: SortDirection,
    pub links: Vec<(&'static str, String)>,
}

impl TableSorting {
    pub fn link(&self, key: &str) -> Option<&str> {
        self.links
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, href)| href.as_str())
    }

    pub fn direction_of(&self, key: &str) -> Option<SortDirection> {
        (self.key == key).then_some(self.direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    enum Fruit {
        #[default]
        Picked,
        Name,
    }

    impl SortKey for Fruit {
        const ALL: &'static [Self] = &[Self::Picked, Self::Name];

        fn as_str(self) -> &'static str {
            match self {
                Self::Picked => "picked",
                Self::Name => "name",
            }
        }

        fn first_direction(self) -> SortDirection {
            match self {
                Self::Picked => SortDirection::Descending,
                Self::Name => SortDirection::Ascending,
            }
        }
    }

    #[test]
    fn each_parameter_is_read_on_its_own() {
        let list = ListQuery::<Fruit>::parse(Some("q=pear&sort=colour&dir=up&page=x"));

        assert_eq!(list.q, "pear");
        assert_eq!(list.sort, Fruit::Picked);
        assert_eq!(list.direction, SortDirection::Descending);
        assert_eq!(list.page, 0);
    }

    #[test]
    fn a_sort_without_a_direction_uses_the_columns_first_direction() {
        let list = ListQuery::<Fruit>::parse(Some("sort=name"));

        assert_eq!(list.direction, SortDirection::Ascending);
        assert_eq!(ListQuery::<Fruit>::parse(None), ListQuery::default());
    }

    #[test]
    fn sorting_by_the_current_column_reverses_it_and_goes_back_to_the_first_page() {
        let list = ListQuery::<Fruit>::parse(Some("q=a b&sort=name&dir=asc&page=3"));

        let again = list.sorted_by(Fruit::Name);
        assert_eq!(again.direction, SortDirection::Descending);
        assert_eq!(again.page, 0);
        let other = list.sorted_by(Fruit::Picked);
        assert_eq!(other.direction, SortDirection::Descending);
        assert_eq!(
            list.page_href("/fruit", 4),
            "/fruit?q=a+b&sort=name&dir=asc&page=4"
        );
    }

    #[test]
    fn the_sorting_links_every_column_and_marks_the_current_one() {
        let sorting = ListQuery::<Fruit>::default().sorting("/fruit");

        assert_eq!(
            sorting.link("name"),
            Some("/fruit?q=&sort=name&dir=asc&page=0")
        );
        assert_eq!(
            sorting.direction_of("picked"),
            Some(SortDirection::Descending)
        );
        assert_eq!(sorting.direction_of("name"), None);
        assert_eq!(sorting.link("colour"), None);
    }
}
