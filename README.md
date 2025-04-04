# Json Schema Derive

A crate that provides a derive macro for generating JSON Schema from your types. It supports custom schema fields via the json_schema_attr macro and a limited set of serde attributes.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
json-schema-derive = { git = "https://github.com/Pingid/json-schema-derive" }
```

## Usage

### Basic Usage

```rust
use json_schema_derive::JsonSchema;

#[derive(JsonSchema)]
#[json_schema_attr(description = "Application user")]
struct User {
    #[json_schema_attr(description = "User's name", minLength = 2)]
    name: String,
    age: u32,
    tags: Vec<String>,
}

let schema = User::json_schema();
```

### Custom Attributes

Use `json_schema_attr` to add arbitrary JSON Schema metadata:

```rust
#[derive(JsonSchema)]
struct Product {
    #[json_schema_attr(description = "Product name", minLength = 3)]
    name: String,

    #[json_schema_attr(minimum = 0, maximum = 1000)]
    price: f64,

    #[json_schema_attr(values = ["new", "used", "refurbished"])]
    condition: String,
}
```

### Supported Serde Attributes

Supports these `serde` attributes for schema generation:

```rust
#[derive(JsonSchema)]
struct Example {
    #[serde(skip)]  // Field is excluded from schema
    internal: String,

    #[serde(rename = "userName")]  // Field is renamed in schema
    name: String,

    #[serde(flatten)]  // Flattens nested struct into parent
    nested: NestedStruct,
}
```

## License

MIT License
