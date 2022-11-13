use std::sync::Arc;

use async_trait::async_trait;

use crate::models::DevilDetail;

#[async_trait]
pub trait DevilDataSource: Send + Sync + 'static {
    async fn scrape(&self) -> Result<Vec<DevilDetail>, std::io::Error>;
}

pub struct DevilService {
    scraper: Arc<dyn DevilDataSource>,
}

impl DevilService {
    pub fn new(scraper: Arc<dyn DevilDataSource>) -> Self {
        Self { scraper }
    }
}
