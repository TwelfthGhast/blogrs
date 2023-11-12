use axum::{
    extract::State,
    http::{StatusCode, Uri},
    response::Html,
    routing::get,
    Router,
};
use md::MarkDownRouteHandler;
mod bootstrap_parser;
mod md;

#[derive(Clone)]
struct AppState {
    blog: MarkDownRouteHandler,
}

async fn test_handler() -> String {
    "Hello, World!".to_string()
}

async fn blog_handler(uri: Uri, State(state): State<AppState>) -> (StatusCode, Html<String>) {
    state.blog.get_html(uri)
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("No route for {}", uri))
}

#[tokio::main]
async fn main() {
    let md_handler = md::MarkDownRouteHandler::new("../example".to_string());
    let state = AppState { blog: md_handler };

    let blog_routes = Router::new().fallback(blog_handler);
    // build our application with a single route
    let app = Router::new()
        .nest("/blog", blog_routes)
        .route("/", get(test_handler))
        .fallback(fallback)
        .with_state(state);

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
