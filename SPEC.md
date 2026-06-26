# Audio Selector Specification

## Overview
A Rust-based desktop application using the Slint GUI framework to manage system audio devices (Input and Output) on Linux systems (PulseAudio/PipeWire).

## Features
- **Real-time Audio Switching:**
  - **No Apply Button:** Device changes happen immediately when selected in the dropdown.
  - **Force Move:** Uses `pactl move-sink-input` to force currently playing audio to the new device.
- **Advanced Bluetooth Control:**
  - **Auto Connection:** Lists paired Bluetooth devices. Selecting one attempts to `connect` via `bluetoothctl`.
  - **Non-blocking Power Toggle:** Bluetooth power commands run in a background thread to prevent UI freezes.
  - **Hide Unknown MACs:** Filter out Bluetooth devices consisting of just MAC addresses.
- **Advanced Options Tab:**
  - **Excluded Devices:** Robust Checkbox list to hide unwanted devices.
- **Performance & Persistence:**
  - **Device Caching:** Stores the last known device list in config. On startup, the UI displays cached devices immediately while a background scan updates the list.
  - **Config Path:** Configuration saved to `~/.config/audio-selector/config.json`.
  - **Window Memory:** Saves and loads position and dimensions.
  - **Multi-language:** Supports EN, PT, ES, FR, DE, IT.
- **Debugging:**
  - **Separate Log Window:** Searchable diagnostic window (logs saved to `~/.config/audio-selector/debug.log`).
- **System Tray:**
  - **Multi-backend:** Uses `tray-icon` crate with GTK + libappindicator + libxdo backends. Works across GNOME, KDE, and other WMs.

## Architecture
The application is modularized for better maintainability:
- `audio`: Handles `pactl` commands and device parsing.
- `bluetooth`: Manages `bluetoothctl` integration.
- `config`: Handles JSON persistence and device caching.
- `i18n`: Manages multi-language translations.
- `utils`: Shared utility functions (logging, paths).

## Technical Stack
- **GUI:** Slint 1.8.0.
- **System Backend:** `pactl` and `bluetoothctl`.
- **Persistence:** `serde` / `serde_json`.

## Installation & Autostart
Run with the `-install` flag:
```bash
./audio-selector -install
```
This handles `.desktop` files and icon caching.
