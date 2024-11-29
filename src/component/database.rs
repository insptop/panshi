use serde::{Deserialize, Serialize};
use std::time::Duration;
use async_trait::async_trait;
use crate::component::ComponentProvider;
use sea_orm::{ConnectOptions, Database, DbConn};

pub use sea_orm::DbConn as DB;

#[derive(Deserialize, Serialize)]
pub struct Config {
    // The URI for connecting to the database. For example:
    /// * Postgres: `postgres://root:12341234@localhost:5432/myapp_development`
    /// * Sqlite: `sqlite://db.sqlite?mode=rwc`
    pub uri: String,

    /// Enable SQLx statement logging
    #[serde(default)]
    pub enable_logging: bool,

    /// Minimum number of connections for a pool
    pub min_connections: Option<u32>,

    /// Maximum number of connections for a pool
    pub max_connections: Option<u32>,

    /// Set the timeout duration when acquiring a connection
    pub connect_timeout: Option<u64>,

    /// Set the idle duration before closing a connection
    pub idle_timeout: Option<u64>,
}

#[async_trait::async_trait]
impl ComponentProvider for DbConn {
    type Error = sea_orm::DbErr;

    type Config = Config;

    fn config_key() -> &'static str {
        "database"
    }

    async fn create(
        config: Self::Config,
        _: &mut crate::component::ComponentRegister,
    ) -> Result<Self, Self::Error> {
        let mut options = ConnectOptions::new(&config.uri);

        tracing::debug!("connect database to {}", config.uri);

        options.sqlx_logging(config.enable_logging);

        if let Some(min) = config.min_connections {
            options.min_connections(min);
        }

        if let Some(max) = config.max_connections {
            options.max_connections(max);
        }

        if let Some(timeout) = config.connect_timeout {
            options.connect_timeout(Duration::from_secs(timeout));
        }

        if let Some(timeout) = config.idle_timeout {
            options.idle_timeout(Duration::from_secs(timeout));
        }

        Database::connect(options).await
    }
}
