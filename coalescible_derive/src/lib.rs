use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericParam, Generics, WherePredicate};

#[proc_macro_derive(Coalescible, attributes(data, nocoalesce))]
pub fn derive_coalescible(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_coalescible(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_coalescible(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_ident = input.ident;
    let vis = input.vis;
    let generics = input.generics;

    let struct_has_nocoalesce = input.attrs.iter().any(|a| a.path().is_ident("nocoalesce"));

    let data = match input.data {
        Data::Struct(s) => s,
        _ => {
            return Err(syn::Error::new(
                struct_ident.span(),
                "Coalescible can only be derived for structs",
            ))
        }
    };

    let named_fields = match data.fields {
        Fields::Named(f) => f.named,
        Fields::Unit => syn::punctuated::Punctuated::new(),
        _ => {
            return Err(syn::Error::new(
                struct_ident.span(),
                "Coalescible requires named fields or a unit struct",
            ))
        }
    };

    let mut key_fields = Vec::new(); // (ident, ty)
    let mut data_fields = Vec::new(); // (ident, ty)

    for f in named_fields.iter() {
        let ident = f.ident.clone().expect("named fields guaranteed");
        let ty = f.ty.clone();

        let is_data = f.attrs.iter().any(|a| a.path().is_ident("data"));
        if is_data {
            data_fields.push((ident, ty));
        } else {
            key_fields.push((ident, ty));
        }
    }

    let has_key_fields = !key_fields.is_empty();
    let should_emit_some_key = has_key_fields && !struct_has_nocoalesce;

    // Add bounds so the message type can be cloned if generic params are present.
    let mut impl_generics = generics.clone();
    add_clone_bound(&mut impl_generics);

    let (impl_g, _impl_ty_g, impl_where_clause) = impl_generics.split_for_impl();
    let (_, orig_ty_g, _) = generics.split_for_impl();

    let key_ident = format_ident!("{}Key", struct_ident);
    let key_generics = only_type_and_lifetime_generics(&generics);
    let (_, key_ty_g, key_where) = key_generics.split_for_impl();

    // Generate key struct only when we're actually using a key.
    let key_struct_tokens = if should_emit_some_key {
        let key_field_defs = key_fields.iter().map(|(id, ty)| quote! { pub #id: #ty });

        quote! {
            #[derive(Clone, Debug, PartialEq, Eq, Hash)]
            #vis struct #key_ident #key_ty_g
            #key_where
            {
                #( #key_field_defs, )*
            }
        }
    } else {
        quote! {}
    };

    let key_type_tokens = if should_emit_some_key {
        quote! { #key_ident #orig_ty_g }
    } else {
        quote! { () }
    };

    let key_fn_tokens = if should_emit_some_key {
        let key_field_inits = key_fields
            .iter()
            .map(|(id, _)| quote! { #id: self.#id.clone() });

        quote! {
            fn key(&self) -> Option<Self::Key> {
                Some(#key_ident {
                    #( #key_field_inits, )*
                })
            }
        }
    } else {
        quote! {
            fn key(&self) -> Option<Self::Key> { None }
        }
    };

    let merge_assigns = data_fields.iter().map(|(id, _)| {
        quote! { self.#id = newer.#id; }
    });

    // Build a single combined where-predicate list.
    let mut all_where_preds: Vec<WherePredicate> = Vec::new();

    if let Some(wc) = impl_where_clause {
        all_where_preds.extend(wc.predicates.iter().cloned());
    }

    if should_emit_some_key {
        for (_, ty) in &key_fields {
            all_where_preds.push(
                syn::parse_quote!(#ty: ::core::clone::Clone + ::core::cmp::Eq + ::core::hash::Hash),
            );
        }
    }

    let coalescible_path = quote! { crate::modes::coalesce::Coalescible };

    Ok(quote! {
        #key_struct_tokens

        impl #impl_g #coalescible_path for #struct_ident #orig_ty_g
        where
            #( #all_where_preds, )*
        {
            type Key = #key_type_tokens;

            #key_fn_tokens

            fn merge_from(&mut self, newer: Self) {
                #( #merge_assigns )*
            }
        }
    })
}

fn add_clone_bound(generics: &mut Generics) {
    for param in generics.params.iter_mut() {
        if let GenericParam::Type(ty) = param {
            ty.bounds.push(syn::parse_quote!(::core::clone::Clone));
        }
    }
}

fn only_type_and_lifetime_generics(g: &Generics) -> Generics {
    let mut out = g.clone();
    out.params = out
        .params
        .into_iter()
        .filter(|p| matches!(p, GenericParam::Type(_) | GenericParam::Lifetime(_)))
        .collect();
    out
}
