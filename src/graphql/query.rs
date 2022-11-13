use async_graphql::Object;

pub struct Query;

#[Object]
impl Query {
    async fn health(&self) -> &'static str {
        "Hello, World!"
    }
}
