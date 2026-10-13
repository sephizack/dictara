use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct EngineResponse {
    #[serde(default)]
    ready: bool,
    text: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptionRequest<'audio> {
    audio_path: &'audio Path,
}

pub struct QwenClient {
    child: Child,
    input: ChildStdin,
    responses: Receiver<Result<EngineResponse, String>>,
    engine_path: PathBuf,
    model_path: PathBuf,
    translation: bool,
}

impl QwenClient {
    pub fn new(model_path: &Path, resources_dir: &Path) -> Result<Self, String> {
        Self::new_with_mode(model_path, resources_dir, false)
    }

    pub fn new_translator(model_path: &Path, resources_dir: &Path) -> Result<Self, String> {
        Self::new_with_mode(model_path, resources_dir, true)
    }

    fn new_with_mode(
        model_path: &Path,
        resources_dir: &Path,
        translation: bool,
    ) -> Result<Self, String> {
        let engine_path = if cfg!(debug_assertions) {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/qwen/dictara-qwen")
        } else {
            resources_dir.join("resources/qwen/dictara-qwen")
        };
        Self::start(&engine_path, model_path, translation)
    }

    fn start(engine_path: &Path, model_path: &Path, translation: bool) -> Result<Self, String> {
        let mut command = Command::new(engine_path);
        if translation {
            command.arg("--translate");
        }
        let mut child = command
            .arg(model_path)
            .env("HF_HUB_OFFLINE", "1")
            .env("HF_HUB_DISABLE_TELEMETRY", "1")
            .env_remove("HF_TOKEN")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("Cannot start the local Qwen engine: {error}"))?;
        let input = child.stdin.take().expect("Qwen stdin is piped");
        let output = child.stdout.take().expect("Qwen stdout is piped");
        let (sender, responses) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let response = line.map_err(|error| error.to_string()).and_then(|line| {
                    serde_json::from_str(&line).map_err(|error| error.to_string())
                });
                if sender.send(response).is_err() {
                    break;
                }
            }
        });
        let mut client = Self {
            child,
            input,
            responses,
            engine_path: engine_path.to_path_buf(),
            model_path: model_path.to_path_buf(),
            translation,
        };
        let response = client.receive(Duration::from_secs(180))?;
        if !response.ready {
            return Err("Qwen engine did not finish loading".into());
        }
        Ok(client)
    }

    fn receive(&mut self, timeout: Duration) -> Result<EngineResponse, String> {
        let response = match self.responses.recv_timeout(timeout) {
            Ok(response) => response?,
            Err(error) => {
                let _ = self.child.kill();
                return Err(format!("Qwen engine stopped or timed out: {error}"));
            }
        };
        if let Some(error) = response.error.as_ref() {
            return Err(error.clone());
        }
        Ok(response)
    }

    pub fn transcribe_file(&mut self, audio_path: &Path) -> Result<String, String> {
        self.request(&TranscriptionRequest { audio_path })
    }

    pub fn translate(&mut self, text: &str) -> Result<String, String> {
        self.request(&serde_json::json!({ "text": text }))
    }

    #[cfg(test)]
    pub(super) fn process_id(&self) -> u32 {
        self.child.id()
    }

    fn request(&mut self, request: &impl Serialize) -> Result<String, String> {
        if self
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            *self = Self::start(&self.engine_path, &self.model_path, self.translation)?;
        }
        serde_json::to_writer(&mut self.input, request).map_err(|error| error.to_string())?;
        self.input
            .write_all(b"\n")
            .map_err(|error| error.to_string())?;
        self.input.flush().map_err(|error| error.to_string())?;
        self.receive(Duration::from_secs(600))?
            .text
            .map(|text| text.trim().to_string())
            .ok_or_else(|| "Qwen engine returned no text".into())
    }
}

impl Drop for QwenClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_ready_and_transcription_responses() {
        let ready: EngineResponse = serde_json::from_str(r#"{"ready":true}"#).unwrap();
        assert!(ready.ready);
        let response: EngineResponse =
            serde_json::from_str(r#"{"text":"Un caf\u00e9, \"s'il vous pla\u00eet\"."}"#).unwrap();
        assert_eq!(
            response.text.as_deref(),
            Some("Un caf\u{e9}, \"s'il vous pla\u{ee}t\".")
        );
    }

    #[test]
    fn preserves_engine_errors() {
        let response: EngineResponse =
            serde_json::from_str(r#"{"error":"Invalid audio"}"#).unwrap();
        assert_eq!(response.error.as_deref(), Some("Invalid audio"));
    }

    #[test]
    fn serializes_audio_paths_without_shell_interpolation() {
        let request = TranscriptionRequest {
            audio_path: Path::new("/tmp/audio with spaces.wav"),
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["audioPath"], "/tmp/audio with spaces.wav");
    }

    #[test]
    #[ignore = "Requires downloaded Qwen weights and a 16kHz WAV fixture"]
    fn transcribes_offline_and_restarts_after_engine_exit() {
        let model = PathBuf::from(std::env::var_os("DICTARA_QWEN_MODEL_DIR").unwrap());
        let audio = PathBuf::from(std::env::var_os("DICTARA_QWEN_TEST_AUDIO").unwrap());
        let resources = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut client = QwenClient::new(&model, resources).unwrap();
        let original_process = client.child.id();
        let text = client.transcribe_file(&audio).unwrap();
        assert!(text.to_lowercase().contains("bonjour"));
        assert!(text.to_lowercase().contains("transcription locale"));
        client.child.kill().unwrap();
        client.child.wait().unwrap();
        assert_eq!(client.transcribe_file(&audio).unwrap(), text);
        assert_ne!(client.child.id(), original_process);
    }
}
