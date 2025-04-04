use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, Attribute, DataEnum, Error, Field, Fields, Meta, Token};

#[derive(Debug, Default)]
pub(crate) struct SerdeAttributes {
    pub(crate) skip: bool,
    pub(crate) flatten: bool,
    pub(crate) rename: Option<proc_macro2::TokenStream>,
    pub(crate) tag: Option<proc_macro2::TokenStream>,
}

impl SerdeAttributes {
    fn try_from_attributes(attrs: &[Attribute]) -> Result<Self, Error> {
        let mut this = Self {
            skip: false,
            flatten: false,
            rename: None,
            tag: None,
        };
        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            let meta_list = attr.meta.require_list()?;
            let meta =
                meta_list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

            for meta in meta {
                if meta.path().is_ident("skip") {
                    this.skip = true;
                }
                if meta.path().is_ident("flatten") {
                    this.flatten = true;
                }
                if meta.path().is_ident("rename") {
                    let name_value = meta.require_name_value()?;
                    this.rename = Some(name_value.value.to_token_stream());
                }
                if meta.path().is_ident("tag") {
                    let name_value = meta.require_name_value()?;
                    this.tag = Some(name_value.value.to_token_stream());
                }
            }
        }
        Ok(this)
    }
}

pub(crate) fn serde_field(field: &Field) -> Option<proc_macro2::TokenStream> {
    let serde_attrs = SerdeAttributes::try_from_attributes(&field.attrs).unwrap_or_default();
    if serde_attrs.skip {
        return Some(quote! {});
    }

    let name = field.ident.as_ref().unwrap().to_string();
    let name = match &serde_attrs.rename {
        Some(rename) => quote! { #rename },
        None => quote! { #name },
    };
    let schema = super::field_schema(field);
    let required = match super::is_option(&field.ty) {
        true => quote! {},
        false => quote! { required.push(#name.into()); },
    };

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
}

pub(crate) fn serde_data_enum<'a>(
    data: &DataEnum,
    attrs: &[Attribute],
) -> Option<proc_macro2::TokenStream> {
    let tag = SerdeAttributes::try_from_attributes(attrs)
        .unwrap_or_default()
        .tag?;
    let attributes = super::parse_attributes(attrs);

    let variants = data.variants.iter().map(|v| {
        let ident = &v.ident.to_string();
        let attributes = super::parse_attributes(&v.attrs);
        let add_field_properties = match &v.fields {
            Fields::Named(fields) => super::field_props(fields),
            Fields::Unit => quote! { (Vec::new(), serde_json::Map::new()) },
            Fields::Unnamed(_) => Error::new_spanned(&v.ident, "Unnamed emum not with tags")
                .to_compile_error(),
        };

        quote! {{
            let mut map = serde_json::Map::new();
            map.insert("type".into(), "object".into());

            let (mut required, mut properties) = #add_field_properties;

            properties.insert(#tag.into(), serde_json::json!({ "type": "string", "const": #ident }));
            required.push(#tag.into());

            map.insert("properties".into(), serde_json::Value::Object(properties));
            map.insert("required".into(), serde_json::Value::Array(required));

            #( map.insert(#attributes); )*
            serde_json::Value::Object(map)
        }}
    });

    Some(quote! {{
        let mut map = serde_json::Map::new();
        let mut one_of: Vec<serde_json::Value> = Vec::new();
        #( one_of.push(#variants); )*
        map.insert("oneOf".into(), serde_json::Value::Array(one_of));
        #( map.insert(#attributes); )*
        serde_json::Value::Object(map)
    }})
}
