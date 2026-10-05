use crate::{commands, i18n, storage::Store};
use anyhow::Result;
use serde_json::json;
use std::fs;

#[test]
fn preferences_persist_without_project_edits_and_survive_project_changes() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let configuration = directory.path().join("configuration");
    let working = directory.path().join("working");
    let project_path = directory.path().join("session.justspeak");
    let mut store = Store::open(configuration.clone(), Some(working))?;
    assert_eq!(store.config.language, "system");
    store.save(&project_path)?;
    let project_before = fs::read(&project_path)?;
    commands::execute(
        &mut store,
        &json!({"command":"preferences_patch", "preference":"pl"}),
    )?;
    assert!(!store.config.dirty);
    assert!(store.history.is_empty());
    assert_eq!(fs::read(&project_path)?, project_before);
    let recovered = Store::open(configuration.clone(), None)?;
    assert_eq!(recovered.config.language, "pl");
    store.load(&project_path)?;
    assert_eq!(store.config.language, "pl");
    store.change_directory(directory.path().join("another-working"))?;
    assert_eq!(store.config.language, "pl");
    assert!(store.set_language("ar").is_err());
    assert_eq!(store.config.language, "pl");
    store.set_language("system")?;
    assert_eq!(Store::open(configuration, None)?.config.language, "system");
    Ok(())
}

#[test]
fn old_configuration_uses_the_system_language() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let configuration = directory.path().join("configuration");
    let store = Store::open(
        configuration.clone(),
        Some(directory.path().join("working")),
    )?;
    let mut settings = serde_json::to_value(&store.config)?;
    settings.as_object_mut().unwrap().remove("language");
    settings.as_object_mut().unwrap().remove("effectPresets");
    fs::write(
        configuration.join("settings.json"),
        serde_json::to_vec(&settings)?,
    )?;
    assert_eq!(
        Store::open(configuration.clone(), None)?.config.language,
        "system"
    );
    settings["language"] = json!("unsupported");
    fs::write(
        configuration.join("settings.json"),
        serde_json::to_vec(&settings)?,
    )?;
    assert_eq!(Store::open(configuration, None)?.config.language, "system");
    Ok(())
}

#[test]
fn custom_presets_persist_independently_and_validate_before_writing() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let configuration = directory.path().join("config");
    let project = directory.path().join("session.justspeak");
    let mut store = Store::open(configuration.clone(), Some(directory.path().join("work")))?;
    store.save(&project)?;
    let before = fs::read(&project)?;
    let effects = crate::model::Effects::voice();
    commands::execute(
        &mut store,
        &json!({"command":"preset_save","name":" My voice ","effects":effects,"fxBypass":false}),
    )?;
    assert!(!store.config.dirty);
    assert!(store.history.is_empty());
    assert_eq!(fs::read(&project)?, before);
    let preset_id = store.config.effect_presets[0].id.clone();
    let recovered = Store::open(configuration, None)?;
    assert_eq!(recovered.config.effect_presets[0].name, "My voice");
    assert!(recovered.config.effect_presets[0].effects == effects);
    store.load(&project)?;
    store.save_preset("my voice", crate::model::Effects::neutral(), true)?;
    assert_eq!(store.config.effect_presets.len(), 1);
    assert_eq!(store.config.effect_presets[0].id, preset_id);
    assert!(store.config.effect_presets[0].fx_bypass);
    let configuration_before = fs::read(store.config_directory.join("settings.json"))?;
    for name in ["", "  ", "line\nbreak"] {
        assert!(store.save_preset(name, effects.clone(), false).is_err());
    }
    let mut invalid = effects;
    invalid.ratio = 100.;
    assert!(store.save_preset("Invalid", invalid, false).is_err());
    assert_eq!(
        fs::read(store.config_directory.join("settings.json"))?,
        configuration_before
    );
    Ok(())
}

#[test]
fn native_dialogs_share_complete_catalogs_and_english_fallback() {
    for language in i18n::LANGUAGES {
        assert_ne!(i18n::text(language, "media.add"), "media.add");
        assert!(i18n::text(language, "dialog.projectFilename").ends_with(".justspeak"));
    }
    assert_eq!(
        i18n::text("unsupported", "media.add"),
        i18n::text("en", "media.add")
    );
    assert_eq!(i18n::message("error.trackLocked"), "[[error.trackLocked]]");
    let message = i18n::formatted_message("progress.import", &[("name", "voice.wav".into())]);
    assert!(message.starts_with("[[progress.import|"));
    assert!(message.ends_with("]]"));
}
