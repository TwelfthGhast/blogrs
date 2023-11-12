use pulldown_cmark::{CowStr, Event, Tag};

pub fn bootstrap_mapper(event: Event) -> Event {
    match event {
        Event::Start(Tag::Paragraph) => Event::Html(CowStr::Borrowed("<p class=\"text-body\">")),
        _ => event,
    }
}
