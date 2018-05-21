use syn::*;
use ext::*;
use proc_macro2::TokenStream;
use spanned::Spanned;

use FieldMember;

pub trait CodegenFieldsExt {
    fn surround(&self, tokens: TokenStream) -> TokenStream;
    fn ignore_tokens(&self) -> TokenStream;
    fn id_match_tokens(&self) -> TokenStream;
}

pub fn field_to_ident(i: usize, field: &Field) -> Ident {
    let name = match field.ident {
        Some(ref id) => format!("_{}", id),
        None => format!("_{}", i)
    };

    Ident::new(&name, field.span().into())
}

pub fn field_to_match((i, field): (usize, &Field)) -> TokenStream {
    let ident = field_to_ident(i, field);
    match field.ident {
        Some(ref id) => quote!(#id: #ident),
        None => quote!(#ident)
    }
}

impl CodegenFieldsExt for Fields {
    fn surround(&self, tokens: TokenStream) -> TokenStream {
        match *self {
            Fields::Named(..) => quote!({ #tokens }),
            Fields::Unnamed(..) => quote!(( #tokens )),
            Fields::Unit => quote!()
        }
    }

    fn ignore_tokens(&self) -> TokenStream {
        self.surround(quote!(..))
    }

    fn id_match_tokens(&self) -> TokenStream {
        let idents = self.iter()
            .enumerate()
            .map(field_to_match);

        self.surround(quote!(#(#idents),*))
    }
}

pub trait TokenStreamExt {
    fn tokens(&self) -> TokenStream;
}

impl<'f> TokenStreamExt for FieldMember<'f> {
    fn tokens(&self) -> TokenStream {
        let index = self.member.unnamed().map(|i| i.index).unwrap_or(0);
        let ident = field_to_ident(index as usize, &self.field);
        quote!(#ident)
    }
}

// use rocket::http::{ContentType, MediaType, Status};

// impl TokenStreamExt for ContentType {
//     fn tokens(&self) -> TokenStream {
//         let mt_tokens = self.0.tokens();
//         quote!(rocket::http::ContentType(#mt_tokens))
//     }
// }

// impl TokenStreamExt for MediaType {
//     fn tokens(&self) -> TokenStream {
//         let (top, sub) = (self.top().as_str(), self.sub().as_str());
//         let (keys, values) = (self.params().map(|(k, _)| k), self.params().map(|(_, v)| v));
//         quote!(rocket::http::MediaType {
//             source: rocket::http::Source::None,
//             top: rocket::http::IndexedStr::Concrete(
//                 std::borrow::Cow::Borrowed(#top)
//             ),
//             sub: rocket::http::IndexedStr::Concrete(
//                 std::borrow::Cow::Borrowed(#sub)
//             ),
//             params: rocket::http::MediaParams::Static(&[
//                 #((
//                     rocket::http::IndexedStr::Concrete(std::borrow::Cow::Borrowed(#keys)),
//                     rocket::http::IndexedStr::Concrete(std::borrow::Cow::Borrowed(#values))
//                 )),*
//             ])
//         })
//     }
// }

// impl TokenStreamExt for Status {
//     fn tokens(&self) -> TokenStream {
//         let (code, reason) = (self.code, self.reason);
//         quote!(rocket::http::Status { code: #code, reason: #reason })
//     }
// }
