use async_graphql::{http::GraphiQLSource, EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::Extension,
    response::{self, IntoResponse},
};

use super::Query;

pub type RootSchema = Schema<Query, EmptyMutation, EmptySubscription>;

pub fn schema() -> RootSchema {
    Schema::build(Query, EmptyMutation, EmptySubscription).finish()
}

pub async fn graphiql() -> impl IntoResponse {
    response::Html(
        GraphiQLSource::build()
            .endpoint("http://localhost:8080/graphql")
            .finish(),
    )
}

pub async fn handle(schema: Extension<RootSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}
