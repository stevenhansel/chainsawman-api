use std::net::SocketAddr;

use axum::{extract::Extension, routing, Router, Server};

use crate::graphql;

pub async fn run() {
    let graphql_schema = graphql::handler::schema();

    let app = Router::new()
        .route("/", routing::get(root))
        .route(
            "/graphql",
            routing::get(graphql::handler::graphiql).post(graphql::handler::handle),
        )
        .layer(Extension(graphql_schema));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    tracing::info!("API Server is listening on {}", addr);

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}
