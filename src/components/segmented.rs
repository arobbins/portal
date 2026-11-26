use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Signal},
    view::{View, component, view},
};

pub struct Segment {
    pub value: String,
    pub href: String,
    pub label: String,
    pub note: Option<String>,
}

#[component]
pub async fn segmented(
    cx: &Cx,
    label: &str,
    segments: &[Segment],
    selected: &Signal<String>,
) -> Result<impl View> {
    Ok(view! {
        <div class="a-segmented" role="group" aria-label=(label)>
            #[key(segment.value.as_str())]
            for segment in segments {
                segment_link(segment: segment, selected: selected)
            }
        </div>
    })
}

#[component]
async fn segment_link(cx: &Cx, segment: &Segment, selected: &Signal<String>) -> Result<impl View> {
    let value = segment.value.clone();
    let href = segment.href.clone();

    Ok(view! {
        <a
            class="a-segmented__option"
            href=(href.as_str())
            :aria-current=$(if selected.get() == value { Some("page") } else { None })
            @click=$(|e: Event| {
                e.prevent_default();
                selected.set(value.clone());
                raw!("history.replaceState(history.state, '', String(${href}))", ())
            })
        >
            (segment.label.as_str())
            if let Some(note) = &segment.note {
                <span class="a-segmented__note">(note.as_str())</span>
            }
        </a>
    })
}
