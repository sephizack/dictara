use crate::config::ShortcutsConfig;
use crate::models::TranslationService;
use crate::recording::{RecordingCommand, RecordingStateManager};
use crate::shortcuts::events::KeyCaptureEvent;
use dictara_keyboard::{grab, Event, EventType};
use log::{error, info};
use std::collections::HashSet;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tauri::AppHandle;
use tauri_specta::Event as EventTrait;
use tokio::sync::mpsc;

/// Operating mode for the keyboard listener
enum ListenerMode {
    /// Normal mode: match shortcuts and trigger recording
    Normal { shortcuts: ShortcutsConfig },
    /// Capture mode: emit key events to frontend for configuration
    Capture { app_handle: AppHandle },
}

#[derive(Default)]
struct TranslationHold {
    active: bool,
    stopped: bool,
}

/// Keyboard listener that detects key events and emits recording commands
pub struct KeyListener {
    _thread_handle: Option<JoinHandle<()>>,
    mode_tx: mpsc::Sender<ListenerMode>, // Send mode updates to thread
}

impl KeyListener {
    pub fn start(
        command_tx: mpsc::Sender<RecordingCommand>,
        state_manager: Arc<RecordingStateManager>,
        initial_config: ShortcutsConfig,
        translation: Arc<TranslationService>,
    ) -> Self {
        info!(
            "Starting KeyListener with initial config: push_to_record={:?}, hands_free={:?}",
            initial_config.push_to_record.keys, initial_config.hands_free.keys
        );

        let (mode_tx, mut mode_rx) = mpsc::channel(10);

        let thread_handle = thread::spawn(move || {
            let mut mode = ListenerMode::Normal {
                shortcuts: initial_config,
            };
            let mut pressed_keys: HashSet<u32> = HashSet::new();
            let mut translated_hold = TranslationHold::default();

            if let Err(err) = grab(move |event| {
                // Phase 1: Sync to latest mode from control channel
                Self::sync_mode(&mut mode, &mut mode_rx, &mut pressed_keys);
                if pressed_keys.is_empty() {
                    translated_hold = TranslationHold::default();
                }

                // Phase 2: Process event with fresh mode
                match &mode {
                    ListenerMode::Normal { shortcuts } => Self::handle_normal_mode(
                        event,
                        shortcuts,
                        &mut pressed_keys,
                        &command_tx,
                        &state_manager,
                        translation.is_enabled(),
                        &mut translated_hold,
                    ),
                    ListenerMode::Capture { app_handle } => {
                        Self::handle_capture_mode(event, app_handle)
                    }
                }
            }) {
                error!(
                    "Keyboard grab failed: {}. Keyboard shortcuts will not work.",
                    err
                );
            }
        });

        Self {
            _thread_handle: Some(thread_handle),
            mode_tx,
        }
    }

    /// Drain all pending mode updates from the control channel to ensure we always
    /// process events with the latest mode (avoids stale state)
    fn sync_mode(
        mode: &mut ListenerMode,
        mode_rx: &mut mpsc::Receiver<ListenerMode>,
        pressed_keys: &mut HashSet<u32>,
    ) {
        while let Ok(new_mode) = mode_rx.try_recv() {
            match &new_mode {
                ListenerMode::Normal { shortcuts } => {
                    info!(
                        "KeyListener mode updated: Normal (push_to_record={:?}, hands_free={:?})",
                        shortcuts.push_to_record.keys, shortcuts.hands_free.keys
                    );
                }
                ListenerMode::Capture { .. } => {
                    info!("KeyListener mode updated: Capture");
                }
            }
            *mode = new_mode;
            pressed_keys.clear(); // Reset on mode change
        }
    }

    /// Handle keyboard events in normal mode (shortcuts matching, recording triggers)
    fn handle_normal_mode(
        event: Event,
        shortcuts: &ShortcutsConfig,
        pressed_keys: &mut HashSet<u32>,
        command_tx: &mpsc::Sender<RecordingCommand>,
        state_manager: &Arc<RecordingStateManager>,
        translation_enabled: bool,
        translated_hold: &mut TranslationHold,
    ) -> Option<Event> {
        match event.event_type {
            EventType::KeyPress(key) => {
                let keycode = key.to_macos_keycode();

                // Check if shortcut was matched BEFORE inserting new key (rising edge detection)
                let was_push_to_record = shortcuts.push_to_record.matches(pressed_keys);
                let was_hands_free = shortcuts.hands_free.matches(pressed_keys);
                let translate = shortcuts.translate.as_ref().filter(|_| translation_enabled);
                let was_translate =
                    translate.is_some_and(|shortcut| shortcut.matches(pressed_keys));

                pressed_keys.insert(keycode);

                if !was_translate
                    && translate.is_some_and(|shortcut| shortcut.matches(pressed_keys))
                {
                    if state_manager.is_recording_locked() {
                        let _ = command_tx.blocking_send(RecordingCommand::StopRecording);
                    } else {
                        translated_hold.active = true;
                        translated_hold.stopped = false;
                        let _ =
                            command_tx.blocking_send(RecordingCommand::StartTranslatedRecording);
                    }
                    return None;
                }

                if translated_hold.active {
                    return None;
                }

                // Push-to-talk: Rising edge detected
                if !was_push_to_record && shortcuts.push_to_record.matches(pressed_keys) {
                    if state_manager.is_recording_locked() {
                        // Stop hands-free mode (push-to-talk can stop hands-free)
                        let _ = command_tx.blocking_send(RecordingCommand::StopRecording);
                    } else {
                        // Start push-to-talk recording
                        let _ = command_tx.blocking_send(RecordingCommand::StartRecording);
                    }
                }

                // Hands-free: Rising edge detected (toggle behavior)
                if !was_hands_free && shortcuts.hands_free.matches(pressed_keys) {
                    if state_manager.is_recording_locked() {
                        // Toggle off: Stop hands-free
                        let _ = command_tx.blocking_send(RecordingCommand::StopRecording);
                    } else {
                        // Toggle on: Start hands-free
                        let _ = command_tx.blocking_send(RecordingCommand::StartRecording);
                        let _ = command_tx.blocking_send(RecordingCommand::LockRecording);
                    }

                    // Swallow Space if it's in the combo
                    if shortcuts.hands_free.keys.iter().any(|k| k.keycode == 49) {
                        return None;
                    }
                }

                // Swallow all keys while push-to-record is active
                if translated_hold.active || shortcuts.push_to_record.matches(pressed_keys) {
                    return None;
                }

                Some(event)
            }
            EventType::KeyRelease(key) => {
                let keycode = key.to_macos_keycode();

                // Check push-to-record BEFORE removing key
                let was_push_to_record = shortcuts.push_to_record.matches(pressed_keys);
                let released_translation_key = shortcuts
                    .translate
                    .as_ref()
                    .is_some_and(|shortcut| shortcut.keys.iter().any(|key| key.keycode == keycode));

                pressed_keys.remove(&keycode);

                if translated_hold.active {
                    if released_translation_key
                        && !translated_hold.stopped
                        && !state_manager.is_recording_locked()
                    {
                        translated_hold.stopped = true;
                        let _ = command_tx.blocking_send(RecordingCommand::StopRecording);
                    }
                    if pressed_keys.is_empty() {
                        *translated_hold = TranslationHold::default();
                    }
                    return None;
                }

                // Release stops recording (unless locked)
                if was_push_to_record && !state_manager.is_recording_locked() {
                    let _ = command_tx.blocking_send(RecordingCommand::StopRecording);
                }

                Some(event)
            }
        }
    }

    /// Handle keyboard events in capture mode (emit to frontend, swallow all)
    fn handle_capture_mode(event: Event, app_handle: &AppHandle) -> Option<Event> {
        match event.event_type {
            EventType::KeyPress(key) => {
                let keycode = key.to_macos_keycode();
                let label = key.to_label();
                let _ = KeyCaptureEvent::KeyDown { keycode, label }.emit(app_handle);
            }
            EventType::KeyRelease(key) => {
                let keycode = key.to_macos_keycode();
                let label = key.to_label();
                let _ = KeyCaptureEvent::KeyUp { keycode, label }.emit(app_handle);
            }
        }

        // Swallow ALL events in capture mode (prevent Cmd+Q, etc.)
        None
    }

    /// Enter capture mode to configure shortcuts
    pub fn enter_capture_mode(&self, app_handle: AppHandle) -> Result<(), String> {
        info!("Sending mode change request: Capture");
        self.mode_tx
            .blocking_send(ListenerMode::Capture { app_handle })
            .map_err(|_| "KeyListener thread is not running".to_string())
    }

    /// Exit capture mode and return to normal mode with updated shortcuts
    pub fn exit_capture_mode(&self, shortcuts: ShortcutsConfig) -> Result<(), String> {
        info!(
            "Sending mode change request: Normal (push_to_record={:?}, hands_free={:?})",
            shortcuts.push_to_record.keys, shortcuts.hands_free.keys
        );
        self.mode_tx
            .blocking_send(ListenerMode::Normal { shortcuts })
            .map_err(|_| "KeyListener thread is not running".to_string())
    }

    /// Update shortcuts at runtime (no restart needed!)
    pub fn update_shortcuts(&self, new_config: ShortcutsConfig) -> Result<(), String> {
        info!(
            "Sending shortcuts update request: push_to_record={:?}, hands_free={:?}",
            new_config.push_to_record.keys, new_config.hands_free.keys
        );
        self.mode_tx
            .blocking_send(ListenerMode::Normal {
                shortcuts: new_config,
            })
            .map_err(|_| "KeyListener thread is not running".to_string())
    }

    /// Check if any shortcut uses Fn key (for globe key fix)
    pub fn uses_fn_key(config: &ShortcutsConfig) -> bool {
        let fn_code = 63u32;
        config
            .push_to_record
            .keys
            .iter()
            .any(|k| k.keycode == fn_code)
            || config.hands_free.keys.iter().any(|k| k.keycode == fn_code)
            || config
                .translate
                .as_ref()
                .is_some_and(|shortcut| shortcut.keys.iter().any(|key| key.keycode == fn_code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Shortcut, ShortcutKey};
    use dictara_keyboard::Key;

    #[test]
    fn translated_chord_promotes_normal_recording_and_stops_once() {
        let mut config = ShortcutsConfig::default();
        config.translate = Some(Shortcut {
            keys: [Key::Function, Key::ControlLeft]
                .into_iter()
                .map(|key| ShortcutKey {
                    keycode: key.to_macos_keycode(),
                    label: key.to_label(),
                })
                .collect(),
        });
        config.validate().unwrap();
        let (sender, mut receiver) = mpsc::channel(10);
        let manager = Arc::new(RecordingStateManager::new());
        let mut pressed = HashSet::new();
        let mut hold = TranslationHold::default();
        for event in [
            EventType::KeyPress(Key::Function),
            EventType::KeyPress(Key::ControlLeft),
            EventType::KeyPress(Key::Space),
            EventType::KeyRelease(Key::ControlLeft),
            EventType::KeyRelease(Key::Space),
            EventType::KeyPress(Key::Space),
            EventType::KeyRelease(Key::Function),
            EventType::KeyRelease(Key::Space),
        ] {
            assert!(KeyListener::handle_normal_mode(
                Event::new(event),
                &config,
                &mut pressed,
                &sender,
                &manager,
                true,
                &mut hold
            )
            .is_none());
        }
        assert!(matches!(
            receiver.try_recv().unwrap(),
            RecordingCommand::StartRecording
        ));
        assert!(matches!(
            receiver.try_recv().unwrap(),
            RecordingCommand::StartTranslatedRecording
        ));
        assert!(matches!(
            receiver.try_recv().unwrap(),
            RecordingCommand::StopRecording
        ));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn disabled_translation_shortcut_never_requests_translation() {
        let mut config = ShortcutsConfig::default();
        config.translate = Some(Shortcut {
            keys: [Key::Function, Key::ControlLeft]
                .into_iter()
                .map(|key| ShortcutKey {
                    keycode: key.to_macos_keycode(),
                    label: key.to_label(),
                })
                .collect(),
        });
        let (sender, mut receiver) = mpsc::channel(10);
        let manager = Arc::new(RecordingStateManager::new());
        let mut pressed = HashSet::new();
        let mut hold = TranslationHold::default();
        for key in [Key::Function, Key::ControlLeft] {
            KeyListener::handle_normal_mode(
                Event::new(EventType::KeyPress(key)),
                &config,
                &mut pressed,
                &sender,
                &manager,
                false,
                &mut hold,
            );
        }
        assert!(matches!(
            receiver.try_recv().unwrap(),
            RecordingCommand::StartRecording
        ));
        assert!(receiver.try_recv().is_err());
        assert!(!hold.active);
    }
}
