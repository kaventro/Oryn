// src/modules/listRenderer.ts
import { defaultIconRegistry } from './icons/iconRegistry.ts';
import { RowRenderer, rowSizeText } from './rows/rowRenderer.ts';
import { VirtualScroller } from './virtualList/virtualScroller.ts';

export { rowSizeText };

export interface ListRendererDeps {
  state: any;
  filteredItems: (pane: any) => any[];
  pathHeaderController?: any;
  updatePaneClass?: () => void;
  focusActiveList?: () => void;
  openSelected?: (side: 'left' | 'right') => void;
  showCtxMenu?: (x: number, y: number, side: 'left' | 'right') => void;
  fmtSize?: (bytes?: number | null) => string;
  rowDateText?: (item: any) => string;
  fileExtFromBase?: (item: any) => string;
  onDrop?: (sourceSide: 'left' | 'right', targetSide: 'left' | 'right', payload: any, isCopy: boolean) => void;
  columnsViewController?: any;
  iconRegistry?: any;
}

export class ListRenderer {
  public state: any;
  public filteredItems: (pane: any) => any[];
  public pathHeaderController: any;
  public updatePaneClass?: () => void;
  public focusActiveList?: () => void;
  public openSelected?: (side: 'left' | 'right') => void;
  public showCtxMenu?: (x: number, y: number, side: 'left' | 'right') => void;
  public fmtSize: any;
  public rowDateText: any;
  public fileExtFromBase: any;
  public onDrop?: (sourceSide: 'left' | 'right', targetSide: 'left' | 'right', payload: any, isCopy: boolean) => void;

  public columnsViewController: any;
  public iconRegistry: any;
  public rowRenderer: any;
  public virtualScroller: any;
  public clickSelect: { t: number; side: string; idx: number };

  constructor(deps: ListRendererDeps) {
    this.state = deps.state;
    this.filteredItems = deps.filteredItems;
    this.pathHeaderController = deps.pathHeaderController;
    this.updatePaneClass = deps.updatePaneClass;
    this.focusActiveList = deps.focusActiveList;
    this.openSelected = deps.openSelected;
    this.showCtxMenu = deps.showCtxMenu;
    this.fmtSize = deps.fmtSize;
    this.rowDateText = deps.rowDateText;
    this.fileExtFromBase = deps.fileExtFromBase;
    this.onDrop = deps.onDrop;

    this.columnsViewController = deps.columnsViewController;
    this.iconRegistry = deps.iconRegistry || defaultIconRegistry;
    this.rowRenderer = new RowRenderer({
      iconRegistry: this.iconRegistry,
      fmtSize: this.fmtSize,
      rowDateText: this.rowDateText,
    });
    this.virtualScroller = new VirtualScroller({
      rowStride: 30,
      chunkSize: 8,
      bufferSize: 16,
    });

    this.clickSelect = { t: 0, side: '', idx: -1 };

    this.onPaneListClick = this.onPaneListClick.bind(this);
    this.onPaneListContextMenu = this.onPaneListContextMenu.bind(this);
    this.onPaneListMouseDown = this.onPaneListMouseDown.bind(this);
    this.onPaneDragStart = this.onPaneDragStart.bind(this);
    this.onPaneDragEnter = this.onPaneDragEnter.bind(this);
    this.onPaneDragOver = this.onPaneDragOver.bind(this);
    this.onPaneDragLeave = this.onPaneDragLeave.bind(this);
    this.onPaneDrop = this.onPaneDrop.bind(this);
  }

  get VSTRIDE(): number {
    return this.virtualScroller.rowStride;
  }

  get CHUNK_SIZE(): number {
    return this.virtualScroller.chunkSize;
  }

  get VBUF(): number {
    return this.virtualScroller.bufferSize;
  }

  public listSideFromHost(hostEl: HTMLElement): 'left' | 'right' {
    return hostEl.id === 'list-right' ? 'right' : 'left';
  }

  public _getVis(hostEl: any, pane: any): any[] {
    const visKey = `${pane.listSerial}|${pane.filter}|${pane.sortField}|${pane.sortAsc}|${pane.items.length}`;
    if (hostEl._vVisCacheKey === visKey && hostEl._vVisCache) {
      return hostEl._vVisCache;
    }
    const vis = this.filteredItems(pane);
    hostEl._vVisCacheKey = visKey;
    hostEl._vVisCache = vis;
    return vis;
  }

  public ensureVirtualListChrome(hostEl: any): void {
    this.virtualScroller.ensureChrome(
      hostEl,
      () => this.paintVirtualPane(this.listSideFromHost(hostEl), false),
      this.onPaneListClick,
      this.onPaneListContextMenu,
      this.onPaneListMouseDown,
      this.onPaneDragStart,
      this.onPaneDragEnter,
      this.onPaneDragOver,
      this.onPaneDragLeave,
      this.onPaneDrop
    );

    if (typeof ResizeObserver === 'function' && !hostEl._vResizeObs) {
      let roRaf = 0;
      hostEl._vResizeObs = new ResizeObserver(() => {
        if (roRaf) return;
        roRaf = requestAnimationFrame(() => {
          roRaf = 0;
          hostEl._vPaintKey = '';
          this.paintVirtualPane(this.listSideFromHost(hostEl), false);
        });
      });
      hostEl._vResizeObs.observe(hostEl);
    }
  }

  public onPaneDragStart(e: DragEvent): void {
    const hostEl = e.currentTarget as HTMLElement;
    const row = (e.target as HTMLElement).closest('.row') as HTMLElement;
    if (!row) return;
    const idx = +row.dataset.vidx!;
    const side = this.listSideFromHost(hostEl);
    const pane = this.state[side];
    const vis = this._getVis(hostEl, pane);
    const item = vis[idx];
    if (!item || item.base === '..') {
      e.preventDefault();
      return;
    }

    const selectedSet: Set<string> = pane.activeTab?.selected || new Set();
    let draggedItems: string[] = [];

    if (selectedSet.has(item.base) && selectedSet.size > 1) {
      draggedItems = Array.from(selectedSet);
    } else {
      draggedItems = [item.base];
    }

    const payload = JSON.stringify({ side, base: item.base, items: draggedItems, count: draggedItems.length });
    if (e.dataTransfer) {
      e.dataTransfer.setData('text/plain', payload);
      e.dataTransfer.setData('application/json', payload);
      e.dataTransfer.effectAllowed = 'copyMove';

      // Custom floating badge
      if (draggedItems.length > 1 && typeof document !== 'undefined') {
        const badge = document.createElement('div');
        badge.className = 'drag-ghost-badge';
        badge.textContent = `📁 ${draggedItems.length} items`;
        document.body.appendChild(badge);
        badge.style.top = '-1000px';
        e.dataTransfer.setDragImage(badge, 15, 15);
        setTimeout(() => badge.remove(), 0);
      }
    }
  }

  public onPaneDragEnter(e: DragEvent): void {
    e.preventDefault();
  }

  public onPaneDragOver(e: DragEvent): void {
    const hostEl = e.currentTarget as HTMLElement;
    e.preventDefault();

    const isMove = e.altKey || e.shiftKey;
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = isMove ? 'move' : 'copy';
    }
    hostEl.classList.add('drag-over');

    // Highlight folder row if hovering over a subdirectory
    const targetRow = (e.target as HTMLElement)?.closest('.row') as HTMLElement;
    document.querySelectorAll('.row--drag-target').forEach((el) => {
      if (el !== targetRow) el.classList.remove('row--drag-target');
    });
    if (targetRow) {
      const idx = +(targetRow.dataset.vidx || -1);
      const side = this.listSideFromHost(hostEl);
      const item = this._getVis(hostEl, this.state[side])[idx];
      if (item && item.isDir && item.base !== '..') {
        targetRow.classList.add('row--drag-target');
      }
    }
  }

  public onPaneDragLeave(e: DragEvent): void {
    const hostEl = e.currentTarget as HTMLElement;
    hostEl.classList.remove('drag-over');
    document.querySelectorAll('.row--drag-target').forEach((el) => el.classList.remove('row--drag-target'));
  }

  public onPaneDrop(e: DragEvent): void {
    const hostEl = e.currentTarget as HTMLElement;
    e.preventDefault();
    e.stopPropagation();
    hostEl.classList.remove('drag-over');
    document.querySelectorAll('.row--drag-target').forEach((el) => el.classList.remove('row--drag-target'));

    if (!e.dataTransfer) return;
    let dataStr = e.dataTransfer.getData('application/json');
    if (!dataStr) dataStr = e.dataTransfer.getData('text/plain');
    if (!dataStr) return;

    try {
      const data = JSON.parse(dataStr);
      const sourceSide = data.side;
      const targetSide = this.listSideFromHost(hostEl);
      const isCopy = !e.altKey && !e.shiftKey;

      if (this.onDrop) {
        this.onDrop(sourceSide, targetSide, data, isCopy);
      }
    } catch (err) {
      console.error('Drop error:', err);
    }
  }

  public onPaneListClick(e: MouseEvent): void {
    const hostEl = e.currentTarget as HTMLElement;
    const row = (e.target as HTMLElement).closest('.row') as HTMLElement;
    if (!row || row.parentElement !== (hostEl as any)._vWin) return;
    e.stopPropagation();

    const side = this.listSideFromHost(hostEl);
    const pane = this.state[side];
    const idx = +row.dataset.vidx!;
    if (Number.isNaN(idx)) return;

    const now = Date.now();
    if (this.clickSelect.side === side && this.clickSelect.idx === idx && now - this.clickSelect.t < 450) {
      this.clickSelect.t = 0;
      pane.cursor = idx;
      this.paintVirtualPane(side, false);
      if (this.openSelected) void this.openSelected(side);
      return;
    }
    this.clickSelect = { t: now, side, idx };

    this.state.active = side;
    pane.cursor = idx;

    if (e.shiftKey && pane.activeTab && !pane.activeTab.selected.size) {
      // Range select logic
    }

    if (this.updatePaneClass) this.updatePaneClass();
    this.paintVirtualPane('left', false);
    this.paintVirtualPane('right', false);
    if (this.focusActiveList) this.focusActiveList();
    else hostEl.focus({ preventScroll: true });
  }

  public onPaneListContextMenu(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    const hostEl = e.currentTarget as HTMLElement;
    const side = this.listSideFromHost(hostEl);
    const row = (e.target as HTMLElement).closest('.row') as HTMLElement;
    if (row && row.parentElement === (hostEl as any)._vWin) {
      const idx = +row.dataset.vidx!;
      if (!Number.isNaN(idx)) {
        this.state[side].cursor = idx;
        this.state.active = side;
        if (this.updatePaneClass) this.updatePaneClass();
        this.paintVirtualPane('left', false);
        this.paintVirtualPane('right', false);
        if (this.focusActiveList) this.focusActiveList();
      }
    }
    if (this.showCtxMenu) this.showCtxMenu(e.clientX, e.clientY, side);
  }

  public onPaneListMouseDown(e: MouseEvent): void {
    const hostEl = e.currentTarget as HTMLElement;
    const side = this.listSideFromHost(hostEl);
    if (this.state.active !== side) {
      this.state.active = side;
      if (this.updatePaneClass) this.updatePaneClass();
      this.paintVirtualPane('left', false);
      this.paintVirtualPane('right', false);
    }
    if (this.focusActiveList) this.focusActiveList();
    else hostEl.focus({ preventScroll: true });
    if (e.button === 2) {
      this.onPaneListContextMenu(e);
    }
  }

  public moveCursor(side: 'left' | 'right', delta: number): void {
    const pane = this.state[side];
    const vis = this.filteredItems(pane);
    if (!vis || !vis.length) return;
    const maxIdx = Math.max(0, vis.length - 1);
    pane.cursor = Math.min(Math.max(0, (pane.cursor ?? 0) + delta), maxIdx);
    this.paintVirtualPane(side, true);
  }

  public paintVirtualPane(side: 'left' | 'right', align: boolean = false): void {
    const hostEl = document.getElementById(`list-${side}`) as HTMLElement & {
      _vInner?: HTMLElement;
      _vWin?: HTMLElement;
      _vPaintKey?: string;
      _vVisCache?: any[];
      _vVisCacheKey?: string;
    };
    if (!hostEl) return;
    this.ensureVirtualListChrome(hostEl);

    const pane = this.state[side];
    const vis = this._getVis(hostEl, pane);
    const cursor = pane.cursor ?? 0;

    if (align) {
      this.virtualScroller.scrollCursorIntoView(hostEl, cursor, vis.length);
    }

    const { totalH, start, need, offsetY } = this.virtualScroller.computeWindow(
      vis.length,
      hostEl.clientHeight,
      hostEl.scrollTop
    );

    const inner = hostEl._vInner;
    const win = hostEl._vWin;
    if (!inner || !win) return;

    inner.style.height = `${totalH}px`;
    win.style.transform = `translateY(${offsetY}px)`;

    const paintKey = `${side}|${start}|${need}|${cursor}|${pane.activeTab?.selectedBases?.size ?? 0}|${pane.listSerial ?? 0}`;
    if (hostEl._vPaintKey === paintKey && !align) return;
    hostEl._vPaintKey = paintKey;

    this.virtualScroller.syncWindowNodes(win, vis, pane, start, need, this.rowRenderer);
  }
}
