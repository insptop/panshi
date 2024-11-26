//! 预定义的一些 HTTP 消息结构体，用于返回 JSON 格式的响应。

use axum::{Json, http::StatusCode};
use axum::response::{IntoResponse, Response};
use colored::Colorize;
use serde::{Deserialize, Serialize};
pub use crate::error::ErrorDetail;

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseMessage<T = ()> {
    /// 是否成功
    pub success: bool,
    /// 返回的数据
    pub data: T,
    /// 行为（该字段可能不存在）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behaviour: Option<()>,
}

pub fn json_response<T: Serialize>(data: T) -> Json<ResponseMessage<T>> {
    Json(ResponseMessage {
        success: true,
        data,
        behaviour: None,
    })
}

pub fn json_error_response<T: Serialize>(data: T) -> Json<ResponseMessage<T>> {
    Json(ResponseMessage {
        success: false,
        data,
        behaviour: None,
    })
}

pub type Resp<T> = crate::error::Result<Json<ResponseMessage<T>>>;

/// Return a success message use default response message, See [`ResponseMessage`].
pub fn ok<T: Serialize>(data: T) -> Resp<T> {
    Ok(json_response(data))
}


impl IntoResponse for crate::error::Error {
    /// Convert an `Error` into an HTTP response.
    fn into_response(self) -> Response {
        match &self {
            Self::WithBacktrace {
                inner,
                backtrace: _,
            } => {
                tracing::error!(
                error.msg = %inner,
                error.details = ?inner,
                "controller_error"
                );
            }
            err => {
                tracing::error!(
                error.msg = %err,
                error.details = ?err,
                "controller_error"
                );
            }
        }

        let public_facing_error = match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                ErrorDetail::new("not_found", "Resource was not found"),
            ),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("internal_server_error", "Internal Server Error"),
            ),
            Self::Unauthorized(err) => {
                tracing::warn!(err);
                (
                    StatusCode::UNAUTHORIZED,
                    ErrorDetail::new(
                        "unauthorized",
                        "You do not have permission to access this resource",
                    ),
                )
            }
            Self::CustomError(status_code, data) => (status_code, data),
            Self::WithBacktrace { inner, backtrace } => {
                println!("\n{}", inner.to_string().red().underline());
                backtrace::print_backtrace(&backtrace).unwrap();
                (
                    StatusCode::BAD_REQUEST,
                    ErrorDetail::with_reason("Bad Request"),
                )
            }
            _ => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::with_reason("Bad Request"),
            ),
        };

        (
            public_facing_error.0,
            json_error_response(public_facing_error.1),
        )
            .into_response()
    }
}

mod backtrace {
    use crate::error::{Error, Result};
    use once_cell::sync::Lazy;
    use regex::Regex;
    static NAME_BLOCKLIST: Lazy<Vec<Regex>> = Lazy::new(|| {
        [
            "^___rust_try",
            "^__pthread",
            "^__clone",
            "^<loco_rs::errors::Error as",
            "^loco_rs::errors::Error::bt",
            /*
            "^<?tokio",
            "^<?future",
            "^<?tower",
            "^<?futures",
            "^<?hyper",
            "^<?axum",
            "<F as futures_core",
            "^<F as axum::",
            "^<?std::panic",
            "^<?core::",
            "^rust_panic",
            "^rayon",
            "^rust_begin_unwind",
            "^start_thread",
            "^call_once",
            "^catch_unwind",
            */
        ]
            .iter()
            .map(|s| Regex::new(s).unwrap())
            .collect::<Vec<_>>()
    });

    static FILE_BLOCKLIST: Lazy<Vec<Regex>> = Lazy::new(|| {
        [
            "axum-.*$",
            "tower-.*$",
            "hyper-.*$",
            "tokio-.*$",
            "futures-.*$",
            "^/rustc",
        ]
            .iter()
            .map(|s| Regex::new(s).unwrap())
            .collect::<Vec<_>>()
    });

    pub fn print_backtrace(bt: &std::backtrace::Backtrace) -> Result<()> {
        backtrace_printer::print_backtrace(
            &mut std::io::stdout(),
            bt,
            &NAME_BLOCKLIST,
            &FILE_BLOCKLIST,
        )
            .map_err(Error::msg)
    }
}