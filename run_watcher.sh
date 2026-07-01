#!/bin/bash

# Watcher Script for Audio Selector
# Runs the watcher utility located in src/bin/watcher.rs

echo "🚀 Starting Audio Selector Watcher..."

# Check if watcher.rs exists
if [ ! -f "src/bin/watcher.rs" ]; then
    echo "❌ Error: src/bin/watcher.rs not found!"
    exit 1
fi

# Run the watcher using cargo
# The --bin watcher flag points to src/bin/watcher.rs
cargo run --bin watcher
