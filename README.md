# PortDrill

A lightweight desktop app for managing SSH port forwarding rules. 
<br>Define Local (`-L`), Remote (`-R`), and Dynamic (`-D`) forwards, group multiple forwards under a single SSH connection, and toggle them on or off.

Built with Tauri v2, Svelte 5, and Rust. Runs on Windows, macOS, and Linux.

## Features

- **Multi-forward rules** — one SSH connection carries multiple forwards, just like `ssh -N -L ... -L ... -R ...`
- **Toggle on/off** — connect and disconnect tunnels with a click
- **Health monitoring** — detects when an SSH process dies and updates the status
- **System tray** — minimizes to tray on macOS and Windows, keeping tunnels alive in the background
- **Config persistence** — rules are saved to a JSON file and restored on launch
- **SSH key auth** — point to a key file per rule, or leave empty to use your default key

## Installation

Download the latest release for your platform from the [Releases](../../releases) page.

### Windows

Run the `.msi` installer. No additional setup needed.

### macOS

Open the `.dmg` and drag PortDrill to Applications. The app is not code-signed, so macOS Gatekeeper will block it on first launch. To fix this, open Terminal and run:

```bash
xattr -cr /Applications/PortDrill.app
```

Then open PortDrill normally.

### Linux

Install the `.deb` package or run the `.AppImage` directly.

## Development

Prerequisites: [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 22+, and platform-specific dependencies.

**Linux (Ubuntu/Debian):**
```bash
sudo apt install libwebkit2gtk-4.1-dev libssl-dev pkg-config build-essential \
  libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libdbus-1-dev
```

**macOS/Windows:** No extra dependencies — WebKit and WebView2 are built in.

```bash
npm install
npm run tauri dev
```
