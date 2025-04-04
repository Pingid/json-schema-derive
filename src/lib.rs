//! Derive macro for generating JSON Schema from Rust types.
//!
//! This crate provides a `#[derive(JsonSchema)]` macro that generates a JSON Schema
//! for your types. It supports custom schema attributes via `#[json_schema(...)]`
//! and optionally integrates with common `serde` attributes when the `serde-compat` feature is enabled.
//!
//! # Example
//! ```rust
//! use json_schema_derive::JsonSchema;
//!
//! #[derive(JsonSchema)]
//! struct User {
//!     #[json_schema(comment = "User's name", minLength = 2)]
//!     name: String,
//!     /// User's age
//!     age: u32,
//!     tags: Vec<String>,
//! }
//!
//! let schema = User::json_schema();
//! ```
//!
//! # Features
//!
//! - `serde-compat`: Enables compatibility with serde attributes for schema generation
//! # Serde Compatibility
//!
//! When the `serde-compat` feature is enabled, the following `serde` attributes are supported:
//!
//! - `#[serde(skip)]` – Omits the field from the schema  
//! - `#[serde(rename = "new_name")]` – Renames the field in the schema  
//! - `#[serde(flatten)]` – Inlines nested struct fields  
//! - `#[serde(tag = "...")]` – Supports internally tagged enums
//!
//! ```rust
//! #[derive(JsonSchema)]
//! #[serde(tag = "type")]
//! enum Event {
//!     Login { user: String },
//!     Logout,
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

impl JsonSchema for () {
    fn json_schema() -> serde_json::Value {
        serde_json::json!({ "type": "null" })
    }
}

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

    pub fn valid<T: JsonSchema + Serialize>(instance: &T) -> bool {
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
    #[json_schema(comment = "Test comment")]
    #[allow(dead_code)]
    struct TestStruct {
        #[json_schema(comment = "test field", minLength = 3)]
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
                    "comment": "test field",
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
            "comment": "Test comment"
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
                            "comment": "test field",
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
                    "comment": "Test comment"
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
    #[json_schema(comment = "Test comment")]
    #[allow(dead_code)]
    struct TestStructUnnamed(String);

    #[test]
    fn test_struct_unnamed() {
        let schema = TestStructUnnamed::json_schema();
        let expected = json!({ "comment": "Test comment", "type": "string" });
        assert_eq!(schema, expected);
        assert!(valid(&TestStructUnnamed("test".to_string())));
    }

    #[derive(JsonSchema, Serialize)]
    #[json_schema(comment = "Test comment")]
    #[allow(dead_code)]
    struct TestStructUnnamedMultiple(String, u32);

    #[test]
    fn test_struct_unnamed_multiple() {
        let schema = TestStructUnnamedMultiple::json_schema();
        let expected = json!({
            "comment": "Test comment",
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
    #[json_schema(comment = "Test comment")]
    #[allow(dead_code)]
    enum EnumUnit {
        A,
        B,
        C,
    }

    #[test]
    fn test_enum_unit() {
        let schema = EnumUnit::json_schema();
        let expected = json!({
            "type": "string",
            "comment": "Test comment",
            "enum": ["A", "B", "C"],
        });
        println!("{:#?}", serde_json::to_value(&EnumUnit::A).unwrap());
        assert_eq!(schema, expected);
        assert!(valid(&EnumUnit::A));
        assert!(valid(&EnumUnit::B));
        assert!(valid(&EnumUnit::C));
    }

    #[derive(JsonSchema, Serialize)]
    #[json_schema(comment = "Test comment")]
    #[allow(dead_code)]
    enum EnumUnnamed {
        A(String),
        B(u32),
    }

    #[test]
    fn test_enum_unit_unnamed() {
        let schema = EnumUnnamed::json_schema();
        let expected = json!({
            "type": "object",
            "comment": "Test comment",
            "properties": {
                "A": { "type": "string" },
                "B": { "type": "number" },
            }
        });
        assert_eq!(schema, expected);
        assert!(valid(&EnumUnnamed::A("test".to_string())));
        assert!(valid(&EnumUnnamed::B(10)));
    }

    #[derive(JsonSchema, Serialize)]
    #[json_schema(comment = "Test comment")]
    #[allow(dead_code)]
    enum EnumNamed {
        A { name: String },
        B { age: u32 },
    }

    #[test]
    fn test_enum_named() {
        let schema = EnumNamed::json_schema();
        let expected = json!({
            "type": "object",
            "comment": "Test comment",
            "properties": {
                "A": { "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] },
                "B": { "type": "object", "properties": { "age": { "type": "number" } }, "required": ["age"] },
            }
        });
        assert_eq!(schema, expected);
        assert!(valid(&EnumNamed::A {
            name: "test".to_string()
        }));
        assert!(valid(&EnumNamed::B { age: 10 }));
    }

    #[derive(JsonSchema, Serialize)]
    #[allow(dead_code)]
    /// Test description
    struct TestStructDoc {
        /// Test field description
        name: String,
    }

    #[test]
    fn test_struct_doc() {
        let schema = TestStructDoc::json_schema();
        let expected = json!({ "type": "object", "description": "Test description", "properties": { "name": { "type": "string", "description": "Test field description" } }, "required": ["name"] });
        assert_eq!(schema, expected);
        assert!(valid(&TestStructDoc {
            name: "test".to_string()
        }));
    }
}

#[cfg(feature = "serde-compat")]
#[cfg(test)]
mod tests_serde_compat {
    use super::*;
    use serde::Serialize;
    use serde_json::json;

    #[derive(JsonSchema, Serialize)]
    #[json_schema(comment = "Test comment")]
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
            "comment": "Test comment"
        });
        assert_eq!(schema, expected);
        assert!(tests::valid(&TestStructWithSerde {
            skip: 0,
            renamed: 10,
        }));
    }

    #[derive(JsonSchema, Serialize)]
    #[json_schema(comment = "Test comment")]
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
            "comment": "Test comment"
        });
        println!("{:#?}", schema);
        assert_eq!(schema, expected);
        assert!(tests::valid(&TestStructWithFlatten {
            inner: TestStructWithSerde {
                skip: 0,
                renamed: 10,
            }
        }));
    }

    #[derive(JsonSchema, Serialize)]
    #[allow(dead_code)]
    #[serde(tag = "type")]
    enum EnumUnitSerdeTag {
        A,
        B,
    }

    #[test]
    fn test_enum_serde_tag() {
        let schema = EnumUnitSerdeTag::json_schema();
        let expected = json!({
            "oneOf": [
                { "type": "object", "properties": { "type": { "type": "string", "const": "A" } }, "required": ["type"] },
                { "type": "object", "properties": { "type": { "type": "string", "const": "B" } }, "required": ["type"] }
            ]
        });
        assert_eq!(schema, expected);
        assert!(tests::valid(&EnumUnitSerdeTag::A));
        assert!(tests::valid(&EnumUnitSerdeTag::B));
    }

    #[derive(JsonSchema, Serialize)]
    #[allow(dead_code)]
    #[serde(tag = "type")]
    enum EnumNamedSerdeTag {
        A { name: String },
        B { age: u32 },
        C,
    }

    #[test]
    fn test_enum_named_serde_tag() {
        let schema = EnumNamedSerdeTag::json_schema();
        let expected = json!({
            "oneOf": [
                { "type": "object", "properties": { "type": { "type": "string", "const": "A" }, "name": { "type": "string" } }, "required": ["name", "type"] },
                { "type": "object", "properties": { "type": { "type": "string", "const": "B" }, "age": { "type": "number" } }, "required": ["age", "type"] },
                { "type": "object", "properties": { "type": { "type": "string", "const": "C" } }, "required": ["type"] }
            ]
        });
        assert_eq!(schema, expected);
        assert!(tests::valid(&EnumNamedSerdeTag::A {
            name: "test".to_string()
        }));
        assert!(tests::valid(&EnumNamedSerdeTag::B { age: 10 }));
    }
}
