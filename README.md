# vViewer

**大量の動画ファイルを高速に閲覧するためのビューアです。**

数千〜数万ファイルのフォルダでもパフォーマンスが落ちません。
Rust製バックエンド（[Tauri](https://tauri.app)）と仮想スクロールのマソングリッドにより、画面に表示されている動画だけをデコードします。

> **High-performance video viewer for large collections.**  
> Handles thousands of files without slowdown. Rust/Tauri backend with virtual masonry grid.

---

## videoswarm との違い

| | videoswarm | vViewer |
|---|---|---|
| バックエンド | Node.js / Electron | Rust (Tauri) |
| ディレクトリスキャン | JS シングルスレッド | Rust + rayon 並列処理 |
| グリッドの DOM ノード数 | ファイル総数分 | 表示中のみ（仮想スクロール） |
| 映像デコーダ数 | 表示中すべて | ビューポート ± バッファ 1画面 |
| メモリ増加 | ファイル数に比例 | 表示数に比例（ほぼ一定） |
| バイナリサイズ | 約 200 MB | 約 8 MB |

---

## 機能

- **仮想マソングリッド** — 全ファイル数によらず DOM ノード数は表示中の数だけ
- **並列スキャナー** — Rayon スレッドプール。10,000 ファイル超のフォルダも高速
- **ハードウェアアクセラレーション再生** — ネイティブ WebView の `<video>` タグ使用
  （Windows: WebView2、macOS: WKWebView、Linux: WebKitGTK）
- **ズームコントロール** — カラム幅 120px〜520px を自由に調整
- **リアルタイム検索** — ファイル名フィルター
- **並べ替え** — 日付・名前・サイズ・評価
- **5段階評価** — SQLite に永続保存
- **ファイル監視** — ファイルの追加・削除を自動検出してグリッドを更新
- **ミュート切り替え** — 全動画を一括ミュート/ミュート解除
- **コンテキストメニュー** — ファイルマネージャーで開く・パスコピー・評価設定
- **ダブルクリックでフルスクリーン再生**

---

## インストール

### ワンコマンド（推奨）

**Linux / macOS**
```sh
curl -sSfL https://raw.githubusercontent.com/dikmri/vViewer/main/install.sh | sh
```

**Windows（PowerShell）**
```powershell
irm https://raw.githubusercontent.com/dikmri/vViewer/main/install.ps1 | iex
```

### 手動ダウンロード

[リリースページ](https://github.com/dikmri/vViewer/releases) から最新版をダウンロード:

| OS | ファイル |
|---|---|
| Windows | `vViewer_*_x64_en-US.msi` |
| macOS（Universal） | `vViewer_*_universal.dmg` |
| Linux | `vViewer_*_amd64.AppImage` または `.deb` |

> **macOS 注意:** コード署名なしのため、初回起動は右クリック→「開く」を選んでください（Gatekeeper 回避）。

---

## ソースからビルド

事前に [Rust](https://rustup.rs) と [Node.js LTS](https://nodejs.org) が必要です。

```sh
git clone https://github.com/dikmri/vViewer
cd vViewer
npm install
npm run build   # src-tauri/target/release/bundle/ にバイナリが生成されます
```

開発用（ホットリロード）:
```sh
npm run dev
```

---

## アーキテクチャ

```
vViewer/
├── src-tauri/              Rust バックエンド（Tauri v2）
│   └── src/
│       ├── lib.rs          Tauri コマンド & アプリ初期化
│       ├── scanner.rs      並列ディレクトリスキャン（rayon + walkdir）
│       ├── database.rs     SQLite（rusqlite、WAL モード）
│       ├── video_server.rs Axum HTTP サーバー（Range リクエスト対応）
│       └── watcher.rs      ファイルシステム監視（notify クレート）
└── ui/                     静的フロントエンド（バンドル不要）
    ├── index.html
    ├── js/
    │   ├── app.js          コマンド呼び出し・ツールバー・コンテキストメニュー
    │   └── grid.js         仮想マソングリッド
    └── css/main.css
```

### ローカル HTTP サーバーを使う理由

ブラウザの `<video>` 要素がシーク（任意の位置への移動）をするには HTTP Range リクエストが必要です。Tauri のアセットプロトコルでも対応可能ですが、3プラットフォーム全てで確実に動作させるため、起動時に Axum HTTP サーバー（`127.0.0.1:ランダムポート`）を立ち上げてファイルをストリーミングしています。

---

## ライセンス

MIT

---

<details>
<summary>English</summary>

## Features
- Virtual masonry grid — O(visible) DOM nodes regardless of total file count
- Parallel scanner — rayon thread pool, fast even for 10,000+ file folders
- Hardware-accelerated playback via native WebView `<video>`
- Zoom control, real-time search, sort, 5-star rating (SQLite), file watcher
- Mute toggle, context menu, double-click fullscreen

## Install
**Linux / macOS:** `curl -sSfL https://raw.githubusercontent.com/dikmri/vViewer/main/install.sh | sh`  
**Windows:** `irm https://raw.githubusercontent.com/dikmri/vViewer/main/install.ps1 | iex`

## Build from source
Requires Rust and Node.js LTS.
```sh
git clone https://github.com/dikmri/vViewer && cd vViewer
npm install && npm run build
```
</details>
