# Contributing to Dictara

Thank you for your interest in contributing to Dictara! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- **Node.js** (LTS version 20.19+ or 22.12+)
- **Rust** (stable) - [Install via rustup](https://rustup.rs)
- **npm**
- **CMake** (3.x) - Required to build the local transcription engines
- **Xcode with Swift 6.2+ and Metal Toolchain** - Required for Qwen on Apple Silicon

#### Installing Rust (Required)

If you don't have Rust installed, run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, restart your terminal or run:

```bash
source "$HOME/.cargo/env"
```

Verify the installation:

```bash
cargo --version
rustc --version
```

### Stopping the Production App

If you have Dictara installed from the official release, quit it before running the development version to avoid conflicts:

1. Click the Dictara icon in your menu bar
2. Select **Quit**

Or from the terminal:

```bash
pkill -x dictara
```

### Development Setup

1. Fork and clone the repository
2. Install dependencies:
   ```bash
   npm install
   ```
3. Run the development server:
   ```bash
   npm run tauri dev
   ```
   
   > **Note:** The first build takes 5-10 minutes as it compiles ~500 Rust crates. Subsequent builds are much faster.

4. The app will open automatically. You may need to:
   - Grant **Accessibility permissions** in System Settings → Privacy & Security → Accessibility
   - Configure an **OpenAI or Azure API key** in the app's preferences

### Local Release Build

Quit the official app before launching your local build, then run:

```bash
npm run release:local
```

This builds and opens `target/release/bundle/macos/Dictara.app`. You can use the
app without a development server or VS Code running. The local app uses the
`app.dictara.dev` identifier, with separate settings and macOS permissions.

Official updates and signed updater artifacts are disabled in this checkout.
After changing the code, run `npm run verify` and rebuild your local app.

### Qwen3 Local Transcription

On Apple Silicon with macOS 14+, the build automatically compiles and bundles a
native MLX Swift engine. No Python installation or separate server is required.
Initialize Xcode and install its Metal compiler before the first build:

```bash
xcodebuild -runFirstLaunch
xcodebuild -downloadComponent MetalToolchain
```

In Preferences, select Local (Offline), download Qwen3-ASR 1.7B (MLX 8-bit), then
enable it. The download is approximately 2.47 GB and every file is verified with
SHA-256 against a pinned model revision. The memory estimate in the UI is
approximate and depends on audio length. Whisper and Parakeet remain available.

Transcription loads only local model files, with automatic language detection,
including French. The engine stays loaded between recordings and is stopped
when the model is unloaded. Qwen is not offered on Intel Macs.

Run the focused backend tests with `cargo test -p dictara models:: --lib`.
To test real offline inference and process recovery, supply a downloaded model
directory and a 16kHz mono WAV saying "Bonjour, ceci est un test de transcription locale":

```bash
DICTARA_QWEN_MODEL_DIR="/path/to/qwen3-asr-1.7b-8bit" \
DICTARA_QWEN_TEST_AUDIO="/path/to/test.wav" \
cargo test -p dictara transcribes_offline_and_restarts_after_engine_exit --lib -- --ignored
```

### Optional French-to-English Dictation

In Preferences > JIT Translation, download Qwen3 4B (MLX 4-bit), enable
Translate to English, and assign its English dictation shortcut. Hold the
shortcut to record and release to transcribe, translate, and paste English.
The normal recording shortcuts remain unchanged.

Translation is disabled by default. Downloading its weights does not load them.
When enabled, its separate native process stays loaded alongside the speech
model, including after restarting Dictara. Disabling translation stops that
process; deleting its weights also disables it. Inference is entirely local.

The preferences show the total estimated RAM for loaded/loading models and a
per-model breakdown. These are approximate model estimates, not measured
application memory; temporary buffers and application overhead are excluded.
Translation failures do not paste the original French text, and Retry keeps
the English output mode.

The translation model download is approximately 2.28 GB and is pinned and
SHA-256 verified. To exercise real inference and verify process shutdown:

```bash
DICTARA_TRANSLATION_MODELS_DIR="/path/to/models" \
cargo test -p dictara translates_offline_and_unloads_when_disabled --lib -- --ignored
```

## Development Workflow

### Before Submitting Changes

Always run the verification script before submitting:

```bash
npm run verify
```

This checks for TypeScript errors, linting issues, and runs tests.

### Code Style

- **TypeScript/JavaScript**: Follow the existing code style. ESLint will catch most issues.
- **Rust**: Use `cargo fmt` for formatting and `cargo clippy` for linting.

### Commit Messages

- Use clear, descriptive commit messages
- Start with a verb (Add, Fix, Update, Remove, etc.)
- Keep the first line under 72 characters

### Pull Requests

1. Create a new branch from `main`
2. Make your changes
3. Run `npm run verify` to ensure everything passes
4. Push your branch and create a Pull Request
5. Fill out the PR template
6. Wait for CI checks to pass
7. Request a review

## Troubleshooting

### Resetting Microphone Permissions

If you need to reset microphone permissions (e.g., to test the onboarding flow or fix permission issues), use the following commands:

**For the released version (production app):**
```bash
tccutil reset Microphone app.dictara
```

**For the dev version (running from VS Code):**
```bash
tccutil reset Microphone com.microsoft.VSCode
```

> **Note:** This removes the app from macOS microphone permissions. The system will prompt you again the next time the app requests microphone access.

To manually grant permissions, open System Settings:
```bash
open "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
```

## Reporting Issues

### Bug Reports

When reporting bugs, please include:

- Dictara version
- Operating system and version
- Steps to reproduce
- Expected vs actual behavior
- Screenshots if applicable

### Feature Requests

We welcome feature requests! Please:

- Check existing issues first
- Clearly describe the feature
- Explain the use case

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Questions?

Feel free to open a [Discussion](https://github.com/vitalii-zinchenko/dictara/discussions) for questions or ideas.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
