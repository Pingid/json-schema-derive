use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, punctuated::Punctuated, Attribute, Data, DeriveInput, Field, Fields,
    FieldsNamed, FieldsUnnamed, Type,
};

#[proc_macro_derive(JsonSchema, attributes(json_schema_attr, serde))]
pub fn json_schema_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let body = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields_named(fields, &input.attrs),
            Fields::Unnamed(fields) => fields_unnamed(fields, &input.attrs),
            Fields::Unit => {
                let attributes = get_schema_attributes(&input.attrs);
                quote! {{
                    let mut map = serde_json::Map::from_iter([#((#attributes),)*]);
                    map.insert("type".into(), serde_json::Value::String("null".into()));
                    serde_json::Value::Object(map)
                }}
            }
        },
        _ => {
            return syn::Error::new_spanned(&input.ident, "Only structs are supported")
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

fn field_with_attributes(field: &Field, attrs: &[Attribute]) -> proc_macro2::TokenStream {
    let ty = &field.ty;
    let attributes = get_schema_attributes(attrs);
    quote! {{
        let mut schema = <#ty>::json_schema();
        if let serde_json::Value::Object(map) = &mut schema {
            #( map.insert(#attributes); )*
        }
        schema
    }}
}

fn fields_unnamed(fields: &FieldsUnnamed, attrs: &[Attribute]) -> proc_macro2::TokenStream {
    let count = fields.unnamed.len();
    if count == 1 {
        let field = fields.unnamed.first().unwrap();
        let ty = &field.ty;
        let field_attributes = get_schema_attributes(&field.attrs);
        let attributes = get_schema_attributes(attrs);
        quote! {{
            let mut schema = <#ty>::json_schema();
            if let serde_json::Value::Object(map) = &mut schema {
                #( map.insert(#field_attributes); )*
                #( map.insert(#attributes); )*
            }
            schema
        }}
    } else {
        let attributes = get_schema_attributes(attrs);
        let items = fields
            .unnamed
            .iter()
            .map(|field| field_with_attributes(field, &field.attrs));
        quote! {{
            let mut map = serde_json::Map::new();
            map.insert("type".into(), serde_json::Value::String("array".into()));
            map.insert("minItems".into(), serde_json::Value::Number(#count.into()));
            map.insert("maxItems".into(), serde_json::Value::Number(#count.into()));
            map.insert("prefixItems".into(), serde_json::Value::Array(vec![#(#items),*]));
            map.insert("unevaluatedItems".into(), serde_json::Value::Bool(false));
            #( map.insert(#attributes); )*;
            serde_json::Value::Object(map)
        }}
    }
}

fn fields_named(fields: &FieldsNamed, attrs: &[Attribute]) -> proc_macro2::TokenStream {
    let attributes = get_schema_attributes(attrs);

    let inner = fields.named.iter().filter_map(|field| {
        let serde_attrs = SerdeFieldAttributes::from_attributes(&field.attrs).unwrap_or_default();
        if serde_attrs.skip {
            return None;
        }

        let field_ident = field.ident.as_ref().unwrap().to_string();
        let name = match &serde_attrs.rename {
            Some(rename) => quote! { #rename },
            None => quote! { #field_ident },
        };

        let required = match is_option(&field.ty) {
            true => quote! {},
            false => quote! { required.push(#name.into()); },
        };

        let schema = field_with_attributes(&field, &field.attrs);

        if serde_attrs.flatten {
            return Some(quote! {
                let schema = #schema;
                if let serde_json::Value::Object(mut inner) = schema {
                    if let Some(serde_json::Value::Array(inner_required)) = inner.remove("required") {
                        required.extend(inner_required);
                    }
                    if let Some(serde_json::Value::Object(inner_properties)) = inner.remove("properties") {
                        properties.extend(inner_properties);
                    }
                }
            });
        }

        Some(quote! {
            properties.insert(#name.into(), #schema);
            #required
        })
    });

    quote! {{
        let mut map = serde_json::Map::from_iter([#((#attributes),)*]);
        map.insert("type".into(), serde_json::Value::String("object".into()));

        let mut required: Vec<serde_json::Value> = Vec::new();
        let mut properties = serde_json::Map::new();

        #(#inner;)*

        map.insert("required".into(), serde_json::Value::Array(required));
        map.insert("properties".into(), serde_json::Value::Object(properties));
        serde_json::Value::Object(map)
    }}
}

fn get_schema_attributes<'a>(
    attrs: &'a [Attribute],
) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("json_schema_attr"))
        .flat_map(|attr| {
            let meta_list = attr.meta.require_list().ok()?;
            let pairs = meta_list
                .parse_args_with(Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated)
                .ok()?;

            Some(pairs.into_iter().filter_map(|pair| {
                let key = pair.path.get_ident()?.to_string();
                let val = &pair.value;
                Some(quote! { (#key).into(), serde_json::to_value(#val).unwrap() })
            }))
        })
        .flatten()
}

#[derive(Debug, Default)]
struct SerdeFieldAttributes {
    skip: bool,
    flatten: bool,
    rename: Option<proc_macro2::TokenStream>,
}

impl SerdeFieldAttributes {
    fn from_attributes(attrs: &[Attribute]) -> Option<Self> {
        let mut this = Self {
            skip: false,
            flatten: false,
            rename: None,
        };
        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            let meta_list = attr.meta.require_list().ok()?;
            let meta = meta_list
                .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
                .ok()?;

            for meta in meta {
                if meta.path().is_ident("skip") {
                    this.skip = true;
                }
                if meta.path().is_ident("flatten") {
                    this.flatten = true;
                }
                if meta.path().is_ident("rename") {
                    let name_value = meta.require_name_value().ok()?;
                    this.rename = Some(name_value.value.to_token_stream());
                }
            }
        }
        Some(this)
    }
}

fn is_option(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(last) = type_path.path.segments.last() {
            if last.ident == "Option" {
                return true;
            }
        }
    }
    false
}
