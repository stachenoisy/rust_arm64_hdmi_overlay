<h1 align="center">ARM64 HDMI Overlay</h1>

<p align="center">
  <a href="https://github.com/stachenoisy/rust_arm64_hdmi_overlay">
    <img src="https://img.shields.io/github/stars/stachenoisy/rust_arm64_hdmi_overlay" alt="GitHub Repo stars">
  </a>
  <a href="https://github.com/stachenoisy/rust_arm64_hdmi_overlay/commits/main">
    <img src="https://img.shields.io/github/last-commit/stachenoisy/rust_arm64_hdmi_overlay" alt="GitHub Last Commit">
  </a>
  <a href="https://github.com/stachenoisy/rust_arm64_hdmi_overlay/commits/main">
    <img src="https://img.shields.io/github/commit-activity/t/stachenoisy/rust_arm64_hdmi_overlay" alt="GitHub Total Commit">
  </a>
  <a href="https://opensource.org/license/gpl-3.0">
    <img src="https://img.shields.io/github/license/stachenoisy/rust_arm64_hdmi_overlay" alt="License: GNU General Public License 3.0">
  </a>
</p>

A lightweight, memory-efficient WebKit2GTK overlay written in Rust for embedded Linux platforms (e.g. Rockchip ARM64). Designed for bare X11/xinit setups with automated backend health checks, seamless fallback routing, and zero overhead.

## Features

- **Bare X11 Fullscreen Geometry**: Automatically detects primary display boundaries and scales without requiring a window manager (Openbox, XFWM, etc.).
- **Smart Fallback Lifecycle**: Displays an instant local boot splash (`boot.html`), routes to the web backend once healthy, and falls back to `error.html` on service failures.
- **GLIBC 2.28 Compatibility**: Pre-configured cross-compilation pipeline tailored for legacy embedded distributions (Linaro Debian Buster).
- **Lightweight Runtime Footprint**: Zero async runtime overhead uses synchronous worker threads and native GLib event dispatchers.
- **Hardware Acceleration Ready**: Forces WebKit compositing modes optimized for embedded Mali/Rockchip GPUs.

---

## State Transition Flow

```text
       System Startup
             │
             ▼
      [ Load Boot UI ] ── (boot.html)
             │
      Poll Backend (10s window)
             │
    ┌────────┴────────┐
    ▼                 ▼
[ Online ]       [ Offline ]
    │                 │
    ▼                 ▼
(http://127.0.0.1:5566) (error.html)
    │                 ▲
    └─► Continuous ───┘
        Health Check (2s)
```

## Building & Cross-Compilation
### Prerequisites
#### Ensure you have Rust and `cross` installed on your host system:

```bash
cargo install cross --git https://github.com/cross-rs/cross
```

### Cross-Compile for ARM64 (Debian Buster / GLIBC 2.28)
#### Run the cross-compilation target configured in `Cross.toml`:

```bash
cross build --target aarch64-unknown-linux-gnu --release
```
#### The resulting binary will be located at:

```text
target/aarch64-unknown-linux-gnu/release/nvr-overlay
```

## Deployment & Systemd Setup

### 1. Directory Structure & Assets
#### Prepare the required runtime directories and static fallback assets on the target device:

```bash
# Create application directories on target
ssh root@<target-ip> "mkdir -p /opt/interface/static/htmls"

# Copy HTML fallback pages
scp htmls/boot.html root@<target-ip>:/opt/interface/static/htmls/
scp htmls/error.html root@<target-ip>:/opt/interface/static/htmls/
```

#### Target filesystem layout:
```text
/opt/interface/
├── overlay                  # Compiled application binary
├── launch.sh                # X11 startup wrapper
└── static/
    └── htmls/
        ├── boot.html        # Initial splash screen shown on boot
        └── error.html       # Fallback screen displayed on backend failure
```

### 2. File Placement
#### Copy the compiled binary to the target device and ensure executable permissions:

```bash
scp target/aarch64-unknown-linux-gnu/release/arm64-hdmi-overlay root@<target-ip>:/opt/interface/overlay
chmod +x /opt/interface/overlay
```

### 3. Launch Script (`/opt/interface/launch.sh`)

```bash
#!/bin/bash
export DISPLAY=:0
xset s off -dpms
exec xinit /opt/interface/overlay -- :0 vt1
```

#### Make the script executable:

```bash
chmod +x /opt/interface/launch.sh
```

### 4. Systemd Service Setup (`/etc/systemd/system/hdmi-overlay.service`)

```ini
[Unit]
Description=HDMI Overlay Service
After=network-online.target backend.service
Wants=network-online.target

[Service]
Type=simple
User=root
ExecStart=/bin/bash /opt/interface/launch.sh
Restart=always
RestartSec=3
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

#### Enable and start the service:

```bash
systemctl daemon-reload
systemctl enable --now hdmi-overlay.service
```

### Target Runtime Requirements
#### Ensure the target root filesystem contains the following dynamic libraries:

- `libgtk-3.so.0`
- `libwebkit2gtk-4.0.so.37`
- `libsoup-2.4.so.1`
- `libjavascriptcoregtk-4.0.so.18`
- `xorg / xinit`