# Json Schema Derive

This crate provides a `#[derive(JsonSchema)]` macro that generates a JSON Schema for your types. It supports custom schema attributes via `#[json_schema(...)]` and optionally integrates with common `serde` attributes when the `serde-compat` feature is enabled.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
json-schema-derive = { version = "0.0.1", features = ["serde-compat"] }
```

## Usage

### Basic Usage

```rust
use json_schema_derive::JsonSchema;

#[derive(JsonSchema)]
#[json_schema(description = "Application user")]
struct User {
    #[json_schema(description = "User's name", minLength = 2)]
    name: String,
    age: u32,
    tags: Vec<String>,
}

let schema = User::json_schema();
```

### Custom Attributes

Use `json_schema` to add arbitrary JSON Schema metadata:

```rust
#[derive(JsonSchema)]
struct Product {
    #[json_schema(description = "Product name", minLength = 3)]
    name: String,

    #[json_schema(minimum = 0, maximum = 1000)]
    price: f64,

    #[json_schema(values = ["new", "used", "refurbished"])]
    condition: String,
}
```

### Serde Compatibility

When the `serde-compat` feature is enabled, the following `serde` attributes are supported for schema generation:

```rust
#[derive(JsonSchema, Serialize)]
struct Example {
    #[serde(skip)]  // Field is excluded from schema
    internal: String,

    #[serde(rename = "userName")]  // Field is renamed in schema
    name: String,

    #[serde(flatten)]  // Flattens nested struct into parent
    nested: NestedStruct,
}

#[derive(JsonSchema, Serialize)]
#[serde(tag = "type")]
struct Animal {
    Dog { name: String },
    Cat { name: String },
}
```

## Features

- `serde-compat`: Enables compatibility with serde attributes for schema generation

## License

MIT License
