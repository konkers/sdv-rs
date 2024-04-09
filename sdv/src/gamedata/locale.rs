use std::{collections::HashMap, path::Path};

use anyhow::Result;
use indexmap::IndexMap;

#[derive(Debug)]
pub struct Locale {
    pub strings: HashMap<String, String>,
}

impl Locale {
    pub fn load<P: AsRef<Path>>(game_content_dir: P, locale: &str) -> Result<Self> {
        let locale_extension = if locale == "en-EN" {
            "xnb".to_string()
        } else {
            format!("{}.xnb", locale)
        };

        let mut strings = HashMap::new();

        let mut strings_dir = game_content_dir.as_ref().to_path_buf();
        strings_dir.push("Strings");
        for entry in std::fs::read_dir(strings_dir)? {
            let path = entry?.path();
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let file_name = file_name.to_string_lossy();
            let Some((base_name, extension)) = file_name.split_once('.') else {
                continue;
            };
            if extension != locale_extension.as_str() {
                continue;
            }
            let data = std::fs::read(&path)?;
            let Ok(entries) = xnb::from_bytes::<IndexMap<String, String>>(&data) else {
                continue;
            };
            for (key, value) in entries {
                let key = format!("[LocalizedText Strings\\{base_name}:{key}]");
                strings.insert(key, value);
            }
        }

        Ok(Self { strings })
    }
}
