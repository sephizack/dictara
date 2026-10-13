use crate::config::{self, ConfigKey, ConfigStore, LocalModelConfig, Provider, TranslationConfig};
use crate::models::{ModelInfo, ModelLoader, ModelManager, TranslationService, TRANSLATION_MODEL};
use std::sync::Arc;
use tauri::State;

// ===== LOCAL MODEL COMMANDS =====

/// Get list of all available models with their current status
#[tauri::command]
#[specta::specta]
pub fn get_available_models(
    model_manager: State<Arc<ModelManager>>,
    model_loader: State<Arc<ModelLoader>>,
    translation: State<Arc<TranslationService>>,
) -> Vec<ModelInfo> {
    let mut models = model_manager.get_all_models(&model_loader);
    for model in &mut models {
        if model.is_translation {
            model.is_loaded = translation.is_loaded();
            model.is_loading = translation.is_loading();
        }
    }
    models
}

/// Start downloading a model
#[tauri::command]
#[specta::specta]
pub async fn download_model(
    model_manager: State<'_, Arc<ModelManager>>,
    app: tauri::AppHandle,
    model_name: String,
) -> Result<(), String> {
    model_manager.download_model(&model_name, app).await
}

/// Cancel an ongoing model download
#[tauri::command]
#[specta::specta]
pub fn cancel_model_download(
    model_manager: State<Arc<ModelManager>>,
    model_name: String,
) -> Result<(), String> {
    model_manager.cancel_download(&model_name)
}

/// Delete a downloaded model
#[tauri::command]
#[specta::specta]
pub fn delete_model(
    model_manager: State<Arc<ModelManager>>,
    model_loader: State<Arc<ModelLoader>>,
    translation: State<Arc<TranslationService>>,
    config_store: State<config::Config>,
    model_name: String,
) -> Result<(), String> {
    if model_name == TRANSLATION_MODEL {
        translation.set_enabled(false, config_store.inner())?;
    }
    model_manager.delete_model(&model_name, &model_loader)
}

/// Load a model into memory for transcription
#[tauri::command]
#[specta::specta]
pub async fn load_model(
    model_loader: State<'_, Arc<ModelLoader>>,
    app: tauri::AppHandle,
    model_name: String,
) -> Result<(), String> {
    if model_name == TRANSLATION_MODEL {
        return Err("Enable the translation feature to load this model".into());
    }
    model_loader.load_model(&model_name, &app).await
}

/// Unload the currently loaded model (frees memory)
#[tauri::command]
#[specta::specta]
pub fn unload_model(model_loader: State<Arc<ModelLoader>>) {
    model_loader.unload_model()
}

/// Get the name of the currently loaded model
#[tauri::command]
#[specta::specta]
pub fn get_loaded_model(model_loader: State<Arc<ModelLoader>>) -> Option<String> {
    model_loader.get_loaded_model_name()
}

/// Load local model configuration
#[tauri::command]
#[specta::specta]
pub fn load_local_model_config(
    config_store: State<config::Config>,
) -> Result<Option<LocalModelConfig>, String> {
    Ok(config_store.get(&ConfigKey::LOCAL_MODEL))
}

/// Save local model configuration (selected model)
#[tauri::command]
#[specta::specta]
pub fn save_local_model_config(
    config_store: State<config::Config>,
    model_name: String,
) -> Result<(), String> {
    if model_name == TRANSLATION_MODEL {
        return Err("A translation model cannot be selected for speech recognition".into());
    }
    let config = LocalModelConfig {
        selected_model: Some(model_name),
    };

    config_store.set(&ConfigKey::LOCAL_MODEL, config)
}

/// Delete local model configuration
#[tauri::command]
#[specta::specta]
pub fn delete_local_model_config(config_store: State<config::Config>) -> Result<(), String> {
    config_store.delete(&ConfigKey::LOCAL_MODEL)
}

#[tauri::command]
#[specta::specta]
pub fn load_translation_config(config_store: State<config::Config>) -> TranslationConfig {
    config_store
        .get(&ConfigKey::TRANSLATION)
        .unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
pub async fn save_translation_config(
    config_store: State<'_, config::Config>,
    translation: State<'_, Arc<TranslationService>>,
    model_manager: State<'_, Arc<ModelManager>>,
    enabled: bool,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if enabled {
        if config_store
            .get(&ConfigKey::APP)
            .unwrap_or_default()
            .active_provider
            != Some(Provider::Local)
        {
            return Err("Enable the local provider before enabling translation".into());
        }
        if !model_manager.is_model_downloaded(TRANSLATION_MODEL) {
            return Err("Download the translation model first".into());
        }
    }
    use tauri_specta::Event;
    let _ = crate::models::events::ModelLoadingStateChanged::Started {
        model_name: TRANSLATION_MODEL.into(),
    }
    .emit(&app);
    let store = config_store.inner().clone();
    let service = translation.inner().clone();
    let result = tokio::task::spawn_blocking(move || service.set_enabled(enabled, &store))
        .await
        .map_err(|error| error.to_string())?;
    let event = match &result {
        Ok(()) => crate::models::events::ModelLoadingStateChanged::Complete {
            model_name: TRANSLATION_MODEL.into(),
        },
        Err(error) => crate::models::events::ModelLoadingStateChanged::Error {
            model_name: TRANSLATION_MODEL.into(),
            error: error.clone(),
        },
    };
    let _ = event.emit(&app);
    result
}
