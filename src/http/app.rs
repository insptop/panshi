use serde::Deserialize;
use crate::app::{AppContext, AppTrait as BaseAppTrait};

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub listen: String,
}

#[async_trait::async_trait]
pub trait AppTrait: BaseAppTrait {
    /// Register application routes
    async fn routes(app: AppContext<Self>) -> crate::error::Result<axum::Router<AppContext<Self>>>;
}