//! # JSON Schema Generation and Validation
//!
//! This module provides utilities for generating JSON schemas from Rust types and
//! working with JSON schema structures. It supports automatic schema generation
//! for common Rust types and provides utilities for creating complex schemas.
//!
//! ## Key Features
//!
//! - **Automatic Schema Generation**: Generate schemas from Rust types using `Serialize`
//! - **Value-based Schema Creation**: Generate schemas from JSON values
//! - **Enum Schema Support**: Create schemas for Rust enums with unit and complex variants
//! - **Schema Validation**: Integration with validation systems
//!
//! ## Usage Examples
//!
//! ```rust
//! use stigmergy::{JsonSchema, JsonSchemaBuilder};
//! use serde_json::json;
//!
//! // Generate schema from a type using derive macro
//! #[derive(stigmergy_derive::JsonSchema)]
//! struct Position {
//!     x: f64,
//!     y: f64,
//! }
//!
//! let schema = Position::json_schema();
//! println!("{}", serde_json::to_string_pretty(&schema).unwrap());
//!
//! // Generate schema from a value
//! let value = json!({"name": "test", "count": 42});
//! let schema = JsonSchemaBuilder::from_value(&value).unwrap();
//! ```

use serde::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer,
};
use serde_json::{Map, Value};

pub(crate) const TYPE_KEY: &str = "type";
pub(crate) const TYPE_NULL: &str = "null";
pub(crate) const TYPE_BOOLEAN: &str = "boolean";
pub(crate) const TYPE_INTEGER: &str = "integer";
pub(crate) const TYPE_NUMBER: &str = "number";
pub(crate) const TYPE_STRING: &str = "string";
pub(crate) const TYPE_ARRAY: &str = "array";
pub(crate) const TYPE_OBJECT: &str = "object";
pub(crate) const PROPERTIES_KEY: &str = "properties";
pub(crate) const REQUIRED_KEY: &str = "required";
pub(crate) const ITEMS_KEY: &str = "items";
pub(crate) const ENUM_KEY: &str = "enum";
pub(crate) const ONE_OF_KEY: &str = "oneOf";

/// Determines the JSON schema type name for a given JSON value
pub(crate) fn get_value_type(value: &Value) -> String {
    match value {
        Value::Null => TYPE_NULL.to_string(),
        Value::Bool(_) => TYPE_BOOLEAN.to_string(),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                TYPE_INTEGER.to_string()
            } else {
                TYPE_NUMBER.to_string()
            }
        }
        Value::String(_) => TYPE_STRING.to_string(),
        Value::Array(_) => TYPE_ARRAY.to_string(),
        Value::Object(_) => TYPE_OBJECT.to_string(),
    }
}

/// A JSON schema representation that can be used for validation and documentation.
///
/// This struct wraps a JSON schema and provides utilities for creating schemas from
/// Rust types and values, as well as converting schemas to various formats.
///
/// # Examples
///
/// ```rust
/// use stigmergy::JsonSchemaBuilder;
/// use serde_json::json;
///
/// let value = json!({"name": "test"});
/// let schema = JsonSchemaBuilder::from_value(&value).unwrap();
/// let schema_json = schema.as_value();
/// ```
#[derive(Debug, Clone)]
pub struct JsonSchemaBuilder {
    /// The underlying JSON schema representation
    schema: Value,
}

impl JsonSchemaBuilder {
    /// Generates a JSON schema from a JSON value.
    ///
    /// This method analyzes the structure of a JSON value and generates
    /// an appropriate schema that would validate similar values.
    ///
    /// # Arguments
    /// * `value` - The JSON value to analyze
    ///
    /// # Returns
    /// * `Ok(JsonSchema)` - The generated schema
    /// * `Err(JsonSchemaError)` - Schema generation error
    ///
    /// # Examples
    /// ```rust
    /// use stigmergy::JsonSchemaBuilder;
    /// use serde_json::json;
    ///
    /// let value = json!({"x": 1.0, "y": 2.0});
    /// let schema = JsonSchemaBuilder::from_value(&value).unwrap();
    /// ```
    pub fn from_value(value: &Value) -> Result<Self, JsonSchemaError> {
        let schema = Self::value_to_schema(value);
        Ok(JsonSchemaBuilder { schema })
    }

    /// Returns a reference to the underlying JSON schema value.
    ///
    /// # Examples
    /// ```rust
    /// use stigmergy::JsonSchemaBuilder;
    /// use serde_json::json;
    ///
    /// let value = json!({"name": "test"});
    /// let schema = JsonSchemaBuilder::from_value(&value).unwrap();
    /// let schema_value = schema.as_value();
    /// ```
    pub fn as_value(&self) -> &Value {
        &self.schema
    }

    /// Serializes the schema to a pretty-printed JSON string.
    ///
    /// # Returns
    /// * `Ok(String)` - The JSON string representation
    /// * `Err(serde_json::Error)` - Serialization error
    ///
    /// # Examples
    /// ```rust
    /// use stigmergy::JsonSchemaBuilder;
    /// use serde_json::json;
    ///
    /// let value = json!({"count": 42});
    /// let schema = JsonSchemaBuilder::from_value(&value).unwrap();
    /// let json_string = schema.to_string().unwrap();
    /// ```
    pub fn to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.schema)
    }

    /// Creates a JSON schema for Rust enums with both unit and complex variants.
    ///
    /// This method generates a schema using `oneOf` to represent enum variants,
    /// supporting both simple unit variants (represented as strings) and complex
    /// variants with associated data.
    ///
    /// # Arguments
    /// * `unit_variants` - Simple enum variants represented as strings
    /// * `complex_variants` - Complex enum variants with their own schemas
    ///
    /// # Returns
    /// A JsonSchema that validates enum values using `oneOf`
    ///
    /// # Examples
    /// ```rust
    /// use stigmergy::JsonSchemaBuilder;
    /// use serde_json::json;
    ///
    /// let unit_variants = vec!["Red".to_string(), "Blue".to_string()];
    /// let complex_schema = JsonSchemaBuilder::from_value(&json!({"type": "object"})).unwrap();
    /// let enum_schema = JsonSchemaBuilder::create_enum_schema(unit_variants, vec![complex_schema]);
    /// ```
    pub fn create_enum_schema(
        unit_variants: Vec<String>,
        complex_variants: Vec<JsonSchemaBuilder>,
    ) -> Self {
        let mut schemas = Vec::new();

        if !unit_variants.is_empty() {
            schemas.push(SchemaGenerator::create_enum_schema(unit_variants));
        }

        for complex_variant in complex_variants {
            schemas.push(complex_variant.schema);
        }

        let schema = if schemas.len() == 1 {
            schemas.into_iter().next().unwrap()
        } else {
            SchemaGenerator::create_one_of_schema(schemas)
        };

        JsonSchemaBuilder { schema }
    }

    fn value_to_schema(value: &Value) -> Value {
        match value {
            Value::Null => SchemaGenerator::create_typed_schema(TYPE_NULL),
            Value::Bool(_) => SchemaGenerator::create_typed_schema(TYPE_BOOLEAN),
            Value::Number(n) => {
                let type_name = if n.is_i64() || n.is_u64() {
                    TYPE_INTEGER
                } else {
                    TYPE_NUMBER
                };
                SchemaGenerator::create_typed_schema(type_name)
            }
            Value::String(_) => SchemaGenerator::create_typed_schema(TYPE_STRING),
            Value::Array(arr) => {
                let item_schema = if arr.is_empty() {
                    SchemaGenerator::create_typed_schema(TYPE_NULL)
                } else {
                    let item_schemas: Vec<Value> = arr.iter().map(Self::value_to_schema).collect();

                    if item_schemas.iter().all(|s| s == &item_schemas[0]) {
                        item_schemas[0].clone()
                    } else {
                        Value::Array(item_schemas)
                    }
                };
                SchemaGenerator::create_array_schema(item_schema)
            }
            Value::Object(obj) => {
                let mut properties = Map::new();
                let mut required_fields = Vec::new();

                for (key, val) in obj {
                    properties.insert(key.clone(), Self::value_to_schema(val));
                    required_fields.push(key.clone());
                }

                SchemaGenerator::create_object_schema_with_required(properties, required_fields)
            }
        }
    }
}

#[derive(Debug)]
pub enum JsonSchemaError {
    SerdeError(String),
    UnsupportedType { type_name: String, reason: String },
    SerializationFailed { context: String, source: String },
}

impl std::fmt::Display for JsonSchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonSchemaError::SerdeError(msg) => write!(f, "Serde error: {}", msg),
            JsonSchemaError::UnsupportedType { type_name, reason } => {
                write!(f, "Unsupported type '{}': {}", type_name, reason)
            }
            JsonSchemaError::SerializationFailed { context, source } => {
                write!(f, "Serialization failed in {}: {}", context, source)
            }
        }
    }
}

impl std::error::Error for JsonSchemaError {}

/// Trait for types that can generate their own JSON Schema
pub trait JsonSchema {
    /// Generate a JSON Schema for this type
    fn json_schema() -> Value;
}

// Implementations for primitive types
impl JsonSchema for String {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "string"
        })
    }
}

impl JsonSchema for &str {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "string"
        })
    }
}

impl JsonSchema for i8 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "integer"
        })
    }
}

impl JsonSchema for i16 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "integer"
        })
    }
}

impl JsonSchema for i32 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "integer"
        })
    }
}

impl JsonSchema for i64 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "integer"
        })
    }
}

impl JsonSchema for u8 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "integer"
        })
    }
}

impl JsonSchema for u16 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "integer"
        })
    }
}

impl JsonSchema for u32 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "integer"
        })
    }
}

impl JsonSchema for u64 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "integer"
        })
    }
}

impl JsonSchema for f32 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "number"
        })
    }
}

impl JsonSchema for f64 {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "number"
        })
    }
}

impl JsonSchema for bool {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "boolean"
        })
    }
}

impl<T: JsonSchema> JsonSchema for Vec<T> {
    fn json_schema() -> Value {
        serde_json::json!({
            "type": "array",
            "items": T::json_schema()
        })
    }
}

impl<T: JsonSchema> JsonSchema for Option<T> {
    fn json_schema() -> Value {
        serde_json::json!({
            "oneOf": [
                {"type": "null"},
                T::json_schema()
            ]
        })
    }
}

impl serde::ser::Error for JsonSchemaError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        JsonSchemaError::SerdeError(msg.to_string())
    }
}

struct SchemaGenerator {
    current_schema: Value,
}

impl SchemaGenerator {
    fn new() -> Self {
        SchemaGenerator {
            current_schema: Value::Null,
        }
    }

    fn into_schema(self) -> Value {
        self.current_schema
    }

    /// Creates a typed schema with just a type field
    fn create_typed_schema(type_name: &str) -> Value {
        let mut schema = Map::new();
        schema.insert(TYPE_KEY.to_string(), Value::String(type_name.to_string()));
        Value::Object(schema)
    }

    /// Creates an object schema with properties and required fields
    fn create_object_schema(properties: Map<String, Value>) -> Value {
        Self::create_object_schema_with_required(
            properties.clone(),
            properties.keys().cloned().collect(),
        )
    }

    /// Creates an object schema with explicit required fields
    fn create_object_schema_with_required(
        properties: Map<String, Value>,
        required_fields: Vec<String>,
    ) -> Value {
        let mut schema = Map::new();
        schema.insert(TYPE_KEY.to_string(), Value::String(TYPE_OBJECT.to_string()));
        let required: Vec<Value> = required_fields.into_iter().map(Value::String).collect();
        schema.insert(PROPERTIES_KEY.to_string(), Value::Object(properties));
        schema.insert(REQUIRED_KEY.to_string(), Value::Array(required));
        Value::Object(schema)
    }

    fn create_array_schema(item_schema: Value) -> Value {
        let mut schema = Map::new();
        schema.insert(TYPE_KEY.to_string(), Value::String(TYPE_ARRAY.to_string()));
        schema.insert(ITEMS_KEY.to_string(), item_schema);
        Value::Object(schema)
    }

    fn create_primitive_schema(type_name: &str) -> Value {
        Self::create_typed_schema(type_name)
    }

    fn create_one_of_schema(schemas: Vec<Value>) -> Value {
        let mut schema = Map::new();
        schema.insert(ONE_OF_KEY.to_string(), Value::Array(schemas));
        Value::Object(schema)
    }

    fn create_enum_schema(variants: Vec<String>) -> Value {
        let mut schema = Map::new();
        schema.insert(TYPE_KEY.to_string(), Value::String(TYPE_STRING.to_string()));
        schema.insert(
            ENUM_KEY.to_string(),
            Value::Array(variants.into_iter().map(Value::String).collect()),
        );
        Value::Object(schema)
    }
}

// Macro to reduce duplication in integer serializers
macro_rules! serialize_integer_methods {
    ($($name:ident, $type:ty),+ $(,)?) => {
        $(
            fn $name(self, _v: $type) -> Result<Self::Ok, Self::Error> {
                self.current_schema = SchemaGenerator::create_primitive_schema(TYPE_INTEGER);
                Ok(())
            }
        )+
    };
}

impl<'a> Serializer for &'a mut SchemaGenerator {
    type Ok = ();
    type Error = JsonSchemaError;

    type SerializeSeq = SeqSchemaBuilder<'a>;
    type SerializeTuple = TupleSchemaBuilder<'a>;
    type SerializeTupleStruct = TupleStructSchemaBuilder<'a>;
    type SerializeTupleVariant = TupleVariantSchemaBuilder<'a>;
    type SerializeMap = MapSchemaBuilder<'a>;
    type SerializeStruct = StructSchemaBuilder<'a>;
    type SerializeStructVariant = StructVariantSchemaBuilder<'a>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema(TYPE_BOOLEAN);
        Ok(())
    }

    serialize_integer_methods!(
        serialize_i8,
        i8,
        serialize_i16,
        i16,
        serialize_i32,
        i32,
        serialize_i64,
        i64,
        serialize_u8,
        u8,
        serialize_u16,
        u16,
        serialize_u32,
        u32,
        serialize_u64,
        u64,
    );

    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
        Err(JsonSchemaError::UnsupportedType {
            type_name: "i128".to_string(),
            reason: "type is not supported for JSON schema generation".to_string(),
        })
    }

    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        Err(JsonSchemaError::UnsupportedType {
            type_name: "u128".to_string(),
            reason: "type is not supported for JSON schema generation".to_string(),
        })
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema(TYPE_NUMBER);
        Ok(())
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema(TYPE_NUMBER);
        Ok(())
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema(TYPE_STRING);
        Ok(())
    }

    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema(TYPE_STRING);
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(JsonSchemaError::UnsupportedType {
            type_name: "&[u8]".to_string(),
            reason: "byte arrays are not supported for JSON schema generation".to_string(),
        })
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_typed_schema(TYPE_NULL);
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_typed_schema(TYPE_NULL);
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_typed_schema(TYPE_NULL);
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_enum_schema(vec![variant.to_string()]);
        Ok(())
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let mut sub_generator = SchemaGenerator::new();
        value.serialize(&mut sub_generator)?;
        let value_schema = sub_generator.into_schema();

        let mut properties = Map::new();
        properties.insert(variant.to_string(), value_schema);

        self.current_schema = SchemaGenerator::create_object_schema_with_required(
            properties,
            vec![variant.to_string()],
        );
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSchemaBuilder {
            generator: self,
            item_schema: None,
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TupleSchemaBuilder {
            generator: self,
            schemas: Vec::new(),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TupleStructSchemaBuilder {
            generator: self,
            schemas: Vec::new(),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSchemaBuilder {
            generator: self,
            schemas: Vec::new(),
            variant_name: variant.to_string(),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSchemaBuilder {
            generator: self,
            properties: Map::new(),
            current_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSchemaBuilder {
            generator: self,
            properties: Map::new(),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSchemaBuilder {
            generator: self,
            properties: Map::new(),
            variant_name: variant.to_string(),
        })
    }
}

struct SeqSchemaBuilder<'a> {
    generator: &'a mut SchemaGenerator,
    item_schema: Option<Value>,
}

impl SerializeSeq for SeqSchemaBuilder<'_> {
    type Ok = ();
    type Error = JsonSchemaError;

    fn serialize_element<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if self.item_schema.is_none() {
            let mut sub_generator = SchemaGenerator::new();
            _value.serialize(&mut sub_generator)?;
            self.item_schema = Some(sub_generator.into_schema());
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let item_schema = self
            .item_schema
            .unwrap_or_else(|| SchemaGenerator::create_primitive_schema(TYPE_NULL));
        self.generator.current_schema = SchemaGenerator::create_array_schema(item_schema);
        Ok(())
    }
}

struct TupleSchemaBuilder<'a> {
    generator: &'a mut SchemaGenerator,
    schemas: Vec<Value>,
}

impl SerializeTuple for TupleSchemaBuilder<'_> {
    type Ok = ();
    type Error = JsonSchemaError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let mut sub_generator = SchemaGenerator::new();
        value.serialize(&mut sub_generator)?;
        self.schemas.push(sub_generator.into_schema());
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.generator.current_schema =
            SchemaGenerator::create_array_schema(Value::Array(self.schemas));
        Ok(())
    }
}

struct TupleStructSchemaBuilder<'a> {
    generator: &'a mut SchemaGenerator,
    schemas: Vec<Value>,
}

impl SerializeTupleStruct for TupleStructSchemaBuilder<'_> {
    type Ok = ();
    type Error = JsonSchemaError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let mut sub_generator = SchemaGenerator::new();
        value.serialize(&mut sub_generator)?;
        self.schemas.push(sub_generator.into_schema());
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.generator.current_schema =
            SchemaGenerator::create_array_schema(Value::Array(self.schemas));
        Ok(())
    }
}

struct TupleVariantSchemaBuilder<'a> {
    generator: &'a mut SchemaGenerator,
    schemas: Vec<Value>,
    variant_name: String,
}

impl SerializeTupleVariant for TupleVariantSchemaBuilder<'_> {
    type Ok = ();
    type Error = JsonSchemaError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let mut sub_generator = SchemaGenerator::new();
        value.serialize(&mut sub_generator)?;
        self.schemas.push(sub_generator.into_schema());
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let tuple_schema = if self.schemas.len() == 1 {
            self.schemas.into_iter().next().unwrap()
        } else {
            SchemaGenerator::create_array_schema(Value::Array(self.schemas))
        };

        let mut properties = Map::new();
        properties.insert(self.variant_name.clone(), tuple_schema);

        self.generator.current_schema = SchemaGenerator::create_object_schema_with_required(
            properties,
            vec![self.variant_name],
        );
        Ok(())
    }
}

struct MapSchemaBuilder<'a> {
    generator: &'a mut SchemaGenerator,
    properties: Map<String, Value>,
    current_key: Option<String>,
}

impl SerializeMap for MapSchemaBuilder<'_> {
    type Ok = ();
    type Error = JsonSchemaError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let mut key_generator = SchemaGenerator::new();
        key.serialize(&mut key_generator)?;
        match key_generator.into_schema() {
            Value::String(key_str) => {
                self.current_key = Some(key_str);
            }
            _ => {
                return Err(JsonSchemaError::UnsupportedType {
                    type_name: "non-string key".to_string(),
                    reason: "JSON objects can only have string keys".to_string(),
                });
            }
        }
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if let Some(key) = self.current_key.take() {
            let mut value_generator = SchemaGenerator::new();
            value.serialize(&mut value_generator)?;
            self.properties.insert(key, value_generator.into_schema());
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.generator.current_schema = SchemaGenerator::create_object_schema(self.properties);
        Ok(())
    }
}

struct StructSchemaBuilder<'a> {
    generator: &'a mut SchemaGenerator,
    properties: Map<String, Value>,
}

impl SerializeStruct for StructSchemaBuilder<'_> {
    type Ok = ();
    type Error = JsonSchemaError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let mut field_generator = SchemaGenerator::new();
        value.serialize(&mut field_generator)?;
        self.properties
            .insert(key.to_string(), field_generator.into_schema());
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.generator.current_schema = SchemaGenerator::create_object_schema(self.properties);
        Ok(())
    }
}

struct StructVariantSchemaBuilder<'a> {
    generator: &'a mut SchemaGenerator,
    properties: Map<String, Value>,
    variant_name: String,
}

impl SerializeStructVariant for StructVariantSchemaBuilder<'_> {
    type Ok = ();
    type Error = JsonSchemaError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let mut field_generator = SchemaGenerator::new();
        value.serialize(&mut field_generator)?;
        self.properties
            .insert(key.to_string(), field_generator.into_schema());
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let struct_schema = SchemaGenerator::create_object_schema(self.properties);

        let mut properties = Map::new();
        properties.insert(self.variant_name.clone(), struct_schema);

        self.generator.current_schema = SchemaGenerator::create_object_schema_with_required(
            properties,
            vec![self.variant_name],
        );
        Ok(())
    }
}
