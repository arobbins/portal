use leptos::prelude::*;

use super::StyleguideSection;
use crate::components::{Notice, NoticeKind};

#[component]
pub fn NoticesSection() -> impl IntoView {
    view! {
        <StyleguideSection title="Notices">
            <Notice kind=NoticeKind::Success>"Success! This is a notice."</Notice>
            <Notice kind=NoticeKind::Warning>"Warning! This is a notice."</Notice>
            <Notice kind=NoticeKind::Error>"Error! This is a notice."</Notice>
            <Notice kind=NoticeKind::Info>"Info: This is a notice."</Notice>
            <Notice kind=NoticeKind::Info dismissible=true>
                "Info: This is a dismissible notice."
            </Notice>
        </StyleguideSection>
    }
}
