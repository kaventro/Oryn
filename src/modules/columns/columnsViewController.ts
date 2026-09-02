// src/modules/columns/columnsViewController.ts
import { applyGitStatusToItems, fetchGitSnapshot } from '../gitStatusMapper.ts';

export interface ColumnItem {
  base: string;
  isDir: boolean;
  size: number;
  mtime: string | number;
  ext: string;
  gitStatus?: string | null;
}

export interface ColumnStack {
  path: string;
  items: ColumnItem[];
  selectedIndex: number;
  selectedItem: ColumnItem | null;
}

export interface ColumnsViewDeps {
  api: () => any;
  iconRegistry?: any;
  rowRenderer?: any;
  onOpenSelected?: (path: string, isDir: boolean) => void;
  onPreviewSelected?: (fp: string, item: ColumnItem) => void;
  showCtxMenu?: (x: number, y: number, side: 'left' | 'right', emptyArea: boolean, item: any, dirPath: string) => void;
  onDrop?: (sourceSide: 'left' | 'right', targetSide: 'left' | 'right', base: string, isCopy: boolean) => void;
  onActivateSide?: (side: 'left' | 'right', activePath?: string) => void;
  setStatus?: (msg: string) => void;
}

export class ColumnsViewController {
  private api: () => any;
  public iconRegistry?: any;
  public onOpenSelected?: (path: string, isDir: boolean) => void;
  public onPreviewSelected?: (fp: string, item: ColumnItem) => void;
  public showCtxMenu?: (x: number, y: number, side: 'left' | 'right', emptyArea: boolean, item: any, dirPath: string) => void;
  public onDrop?: (sourceSide: 'left' | 'right', targetSide: 'left' | 'right', base: string, isCopy: boolean) => void;
  public onActivateSide?: (side: 'left' | 'right', activePath?: string) => void;
  public setStatus?: (msg: string) => void;
  public side: 'left' | 'right' = 'left';
  public paneColumns: Record<'left' | 'right', ColumnStack[]> = { left: [], right: [] };
  public activeColIndexes: Record<'left' | 'right', number> = { left: 0, right: 0 };

  constructor(deps: ColumnsViewDeps) {
    this.api = deps.api;
    this.iconRegistry = deps.iconRegistry;
    this.onOpenSelected = deps.onOpenSelected;
    this.onPreviewSelected = deps.onPreviewSelected;
    this.showCtxMenu = deps.showCtxMenu;
    this.onDrop = deps.onDrop;
    this.onActivateSide = deps.onActivateSide;
    this.setStatus = deps.setStatus;
  }

  getColumns(side: 'left' | 'right' = this.side): ColumnStack[] {
    return this.paneColumns[side] || [];
  }

    getActiveColumn(side: 'left' | 'right' = this.side): { path: string; selectedItem: ColumnItem | null; selectedIndex: number; colIndex: number } | null {
    const cols = this.paneColumns[side] || [];
    if (!cols.length) return null;
    
    // Find the rightmost column that has a selected item
    for (let i = cols.length - 1; i >= 0; i--) {
      if (cols[i]?.selectedItem?.base && cols[i]?.selectedItem?.base !== '..') {
        return { ...cols[i], colIndex: i };
      }
    }
    
    // Fallback to activeColIndex or rightmost column
    const activeIdx = Math.min(Math.max(0, this.activeColIndexes[side] || 0), cols.length - 1);
    return { ...cols[activeIdx], colIndex: activeIdx };
  }

  getActiveColumnIndex(side: 'left' | 'right' = this.side): number {
    return this.activeColIndexes[side] || 0;
  }

  async syncPane(side: 'left' | 'right', pane: any): Promise<void> {
    this.side = side;
    const rootPath = pane.path;
    const cols = this.paneColumns[side];
    if (!cols || cols.length === 0) {
      await this.loadRoot(side, pane);
      return;
    }
    const matchingIdx = cols.findIndex((c) => c.path === rootPath);
    if (matchingIdx >= 0) {
      this.activeColIndexes[side] = matchingIdx;
      for (const col of cols) {
        col.items = await this.fetchDirectory(col.path);
        if (col.selectedIndex >= 0 && col.items[col.selectedIndex]) {
          col.selectedItem = col.items[col.selectedIndex];
        }
      }
      this.render(side);
      this.scrollToColumn(side, matchingIdx);
    } else if (cols[0].path !== rootPath) {
      await this.loadRoot(side, pane);
    } else {
      for (const col of cols) {
        col.items = await this.fetchDirectory(col.path);
        if (col.selectedIndex >= 0 && col.items[col.selectedIndex]) {
          col.selectedItem = col.items[col.selectedIndex];
        }
      }
      this.render(side);
    }
  }

  /**
   * Initializes or syncs the column stack for a given pane
   */
  async loadRoot(side: 'left' | 'right', pane: any): Promise<void> {
    this.side = side;
    const rootPath = pane.path;
    this.activeColIndexes[side] = 0;

    let items: ColumnItem[] = (pane.items && pane.items.length)
      ? pane.items
          .filter((i: any) => {
            const b = i.base || i.display || i.name || '';
            return b && b !== '..' && !b.startsWith(' [');
          })
          .map((i: any) => ({
            base: i.base || i.display || i.name || '',
            isDir: Boolean(i.isDir || i.is_dir),
            size: i.size || 0,
            mtime: i.mtime || '',
            ext: i.ext || (i.base || i.name || '').split('.').pop()?.toLowerCase() || '',
            gitStatus: i.gitStatus || null,
          }))
      : await this.fetchDirectory(rootPath);

    if (items.length && pane.items && pane.items.length) {
      const apiObj = typeof this.api === 'function' ? this.api() : this.api;
      if (typeof apiObj?.gitIsRepo === 'function') {
        const snapshot = await fetchGitSnapshot(apiObj, rootPath);
        applyGitStatusToItems(rootPath, items, snapshot);
      }
    }

    this.paneColumns[side] = [
      {
        path: rootPath,
        items,
        selectedIndex: -1,
        selectedItem: null,
      },
    ];

    this.render(side);
  }

  /**
   * User clicks or selects an item inside column colIndex
   */
  async selectItem(colIndex: number, itemIndex: number, side: 'left' | 'right' = this.side): Promise<void> {
    this.side = side;
    const cols = this.paneColumns[side];
    if (!cols || colIndex < 0 || colIndex >= cols.length) return;
    const col = cols[colIndex];
    if (itemIndex < 0 || itemIndex >= col.items.length) return;

    col.selectedIndex = itemIndex;
    const item = col.items[itemIndex];
    col.selectedItem = item;
    this.activeColIndexes[side] = colIndex;

    // Truncate any child columns beyond this one
    this.paneColumns[side] = cols.slice(0, colIndex + 1);

    let activePath = col.path;
    if (item && item.isDir && item.base !== '..') {
      const subPath = await this.joinPath(col.path, item.base);
      activePath = subPath;
      const subItems = await this.fetchDirectory(subPath);
      this.paneColumns[side].push({
        path: subPath,
        items: subItems,
        selectedIndex: -1,
        selectedItem: null,
      });
    }

    if (this.onActivateSide) {
      this.onActivateSide(side, activePath);
    }

    this.render(side);
    this.scrollToEnd(side);
  }

  /**
   * Navigate back to previous column (parent folder) or prepend parent directory without leaving column view
   */
  async goBack(side: 'left' | 'right' = this.side): Promise<void> {
    this.side = side;
    const cols = this.paneColumns[side];
    if (!cols || cols.length === 0) return;
    const activeIdx = this.activeColIndexes[side] || 0;

    if (activeIdx > 0) {
      this.activeColIndexes[side] = activeIdx - 1;
      for (let i = this.activeColIndexes[side] + 1; i < cols.length; i++) {
        cols[i].selectedIndex = -1;
        cols[i].selectedItem = null;
      }
      const prevCol = cols[this.activeColIndexes[side]];
      if (this.onActivateSide && prevCol) {
        this.onActivateSide(side, prevCol.path);
      }
      this.render(side);
      this.scrollToColumn(side, this.activeColIndexes[side]);
    } else {
      const curCol = cols[0];
      if (!curCol) return;
      const curPath = curCol.path;
      const apiObj = typeof this.api === 'function' ? this.api() : this.api;
      const parentPath = await apiObj.pathDirname?.(curPath);
      if (parentPath && parentPath !== curPath) {
        const parentItems = await this.fetchDirectory(parentPath);
        const curBase = curPath.split(/[/|\\]/).filter(Boolean).pop() || '';
        const matchingIdx = parentItems.findIndex((it) => it.base.toLowerCase() === curBase.toLowerCase());

        this.paneColumns[side].unshift({
          path: parentPath,
          items: parentItems,
          selectedIndex: matchingIdx >= 0 ? matchingIdx : 0,
          selectedItem: matchingIdx >= 0 ? parentItems[matchingIdx] : (parentItems[0] || null),
        });
        this.activeColIndexes[side] = 0;
        if (this.onActivateSide) {
          this.onActivateSide(side, parentPath);
        }
        this.render(side);
        this.scrollToColumn(side, 0);
      }
    }
  }

  /**
   * Navigate forward to child column or step into selected directory
   */
  async goForward(side: 'left' | 'right' = this.side): Promise<void> {
    this.side = side;
    const cols = this.paneColumns[side];
    if (!cols || cols.length === 0) return;
    const activeIdx = this.activeColIndexes[side] || 0;

    if (activeIdx + 1 < cols.length) {
      this.activeColIndexes[side] = activeIdx + 1;
      const nextCol = cols[this.activeColIndexes[side]];
      if (nextCol.selectedIndex === -1 && nextCol.items.length > 0) {
        await this.selectItem(activeIdx + 1, 0, side);
      } else {
        if (this.onActivateSide && nextCol) {
          this.onActivateSide(side, nextCol.path);
        }
        this.render(side);
        this.scrollToColumn(side, this.activeColIndexes[side]);
      }
    } else {
      const curCol = cols[activeIdx];
      if (curCol && curCol.selectedItem && curCol.selectedItem.isDir && curCol.selectedItem.base !== '..') {
        const subPath = await this.joinPath(curCol.path, curCol.selectedItem.base);
        const subItems = await this.fetchDirectory(subPath);
        this.paneColumns[side].push({
          path: subPath,
          items: subItems,
          selectedIndex: subItems.length > 0 ? 0 : -1,
          selectedItem: subItems.length > 0 ? subItems[0] : null,
        });
        this.activeColIndexes[side] = activeIdx + 1;
        if (this.onActivateSide) {
          this.onActivateSide(side, subPath);
        }
        this.render(side);
        this.scrollToEnd(side);
      }
    }
  }

  /**
   * Handles keyboard arrow navigation across Miller columns
   */
  async navigate(deltaX: number, deltaY: number, side: 'left' | 'right' = this.side): Promise<void> {
    this.side = side;
    const cols = this.paneColumns[side];
    if (!cols || cols.length === 0) return;
    const activeIdx = this.activeColIndexes[side] || 0;

    if (deltaY !== 0) {
      const col = cols[activeIdx];
      if (!col || col.items.length === 0) return;
      const cur = col.selectedIndex >= 0 ? col.selectedIndex : 0;
      const nextIdx = Math.min(Math.max(0, cur + deltaY), col.items.length - 1);
      await this.selectItem(activeIdx, nextIdx, side);
    } else if (deltaX > 0) {
      await this.goForward(side);
    } else if (deltaX < 0) {
      await this.goBack(side);
    }
  }

  scrollToColumn(side: 'left' | 'right', colIndex: number): void {
    if (typeof document === 'undefined') return;
    const paneBody = document.querySelector(`#pane-${side} .pane-body`);
    const container = paneBody?.querySelector('.columns-container') as HTMLElement | null;
    if (!container) return;
    const colEls = container.querySelectorAll('.columns-column');
    const targetCol = colEls[colIndex] as HTMLElement | null;
    if (targetCol) {
      targetCol.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'nearest' });
      const selectedRow = targetCol.querySelector('.row.selected') as HTMLElement | null;
      if (selectedRow) {
        selectedRow.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }
    }
  }

  /**
   * Renders the columns container DOM for a pane
   */
  render(side: 'left' | 'right' = this.side): void {
    if (typeof document === 'undefined') return;
    const paneBody = document.querySelector(`#pane-${side} .pane-body`);
    if (!paneBody) return;

    let container = paneBody.querySelector('.columns-container') as HTMLElement | null;
    if (!container) {
      container = document.createElement('div');
      container.className = 'columns-container';
      paneBody.appendChild(container);
    }

    container.onmousedown = () => {
      this.side = side;
      const curCols = this.paneColumns[side] || [];
      const curActiveIdx = this.activeColIndexes[side] || 0;
      const curCol = curCols[curActiveIdx];
      if (this.onActivateSide && curCol) {
        this.onActivateSide(side, curCol.path);
      }
    };

    const cols = this.paneColumns[side] || [];
    const activeColIdx = this.activeColIndexes[side] || 0;

    const savedScrollTops = Array.from(container.querySelectorAll('.columns-column')).map(
      (el) => (el as HTMLElement).scrollTop || 0,
    );

    container.replaceChildren();

    cols.forEach((col, colIdx) => {
      const colEl = document.createElement('div');
      colEl.className = `columns-column ${colIdx === activeColIdx ? 'active-col' : ''}`;
      colEl.dataset.colidx = String(colIdx);

      colEl.addEventListener('click', (e) => {
        if ((e.target as HTMLElement).closest('.row')) return;
        this.side = side;
        this.activeColIndexes[side] = colIdx;
        if (this.onActivateSide) {
          this.onActivateSide(side, col.path);
        }
        this.render(side);
      });

      colEl.addEventListener('contextmenu', (e) => {
        if ((e.target as HTMLElement).closest('.row')) return;
        e.preventDefault();
        e.stopPropagation();
        this.activeColIndexes[side] = colIdx;
        this.render(side);
        if (this.showCtxMenu) {
          this.showCtxMenu(e.clientX, e.clientY, side, true, null, col.path);
        }
      });

      col.items.forEach((item, itemIdx) => {
        const isSelected = col.selectedIndex === itemIdx;
        const rowEl = document.createElement('div');
        rowEl.className = `row ${item.isDir ? 'dir' : 'file'} ${isSelected ? 'selected' : ''}`;
        rowEl.draggable = true;

        const nameEl = document.createElement('div');
        nameEl.className = 'row-name';

        const iconKey = this.iconRegistry?.resolveIconKey?.(item) || 'file';
        const iconSvg = this.iconRegistry?.getSvg?.(iconKey) || '';
        const iconWrap = document.createElement('span');
        iconWrap.className = 'row-icon';
        iconWrap.innerHTML = iconSvg;

        const textEl = document.createElement('span');
        textEl.className = 'row-name-text';
        textEl.textContent = item.base;

        nameEl.appendChild(iconWrap);
        nameEl.appendChild(textEl);
        if (item.gitStatus) {
          const gitBadge = document.createElement('span');
          gitBadge.className = `git-status-tag git-status-tag--${item.gitStatus}`;
          gitBadge.textContent = item.gitStatus;
          nameEl.appendChild(gitBadge);
        }
        rowEl.appendChild(nameEl);

        rowEl.addEventListener('click', (e) => {
          e.stopPropagation();
          void this.selectItem(colIdx, itemIdx, side);
        });

        rowEl.addEventListener('dblclick', (e) => {
          e.stopPropagation();
          if (this.onOpenSelected) {
            void this.joinPath(col.path, item.base).then((fp) => {
              this.onOpenSelected?.(fp, item.isDir);
            });
          }
        });

        rowEl.addEventListener('contextmenu', (e) => {
          e.preventDefault();
          e.stopPropagation();
          this.activeColIndexes[side] = colIdx;
          col.selectedIndex = itemIdx;
          col.selectedItem = item;
          this.render(side);
          if (this.showCtxMenu) {
            this.showCtxMenu(e.clientX, e.clientY, side, false, item, col.path);
          }
        });

        colEl.appendChild(rowEl);
      });

      if (col.items.length === 0) {
        const emptyEl = document.createElement('div');
        emptyEl.className = 'row';
        emptyEl.style.color = '#8e8e93';
        emptyEl.style.fontStyle = 'italic';
        emptyEl.style.fontSize = '12px';
        emptyEl.style.justifyContent = 'center';
        emptyEl.style.padding = '8px';
        emptyEl.textContent = 'Empty Folder';
        colEl.appendChild(emptyEl);
      }

      container!.appendChild(colEl);

      if (savedScrollTops[colIdx] != null) {
        colEl.scrollTop = savedScrollTops[colIdx];
      }
    });

    const lastCol = cols[cols.length - 1];
    if (lastCol && lastCol.selectedItem && !lastCol.selectedItem.isDir) {
      const inspector = this.createInspectorElement(lastCol.selectedItem, lastCol.path);
      container.appendChild(inspector);
    }
  }

  createInspectorElement(item: ColumnItem, dirPath: string): HTMLElement {
    const card = document.createElement('div');
    card.className = 'columns-preview-column';

    const ext = (item.ext || '').toLowerCase();
    const isImage = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'bmp', 'ico'].includes(ext);

    const iconEl = document.createElement('div');
    iconEl.className = 'columns-preview-icon';

    if (isImage) {
      const img = document.createElement('img');
      img.style.maxWidth = '100%';
      img.style.maxHeight = '140px';
      img.style.objectFit = 'contain';
      img.style.borderRadius = '6px';
      img.alt = item.base;
      void this.joinPath(dirPath, item.base).then((fp) => {
        try {
          const apiObj = typeof this.api === 'function' ? this.api() : this.api;
          if (apiObj?.assetUrl) {
            img.onerror = async () => {
              try {
                if (apiObj?.readMediaDataUrl) {
                  const dataUrl = await apiObj.readMediaDataUrl(fp);
                  img.onerror = null;
                  img.src = dataUrl;
                }
              } catch (_) {}
            };
            img.src = apiObj.assetUrl(fp);
          }
        } catch (_) {}
      });
      iconEl.appendChild(img);
    } else {
      const iconKey = this.iconRegistry?.resolveIconKey?.(item) || 'file';
      const iconSvg = this.iconRegistry?.getSvg?.(iconKey) || '';
      iconEl.innerHTML = iconSvg;
    }

    const nameEl = document.createElement('div');
    nameEl.className = 'columns-preview-name';
    nameEl.textContent = item.base;

    const metaTable = document.createElement('div');
    metaTable.className = 'columns-preview-meta';

    const addMetaRow = (label: string, val: string | number) => {
      const row = document.createElement('div');
      row.className = 'columns-meta-row';
      const l = document.createElement('span');
      l.className = 'columns-meta-label';
      l.textContent = label;
      const v = document.createElement('span');
      v.className = 'columns-meta-val';
      v.textContent = String(val);
      row.appendChild(l);
      row.appendChild(v);
      metaTable.appendChild(row);
    };

    addMetaRow('Kind', (item.ext || 'File').toUpperCase());
    if (item.size) addMetaRow('Size', `${item.size} B`);
    if (item.mtime) addMetaRow('Modified', String(item.mtime));

    const actions = document.createElement('div');
    actions.className = 'columns-preview-actions';

    const previewBtn = document.createElement('button');
    previewBtn.className = 'mac-toolbar-btn mac-toolbar-btn--primary';
    previewBtn.textContent = 'Quick Look';
    previewBtn.onclick = async () => {
      const fp = await this.joinPath(dirPath, item.base);
      if (this.onPreviewSelected) this.onPreviewSelected(fp, item);
    };

    actions.appendChild(previewBtn);

    card.appendChild(iconEl);
    card.appendChild(nameEl);
    card.appendChild(metaTable);
    card.appendChild(actions);

    return card;
  }

  scrollToEnd(side: 'left' | 'right' = this.side): void {
    if (typeof document === 'undefined') return;
    const paneBody = document.querySelector(`#pane-${side} .pane-body`);
    const container = paneBody?.querySelector('.columns-container') as HTMLElement | null;
    if (container) {
      container.scrollTo({ left: container.scrollWidth, behavior: 'smooth' });
    }
  }

  async fetchDirectory(dirPath: string): Promise<ColumnItem[]> {
    try {
      const apiObj = typeof this.api === 'function' ? this.api() : this.api;
      const res = await apiObj.readDir(dirPath);
      const raw = Array.isArray(res) ? res : (res?.items || res?.entries || []);
      const entries = Array.isArray(raw) ? raw : [];
      const items: ColumnItem[] = entries
        .filter((e: any) => {
          const b = e.base || e.name || '';
          return b && b !== '..';
        })
        .map((e: any) => ({
          base: e.base || e.name || '',
          isDir: Boolean(e.isDir || e.is_dir),
          size: e.size || 0,
          mtime: e.mtime || e.modified || 0,
          ext: e.ext || (e.base || e.name || '').split('.').pop()?.toLowerCase() || '',
          gitStatus: e.gitStatus || null,
        }));
      const snapshot = await fetchGitSnapshot(apiObj, dirPath);
      applyGitStatusToItems(dirPath, items, snapshot);
      return items;
    } catch (_) {
      return [];
    }
  }

  async joinPath(parent: string, child: string): Promise<string> {
    try {
      const apiObj = typeof this.api === 'function' ? this.api() : this.api;
      if (apiObj?.pathJoin) {
        return await apiObj.pathJoin(parent, child);
      }
    } catch (_) {}
    const sep = parent.includes('\\') ? '\\' : '/';
    return parent.endsWith(sep) ? `${parent}${child}` : `${parent}${sep}${child}`;
  }
}
