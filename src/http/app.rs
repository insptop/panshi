use serde::Deserialize;
use tokio::signal;
use crate::app::{AppContext, AppTrait as BaseAppTrait};
use crate::http::route::AppRoutes;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub listen: String,
}

#[async_trait::async_trait]
pub trait AppTrait: BaseAppTrait {
    /// Register application routes
    async fn routes(app: AppContext<Self>) -> crate::error::Result<AppRoutes<Self>>;
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}