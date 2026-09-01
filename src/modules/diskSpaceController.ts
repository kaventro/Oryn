// src/modules/diskSpaceController.ts
import { fmtBytes, escHtml } from './formatUtils.ts';

export interface DiskSpaceItem {
  name: string;
  path: string;
  size: number;
  files: number;
  dirs: number;
  isDir: boolean;
}

export interface DiskSpaceAnalysis {
  ok: boolean;
  path: string;
  totalSize: number;
  totalFiles: number;
  totalDirs: number;
  items: DiskSpaceItem[];
}

export interface DiskSpaceControllerDeps {
  api: () => any;
  state: any;
  setStatus: (msg: string) => void;
  navigateTo?: (side: 'left' | 'right', path: string) => Promise<void> | void;
  focusActiveList?: () => void;
  fileOps?: any;
}

const TREEMAP_COLORS = [
  '#3b82f6', // blue
  '#10b981', // emerald
  '#f59e0b', // amber
  '#ec4899', // pink
  '#8b5cf6', // violet
  '#06b6d4', // cyan
  '#f97316', // orange
  '#14b8a6', // teal
  '#6366f1', // indigo
  '#e11d48', // rose
  '#84cc16', // lime
  '#a855f7', // purple
];

export class DiskSpaceController {
  private deps: DiskSpaceControllerDeps;
  private overlay: HTMLElement | null = null;
  private modal: HTMLElement | null = null;
  private breadcrumbsEl: HTMLElement | null = null;
  private summaryEl: HTMLElement | null = null;
  private treemapContainer: HTMLElement | null = null;
  private listContainer: HTMLElement | null = null;
  private refreshBtn: HTMLElement | null = null;
  private closeBtn: HTMLElement | null = null;
  private upBtn: HTMLElement | null = null;
  private loadingEl: HTMLElement | null = null;

  private currentPath: string = '';
  private currentAnalysis: DiskSpaceAnalysis | null = null;
  private isScanning: boolean = false;
  private isOpenState: boolean = false;

  constructor(deps: DiskSpaceControllerDeps) {
    this.deps = deps;
    this.bindElements();
    this.setupEvents();
  }

  private bindElements(): void {
    if (typeof document === 'undefined') return;
    this.overlay = document.getElementById('disk-space-overlay');
    this.modal = document.getElementById('disk-space-modal');
    this.breadcrumbsEl = document.getElementById('disk-space-crumbs');
    this.summaryEl = document.getElementById('disk-space-summary');
    this.treemapContainer = document.getElementById('disk-space-treemap');
    this.listContainer = document.getElementById('disk-space-list');
    this.refreshBtn = document.getElementById('disk-space-refresh');
    this.closeBtn = document.getElementById('disk-space-close');
    this.upBtn = document.getElementById('disk-space-up');
    this.loadingEl = document.getElementById('disk-space-loading');
  }

  private setupEvents(): void {
    this.modal?.addEventListener('click', (e) => e.stopPropagation());
    this.overlay?.addEventListener('click', (e) => {
      if (e.target === this.overlay) this.hide();
    });
    this.closeBtn?.addEventListener('click', () => this.hide());
    this.refreshBtn?.addEventListener('click', () => {
      if (this.currentPath) void this.scan(this.currentPath);
    });
    this.upBtn?.addEventListener('click', () => this.goUp());
  }

  public isOpen(): boolean {
    return this.isOpenState;
  }

  public open(targetPath?: string): void {
    if (!this.overlay) this.bindElements();
    if (!this.overlay) return;

    const activeSide = this.deps.state?.active || 'left';
    const path = targetPath || this.deps.state?.[activeSide]?.path || '';

    this.isOpenState = true;
    this.overlay.classList.remove('hidden');
    this.overlay.setAttribute('aria-hidden', 'false');

    if (path) {
      void this.scan(path);
    }
  }

  public hide(): void {
    if (!this.overlay) return;
    this.isOpenState = false;
    this.overlay.classList.add('hidden');
    this.overlay.setAttribute('aria-hidden', 'true');
    this.deps.focusActiveList?.();
  }

  public toggle(targetPath?: string): void {
    if (this.isOpen()) {
      this.hide();
    } else {
      this.open(targetPath);
    }
  }

  public goUp(): void {
    if (!this.currentPath) return;
    const isWindowsDrive = /^[a-zA-Z]:[/\\]?$/.test(this.currentPath.trim());
    if (isWindowsDrive || this.currentPath === '/' || this.currentPath === '\\') {
      return; // Already at root
    }

    const sep = this.currentPath.includes('\\') ? '\\' : '/';
    const parts = this.currentPath.split(/[/\\]/).filter(Boolean);
    if (parts.length <= 1) {
      if (this.currentPath.startsWith('/')) {
        void this.scan('/');
      } else if (/^[a-zA-Z]:/.test(this.currentPath)) {
        void this.scan(parts[0] + (sep === '\\' ? '\\' : '/'));
      }
      return;
    }

    parts.pop();
    let parentPath = '';
    if (this.currentPath.startsWith('/')) {
      parentPath = '/' + parts.join('/');
    } else if (/^[a-zA-Z]:/.test(parts[0])) {
      if (parts.length === 1) {
        parentPath = parts[0] + (sep === '\\' ? '\\' : '/');
      } else {
        parentPath = parts[0] + sep + parts.slice(1).join(sep);
      }
    } else {
      parentPath = parts.join(sep);
    }
    void this.scan(parentPath);
  }

  public async scan(dirPath: string): Promise<void> {
    if (this.isScanning) return;
    this.currentPath = dirPath;
    this.isScanning = true;

    this.renderCrumbs();

    if (this.loadingEl) this.loadingEl.classList.remove('hidden');
    if (this.treemapContainer) this.treemapContainer.replaceChildren();
    if (this.listContainer) this.listContainer.replaceChildren();
    if (this.summaryEl) this.summaryEl.textContent = 'Analyzing disk space…';

    try {
      const apiObj = typeof this.deps.api === 'function' ? this.deps.api() : this.deps.api;
      const res: DiskSpaceAnalysis = await apiObj.analyzeDir(dirPath);

      this.currentAnalysis = res;
      this.render();
    } catch (err: any) {
      if (this.summaryEl) {
        this.summaryEl.textContent = `Analysis failed: ${err?.message || err}`;
      }
      this.deps.setStatus(`Disk space scan failed: ${err?.message || err}`);
    } finally {
      this.isScanning = false;
      if (this.loadingEl) this.loadingEl.classList.add('hidden');
    }
  }

  private renderCrumbs(): void {
    if (!this.breadcrumbsEl) return;
    this.breadcrumbsEl.replaceChildren();

    const parts = this.currentPath.split(/[/\\]/).filter(Boolean);
    const isUnixAbs = this.currentPath.startsWith('/');
    const sep = this.currentPath.includes('\\') ? '\\' : '/';

    let accum = '';

    parts.forEach((p, idx) => {
      if (idx === 0) {
        if (isUnixAbs) {
          accum = '/' + p;
        } else if (/^[a-zA-Z]:$/.test(p)) {
          accum = p + sep;
        } else {
          accum = p;
        }
      } else {
        accum += (accum.endsWith('/') || accum.endsWith('\\')) ? p : (sep + p);
      }

      const snapPath = accum;
      const crumb = document.createElement('button');
      crumb.type = 'button';
      crumb.className = `disk-crumb${idx === parts.length - 1 ? ' active' : ''}`;
      crumb.textContent = p;
      crumb.addEventListener('click', () => {
        void this.scan(snapPath);
      });

      this.breadcrumbsEl!.appendChild(crumb);

      if (idx < parts.length - 1) {
        const sepEl = document.createElement('span');
        sepEl.className = 'disk-crumb-sep';
        sepEl.textContent = sep;
        this.breadcrumbsEl!.appendChild(sepEl);
      }
    });
  }

  private render(): void {
    if (!this.currentAnalysis) return;
    const a = this.currentAnalysis;

    if (this.summaryEl) {
      this.summaryEl.innerHTML = `
        <span class="disk-stat-val">${fmtBytes(a.totalSize)}</span> total in
        <span class="disk-stat-count">${a.totalFiles.toLocaleString()} files</span> and
        <span class="disk-stat-count">${a.totalDirs.toLocaleString()} folders</span>
      `;
    }

    this.renderTreemap();
    this.renderRankList();
  }

  private renderTreemap(): void {
    if (!this.treemapContainer || !this.currentAnalysis) return;
    this.treemapContainer.replaceChildren();

    const items = this.currentAnalysis.items;
    const total = this.currentAnalysis.totalSize;

    if (items.length === 0 || total === 0) {
      const empty = document.createElement('div');
      empty.className = 'disk-empty';
      empty.textContent = 'Empty directory (0 bytes)';
      this.treemapContainer.appendChild(empty);
      return;
    }

    // Compute visual grid blocks
    items.forEach((it, idx) => {
      const pct = total > 0 ? (it.size / total) * 100 : 0;
      if (pct < 0.5 && idx > 15) return; // skip tiny slivers in treemap

      const block = document.createElement('div');
      block.className = `treemap-block${it.isDir ? ' is-dir' : ''}`;
      const color = TREEMAP_COLORS[idx % TREEMAP_COLORS.length];
      block.style.background = `color-mix(in srgb, ${color} 24%, var(--surface))`;
      block.style.borderColor = `color-mix(in srgb, ${color} 65%, transparent)`;
      block.style.flexGrow = `${Math.max(1, Math.round(pct * 10))}`;

      const nameEl = document.createElement('span');
      nameEl.className = 'treemap-name';
      nameEl.textContent = it.name;

      const sizeEl = document.createElement('span');
      sizeEl.className = 'treemap-size';
      sizeEl.textContent = `${fmtBytes(it.size)} (${pct.toFixed(1)}%)`;

      block.append(nameEl, sizeEl);
      block.title = `${it.name}\n${fmtBytes(it.size)} (${it.size.toLocaleString()} bytes)\n${it.files} files, ${it.dirs} dirs\n${pct.toFixed(2)}% of parent`;

      block.addEventListener('click', () => {
        if (it.isDir) {
          void this.scan(it.path);
        } else {
          this.revealInPane(it.path);
        }
      });

      this.treemapContainer!.appendChild(block);
    });
  }

  private renderRankList(): void {
    if (!this.listContainer || !this.currentAnalysis) return;
    this.listContainer.replaceChildren();

    const items = this.currentAnalysis.items;
    const total = this.currentAnalysis.totalSize;

    items.forEach((it, idx) => {
      const pct = total > 0 ? (it.size / total) * 100 : 0;
      const color = TREEMAP_COLORS[idx % TREEMAP_COLORS.length];

      const row = document.createElement('div');
      row.className = 'disk-rank-row';

      const rankEl = document.createElement('span');
      rankEl.className = 'disk-rank-num';
      rankEl.textContent = `${idx + 1}`;

      const iconEl = document.createElement('span');
      iconEl.className = 'disk-rank-icon';
      iconEl.textContent = it.isDir ? '📁' : '📄';

      const infoEl = document.createElement('div');
      infoEl.className = 'disk-rank-info';

      const topRow = document.createElement('div');
      topRow.className = 'disk-rank-top';

      const nameEl = document.createElement('span');
      nameEl.className = 'disk-rank-name';
      nameEl.textContent = it.name;

      const sizeEl = document.createElement('span');
      sizeEl.className = 'disk-rank-size';
      sizeEl.textContent = `${fmtBytes(it.size)} (${pct.toFixed(1)}%)`;

      topRow.append(nameEl, sizeEl);

      const barBg = document.createElement('div');
      barBg.className = 'disk-rank-bar-bg';

      const barFill = document.createElement('div');
      barFill.className = 'disk-rank-bar-fill';
      barFill.style.width = `${Math.min(100, Math.max(1, pct))}%`;
      barFill.style.backgroundColor = color;

      barBg.appendChild(barFill);
      infoEl.append(topRow, barBg);

      const actions = document.createElement('div');
      actions.className = 'disk-rank-actions';

      if (it.isDir) {
        const openBtn = document.createElement('button');
        openBtn.type = 'button';
        openBtn.className = 'search-action-btn search-action-btn--sm';
        openBtn.textContent = 'Scan ▶';
        openBtn.title = 'Scan inside this folder';
        openBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          void this.scan(it.path);
        });
        actions.appendChild(openBtn);
      }

      const revealBtn = document.createElement('button');
      revealBtn.type = 'button';
      revealBtn.className = 'search-action-btn search-action-btn--sm';
      revealBtn.textContent = 'Reveal';
      revealBtn.title = 'Reveal in Oryn Pane';
      revealBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        this.revealInPane(it.path);
      });
      actions.appendChild(revealBtn);

      const delBtn = document.createElement('button');
      delBtn.type = 'button';
      delBtn.className = 'search-action-btn search-action-btn--sm search-action-btn--danger';
      delBtn.textContent = '🗑';
      delBtn.title = 'Move to Trash';
      delBtn.addEventListener('click', async (e) => {
        e.stopPropagation();
        if (confirm(`Move "${it.name}" (${fmtBytes(it.size)}) to Trash?`)) {
          try {
            const apiObj = typeof this.deps.api === 'function' ? this.deps.api() : this.deps.api;
            await apiObj.deletePath(it.path, true);
            this.deps.setStatus(`Moved to Trash: ${it.name}`);
            void this.scan(this.currentPath);
          } catch (err: any) {
            alert(`Delete failed: ${err?.message || err}`);
          }
        }
      });
      actions.appendChild(delBtn);

      row.append(rankEl, iconEl, infoEl, actions);
      this.listContainer!.appendChild(row);
    });
  }

  private revealInPane(targetPath: string): void {
    this.hide();
    const activeSide = this.deps.state?.active || 'left';
    void this.deps.navigateTo?.(activeSide, targetPath);
  }
}
