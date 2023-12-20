//! Proc-macros for implementing execution-plan traits.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, Fields, GenericParam};

/// This will derive the trait `Value` from the `kittycad-execution-plan-traits` crate.
#[proc_macro_derive(ExecutionPlanValue)]
pub fn impl_value(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    let span = input.span();
    // Name of type that is deriving Value
    let name = input.ident;

    // Build the output, possibly using quasi-quotation
    let expanded = match input.data {
        syn::Data::Struct(data) => impl_value_on_struct(span, name, data, input.generics),
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => quote_spanned! {span =>
            compile_error!("Value cannot be implemented on a union type")
        },
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

fn impl_value_on_struct(
    span: Span,
    name: proc_macro2::Ident,
    data: syn::DataStruct,
    generics: syn::Generics,
) -> proc_macro2::TokenStream {
    let Fields::Named(ref fields) = data.fields else {
        return quote_spanned! {span =>
            compile_error!("Value cannot be implemented on a struct with unnamed fields")
        };
    };

    // We're going to construct some fragments of Rust source code, which will get used in the
    // final generated code this function returns.

    // For every field in the struct, this macro will:
    // - In the `into_parts`, extend the Vec of parts with that field, turned into parts.
    // - In the `from_parts`, instantiate a Self with a field from that part.
    // Step one is to get a list of all named fields in the struct (and their spans):
    let field_names: Vec<_> = fields
        .named
        .iter()
        .filter_map(|field| field.ident.as_ref().map(|ident| (ident, field.span())))
        .collect();
    // Now we can construct those `into_parts` and `from_parts` fragments.
    // We take some care to use the span of each `syn::Field` as
    // the span of the corresponding `into_parts()` and `from_parts()`
    // calls. This way if one of the field types does not
    // implement `Value` then the compiler's error message
    // underlines which field it is.
    let extend_per_field = field_names.iter().map(|(ident, span)| {
        quote_spanned! {*span=>
            parts.extend(self.#ident.into_parts());
        }
    });
    let instantiate_each_field = field_names.iter().map(|(ident, span)| {
        quote_spanned! {*span=>
            #ident: kittycad_execution_plan_traits::Value::from_parts(values)?,
        }
    });

    // Handle generics in the original struct.
    // Firstly, if the original struct has defaults on its generics, e.g. Point2d<T = f32>,
    // don't include those defaults in this macro's output, because the compiler
    // complains it's unnecessary and will soon be a compile error.
    let mut generics_without_defaults = generics.clone();
    for generic_param in generics_without_defaults.params.iter_mut() {
        if let GenericParam::Type(type_param) = generic_param {
            type_param.default = None;
        }
    }
    let where_clause = generics.where_clause;

    // Final return value: the generated Rust code to implement the trait.
    // This uses the fragments above, interpolating them into the final outputted code.
    quote! {
        impl #generics_without_defaults kittycad_execution_plan_traits::Value for #name #generics_without_defaults
        #where_clause
        {
            fn into_parts(self) -> Vec<kittycad_execution_plan_traits::Primitive> {
                let mut parts = Vec::new();
                #(#extend_per_field)*
                parts
            }

            fn from_parts<I>(values: &mut I) -> Result<Self, kittycad_execution_plan_traits::MemoryError>
            where
                I: Iterator<Item = Option<kittycad_execution_plan_traits::Primitive>>,
            {
                Ok(Self {
                #(#instantiate_each_field)*
                })
            }
        }
    }
}
