use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    str::FromStr,
    sync::{Mutex, MutexGuard, OnceLock},
};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh32::xxh32;
mod parser;

#[derive(Clone, Debug)]
#[doc(hidden)]
pub enum OriginalString {
    Static(&'static str),
    Owned(String),
}

#[derive(Clone, Debug)]
pub struct HashedString {
    pub hash: u32,
    #[doc(hidden)]
    pub original: OriginalString,
}

impl HashedString {
    pub fn new(s: &str) -> Self {
        let hash = xxh32(s.as_bytes(), 0);
        Self {
            hash,
            original: OriginalString::Owned(s.to_string()),
        }
    }

    pub fn new_static(s: &'static str) -> Self {
        let hash = xxh32(s.as_bytes(), 0);
        Self {
            hash,
            original: OriginalString::Static(s),
        }
    }
}

impl PartialEq for HashedString {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Display for HashedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.original {
            OriginalString::Static(s) => f.write_str(s),
            OriginalString::Owned(s) => f.write_str(s),
        }
    }
}

#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ItemId {
    BigCraftable(u32),
    Boot(u32),
    Flooring(u32),
    Furniture(u32),
    Hat(u32),
    Object(u32),
    Mannequin(u32),
    Pants(u32),
    Shirt(u32),
    Tool(u32),
    Trinket(u32),
    Wallpaper(u32),
    Weapon(u32),
}

impl ItemId {
    fn get_lookup_table() -> MutexGuard<'static, HashMap<Self, String>> {
        static MAP: OnceLock<Mutex<HashMap<ItemId, String>>> = OnceLock::new();
        MAP.get_or_init(Default::default)
            .lock()
            .expect("Let's hope the lock isn't poisoned")
    }

    fn add_lookup_entry(&self, display: &str) {
        let mut table = Self::get_lookup_table();
        table.insert(self.clone(), display.to_string());
    }

    fn lookup_display(&self) -> Option<String> {
        let table = Self::get_lookup_table();
        table.get(self).cloned()
    }
}

impl Display for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(display) = self.lookup_display() {
            f.write_str(&display)
        } else {
            match self {
                ItemId::BigCraftable(v) => f.write_fmt(format_args!("BigCraftable({v}")),
                ItemId::Boot(v) => f.write_fmt(format_args!("Boot({v})")),
                ItemId::Flooring(v) => f.write_fmt(format_args!("Flooring({v})")),
                ItemId::Furniture(v) => f.write_fmt(format_args!("Furnature({v})")),
                ItemId::Hat(v) => f.write_fmt(format_args!("Hat({v})")),
                ItemId::Object(v) => f.write_fmt(format_args!("Object({v})")),
                ItemId::Mannequin(v) => f.write_fmt(format_args!("Mannequin({v})")),
                ItemId::Pants(v) => f.write_fmt(format_args!("Pants({v})")),
                ItemId::Shirt(v) => f.write_fmt(format_args!("Shirt({v})")),
                ItemId::Tool(v) => f.write_fmt(format_args!("Tool({v})")),
                ItemId::Trinket(v) => f.write_fmt(format_args!("Trinket({v})")),
                ItemId::Wallpaper(v) => f.write_fmt(format_args!("Wallpaper({v})")),
                ItemId::Weapon(v) => f.write_fmt(format_args!("Weapon({v})")),
            }
        }
    }
}

impl Debug for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = self.lookup_display();
        match self {
            Self::BigCraftable(arg0) => f
                .debug_tuple("BigCraftable")
                .field(arg0)
                .field(&display)
                .finish(),
            Self::Boot(arg0) => f.debug_tuple("Boot").field(arg0).field(&display).finish(),
            Self::Flooring(arg0) => f
                .debug_tuple("Flooring")
                .field(arg0)
                .field(&display)
                .finish(),
            Self::Furniture(arg0) => f
                .debug_tuple("Furniture")
                .field(arg0)
                .field(&display)
                .finish(),
            Self::Hat(arg0) => f.debug_tuple("Hat").field(arg0).field(&display).finish(),
            Self::Object(arg0) => f.debug_tuple("Object").field(arg0).field(&display).finish(),
            Self::Mannequin(arg0) => f
                .debug_tuple("Mannequin")
                .field(arg0)
                .field(&display)
                .finish(),
            Self::Pants(arg0) => f.debug_tuple("Pants").field(arg0).field(&display).finish(),
            Self::Shirt(arg0) => f.debug_tuple("Shirt").field(arg0).field(&display).finish(),
            Self::Tool(arg0) => f.debug_tuple("Tool").field(arg0).field(&display).finish(),
            Self::Trinket(arg0) => f
                .debug_tuple("Trinket")
                .field(arg0)
                .field(&display)
                .finish(),
            Self::Wallpaper(arg0) => f
                .debug_tuple("Wallpaper")
                .field(arg0)
                .field(&display)
                .finish(),
            Self::Weapon(arg0) => f.debug_tuple("Weapon").field(arg0).field(&display).finish(),
        }
    }
}

impl FromStr for ItemId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (rest, id) = parser::item_id(s).map_err(|_| anyhow!("Can't parse item id \"{s}\""))?;
        if !rest.is_empty() {
            return Err(anyhow!("trailing input at end of valid item id \"{s}\""));
        }
        id.add_lookup_entry(s);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! tagged_test {
        ($variant:ident, $tag:literal) => {
            assert_eq!(
                format!("{}0", $tag).parse::<ItemId>().unwrap(),
                ItemId::$variant(xxh32("0".as_bytes(), 0))
            );
            assert_eq!(
                format!("{}123", $tag).parse::<ItemId>().unwrap(),
                ItemId::$variant(xxh32("123".as_bytes(), 0))
            );
            assert_eq!(
                format!("{}ItemId", $tag).parse::<ItemId>().unwrap(),
                ItemId::$variant(xxh32("ItemId".as_bytes(), 0))
            );
            assert_eq!(
                format!("{}CalicoEggStone_0", $tag)
                    .parse::<ItemId>()
                    .unwrap(),
                ItemId::$variant(xxh32("CalicoEggStone_0".as_bytes(), 0))
            );
        };
    }

    #[test]
    fn item_ids_parse_correctly() {
        tagged_test!(BigCraftable, "(BC)");
        tagged_test!(Boot, "(B)");
        tagged_test!(Flooring, "(FL)");
        tagged_test!(Furniture, "(F)");
        tagged_test!(Hat, "(H)");
        tagged_test!(Mannequin, "(M)");
        tagged_test!(Object, "(O)");
        tagged_test!(Pants, "(P)");
        tagged_test!(Shirt, "(S)");
        tagged_test!(Tool, "(T)");
        tagged_test!(Trinket, "(TR)");
        tagged_test!(Wallpaper, "(WP)");
        tagged_test!(Weapon, "(W)");

        tagged_test!(Object, "");
    }

    #[test]
    fn unknown_item_tag_returns_error() {
        assert!("(💣)123".parse::<ItemId>().is_err());
    }

    #[test]
    fn bad_item_id_returns_error() {
        assert!("(BC)-1".parse::<ItemId>().is_err());
    }

    #[test]
    fn trailing_input_returns_error() {
        assert!("(BC)123💣".parse::<ItemId>().is_err());
    }

    #[test]
    fn owned_hash_string_calculates_hash() {
        let s = HashedString::new("Test String");
        assert_eq!(s.hash, xxh32("Test String".as_bytes(), 0));
    }

    #[test]
    fn owned_hash_string_diplays_corrrectly() {
        let s = HashedString::new("Test String");
        assert_eq!(format!("{s}"), "Test String".to_string());
    }

    #[test]
    fn static_hash_string_calculates_hash() {
        let s = HashedString::new_static("Test String");
        assert_eq!(s.hash, xxh32("Test String".as_bytes(), 0));
    }

    #[test]
    fn static_hash_string_diplays_corrrectly() {
        let s = HashedString::new_static("Test String");
        assert_eq!(format!("{s}"), "Test String".to_string());
    }

    #[test]
    fn hash_string_equality_works() {
        assert_eq!(
            HashedString::new_static("Test String"),
            HashedString::new_static("Test String")
        );
        assert_eq!(
            HashedString::new("Test String"),
            HashedString::new_static("Test String")
        );
        assert_eq!(
            HashedString::new_static("Test String"),
            HashedString::new("Test String")
        );
        assert_eq!(
            HashedString::new("Test String"),
            HashedString::new("Test String")
        );
        assert_ne!(
            HashedString::new_static("Test String"),
            HashedString::new_static("Test String2")
        );
        assert_ne!(
            HashedString::new("Test String"),
            HashedString::new_static("Test String2")
        );
        assert_ne!(
            HashedString::new_static("Test String"),
            HashedString::new("Test String2")
        );
        assert_ne!(
            HashedString::new("Test String"),
            HashedString::new("Test String2")
        );
    }
}
