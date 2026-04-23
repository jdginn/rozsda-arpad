use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields};

#[proc_macro_derive(EnumFrom, attributes(enum_from))]
pub fn enum_from_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let enum_ident = input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let Data::Enum(data_enum) = input.data else {
        return syn::Error::new_spanned(enum_ident, "EnumFrom can only be derived for enums")
            .to_compile_error()
            .into();
    };

    // Helper: does this variant have #[enum_from] ?
    fn has_enum_from_attr(attrs: &[Attribute]) -> bool {
        attrs.iter().any(|a| a.path().is_ident("enum_from"))
    }

    // Policy switch: if ANY opt-in exists, only generate for opted-in variants.
    let any_opt_in = data_enum
        .variants
        .iter()
        .any(|v| has_enum_from_attr(&v.attrs));

    let mut impls = Vec::new();

    for v in data_enum.variants {
        let opted_in = has_enum_from_attr(&v.attrs);
        if any_opt_in && !opted_in {
            continue;
        }

        let variant_ident = v.ident;

        let Fields::Unnamed(fields_unnamed) = v.fields else {
            // skip unit variants and struct variants
            continue;
        };

        if fields_unnamed.unnamed.len() != 1 {
            // skip tuple variants with != 1 fields
            continue;
        }

        let inner_ty = &fields_unnamed.unnamed[0].ty;

        impls.push(quote! {
            impl #impl_generics ::core::convert::From<#inner_ty>
                for #enum_ident #ty_generics #where_clause
            {
                #[inline]
                fn from(msg: #inner_ty) -> Self {
                    Self::#variant_ident(msg)
                }
            }
        });
    }

    // (Optional) If in opt-in mode and nothing matched, you *might* want to error.
    // For now we just generate nothing, which is harmless but could be confusing.
    // If you'd prefer an error, say so and I'll add it.

    quote! { #(#impls)* }.into()
}
