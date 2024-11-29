use std::path::Path;

use serde::Serialize;

use crate::error::{Error, Result};
use super::ViewRenderer;

const VIEWS_DIR: &str = "assets/views";

#[derive(Clone, Debug)]
pub struct TeraView {
    #[cfg(debug_assertions)]
    pub tera: std::sync::Arc<std::sync::Mutex<tera::Tera>>,

    #[cfg(not(debug_assertions))]
    pub tera: tera::Tera,

    #[cfg(debug_assertions)]
    pub view_dir: String,

    pub default_context: tera::Context,
}

impl TeraView {
    /// Create a Tera view engine
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    pub fn build() -> Result<Self> {
        Self::from_custom_dir(&VIEWS_DIR)
    }

    /// Create a Tera view engine from a custom directory
    ///
    /// # Errors
    ///
    /// This function will return an error if building fails
    pub fn from_custom_dir<P: AsRef<Path>>(path: &P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(Error::string(&format!(
                "missing views directory: `{}`",
                path.as_ref().display()
            )));
        }

        let mut tera = tera::Tera::new(
            path.as_ref()
                .join("**")
                .join("*.html")
                .to_str()
                .ok_or_else(|| Error::string("invalid blob"))?,
        )?;
        tera_builtins::filters::register_filters(&mut tera);
        let ctx = tera::Context::default();
        Ok(Self {
            #[cfg(debug_assertions)]
            view_dir: path.as_ref().to_string_lossy().to_string(),
            #[cfg(debug_assertions)]
            tera: std::sync::Arc::new(std::sync::Mutex::new(tera)),
            #[cfg(not(debug_assertions))]
            tera: tera,
            default_context: ctx,
        })
    }
}

impl ViewRenderer for TeraView {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        #[cfg(debug_assertions)]
        use std::borrow::BorrowMut;

        let context = tera::Context::from_serialize(data)?;

        #[cfg(debug_assertions)]
        tracing::debug!(key = key, "Tera rendering in non-optimized debug mode");
        #[cfg(debug_assertions)]
        return Ok(self.tera.lock().expect("lock").borrow_mut().render_str(
            &std::fs::read_to_string(Path::new(&self.view_dir).join(key))
                .map_err(|_e| tera::Error::template_not_found(key))?,
            &context,
        )?);

        #[cfg(not(debug_assertions))]
        return Ok(self.tera.render(key, &context)?);
    }
}

pub mod tera_builtins {
    pub mod filters {
        pub fn register_filters(tera: &mut tera::Tera) {
            tera.register_filter("number_with_delimiter", number::number_with_delimiter);
            tera.register_filter("number_to_human_size", number::number_to_human_size);
            tera.register_filter("number_to_percentage", number::number_to_percentage);
        }

        pub mod number {
            #![allow(clippy::implicit_hasher)]
            use byte_unit::Byte;
            use serde_json::value::Value;
            use std::collections::HashMap;
            use tera::Result;
            use thousands::Separable;

            /// Formats a numeric value by adding commas as thousands separators.
            ///
            ///
            /// # Examples:
            ///
            /// ```ignore
            /// {{1000 | number_with_delimiter}}
            /// ```
            ///
            /// # Errors
            ///
            /// If the `value` is not a numeric value, the function will return the original
            /// value as a string without any error.
            pub fn number_with_delimiter(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
                match value {
                    Value::Number(number) => Ok(Value::String(number.separate_with_commas())),
                    _ => Ok(value.clone()),
                }
            }

            /// Converts a numeric value (in bytes) into a human-readable size string with appropriate units.
            ///
            /// # Examples:
            ///
            /// ```ignore
            /// {{70691577 | number_to_human_size}}
            /// ```
            ///
            /// # Errors
            ///
            /// If the `value` is not a numeric value, the function will return the original
            /// value as a string without any error.
            pub fn number_to_human_size(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
                Byte::from_str(value.to_string()).map_or_else(
                    |_| Ok(value.clone()),
                    |byte_unit| {
                        Ok(Value::String(
                            byte_unit.get_appropriate_unit(false).to_string(),
                        ))
                    },
                )
            }

            /// Converts a numeric value into a formatted percentage string.
            ///
            /// # Examples:
            ///
            /// ```ignore
            /// {{100 | number_to_percentage}}
            /// {{100 | number_to_percentage(format='%n %')}}
            /// ```
            ///
            /// # Errors
            ///
            /// If the `value` is not a numeric value, the function will return the original
            /// value as a string without any error.
            pub fn number_to_percentage(value: &Value, options: &HashMap<String, Value>) -> Result<Value> {
                match value {
                    Value::Number(number) => {
                        let format = options
                            .get("format")
                            .and_then(|v| v.as_str())
                            .unwrap_or("%n%");

                        Ok(Value::String(format.replace("%n", &number.to_string())))
                    }
                    _ => Ok(value.clone()),
                }
            }

        }
    }
}