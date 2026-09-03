// src/modules/tagController.ts
import { safeColor } from './formatUtils.ts';
import type { AppState, Item } from './stateModels.ts';

export interface Tag {
  id: string;
  name: string;
  color: string;
  isCustom?: boolean;
}

export const DEFAULT_TAGS: Tag[] = [
  { id: 'red', name: 'Red', color: '#ff453a' },
  { id: 'orange', name: 'Orange', color: '#ff9f0a' },
  { id: 'yellow', name: 'Yellow', color: '#ffd60a' },
  { id: 'green', name: 'Green', color: '#30d158' },
  { id: 'blue', name: 'Blue', color: '#0a84ff' },
  { id: 'purple', name: 'Purple', color: '#bf5af2' },
  { id: 'gray', name: 'Gray', color: '#8e8e93' },
];

export interface TagControllerDeps {
  api: () => any;
  state: any;
  setStatus: (msg: string) => void;
  renderPane: (side: 'left' | 'right') => void;
}

export class TagController {
  public api: () => any;
  public state: AppState;
  public setStatus: (msg: string) => void;
  public renderPane: (side: 'left' | 'right') => void;

  public fileTags: Record<string, string[]>;
  public customTags: Tag[];
  public activeTagFilter: string | null;
  public isEnabled: boolean = true;

  constructor(deps: TagControllerDeps) {
    this.api = deps.api;
    this.state = deps.state;
    this.setStatus = deps.setStatus;
    this.renderPane = deps.renderPane;

    this.fileTags = {};
    this.customTags = [];
    this.activeTagFilter = null;

    this.loadFromStorage();
  }

  public loadFromStorage(): void {
    try {
      const rawTags = localStorage.getItem('Oryn.fileTags') || localStorage.getItem('Oswin.fileTags');
      const parsed = rawTags ? JSON.parse(rawTags) : null;
      this.fileTags = parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
    } catch {
      this.fileTags = {};
    }

    try {
      const rawCustom = localStorage.getItem('Oryn.customTags') || localStorage.getItem('Oswin.customTags');
      const parsed = rawCustom ? JSON.parse(rawCustom) : null;
      this.customTags = Array.isArray(parsed) ? parsed : [];
    } catch {
      this.customTags = [];
    }
  }

  public saveToStorage(): void {
    try {
      localStorage.setItem('Oryn.fileTags', JSON.stringify(this.fileTags));
      localStorage.setItem('Oryn.customTags', JSON.stringify(this.customTags));
      localStorage.setItem('Oswin.fileTags', JSON.stringify(this.fileTags));
      localStorage.setItem('Oswin.customTags', JSON.stringify(this.customTags));
    } catch {}
  }

  public getAllTags(): Tag[] {
    if (this.isEnabled === false) return [];
    return [...DEFAULT_TAGS, ...this.customTags];
  }

  public getTag(id: string): Tag {
    return this.getAllTags().find((t) => t.id === id) || { id, name: id, color: '#8e8e93' };
  }

  public getTagsForFile(filePath?: string | null): string[] {
    if (this.isEnabled === false) return [];
    if (!filePath) return [];
    const normalized = filePath.replace(/[/\\]+$/, '');
    return this.fileTags[normalized] || [];
  }

  public hasTag(filePath: string, tagId: string): boolean {
    return this.getTagsForFile(filePath).includes(tagId);
  }

  public addTagToFile(filePath?: string | null, tagId?: string): void {
    if (!filePath || !tagId) return;
    const normalized = filePath.replace(/[/\\]+$/, '');
    const current = this.fileTags[normalized] || [];
    if (!current.includes(tagId)) {
      this.fileTags[normalized] = [...current, tagId];
      this.saveToStorage();
      this.refreshAfterTagChange();
    }
  }

  public removeTagFromFile(filePath?: string | null, tagId?: string): void {
    if (!filePath || !tagId) return;
    const normalized = filePath.replace(/[/\\]+$/, '');
    const current = this.fileTags[normalized] || [];
    if (current.includes(tagId)) {
      const next = current.filter((t) => t !== tagId);
      if (next.length === 0) {
        delete this.fileTags[normalized];
      } else {
        this.fileTags[normalized] = next;
      }
      this.saveToStorage();
      this.refreshAfterTagChange();
    }
  }

  public toggleTagForFile(filePath: string, tagId: string): void {
    if (this.hasTag(filePath, tagId)) {
      this.removeTagFromFile(filePath, tagId);
    } else {
      this.addTagToFile(filePath, tagId);
    }
  }

  public clearTagsForFile(filePath?: string | null): void {
    if (!filePath) return;
    const normalized = filePath.replace(/[/\\]+$/, '');
    if (this.fileTags[normalized]) {
      delete this.fileTags[normalized];
      this.saveToStorage();
      this.refreshAfterTagChange();
    }
  }

  public getFilesForTag(tagId: string): string[] {
    const results: string[] = [];
    Object.entries(this.fileTags).forEach(([path, tags]) => {
      if (Array.isArray(tags) && tags.includes(tagId)) {
        results.push(path);
      }
    });
    return results;
  }

  public createCustomTag(name: string, color?: string): Tag | null {
    const trimmed = (name || '').trim();
    if (!trimmed) return null;
    const id = trimmed.toLowerCase().replace(/[^a-z0-9_-]/g, '_');
    if (this.getAllTags().some((t) => t.id === id)) {
      this.setStatus(`Tag "${trimmed}" already exists.`);
      return null;
    }
    const newTag: Tag = { id, name: trimmed, color: color || '#0a84ff', isCustom: true };
    this.customTags.push(newTag);
    this.saveToStorage();
    this.renderSidebar();
    this.setStatus(`Tag "${trimmed}" created.`);
    return newTag;
  }

  public deleteCustomTag(tagId: string): void {
    this.customTags = this.customTags.filter((t) => t.id !== tagId);
    // Remove this tag from all files
    Object.keys(this.fileTags).forEach((path) => {
      if (this.fileTags[path]?.includes(tagId)) {
        this.fileTags[path] = this.fileTags[path].filter((t) => t !== tagId);
        if (this.fileTags[path].length === 0) delete this.fileTags[path];
      }
    });
    this.saveToStorage();
    this.renderSidebar();
    this.refreshAfterTagChange();
  }

  public refreshAfterTagChange(): void {
    (['left', 'right'] as const).forEach((side) => {
      const pane = this.state[side];
      if (pane && Array.isArray(pane.items)) {
        pane.items.forEach((item) => {
          if (item.base !== '..' && item.base !== '') {
            const fp = item.fullPath || (pane.path ? `${pane.path.replace(/[/\\]+$/, '')}/${item.base}` : null);
            item.tags = fp ? this.getTagsForFile(fp) : [];
          }
        });
        pane.listSerial += 1;
        if (this.renderPane) this.renderPane(side);
      }
    });
    this.renderSidebar();
  }

  public renderSidebar(): void {
    if (typeof document === 'undefined') return;
    const nav = document.getElementById('sidebar-tags-nav');
    if (!nav) return;

    if (this.isEnabled === false) {
      nav.replaceChildren();
      return;
    }

    nav.replaceChildren();
    const allTags = this.getAllTags();

    allTags.forEach((tag) => {
      const count = this.getFilesForTag(tag.id).length;
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = `sidebar-item${this.activeTagFilter === tag.id ? ' active' : ''}`;
      btn.dataset.tag = tag.id;

      const dot = document.createElement('span');
      dot.className = `tag-dot tag-dot--${tag.id}`;
      if (tag.color && !tag.id.match(/^(red|orange|yellow|green|blue|purple|gray)$/)) {
        dot.style.background = safeColor(tag.color);
      }

      const label = document.createElement('span');
      label.className = 'sidebar-item-label';
      label.textContent = tag.name;

      btn.append(dot, label);

      if (count > 0) {
        const countBadge = document.createElement('span');
        countBadge.className = 'sidebar-tag-count';
        countBadge.textContent = String(count);
        btn.appendChild(countBadge);
      }

      btn.addEventListener('click', () => {
        document.querySelectorAll('.sidebar-item').forEach((el) => el.classList.remove('active'));
        btn.classList.add('active');
        void this.openTagInActivePane(tag.id);
      });

      if (tag.isCustom) {
        btn.addEventListener('contextmenu', (e) => {
          e.preventDefault();
          const ok = confirm(`Delete custom tag "${tag.name}"?`);
          if (ok) this.deleteCustomTag(tag.id);
        });
      }

      nav.appendChild(btn);
    });
  }

  public async openTagInActivePane(tagId: string): Promise<void> {
    this.activeTagFilter = tagId;
    const tag = this.getTag(tagId);
    const side = this.state.active;
    const pane = this.state[side];

    const taggedPaths = this.getFilesForTag(tagId);
    this.setStatus(`Tag "${tag.name}": ${taggedPaths.length} file(s) found.`);

    if (taggedPaths.length === 0) {
      pane.items = [{ display: ` [No files tagged with "${tag.name}"] `, base: '', isDir: false }];
      pane.cursor = 0;
      pane.listSerial += 1;
      if (this.renderPane) this.renderPane(side);
      return;
    }

    const items: Item[] = [];
    for (const fp of taggedPaths) {
      const base = fp.split(/[/\\]/).pop() || fp;
      let size = null;
      let mtime = null;
      let isDir = false;

      try {
        const props = await this.api().statProps(fp);
        if (props.ok) {
          size = props.props.size;
          mtime = props.props.mtime;
          isDir = props.props.isDir;
        }
      } catch { }

      items.push({
        base,
        display: base,
        fullPath: fp,
        isDir,
        size,
        modified: mtime,
        tags: this.getTagsForFile(fp),
      });
    }

    pane.items = items;
    pane.cursor = 0;
    pane.listSerial += 1;
    if (this.renderPane) this.renderPane(side);
  }

  public setup(): void {
    this.renderSidebar();

    document.getElementById('btn-add-custom-tag')?.addEventListener('click', () => {
      const name = prompt('Enter new tag name (e.g. Work, Project, Personal):');
      if (!name || !name.trim()) return;

      const colors = ['#ff453a', '#ff9f0a', '#ffd60a', '#30d158', '#0a84ff', '#bf5af2', '#ff2d55', '#5856d6', '#64d2ff'];
      const randomColor = colors[Math.floor(Math.random() * colors.length)];
      this.createCustomTag(name.trim(), randomColor);
    });
  }
}
