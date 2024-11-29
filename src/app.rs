use crate::config::{Config, Environment};
use crate::error::Result;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppContext<T>
where
    T: AppTrait,
{
    pub app: T,
    pub config: Arc<Config>,
    pub environment: Arc<Environment>,
}

impl<T> AppContext<T>
where
    T: AppTrait,
{
    pub fn new(app: T, config: Config, environment: Environment) -> Self {
        AppContext {
            app,
            config: Arc::new(config),
            environment: Arc::new(environment),
        }
    }
}

impl<T> Deref for AppContext<T>
where
    T: AppTrait,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

/// Trait for define an application
#[async_trait::async_trait]
pub trait AppTrait: Sized + Clone + Send + Sync + 'static {
    fn app_name() -> &'static str;

    async fn init(config: Config, environment: Environment) -> Result<Self>;
}

pub(crate) async fn create_app<T>(config: Config, environment: Environment) -> Result<AppContext<T>>
where
    T: AppTrait + 'static,
{
    let app = T::init(config.clone(), environment.clone()).await?;
    Ok(AppContext::new(app, config, environment))
}
