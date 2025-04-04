//! JSON Schema generation with support for arbitrary extra fields.
//!
//! This crate provides a derive macro for generating JSON Schema from Rust types.
//! It supports arbitrary JSON Schema attributes via the `json_schema_attr` attribute.
//!
//! # Example
//!
//! ```rust
//! use json_schema_derive::JsonSchema;
//!
//! #[derive(JsonSchema)]
//! struct User {
//!     #[json_schema_attr(description = "User's name", minLength = 2)]
//!     name: String,
//!     age: u32,
//!     tags: Vec<String>,
//! }
//!
//! let schema = User::json_schema();
//! ```
//!
//! ## Supported Serde Attributes
//!
//! The crate supports the following serde attributes:
//!
//! - `#[serde(skip)]` - Excludes a field from the generated schema
//! - `#[serde(rename = "name")]` - Renames a field in the generated schema
//! - `#[serde(flatten)]` - Flattens a nested struct into its parent in the schema
//!
//! ```rust
//! use json_schema_derive::JsonSchema;
//!
//! #[derive(JsonSchema)]
//! struct Example {
//!     #[serde(skip)]
//!     internal: String,
//!
//!     #[serde(rename = "userName")]
//!     name: String,
//!
//!     #[serde(flatten)]
//!     nested: NestedStruct,
//! }
//! ```

use core::str;

pub use json_schema_derive_macro::JsonSchema;
// mod expanded;

/// Trait for generating JSON Schema from a type.
///
/// This trait is automatically implemented for types that derive `JsonSchema`.
/// It provides a method to generate a JSON Schema representation of the type.
pub trait JsonSchema {
    /// Generate a JSON Schema representation of the type.
    ///
    /// Returns a `serde_json::Value` containing the JSON Schema.
    fn json_schema() -> serde_json::Value;
}

macro_rules! impl_json_schema {
    ($name:expr, $($t:ty),*) => {
        $(
            impl JsonSchema for $t {
                fn json_schema() -> serde_json::Value {
                    serde_json::json!({ "type": $name })
                }
            }
        )*
    };
}

impl_json_schema!("number", u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
impl_json_schema!("boolean", bool);
impl_json_schema!("string", String, &str);

impl<T: JsonSchema> JsonSchema for Vec<T> {
    fn json_schema() -> serde_json::Value {
        serde_json::json!({ "type": "array", "items": T::json_schema() })
    }
}

impl<T: JsonSchema, const N: usize> JsonSchema for [T; N] {
    fn json_schema() -> serde_json::Value {
        serde_json::json!({ "type": "array", "items": T::json_schema(), "maxItems": N, "minItems": N })
    }
}

impl<T: JsonSchema> JsonSchema for Option<T> {
    fn json_schema() -> serde_json::Value {
        T::json_schema()
    }
}

impl<T: JsonSchema> JsonSchema for &Option<T> {
    fn json_schema() -> serde_json::Value {
        T::json_schema()
    }
}

impl<T: JsonSchema> JsonSchema for Box<T> {
    fn json_schema() -> serde_json::Value {
        T::json_schema()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_json::json;

    fn valid<T: JsonSchema + Serialize>(instance: &T) -> bool {
        let schema = T::json_schema();
        let json = serde_json::to_value(instance).unwrap();
        jsonschema::is_valid(&schema, &json)
    }

    #[test]
    fn test_impl_json_schema() {
        assert_eq!(u32::json_schema(), json!({ "type": "number" }));
        assert_eq!(bool::json_schema(), json!({ "type": "boolean" }));
        assert_eq!(String::json_schema(), json!({ "type": "string" }));
        assert_eq!(
            <Vec<u32>>::json_schema(),
            json!({ "type": "array", "items": { "type": "number" } })
        );
        assert_eq!(<Option<bool>>::json_schema(), json!({ "type": "boolean" }));
        assert_eq!(
            <[u32; 3]>::json_schema(),
            json!({ "type": "array", "items": { "type": "number" }, "maxItems": 3, "minItems": 3 })
        );

        assert!(valid::<u32>(&10));
        assert!(valid::<bool>(&true));
        assert!(valid::<String>(&"test".to_string()));
        assert!(valid::<Vec<u32>>(&vec![1, 2, 3]));
        assert!(valid::<Option<bool>>(&Some(true)));
        assert!(valid::<[u32; 3]>(&[1, 2, 3]));
    }

    #[derive(JsonSchema, Serialize)]
    #[json_schema_attr(description = "Test description")]
    #[allow(dead_code)]
    struct TestStruct {
        #[json_schema_attr(description = "test field", minLength = 3)]
        name: String,
        age: u32,
        active: Option<bool>,
        scores: Vec<i32>,
    }

    #[test]
    fn test_struct_schema() {
        let schema = TestStruct::json_schema();
        let expected = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "test field",
                    "minLength": 3
                },
                "age": {
                    "type": "number"
                },
                "active": {
                    "type": "boolean"
                },
                "scores": {
                    "type": "array",
                    "items": {"type": "number"}
                }
            },
            "required": ["name", "age", "scores"],
            "description": "Test description"
        });
        assert_eq!(schema, expected);
        assert!(valid(&TestStruct {
            name: "test".to_string(),
            age: 10,
            active: Some(true),
            scores: vec![1, 2, 3],
        }));
    }

    #[derive(JsonSchema)]
    #[allow(dead_code)]
    struct NestedStruct {
        inner: Option<TestStruct>,
        tags: Option<Vec<String>>,
    }

    #[test]
    fn test_nested_struct() {
        let schema = NestedStruct::json_schema();
        let expected = json!({
            "type": "object",
            "properties": {
                "inner": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "test field",
                            "minLength": 3
                        },
                        "age": {
                            "type": "number"
                        },
                        "active": {
                            "type": "boolean"
                        },
                        "scores": {
                            "type": "array",
                            "items": {"type": "number"}
                        }
                    },
                    "required": ["name", "age", "scores"],
                    "description": "Test description"
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            },
            "required": []
        });
        assert_eq!(schema, expected);
    }

    #[derive(JsonSchema, Serialize)]
    #[json_schema_attr(description = "Test description")]
    #[allow(dead_code)]
    struct TestStructUnnamed(String);

    #[test]
    fn test_struct_unnamed() {
        let schema = TestStructUnnamed::json_schema();
        let expected = json!({ "description": "Test description", "type": "string" });
        assert_eq!(schema, expected);
        assert!(valid(&TestStructUnnamed("test".to_string())));
    }

    #[derive(JsonSchema, Serialize)]
    #[json_schema_attr(description = "Test description")]
    #[allow(dead_code)]
    struct TestStructUnnamedMultiple(String, u32);

    #[test]
    fn test_struct_unnamed_multiple() {
        let schema = TestStructUnnamedMultiple::json_schema();
        let expected = json!({
            "description": "Test description",
            "type": "array",
            "prefixItems": [{ "type": "string" }, { "type": "number" }],
            "minItems": 2,
            "maxItems": 2,
            "unevaluatedItems": false,
        });
        assert_eq!(schema, expected);
        assert!(valid(&TestStructUnnamedMultiple("test".to_string(), 10)));
    }

    #[derive(JsonSchema, Serialize)]
    #[json_schema_attr(description = "Test description")]
    #[allow(dead_code)]
    struct TestStructWithSerde {
        #[serde(skip)]
        skip: u32,
        #[serde(rename = "foo")]
        renamed: u32,
    }

    #[test]
    fn test_struct_with_serde() {
        let schema = TestStructWithSerde::json_schema();
        let expected = json!({
            "type": "object",
            "properties": { "foo": { "type": "number" } },
            "required": ["foo"],
            "description": "Test description"
        });
        assert_eq!(schema, expected);
        assert!(valid(&TestStructWithSerde {
            skip: 0,
            renamed: 10,
        }));
    }

    #[derive(JsonSchema, Serialize)]
    #[json_schema_attr(description = "Test description")]
    #[allow(dead_code)]
    struct TestStructWithFlatten {
        #[serde(flatten)]
        inner: TestStructWithSerde,
    }
    #[test]
    fn test_struct_with_flatten() {
        let schema = TestStructWithFlatten::json_schema();
        let expected = json!({
            "type": "object",
            "properties": { "foo": { "type": "number" } },
            "required": ["foo"],
            "description": "Test description"
        });
        println!("{:#?}", schema);
        assert_eq!(schema, expected);
        assert!(valid(&TestStructWithFlatten {
            inner: TestStructWithSerde {
                skip: 0,
                renamed: 10,
            }
        }));
    }
}
