use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use super::qwen::QwenClient;
use crate::config::{ConfigKey, ConfigStore, TranslationConfig};

pub const TRANSLATION_MODEL: &str = "qwen3-4b-4bit";

pub struct TranslationService {
    client: Mutex<Option<QwenClient>>,
    enabled: AtomicBool,
    loading: AtomicBool,
    loaded: AtomicBool,
    models_dir: PathBuf,
    resources_dir: PathBuf,
}

impl TranslationService {
    pub fn new(models_dir: PathBuf, resources_dir: PathBuf) -> Self {
        Self {
            client: Mutex::new(None),
            enabled: AtomicBool::new(false),
            loading: AtomicBool::new(false),
            loaded: AtomicBool::new(false),
            models_dir,
            resources_dir,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::Acquire)
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded.load(Ordering::Acquire)
    }

    pub fn set_enabled(&self, enabled: bool, store: &impl ConfigStore) -> Result<(), String> {
        if !enabled {
            self.enabled.store(false, Ordering::Release);
        }
        let mut client = self.client.lock().map_err(|error| error.to_string())?;
        if !enabled {
            *client = None;
            self.loaded.store(false, Ordering::Release);
            return store.set(&ConfigKey::TRANSLATION, TranslationConfig { enabled });
        }
        if enabled && client.is_none() {
            self.loading.store(true, Ordering::Release);
            let result = QwenClient::new_translator(
                &self.models_dir.join(TRANSLATION_MODEL),
                &self.resources_dir,
            );
            self.loading.store(false, Ordering::Release);
            let loaded = result?;
            store.set(&ConfigKey::TRANSLATION, TranslationConfig { enabled })?;
            *client = Some(loaded);
            self.loaded.store(true, Ordering::Release);
        } else {
            store.set(&ConfigKey::TRANSLATION, TranslationConfig { enabled })?;
        }
        self.enabled.store(enabled, Ordering::Release);
        Ok(())
    }

    pub fn translate(&self, text: &str) -> Result<String, String> {
        let mut client = self.client.lock().map_err(|error| error.to_string())?;
        if !self.is_enabled() {
            return Err("Local translation is disabled".into());
        }
        let text = client
            .as_mut()
            .ok_or("The translation model is not loaded")?
            .translate(text)?;
        if !self.is_enabled() {
            return Err("Local translation was disabled during this recording".into());
        }
        if text.trim().is_empty() {
            return Err("The translation was empty".into());
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_translation_never_starts_a_process() {
        let service = TranslationService::new(
            PathBuf::from("/missing-models"),
            PathBuf::from("/missing-engine"),
        );
        assert!(!service.is_enabled());
        assert!(!service.is_loading());
        assert!(!service.is_loaded());
        assert!(service.client.lock().unwrap().is_none());
        assert!(service
            .translate("Bonjour")
            .unwrap_err()
            .contains("disabled"));
        assert!(service.client.lock().unwrap().is_none());
    }

    #[derive(Default)]
    struct TestStore(Mutex<std::collections::HashMap<String, serde_json::Value>>);

    impl ConfigStore for TestStore {
        fn get<T: serde::de::DeserializeOwned>(&self, key: &ConfigKey<T>) -> Option<T> {
            self.0
                .lock()
                .unwrap()
                .get(key.key_name())
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok())
        }
        fn set<T: serde::Serialize>(&self, key: &ConfigKey<T>, value: T) -> Result<(), String> {
            self.0.lock().unwrap().insert(
                key.key_name().into(),
                serde_json::to_value(value).map_err(|error| error.to_string())?,
            );
            Ok(())
        }
        fn delete<T>(&self, key: &ConfigKey<T>) -> Result<(), String> {
            self.0.lock().unwrap().remove(key.key_name());
            Ok(())
        }
    }

    #[test]
    fn disabling_translation_persists_without_starting_a_process() {
        let service = TranslationService::new(
            PathBuf::from("/missing-models"),
            PathBuf::from("/missing-engine"),
        );
        let store = TestStore::default();
        service.set_enabled(false, &store).unwrap();
        assert!(!store.get(&ConfigKey::TRANSLATION).unwrap().enabled);
        assert!(!service.is_loaded());
    }

    #[test]
    #[ignore = "Requires downloaded Qwen3 translation weights"]
    fn translates_offline_and_unloads_when_disabled() {
        let model_dir = PathBuf::from(std::env::var_os("DICTARA_TRANSLATION_MODELS_DIR").unwrap());
        let service = TranslationService::new(model_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        let store = TestStore::default();
        assert!(!service.is_loaded());
        service.set_enabled(true, &store).unwrap();
        assert!(store.get(&ConfigKey::TRANSLATION).unwrap().enabled);
        assert!(service.is_enabled());
        assert!(service.is_loaded());
        let process_id = service
            .client
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .process_id();
        let question = service
            .translate("Bonjour, comment allez-vous ?")
            .unwrap()
            .to_lowercase();
        assert!(
            question.contains("hello") && question.contains("how are you"),
            "{question}"
        );
        let meeting = service
            .translate("La réunion avec Alice commence demain à 14h30.")
            .unwrap();
        assert!(
            meeting.contains("Alice") && meeting.to_lowercase().contains("tomorrow"),
            "{meeting}"
        );
        assert!(
            meeting.contains("14:30") || meeting.contains("2:30"),
            "{meeting}"
        );
        let thanks = service.translate("Merci.").unwrap().to_lowercase();
        assert!(
            thanks.contains("thank") && !thanks.contains("alice"),
            "{thanks}"
        );
        service.set_enabled(false, &store).unwrap();
        assert!(!service.is_enabled());
        assert!(!service.is_loaded());
        assert!(!store.get(&ConfigKey::TRANSLATION).unwrap().enabled);
        assert!(service.client.lock().unwrap().is_none());
        assert!(service.translate("Bonjour").is_err());
        assert!(!std::process::Command::new("kill")
            .args(["-0", &process_id.to_string()])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success());
    }

    #[test]
    fn translation_manifest_is_complete_and_pinned() {
        let entry: super::super::catalog::ModelCatalogEntry =
            serde_json::from_str(include_str!("../../qwen/translation-model.json")).unwrap();
        assert_eq!(entry.name, TRANSLATION_MODEL);
        assert_eq!(entry.files.len(), 9);
        for file in entry.files {
            assert_eq!(file.sha256.len(), 64);
            assert!(file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
            assert!(file
                .url
                .contains("/resolve/4dcb3d101c2a062e5c1d4bb173588c54ea6c4d25/"));
        }
    }
}
