// src/modules/rows/rowRenderer.ts
import { fmtSizeExact } from '../formatUtils.ts';
import type { IconRegistry } from '../icons/iconRegistry.ts';

export interface PaneItem {
  base: string;
  display?: string;
  isDir?: boolean;
  size?: number | null;
  tags?: string[];
  gitStatus?: string;
  [key: string]: any;
}

export interface PaneState {
  cursor: number;
  activeTab: {
    selectedBases: Set<string>;
    [key: string]: any;
  };
  [key: string]: any;
}

export interface RowRendererDeps {
  iconRegistry: IconRegistry;
  fmtSize: (sz: number | null | undefined, isDir?: boolean) => string;
  rowDateText: (item: PaneItem) => string;
}

export function rowSizeText(item: PaneItem, fmtSize: (sz: number | null | undefined, isDir?: boolean) => string): string {
  if (item.base === '..') return '';
  if (item.isDir) {
    if (item.size != null) return fmtSize(item.size, true);
    return '<DIR>';
  }
  return fmtSize(item.size, false);
}

export class RowRenderer {
  public iconRegistry: IconRegistry;
  public fmtSize: (sz: number | null | undefined, isDir?: boolean) => string;
  public rowDateText: (item: PaneItem) => string;

  constructor(deps: RowRendererDeps) {
    this.iconRegistry = deps.iconRegistry;
    this.fmtSize = deps.fmtSize;
    this.rowDateText = deps.rowDateText;
  }

  public createRow(pane: PaneState, item: PaneItem, idx: number): HTMLDivElement {
    const row = document.createElement('div') as HTMLDivElement & {
      _iconKey?: string;
      _display?: string;
      _tagsKey?: string;
      _gitStatus?: string;
      _dateText?: string;
      _sizeText?: string;
    };
    const isDir = Boolean(item.isDir);
    const isUserSel = pane.activeTab.selectedBases.has(item.base);
    const isSel = idx === pane.cursor;

    row.className = 'row' + (isDir ? ' dir' : '') + (isUserSel ? ' user-selected' : '') + (isSel ? ' selected' : '');
    row.draggable = true;
    row.dataset.vidx = String(idx);

    const name = document.createElement('span');
    name.className = 'row-name';

    const icon = document.createElement('span');
    icon.className = 'row-icon';
    const iconKey = this.iconRegistry.resolveIconKey(item);
    icon.innerHTML = this.iconRegistry.getSvg(iconKey);
    row._iconKey = iconKey;

    const nameText = document.createElement('span');
    nameText.className = 'row-name-text';
    nameText.textContent = item.display || item.base;
    row._display = item.display;
    name.append(icon, nameText);

    if (item.tags && item.tags.length > 0) {
      const tagsWrap = document.createElement('span');
      tagsWrap.className = 'row-tag-dots';
      item.tags.forEach((t) => {
        const dot = document.createElement('span');
        dot.className = `tag-dot tag-dot--${t}`;
        dot.title = `Tag: ${t}`;
        tagsWrap.appendChild(dot);
      });
      name.append(tagsWrap);
      row._tagsKey = item.tags.join(',');
    } else {
      row._tagsKey = '';
    }

    if (item.gitStatus) {
      const gitBadge = document.createElement('span');
      gitBadge.className = `git-status-tag git-status-tag--${item.gitStatus}`;
      gitBadge.textContent = item.gitStatus;
      name.append(gitBadge);
      row._gitStatus = item.gitStatus;
    } else {
      row._gitStatus = '';
    }

    const date = document.createElement('span');
    date.className = 'row-date';
    const dateText = this.rowDateText(item);
    date.textContent = dateText;
    row._dateText = dateText;

    const size = document.createElement('span');
    size.className = 'row-size';
    const sizeText = rowSizeText(item, this.fmtSize);
    size.textContent = sizeText;
    row._sizeText = sizeText;
    if (item.base !== '..' && item.size != null) size.title = fmtSizeExact(item.size);

    row.append(name, date, size);
    return row;
  }

  public syncRow(row: HTMLDivElement & {
    _iconKey?: string;
    _display?: string;
    _tagsKey?: string;
    _gitStatus?: string;
    _dateText?: string;
    _sizeText?: string;
  }, item: PaneItem, pane: PaneState, idx: number): void {
    const isDir = Boolean(item.isDir);
    const isUserSel = pane.activeTab.selectedBases.has(item.base);
    const isSel = idx === pane.cursor;

    row.className = 'row' + (isDir ? ' dir' : '') + (isUserSel ? ' user-selected' : '') + (isSel ? ' selected' : '');
    row.draggable = true;
    row.dataset.vidx = String(idx);

    const name = row.children[0] as HTMLElement;
    const icon = name.children[0] as HTMLElement;
    const nameText = name.children[1] as HTMLElement;

    const iconKey = this.iconRegistry.resolveIconKey(item);
    if (row._iconKey !== iconKey) {
      icon.innerHTML = this.iconRegistry.getSvg(iconKey);
      row._iconKey = iconKey;
    }

    const display = item.display || item.base;
    if (row._display !== display) {
      nameText.textContent = display;
      row._display = display;
    }

    const tagsKey = item.tags ? item.tags.join(',') : '';
    if (row._tagsKey !== tagsKey) {
      row._tagsKey = tagsKey;
      let tagsWrap = name.querySelector('.row-tag-dots') as HTMLElement | null;
      if (item.tags && item.tags.length > 0) {
        if (!tagsWrap) {
          tagsWrap = document.createElement('span');
          tagsWrap.className = 'row-tag-dots';
          name.appendChild(tagsWrap);
        }
        tagsWrap.replaceChildren();
        item.tags.forEach((t) => {
          const dot = document.createElement('span');
          dot.className = `tag-dot tag-dot--${t}`;
          dot.title = `Tag: ${t}`;
          tagsWrap!.appendChild(dot);
        });
      } else if (tagsWrap) {
        tagsWrap.remove();
      }
    }

    const gitStatus = item.gitStatus || '';
    if (row._gitStatus !== gitStatus) {
      row._gitStatus = gitStatus;
      let gitBadge = name.querySelector('.git-status-tag') as HTMLElement | null;
      if (gitStatus) {
        if (!gitBadge) {
          gitBadge = document.createElement('span');
          name.appendChild(gitBadge);
        }
        gitBadge.className = `git-status-tag git-status-tag--${gitStatus}`;
        gitBadge.textContent = gitStatus;
      } else if (gitBadge) {
        gitBadge.remove();
      }
    }

    const dateText = this.rowDateText(item);
    if (row._dateText !== dateText) {
      row.children[1].textContent = dateText;
      row._dateText = dateText;
    }

    const sizeText = rowSizeText(item, this.fmtSize);
    if (row._sizeText !== sizeText) {
      const sizeEl = row.children[2] as HTMLElement;
      sizeEl.textContent = sizeText;
      row._sizeText = sizeText;
      if (item.base !== '..' && item.size != null) {
        sizeEl.title = fmtSizeExact(item.size);
      } else {
        sizeEl.removeAttribute('title');
      }
    }
  }
}
