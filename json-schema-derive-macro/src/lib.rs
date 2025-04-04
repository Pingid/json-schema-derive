use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, punctuated::Punctuated, Attribute, Data, DataEnum, DeriveInput, Error,
    Field, Fields, FieldsNamed, FieldsUnnamed, MetaNameValue, Token, Type, Variant,
};

#[cfg(feature = "serde-compat")]
mod serde_compat;

#[proc_macro_derive(JsonSchema, attributes(json_schema, serde))]
pub fn json_schema_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let body = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => struct_named(fields, &input.attrs),
            Fields::Unnamed(fields) => struct_unnamed(fields, &input.attrs),
            Fields::Unit => struct_field_unit(&input.attrs),
        },
        Data::Enum(data) => data_enum(data, &input.attrs),
        _ => {
            return Error::new_spanned(&input.ident, "Only structs and enums are supported")
                .to_compile_error()
                .into()
        }
    };

    quote! {
        impl JsonSchema for #name {
            fn json_schema() -> serde_json::Value {
                #body
            }
        }
    }
    .into()
}

fn struct_field_unit(attrs: &[Attribute]) -> proc_macro2::TokenStream {
    let attributes = parse_attributes(attrs);
    quote! {{
        let mut map = serde_json::Map::new();
        map.insert("type".into(), serde_json::Value::String("null".into()));
        #( map.insert(#attributes); )*
        serde_json::Value::Object(map)
    }}
}

fn struct_named(fields: &FieldsNamed, attrs: &[Attribute]) -> proc_macro2::TokenStream {
    let attributes = parse_attributes(attrs);
    let generate_field_properties = field_props(fields);

    quote! {{
        let mut map = serde_json::Map::new();
        map.insert("type".into(), serde_json::Value::String("object".into()));

        let (required, properties) = #generate_field_properties;

        map.insert("required".into(), serde_json::Value::Array(required));
        map.insert("properties".into(), serde_json::Value::Object(properties));

        #( map.insert(#attributes); )*

        serde_json::Value::Object(map)
    }}
}

fn struct_unnamed(fields: &FieldsUnnamed, attrs: &[Attribute]) -> proc_macro2::TokenStream {
    let count = fields.unnamed.len();
    if count == 1 {
        let field = fields.unnamed.first().unwrap();
        let ty = &field.ty;
        let field_attributes = parse_attributes(&field.attrs);
        let attributes = parse_attributes(attrs);
        quote! {{
            let mut schema = <#ty>::json_schema();
            if let serde_json::Value::Object(map) = &mut schema {
                #( map.insert(#attributes); )*
                #( map.insert(#field_attributes); )*
            }
            schema
        }}
    } else {
        let attributes = parse_attributes(attrs);
        let items = fields.unnamed.iter().map(field_schema);
        let items_count = items.len();
        quote! {{
            let mut map = serde_json::Map::new();
            map.insert("type".into(), serde_json::Value::String("array".into()));
            map.insert("minItems".into(), serde_json::Value::Number(#count.into()));
            map.insert("maxItems".into(), serde_json::Value::Number(#count.into()));
            map.insert("unevaluatedItems".into(), serde_json::Value::Bool(false));

            let mut prefixItems = Vec::with_capacity(#items_count);
            #( prefixItems.push(#items); )*
            map.insert("prefixItems".into(), serde_json::Value::Array(prefixItems));

            #( map.insert(#attributes); )*

            serde_json::Value::Object(map)
        }}
    }
}

fn data_enum(data: &DataEnum, attrs: &[Attribute]) -> proc_macro2::TokenStream {
    #[cfg(feature = "serde-compat")]
    if let Some(s) = serde_compat::serde_data_enum(data, attrs) {
        return s;
    }

    let all_variants_unit_type = data
        .variants
        .iter()
        .all(|v| matches!(v.fields, Fields::Unit));

    match all_variants_unit_type {
        true => enum_unit(data.variants.iter(), attrs),
        false => enum_complex(data.variants.iter(), attrs),
    }
}

fn enum_unit<'a>(
    variants: impl Iterator<Item = &'a Variant>,
    attrs: &[Attribute],
) -> proc_macro2::TokenStream {
    let attributes = parse_attributes(attrs);
    let variants = variants.into_iter().map(|v| v.ident.to_string());
    quote! {{
        let mut map = serde_json::Map::new();
        map.insert("type".into(), serde_json::Value::String("string".into()));
        let mut enum_values: Vec<serde_json::Value> = Vec::new();
        #( enum_values.push(#variants.into()); )*
        map.insert("enum".into(), serde_json::Value::Array(enum_values));
        #( map.insert(#attributes); )*
        serde_json::Value::Object(map)
    }}
}

fn enum_complex<'a>(
    variants: impl Iterator<Item = &'a Variant>,
    attrs: &[Attribute],
) -> proc_macro2::TokenStream {
    let attributes = parse_attributes(attrs);
    let variants = variants.into_iter().map(|v| {
        let ident = &v.ident.to_string();
        let inner = match &v.fields {
            Fields::Named(named) => struct_named(named, &v.attrs),
            Fields::Unnamed(unnamed) => struct_unnamed(unnamed, &v.attrs),
            Fields::Unit => Error::new_spanned(&v.ident, "Unit variants are not yet supported")
                .to_compile_error(),
        };
        quote! {
            properties.insert(#ident.into(), #inner);
        }
    });
    quote! {{
        let mut map = serde_json::Map::new();
        map.insert("type".into(), serde_json::Value::String("object".into()));
        let mut properties = serde_json::Map::new();
        #(#variants;)*;
        map.insert("properties".into(), serde_json::Value::Object(properties));
        #( map.insert(#attributes); )*
        serde_json::Value::Object(map)
    }}
}

// Utilities
pub(crate) fn field_props(fields: &FieldsNamed) -> proc_macro2::TokenStream {
    let inner = fields.named.iter().map(|field| {
        #[cfg(feature = "serde-compat")]
        if let Some(serde_field) = serde_compat::serde_field(field) {
            return serde_field;
        }

        let name = field.ident.as_ref().unwrap().to_string();
        let schema = field_schema(field);
        let required = match is_option(&field.ty) {
            true => quote! {},
            false => quote! { required.push(#name.into()); },
        };

        quote! {
            let field_schema = #schema;
            properties.insert(#name.into(), field_schema);
            #required
        }
    });

    quote! {{
        let mut required: Vec<serde_json::Value> = Vec::new();
        let mut properties = serde_json::Map::new();
        #(#inner;)*
        (required, properties)
    }}
}

pub(crate) fn field_schema(field: &Field) -> proc_macro2::TokenStream {
    let ty = &field.ty;
    let attributes = parse_attributes(&field.attrs);
    quote! {{
        let mut schema = <#ty>::json_schema();
        if let serde_json::Value::Object(map) = &mut schema {
            #( map.insert(#attributes); )*
        }
        schema
    }}
}

pub(crate) fn parse_attributes(
    attrs: &[Attribute],
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                let meta_list = attr.meta.require_name_value().ok()?;
                let val = &meta_list.value;
                return Some(vec![
                    quote! { "description".into(), serde_json::to_value(#val.trim()).unwrap() },
                ]);
            }
            if attr.path().is_ident("json_schema") {
                let meta_list = attr.meta.require_list().ok()?;
                let pairs = meta_list
                    .parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
                    .ok()?;
                return Some(
                    pairs
                        .into_iter()
                        .filter_map(|pair| {
                            let key = pair.path.get_ident()?.to_string();
                            let val = &pair.value;
                            Some(quote! { (#key).into(), serde_json::to_value(#val).unwrap() })
                        })
                        .collect(),
                );
            }
            None
        })
        .flatten()
}

pub(crate) fn is_option(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(last) = type_path.path.segments.last() {
            if last.ident == "Option" {
                return true;
            }
        }
    }
    false
}
