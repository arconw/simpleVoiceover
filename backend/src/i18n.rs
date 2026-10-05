use serde_json::Value;
use std::{collections::BTreeMap, sync::OnceLock};

pub const LANGUAGES: [&str; 13] = [
    "en", "ru", "fr", "pl", "es", "pt", "de", "it", "uk", "tr", "ja", "ko", "zh-CN",
];

pub fn message(key: &str) -> String {
    format!("[[{key}]]")
}

pub fn formatted_message(key: &str, params: &[(&str, String)]) -> String {
    let params: BTreeMap<_, _> = params.iter().map(|(name, value)| (*name, value)).collect();
    let encoded: String = serde_json::to_vec(&params)
        .unwrap()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("[[{key}|{encoded}]]")
}

pub fn text(language: &str, key: &str) -> String {
    static CATALOGS: OnceLock<BTreeMap<&str, Value>> = OnceLock::new();
    let catalogs = CATALOGS.get_or_init(|| {
        [
            ("en", include_str!("../../src/locales/en.json")),
            ("ru", include_str!("../../src/locales/ru.json")),
            ("fr", include_str!("../../src/locales/fr.json")),
            ("pl", include_str!("../../src/locales/pl.json")),
            ("es", include_str!("../../src/locales/es.json")),
            ("pt", include_str!("../../src/locales/pt.json")),
            ("de", include_str!("../../src/locales/de.json")),
            ("it", include_str!("../../src/locales/it.json")),
            ("uk", include_str!("../../src/locales/uk.json")),
            ("tr", include_str!("../../src/locales/tr.json")),
            ("ja", include_str!("../../src/locales/ja.json")),
            ("ko", include_str!("../../src/locales/ko.json")),
            ("zh-CN", include_str!("../../src/locales/zh-CN.json")),
        ]
        .into_iter()
        .map(|(language, data)| (language, serde_json::from_str(data).unwrap()))
        .collect()
    });
    catalogs
        .get(language)
        .and_then(|catalog| catalog[key].as_str())
        .filter(|value| !value.is_empty())
        .or_else(|| catalogs["en"][key].as_str())
        .unwrap_or(key)
        .to_owned()
}
