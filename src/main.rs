use axum::{
    extract::{OriginalUri, State},
    http::{StatusCode, Uri},
    response::Html,
    routing::get,
    Router,
};
use md::MarkDownRouteHandler;
mod bootstrap_parser;
mod md;
use tower_http::services::ServeDir;
use tracing;
use tracing_subscriber;

#[derive(Clone)]
struct AppState {
    blog: MarkDownRouteHandler,
}

async fn feed_handler(State(state): State<AppState>) -> Html<String> {
    state.blog.get_feed()
}

async fn blog_handler(
    OriginalUri(original_uri): OriginalUri,
    State(state): State<AppState>,
) -> (StatusCode, Html<String>) {
    state.blog.get_html(OriginalUri(original_uri))
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("No route for {}", uri))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().json().init();
    tracing::info!("initializingapp");
    let md_handler = md::MarkDownRouteHandler::new("example".to_string());
    let state = AppState { blog: md_handler };

    let blog_routes = Router::new()
        .route("/", get(feed_handler))
        .fallback(blog_handler);
    let static_serve_dir = ServeDir::new("static");
    // build our application with a single route
    let app = Router::new()
        .nest("/blog", blog_routes)
        .route("/", get(feed_handler))
        .nest_service("/static", static_serve_dir)
        .fallback(fallback)
        .with_state(state);

    tracing::info!("initialized app");
    tracing::info!("serving app");
    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
