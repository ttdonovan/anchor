use crate::parser::docs;
use crate::Program;
use syn::parse::{Error as ParseError, Result as ParseResult};
use syn::spanned::Spanned;

mod instructions;
mod state;

pub fn parse(mut program_mod: syn::ItemMod) -> ParseResult<Program> {
    let state = state::parse(&program_mod)?;
    let docs = docs::parse(&program_mod.attrs);
    let (ixs, fallback_fn) = instructions::parse(&program_mod)?;
    // strip #[remaining_accounts] from the program mod items.
    program_mod.content.iter_mut().for_each(|(_, items)| {
        for item in items.iter_mut() {
            if let syn::Item::Fn(item_fn) = item {
                item_fn.attrs.retain(|attr| {
                    match attr.parse_meta() {
                        Ok(syn::Meta::Path(path)) => !path.is_ident("remaining_accounts"),
                        _ => true,
                    }
                });
            }
        }
    });
    
    Ok(Program {
        state,
        ixs,
        name: program_mod.ident.clone(),
        docs,
        program_mod,
        fallback_fn,
    })
}

fn ctx_accounts_ident(path_ty: &syn::PatType) -> ParseResult<proc_macro2::Ident> {
    let p = match &*path_ty.ty {
        syn::Type::Path(p) => &p.path,
        _ => return Err(ParseError::new(path_ty.ty.span(), "invalid type")),
    };
    let segment = p
        .segments
        .first()
        .ok_or_else(|| ParseError::new(p.segments.span(), "expected generic arguments here"))?;

    let generic_args = match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        _ => return Err(ParseError::new(path_ty.span(), "missing accounts context")),
    };
    let generic_ty = generic_args
        .args
        .iter()
        .filter_map(|arg| match arg {
            syn::GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .next()
        .ok_or_else(|| ParseError::new(generic_args.span(), "expected Accounts type"))?;

    let path = match generic_ty {
        syn::Type::Path(ty_path) => &ty_path.path,
        _ => {
            return Err(ParseError::new(
                generic_ty.span(),
                "expected Accounts struct type",
            ))
        }
    };
    Ok(path.segments[0].ident.clone())
}
