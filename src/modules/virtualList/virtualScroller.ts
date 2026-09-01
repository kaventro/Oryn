// src/modules/virtualList/virtualScroller.ts
import type { RowRenderer, PaneItem, PaneState } from '../rows/rowRenderer.ts';

export interface VirtualWindowCalculation {
  totalH: number;
  start: number;
  end: number;
  need: number;
  offsetY: number;
}

export interface VirtualScrollerConfig {
  rowStride?: number;
  chunkSize?: number;
  bufferSize?: number;
}

export class VirtualScroller {
  public rowStride: number;
  public chunkSize: number;
  public bufferSize: number;

  constructor(config?: VirtualScrollerConfig) {
    this.rowStride = config?.rowStride ?? 30;
    this.chunkSize = config?.chunkSize ?? 8;
    this.bufferSize = config?.bufferSize ?? 16;
  }

  /**
   * Computes visible row range and offset for virtualization
   */
  public computeWindow(totalItems: number, clientHeight: number, scrollTop: number): VirtualWindowCalculation {
    const totalH = totalItems * this.rowStride;
    const sh = clientHeight || 1;
    const maxScroll = Math.max(0, totalH - sh);
    const st = Math.min(Math.max(0, scrollTop), maxScroll);

    const chunk = Math.floor(st / (this.chunkSize * this.rowStride));
    const chunkStart = chunk * this.chunkSize;
    const visibleCount = Math.ceil(sh / this.rowStride);
    const start = Math.max(0, chunkStart - this.bufferSize);
    const end = Math.min(totalItems, chunkStart + this.chunkSize + visibleCount + this.bufferSize);
    const need = Math.max(0, end - start);
    const offsetY = start * this.rowStride;

    return {
      totalH,
      start,
      end,
      need,
      offsetY,
    };
  }

  /**
   * Ensures virtual inner and window container elements exist
   */
  public ensureChrome(
    hostEl: HTMLElement,
    onScroll: () => void,
    onClick: (e: MouseEvent) => void,
    onContextMenu: (e: MouseEvent) => void,
    onMouseDown: (e: MouseEvent) => void,
    onDragStart: (e: DragEvent) => void,
    onDragEnter: (e: DragEvent) => void,
    onDragOver: (e: DragEvent) => void,
    onDragLeave: (e: DragEvent) => void,
    onDrop: (e: DragEvent) => void
  ): { inner: HTMLElement; win: HTMLElement } {
    const el = hostEl as HTMLElement & { _vInner?: HTMLElement; _vWin?: HTMLElement };
    if (el._vInner && el._vWin) {
      return { inner: el._vInner, win: el._vWin };
    }

    const inner = document.createElement('div');
    inner.className = 'virtual-inner';
    const win = document.createElement('div');
    win.className = 'virtual-window';
    inner.appendChild(win);
    hostEl.appendChild(inner);

    el._vInner = inner;
    el._vWin = win;

    let raf = 0;
    hostEl.addEventListener(
      'scroll',
      () => {
        if (raf) return;
        raf = requestAnimationFrame(() => {
          raf = 0;
          onScroll();
        });
      },
      { passive: true }
    );

    hostEl.addEventListener('click', onClick as EventListener);
    hostEl.addEventListener('contextmenu', onContextMenu as EventListener);
    hostEl.addEventListener('mousedown', onMouseDown as EventListener);
    hostEl.addEventListener('dragstart', onDragStart as EventListener);
    hostEl.addEventListener('dragenter', onDragEnter as EventListener);
    hostEl.addEventListener('dragover', onDragOver as EventListener);
    hostEl.addEventListener('dragleave', onDragLeave as EventListener);
    hostEl.addEventListener('drop', onDrop as EventListener);

    return { inner, win };
  }

  /**
   * Recycles and synchronizes DOM nodes inside the virtual window
   */
  public syncWindowNodes(
    win: HTMLElement,
    items: PaneItem[],
    pane: PaneState,
    start: number,
    need: number,
    rowRenderer: RowRenderer
  ): void {
    while (win.childElementCount < need) {
      const idx = start + win.childElementCount;
      win.appendChild(rowRenderer.createRow(pane, items[idx], idx));
    }
    while (win.childElementCount > need) {
      win.lastElementChild?.remove();
    }
    for (let j = 0; j < need; j++) {
      const idx = start + j;
      rowRenderer.syncRow(win.children[j] as HTMLDivElement, items[idx], pane, idx);
    }
  }

  /**
   * Smoothly scrolls the cursor index into the visible area if needed
   */
  public scrollCursorIntoView(hostEl: HTMLElement, cursor: number, total: number): void {
    if (cursor < 0 || cursor >= total) return;
    const top = cursor * this.rowStride;
    const bottom = top + this.rowStride;
    const st = hostEl.scrollTop;
    const sh = hostEl.clientHeight || 1;

    if (top < st) {
      hostEl.scrollTop = top;
    } else if (bottom > st + sh) {
      hostEl.scrollTop = bottom - sh;
    }
  }
}
