mod component;
mod data_operations;
mod data_store;
mod entity;
mod json_schema;
mod log_entry;
mod test_utils;
mod validate;

// CLI utility modules
pub mod cli_utils;
pub mod component_utils;
pub mod http_utils;

pub use component::{Component, ComponentDefinition, create_component_router};
pub use data_operations::{DataStoreOperations, OperationResult};
pub use data_store::{ComponentList, DataStore, DataStoreError, InMemoryDataStore};
pub use entity::{
    CreateEntityRequest, CreateEntityResponse, Entity, EntityParseError, create_entity_router,
};
pub use json_schema::JsonSchema;
pub use log_entry::{
    DurableLogger, LogEntry, LogMetadata, LogOperation, OperationStatus, ReplayResult,
    ValidationResult, ValidationType,
};
pub use validate::{ValidationError, validate_value};

///////////////////////////////////////// generate_id_serde ////////////////////////////////////////

#[macro_export]
macro_rules! generate_id_serde {
    ($name:ident, $visitor:ident) => {
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let s = self.to_string();
                serializer.serialize_str(&s)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<$name, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_str($visitor)
            }
        }

        struct $visitor;

        impl<'de> serde::de::Visitor<'de> for $visitor {
            type Value = $name;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an ID")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                $name::from_human_readable(value).ok_or_else(|| E::custom("not a valid tx:UUID"))
            }
        }
    };
}
