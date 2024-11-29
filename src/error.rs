//! 错误相关定义

use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not found")]
    NotFound,

    #[error("internal server error")]
    InternalServerError,

    #[error("{0}")]
    Unauthorized(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    ConfigError(#[from] config::ConfigError),

    #[error(transparent)]
    RedisError(#[from] redis::RedisError),

    #[error(transparent)]
    DB(#[from] sea_orm::DbErr),

    #[error(transparent)]
    RedisPoolError(#[from] redis_pool::errors::RedisPoolError),

    #[error(transparent)]
    SessionError(#[from] axum_session::SessionError),

    #[error(transparent)]
    Tera(#[from] tera::Error),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("{0}")]
    Message(String),

    #[error("")]
    CustomError(axum::http::StatusCode, ErrorDetail),

    #[error("{inner}\n{backtrace}")]
    WithBacktrace {
        inner: Box<Self>,
        backtrace: Box<std::backtrace::Backtrace>,
    },
}

#[derive(Debug, Serialize)]
/// Structure representing details about an error.
pub struct ErrorDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ErrorDetail {
    /// Create a new `ErrorDetail` with the specified error and description.
    #[must_use]
    pub fn new<T: Into<String>>(error: T, description: T) -> Self {
        Self {
            error: Some(error.into()),
            description: Some(description.into()),
        }
    }

    /// Create an `ErrorDetail` with only an error reason and no description.
    #[must_use]
    pub fn with_reason<T: Into<String>>(error: T) -> Self {
        Self {
            error: Some(error.into()),
            description: None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Wrap standard error to [Error]
    pub fn wrap(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Any(Box::new(err)) //.bt()
    }

    /// Only get message from error
    pub fn msg(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Message(err.to_string()) //.bt()
    }
    #[must_use]
    pub fn string(s: &str) -> Self {
        Self::Message(s.to_string())
    }
    #[must_use]
    pub fn bt(self) -> Self {
        let backtrace = std::backtrace::Backtrace::capture();
        match backtrace.status() {
            std::backtrace::BacktraceStatus::Disabled
            | std::backtrace::BacktraceStatus::Unsupported => self,
            _ => Self::WithBacktrace {
                inner: Box::new(self),
                backtrace: Box::new(backtrace),
            },
        }
    }
}