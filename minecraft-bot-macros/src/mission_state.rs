use syn::*;
use quote::{quote, ToTokens};

#[derive(Debug, Clone)]
pub(crate) struct MissionState {
    pub(crate) variant_ident: Ident,
    pub(crate) parent_loops: Vec<String>,
    pub(crate) fields: Vec<(Option<token::Mut>, Ident, PermissiveType)>,
    pub(crate) stmts: Vec<Stmt>,
    pub(crate) next_mission: Option<Box<MissionState>>,
}

impl MissionState {
    pub(crate) fn declaration(&self) -> proc_macro2::TokenStream {
        let variant_ident = &self.variant_ident;
        let variant_field_idents = self.fields.iter().map(|t| &t.1);
        let variant_field_types = self.fields.iter().map(|t| &t.2);
        quote! {
            #variant_ident { #(#variant_field_idents: #variant_field_types,)* }
        }
    }

    pub(crate) fn match_arm(&self) -> proc_macro2::TokenStream {
        let variant_ident = &self.variant_ident;
        let stmts = &self.stmts;
        let variant_field_idents = self.fields.iter().map(|t| &t.1);
        let variant_field_idents2 = self.fields.iter().map(|t| &t.1);
        let variant_field_mutability = self.fields.iter().map(|t| &t.0);

        if let Some(next_mission) = &self.next_mission {
            let next_variant_ident = &next_mission.variant_ident;
            let next_variant_fields = next_mission.fields.iter().map(|f| &f.1);

            quote! {
                GeneratedMissionState::#variant_ident { #(#variant_field_idents,)* } => {
                    #(let #variant_field_mutability #variant_field_idents2 = *#variant_field_idents2;)*
                    #(#stmts)*
                    self.state = GeneratedMissionState::#next_variant_ident { #(#next_variant_fields, )* };
                },
            }
        } else {
            quote! {
                GeneratedMissionState::#variant_ident { #(#variant_field_idents,)* } => {
                    #(let #variant_field_mutability #variant_field_idents2 = *#variant_field_idents2;)*
                    #(#stmts)*
                },
            }
        }
    }

    pub(crate) fn switch_to_this_state(&self) -> Expr {
        let variant_ident = &self.variant_ident;
        let fields = self.fields.iter().map(|f| &f.1);
        let tokens = quote! {{
            self.state = GeneratedMissionState::#variant_ident { #(#fields, )* };
            return MissionResult::InProgress;
        }};
        syn::parse2(tokens).unwrap()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum PermissiveType {
    RestrictiveType(Type),
    Path(TypePath),
}

impl ToTokens for PermissiveType {
    fn to_tokens(&self, tokens: &mut __private::TokenStream2) {
        match self {
            PermissiveType::RestrictiveType(e) => e.to_tokens(tokens),
            PermissiveType::Path(e) => e.to_tokens(tokens),
        }
    }
}
