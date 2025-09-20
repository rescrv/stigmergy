use serde::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer,
};
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct JsonSchema {
    schema: Value,
}

impl JsonSchema {
    pub fn from_type<T: Serialize + Default>() -> Result<Self, JsonSchemaError> {
        let mut generator = SchemaGenerator::new();
        let sample = T::default();
        sample.serialize(&mut generator)?;
        Ok(JsonSchema {
            schema: generator.into_schema(),
        })
    }

    pub fn from_value(value: &Value) -> Result<Self, JsonSchemaError> {
        let schema = Self::value_to_schema(value);
        Ok(JsonSchema { schema })
    }

    pub fn as_value(&self) -> &Value {
        &self.schema
    }

    pub fn to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.schema)
    }

    fn value_to_schema(value: &Value) -> Value {
        match value {
            Value::Null => {
                let mut schema = Map::new();
                schema.insert("type".to_string(), Value::String("null".to_string()));
                Value::Object(schema)
            }
            Value::Bool(_) => {
                let mut schema = Map::new();
                schema.insert("type".to_string(), Value::String("boolean".to_string()));
                Value::Object(schema)
            }
            Value::Number(n) => {
                let mut schema = Map::new();
                let type_name = if n.is_i64() || n.is_u64() {
                    "integer"
                } else {
                    "number"
                };
                schema.insert("type".to_string(), Value::String(type_name.to_string()));
                Value::Object(schema)
            }
            Value::String(_) => {
                let mut schema = Map::new();
                schema.insert("type".to_string(), Value::String("string".to_string()));
                Value::Object(schema)
            }
            Value::Array(arr) => {
                let mut schema = Map::new();
                schema.insert("type".to_string(), Value::String("array".to_string()));

                if arr.is_empty() {
                    schema.insert(
                        "items".to_string(),
                        Value::Object({
                            let mut item_schema = Map::new();
                            item_schema
                                .insert("type".to_string(), Value::String("null".to_string()));
                            item_schema
                        }),
                    );
                } else {
                    let item_schemas: Vec<Value> = arr.iter().map(Self::value_to_schema).collect();

                    if item_schemas.iter().all(|s| s == &item_schemas[0]) {
                        schema.insert("items".to_string(), item_schemas[0].clone());
                    } else {
                        schema.insert("items".to_string(), Value::Array(item_schemas));
                    }
                }
                Value::Object(schema)
            }
            Value::Object(obj) => {
                let mut schema = Map::new();
                schema.insert("type".to_string(), Value::String("object".to_string()));

                let mut properties = Map::new();
                let mut required = Vec::new();

                for (key, val) in obj {
                    properties.insert(key.clone(), Self::value_to_schema(val));
                    required.push(Value::String(key.clone()));
                }

                schema.insert("properties".to_string(), Value::Object(properties));
                schema.insert("required".to_string(), Value::Array(required));
                Value::Object(schema)
            }
        }
    }
}

#[derive(Debug)]
pub enum JsonSchemaError {
    SerdeError(String),
}

impl std::fmt::Display for JsonSchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonSchemaError::SerdeError(msg) => write!(f, "Serde error: {}", msg),
        }
    }
}

impl std::error::Error for JsonSchemaError {}

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

    fn create_object_schema(properties: Map<String, Value>) -> Value {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("object".to_string()));
        let required: Vec<Value> = properties
            .keys()
            .map(|k| Value::String(k.clone()))
            .collect();
        schema.insert("properties".to_string(), Value::Object(properties));
        schema.insert("required".to_string(), Value::Array(required));
        Value::Object(schema)
    }

    fn create_array_schema(item_schema: Value) -> Value {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("array".to_string()));
        schema.insert("items".to_string(), item_schema);
        Value::Object(schema)
    }

    fn create_primitive_schema(type_name: &str) -> Value {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String(type_name.to_string()));
        Value::Object(schema)
    }
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
        self.current_schema = SchemaGenerator::create_primitive_schema("boolean");
        Ok(())
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("integer");
        Ok(())
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("integer");
        Ok(())
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("integer");
        Ok(())
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("integer");
        Ok(())
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("integer");
        Ok(())
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("integer");
        Ok(())
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("integer");
        Ok(())
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("integer");
        Ok(())
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("number");
        Ok(())
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("number");
        Ok(())
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("string");
        Ok(())
    }

    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("string");
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.current_schema = SchemaGenerator::create_primitive_schema("string");
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.current_schema = Value::Object({
            let mut schema = Map::new();
            schema.insert("type".to_string(), Value::String("null".to_string()));
            schema
        });
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.current_schema = Value::Object({
            let mut schema = Map::new();
            schema.insert("type".to_string(), Value::String("null".to_string()));
            schema
        });
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.current_schema = Value::Object({
            let mut schema = Map::new();
            schema.insert("type".to_string(), Value::String("null".to_string()));
            schema
        });
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("string".to_string()));
        schema.insert(
            "enum".to_string(),
            Value::Array(vec![Value::String(variant.to_string())]),
        );
        self.current_schema = Value::Object(schema);
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
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
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
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSchemaBuilder {
            generator: self,
            schemas: Vec::new(),
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
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSchemaBuilder {
            generator: self,
            properties: Map::new(),
        })
    }
}

pub struct SeqSchemaBuilder<'a> {
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
            .unwrap_or_else(|| SchemaGenerator::create_primitive_schema("null"));
        self.generator.current_schema = SchemaGenerator::create_array_schema(item_schema);
        Ok(())
    }
}

pub struct TupleSchemaBuilder<'a> {
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
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("array".to_string()));
        schema.insert("items".to_string(), Value::Array(self.schemas));
        self.generator.current_schema = Value::Object(schema);
        Ok(())
    }
}

pub struct TupleStructSchemaBuilder<'a> {
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
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("array".to_string()));
        schema.insert("items".to_string(), Value::Array(self.schemas));
        self.generator.current_schema = Value::Object(schema);
        Ok(())
    }
}

pub struct TupleVariantSchemaBuilder<'a> {
    generator: &'a mut SchemaGenerator,
    schemas: Vec<Value>,
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
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("array".to_string()));
        schema.insert("items".to_string(), Value::Array(self.schemas));
        self.generator.current_schema = Value::Object(schema);
        Ok(())
    }
}

pub struct MapSchemaBuilder<'a> {
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
        if let Value::String(key_str) = key_generator.into_schema() {
            self.current_key = Some(key_str);
        } else {
            self.current_key = Some("key".to_string());
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

pub struct StructSchemaBuilder<'a> {
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

pub struct StructVariantSchemaBuilder<'a> {
    generator: &'a mut SchemaGenerator,
    properties: Map<String, Value>,
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
        self.generator.current_schema = SchemaGenerator::create_object_schema(self.properties);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize, Default)]
    struct SimpleStruct {
        name: String,
        age: u32,
        active: bool,
    }

    #[derive(Serialize, Default)]
    struct NestedStruct {
        id: i64,
        metadata: SimpleStruct,
        tags: Vec<String>,
    }

    #[derive(Serialize, Default)]
    struct OptionalFields {
        required_field: String,
        optional_field: Option<i32>,
    }

    #[derive(Serialize, Default)]
    struct TupleStruct(String, i32, bool);

    #[derive(Serialize, Default)]
    struct WithHashMap {
        data: std::collections::HashMap<String, i32>,
    }

    #[test]
    fn simple_struct_schema() {
        let schema = JsonSchema::from_type::<SimpleStruct>().unwrap();
        let expected = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" },
                "active": { "type": "boolean" }
            },
            "required": ["active", "age", "name"]
        });

        assert_eq!(*schema.as_value(), expected);
        println!("Simple struct schema: {}", schema.to_string().unwrap());
    }

    #[test]
    fn nested_struct_schema() {
        let schema = JsonSchema::from_type::<NestedStruct>().unwrap();
        let expected = serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "metadata": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "age": { "type": "integer" },
                        "active": { "type": "boolean" }
                    },
                    "required": ["active", "age", "name"]
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "null" }
                }
            },
            "required": ["id", "metadata", "tags"]
        });

        assert_eq!(*schema.as_value(), expected);
        println!("Nested struct schema: {}", schema.to_string().unwrap());
    }

    #[test]
    fn optional_fields_schema() {
        let schema = JsonSchema::from_type::<OptionalFields>().unwrap();
        let expected = serde_json::json!({
            "type": "object",
            "properties": {
                "required_field": { "type": "string" },
                "optional_field": { "type": "null" }
            },
            "required": ["optional_field", "required_field"]
        });

        assert_eq!(*schema.as_value(), expected);
        println!("Optional fields schema: {}", schema.to_string().unwrap());
    }

    #[test]
    fn tuple_struct_schema() {
        let schema = JsonSchema::from_type::<TupleStruct>().unwrap();
        let expected = serde_json::json!({
            "type": "array",
            "items": [
                { "type": "string" },
                { "type": "integer" },
                { "type": "boolean" }
            ]
        });

        assert_eq!(*schema.as_value(), expected);
        println!("Tuple struct schema: {}", schema.to_string().unwrap());
    }

    #[test]
    fn vector_schema() {
        let schema = JsonSchema::from_type::<Vec<String>>().unwrap();
        let expected = serde_json::json!({
            "type": "array",
            "items": { "type": "null" }
        });

        assert_eq!(*schema.as_value(), expected);
        println!("Vector schema: {}", schema.to_string().unwrap());
    }

    #[test]
    fn primitive_types_schema() {
        let string_schema = JsonSchema::from_type::<String>().unwrap();
        let expected_string = serde_json::json!({ "type": "string" });
        assert_eq!(*string_schema.as_value(), expected_string);

        let int_schema = JsonSchema::from_type::<i32>().unwrap();
        let expected_int = serde_json::json!({ "type": "integer" });
        assert_eq!(*int_schema.as_value(), expected_int);

        let bool_schema = JsonSchema::from_type::<bool>().unwrap();
        let expected_bool = serde_json::json!({ "type": "boolean" });
        assert_eq!(*bool_schema.as_value(), expected_bool);

        let float_schema = JsonSchema::from_type::<f64>().unwrap();
        let expected_float = serde_json::json!({ "type": "number" });
        assert_eq!(*float_schema.as_value(), expected_float);

        println!("Primitive types validated");
    }

    #[test]
    fn hashmap_schema() {
        let schema = JsonSchema::from_type::<WithHashMap>().unwrap();
        let expected = serde_json::json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            "required": ["data"]
        });

        assert_eq!(*schema.as_value(), expected);
        println!("HashMap schema: {}", schema.to_string().unwrap());
    }

    #[test]
    fn from_value_primitive_types() {
        let null_value = Value::Null;
        let schema = JsonSchema::from_value(&null_value).unwrap();
        let expected = serde_json::json!({ "type": "null" });
        assert_eq!(*schema.as_value(), expected);

        let bool_value = serde_json::json!(true);
        let schema = JsonSchema::from_value(&bool_value).unwrap();
        let expected = serde_json::json!({ "type": "boolean" });
        assert_eq!(*schema.as_value(), expected);

        let string_value = serde_json::json!("hello");
        let schema = JsonSchema::from_value(&string_value).unwrap();
        let expected = serde_json::json!({ "type": "string" });
        assert_eq!(*schema.as_value(), expected);

        let integer_value = serde_json::json!(42);
        let schema = JsonSchema::from_value(&integer_value).unwrap();
        let expected = serde_json::json!({ "type": "integer" });
        assert_eq!(*schema.as_value(), expected);

        let float_value = serde_json::json!(2.5);
        let schema = JsonSchema::from_value(&float_value).unwrap();
        let expected = serde_json::json!({ "type": "number" });
        assert_eq!(*schema.as_value(), expected);
    }

    #[test]
    fn from_value_object() {
        let value = serde_json::json!({
            "name": "John",
            "age": 30,
            "active": true
        });
        let schema = JsonSchema::from_value(&value).unwrap();
        let expected = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" },
                "active": { "type": "boolean" }
            },
            "required": ["active", "age", "name"]
        });
        assert_eq!(*schema.as_value(), expected);
    }

    #[test]
    fn from_value_nested_object() {
        let value = serde_json::json!({
            "user": {
                "name": "Jane",
                "email": "jane@example.com"
            },
            "score": 95.5
        });
        let schema = JsonSchema::from_value(&value).unwrap();
        let expected = serde_json::json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "email": { "type": "string" }
                    },
                    "required": ["email", "name"]
                },
                "score": { "type": "number" }
            },
            "required": ["score", "user"]
        });
        assert_eq!(*schema.as_value(), expected);
    }

    #[test]
    fn from_value_array_homogeneous() {
        let value = serde_json::json!([1, 2, 3, 4]);
        let schema = JsonSchema::from_value(&value).unwrap();
        let expected = serde_json::json!({
            "type": "array",
            "items": { "type": "integer" }
        });
        assert_eq!(*schema.as_value(), expected);
    }

    #[test]
    fn from_value_array_heterogeneous() {
        let value = serde_json::json!([1, "hello", true]);
        let schema = JsonSchema::from_value(&value).unwrap();
        let expected = serde_json::json!({
            "type": "array",
            "items": [
                { "type": "integer" },
                { "type": "string" },
                { "type": "boolean" }
            ]
        });
        assert_eq!(*schema.as_value(), expected);
    }

    #[test]
    fn from_value_empty_array() {
        let value = serde_json::json!([]);
        let schema = JsonSchema::from_value(&value).unwrap();
        let expected = serde_json::json!({
            "type": "array",
            "items": { "type": "null" }
        });
        assert_eq!(*schema.as_value(), expected);
    }

    #[test]
    fn from_value_array_of_objects() {
        let value = serde_json::json!([
            { "name": "Alice", "age": 25 },
            { "name": "Bob", "age": 30 }
        ]);
        let schema = JsonSchema::from_value(&value).unwrap();
        let expected = serde_json::json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "age": { "type": "integer" }
                },
                "required": ["age", "name"]
            }
        });
        assert_eq!(*schema.as_value(), expected);
    }

    #[test]
    fn from_value_complex_nested() {
        let value = serde_json::json!({
            "metadata": {
                "version": "1.0",
                "tags": ["api", "json", "schema"]
            },
            "data": [
                {
                    "id": 1,
                    "items": [10, 20, 30]
                },
                {
                    "id": 2,
                    "items": [40, 50]
                }
            ]
        });
        let schema = JsonSchema::from_value(&value).unwrap();
        let expected = serde_json::json!({
            "type": "object",
            "properties": {
                "metadata": {
                    "type": "object",
                    "properties": {
                        "version": { "type": "string" },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["tags", "version"]
                },
                "data": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "integer" },
                            "items": {
                                "type": "array",
                                "items": { "type": "integer" }
                            }
                        },
                        "required": ["id", "items"]
                    }
                }
            },
            "required": ["data", "metadata"]
        });
        assert_eq!(*schema.as_value(), expected);
    }
}
