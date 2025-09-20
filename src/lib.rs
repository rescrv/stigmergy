mod json_schema;
mod validate;

pub use json_schema::JsonSchema;
pub use validate::{validate_value, ValidationError};
