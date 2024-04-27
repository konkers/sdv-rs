pub mod analyzer;
pub mod common;
pub mod gamedata;
pub mod predictor;
pub mod rng;
pub mod save;

pub use gamedata::{GameData, Locale};
pub use save::SaveGame;

pub trait FromJsonReader
where
    Self: Sized,
{
    fn from_json_reader<R: std::io::Read>(reader: R) -> anyhow::Result<Self>;
}

// exports for proc macros
#[doc(hidden)]
pub mod __private {
    pub use crate::common::ItemId;
    pub use sdv_core;
    pub use sdv_macro::{_hashed_match, _hashed_string, _item_id};
}

#[macro_export]
macro_rules! item_id {
    ($item_id:literal) => {{
        use $crate::__private as __sdv_crate_private;
        $crate::__private::_item_id!($item_id)
    }};
}

#[macro_export]
macro_rules! hashed_string {
    ($s:literal) => {
        $crate::__private::_hashed_string!($crate::__private::sdv_core, $s)
    };
}

#[macro_export]
macro_rules! hashed_match {
    ($s:literal) => {
        $crate::__private::_hashed_match!($crate::__private::sdv_core, $s)
    };
}

#[macro_export]
macro_rules! generate_seed {
    ($generator:ty, $a:expr) => {
        <$generator>::generate_seed($a as f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64)
    };
    ($generator:ty, $a:expr, $b:expr) => {
        <$generator>::generate_seed($a as f64, $b as f64, 0.0f64, 0.0f64, 0.0f64)
    };
    ($generator:ty, $a:expr, $b:expr, $c:expr) => {
        <$generator>::generate_seed($a as f64, $b as f64, $c as f64, 0.0f64, 0.0f64)
    };
    ($generator:ty, $a:expr, $b:expr, $c:expr, $d:expr) => {
        <$generator>::generate_seed($a as f64, $b as f64, $c as f64, $d as f64, 0.0f64)
    };
    ($generator:ty, $a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {
        <$generator>::generate_seed($a as f64, $b as f64, $c as f64, $d as f64, $e as f64)
    };
}

#[macro_export]
macro_rules! generate_day_save_seed {
    ($generator:ty, $days_played:expr, $game_id:expr) => {
        <$generator>::generate_day_save_seed(
            $days_played as u32,
            $game_id as u32,
            0.0 as f64,
            0.0 as f64,
            0.0 as f64,
        )
    };
    ($generator:ty, $days_played:expr, $game_id:expr, $a:expr) => {
        <$generator>::generate_day_save_seed(
            $days_played as u32,
            $game_id as u32,
            $a as f64,
            0.0 as f64,
            0.0 as f64,
        )
    };
    ($generator:ty, $days_played:expr, $game_id:expr, $a:expr, $b:expr) => {
        <$generator>::generate_day_save_seed(
            $days_played as u32,
            $game_id as u32,
            $a as f64,
            $b as f64,
            0.0 as f64,
        )
    };
    ($generator:ty, $days_played:expr, $game_id:expr, $a:expr, $b:expr, $c:expr) => {
        <$generator>::generate_day_save_seed(
            $days_played as u32,
            $game_id as u32,
            $a as f64,
            $b as f64,
            $c as f64,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_id_macro_generates_correct_item_ids() {
        use xxhash_rust::xxh32::xxh32;
        let hash_123 = xxh32("123".as_bytes(), 0);
        let hash_item_id = xxh32("ItemId".as_bytes(), 0);
        assert_eq!(item_id!("(BC)123"), common::ItemId::BigCraftable(hash_123));
        assert_eq!(item_id!("(O)123"), common::ItemId::Object(hash_123));
        assert_eq!(item_id!("123"), common::ItemId::Object(hash_123));
        assert_eq!(item_id!("(O)ItemId"), common::ItemId::Object(hash_item_id));
        assert_eq!(item_id!("ItemId"), common::ItemId::Object(hash_item_id));
    }

    #[test]
    fn hashed_string_macro_generates_correct_hashed_strings() {
        assert_eq!(
            hashed_string!("Test String"),
            common::HashedString::new("Test String")
        );
        assert_eq!(
            hashed_string!("Test String"),
            common::HashedString::new_static("Test String")
        );
        assert_ne!(
            hashed_string!("Test String"),
            common::HashedString::new("Test String2")
        );
        assert_ne!(
            hashed_string!("Test String"),
            common::HashedString::new_static("Test String2")
        );
    }

    #[test]
    fn hasehed_match_works() {
        let string = hashed_string!("Test String");
        let result = match string {
            hashed_match!("Test String 2") => false,
            hashed_match!("Test String") => true,
            _ => false,
        };

        assert!(result);
    }
}
