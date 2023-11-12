use pulldown_cmark::{html, Event, Parser};
use std::collections::HashMap;
use std::fs;
use walkdir::WalkDir;

use axum::http::{StatusCode, Uri};
use axum::response::Html;

use crate::bootstrap_parser::bootstrap_mapper;
use askama::Template;

#[derive(Clone, Default)]
pub struct MarkDownRouteHandler {
    pub directory: String,
    _rendered_paths: HashMap<String, String>,
}

#[derive(Template)]
#[template(path = "markdown.html", escape = "none")]
struct MarkdownTemplate<'a> {
    md: &'a String,
}

#[derive(Template)]
#[template(path = "feed.html", escape = "none")]
struct FeedTemplate<'a> {
    items: &'a Vec<FeedItem>,
}

struct FeedItem {
    title: String,
    body: String,
    path_from_root: String,
}

impl MarkDownRouteHandler {
    pub fn new(directory: String) -> MarkDownRouteHandler {
        let mut handler = MarkDownRouteHandler {
            directory: directory,
            _rendered_paths: HashMap::new(),
        };
        handler.render();
        println!("{:?}", handler._rendered_paths);
        handler
    }

    fn render(&mut self) {
        for entry in WalkDir::new(&self.directory) {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                match fs::read_to_string(entry.path()) {
                    Ok(contents) => {
                        let parser = Parser::new(&contents)
                            .map(|event| -> Event { bootstrap_mapper(event) });
                        let mut html_output = String::new();
                        html::push_html(&mut html_output, parser);
                        let mut stripped_path = entry.path().to_str().unwrap().to_string();
                        assert!(stripped_path.starts_with(&self.directory));
                        stripped_path = stripped_path
                            .strip_prefix(&self.directory)
                            .unwrap()
                            .to_string();
                        if stripped_path.ends_with(".md") {
                            stripped_path = stripped_path.strip_suffix(".md").unwrap().to_string();
                        }
                        println!(
                            "Found file: {} {} \n{:?}",
                            entry.path().display(),
                            stripped_path,
                            html_output
                        );
                        self._rendered_paths.insert(stripped_path, html_output);
                    }
                    Err(_) => {
                        panic!("Could not open file {:?}", entry.path());
                    }
                }
            }
        }
    }

    pub fn get_html(self, uri: Uri) -> (StatusCode, Html<String>) {
        match self._rendered_paths.get(&uri.to_string()) {
            Some(html) => (
                StatusCode::OK,
                Html(MarkdownTemplate { md: html }.render().unwrap()),
            ),
            None => (StatusCode::NOT_FOUND, Html(format!("No route for {}", uri))),
        }
    }

    pub fn get_feed(self) -> Html<String> {
        let mut x: Vec<FeedItem> = Vec::new();
        for (path, entry) in self._rendered_paths.iter() {
            x.push(FeedItem {
                title: "Title".to_string(),
                body: entry.to_owned(),
                path_from_root: format!("{}{}", "/blog", path.to_owned()),
            });
        }
        Html(FeedTemplate { items: &x }.render().unwrap())
    }
}
