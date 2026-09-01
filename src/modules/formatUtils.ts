// src/modules/formatUtils.ts
// Pure utility functions — zero dependencies. Import statically, never inject via constructor.

export interface PaneItem {
  base: string;
  isDir?: boolean;
  size?: number | null;
  mtime?: number | string | null;
  ext?: string;
  gitStatus?: string | null;
  sizing?: boolean;
  [key: string]: any;
}

export function fuzzyScore(base: string, query: string): number {
  const b = base.toLowerCase();
  const q = query.toLowerCase();
  if (!q) return 1;
  if (b.startsWith(q)) return 1000 - b.length;
  const idx = b.indexOf(q);
  if (idx >= 0) return 500 - idx;
  let bi = 0;
  for (let qi = 0; qi < q.length; qi++) {
    const found = b.indexOf(q[qi], bi);
    if (found < 0) return 0;
    bi = found + 1;
  }
  return 100;
}

export function filteredItems(pane: { filter?: string; items: PaneItem[]; sortField?: string; sortAsc?: boolean }): PaneItem[] {
  const f = (pane.filter || '').trim();
  if (f) {
    const scored: { it: PaneItem; score: number }[] = [];
    for (const it of pane.items) {
      if (it.base === '..') { scored.push({ it, score: 9999 }); continue; }
      const score = fuzzyScore(it.base, f);
      if (score > 0) scored.push({ it, score });
    }
    scored.sort((a, b) => b.score - a.score);
    return scored.map((x) => x.it);
  }

  const items = pane.items.slice();
  const parentIdx = items.findIndex((it) => it.base === '..');
  let parentDir: PaneItem | null = null;
  if (parentIdx >= 0) {
    parentDir = items.splice(parentIdx, 1)[0];
  }

  const field = pane.sortField || 'name';
  const asc = pane.sortAsc !== false;
  const dir = asc ? 1 : -1;

  items.sort((a, b) => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
    if (field === 'name') {
      return a.base.localeCompare(b.base) * dir;
    }
    if (field === 'ext') {
      const extA = fileExtFromBase(a);
      const extB = fileExtFromBase(b);
      if (extA !== extB) return extA.localeCompare(extB) * dir;
      return a.base.localeCompare(b.base) * dir;
    }
    if (field === 'size') {
      const sa = a.size || 0;
      const sb = b.size || 0;
      if (sa !== sb) return (sa - sb) * dir;
      return a.base.localeCompare(b.base) * dir;
    }
    if (field === 'date') {
      const da = Number(a.mtime) || 0;
      const db = Number(b.mtime) || 0;
      if (da !== db) return (da - db) * dir;
      return a.base.localeCompare(b.base) * dir;
    }
    return 0;
  });

  if (parentDir) items.unshift(parentDir);
  return items;
}

export function fmtSize(bytes?: number | null): string {
  if (bytes == null) return '';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} kB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function fmtBytes(bytes?: number | null): string {
  if (bytes == null || Number.isNaN(Number(bytes))) return '—';
  const n = Number(bytes);
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} kB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function formatIsoLocal(value?: string | number | Date | null): string {
  if (!value) return '—';
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return String(value);
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(d);
}

export function fileExtFromBase(item?: { base?: string; isDir?: boolean } | null): string {
  if (!item || item.base === '..' || item.isDir) return '—';
  const b = item.base || '';
  const i = b.lastIndexOf('.');
  if (i <= 0 || i === b.length - 1) return '—';
  return b.slice(i + 1).toLowerCase().slice(0, 12);
}

export function fileExtFullFromBase(item?: { base?: string; isDir?: boolean } | null): string {
  if (!item || item.base === '..' || item.isDir) return '';
  const b = item.base || '';
  const i = b.lastIndexOf('.');
  if (i <= 0 || i === b.length - 1) return '';
  return b.slice(i + 1).toLowerCase();
}

export function fmtSizeExact(bytes?: number | null): string {
  if (bytes == null) return '';
  return `${Number(bytes).toLocaleString()} bytes`;
}

export function rowDateText(item?: { base?: string; mtime?: number | string } | null): string {
  if (!item || item.base === '..' || !item.mtime) return '—';
  const d = new Date(item.mtime);
  if (Number.isNaN(d.getTime())) return '—';
  const day = d.getDate();
  const month = d.toLocaleString('en-US', { month: 'short' });
  const year = d.getFullYear();
  const hours = String(d.getHours()).padStart(2, '0');
  const minutes = String(d.getMinutes()).padStart(2, '0');
  return `${day} ${month} ${year} at ${hours}:${minutes}`;
}

export function shortPath(p: string): string {
  const max = 52;
  if (p.length <= max) return p;
  return '…' + p.slice(p.length - max + 1);
}

/**
 * Return `value` only if it is a safe CSS color literal, else `fallback`.
 * Blocks `url(...)`, `expression(...)` and other CSS-injection payloads that a
 * poisoned persisted tag color could otherwise smuggle into `element.style`.
 */
export function safeColor(value?: string | null, fallback = 'transparent'): string {
  if (typeof value !== 'string') return fallback;
  const v = value.trim();
  if (/^#[0-9a-fA-F]{3,8}$/.test(v)) return v;
  if (/^rgba?\([\d.,\s%/]+\)$/.test(v)) return v;
  if (/^hsla?\([\d.,\s%/]+\)$/.test(v)) return v;
  if (/^[a-zA-Z]{1,20}$/.test(v)) return v; // named color keyword
  return fallback;
}

export function escHtml(s: any): string {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

const DIFF_UI_MAX_LINES = 2500;

export function renderDiffContent(container: HTMLElement, diffText?: string): void {
  container.replaceChildren();
  if (!diffText || !diffText.trim()) {
    const empty = document.createElement('div');
    empty.className = 'diff-line diff-line--file';
    empty.textContent = 'No changes.';
    container.appendChild(empty);
    return;
  }
  const lines = diffText.split('\n');
  const total = lines.length;
  const slice = total > DIFF_UI_MAX_LINES ? lines.slice(0, DIFF_UI_MAX_LINES) : lines;
  const frag = document.createDocumentFragment();
  if (total > DIFF_UI_MAX_LINES) {
    const note = document.createElement('div');
    note.className = 'diff-line diff-line--hunk';
    note.textContent = `… UI shows first ${DIFF_UI_MAX_LINES} of ${total} lines (scroll performance).`;
    frag.appendChild(note);
  }
  slice.forEach((line) => {
    const div = document.createElement('div');
    div.className = 'diff-line';
    if (line.startsWith('+++') || line.startsWith('---')) div.classList.add('diff-line--file');
    else if (line.startsWith('+')) div.classList.add('diff-line--add');
    else if (line.startsWith('-')) div.classList.add('diff-line--del');
    else if (line.startsWith('@@')) div.classList.add('diff-line--hunk');
    div.textContent = line;
    frag.appendChild(div);
  });
  container.appendChild(frag);
}
