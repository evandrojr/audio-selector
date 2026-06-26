# Audio Selector Specification

## Overview
A Rust-based desktop application using the Slint GUI framework to manage system audio devices (Input and Output) on Linux systems (PulseAudio/PipeWire).

## Features
- **Real-time Audio Switching:**
  - **No Apply Button:** Device changes happen immediately when selected in the dropdown.
  - **Force Move:** Uses `pactl move-sink-input` to force currently playing audio to the new device (e.g., HDMI, Bluetooth).
- **Advanced Bluetooth Control:**
  - **Auto Connection:** Lists paired (even if disconnected) Bluetooth devices. Selecting one attempts to `connect` via `bluetoothctl` and then switches audio to it.
  - **Hide Unknown MACs:** Feature to filter out Bluetooth devices consisting of just MAC addresses.
- **Advanced Options Tab:**
  - **Excluded Devices:** A robust Checkbox list to hide unwanted outputs/inputs from the main dropdowns.
- **Persistence & UI:**
  - **Config Path:** Configuration saved to `~/.config/audio-selector/config.json`.
  - **Window Memory:** Saves and loads position and dimensions.
  - **Multi-language:** Supports 3 languages based on system locale (EN, PT, ES, FR, DE, IT).
- **Debugging:**
  - **Separate Log Window:** Searchable diagnostic window for troubleshooting (logs saved to `~/.config/audio-selector/debug.log`).

## Technical Stack
- **GUI:** Slint 1.8.0.
- **System Backend:** `pactl` (PulseAudio/PipeWire) and `bluetoothctl`.
- **System Integration:** `dirs`, `sys-locale`.
- **Logging:** File-based logging to `~/.config/audio-selector/debug.log`.

## Installation & Autostart
Run the binary with the `-install` flag to copy the app to your `PATH` and set it to start with your window manager:
```bash
./audio-selector -install
```
This automatically handles `.desktop` files in `~/.local/share/applications` and `~/.config/autostart/`.
