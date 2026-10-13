mod catalog;
pub mod events;
mod loader;
mod local_client;
mod manager;
mod qwen;
mod status;
mod translation;

pub use catalog::{is_model_in_catalog, ModelCatalogEntry};
pub use loader::ModelLoader;
pub use manager::ModelManager;
pub use status::ModelStatus;
pub use translation::{TranslationService, TRANSLATION_MODEL};

use serde::{Deserialize, Serialize};

/// Combined view sent to frontend (catalog + status merged).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    // From catalog
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub size_bytes: u64,
    pub estimated_ram_mb: u64,
    pub is_translation: bool,

    // From status
    pub is_downloaded: bool,
    pub is_downloading: bool,
    pub is_loaded: bool,
    pub is_loading: bool,
    pub downloaded_bytes: u64,
}

impl ModelInfo {
    pub fn from_catalog_and_status(catalog: &ModelCatalogEntry, status: &ModelStatus) -> Self {
        Self {
            name: catalog.name.clone(),
            display_name: catalog.display_name.clone(),
            description: catalog.description.clone(),
            size_bytes: catalog.size_bytes,
            estimated_ram_mb: catalog.estimated_ram_mb,
            is_translation: catalog.model_type == catalog::ModelType::QwenTranslation,
            is_downloaded: status.is_downloaded,
            is_downloading: status.is_downloading,
            is_loaded: status.is_loaded,
            is_loading: status.is_loading,
            downloaded_bytes: status.downloaded_bytes,
        }
    }
}
