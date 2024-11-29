use std::any::{Any, TypeId};
use config::Config;
use dashmap::DashMap;
use serde::de::DeserializeOwned;

pub mod redis;
pub mod session;
mod database;

pub struct ComponentRegister {
    config: Config,
    created_components: DashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ComponentRegister {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            created_components: DashMap::new(),
        }
    }

    pub async fn component<T>(&mut self) -> crate::error::Result<T>
    where
        T: ComponentProvider + 'static,
    {
        if let Some(component) = self.get::<T>().await {
            return Ok(component);
        }

        let component = T::create(self.config.get::<T::Config>(T::config_key())?, self)
            .await
            .map_err(|err| err.into())?;

        self.created_components
            .insert(TypeId::of::<T>(), Box::new(component.clone()));

        Ok(component)
    }

    pub async fn get<T>(&self) -> Option<T>
    where
        T: ComponentProvider + 'static,
    {
        if self.created_components.contains_key(&TypeId::of::<T>()) {
            let created_component = self
                .created_components
                .get(&TypeId::of::<T>())
                .expect("Component not found")
                .value()
                .downcast_ref::<T>()
                .unwrap()
                .clone();
            return Some(created_component);
        }

        None
    }
}

#[async_trait::async_trait]
pub trait ComponentProvider: Sized + Send + Sync + Clone {
    type Error: Into<crate::error::Error>;

    /// 配置的类型
    type Config: DeserializeOwned;

    /// 配置的键名
    fn config_key() -> &'static str;

    async fn create(
        config: Self::Config,
        component_register: &mut ComponentRegister,
    ) -> Result<Self, Self::Error>;
}
