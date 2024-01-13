use axum::{
    extract::connect_info::ConnectInfo,
    extract::{OriginalUri, State},
    http::{StatusCode, Uri},
    response::Html,
    routing::get,
    Router,
};
use md::MarkDownRouteHandler;
use std::net::SocketAddr;
use tokio::time::Instant;
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
    ConnectInfo(ipv4): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> (StatusCode, Html<String>) {
    let start = Instant::now();
    let (status_code, html) = state.blog.get_html(OriginalUri(original_uri.clone()));
    match status_code {
        StatusCode::OK => {
            tracing::info!(
                "[{}] uri ({}) found; {:?}",
                ipv4,
                original_uri,
                start.elapsed()
            );
        }
        StatusCode::NOT_FOUND => {
            tracing::info!(
                "[{}] uri ({}) not found; {:?}",
                ipv4,
                original_uri,
                start.elapsed()
            );
        }
        default => {
            tracing::info!(
                "[{}] uri ({}) unknown status {}; {:?}",
                ipv4,
                original_uri,
                default,
                start.elapsed()
            );
        }
    };
    (status_code, html)
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
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
