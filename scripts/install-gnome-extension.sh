#!/usr/bin/env bash
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
extension_dir="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/sys-mon@jcaple.dev"
cd "$project_dir"
cargo build --release --locked
install -d "$extension_dir" "$extension_dir/icons"
for source in extension.js panel-format.js metadata.json stylesheet.css; do
    install -m644 "gnome-extension/$source" "$extension_dir/$source"
done
install -m644 gnome-extension/icons/*-symbolic.svg "$extension_dir/icons/"
install -m755 target/release/sys-mon "$extension_dir/sys-mon"
printf 'Installed sys-mon in %s\n' "$extension_dir"
printf 'Enable with: gnome-extensions enable sys-mon@jcaple.dev\n'
printf 'If GNOME has not discovered it yet, sign out and back in first.\n'
