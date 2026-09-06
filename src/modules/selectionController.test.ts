// src/modules/selectionController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { maskToRegex, SelectionController } from './selectionController.ts';
import type { AppState, Item } from './stateModels.ts';

function createMockElement(tag = 'div', id = ''): any {
  const classes = new Set<string>();
  const children: any[] = [];
  const attrs: Record<string, string> = {};

  return {
    id,
    tagName: tag.toUpperCase(),
    textContent: '',
    value: '',
    children,
    onclick: null as any,
    onkeydown: null as any,
    classList: {
      add(c: string) { classes.add(c); },
      remove(c: string) { classes.delete(c); },
      contains(c: string) { return classes.has(c); },
    },
    setAttribute(k: string, v: string) { attrs[k] = v; },
    getAttribute(k: string) { return attrs[k] ?? null; },
    appendChild(c: any) { children.push(c); },
    append(...cs: any[]) { children.push(...cs); },
    remove() {},
    focus() {},
    select() {},
    addEventListener(_ev: string, _fn: any) {},
    querySelector(selector: string) {
      function findDescendant(node: any, sel: string): any {
        if (sel.startsWith('#') && node.id === sel.slice(1)) return node;
        for (const child of node.children || []) {
          const found = findDescendant(child, sel);
          if (found) return found;
        }
        return null;
      }
      return findDescendant(this, selector);
    },
    querySelectorAll(selector: string) {
      if (selector === '.pane-sel-info') {
        return children.filter((c) => c.className === 'pane-sel-info');
      }
      return [];
    },
  };
}

function setupDom() {
  const origDoc = globalThis.document;
  const elements: Record<string, any> = {};

  const doc = {
    body: { appendChild: () => {} },
    getElementById: (id: string) => elements[id] || null,
    createElement: (tag: string) => {
      let _id = '';
      const el = createMockElement(tag);
      Object.defineProperty(el, 'id', {
        get: () => _id,
        set: (val: string) => {
          _id = val;
          elements[val] = el;
        },
      });
      return el;
    },
    querySelector: (sel: string) => {
      if (sel.startsWith('#pane-left')) return elements['pane-left-body'] || null;
      if (sel.startsWith('#pane-right')) return elements['pane-right-body'] || null;
      if (sel.startsWith('#')) return elements[sel.slice(1)] || null;
      return null;
    },
  };

  (globalThis as any).document = doc;

  return {
    elements,
    cleanup: () => {
      globalThis.document = origDoc;
    },
  };
}

function createMockPane(items: Item[]): any {
  return {
    items,
    listSerial: 0,
    activeTab: {
      selectedBases: new Set<string>(),
    },
  };
}

test('maskToRegex converts wildcard patterns to regex', () => {
  const r1 = maskToRegex('*.ts');
  assert.ok(r1.test('index.ts'));
  assert.ok(r1.test('INDEX.TS')); // case-insensitive
  assert.equal(r1.test('index.js'), false);

  const r2 = maskToRegex('test-?.txt');
  assert.ok(r2.test('test-1.txt'));
  assert.equal(r2.test('test-12.txt'), false);

  const r3 = maskToRegex('name[1].txt');
  assert.ok(r3.test('name[1].txt'));
  assert.equal(r3.test('name1.txt'), false);
});

test('SelectionController applyMask, invert, selectAll, and clearAll', () => {
  const items: Item[] = [
    { base: '..', isDir: true } as any,
    { base: 'file1.txt', isDir: false, size: 100 } as any,
    { base: 'file2.png', isDir: false, size: 200 } as any,
    { base: 'file3.txt', isDir: false, size: 300 } as any,
    { base: 'subfolder', isDir: true, size: 0 } as any,
  ];

  const leftPane = createMockPane(items);
  const state: AppState = {
    active: 'left',
    left: leftPane,
    right: createMockPane([]),
  } as any;

  let repaintedSide = '';
  let statusMessage = '';
  let focused = false;

  const controller = new SelectionController({
    state,
    renderPane: (s) => { repaintedSide = s; },
    setStatus: (m) => { statusMessage = m; },
    focusActiveList: () => { focused = true; },
    getFilteredSelection: () => ({ item: null }),
  });

  // Apply mask: select *.txt
  controller.applyMask('left', '*.txt', true);
  assert.equal(repaintedSide, 'left');
  assert.equal(leftPane.listSerial, 1);
  assert.equal(leftPane.activeTab.selectedBases.has('file1.txt'), true);
  assert.equal(leftPane.activeTab.selectedBases.has('file3.txt'), true);
  assert.equal(leftPane.activeTab.selectedBases.has('file2.png'), false);
  assert.equal(leftPane.activeTab.selectedBases.has('..'), false);
  assert.ok(statusMessage.includes('Selected by *.txt: 2 matched, 2 total'));

  // Deselect file1.txt
  controller.applyMask('left', 'file1.*', false);
  assert.equal(leftPane.activeTab.selectedBases.has('file1.txt'), false);
  assert.equal(leftPane.activeTab.selectedBases.has('file3.txt'), true);
  assert.ok(statusMessage.includes('Deselected by file1.*'));

  // Invert selection
  controller.invert('left');
  // Selectable items are file1.txt, file2.png, file3.txt, subfolder
  // Previously selected: file3.txt
  // Inverted: file1.txt, file2.png, subfolder
  assert.equal(leftPane.activeTab.selectedBases.has('file3.txt'), false);
  assert.equal(leftPane.activeTab.selectedBases.has('file1.txt'), true);
  assert.equal(leftPane.activeTab.selectedBases.has('file2.png'), true);
  assert.equal(leftPane.activeTab.selectedBases.has('subfolder'), true);
  assert.equal(leftPane.activeTab.selectedBases.has('..'), false);

  // Select all
  controller.selectAll('left');
  assert.equal(leftPane.activeTab.selectedBases.size, 4);
  assert.equal(leftPane.activeTab.selectedBases.has('..'), false);

  // Clear all
  controller.clearAll('left');
  assert.equal(leftPane.activeTab.selectedBases.size, 0);
  assert.equal(statusMessage, 'Selection cleared.');
});

test('SelectionController selectByExtension selects matching files', () => {
  const items: Item[] = [
    { base: '..', isDir: true } as any,
    { base: 'app.min.js', isDir: false, size: 500 } as any,
    { base: 'bundle.min.js', isDir: false, size: 600 } as any,
    { base: 'style.css', isDir: false, size: 100 } as any,
    { base: 'noext', isDir: false, size: 50 } as any,
    { base: 'folder.js', isDir: true, size: 0 } as any,
  ];

  const leftPane = createMockPane(items);
  const state: AppState = {
    active: 'left',
    left: leftPane,
    right: createMockPane([]),
  } as any;

  let currentItem: Item | null = null;
  let status = '';

  const controller = new SelectionController({
    state,
    renderPane: () => {},
    setStatus: (m) => { status = m; },
    focusActiveList: () => {},
    getFilteredSelection: () => ({ item: currentItem }),
  });

  // No item or '..'
  currentItem = null;
  controller.selectByExtension('left');
  assert.equal(leftPane.activeTab.selectedBases.size, 0);

  currentItem = items[0]; // '..'
  controller.selectByExtension('left');
  assert.equal(leftPane.activeTab.selectedBases.size, 0);

  // Item with no extension
  currentItem = items[4]; // 'noext'
  controller.selectByExtension('left');
  assert.equal(status, 'Cursor item has no extension.');

  // Item with extension (bundle.min.js -> js or min.js)
  currentItem = items[1]; // app.min.js
  controller.selectByExtension('left');
  // Should select app.min.js and bundle.min.js, but NOT folder.js (because folder.js isDir)
  assert.equal(leftPane.activeTab.selectedBases.has('app.min.js'), true);
  assert.equal(leftPane.activeTab.selectedBases.has('bundle.min.js'), true);
  assert.equal(leftPane.activeTab.selectedBases.has('folder.js'), false);
  assert.ok(status.includes('Selected *.'));
});

test('SelectionController summary calculates accurate counts and sizes', () => {
  const items: Item[] = [
    { base: '..', isDir: true, size: 0 } as any,
    { base: 'doc1.txt', isDir: false, size: 1000 } as any,
    { base: 'doc2.txt', isDir: false, size: 2000 } as any,
    { base: 'photos', isDir: true, size: 0 } as any,
  ];

  const leftPane = createMockPane(items);
  leftPane.activeTab.selectedBases.add('doc1.txt');
  leftPane.activeTab.selectedBases.add('photos');

  const state: AppState = {
    active: 'left',
    left: leftPane,
    right: createMockPane([]),
  } as any;

  const controller = new SelectionController({
    state,
    renderPane: () => {},
    setStatus: () => {},
    focusActiveList: () => {},
    getFilteredSelection: () => ({ item: null }),
  });

  const sum = controller.summary('left');
  assert.equal(sum.totalFiles, 2);
  assert.equal(sum.totalDirs, 1);
  assert.equal(sum.totalBytes, 3000);
  assert.equal(sum.selCount, 2);
  assert.equal(sum.selBytes, 1000);
  assert.equal(sum.selDirs, 1);
});

test('SelectionController updateIndicator and updateAll remove old indicator elements', () => {
  const { elements, cleanup } = setupDom();

  const removed: any[] = [];
  const oldInfo1 = { className: 'pane-sel-info', remove: () => removed.push(1) };
  const oldInfo2 = { className: 'pane-sel-info', remove: () => removed.push(2) };

  const paneLeftBody = createMockElement('div');
  paneLeftBody.children.push(oldInfo1);

  const paneRightBody = createMockElement('div');
  paneRightBody.children.push(oldInfo2);

  elements['pane-left-body'] = paneLeftBody;
  elements['pane-right-body'] = paneRightBody;

  const state: AppState = {
    active: 'left',
    left: createMockPane([]),
    right: createMockPane([]),
  } as any;

  const controller = new SelectionController({
    state,
    renderPane: () => {},
    setStatus: () => {},
    focusActiveList: () => {},
    getFilteredSelection: () => ({ item: null }),
  });

  try {
    controller.updateAll();
    assert.equal(removed.length, 2);
  } finally {
    cleanup();
  }
});

test('SelectionController selectByMaskDialog prompts and applies mask', async () => {
  const { elements, cleanup } = setupDom();

  const items: Item[] = [
    { base: 'a.js', isDir: false } as any,
    { base: 'b.py', isDir: false } as any,
  ];
  const leftPane = createMockPane(items);
  const state: AppState = {
    active: 'left',
    left: leftPane,
    right: createMockPane([]),
  } as any;

  let focused = false;
  const controller = new SelectionController({
    state,
    renderPane: () => {},
    setStatus: () => {},
    focusActiveList: () => { focused = true; },
    getFilteredSelection: () => ({ item: null }),
  });

  try {
    const p = controller.selectByMaskDialog('left', true);
    await new Promise((r) => setTimeout(r, 0));

    const input = elements['prompt-input'];
    const okBtn = elements['prompt-ok'];
    assert.ok(input);
    assert.ok(okBtn);

    input.value = '*.js';
    okBtn.onclick();
    await p;

    assert.equal(focused, true);
    assert.equal(leftPane.activeTab.selectedBases.has('a.js'), true);
    assert.equal(leftPane.activeTab.selectedBases.has('b.py'), false);
  } finally {
    cleanup();
  }
});
