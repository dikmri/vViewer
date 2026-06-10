import { MasonryGrid } from './grid.js';

// ── Tauri bridge ──────────────────────────────────────────────────────────────
const invoke = window.__TAURI__?.core?.invoke
  ?? ((cmd, args) => { console.warn('invoke stub:', cmd, args); return Promise.resolve(null); });

const listen = window.__TAURI__?.event?.listen
  ?? (() => Promise.resolve(() => {}));

// Expose for grid.js
window.__tauri_invoke = invoke;

// ── App state ─────────────────────────────────────────────────────────────────
let grid          = null;
let allFiles      = [];
let filtered      = [];
let currentFolder = null;
let ctxTarget     = null;   // path string for context-menu

// ── Update bar ────────────────────────────────────────────────────────────────
function bindUpdateBar() {
  const bar      = document.getElementById('update-bar');
  const msg      = document.getElementById('update-msg');
  const progWrap = document.getElementById('update-progress-wrap');
  const progBar  = document.getElementById('update-progress-bar');
  const progTxt  = document.getElementById('update-progress-text');
  const nowBtn   = document.getElementById('update-now-btn');
  const laterBtn = document.getElementById('update-later-btn');

  function showBar() {
    bar.classList.add('visible');
    const h = bar.offsetHeight;
    document.documentElement.style.setProperty('--update-bar-h', h + 'px');
  }

  function hideBar() {
    bar.classList.remove('visible');
    document.documentElement.style.setProperty('--update-bar-h', '0px');
  }

  listen('update-available', (e) => {
    const { version, body } = e.payload;
    msg.textContent = `新しいバージョン v${version} が利用可能です。${body ? '  ' + body.split('\n')[0] : ''}`;
    progWrap.style.display = 'none';
    nowBtn.disabled = false;
    nowBtn.textContent = '今すぐ更新';
    showBar();
  });

  listen('update-progress', (e) => {
    const pct = e.payload;
    progWrap.style.display = 'flex';
    progBar.style.setProperty('--pct', pct + '%');
    progTxt.textContent = pct + '%';
  });

  nowBtn.addEventListener('click', async () => {
    nowBtn.disabled = true;
    nowBtn.textContent = 'ダウンロード中…';
    msg.textContent = 'アップデートをダウンロード中…';
    progWrap.style.display = 'flex';
    try {
      await invoke('install_update');
      // app restarts — code below won't run
    } catch (err) {
      msg.textContent = 'アップデート失敗: ' + err;
      nowBtn.disabled = false;
      nowBtn.textContent = '今すぐ更新';
    }
  });

  laterBtn.addEventListener('click', hideBar);
}

// ── Keyboard shortcuts ────────────────────────────────────────────────────────
function bindKeyboard() {
  window.addEventListener('keydown', (e) => {
    // Don't capture when typing in an input
    if (document.activeElement?.tagName === 'INPUT') return;

    const amount = Math.round(window.innerHeight * 0.6);
    if (e.key === 'j' || e.key === 'J') {
      e.preventDefault();
      window.scrollBy({ top: amount, behavior: 'smooth' });
    } else if (e.key === 'k' || e.key === 'K') {
      e.preventDefault();
      window.scrollBy({ top: -amount, behavior: 'smooth' });
    }
  });
}

// ── Init ──────────────────────────────────────────────────────────────────────
async function init() {
  const port = await invoke('get_video_port');
  window.__videoPort = port;

  bindToolbar();
  bindContextMenu();
  bindUpdateBar();
  bindKeyboard();
  listenFileWatcher();

  showEmpty(true);
}

// ── Toolbar ───────────────────────────────────────────────────────────────────
function bindToolbar() {
  document.getElementById('btn-open').addEventListener('click', openFolder);
  document.getElementById('btn-open-empty')?.addEventListener('click', openFolder);

  const searchEl = document.getElementById('search');
  searchEl.addEventListener('input', debounce(() => applyFilter(searchEl.value), 160));

  const zoomEl  = document.getElementById('zoom-slider');
  const zoomLbl = document.getElementById('zoom-value');
  zoomEl.addEventListener('input', () => {
    const v = parseInt(zoomEl.value);
    zoomLbl.textContent = v + 'px';
    grid?.setColWidth(v);
  });

  document.getElementById('sort-select').addEventListener('change', (e) => {
    sortInPlace(filtered, e.target.value);
    grid?.setItems(filtered);
  });

  const muteBtn = document.getElementById('mute-btn');
  let muted = true;
  muteBtn.addEventListener('click', () => {
    muted = !muted;
    muteBtn.textContent = muted ? '🔇' : '🔊';
    grid?.setMute(muted);
  });
}

async function openFolder() {
  const folder = await invoke('pick_folder');
  if (!folder) return;

  currentFolder = folder;
  setCount('Scanning…');
  showEmpty(false);

  invoke('watch_folder', { path: folder });

  const files = await invoke('scan_folder', { path: folder });
  loadFiles(files);
}

// ── File loading / filtering ──────────────────────────────────────────────────
function loadFiles(files) {
  allFiles = files;
  applyFilter(document.getElementById('search').value);
}

function applyFilter(query) {
  const q = query.trim().toLowerCase();
  filtered = q
    ? allFiles.filter(f => f.filename.toLowerCase().includes(q))
    : allFiles.slice();

  const sortBy = document.getElementById('sort-select').value;
  sortInPlace(filtered, sortBy);
  renderGrid();
}

function sortInPlace(arr, by) {
  arr.sort((a, b) => {
    switch (by) {
      case 'filename': return a.filename.localeCompare(b.filename);
      case 'modified': return b.modified - a.modified;
      case 'size':     return b.size - a.size;
      case 'rating':   return (b.rating ?? 0) - (a.rating ?? 0);
      default:         return 0;
    }
  });
}

function renderGrid() {
  setCount(filtered.length + ' files');
  const container = document.getElementById('grid-container');
  const colWidth  = parseInt(document.getElementById('zoom-slider').value);

  if (!grid) {
    grid = new MasonryGrid(container, colWidth);
  }
  grid.setItems(filtered);
}

// ── File watcher ──────────────────────────────────────────────────────────────
function listenFileWatcher() {
  listen('files-changed', debounce(async () => {
    if (!currentFolder) return;
    const files = await invoke('scan_folder', { path: currentFolder });
    loadFiles(files);
  }, 800));
}

// ── Context menu ──────────────────────────────────────────────────────────────
function bindContextMenu() {
  const menu = document.getElementById('ctx-menu');

  document.addEventListener('contextmenu', (e) => {
    const tile = e.target.closest('.tile');
    if (!tile) { menu.style.display = 'none'; return; }
    e.preventDefault();

    ctxTarget = tile.dataset.path;

    // Update star highlights
    const currentRating = allFiles.find(f => f.path === ctxTarget)?.rating ?? 0;
    document.querySelectorAll('.star-btn').forEach(btn => {
      btn.classList.toggle('on', parseInt(btn.dataset.r) <= currentRating);
    });

    const x = Math.min(e.clientX, window.innerWidth  - 200);
    const y = Math.min(e.clientY, window.innerHeight - 220);
    menu.style.cssText = `display:block;left:${x}px;top:${y}px;`;
  });

  document.addEventListener('click', (e) => {
    if (!e.target.closest('#ctx-menu')) menu.style.display = 'none';
  });

  document.getElementById('ctx-explorer').addEventListener('click', () => {
    if (ctxTarget) invoke('open_in_explorer', { path: ctxTarget });
  });

  document.getElementById('ctx-copy').addEventListener('click', () => {
    if (ctxTarget) navigator.clipboard?.writeText(ctxTarget);
  });

  // Rating buttons
  document.querySelectorAll('.star-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      if (!ctxTarget) return;
      const r = parseInt(btn.dataset.r);
      await invoke('set_rating', { path: ctxTarget, rating: r });
      const file = allFiles.find(f => f.path === ctxTarget);
      if (file) {
        file.rating = r;
        // Refresh rendered tile's star display
        const el = document.querySelector(`.tile[data-path="${CSS.escape(ctxTarget)}"]`);
        if (el) {
          el.classList.toggle('has-rating', r > 0);
          const s = el.querySelector('.tile-stars');
          if (s) s.textContent = '★'.repeat(r);
        }
      }
    });
  });
}

// ── Helpers ───────────────────────────────────────────────────────────────────
function showEmpty(show) {
  document.getElementById('empty-state').style.display = show ? 'flex' : 'none';
}

function setCount(txt) {
  document.getElementById('file-count').textContent = txt;
}

function debounce(fn, ms) {
  let t;
  return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); };
}

init();
