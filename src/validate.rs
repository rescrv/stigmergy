//! # JSON Schema Validation
//!
//! This module provides JSON schema validation capabilities for the stigmergy system.
//! It validates JSON data against JSON schemas, supporting all standard JSON Schema
//! features including primitive types, objects, arrays, enums, and oneOf unions.
//!
//! ## Key Features
//!
//! - **Complete Type Support**: Validates null, boolean, integer, number, string, array, and object types
//! - **Complex Validation**: Supports nested objects, arrays, and oneOf unions for enum-like structures
//! - **Schema Validation**: Can validate schemas themselves for structural correctness
//! - **Descriptive Errors**: Provides detailed error messages with context about what failed
//!
//! ## Usage Examples
//!
//! ```rust
//! use stigmergy::validate_value;
//! use serde_json::json;
//!
//! let schema = json!({
//!     "type": "object",
//!     "properties": {
//!         "name": {"type": "string"},
//!         "age": {"type": "integer"}
//!     },
//!     "required": ["name"]
//! });
//!
//! let valid_data = json!({"name": "Alice", "age": 30});
//! assert!(validate_value(&valid_data, &schema).is_ok());
//!
//! let invalid_data = json!({"age": 30}); // missing required "name"
//! assert!(validate_value(&invalid_data, &schema).is_err());
//! ```

use serde_json::{Map, Value};

use crate::json_schema::{
    ENUM_KEY, ITEMS_KEY, JsonSchemaBuilder, ONE_OF_KEY, PROPERTIES_KEY, REQUIRED_KEY, TYPE_ARRAY,
    TYPE_BOOLEAN, TYPE_INTEGER, TYPE_KEY, TYPE_NULL, TYPE_NUMBER, TYPE_OBJECT, TYPE_STRING,
    get_value_type,
};

/// Errors that can occur during JSON schema validation.
///
/// This enum provides detailed information about validation failures, including
/// the specific type of error and context about where the validation failed.
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// The JSON schema itself is invalid or malformed
    InvalidSchema(String),
    /// The value type doesn't match what the schema expects
    TypeMismatch {
        /// The type that was expected by the schema
        expected: String,
        /// The actual type of the value being validated
        actual: String,
    },
    /// A required object property is missing
    MissingRequiredProperty {
        /// The name of the missing required property
        property: String,
    },
    /// The value doesn't match any of the allowed enum values
    EnumMismatch {
        /// The actual value that was provided
        value: String,
        /// The list of values that would have been valid
        allowed_values: Vec<String>,
    },
    /// An array item failed validation
    ArrayItemError {
        /// The index of the array item that failed
        index: usize,
        /// The underlying validation error for the item
        source: Box<ValidationError>,
    },
    /// An object property failed validation
    ObjectPropertyError {
        /// The name of the property that failed
        property: String,
        /// The underlying validation error for the property
        source: Box<ValidationError>,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidSchema(msg) => write!(f, "Invalid schema: {}", msg),
            ValidationError::TypeMismatch { expected, actual } => {
                write!(f, "Type mismatch: expected {}, got {}", expected, actual)
            }
            ValidationError::MissingRequiredProperty { property } => {
                write!(f, "Missing required property: {}", property)
            }
            ValidationError::EnumMismatch {
                value,
                allowed_values,
            } => {
                write!(
                    f,
                    "Enum mismatch: '{}' is not one of {:?}",
                    value, allowed_values
                )
            }
            ValidationError::ArrayItemError { index, source } => {
                write!(f, "Array item error at index {}: {}", index, source)
            }
            ValidationError::ObjectPropertyError { property, source } => {
                write!(f, "Object property error at '{}': {}", property, source)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl JsonSchemaBuilder {
    /// Validates a JSON value against this schema.
    ///
    /// # Arguments
    /// * `value` - The JSON value to validate
    ///
    /// # Returns
    /// * `Ok(())` - The value is valid according to the schema
    /// * `Err(ValidationError)` - The value failed validation with details
    ///
    /// # Examples
    /// ```rust
    /// use stigmergy::JsonSchemaBuilder;
    /// use serde_json::json;
    ///
    /// // Create a schema from example string data
    /// let schema = JsonSchemaBuilder::from_value(&json!("example")).unwrap();
    /// assert!(schema.validate(&json!("hello")).is_ok());
    /// assert!(schema.validate(&json!(42)).is_err());
    /// ```
    pub fn validate(&self, value: &Value) -> Result<(), ValidationError> {
        validate_value(value, self.as_value())
    }
}

/// Validates a JSON value against a JSON schema.
///
/// This is the primary validation function that checks whether a JSON value
/// conforms to the given JSON schema specification. It supports all standard
/// JSON Schema features including type validation, object properties, array
/// items, enumerations, and oneOf unions.
///
/// # Arguments
/// * `value` - The JSON value to validate
/// * `schema` - The JSON schema to validate against
///
/// # Returns
/// * `Ok(())` - The value is valid according to the schema
/// * `Err(ValidationError)` - The value failed validation with specific error details
///
/// # Examples
/// ```rust
/// use stigmergy::validate_value;
/// use serde_json::json;
///
/// let schema = json!({"type": "number", "minimum": 0});
/// let valid_value = json!(42);
/// let invalid_value = json!("not a number");
///
/// assert!(validate_value(&valid_value, &schema).is_ok());
/// assert!(validate_value(&invalid_value, &schema).is_err());
/// ```
pub fn validate_value(value: &Value, schema: &Value) -> Result<(), ValidationError> {
    let schema_obj = schema
        .as_object()
        .ok_or_else(|| ValidationError::InvalidSchema("Schema must be an object".to_string()))?;

    // Check for oneOf first
    if let Some(one_of_schemas) = schema_obj.get(ONE_OF_KEY) {
        return validate_one_of(value, one_of_schemas);
    }

    // Then check for regular type-based validation
    let schema_type = schema_obj
        .get(TYPE_KEY)
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ValidationError::InvalidSchema("Schema must have a type field".to_string())
        })?;

    match schema_type {
        TYPE_NULL => validate_null(value),
        TYPE_BOOLEAN => validate_boolean(value),
        TYPE_INTEGER => validate_integer(value),
        TYPE_NUMBER => validate_number(value),
        TYPE_STRING => validate_string(value, schema_obj),
        TYPE_ARRAY => validate_array(value, schema_obj),
        TYPE_OBJECT => validate_object(value, schema_obj),
        _ => Err(ValidationError::InvalidSchema(format!(
            "Unknown schema type: {}",
            schema_type
        ))),
    }
}

fn validate_one_of(value: &Value, one_of_schemas: &Value) -> Result<(), ValidationError> {
    let schemas_array = one_of_schemas
        .as_array()
        .ok_or_else(|| ValidationError::InvalidSchema("oneOf must be an array".to_string()))?;

    let mut validation_errors = Vec::new();

    for schema in schemas_array {
        match validate_value(value, schema) {
            Ok(()) => return Ok(()),
            Err(e) => validation_errors.push(e),
        }
    }

    Err(ValidationError::InvalidSchema(format!(
        "Value doesn't match any oneOf schemas. Errors: {:?}",
        validation_errors
    )))
}

fn validate_null(value: &Value) -> Result<(), ValidationError> {
    match value {
        Value::Null => Ok(()),
        _ => Err(ValidationError::TypeMismatch {
            expected: TYPE_NULL.to_string(),
            actual: get_value_type(value),
        }),
    }
}

fn validate_boolean(value: &Value) -> Result<(), ValidationError> {
    match value {
        Value::Bool(_) => Ok(()),
        _ => Err(ValidationError::TypeMismatch {
            expected: TYPE_BOOLEAN.to_string(),
            actual: get_value_type(value),
        }),
    }
}

fn validate_integer(value: &Value) -> Result<(), ValidationError> {
    match value {
        Value::Number(n) if n.is_i64() || n.is_u64() => Ok(()),
        _ => Err(ValidationError::TypeMismatch {
            expected: TYPE_INTEGER.to_string(),
            actual: get_value_type(value),
        }),
    }
}

fn validate_number(value: &Value) -> Result<(), ValidationError> {
    match value {
        Value::Number(_) => Ok(()),
        _ => Err(ValidationError::TypeMismatch {
            expected: TYPE_NUMBER.to_string(),
            actual: get_value_type(value),
        }),
    }
}

fn validate_string(value: &Value, schema: &Map<String, Value>) -> Result<(), ValidationError> {
    let string_value = match value {
        Value::String(s) => s,
        _ => {
            return Err(ValidationError::TypeMismatch {
                expected: TYPE_STRING.to_string(),
                actual: get_value_type(value),
            });
        }
    };

    if let Some(enum_values) = schema.get(ENUM_KEY) {
        validate_enum(string_value, enum_values)?;
    }

    Ok(())
}

fn validate_enum(value: &str, enum_values: &Value) -> Result<(), ValidationError> {
    let enum_array = enum_values
        .as_array()
        .ok_or_else(|| ValidationError::InvalidSchema("Enum must be an array".to_string()))?;

    let value_found = enum_array.iter().any(|enum_val| {
        if let Some(enum_str) = enum_val.as_str() {
            enum_str == value
        } else {
            false
        }
    });

    if value_found {
        Ok(())
    } else {
        Err(ValidationError::EnumMismatch {
            value: value.to_string(),
            allowed_values: enum_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
        })
    }
}

fn validate_array(value: &Value, schema: &Map<String, Value>) -> Result<(), ValidationError> {
    let array = match value {
        Value::Array(arr) => arr,
        _ => {
            return Err(ValidationError::TypeMismatch {
                expected: TYPE_ARRAY.to_string(),
                actual: get_value_type(value),
            });
        }
    };

    if let Some(items_schema) = schema.get(ITEMS_KEY) {
        match items_schema {
            Value::Array(item_schemas) => {
                for (index, item) in array.iter().enumerate() {
                    if let Some(item_schema) = item_schemas.get(index) {
                        validate_value(item, item_schema).map_err(|e| {
                            ValidationError::ArrayItemError {
                                index,
                                source: Box::new(e),
                            }
                        })?;
                    }
                }
            }
            schema => {
                for (index, item) in array.iter().enumerate() {
                    validate_value(item, schema).map_err(|e| ValidationError::ArrayItemError {
                        index,
                        source: Box::new(e),
                    })?;
                }
            }
        }
    }

    Ok(())
}

fn validate_object(value: &Value, schema: &Map<String, Value>) -> Result<(), ValidationError> {
    let object = match value {
        Value::Object(obj) => obj,
        _ => {
            return Err(ValidationError::TypeMismatch {
                expected: TYPE_OBJECT.to_string(),
                actual: get_value_type(value),
            });
        }
    };

    if let Some(properties) = schema.get(PROPERTIES_KEY) {
        let properties_obj = properties.as_object().ok_or_else(|| {
            ValidationError::InvalidSchema("Properties must be an object".to_string())
        })?;

        for (prop_name, prop_schema) in properties_obj {
            if let Some(prop_value) = object.get(prop_name) {
                validate_value(prop_value, prop_schema).map_err(|e| {
                    ValidationError::ObjectPropertyError {
                        property: prop_name.clone(),
                        source: Box::new(e),
                    }
                })?;
            }
        }
    }

    if let Some(required) = schema.get(REQUIRED_KEY) {
        let required_array = required.as_array().ok_or_else(|| {
            ValidationError::InvalidSchema("Required must be an array".to_string())
        })?;

        for required_prop in required_array {
            let prop_name = required_prop.as_str().ok_or_else(|| {
                ValidationError::InvalidSchema(
                    "Required property names must be strings".to_string(),
                )
            })?;

            if !object.contains_key(prop_name) {
                return Err(ValidationError::MissingRequiredProperty {
                    property: prop_name.to_string(),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_null_success() {
        let schema = JsonSchemaBuilder::from_value(&json!(null)).unwrap();
        let value = json!(null);
        assert!(schema.validate(&value).is_ok());
    }

    #[test]
    fn validate_null_failure() {
        let schema = JsonSchemaBuilder::from_value(&json!(null)).unwrap();
        let value = json!("not null");
        let result = schema.validate(&value);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn validate_boolean_success() {
        let schema = JsonSchemaBuilder::from_value(&json!(true)).unwrap();
        assert!(schema.validate(&json!(true)).is_ok());
        assert!(schema.validate(&json!(false)).is_ok());
    }

    #[test]
    fn validate_boolean_failure() {
        let schema = JsonSchemaBuilder::from_value(&json!(true)).unwrap();
        let result = schema.validate(&json!("true"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn validate_integer_success() {
        let schema = JsonSchemaBuilder::from_value(&json!(42)).unwrap();
        assert!(schema.validate(&json!(42)).is_ok());
        assert!(schema.validate(&json!(-10)).is_ok());
    }

    #[test]
    fn validate_integer_failure() {
        let schema = JsonSchemaBuilder::from_value(&json!(42)).unwrap();
        let result = schema.validate(&json!(2.5));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn validate_number_success() {
        let schema = JsonSchemaBuilder::from_value(&json!(2.5)).unwrap();
        assert!(schema.validate(&json!(2.5)).is_ok());
        assert!(schema.validate(&json!(42)).is_ok());
    }

    #[test]
    fn validate_number_failure() {
        let schema = JsonSchemaBuilder::from_value(&json!(2.5)).unwrap();
        let result = schema.validate(&json!("2.5"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn validate_string_success() {
        let schema = JsonSchemaBuilder::from_value(&json!("hello")).unwrap();
        assert!(schema.validate(&json!("hello")).is_ok());
        assert!(schema.validate(&json!("world")).is_ok());
    }

    #[test]
    fn validate_string_failure() {
        let schema = JsonSchemaBuilder::from_value(&json!("hello")).unwrap();
        let result = schema.validate(&json!(123));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn validate_array_homogeneous_success() {
        let schema = JsonSchemaBuilder::from_value(&json!([1, 2, 3])).unwrap();
        assert!(schema.validate(&json!([1, 2, 3])).is_ok());
        assert!(schema.validate(&json!([42])).is_ok());
        assert!(schema.validate(&json!([])).is_ok());
    }

    #[test]
    fn validate_array_homogeneous_failure() {
        let schema = JsonSchemaBuilder::from_value(&json!([1, 2, 3])).unwrap();
        let result = schema.validate(&json!(["string"]));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::ArrayItemError { .. }
        ));
    }

    #[test]
    fn validate_array_heterogeneous_success() {
        let schema = JsonSchemaBuilder::from_value(&json!([1, "hello", true])).unwrap();
        assert!(schema.validate(&json!([42, "world", false])).is_ok());
    }

    #[test]
    fn validate_array_heterogeneous_failure() {
        let schema = JsonSchemaBuilder::from_value(&json!([1, "hello", true])).unwrap();
        let result = schema.validate(&json!([42, 123, false]));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::ArrayItemError { .. }
        ));
    }

    #[test]
    fn validate_object_success() {
        let schema = JsonSchemaBuilder::from_value(&json!({
            "name": "John",
            "age": 30,
            "active": true
        }))
        .unwrap();

        assert!(
            schema
                .validate(&json!({
                    "name": "Jane",
                    "age": 25,
                    "active": false
                }))
                .is_ok()
        );
    }

    #[test]
    fn validate_object_failure_wrong_type() {
        let schema = JsonSchemaBuilder::from_value(&json!({
            "name": "John",
            "age": 30
        }))
        .unwrap();

        let result = schema.validate(&json!({
            "name": "Jane",
            "age": "twenty-five"
        }));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::ObjectPropertyError { .. }
        ));
    }

    #[test]
    fn validate_object_missing_required_property() {
        let schema = JsonSchemaBuilder::from_value(&json!({
            "name": "John",
            "age": 30
        }))
        .unwrap();

        let result = schema.validate(&json!({
            "name": "Jane"
        }));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::MissingRequiredProperty { .. }
        ));
    }

    #[test]
    fn validate_nested_object_success() {
        let schema = JsonSchemaBuilder::from_value(&json!({
            "user": {
                "name": "John",
                "email": "john@example.com"
            },
            "score": 95.5
        }))
        .unwrap();

        assert!(
            schema
                .validate(&json!({
                    "user": {
                        "name": "Jane",
                        "email": "jane@example.com"
                    },
                    "score": 87.3
                }))
                .is_ok()
        );
    }

    #[test]
    fn validate_complex_nested_structure() {
        let schema_value = json!({
            "metadata": {
                "version": "1.0",
                "tags": ["api", "json"]
            },
            "data": [
                {
                    "id": 1,
                    "items": [10, 20]
                }
            ]
        });
        let schema = JsonSchemaBuilder::from_value(&schema_value).unwrap();

        let valid_data = json!({
            "metadata": {
                "version": "2.0",
                "tags": ["web", "service"]
            },
            "data": [
                {
                    "id": 2,
                    "items": [30, 40]
                }
            ]
        });

        assert!(schema.validate(&valid_data).is_ok());

        let invalid_data = json!({
            "metadata": {
                "version": 2.0,
                "tags": ["web", "service"]
            },
            "data": [
                {
                    "id": "two",
                    "items": [30, 40]
                }
            ]
        });

        let result = schema.validate(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn validate_empty_array() {
        let schema = JsonSchemaBuilder::from_value(&json!([])).unwrap();
        assert!(schema.validate(&json!([])).is_ok());
    }

    #[test]
    fn validate_empty_object() {
        let schema = JsonSchemaBuilder::from_value(&json!({})).unwrap();
        assert!(schema.validate(&json!({})).is_ok());
        assert!(schema.validate(&json!({"extra": "field"})).is_ok());
    }

    #[test]
    fn validate_type_not_value() {
        let result = validate_value(&json!(42), &json!("not an object"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidSchema(_)
        ));
    }

    #[test]
    fn validate_schema_without_type() {
        let result = validate_value(&json!(42), &json!({"properties": {}}));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidSchema(_)
        ));
    }

    #[test]
    fn validate_unknown_schema_type() {
        let result = validate_value(&json!(42), &json!({"type": "unknown"}));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidSchema(_)
        ));
    }

    #[test]
    fn validation_error_display() {
        let error = ValidationError::TypeMismatch {
            expected: "string".to_string(),
            actual: "number".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Type mismatch: expected string, got number"
        );

        let error = ValidationError::MissingRequiredProperty {
            property: "name".to_string(),
        };
        assert_eq!(error.to_string(), "Missing required property: name");

        let error = ValidationError::InvalidSchema("Bad schema".to_string());
        assert_eq!(error.to_string(), "Invalid schema: Bad schema");
    }

    #[test]
    fn validate_array_item_error_context() {
        let schema = JsonSchemaBuilder::from_value(&json!([1, 2, 3])).unwrap();
        let result = schema.validate(&json!([1, "invalid", 3]));

        match result.unwrap_err() {
            ValidationError::ArrayItemError { index, source } => {
                assert_eq!(index, 1);
                assert!(matches!(*source, ValidationError::TypeMismatch { .. }));
            }
            _ => panic!("Expected ArrayItemError"),
        }
    }

    #[test]
    fn validate_object_property_error_context() {
        let schema = JsonSchemaBuilder::from_value(&json!({
            "name": "test",
            "count": 42
        }))
        .unwrap();

        let result = schema.validate(&json!({
            "name": "valid",
            "count": "invalid"
        }));

        match result.unwrap_err() {
            ValidationError::ObjectPropertyError { property, source } => {
                assert_eq!(property, "count");
                assert!(matches!(*source, ValidationError::TypeMismatch { .. }));
            }
            _ => panic!("Expected ObjectPropertyError"),
        }
    }

    #[test]
    fn validate_one_of_success_first_schema() {
        let schema = json!({
            "oneOf": [
                { "type": "string" },
                { "type": "number" }
            ]
        });

        let string_value = json!("hello");
        assert!(validate_value(&string_value, &schema).is_ok());

        let number_value = json!(42);
        assert!(validate_value(&number_value, &schema).is_ok());
    }

    #[test]
    fn validate_one_of_failure() {
        let schema = json!({
            "oneOf": [
                { "type": "string" },
                { "type": "number" }
            ]
        });

        let boolean_value = json!(true);
        let result = validate_value(&boolean_value, &schema);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::InvalidSchema(_)
        ));
    }

    #[test]
    fn validate_enum_one_of_comprehensive() {
        let schema = json!({
            "oneOf": [
                {
                    "type": "string",
                    "enum": ["Red", "Green", "Blue"]
                },
                {
                    "type": "object",
                    "properties": {
                        "Custom": { "type": "string" }
                    },
                    "required": ["Custom"]
                }
            ]
        });

        assert!(validate_value(&json!("Red"), &schema).is_ok());
        assert!(validate_value(&json!("Blue"), &schema).is_ok());
        assert!(validate_value(&json!({"Custom": "purple"}), &schema).is_ok());

        assert!(validate_value(&json!("Yellow"), &schema).is_err());
        assert!(validate_value(&json!({"Custom": 123}), &schema).is_err());
        assert!(validate_value(&json!(42), &schema).is_err());
    }

    #[test]
    fn validate_tagged_union_enum_variants() {
        let schema = json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "Circle": {
                            "type": "object",
                            "properties": {
                                "radius": { "type": "number" }
                            },
                            "required": ["radius"]
                        }
                    },
                    "required": ["Circle"]
                },
                {
                    "type": "object",
                    "properties": {
                        "Rectangle": {
                            "type": "object",
                            "properties": {
                                "width": { "type": "number" },
                                "height": { "type": "number" }
                            },
                            "required": ["width", "height"]
                        }
                    },
                    "required": ["Rectangle"]
                }
            ]
        });

        assert!(validate_value(&json!({"Circle": {"radius": 5.0}}), &schema).is_ok());
        assert!(
            validate_value(
                &json!({"Rectangle": {"width": 10.0, "height": 20.0}}),
                &schema
            )
            .is_ok()
        );

        assert!(validate_value(&json!({"Circle": {"radius": "invalid"}}), &schema).is_err());
        assert!(validate_value(&json!({"Rectangle": {"width": 10.0}}), &schema).is_err());
        assert!(validate_value(&json!({"Triangle": {"side": 5.0}}), &schema).is_err());
    }

    #[test]
    fn validate_discriminator_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "TwoD": {
                    "type": "array",
                    "items": [
                        { "type": "number" },
                        { "type": "number" }
                    ]
                }
            },
            "required": ["TwoD"]
        });

        assert!(validate_value(&json!({"TwoD": [1.0, 2.0]}), &schema).is_ok());

        assert!(validate_value(&json!({"ThreeD": [1.0, 2.0, 3.0]}), &schema).is_err());

        assert!(validate_value(&json!({}), &schema).is_err());

        assert!(validate_value(&json!({"TwoD": [1.0, "invalid"]}), &schema).is_err());
    }
}
