use crate::PermissiveType;
use proc_macro2::*;
use quote::quote;
use syn::*;

pub struct MissionArgument {
    pub attrs: Vec<Attribute>,
    pub ident: PatIdent,
    pub colon_token: syn::token::Colon,
    pub ty: Type,
}

impl From<&MissionArgument> for FnArg {
    fn from(val: &MissionArgument) -> Self {
        FnArg::Typed(PatType {
            attrs: val.attrs.to_owned(),
            pat: Box::new(Pat::Ident(val.ident.to_owned())),
            colon_token: val.colon_token,
            ty: Box::new(val.ty.to_owned()),
        })
    }
}

impl From<&MissionArgument> for (Option<syn::token::Mut>, Ident, PermissiveType) {
    fn from(val: &MissionArgument) -> Self {
        (
            val.ident.mutability,
            val.ident.ident.clone(),
            PermissiveType::RestrictiveType(val.ty.to_owned()),
        )
    }
}

pub fn generate_mission_builder(mut function: ItemFn, mission_name: &Ident, state_name: &Ident) -> (ItemFn, Vec<MissionArgument>, Vec<MissionArgument>) {
    let raw_mission_args: Vec<_> = function.sig.inputs.iter().cloned().collect();
    let mut init_args = Vec::new();
    let mut mt_args = Vec::new();
    for raw_mission_arg in raw_mission_args {
        match &raw_mission_arg {
            FnArg::Typed(ty) => {
                if let Pat::Ident(ident) = *ty.pat.clone() {
                    let mut arg = MissionArgument {
                        attrs: ty.attrs.clone(),
                        ident,
                        colon_token: ty.colon_token,
                        ty: *ty.ty.clone(),
                    };
                    match arg.ident.ident.to_string().starts_with("mt_") {
                        true => {
                            arg.ident.ident = Ident::new(&arg.ident.ident.to_string()[3..], arg.ident.ident.span());
                            mt_args.push(arg);
                        }
                        false => init_args.push(arg),
                    }
                } else {
                    panic!("Unsupported pat");
                }
            }
            FnArg::Receiver(_) => panic!("Cannot use methods"),
        }
    }
    function.sig.inputs = punctuated::Punctuated::new();
    init_args.iter().for_each(|f| {
        function.sig.inputs.push_value(f.into());
        function.sig.inputs.push_punct(Token![,](Span::call_site()));
    });
    let mission_args_idents = init_args.iter().map(|a| &a.ident.ident);
    function.sig.output = parse2(quote! { -> #mission_name  }).unwrap();

    function.block = Box::new(
        parse2(quote! {{
            #mission_name {
                state: Some(#state_name::State0{#(#mission_args_idents,)*}),
            }
        }})
        .expect("Failed to create a mission builder"),
    );

    (function, init_args, mt_args)
}
