# vViewer

**High-performance video viewer for large collections.**

Handles folders with thousands of video files without slowdown. Built in Rust with [Tauri](https://tauri.app) for a lightweight native binary (~8 MB), with a virtual-scroll masonry grid that only decodes videos currently on screen.

---

## Key improvements over videoswarm

| | videoswarm | vViewer |
|---|---|---|
| Backend | Node.js / Electron | Rust (Tauri) |
| Directory scan | JS single-thread | Rust + rayon parallel |
| Grid DOM nodes | All files | Visible only (virtual scroll) |
| Video decoders | All visible | Viewport + 1 buffer screen |
| Memory growth | Linear with file count | Constant with file count |
| Install size | ~200 MB | ~8 MB |

---

## Features

- **Virtual masonry grid** — renders O(visible) tiles regardless of total count
- **Parallel scanner** — rayon thread pool, fast even on 10 000+ file folders
- **Hardware-accelerated playback** — native WebView `<video>` (WebView2 / WKWebView / WebKitGTK)
- **Zoom control** — adjust column width from 120 px to 520 px
- **Search** — real-time filename filter
- **Sort** — by date, name, size, or rating
- **5-star rating** — stored in SQLite, persists across sessions
- **File watcher** — auto-refreshes when files are added or removed
- **Mute toggle** — unmute all playing videos at once
- **Context menu** — Show in file manager, copy path, set rating

---

## Install

### One command

**Linux / macOS**
```sh
curl -sSfL https://raw.githubusercontent.com/dikmri/vViewer/main/install.sh | sh
```

**Windows (PowerShell)**
```powershell
irm https://raw.githubusercontent.com/dikmri/vViewer/main/install.ps1 | iex
```

### Manual download

Download the latest release from the [Releases page](https://github.com/dikmri/vViewer/releases):

| Platform | File |
|---|---|
| Windows | `vViewer_*_x64_en-US.msi` |
| macOS (Universal) | `vViewer_*_universal.dmg` |
| Linux | `vViewer_*_amd64.AppImage` or `.deb` |

> **macOS note:** Right-click → Open on first launch to bypass Gatekeeper (no code-signing yet).

---

## Build from source

Prerequisites: [Rust](https://rustup.rs) and [Node.js](https://nodejs.org) (LTS).

```sh
git clone https://github.com/dikmri/vViewer
cd vViewer
npm install
npm run build          # produces release binary in src-tauri/target/release/bundle/
```

For development with hot-reload:
```sh
npm run dev
```

---

## Architecture

```
vViewer/
├── src-tauri/          Rust backend (Tauri)
│   └── src/
│       ├── lib.rs          Tauri commands & app setup
│       ├── scanner.rs      Parallel directory scanner (rayon + walkdir)
│       ├── database.rs     SQLite (rusqlite, WAL mode)
│       ├── video_server.rs Axum HTTP server — streams files with Range support
│       └── watcher.rs      fs-event watcher (notify crate)
└── ui/                 Static HTML/JS/CSS frontend
    ├── index.html
    ├── js/
    │   ├── app.js          Tauri commands, toolbar, context menu
    │   └── grid.js         Virtual masonry grid
    └── css/main.css
```

### Why a local HTTP server for video?

The built-in browser `<video>` element needs HTTP range requests to seek within files. The Tauri asset protocol supports this, but for maximum reliability across all three platforms we spin up an Axum HTTP server (`127.0.0.1:<random port>`) at startup. This also makes it trivial to stream any path the user opens, without security-scope configuration.

---

## License

MIT
