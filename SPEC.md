# Audio Selector Specification

## Overview
A Rust-based desktop application using the Slint GUI framework to manage system audio devices (Input and Output) on Linux systems (PulseAudio/PipeWire).

## Features
- **Multi-language Support:**
  - Automatic locale detection (system language).
  - Supported languages: English (default), Portuguese.
- **Real-time Audio Switching:**
  - **No Apply Button:** Device changes happen immediately when selected in the dropdown.
  - **Force Move:** Uses `pactl move-sink-input` to force currently playing audio to the new device (e.g., HDMI, Bluetooth).
  - **Unified Mode:** Toggle switch to use the same device for input/output.
- **Advanced Bluetooth Control:**
  - **Bluetooth Switch:** Real-time toggle for Bluetooth power.
  - **Hardware Detection:** Switch is disabled if no Bluetooth controller is found.
  - **Force Connection:** Lists paired (even if disconnected) Bluetooth devices. Selecting one attempts to `connect` via `bluetoothctl` and then switches audio to it.
- **Persistence:** Saves and loads settings (Unified Mode, Bluetooth state, last selected devices) to/from `config.json`.
- **User Interface:**
  - Modern Slint `Switch` widgets.
  - Application Icon included in the window.
  - Clean design without redundant headers.

## Technical Stack
- **Language:** Rust
- **GUI:** Slint 1.8.0
- **System Backend:** `pactl` (PulseAudio/PipeWire) and `bluetoothctl`.
- **Localization:** `sys-locale` for detection.
- **Storage:** `serde_json` for persistence.
- **Time/Status:** `chrono` for timestamped feedback.

## Implementation Details
- **Immediate Apply:** Rust callbacks trigger both `set-default-*` and `move-sink-input` for instant results.
- **Bluetooth Fix:** Paired devices from `bluetoothctl` are merged into the sink list. Selecting one triggers an automatic connection attempt.
- **Icon:** Embedded PNG icon in the `ui/assets` directory.
