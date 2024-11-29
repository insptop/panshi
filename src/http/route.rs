use axum::routing::MethodRouter;
use axum::{extract::Request, response::IntoResponse, routing::Route};
use regex::Regex;
use std::convert::Infallible;
use std::fmt;
use std::sync::OnceLock;
use tower::{Layer, Service};

use crate::app::AppContext;
use crate::error::Result;
use crate::http::app::AppTrait;

static DESCRIBE_METHOD_ACTION: OnceLock<Regex> = OnceLock::new();

fn get_describe_method_action() -> &'static Regex {
    DESCRIBE_METHOD_ACTION.get_or_init(|| Regex::new(r"\b(\w+):\s*BoxedHandler\b").unwrap())
}

/// Extract the allow list method actions from [`MethodRouter`].
///
/// Currently axum not exposed the action type of the router. for hold extra
/// information about routers we need to convert the `method` to string and
/// capture the details
pub fn method_action<T: AppTrait>(method: &MethodRouter<AppContext<T>>) -> Vec<http::Method> {
    let method_str = format!("{method:?}");

    get_describe_method_action()
        .captures(&method_str)
        .and_then(|captures| captures.get(1).map(|m| m.as_str().to_lowercase()))
        .and_then(|method_name| match method_name.as_str() {
            "get" => Some(http::Method::GET),
            "post" => Some(http::Method::POST),
            "put" => Some(http::Method::PUT),
            "delete" => Some(http::Method::DELETE),
            "head" => Some(http::Method::HEAD),
            "options" => Some(http::Method::OPTIONS),
            "connect" => Some(http::Method::CONNECT),
            "patch" => Some(http::Method::PATCH),
            "trace" => Some(http::Method::TRACE),
            _ => {
                tracing::info!("Unknown method: {}", method_name);
                None
            }
        })
        .into_iter()
        .collect::<Vec<_>>()
}

static NORMALIZE_URL: OnceLock<Regex> = OnceLock::new();

fn get_normalize_url() -> &'static Regex {
    NORMALIZE_URL.get_or_init(|| Regex::new(r"/+").unwrap())
}

#[derive(Clone, Debug)]
pub struct Routes<T>
where
    T: AppTrait,
{
    pub prefix: Option<String>,
    pub handlers: Vec<Handler<T>>,
    // pub version: Option<String>,
}

impl<T> Default for Routes<T>
where
    T: AppTrait,
{
    fn default() -> Self {
        Self {
            prefix: None,
            handlers: vec![],
            // version: None,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct Handler<T>
where
    T: AppTrait,
{
    pub uri: String,
    pub method: axum::routing::MethodRouter<AppContext<T>>,
    pub actions: Vec<axum::http::Method>,
}

impl<T> Routes<T>
where
    T: AppTrait,
{
    /// Creates a new [`Routes`] instance with default settings.
    #[must_use]
    pub fn new() -> Self {
        Routes::<T>::default()
    }

    #[must_use]
    pub fn at(prefix: &str) -> Self {
        Self {
            prefix: Some(prefix.to_string()),
            ..Routes::<T>::default()
        }
    }

    #[must_use]
    pub fn add(mut self, uri: &str, method: axum::routing::MethodRouter<AppContext<T>>) -> Self {
        method_action(&method);
        self.handlers.push(Handler {
            uri: uri.to_owned(),
            actions: method_action(&method),
            method,
        });
        self
    }

    #[must_use]
    pub fn prefix(mut self, uri: &str) -> Self {
        self.prefix = Some(uri.to_owned());
        self
    }

    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + 'static,
        L::Service: Service<Request> + Clone + Send + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        Self {
            prefix: self.prefix,
            handlers: self
                .handlers
                .iter()
                .map(|handler| Handler {
                    uri: handler.uri.clone(),
                    actions: handler.actions.clone(),
                    method: handler.method.clone().layer(layer.clone()),
                })
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct AppRoutes<T>
where
    T: AppTrait,
{
    prefix: Option<String>,
    routes: Vec<Routes<T>>,
}

#[derive(Debug)]
pub struct ListRoutes<T>
where
    T: AppTrait,
{
    pub uri: String,
    pub actions: Vec<axum::http::Method>,
    pub method: axum::routing::MethodRouter<AppContext<T>>,
}

impl<T> fmt::Display for ListRoutes<T>
where
    T: AppTrait,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let actions_str = self
            .actions
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");

        write!(f, "[{}] {}", actions_str, self.uri)
    }
}

impl<T> AppRoutes<T>
where
    T: AppTrait,
{
    /// Create a new instance with the default routes.
    #[must_use]
    pub fn with_default_routes() -> Self {
        let routes = Self::empty().add_route(default_routes::ping::routes());
        #[cfg(feature = "with-db")]
        let routes = routes.add_route(super::health::routes());

        routes
    }

    /// Create an empty instance.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            prefix: None,
            routes: vec![],
        }
    }

    #[must_use]
    pub fn collect(&self) -> Vec<ListRoutes<T>> {
        let base_url_prefix = self
            .get_prefix()
            // add a leading slash forcefully. Axum routes must start with a leading slash.
            // if we have double leading slashes - it will get normalized into a single slash later
            .map_or("/".to_string(), |url| format!("/{}", url.as_str()));

        self.get_routes()
            .iter()
            .flat_map(|controller| {
                let mut uri_parts = vec![base_url_prefix.clone()];
                if let Some(prefix) = controller.prefix.as_ref() {
                    uri_parts.push(prefix.to_string());
                }
                controller.handlers.iter().map(move |handler| {
                    let mut parts = uri_parts.clone();
                    parts.push(handler.uri.to_string());
                    let joined_parts = parts.join("/");

                    let normalized = get_normalize_url().replace_all(&joined_parts, "/");
                    let uri = if normalized == "/" {
                        normalized.to_string()
                    } else {
                        normalized.strip_suffix('/').map_or_else(
                            || normalized.to_string(),
                            std::string::ToString::to_string,
                        )
                    };

                    ListRoutes {
                        uri,
                        actions: handler.actions.clone(),
                        method: handler.method.clone(),
                    }
                })
            })
            .collect()
    }

    /// Get the prefix of the routes.
    #[must_use]
    pub fn get_prefix(&self) -> Option<&String> {
        self.prefix.as_ref()
    }

    /// Get the routes.
    #[must_use]
    pub fn get_routes(&self) -> &[Routes<T>] {
        self.routes.as_ref()
    }

    #[must_use]
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    /// Add a single route.
    #[must_use]
    pub fn add_route(mut self, route: Routes<T>) -> Self {
        self.routes.push(route);
        self
    }

    /// Add multiple routes.
    #[must_use]
    pub fn add_routes(mut self, mounts: Vec<Routes<T>>) -> Self {
        for mount in mounts {
            self.routes.push(mount);
        }
        self
    }

    /// Add the routes to an existing Axum Router, and set a list of middlewares
    /// that configure in the [`config::Config`]
    ///
    /// # Errors
    /// Return an [`Result`] when could not convert the router setup to
    /// [`axum::Router`].
    #[allow(clippy::cognitive_complexity)]
    pub fn to_router(
        &self,
        ctx: AppContext<T>,
        mut app: axum::Router<AppContext<T>>,
    ) -> Result<axum::Router> {
        // IMPORTANT: middleware ordering in this function is opposite to what you
        // intuitively may think. when using `app.layer` to add individual middleware,
        // the LAST middleware is the FIRST to meet the outside world (a user request
        // starting), or "LIFO" order.
        // We build the "onion" from the inside (start of this function),
        // outwards (end of this function). This is why routes is first in coding order
        // here (the core of the onion), and request ID is amongst the last
        // (because every request is assigned with a unique ID, which starts its
        // "life").
        //
        // NOTE: when using ServiceBuilder#layer the order is FIRST to LAST (but we
        // don't use ServiceBuilder because it requires too complex generic typing for
        // this function). ServiceBuilder is recommended to save compile times, but that
        // may be a thing of the past as we don't notice any issues with compile times
        // using the router directly, and ServiceBuilder has been reported to give
        // issues in compile times itself (https://github.com/rust-lang/crates.io/pull/7443).
        //
        for router in self.collect() {
            tracing::info!("{}", router.to_string());
            app = app.route(&router.uri, router.method);
        }

        // let middlewares = self.middlewares::<H>(&ctx);
        // for mid in middlewares {
        //     app = mid.apply(app)?;
        //     tracing::info!(name = mid.name(), "+middleware");
        // }

        let router = app.with_state(ctx);
        Ok(router)
    }
}

pub mod default_routes {
    pub mod ping {
        use axum::{response::Response, routing::get};
        use serde::Serialize;

        use crate::error::Result;
        use crate::http::app::AppTrait;
        use crate::http::{message, route::Routes};

        /// Represents the health status of the application.
        #[derive(Serialize)]
        struct Health {
            pub ok: bool,
        }

        /// Check application ping endpoint
        async fn ping() -> Result<Response> {
            message::json(Health { ok: true })
        }

        /// Defines and returns the health-related routes.
        pub fn routes<T>() -> Routes<T>
        where
            T: AppTrait,
        {
            Routes::new().add("/_ping", get(ping))
        }
    }
}
