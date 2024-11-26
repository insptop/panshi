use futures::future::FutureExt;
use redis::aio::ConnectionLike;
use redis::{Cmd, Pipeline, RedisFuture, Value};
use redis_pool::factory::ConnectionFactory;
use redis_pool::RedisPool;
use serde::{Deserialize, Serialize};

use crate::component::ComponentProvider;
pub use redis::{AsyncCommands, RedisError, RedisResult};

pub type AnyRedisPool = RedisPool<AnyClient, AnyConnection>;

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Redis 连接配置
    pub connection: Connection,

    /// 连接池大小
    pub pool_size: Option<usize>,

    /// 连接限制
    pub connection_limit: Option<usize>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Connection {
    /// 单机 Redis
    #[serde(rename = "single")]
    Single { url: String },
    /// Redis 集群
    #[serde(rename = "cluster")]
    Cluster { urls: Vec<String> },
}

#[derive(Clone)]
pub enum AnyClient {
    Single(redis::Client),
    Cluster(redis::cluster::ClusterClient),
}

impl AnyClient {
    pub fn new(config: &Config) -> RedisResult<Self> {
        match &config.connection {
            Connection::Single { url } => {
                let client = redis::Client::open(url.as_str())?;
                Ok(AnyClient::Single(client))
            }
            Connection::Cluster { urls } => {
                let client = redis::cluster::ClusterClient::new(urls.clone())?;
                Ok(AnyClient::Cluster(client))
            }
        }
    }

    pub async fn get_connection(&self) -> RedisResult<AnyConnection> {
        match self {
            AnyClient::Single(client) => {
                let conn = client.get_multiplexed_tokio_connection().await?;
                Ok(AnyConnection::Single(conn))
            }
            AnyClient::Cluster(client) => {
                let conn = client.get_async_connection().await?;
                Ok(AnyConnection::Cluster(conn))
            }
        }
    }
}

pub enum AnyConnection {
    Single(redis::aio::MultiplexedConnection),
    Cluster(redis::cluster_async::ClusterConnection),
}

impl ConnectionLike for AnyConnection {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        (async move {
            match self {
                AnyConnection::Single(conn) => conn.req_packed_command(cmd).await,
                AnyConnection::Cluster(conn) => conn.req_packed_command(cmd).await,
            }
        })
        .boxed()
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        (async move {
            match self {
                AnyConnection::Single(conn) => conn.req_packed_commands(cmd, offset, count).await,
                AnyConnection::Cluster(conn) => conn.req_packed_commands(cmd, offset, count).await,
            }
        })
        .boxed()
    }

    fn get_db(&self) -> i64 {
        match self {
            AnyConnection::Single(conn) => conn.get_db(),
            AnyConnection::Cluster(conn) => conn.get_db(),
        }
    }
}

#[async_trait::async_trait]
impl ConnectionFactory<AnyConnection> for AnyClient {
    async fn create(&self) -> RedisResult<AnyConnection> {
        self.get_connection().await
    }
}



#[async_trait::async_trait]
impl ComponentProvider for RedisPool<AnyClient, AnyConnection> {
    type Error = RedisError;

    type Config = Config;

    fn config_key() -> &'static str {
        "redis"
    }

    async fn create(
        config: Self::Config,
        _: &mut crate::component::ComponentRegister,
    ) -> Result<Self, Self::Error> {
        tracing::debug!("Creating RedisPool");

        let client = AnyClient::new(&config)?;
        let pool = RedisPool::new(
            client,
            config
                .pool_size
                .unwrap_or(redis_pool::pool::DEFAULT_POOL_SIZE),
            config.connection_limit,
        );
        Ok(pool)
    }
}
