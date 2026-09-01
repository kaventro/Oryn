// src/modules/propertiesController.ts
import { fmtBytes, formatIsoLocal, safeColor, escHtml } from './formatUtils.ts';

export interface PropertyItem {
  path: string;
  size: number;
  mode: string;
  modeString?: string;
  isDir: boolean;
  mtime: string;
  created?: string;
  accessed?: string;
  readonly?: boolean;
}

export interface PropertiesControllerDeps {
  api: () => any;
  setStatus: (msg: string) => void;
  focusActiveList: () => void;
  getFilteredSelection: (side: 'left' | 'right') => any;
  fullPath: (side: 'left' | 'right', item: any) => Promise<string | null>;
  tagController?: any;
}

export class PropertiesController {
  private api: () => any;
  private setStatus: (msg: string) => void;
  private focusActiveList: () => void;
  private getFilteredSelection: (side: 'left' | 'right') => any;
  private fullPath: (side: 'left' | 'right', item: any) => Promise<string | null>;
  private tagController: any;

  private overlay: HTMLElement | null = null;
  private modal: HTMLElement | null = null;
  private titleEl: HTMLElement | null = null;
  private kindEl: HTMLElement | null = null;
  private pathEl: HTMLElement | null = null;
  private typeEl: HTMLElement | null = null;
  private sizeEl: HTMLElement | null = null;
  private modeEl: HTMLElement | null = null;
  private mtimeEl: HTMLElement | null = null;
  private tagsListEl: HTMLElement | null = null;
  private summaryEl: HTMLElement | null = null;
  private closeBtn: HTMLElement | null = null;
  private copyBtn: HTMLElement | null = null;
  private currentPath: string | null = null;

  constructor(deps: PropertiesControllerDeps) {
    this.api = deps.api;
    this.setStatus = deps.setStatus;
    this.focusActiveList = deps.focusActiveList;
    this.getFilteredSelection = deps.getFilteredSelection;
    this.fullPath = deps.fullPath;
    this.tagController = deps.tagController || null;

    this.bindElements();
    this.setupEvents();
  }

  private bindElements(): void {
    if (typeof document === 'undefined') return;
    this.overlay = document.getElementById('properties-overlay');
    this.modal = document.getElementById('properties-modal');
    this.titleEl = document.getElementById('properties-title');
    this.kindEl = document.getElementById('properties-kind');
    this.pathEl = document.getElementById('properties-path');
    this.typeEl = document.getElementById('properties-type');
    this.sizeEl = document.getElementById('properties-size');
    this.modeEl = document.getElementById('properties-mode');
    this.mtimeEl = document.getElementById('properties-mtime');
    this.tagsListEl = document.getElementById('properties-tags-list');
    this.summaryEl = document.getElementById('properties-summary');
    this.closeBtn = document.getElementById('properties-close');
    this.copyBtn = document.getElementById('properties-copy');
  }

  private setupEvents(): void {
    this.modal?.addEventListener('click', (e) => e.stopPropagation());
    this.overlay?.addEventListener('click', (e) => {
      if (e.target === this.overlay) this.hide();
    });
    this.closeBtn?.addEventListener('click', () => this.hide());
    this.copyBtn?.addEventListener('click', async () => {
      const text = this.pathEl?.textContent || '';
      if (!text) return;
      try {
        const apiObj = typeof this.api === 'function' ? this.api() : this.api;
        await apiObj.clipboardWrite(text);
        this.setStatus('Path copied to clipboard.');
      } catch (_) {}
    });
  }

  public hide(): void {
    if (!this.overlay) return;
    this.overlay.classList.add('hidden');
    this.overlay.setAttribute('aria-hidden', 'true');
    this.focusActiveList();
  }

  public show(props: PropertyItem): void {
    if (!this.overlay) this.bindElements();
    if (!this.overlay) return;

    this.currentPath = props.path;
    const baseName = props.path.split(/[/|\\]/).filter(Boolean).pop() || props.path;

    if (this.titleEl) this.titleEl.textContent = baseName;
    if (this.kindEl) this.kindEl.textContent = (props.isDir ? 'folder' : 'file').toUpperCase();
    if (this.pathEl) {
      this.pathEl.textContent = props.path;
      this.pathEl.title = props.path;
    }
    if (this.typeEl) this.typeEl.textContent = props.isDir ? 'Directory / Folder' : 'File';
    if (this.sizeEl) {
      if (props.isDir) {
        this.sizeEl.textContent = 'Calculating…';
      } else {
        this.sizeEl.textContent = `${fmtBytes(props.size)} (${props.size.toLocaleString()} bytes)`;
      }
    }
    if (this.modeEl) {
      this.modeEl.textContent = props.modeString ? `${props.modeString} (0o${props.mode})` : `0o${props.mode}`;
    }
    if (this.mtimeEl) this.mtimeEl.textContent = formatIsoLocal(props.mtime);

    // Tags list
    if (this.tagsListEl && this.tagController) {
      this.tagsListEl.replaceChildren();
      const allTags = this.tagController.getAllTags?.() || [];
      const fileTags = this.tagController.getTagsForFile?.(props.path) || [];

      allTags.forEach((tag: any) => {
        const chip = document.createElement('button');
        chip.type = 'button';
        const active = fileTags.includes(tag.id);
        chip.className = `properties-tag-chip${active ? ' active' : ''}`;

        const dot = document.createElement('span');
        dot.className = `tag-dot tag-dot--${tag.id}`;
        if (tag.color && !tag.id.match(/^(red|orange|yellow|green|blue|purple|gray)$/)) {
          dot.style.background = safeColor(tag.color);
        }

        const label = document.createElement('span');
        label.textContent = tag.name;

        chip.append(dot, label);
        chip.addEventListener('click', () => {
          this.tagController.toggleTagForFile(props.path, tag.id);
          chip.classList.toggle('active');
          this.setStatus(`Toggled tag "${tag.name}"`);
        });

        this.tagsListEl!.appendChild(chip);
      });
    }

    if (this.summaryEl) {
      this.summaryEl.textContent = props.isDir
        ? 'Directory size includes the recursive contents below it.'
        : 'Click "Copy path" to copy full file location.';
    }

    this.overlay.classList.remove('hidden');
    this.overlay.setAttribute('aria-hidden', 'false');
    setTimeout(() => this.closeBtn?.focus(), 0);
  }

  public async showFor(side: 'left' | 'right', explicitFp: string | null = null): Promise<void> {
    let fp = explicitFp;
    if (!fp) {
      const { item } = this.getFilteredSelection(side);
      if (!item || item.base === '') return;
      fp = await this.fullPath(side, item);
    }
    if (!fp) return;

    let res: any;
    try {
      const apiObj = typeof this.api === 'function' ? this.api() : this.api;
      res = await apiObj.statProps(fp);
    } catch (e: any) {
      this.setStatus(e?.message || 'stat failed');
      return;
    }

    if (!res?.ok || !res.props) {
      this.setStatus(res?.error || 'stat failed');
      return;
    }

    this.show(res.props);

    if (!res.props.isDir) return;

    // Asynchronously calculate directory size
    try {
      const apiObj = typeof this.api === 'function' ? this.api() : this.api;
      const sz = await apiObj.getDirSize(fp);
      if (!sz?.ok || this.currentPath !== fp) return;
      const extra = sz.files !== undefined ? ` — ${sz.files} files, ${sz.dirs} dirs` : '';
      if (this.sizeEl) {
        this.sizeEl.textContent = `${fmtBytes(sz.size)} (${sz.size.toLocaleString()} bytes)${extra}`;
      }
    } catch {
      if (this.currentPath === fp && this.sizeEl) {
        this.sizeEl.textContent = 'Size unavailable';
      }
    }
  }
}
