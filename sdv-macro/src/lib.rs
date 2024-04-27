use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use sdv_core::{HashedString, ItemId};
use syn::{parse::Parse, parse_macro_input, LitStr, Path, Token};

struct ItemIdArg {
    id: ItemId,
}

impl Parse for ItemIdArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let id_string: LitStr = input.parse()?;
        let id = id_string
            .value()
            .parse::<ItemId>()
            .map_err(|_| input.error(format!("error prasing item id \"{}\"", id_string.value())))?;

        Ok(ItemIdArg { id })
    }
}

fn _item_id_macro_impl(arg: ItemIdArg) -> TokenStream {
    let (variant, id_hash) = match arg.id {
        ItemId::BigCraftable(id) => ("BigCraftable", id),
        ItemId::Boot(id) => ("Boot", id),
        ItemId::Flooring(id) => ("Flooring", id),
        ItemId::Furniture(id) => ("Furniture", id),
        ItemId::Hat(id) => ("Hat", id),
        ItemId::Object(id) => ("Object", id),
        ItemId::Mannequin(id) => ("Mannequin", id),
        ItemId::Pants(id) => ("Pants", id),
        ItemId::Shirt(id) => ("Shirt", id),
        ItemId::Tool(id) => ("Tool", id),
        ItemId::Trinket(id) => ("Trinket", id),
        ItemId::Wallpaper(id) => ("Wallpaper", id),
        ItemId::Weapon(id) => ("Weapon", id),
    };
    let variant = format_ident!("{}", variant);

    quote! {
        {
            __sdv_crate_private::ItemId::#variant(#id_hash)
        }
    }
}

#[proc_macro]
pub fn _item_id(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as ItemIdArg);

    _item_id_macro_impl(input).into()
}

struct HashedStringArgs {
    core_crate_path: Path,
    value: LitStr,
}

impl Parse for HashedStringArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let core_crate_path: Path = input.parse()?;
        let _: Token![,] = input.parse()?;
        let value: LitStr = input.parse()?;
        Ok(Self {
            core_crate_path,
            value,
        })
    }
}

#[proc_macro]
pub fn _hashed_string(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as HashedStringArgs);
    let value = input.value;
    let core_crate_path = input.core_crate_path;
    let hashed = HashedString::new(&value.value());
    let hash = hashed.hash;
    quote! {{
        #core_crate_path::HashedString{
            hash: #hash,
            original: #core_crate_path::OriginalString::Static(#value),
        }
    }}
    .into()
}

#[proc_macro]
pub fn _hashed_match(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as HashedStringArgs);
    let value = input.value;
    let core_crate_path = input.core_crate_path;
    let hashed = HashedString::new(&value.value());
    let hash = hashed.hash;
    quote! {
        #core_crate_path::HashedString{
            hash: #hash,
            ..
        }
    }
    .into()
}
