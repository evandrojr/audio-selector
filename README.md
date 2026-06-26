# Audio Selector

A modern, fast, and lightweight audio device switcher written in Rust with a Slint GUI. Designed to handle system audio inputs and outputs with a focus on Bluetooth device management.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey.svg)

## Features

- **Instant Switching:** No "Apply" button needed. Changes are immediate.
- **Bluetooth Integration:** 
  - Real-time Bluetooth power control.
  - Automatic connection attempt when selecting a paired but disconnected device.
- **Unified Mode:** Toggle to use the same device for both input and output with one click.
- **Persistence:** Automatically saves and restores your last used configuration (Bluetooth state, Unified mode, and selected devices).
- **Multilingual:** Automatically detects system language (English and Portuguese supported).
- **Lightweight:** Minimal CPU/Memory footprint.

## Requirements

### Linux
- **Audio Backend:** PulseAudio or PipeWire (with `pactl` installed).
- **Bluetooth:** `bluetoothctl`.
- **System Libs:** `libfontconfig1-dev`, `libx11-dev`.

### Windows
- Standard Windows Audio service support.
- Bluetooth management via Windows native API (handled by the OS).

## Installation

### From Binaries
Download the latest version from the [Releases](https://github.com/youruser/audio-selector/releases) page.

### From Source
Ensure you have [Rust](https://rustup.rs/) installed.

```bash
git clone https://github.com/youruser/audio-selector.git
cd audio-selector
cargo run --release
```

## Technical Details

- **Backend:** Rust 1.80+ (Edition 2021/2024 compatible).
- **GUI:** [Slint](https://slint.dev/) framework.
- **State Management:** JSON-based configuration storage.

## Development

To build for all platforms using GitHub Actions, simply push to the `main` branch. The CI will generate binaries for Linux and Windows.

---
Created with ❤️ by Gemini CLI.
