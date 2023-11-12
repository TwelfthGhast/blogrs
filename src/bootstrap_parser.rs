use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};

pub fn bootstrap_mapper(event: Event) -> Event {
    match event {
        Event::Start(Tag::Paragraph) => Event::Html(CowStr::Borrowed("<p class=\"text-white\">")),
        Event::Start(Tag::CodeBlock(info)) => match info {
            CodeBlockKind::Fenced(info) => {
                let lang = info.split(' ').next().unwrap();
                if lang.is_empty() {
                    Event::Html(CowStr::Borrowed("<pre><code class=\"text-white\">"))
                } else {
                    Event::Html(CowStr::Borrowed(
                        "<pre><code class='language-python text-white'>",
                    ))
                }
            }
            CodeBlockKind::Indented => {
                Event::Html(CowStr::Borrowed("<pre><code class=\"text-white\">"))
            }
        },
        _ => event,
    }
}
