use axum::extract::OriginalUri;
use pulldown_cmark::{html, Event, Parser};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use axum::http::StatusCode;
use axum::response::Html;

use crate::bootstrap_parser::bootstrap_mapper;
use askama::Template;

use chrono::NaiveDateTime;
use serde::{de, Deserialize, Deserializer};

#[derive(Clone, Default)]
pub struct MarkDownRouteHandler {
    pub directory: String,
    _path_index: HashMap<String, usize>,
    _rendered_paths: Vec<Post>
}

// https://stackoverflow.com/questions/57614558/how-to-use-a-custom-serde-deserializer-for-chrono-timestamps
// You can use this deserializer for any type that implements FromStr
// and the FromStr::Err implements Display
fn deserialize_from_str<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%z").map_err(de::Error::custom)
}

#[derive(Clone, Debug, Deserialize)]
struct PostMetadata {
    title: String,
    show_in_feed: bool,
    #[serde(deserialize_with = "deserialize_from_str")]
    publish_dt: NaiveDateTime,
}

#[derive(Clone, Debug)]
struct Post {
    meta: PostMetadata,
    body: String,
    path_from_root: String,
}

#[derive(Template)]
#[template(path = "markdown.html", escape = "none")]
struct MarkdownTemplate<'a> {
    post: &'a Post,
}

#[derive(Template)]
#[template(path = "feed.html", escape = "none")]
struct FeedTemplate<'a> {
    posts: &'a Vec<Post>,
}

impl MarkDownRouteHandler {
    pub fn new(directory: String) -> MarkDownRouteHandler {
        let mut handler = MarkDownRouteHandler {
            directory: directory,
            _path_index: HashMap::new(),
            _rendered_paths: Vec::new(),
        };
        handler.render();
        println!("{:?}", handler._rendered_paths);
        handler
    }

    fn render(&mut self) {
        self._rendered_paths = Vec::new();
        self._path_index = HashMap::new();
        for entry in WalkDir::new(&self.directory) {
            let entry = entry.unwrap();
            if entry.file_type().is_dir() {
                let post_file = entry.path().join("post.md");
                let post_file_path = Path::new(&post_file);
                let metadata_file = entry.path().join("metadata.toml");
                let metadata_file_path = Path::new(&metadata_file);

                #[allow(unused_assignments)]
                let mut post_metadata: Option<PostMetadata> = None;
                match fs::read_to_string(metadata_file_path) {
                    Ok(metadata) => {
                        println!("{}", metadata);
                        post_metadata = Some(toml::from_str(&metadata).unwrap());
                    }
                    Err(err) => {
                        println!("Error with {:?}: {}", metadata_file_path, err);
                        continue;
                    }
                }
                match fs::read_to_string(post_file_path) {
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
                        println!(
                            "Found file: {} {} \n{:?}",
                            entry.path().display(),
                            stripped_path,
                            html_output
                        );
                        self._rendered_paths.push(
                            Post {
                                meta: post_metadata.unwrap(),
                                body: html_output,
                                path_from_root: format!("/blog{}", stripped_path),
                            },
                        );
                    }
                    Err(_) => {
                        println!("Could not open file {:?}", entry.path());
                        continue;
                    }
                }
            }
        }
        self._rendered_paths.sort_by(|a, b| a.meta.publish_dt.cmp(&b.meta.publish_dt));
        for (idx, p) in self._rendered_paths.iter().enumerate() {
            self._path_index.insert(p.path_from_root.clone(), idx);
        }
    }

    pub fn get_html(self, OriginalUri(uri): OriginalUri) -> (StatusCode, Html<String>) {
        let vec_index = self._path_index.get(&uri.to_string());
        match vec_index {
            Some(index) => {
                let post = &self._rendered_paths[*index];
                (StatusCode::OK, Html(MarkdownTemplate { post: &post.clone() }.render().unwrap()))
            },
            None => (StatusCode::NOT_FOUND, Html(format!("No route for {:?}", uri)))
        }
    }

    pub fn get_feed(self) -> Html<String> {
        let mut x: Vec<Post> = Vec::new();
        for post in self._rendered_paths.iter().rev() {
            x.push(post.clone());
        }
        Html(FeedTemplate { posts: &x }.render().unwrap())
    }
}
