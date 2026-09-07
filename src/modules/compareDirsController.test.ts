// src/modules/compareDirsController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { CompareDirsController, type CompareDirsDeps } from './compareDirsController.ts';
import { AppState } from './stateModels.ts';

interface MockEl {
  id: string;
  className: string;
  dataset: Record<string, string>;
  classList: {
    contains: (c: string) => boolean;
    add: (c: string) => void;
    remove: (c: string) => void;
    toggle: (c: string, force?: boolean) => boolean;
  };
  textContent: string;
  innerHTML: string;
  title: string;
  children: MockEl[];
  replaceChildren: (...nodes: MockEl[]) => void;
  appendChild: (child: MockEl) => void;
  append: (...children: MockEl[]) => void;
  addEventListener: (event: string, handler: (e: any) => void) => void;
  click: (eventPayload?: any) => void;
  closest: (selector: string) => MockEl | null;
  querySelector: (selector: string) => MockEl | null;
  querySelectorAll: (selector: string) => MockEl[];
}

function createMockEl(id = '', className = '', dataset: Record<string, string> = {}): MockEl {
  let _className = className;
  const classes = new Set<string>(className ? className.split(/\s+/).filter(Boolean) : []);
  const listeners: Record<string, ((e: any) => void)[]> = {};
  const children: MockEl[] = [];

  const el: MockEl = {
    id,
    get className() { return _className; },
    set className(val: string) {
      _className = val;
      classes.clear();
      val.split(/\s+/).filter(Boolean).forEach((c) => classes.add(c));
    },
    dataset: { ...dataset },
    classList: {
      contains: (c: string) => classes.has(c),
      add: (c: string) => { classes.add(c); _className = Array.from(classes).join(' '); },
      remove: (c: string) => { classes.delete(c); _className = Array.from(classes).join(' '); },
      toggle: (c: string, force?: boolean) => {
        const has = classes.has(c);
        const shouldHave = force !== undefined ? force : !has;
        if (shouldHave) classes.add(c);
        else classes.delete(c);
        _className = Array.from(classes).join(' ');
        return shouldHave;
      },
    },
    textContent: '',
    innerHTML: '',
    title: '',
    children,
    replaceChildren: (...nodes: MockEl[]) => {
      children.length = 0;
      if (nodes && nodes.length) {
        for (const n of nodes) {
          el.appendChild(n);
        }
      }
    },
    appendChild: (child: MockEl) => {
      if (child.id === 'fragment') {
        children.push(...child.children);
        child.children.length = 0;
      } else {
        children.push(child);
      }
    },
    append: (...newChildren: MockEl[]) => {
      for (const c of newChildren) {
        el.appendChild(c);
      }
    },
    addEventListener: (event: string, handler: (e: any) => void) => {
      if (!listeners[event]) listeners[event] = [];
      listeners[event].push(handler);
    },
    click: (eventPayload: any = {}) => {
      const e = {
        target: el,
        stopPropagation: () => {},
        ...eventPayload,
      };
      (listeners['click'] || []).forEach((fn) => fn(e));
    },
    closest: (selector: string): MockEl | null => {
      if (selector.startsWith('.')) {
        const cls = selector.slice(1);
        if (classes.has(cls)) return el;
      }
      return null;
    },
    querySelector: (selector: string): MockEl | null => {
      if (selector.startsWith('.')) {
        const cls = selector.slice(1);
        for (const c of children) {
          if (c.classList.contains(cls)) return c;
          const found = c.querySelector(selector);
          if (found) return found;
        }
      }
      return null;
    },
    querySelectorAll: (selector: string): MockEl[] => {
      const results: MockEl[] = [];
      if (selector.startsWith('.')) {
        const cls = selector.slice(1);
        for (const c of children) {
          if (c.classList.contains(cls)) results.push(c);
          results.push(...c.querySelectorAll(selector));
        }
      }
      return results;
    },
  };

  return el;
}

function setupMockDocument(elements: Record<string, MockEl>, allTabButtons: MockEl[] = []) {
  const origDoc = globalThis.document;
  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
    querySelectorAll: (selector: string) => {
      if (selector === '.compare-tab') return allTabButtons;
      return [];
    },
    createElement: (tag: string) => createMockEl('', tag),
    createDocumentFragment: () => createMockEl('fragment'),
  };
  return () => {
    globalThis.document = origDoc;
  };
}

test('CompareDirsController open() and hide() update overlay and paths', async () => {
  const overlay = createMockEl('compare-overlay', 'hidden');
  const leftPath = createMockEl('compare-left-path');
  const rightPath = createMockEl('compare-right-path');
  const statsEl = createMockEl('compare-stats');
  const resultsEl = createMockEl('compare-results');

  const elements = {
    'compare-overlay': overlay,
    'compare-left-path': leftPath,
    'compare-right-path': rightPath,
    'compare-stats': statsEl,
    'compare-results': resultsEl,
  };

  const cleanup = setupMockDocument(elements);
  let focused = false;
  const state = new AppState();
  state.left.path = '/home/user/dirA';
  state.right.path = '/home/user/dirB';

  const mockApi = {
    compareDirs: async () => ({ ok: true, different: [], onlyLeft: [], onlyRight: [], same: [] }),
  };

  const controller = new CompareDirsController({
    api: () => mockApi,
    state,
    setStatus: () => {},
    openFileDiff: () => {},
    focusActiveList: () => { focused = true; },
  });

  try {
    controller.open();
    assert.equal(leftPath.textContent, '/home/user/dirA');
    assert.equal(rightPath.textContent, '/home/user/dirB');
    assert.equal(overlay.classList.contains('hidden'), false);

    controller.hide();
    assert.equal(overlay.classList.contains('hidden'), true);
    assert.equal(focused, true);

    // Fallback when paths are empty
    state.left.path = '';
    state.right.path = '';
    controller.open();
    assert.equal(leftPath.textContent, '—');
    assert.equal(rightPath.textContent, '—');
  } finally {
    cleanup();
  }
});

test('CompareDirsController setup() binds overlay, close, run, swap and tab clicks', async () => {
  const overlay = createMockEl('compare-overlay');
  const closeBtn = createMockEl('compare-close');
  const runBtn = createMockEl('compare-run-btn');
  const swapBtn = createMockEl('compare-swap-btn');
  const leftPath = createMockEl('compare-left-path');
  const rightPath = createMockEl('compare-right-path');
  const statsEl = createMockEl('compare-stats');
  const resultsEl = createMockEl('compare-results');

  const tabDiff = createMockEl('tab-diff', 'compare-tab', { ctab: 'different' });
  const tabOnlyLeft = createMockEl('tab-left', 'compare-tab', { ctab: 'onlyLeft' });
  const tabOnlyRight = createMockEl('tab-right', 'compare-tab', { ctab: 'onlyRight' });
  const tabSame = createMockEl('tab-same', 'compare-tab', { ctab: 'same' });
  const tabButtons = [tabDiff, tabOnlyLeft, tabOnlyRight, tabSame];

  const elements: Record<string, MockEl> = {
    'compare-overlay': overlay,
    'compare-close': closeBtn,
    'compare-run-btn': runBtn,
    'compare-swap-btn': swapBtn,
    'compare-left-path': leftPath,
    'compare-right-path': rightPath,
    'compare-stats': statsEl,
    'compare-results': resultsEl,
  };

  const cleanup = setupMockDocument(elements, tabButtons);
  const state = new AppState();
  state.left.path = '/path/A';
  state.right.path = '/path/B';

  let compareCallCount = 0;
  let lastLeft = '';
  let lastRight = '';
  const mockApi = {
    compareDirs: async (l: string, r: string) => {
      compareCallCount++;
      lastLeft = l;
      lastRight = r;
      return {
        ok: true,
        different: [{ rel: 'diff.txt', left: { size: 10, mtime: 1000, full: '/path/A/diff.txt' }, right: { size: 20, mtime: 2000, full: '/path/B/diff.txt' } }],
        onlyLeft: [{ rel: 'left.txt', isDir: false }],
        onlyRight: [{ rel: 'sub/right.txt', isDir: true }],
        same: [{ rel: 'same.txt', isDir: false }],
      };
    },
  };

  let focused = false;
  const controller = new CompareDirsController({
    api: () => mockApi,
    state,
    setStatus: () => {},
    openFileDiff: () => {},
    focusActiveList: () => { focused = true; },
  });

  try {
    controller.setup();

    // 1. Overlay click on overlay itself hides it
    overlay.click({ target: overlay });
    assert.equal(overlay.classList.contains('hidden'), true);
    assert.equal(focused, true);

    // Click inside child doesn't hide
    overlay.classList.remove('hidden');
    const innerChild = createMockEl();
    overlay.click({ target: innerChild });
    assert.equal(overlay.classList.contains('hidden'), false);

    // 2. Close button click hides
    closeBtn.click();
    assert.equal(overlay.classList.contains('hidden'), true);

    // 3. Run button triggers compare
    runBtn.click();
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(compareCallCount, 1);
    assert.equal(lastLeft, '/path/A');
    assert.equal(lastRight, '/path/B');

    // 4. Swap button swaps paths and runs compare
    swapBtn.click();
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(state.left.path, '/path/B');
    assert.equal(state.right.path, '/path/A');
    assert.equal(leftPath.textContent, '/path/B');
    assert.equal(rightPath.textContent, '/path/A');
    assert.equal(compareCallCount, 2);
    assert.equal(lastLeft, '/path/B');
    assert.equal(lastRight, '/path/A');

    // 5. Clicking tab renders tab
    tabOnlyLeft.click();
    assert.equal(tabOnlyLeft.classList.contains('compare-tab--active'), true);
    assert.equal(resultsEl.children.length, 1);

    tabOnlyRight.click();
    assert.equal(tabOnlyRight.classList.contains('compare-tab--active'), true);
    assert.equal(resultsEl.children.length, 1);

    tabSame.click();
    assert.equal(tabSame.classList.contains('compare-tab--active'), true);
    assert.equal(resultsEl.children.length, 1);
  } finally {
    cleanup();
  }
});

test('CompareDirsController validation: reports error when either path is empty', async () => {
  const overlay = createMockEl('compare-overlay');
  const statsEl = createMockEl('compare-stats');
  const resultsEl = createMockEl('compare-results');
  const elements = {
    'compare-overlay': overlay,
    'compare-stats': statsEl,
    'compare-results': resultsEl,
  };
  const cleanup = setupMockDocument(elements);

  let statusMsg = '';
  const state = new AppState();
  state.left.path = '/some/path';
  state.right.path = '';

  const controller = new CompareDirsController({
    api: () => ({}),
    state,
    setStatus: (msg) => { statusMsg = msg; },
    openFileDiff: () => {},
    focusActiveList: () => {},
  });

  try {
    controller.open();
    assert.equal(statusMsg, 'Both panels must have a path to compare.');
  } finally {
    cleanup();
  }
});

test('CompareDirsController _run() handles onUpdate events (partial, progress, done)', async () => {
  const overlay = createMockEl('compare-overlay');
  const statsEl = createMockEl('compare-stats');
  const resultsEl = createMockEl('compare-results');
  const tabDiff = createMockEl('tab-diff', 'compare-tab', { ctab: 'different' });
  const tabOnlyLeft = createMockEl('tab-left', 'compare-tab', { ctab: 'onlyLeft' });
  const tabButtons = [tabDiff, tabOnlyLeft];

  const elements = {
    'compare-overlay': overlay,
    'compare-stats': statsEl,
    'compare-results': resultsEl,
  };
  const cleanup = setupMockDocument(elements, tabButtons);

  const state = new AppState();
  state.left.path = '/a';
  state.right.path = '/b';

  let capturedOnUpdate: ((update: any) => void) | null = null;
  const mockApi = {
    compareDirs: async (_l: string, _r: string, onUpdate: (u: any) => void) => {
      capturedOnUpdate = onUpdate;
      // 1. Send partial update
      onUpdate({
        type: 'partial',
        different: [{ rel: 'file1.txt', left: { size: 100, full: '/a/file1.txt' }, right: { size: 200, full: '/b/file1.txt' } }],
        onlyLeft: [],
        onlyRight: [],
        same: [],
      });
      // 2. Send second partial update
      onUpdate({
        type: 'partial',
        different: [
          { rel: 'file1.txt', left: { size: 100, full: '/a/file1.txt' }, right: { size: 200, full: '/b/file1.txt' } },
          { rel: 'dir/file2.txt', left: { size: 50, full: '/a/dir/file2.txt' }, right: { size: 50, full: '/b/dir/file2.txt' } },
        ],
        onlyLeft: [{ rel: 'left-only.txt', isDir: false }],
        onlyRight: [],
        same: [],
      });
      // 3. Send progress update
      // Attach mock .cmp-loading child inside statsEl
      const loadingEl = createMockEl('', 'cmp-loading');
      statsEl.children.push(loadingEl);
      onUpdate({
        type: 'progress',
        done: 5,
        total: 10,
      });
      assert.equal(loadingEl.textContent, '⏳ Hashing files… 5/10');

      // 4. Send done update
      onUpdate({
        type: 'done',
        different: [],
        onlyLeft: [{ rel: 'left-only.txt', isDir: false }],
        onlyRight: [],
        same: [],
      });

      return {
        ok: true,
        different: [],
        onlyLeft: [{ rel: 'left-only.txt', isDir: false }],
        onlyRight: [],
        same: [],
      };
    },
  };

  const controller = new CompareDirsController({
    api: () => mockApi,
    state,
    setStatus: () => {},
    openFileDiff: () => {},
    focusActiveList: () => {},
  });

  try {
    controller.open();
    await new Promise((r) => setTimeout(r, 20));
    assert.ok(capturedOnUpdate !== null);
    // After done, preferred tab should be onlyLeft
    assert.equal(tabOnlyLeft.classList.contains('compare-tab--active'), true);
  } finally {
    cleanup();
  }
});

test('CompareDirsController _run() handles API failures and exceptions', async () => {
  const overlay = createMockEl('compare-overlay');
  const statsEl = createMockEl('compare-stats');
  const resultsEl = createMockEl('compare-results');
  const elements = {
    'compare-overlay': overlay,
    'compare-stats': statsEl,
    'compare-results': resultsEl,
  };
  const cleanup = setupMockDocument(elements);

  const state = new AppState();
  state.left.path = '/a';
  state.right.path = '/b';

  // 1. API returns { ok: false, error: 'Access denied' }
  let mockApi: any = {
    compareDirs: async () => ({ ok: false, error: 'Access denied' }),
  };

  let controller = new CompareDirsController({
    api: () => mockApi,
    state,
    setStatus: () => {},
    openFileDiff: () => {},
    focusActiveList: () => {},
  });

  try {
    controller.open();
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(statsEl.textContent, 'Access denied');

    // 2. API throws exception
    mockApi = {
      compareDirs: async () => { throw new Error('Network timeout'); },
    };
    controller = new CompareDirsController({
      api: () => mockApi,
      state,
      setStatus: () => {},
      openFileDiff: () => {},
      focusActiveList: () => {},
    });

    controller.open();
    await new Promise((r) => setTimeout(r, 10));
    assert.ok(statsEl.textContent.includes('Network timeout'));
  } finally {
    cleanup();
  }
});

test('CompareDirsController row rendering, empty state, and openFileDiff invocation', async () => {
  const overlay = createMockEl('compare-overlay');
  const statsEl = createMockEl('compare-stats');
  const resultsEl = createMockEl('compare-results');
  const elements = {
    'compare-overlay': overlay,
    'compare-stats': statsEl,
    'compare-results': resultsEl,
  };
  const cleanup = setupMockDocument(elements);

  let diffLeft = '';
  let diffRight = '';
  const state = new AppState();
  state.left.path = '/pathA';
  state.right.path = '/pathB';

  const mockApi = {
    compareDirs: async () => ({
      ok: true,
      different: [
        {
          rel: 'src/config.json',
          left: { size: 1024, mtime: 1600000000000, full: '/pathA/src/config.json', isDir: false },
          right: { size: 2048, mtime: 1700000000000, full: '/pathB/src/config.json', isDir: false },
        },
      ],
      onlyLeft: [],
      onlyRight: [],
      same: [],
    }),
  };

  const controller = new CompareDirsController({
    api: () => mockApi,
    state,
    setStatus: () => {},
    openFileDiff: (l, r) => { diffLeft = l; diffRight = r; },
    focusActiveList: () => {},
  });

  try {
    controller.open();
    await new Promise((r) => setTimeout(r, 10));

    // Results element should have the rendered row
    assert.equal(resultsEl.children.length, 1);
    const row = resultsEl.children[0];
    assert.equal(row.classList.contains('compare-row--diff'), true);
    assert.equal(row.title, 'Click to view diff: src/config.json');

    // Click row to invoke openFileDiff
    row.click();
    assert.equal(diffLeft, '/pathA/src/config.json');
    assert.equal(diffRight, '/pathB/src/config.json');

    // Switching to empty tab renders empty message
    const tabSame = createMockEl('', 'compare-tab', { ctab: 'same' });
    (controller as any)._renderTab('same');
    assert.ok(resultsEl.innerHTML.includes('No entries in this category.'));

    // When running and 'different' tab is empty, shows comparing message
    (controller as any)._running = true;
    (controller as any)._data.different = [];
    (controller as any)._renderTab('different');
    assert.ok(resultsEl.innerHTML.includes('⏳ Comparing…'));
    (controller as any)._running = false;

    // Delegated click on stats badge
    const statBadge = createMockEl('', 'cmp-stat', { ctab: 'onlyLeft' });
    statsEl.children.push(statBadge);
    statsEl.click({ target: statBadge });
    assert.equal((controller as any)._currentTab, 'onlyLeft');
  } finally {
    cleanup();
  }
});

test('CompareDirsController _pickPreferredTab logic and row details', () => {
  const controller = new CompareDirsController({
    api: () => ({}),
    state: new AppState(),
    setStatus: () => {},
    openFileDiff: () => {},
    focusActiveList: () => {},
  });

  // Null data
  assert.equal((controller as any)._pickPreferredTab(null), 'different');
  // All empty
  assert.equal((controller as any)._pickPreferredTab({ different: [], onlyLeft: [], onlyRight: [], same: [] }), 'different');
  // Only same has items
  assert.equal((controller as any)._pickPreferredTab({ different: [], onlyLeft: [], onlyRight: [], same: [{}] }), 'same');
  // Only right has items
  assert.equal((controller as any)._pickPreferredTab({ different: [], onlyLeft: [], onlyRight: [{}] }), 'onlyRight');
  // Only left has items
  assert.equal((controller as any)._pickPreferredTab({ different: [], onlyLeft: [{}] }), 'onlyLeft');
  // Different has items
  assert.equal((controller as any)._pickPreferredTab({ different: [{}] }), 'different');
});

