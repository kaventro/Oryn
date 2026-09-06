// src/modules/virtualSearchResultsView.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { VirtualSearchResultsView } from './virtualSearchResultsView.ts';

function createMockElement(tag = 'div'): any {
  const children: any[] = [];
  const dataset: Record<string, string> = {};
  const listeners: Record<string, Function[]> = {};
  const classes = new Set<string>();

  const el: any = {
    tagName: tag.toUpperCase(),
    className: '',
    textContent: '',
    dataset,
    children,
    scrollTop: 0,
    clientHeight: 300,
    style: { height: '', transform: '' },
    classList: {
      add(c: string) { classes.add(c); el.className = [...classes].join(' '); },
      remove(c: string) { classes.delete(c); el.className = [...classes].join(' '); },
      contains(c: string) { return classes.has(c); },
      toggle(c: string, force?: boolean) {
        const next = force !== undefined ? force : !classes.has(c);
        if (next) classes.add(c);
        else classes.delete(c);
        el.className = [...classes].join(' ');
        return next;
      },
    },
    append(...cs: any[]) { children.push(...cs); },
    appendChild(c: any) {
      if (c && c.isFragment) {
        children.push(...c.children);
      } else {
        children.push(c);
      }
      return c;
    },
    replaceChildren(...cs: any[]) {
      children.length = 0;
      for (const c of cs) {
        if (c && c.isFragment) {
          children.push(...c.children);
        } else {
          children.push(c);
        }
      }
    },
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    dispatchEvent(ev: string, payload: any) {
      (listeners[ev] || []).forEach((fn) => fn(payload));
    },
    contains(target: any) {
      return children.includes(target) || this === target;
    },
    closest(sel: string) {
      if (sel === '.search-hit') return this;
      return null;
    },
  };
  return el;
}

function setupDom() {
  const origDoc = globalThis.document;
  const origRaf = globalThis.requestAnimationFrame;

  (globalThis as any).document = {
    createElement: (tag: string) => createMockElement(tag),
    createDocumentFragment: () => {
      const fragment = createMockElement('fragment');
      fragment.isFragment = true;
      return fragment;
    },
  };

  (globalThis as any).requestAnimationFrame = (fn: Function) => {
    return setTimeout(fn, 0) as any;
  };

  return () => {
    globalThis.document = origDoc;
    globalThis.requestAnimationFrame = origRaf;
  };
}

test('VirtualSearchResultsView attach wires DOM and click/dblclick events', () => {
  const cleanup = setupDom();

  let selectedIdx = -1;
  let openedIdx = -1;

  const view = new VirtualSearchResultsView({
    getStore: () => ({ resultCount: 0, sessionId: 's1', peek: () => null, ensureRange: async () => false } as any),
    getSelectedIndex: () => 2,
    formatRow: () => {},
    onSelect: (idx) => { selectedIdx = idx; },
    onOpen: (idx) => { openedIdx = idx; },
  });

  const container = createMockElement('div');

  try {
    view.attach(container);
    assert.equal(view.container, container);
    assert.ok(view.inner);
    assert.ok(view.window);
    assert.equal(container.children.length, 1);
    assert.equal(container.children[0], view.inner);

    // Click event delegation
    const mockRow = createMockElement('div');
    mockRow.dataset.index = '5';
    container.children.push(mockRow);

    container.dispatchEvent('click', { target: mockRow });
    assert.equal(selectedIdx, 5);

    // Double click event delegation
    container.dispatchEvent('dblclick', { target: mockRow });
    assert.equal(openedIdx, 5);

    // Re-attach same container is a no-op
    view.attach(container);
    assert.equal(container.children.length, 2);
  } finally {
    cleanup();
  }
});

test('VirtualSearchResultsView clear resets container scroll, inner height and window', () => {
  const cleanup = setupDom();
  const container = createMockElement('div');

  const view = new VirtualSearchResultsView({
    getStore: () => ({} as any),
    getSelectedIndex: () => -1,
    formatRow: () => {},
    onSelect: () => {},
    onOpen: () => {},
  });

  try {
    view.attach(container);
    container.scrollTop = 150;
    view.inner!.style.height = '1000px';
    view.window!.appendChild(createMockElement('div'));

    view.clear();
    assert.equal(container.scrollTop, 0);
    assert.equal(view.inner!.style.height, '0px');
    assert.equal(view.window!.children.length, 0);
  } finally {
    cleanup();
  }
});

test('VirtualSearchResultsView reveal scrolls item into view', () => {
  const cleanup = setupDom();
  const container = createMockElement('div');
  container.scrollTop = 100;
  container.clientHeight = 200; // visible range: 100 to 300

  let scheduled = false;
  const view = new VirtualSearchResultsView({
    getStore: () => ({} as any),
    getSelectedIndex: () => -1,
    formatRow: () => {},
    onSelect: () => {},
    onOpen: () => {},
  });
  view.scheduleRender = () => { scheduled = true; };

  try {
    view.attach(container);

    // Reveal index 0 (top = 0 < 100) -> should scroll up to 0
    view.reveal(0);
    assert.equal(container.scrollTop, 0);
    assert.equal(scheduled, true);

    // Reveal index 20 (top = 640, bottom = 672 > 0 + 200) -> should scroll down to 672 - 200 = 472
    view.reveal(20);
    assert.equal(container.scrollTop, 472);
  } finally {
    cleanup();
  }
});

test('VirtualSearchResultsView render populates visible rows and triggers ensureRange', async () => {
  const cleanup = setupDom();
  const container = createMockElement('div');
  container.clientHeight = 96; // ~3 rows visible
  container.scrollTop = 0;

  const items = ['/path/one.txt', '/path/two.txt', null, '/path/four.txt'];
  let rangeRequested = false;
  let rangeCallCount = 0;

  const mockStore: any = {
    resultCount: 4,
    sessionId: 'session_123',
    peek: (i: number) => items[i] ?? null,
    ensureRange: async (from: number, to: number) => {
      rangeRequested = true;
      rangeCallCount++;
      return rangeCallCount === 1;
    },
  };

  const formattedRows: string[] = [];
  const view = new VirtualSearchResultsView({
    getStore: () => mockStore,
    getSelectedIndex: () => 1,
    formatRow: (row, fullPath) => {
      row.textContent = fullPath;
      formattedRows.push(fullPath);
    },
    onSelect: () => {},
    onOpen: () => {},
  });

  try {
    view.attach(container);
    await view.render();

    assert.equal(view.inner!.style.height, `${4 * view.rowHeight}px`);
    assert.equal(view.window!.children.length, 4);

    // Row 0: loaded, not selected
    const row0 = view.window!.children[0];
    assert.equal(row0.textContent, '/path/one.txt');
    assert.equal(row0.classList.contains('search-hit--sel'), false);

    // Row 1: loaded, selected
    const row1 = view.window!.children[1];
    assert.equal(row1.textContent, '/path/two.txt');
    assert.equal(row1.classList.contains('search-hit--sel'), true);

    // Row 2: not loaded (null) -> loading placeholder
    const row2 = view.window!.children[2];
    assert.equal(row2.textContent, 'Loading…');
    assert.equal(row2.classList.contains('search-hit--loading'), true);

    assert.equal(rangeRequested, true);
    await new Promise((r) => setTimeout(r, 20));
  } finally {
    cleanup();
  }
});
