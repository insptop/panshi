use crate::component::redis::{AnyClient, AnyConnection, AnyRedisPool};
use crate::component::{ComponentProvider, ComponentRegister};
use axum_session::{
    DatabaseError, DatabasePool, SessionAnyPool, SessionConfig,
};
use serde::{Deserialize, Serialize};
pub use axum_session::{SessionAnySessionStore, SessionLayer};

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub life_time: Option<u64>,
}

#[async_trait::async_trait]
impl ComponentProvider for SessionAnySessionStore {
    type Error = crate::error::Error;

    type Config = Config;

    fn config_key() -> &'static str {
        "session"
    }

    async fn create(
        config: Self::Config,
        component_register: &mut ComponentRegister,
    ) -> Result<Self, Self::Error> {
        let redis_pool = component_register.component::<AnyRedisPool>().await?;

        let client = redis_pool.factory().clone();
        let store = match client {
            AnyClient::Single(client) => {
                SessionAnySessionStore::new(
                    Some(SessionAnyPool::new(
                        axum_session_redispool::SessionRedisPool::from(
                            redis_pool::SingleRedisPool::from(client),
                        ),
                    )),
                    SessionConfig::default(),
                )
                .await?
            }
            AnyClient::Cluster(client) => {
                SessionAnySessionStore::new(
                    Some(SessionAnyPool::new(
                        axum_session_redispool::SessionRedisClusterPool::from(
                            redis_pool::ClusterRedisPool::from(client),
                        ),
                    )),
                    SessionConfig::default(),
                )
                .await?
            }
        };

        Ok(store)
    }
}
