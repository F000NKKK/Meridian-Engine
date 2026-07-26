//! `#[dsl_tag(name = "...")]`: attach to a plain struct with named
//! fields, get an `impl meridian_dsl_core::DslTag` for it — the
//! mechanism a game developer uses to add their *own* tag to the scene
//! DSL, per the explicit design goal (see `meridian-sdk::dsl`'s module
//! doc): no fixed schema anywhere in this workspace lists tag names,
//! only whatever structs each crate happens to annotate.
//!
//! Every field becomes a required attribute parsed via `FromStr`
//! (`f32`, `bool`, `String`, `u32`, ... anything `.parse()`-able), except
//! `Option<T>` fields, which are optional (`None` when the attribute is
//! absent, `Some` parsed via `T::from_str` when present). No per-field
//! attributes, no renaming, no defaults beyond `Option`'s `None` — kept
//! deliberately small; a tag whose parsing doesn't fit this shape
//! implements `DslTag` by hand instead (the trait is public exactly so
//! that's always an escape hatch, not a dead end).
//!
//! **Path assumption:** the generated code refers to
//! `meridian_sdk::dsl_core::{DslTag, TagParseError}` — this macro is
//! only meant to be used via `meridian_sdk::dsl_tag` (re-exported
//! there), matching this workspace's rule that applications reach
//! everything through `meridian-sdk` alone; it isn't meant to be usable
//! from a crate that doesn't depend on `meridian-sdk`.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// The path prefix for reaching `meridian_sdk::dsl_core` from wherever
/// this macro is expanded: `crate::dsl_core` when expanding *inside*
/// `meridian-sdk` itself (its own built-in tags, e.g. this crate's
/// `dsl::Entity`/`dsl::Mesh` — `meridian_sdk` isn't a name a crate can
/// use to refer to itself), `::meridian_sdk::dsl_core` everywhere else
/// (a game crate's own `#[dsl_tag]`-annotated struct, which reaches
/// `meridian-sdk` only as an ordinary dependency).
fn sdk_path() -> proc_macro2::TokenStream {
    match crate_name("meridian-sdk") {
        Ok(FoundCrate::Itself) => quote! { crate },
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote! { ::#ident }
        }
        Err(_) => quote! { ::meridian_sdk },
    }
}

/// See the module doc comment for the full contract.
#[proc_macro_attribute]
pub fn dsl_tag(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = syn::parse_macro_input!(attr as syn::MetaNameValue);
    if !attr_args.path.is_ident("name") {
        return syn::Error::new_spanned(attr_args.path, "expected `#[dsl_tag(name = \"...\")]`")
            .to_compile_error()
            .into();
    }
    let tag_name = match &attr_args.value {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => s.value(),
        other => {
            return syn::Error::new_spanned(other, "expected a string literal, e.g. \"RigidBody\"")
                .to_compile_error()
                .into();
        }
    };

    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return syn::Error::new_spanned(
                    &input.ident,
                    "#[dsl_tag] requires a struct with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input.ident, "#[dsl_tag] only applies to structs")
                .to_compile_error()
                .into();
        }
    };

    let sdk = sdk_path();

    let mut field_inits = Vec::new();
    for field in fields {
        let field_name = field.ident.as_ref().expect("named field has an ident");
        let field_name_str = field_name.to_string();
        let is_option = is_option_type(&field.ty);

        let init = if let Some(inner_ty) = is_option {
            quote! {
                #field_name: match attrs.iter().find(|(k, _)| k == #field_name_str) {
                    Some((_, v)) => Some(v.parse::<#inner_ty>().map_err(|e| {
                        #sdk::dsl_core::TagParseError {
                            message: format!(
                                concat!(#tag_name, ".", #field_name_str, ": {}"),
                                e
                            ),
                        }
                    })?),
                    None => None,
                }
            }
        } else {
            let field_ty = &field.ty;
            quote! {
                #field_name: attrs
                    .iter()
                    .find(|(k, _)| k == #field_name_str)
                    .ok_or_else(|| #sdk::dsl_core::TagParseError {
                        message: concat!(#tag_name, ": missing attribute '", #field_name_str, "'").to_string(),
                    })?
                    .1
                    .parse::<#field_ty>()
                    .map_err(|e| #sdk::dsl_core::TagParseError {
                        message: format!(concat!(#tag_name, ".", #field_name_str, ": {}"), e),
                    })?
            }
        };
        field_inits.push(init);
    }

    let expanded = quote! {
        #input

        impl #sdk::dsl_core::DslTag for #struct_name {
            const TAG_NAME: &'static str = #tag_name;

            fn from_attrs(
                attrs: &[(String, String)],
            ) -> Result<Self, #sdk::dsl_core::TagParseError> {
                Ok(Self {
                    #(#field_inits),*
                })
            }
        }
    };
    expanded.into()
}

/// Extracts `T` from a field type written as `Option<T>` — a purely
/// syntactic check (matches the path shape, doesn't resolve type
/// aliases), the same limitation every `#[derive]`-style macro that
/// special-cases `Option` has, since proc-macros run before type
/// checking and can't see through an alias.
fn is_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        syn::GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}
