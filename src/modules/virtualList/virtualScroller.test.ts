// src/modules/virtualList/virtualScroller.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { VirtualScroller } from './virtualScroller.ts';

function createMockElement(tag = 'div'): any {
  const children: any[] = [];
  const listeners: Record<string, Function[]> = {};
  const el: any = {
    tagName: tag.toUpperCase(),
    className: '',
    children,
    parentElement: null,
    get childElementCount() {
      return children.length;
    },
    get lastElementChild() {
      return children.length > 0 ? children[children.length - 1] : null;
    },
    appendChild(child: any) {
      children.push(child);
      child.parentElement = el;
      return child;
    },
    removeChild(child: any) {
      const idx = children.indexOf(child);
      if (idx !== -1) {
        children.splice(idx, 1);
        child.parentElement = null;
      }
      return child;
    },
    remove() {
      if (el.parentElement) {
        el.parentElement.removeChild(el);
      }
    },
    addEventListener(event: string, fn: Function) {
      if (!listeners[event]) listeners[event] = [];
      listeners[event].push(fn);
    },
    dispatchEvent(event: string, evObj: any = {}) {
      (listeners[event] || []).forEach((fn) => fn(evObj));
    },
    scrollTop: 0,
    clientHeight: 0,
  };
  return el;
}

test('VirtualScroller computes visible window slices accurately', () => {
  const scroller = new VirtualScroller({ rowStride: 30, chunkSize: 8, bufferSize: 10 });
  const totalItems = 1000;
  const clientHeight = 600; // ~20 visible items
  const scrollTop = 3000; // ~100 items down

  const calc = scroller.computeWindow(totalItems, clientHeight, scrollTop);
  assert.equal(calc.totalH, 30000);
  assert.ok(calc.start <= 100);
  assert.ok(calc.end >= 120);
  assert.equal(calc.need, calc.end - calc.start);
  assert.equal(calc.offsetY, calc.start * 30);
});

test('VirtualScroller handles empty lists and small boundaries', () => {
  const scroller = new VirtualScroller();
  const calc = scroller.computeWindow(0, 500, 0);
  assert.equal(calc.totalH, 0);
  assert.equal(calc.start, 0);
  assert.equal(calc.end, 0);
  assert.equal(calc.need, 0);
  assert.equal(calc.offsetY, 0);

  // Test clientHeight fallback to 1 when clientHeight is 0
  const zeroH = scroller.computeWindow(10, 0, 0);
  assert.equal(zeroH.totalH, 300);
});

test('VirtualScroller.ensureChrome creates DOM chrome and wires event listeners', () => {
  const origDoc = globalThis.document;
  const origRaf = globalThis.requestAnimationFrame;

  let rafCallback: FrameRequestCallback | null = null;
  (globalThis as any).document = {
    createElement: (tag: string) => createMockElement(tag),
  };
  (globalThis as any).requestAnimationFrame = (cb: FrameRequestCallback) => {
    rafCallback = cb;
    return 42;
  };

  try {
    const scroller = new VirtualScroller();
    const hostEl = createMockElement('div');

    let scrollCalled = 0;
    let clickCalled = 0;
    let contextMenuCalled = 0;
    let mouseDownCalled = 0;
    let dragStartCalled = 0;
    let dragEnterCalled = 0;
    let dragOverCalled = 0;
    let dragLeaveCalled = 0;
    let dropCalled = 0;

    const chrome1 = scroller.ensureChrome(
      hostEl,
      () => { scrollCalled++; },
      () => { clickCalled++; },
      () => { contextMenuCalled++; },
      () => { mouseDownCalled++; },
      () => { dragStartCalled++; },
      () => { dragEnterCalled++; },
      () => { dragOverCalled++; },
      () => { dragLeaveCalled++; },
      () => { dropCalled++; }
    );

    assert.ok(chrome1.inner);
    assert.ok(chrome1.win);
    assert.equal(chrome1.inner.className, 'virtual-inner');
    assert.equal(chrome1.win.className, 'virtual-window');
    assert.equal(chrome1.inner.parentElement, hostEl);
    assert.equal(chrome1.win.parentElement, chrome1.inner);

    // Re-calling ensureChrome on the same host returns cached chrome
    const chrome2 = scroller.ensureChrome(
      hostEl,
      () => {},
      () => {},
      () => {},
      () => {},
      () => {},
      () => {},
      () => {},
      () => {},
      () => {}
    );
    assert.equal(chrome2.inner, chrome1.inner);
    assert.equal(chrome2.win, chrome1.win);

    // Test scroll throttling with requestAnimationFrame
    hostEl.dispatchEvent('scroll');
    assert.equal(scrollCalled, 0, 'scroll should be throttled by rAF');
    hostEl.dispatchEvent('scroll'); // ignored because raf is already pending
    assert.ok(rafCallback);
    rafCallback(0);
    assert.equal(scrollCalled, 1);

    // Test other event dispatchers
    hostEl.dispatchEvent('click', { type: 'click' });
    assert.equal(clickCalled, 1);

    hostEl.dispatchEvent('contextmenu', { type: 'contextmenu' });
    assert.equal(contextMenuCalled, 1);

    hostEl.dispatchEvent('mousedown', { type: 'mousedown' });
    assert.equal(mouseDownCalled, 1);

    hostEl.dispatchEvent('dragstart', { type: 'dragstart' });
    assert.equal(dragStartCalled, 1);

    hostEl.dispatchEvent('dragenter', { type: 'dragenter' });
    assert.equal(dragEnterCalled, 1);

    hostEl.dispatchEvent('dragover', { type: 'dragover' });
    assert.equal(dragOverCalled, 1);

    hostEl.dispatchEvent('dragleave', { type: 'dragleave' });
    assert.equal(dragLeaveCalled, 1);

    hostEl.dispatchEvent('drop', { type: 'drop' });
    assert.equal(dropCalled, 1);
  } finally {
    globalThis.document = origDoc;
    globalThis.requestAnimationFrame = origRaf;
  }
});

test('VirtualScroller.syncWindowNodes adds, removes, and synchronizes rows', () => {
  const scroller = new VirtualScroller();
  const win = createMockElement('div');
  const items: any[] = [
    { name: 'file1.txt', isDir: false },
    { name: 'file2.txt', isDir: false },
    { name: 'file3.txt', isDir: false },
    { name: 'folder1', isDir: true },
    { name: 'folder2', isDir: true },
  ];
  const paneState: any = { path: '/home', activeIndex: 0 };

  const createdRows: any[] = [];
  const syncedRows: any[] = [];

  const mockRowRenderer: any = {
    createRow(pane: any, item: any, idx: number) {
      const row = createMockElement('div');
      row.className = 'file-row';
      createdRows.push({ pane, item, idx });
      return row;
    },
    syncRow(row: any, item: any, pane: any, idx: number) {
      syncedRows.push({ row, item, pane, idx });
    },
  };

  // 1. Initial sync: expands from 0 to 3 rows
  scroller.syncWindowNodes(win, items, paneState, 0, 3, mockRowRenderer);
  assert.equal(win.childElementCount, 3);
  assert.equal(createdRows.length, 3);
  assert.equal(syncedRows.length, 3);
  assert.equal(syncedRows[0].idx, 0);
  assert.equal(syncedRows[1].idx, 1);
  assert.equal(syncedRows[2].idx, 2);

  // 2. Expand from 3 to 5 rows
  createdRows.length = 0;
  syncedRows.length = 0;
  scroller.syncWindowNodes(win, items, paneState, 0, 5, mockRowRenderer);
  assert.equal(win.childElementCount, 5);
  assert.equal(createdRows.length, 2);
  assert.equal(syncedRows.length, 5);

  // 3. Shrink from 5 to 2 rows
  createdRows.length = 0;
  syncedRows.length = 0;
  scroller.syncWindowNodes(win, items, paneState, 2, 2, mockRowRenderer);
  assert.equal(win.childElementCount, 2);
  assert.equal(createdRows.length, 0);
  assert.equal(syncedRows.length, 2);
  assert.equal(syncedRows[0].idx, 2);
  assert.equal(syncedRows[1].idx, 3);
});

test('VirtualScroller.scrollCursorIntoView scrolls up and down appropriately', () => {
  const scroller = new VirtualScroller({ rowStride: 30 });
  const hostEl = createMockElement('div');
  hostEl.clientHeight = 300; // 10 visible rows
  hostEl.scrollTop = 300; // rows 10 to 20 visible (y: 300 to 600)

  // Out of bounds: < 0 or >= total does nothing
  scroller.scrollCursorIntoView(hostEl, -1, 100);
  assert.equal(hostEl.scrollTop, 300);

  scroller.scrollCursorIntoView(hostEl, 100, 100);
  assert.equal(hostEl.scrollTop, 300);

  // Already inside visible viewport (e.g. index 12 -> y: 360 to 390)
  scroller.scrollCursorIntoView(hostEl, 12, 100);
  assert.equal(hostEl.scrollTop, 300);

  // Cursor is above visible area (e.g. index 5 -> top is 150 < st 300)
  scroller.scrollCursorIntoView(hostEl, 5, 100);
  assert.equal(hostEl.scrollTop, 150);

  // Cursor is below visible area (e.g. index 25 -> bottom is 780 > st 150 + sh 300 = 450)
  // New scrollTop = bottom - sh = 780 - 300 = 480
  scroller.scrollCursorIntoView(hostEl, 25, 100);
  assert.equal(hostEl.scrollTop, 480);

  // Fallback clientHeight = 0 -> uses sh = 1
  hostEl.clientHeight = 0;
  hostEl.scrollTop = 0;
  scroller.scrollCursorIntoView(hostEl, 2, 100); // bottom = 90 > 0 + 1 => 90 - 1 = 89
  assert.equal(hostEl.scrollTop, 89);
});
