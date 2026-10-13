use crate::config::{self, ConfigKey, ConfigStore, Provider};
use crate::models::TranslationService;
use log::error;
use std::sync::Arc;
use tauri::State;

// ===== PROVIDER SELECTION COMMANDS =====

/// Get the currently active provider
#[tauri::command]
#[specta::specta]
pub fn get_current_provider(
    config_store: State<config::Config>,
) -> Result<Option<Provider>, String> {
    let config = config_store.get(&ConfigKey::APP).unwrap_or_default();
    Ok(config.active_provider)
}

/// Set the currently active provider
#[tauri::command]
#[specta::specta]
pub fn set_current_provider(
    config_store: State<config::Config>,
    provider: String,
    translation: State<Arc<TranslationService>>,
) -> Result<(), String> {
    // Load existing config to preserve other fields
    let mut config = config_store.get(&ConfigKey::APP).unwrap_or_default();

    // Parse and set the provider
    config.active_provider = Some(match provider.as_str() {
        "open_ai" | "openai" => Provider::OpenAI,
        "azure_open_ai" | "azure_openai" | "azure" => Provider::AzureOpenAI,
        "local" => Provider::Local,
        _ => {
            error!("Invalid provider: {}", provider);
            return Err(format!("Invalid provider: {}", provider));
        }
    });

    if config.active_provider != Some(Provider::Local) {
        translation.set_enabled(false, config_store.inner())?;
    }
    config_store.set(&ConfigKey::APP, config)
}

/// Clear the currently active provider (set to None)
#[tauri::command]
#[specta::specta]
pub fn clear_current_provider(
    config_store: State<config::Config>,
    translation: State<Arc<TranslationService>>,
) -> Result<(), String> {
    // Load existing config to preserve other fields
    let mut config = config_store.get(&ConfigKey::APP).unwrap_or_default();
    config.active_provider = None;

    translation.set_enabled(false, config_store.inner())?;
    config_store.set(&ConfigKey::APP, config)
}
