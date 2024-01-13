use axum::extract::OriginalUri;
use pulldown_cmark::{html, Event, Parser};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::time::Instant;
use walkdir::WalkDir;

use axum::http::StatusCode;
use axum::response::Html;

use crate::bootstrap_parser::bootstrap_mapper;
use askama::Template;

use chrono::NaiveDateTime;
use serde::{de, Deserialize, Deserializer};
use tracing;

#[derive(Clone, Default)]
pub struct MarkDownRouteHandler {
    pub directory: String,
    _path_index: HashMap<String, usize>,
    _rendered_paths: Vec<Post>,
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
    #[allow(dead_code)]
    show_in_feed: bool,
    #[serde(deserialize_with = "deserialize_from_str")]
    publish_dt: NaiveDateTime,
    #[serde(default)]
    feed_summary: String,
    #[serde(skip_deserializing)]
    path_from_root: String,
}

#[derive(Clone, Debug)]
struct Post {
    meta: PostMetadata,
    body: String,
}

#[derive(Clone, Debug)]
struct PostWrapper {
    prev_meta: Option<PostMetadata>,
    current_post: Post,
    next_meta: Option<PostMetadata>,
}

#[derive(Template)]
#[template(path = "markdown.html", escape = "none")]
struct MarkdownTemplate<'a> {
    post_wrapper: &'a PostWrapper,
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
        handler
    }

    fn render(&mut self) {
        let start = Instant::now();
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
                        tracing::debug!(metadata);
                        let mut meta: PostMetadata = toml::from_str(&metadata).unwrap();

                        let mut stripped_path = entry.path().to_str().unwrap().to_string();
                        assert!(stripped_path.starts_with(&self.directory));
                        stripped_path = stripped_path
                            .strip_prefix(&self.directory)
                            .unwrap()
                            .to_string();
                        meta.path_from_root = format!("/blog{}", stripped_path);
                        post_metadata = Some(meta);
                    }
                    Err(err) => {
                        tracing::error!(?metadata_file_path, ?err);
                        continue;
                    }
                }
                match fs::read_to_string(post_file_path) {
                    Ok(contents) => {
                        let parser = Parser::new(&contents)
                            .map(|event| -> Event { bootstrap_mapper(event) });
                        let mut html_output = String::new();
                        html::push_html(&mut html_output, parser);

                        tracing::info!("loaded file: {}", entry.path().display());
                        self._rendered_paths.push(Post {
                            meta: post_metadata.unwrap(),
                            body: html_output,
                        });
                    }
                    Err(_) => {
                        tracing::error!("Error reading file {}", entry.path().display());
                        continue;
                    }
                }
            }
        }
        self._rendered_paths
            .sort_by(|a, b| a.meta.publish_dt.cmp(&b.meta.publish_dt));
        for (idx, p) in self._rendered_paths.iter().enumerate() {
            self._path_index.insert(p.meta.path_from_root.clone(), idx);
        }
        tracing::info!("indexing finished in {:?}", start.elapsed());
    }

    pub fn get_html(self, OriginalUri(uri): OriginalUri) -> (StatusCode, Html<String>) {
        let vec_index = self._path_index.get(&uri.to_string());
        match vec_index {
            Some(index) => {
                let post = &self._rendered_paths[*index];
                let wrapper = PostWrapper {
                    prev_meta: if index >= &1 {
                        Some(self._rendered_paths[*index - 1].meta.clone())
                    } else {
                        None
                    },
                    current_post: post.clone(),
                    next_meta: if index + 1 < self._rendered_paths.len() {
                        Some(self._rendered_paths[*index + 1].meta.clone())
                    } else {
                        None
                    },
                };
                (
                    StatusCode::OK,
                    Html(
                        MarkdownTemplate {
                            post_wrapper: &wrapper,
                        }
                        .render()
                        .unwrap(),
                    ),
                )
            }
            None => (
                StatusCode::NOT_FOUND,
                Html(format!("No route for {:?}", uri)),
            ),
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
