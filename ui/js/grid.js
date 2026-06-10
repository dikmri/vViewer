/**
 * Virtual masonry grid.
 * Only creates DOM elements for tiles visible in the viewport (± one buffer
 * screen). Tiles that scroll out are destroyed and their video decoders freed.
 * This keeps memory and CPU proportional to what is *visible*, not to the
 * total file count.
 */
export class MasonryGrid {
  /**
   * @param {HTMLElement} container  Positioned container element
   * @param {number}      colWidth   Initial column width in px
   * @param {number}      gap        Gap between tiles in px
   */
  constructor(container, colWidth = 240, gap = 6) {
    this.container  = container;
    this.colWidth   = colWidth;
    this.gap        = gap;
    this.items      = [];     // VideoFile[]
    this.positions  = [];     // {x,y,w,h}[]
    this.rendered   = new Map(); // index -> tile element
    this.globalMute = true;

    this.container.style.position = 'relative';

    // Throttle scroll via rAF.
    // Listen on window (standard) AND document.documentElement as fallback,
    // since some webview configurations route scroll to the html element.
    let rafPending = false;
    this._onScroll = () => {
      if (!rafPending) {
        rafPending = true;
        requestAnimationFrame(() => { this._updateViewport(); rafPending = false; });
      }
    };
    window.addEventListener('scroll', this._onScroll, { passive: true });
    document.documentElement.addEventListener('scroll', this._onScroll, { passive: true });

    this._ro = new ResizeObserver(() => this._relayout());
    this._ro.observe(container.parentElement || document.documentElement);
  }

  setColWidth(w) {
    this.colWidth = w;
    this._relayout();
  }

  setMute(muted) {
    this.globalMute = muted;
    for (const [, el] of this.rendered) {
      const v = el.querySelector('video');
      if (v) v.muted = muted;
    }
  }

  /** Replace the entire item list and relayout. */
  setItems(items) {
    this.items = items;
    this._relayout();
  }

  /** Append more items (e.g. from a file-watcher event) without full relayout. */
  appendItems(newItems) {
    this.items.push(...newItems);
    this._relayout();
  }

  // ── Private ───────────────────────────────────────────────────────────────

  _relayout() {
    const containerWidth =
      this.container.parentElement?.clientWidth || window.innerWidth - 12;
    const numCols = Math.max(
      1,
      Math.floor((containerWidth + this.gap) / (this.colWidth + this.gap))
    );

    const colH = new Array(numCols).fill(0);
    this.positions = this.items.map((item) => {
      const col = colH.indexOf(Math.min(...colH));
      const ar  = item.height > 0 && item.width > 0
        ? item.height / item.width
        : 9 / 16;
      const h   = Math.round(this.colWidth * ar);
      const x   = col * (this.colWidth + this.gap);
      const y   = colH[col];
      colH[col] += h + this.gap;
      return { x, y, w: this.colWidth, h };
    });

    const totalH = colH.length ? Math.max(...colH) : 0;
    this.container.style.height = totalH + 'px';

    this._clearAll();
    this._updateViewport();
  }

  _updateViewport() {
    const scrollY = window.scrollY || document.documentElement.scrollTop || 0;
    const viewH   = window.innerHeight;
    const buffer  = viewH;                   // one extra screen above and below
    const top     = scrollY - buffer;
    const bottom  = scrollY + viewH + buffer;

    const visible = new Set();
    for (let i = 0; i < this.positions.length; i++) {
      const { y, h } = this.positions[i];
      if (y + h > top && y < bottom) visible.add(i);
    }

    // Remove tiles that left the viewport
    for (const [i, el] of this.rendered) {
      if (!visible.has(i)) {
        const v = el.querySelector('video');
        if (v) { v.pause(); v.src = ''; v.load(); }
        el.remove();
        this.rendered.delete(i);
      }
    }

    // Mount tiles that entered the viewport
    const frag = document.createDocumentFragment();
    for (const i of visible) {
      if (!this.rendered.has(i)) {
        const el = this._createTile(i);
        frag.appendChild(el);
        this.rendered.set(i, el);
      }
    }
    this.container.appendChild(frag);
  }

  _createTile(index) {
    const item = this.items[index];
    const pos  = this.positions[index];

    const tile = document.createElement('div');
    tile.className   = 'tile' + (item.rating ? ' has-rating' : '');
    tile.dataset.idx  = index;
    tile.dataset.path = item.path;
    tile.style.cssText =
      `left:${pos.x}px;top:${pos.y}px;width:${pos.w}px;height:${pos.h}px;`;

    // Video element
    const video = document.createElement('video');
    video.muted     = this.globalMute;
    video.loop      = true;
    video.autoplay  = true;
    video.playsInline = true;
    video.preload   = 'auto';

    video.addEventListener('loadedmetadata', () => {
      if (video.videoWidth && video.videoHeight) {
        item.width  = video.videoWidth;
        item.height = video.videoHeight;
        window.__tauri_invoke?.('update_dimensions', {
          path:   item.path,
          width:  video.videoWidth,
          height: video.videoHeight,
        });
      }
    });

    // Mark loaded when the browser can start playing (hides spinner)
    video.addEventListener('canplay', () => tile.classList.add('loaded'), { once: true });

    video.src = `http://127.0.0.1:${window.__videoPort}/file?path=${encodeURIComponent(item.path)}`;

    // Spinner (shown until canplay)
    const spinner = document.createElement('div');
    spinner.className = 'tile-spinner';

    // Overlay
    const overlay = document.createElement('div');
    overlay.className = 'tile-overlay';
    overlay.innerHTML =
      `<div class="tile-name">${escHtml(item.filename)}</div>` +
      `<div class="tile-meta">${fmtSize(item.size)}</div>`;

    // Stars
    const stars = document.createElement('div');
    stars.className   = 'tile-stars';
    stars.textContent = '★'.repeat(item.rating || 0);

    tile.append(video, spinner, overlay, stars);

    // Double-click → fullscreen
    tile.addEventListener('dblclick', () => {
      if (video.requestFullscreen) video.requestFullscreen();
    });

    return tile;
  }

  _clearAll() {
    for (const [, el] of this.rendered) {
      const v = el.querySelector('video');
      if (v) { v.src = ''; v.load(); }
      el.remove();
    }
    this.rendered.clear();
  }

  destroy() {
    this._clearAll();
    window.removeEventListener('scroll', this._onScroll);
    document.documentElement.removeEventListener('scroll', this._onScroll);
    this._ro.disconnect();
  }
}

// ── Helpers ────────────────────────────────────────────────────────────────

function escHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

function fmtSize(bytes) {
  if (!bytes) return '';
  if (bytes < 1024)       return bytes + ' B';
  if (bytes < 1048576)    return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + ' MB';
  return (bytes / 1073741824).toFixed(2) + ' GB';
}
