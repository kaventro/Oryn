// src/modules/duplicateFinderController.ts
import { fmtBytes, formatIsoLocal, escHtml } from './formatUtils.ts';

export interface DuplicateFile {
  path: string;
  name: string;
  size: number;
  mtime: string;
}

export interface DuplicateGroup {
  hash: string;
  size: number;
  files: DuplicateFile[];
  totalWasted: number;
}

export interface DuplicateScanResult {
  ok: boolean;
  totalScanned: number;
  duplicateGroups: DuplicateGroup[];
  totalWastedBytes: number;
  duplicateFilesCount: number;
}

export interface DuplicateFinderDeps {
  api: () => any;
  state: any;
  setStatus: (msg: string) => void;
  navigateTo?: (side: 'left' | 'right', path: string) => Promise<void> | void;
  focusActiveList?: () => void;
  openViewer?: (fp: string, item: any) => void;
}

export class DuplicateFinderController {
  private deps: DuplicateFinderDeps;
  private overlay: HTMLElement | null = null;
  private modal: HTMLElement | null = null;
  private pathInput: HTMLInputElement | null = null;
  private minSizeSelect: HTMLSelectElement | null = null;
  private scanBtn: HTMLButtonElement | null = null;
  private closeBtn: HTMLButtonElement | null = null;
  private browseBtn: HTMLButtonElement | null = null;
  private summaryEl: HTMLElement | null = null;
  private groupsContainer: HTMLElement | null = null;
  private loadingEl: HTMLElement | null = null;
  private footerEl: HTMLElement | null = null;
  private selectedCountEl: HTMLElement | null = null;
  private deleteSelectedBtn: HTMLButtonElement | null = null;

  private currentResult: DuplicateScanResult | null = null;
  private selectedPaths: Set<string> = new Set();
  private isScanning: boolean = false;
  private isOpenState: boolean = false;

  constructor(deps: DuplicateFinderDeps) {
    this.deps = deps;
    this.bindElements();
    this.setupEvents();
  }

  private bindElements(): void {
    if (typeof document === 'undefined') return;
    this.overlay = document.getElementById('duplicates-overlay');
    this.modal = document.getElementById('duplicates-modal');
    this.pathInput = document.getElementById('duplicates-path-input') as HTMLInputElement;
    this.minSizeSelect = document.getElementById('duplicates-min-size') as HTMLSelectElement;
    this.scanBtn = document.getElementById('duplicates-scan-btn') as HTMLButtonElement;
    this.closeBtn = document.getElementById('duplicates-close-btn') as HTMLButtonElement;
    this.browseBtn = document.getElementById('duplicates-browse-btn') as HTMLButtonElement;
    this.summaryEl = document.getElementById('duplicates-summary');
    this.groupsContainer = document.getElementById('duplicates-groups');
    this.loadingEl = document.getElementById('duplicates-loading');
    this.footerEl = document.getElementById('duplicates-footer');
    this.selectedCountEl = document.getElementById('duplicates-selected-count');
    this.deleteSelectedBtn = document.getElementById('duplicates-delete-btn') as HTMLButtonElement;
  }

  private setupEvents(): void {
    this.modal?.addEventListener('click', (e) => e.stopPropagation());
    this.overlay?.addEventListener('click', (e) => {
      if (e.target === this.overlay) this.hide();
    });
    this.closeBtn?.addEventListener('click', () => this.hide());
    this.scanBtn?.addEventListener('click', () => {
      const p = this.pathInput?.value.trim();
      if (p) void this.startScan(p);
    });
    this.browseBtn?.addEventListener('click', async () => {
      try {
        const apiObj = typeof this.deps.api === 'function' ? this.deps.api() : this.deps.api;
        const cur = this.pathInput?.value.trim() || this.deps.state?.[this.deps.state?.active || 'left']?.path || '';
        const picked = await apiObj.pickFolder(cur);
        if (picked && this.pathInput) {
          this.pathInput.value = picked;
          void this.startScan(picked);
        }
      } catch (_) { }
    });

    document.getElementById('duplicates-select-newest')?.addEventListener('click', () => this.selectStrategy('newest'));
    document.getElementById('duplicates-select-oldest')?.addEventListener('click', () => this.selectStrategy('oldest'));
    document.getElementById('duplicates-select-shortest')?.addEventListener('click', () => this.selectStrategy('shortest'));
    document.getElementById('duplicates-select-none')?.addEventListener('click', () => this.selectStrategy('none'));

    this.deleteSelectedBtn?.addEventListener('click', () => void this.deleteSelected());
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

    if (this.pathInput && path) {
      this.pathInput.value = path;
    }

    if (path) {
      void this.startScan(path);
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

  public async startScan(dirPath: string): Promise<void> {
    if (this.isScanning) return;
    this.isScanning = true;
    this.selectedPaths.clear();

    if (this.loadingEl) this.loadingEl.classList.remove('hidden');
    if (this.groupsContainer) this.groupsContainer.replaceChildren();
    if (this.summaryEl) this.summaryEl.textContent = 'Scanning directory for identical files…';
    if (this.footerEl) this.footerEl.classList.add('hidden');

    const minSizeBytes = parseInt(this.minSizeSelect?.value || '1024', 10);

    try {
      const apiObj = typeof this.deps.api === 'function' ? this.deps.api() : this.deps.api;
      const res: DuplicateScanResult = await apiObj.scanDuplicates(dirPath, minSizeBytes, 500);

      this.currentResult = res;
      this.render();
      this.selectStrategy('oldest'); // default: mark older duplicates for deletion
    } catch (err: any) {
      if (this.summaryEl) {
        this.summaryEl.textContent = `Duplicate scan failed: ${err?.message || err}`;
      }
      this.deps.setStatus(`Duplicate scan error: ${err?.message || err}`);
    } finally {
      this.isScanning = false;
      if (this.loadingEl) this.loadingEl.classList.add('hidden');
    }
  }

  private render(): void {
    if (!this.currentResult) return;
    const r = this.currentResult;

    if (this.summaryEl) {
      if (r.duplicateGroups.length === 0) {
        this.summaryEl.innerHTML = `<span>No duplicate files found in ${r.totalScanned.toLocaleString()} files scanned.</span>`;
      } else {
        this.summaryEl.innerHTML = `
          Found <strong>${r.duplicateGroups.length.toLocaleString()} sets</strong> of duplicates (${r.duplicateFilesCount.toLocaleString()} files)
          — <strong>${fmtBytes(r.totalWastedBytes)}</strong> recoverable space.
        `;
      }
    }

    if (this.footerEl) {
      if (r.duplicateGroups.length > 0) this.footerEl.classList.remove('hidden');
      else this.footerEl.classList.add('hidden');
    }

    this.renderGroups();
    this.updateSelectedFooter();
  }

  private renderGroups(): void {
    if (!this.groupsContainer || !this.currentResult) return;
    this.groupsContainer.replaceChildren();

    const groups = this.currentResult.duplicateGroups;

    if (groups.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'duplicates-empty';
      empty.textContent = '🎉 All files in this folder are unique! No duplicates detected.';
      this.groupsContainer.appendChild(empty);
      return;
    }

    groups.forEach((g, gIdx) => {
      const groupCard = document.createElement('div');
      groupCard.className = 'duplicate-group-card';

      const head = document.createElement('div');
      head.className = 'duplicate-group-head';

      const title = document.createElement('span');
      title.className = 'duplicate-group-title';

      const setPrefix = document.createTextNode(`Set #${gIdx + 1} • `);
      const strongName = document.createElement('strong');
      strongName.textContent = g.files[0]?.name || 'Unknown';
      const sizeSuffix = document.createTextNode(` (${fmtBytes(g.size)} each)`);
      title.append(setPrefix, strongName, sizeSuffix);

      const badge = document.createElement('span');
      badge.className = 'duplicate-group-badge';
      badge.textContent = `Wastes ${fmtBytes(g.totalWasted)}`;

      head.append(title, badge);
      groupCard.appendChild(head);

      const list = document.createElement('div');
      list.className = 'duplicate-group-files';

      g.files.forEach((f) => {
        const row = document.createElement('div');
        row.className = 'duplicate-file-row';

        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.className = 'duplicate-checkbox';
        checkbox.checked = this.selectedPaths.has(f.path);
        checkbox.addEventListener('change', () => {
          if (checkbox.checked) this.selectedPaths.add(f.path);
          else this.selectedPaths.delete(f.path);
          this.updateSelectedFooter();
        });

        const icon = document.createElement('span');
        icon.className = 'duplicate-file-icon';
        icon.textContent = '📄';

        const info = document.createElement('div');
        info.className = 'duplicate-file-info';

        const pathEl = document.createElement('span');
        pathEl.className = 'duplicate-file-path';
        pathEl.textContent = f.path;
        pathEl.title = f.path;

        const dateEl = document.createElement('span');
        dateEl.className = 'duplicate-file-date';
        dateEl.textContent = formatIsoLocal(f.mtime);

        info.append(pathEl, dateEl);

        const actions = document.createElement('div');
        actions.className = 'duplicate-file-actions';

        const revealBtn = document.createElement('button');
        revealBtn.type = 'button';
        revealBtn.className = 'search-action-btn search-action-btn--sm';
        revealBtn.textContent = 'Reveal';
        revealBtn.title = 'Reveal file in Oryn panel';
        revealBtn.addEventListener('click', () => {
          this.hide();
          const activeSide = this.deps.state?.active || 'left';
          void this.deps.navigateTo?.(activeSide, f.path);
        });

        actions.appendChild(revealBtn);
        row.append(checkbox, icon, info, actions);
        list.appendChild(row);
      });

      groupCard.appendChild(list);
      this.groupsContainer!.appendChild(groupCard);
    });
  }

  public selectStrategy(strategy: 'newest' | 'oldest' | 'shortest' | 'none'): void {
    if (!this.currentResult) return;
    this.selectedPaths.clear();

    if (strategy !== 'none') {
      this.currentResult.duplicateGroups.forEach((g) => {
        const sorted = g.files.slice();
        if (strategy === 'newest') {
          // Keep newest: sort newest first, select the rest
          sorted.sort((a, b) => new Date(b.mtime).getTime() - new Date(a.mtime).getTime());
        } else if (strategy === 'oldest') {
          // Keep oldest: sort oldest first, select the rest
          sorted.sort((a, b) => new Date(a.mtime).getTime() - new Date(b.mtime).getTime());
        } else if (strategy === 'shortest') {
          // Keep shortest path length
          sorted.sort((a, b) => a.path.length - b.path.length);
        }

        // Keep index 0, mark index 1..N for deletion
        for (let i = 1; i < sorted.length; i++) {
          this.selectedPaths.add(sorted[i].path);
        }
      });
    }

    this.renderGroups();
    this.updateSelectedFooter();
  }

  private updateSelectedFooter(): void {
    if (!this.selectedCountEl || !this.currentResult) return;

    let reclaimedBytes = 0;
    this.currentResult.duplicateGroups.forEach((g) => {
      g.files.forEach((f) => {
        if (this.selectedPaths.has(f.path)) {
          reclaimedBytes += f.size;
        }
      });
    });

    const count = this.selectedPaths.size;
    this.selectedCountEl.textContent = `${count.toLocaleString()} files selected (${fmtBytes(reclaimedBytes)} to reclaim)`;

    if (this.deleteSelectedBtn) {
      this.deleteSelectedBtn.disabled = count === 0;
    }
  }

  public async deleteSelected(): Promise<void> {
    if (this.selectedPaths.size === 0) return;
    const count = this.selectedPaths.size;

    let reclaimedBytes = 0;
    this.currentResult?.duplicateGroups.forEach((g) => {
      g.files.forEach((f) => {
        if (this.selectedPaths.has(f.path)) reclaimedBytes += f.size;
      });
    });

    if (!confirm(`Move ${count} duplicate files (${fmtBytes(reclaimedBytes)}) to Trash?`)) {
      return;
    }

    const apiObj = typeof this.deps.api === 'function' ? this.deps.api() : this.deps.api;
    let deletedCount = 0;

    for (const fp of this.selectedPaths) {
      try {
        await apiObj.deletePath(fp, true);
        deletedCount++;
      } catch (err: any) {
        console.error(`Failed to trash ${fp}:`, err);
      }
    }

    this.deps.setStatus(`Moved ${deletedCount} duplicate files to Trash (${fmtBytes(reclaimedBytes)} freed).`);
    const currentDir = this.pathInput?.value.trim();
    if (currentDir) {
      void this.startScan(currentDir);
    }
  }
}
