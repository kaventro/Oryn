// src/modules/statusBarController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { StatusBarController } from './statusBarController.ts';
import type { AppState, PaneState, Item } from './stateModels.ts';

function createMockElement(): any {
  return {
    textContent: '',
    title: '',
  };
}

function setupDom(elements: Record<string, any>) {
  const origDoc = globalThis.document;
  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
  };
  return () => {
    globalThis.document = origDoc;
  };
}

function createMockState(opts: {
  path?: string;
  items?: Item[];
  selected?: string[];
  cursor?: number;
  copyInProgress?: boolean;
} = {}): AppState {
  const items = opts.items || [];
  const selectedBases = new Set<string>(opts.selected || []);
  const pane: PaneState = {
    path: opts.path ?? '/home/user',
    items,
    cursor: opts.cursor ?? 0,
    activeTab: {
      selectedBases,
    } as any,
  } as any;

  return {
    active: 'left',
    copyInProgress: !!opts.copyInProgress,
    left: pane,
    right: {
      path: '/right',
      items: [],
      cursor: 0,
      activeTab: { selectedBases: new Set() },
    } as any,
  } as any;
}

test('StatusBarController refresh renders idle and ready status', () => {
  const statusEl = createMockElement();
  const itemCountEl = createMockElement();
  const fullPathEl = createMockElement();

  const elements = {
    status: statusEl,
    'item-count-status': itemCountEl,
    'full-path-status': fullPathEl,
  };
  const cleanup = setupDom(elements);

  const items: Item[] = [
    { base: '..', isDir: true } as any,
    { base: 'file1.txt', isDir: false, size: 1024 } as any,
    { base: 'folder1', isDir: true, size: 0 } as any,
  ];

  const state = createMockState({ items });
  const controller = new StatusBarController({
    state,
    api: () => ({}),
    filteredItems: (p) => p.items,
    fmtSize: (n) => `${n} bytes`,
    shortPath: (p) => `~/${p.split('/').pop()}`,
  });

  try {
    controller.refresh();
    assert.ok(statusEl.textContent.includes('SEL 0')); // cursor at 0 is '..' which is ignored
    assert.ok(statusEl.textContent.includes('VIEW 1F/1D'));
    assert.ok(statusEl.textContent.includes('PATH ~/user'));
    assert.ok(statusEl.textContent.includes('TASK idle'));
    // 'ready' message is considered noisy and omitted from suffix
    assert.equal(statusEl.textContent.endsWith('TASK idle'), true);

    assert.equal(itemCountEl.textContent, '2 items — 1024 bytes');
    assert.equal(fullPathEl.textContent, '/home/user');
  } finally {
    cleanup();
  }
});

test('StatusBarController refresh handles cursor single selection and custom message', () => {
  const statusEl = createMockElement();
  const itemCountEl = createMockElement();
  const cleanup = setupDom({
    status: statusEl,
    'item-count-status': itemCountEl,
  });

  const items: Item[] = [
    { base: 'file1.txt', isDir: false, size: 2048 } as any,
  ];

  const state = createMockState({ items, cursor: 0 });
  const controller = new StatusBarController({
    state,
    api: () => ({}),
    filteredItems: (p) => p.items,
    fmtSize: (n) => `${n} bytes`,
  });

  try {
    controller.setMessage('Scanning files...');
    assert.ok(statusEl.textContent.includes('SEL 1 2048 bytes'));
    assert.ok(statusEl.textContent.includes('· Scanning files...'));
    assert.equal(itemCountEl.textContent, '1 item — 2048 bytes');
  } finally {
    cleanup();
  }
});

test('StatusBarController refresh handles multi-selection and copy task status', () => {
  const statusEl = createMockElement();
  const itemCountEl = createMockElement();
  const cleanup = setupDom({
    status: statusEl,
    'item-count-status': itemCountEl,
  });

  const items: Item[] = [
    { base: 'a.txt', isDir: false, size: 100 } as any,
    { base: 'b.txt', isDir: false, size: 200 } as any,
    { base: 'dir1', isDir: true, size: 0 } as any,
  ];

  const state = createMockState({
    items,
    selected: ['a.txt', 'b.txt'],
    copyInProgress: true,
  });

  const controller = new StatusBarController({
    state,
    api: () => ({}),
    filteredItems: (p) => p.items,
    fmtSize: (n) => `${n} B`,
  });

  try {
    controller.refresh();
    assert.ok(statusEl.textContent.includes('SEL 2 300 B'));
    assert.ok(statusEl.textContent.includes('TASK copy'));
    assert.equal(itemCountEl.textContent, '2 of 3 selected (300 B)');

    // Single item selected in selectedBases
    state.left.activeTab.selectedBases.clear();
    state.left.activeTab.selectedBases.add('a.txt');
    controller.refresh();
    assert.equal(itemCountEl.textContent, '1 of 3 selected (100 B)');
  } finally {
    cleanup();
  }
});

test('StatusBarController requestPathSpace fetches and caches space info', async () => {
  const statusEl = createMockElement();
  const cleanup = setupDom({ status: statusEl });

  let getPathSpaceCalls = 0;
  const mockApi = {
    getPathSpace: async (_path: string) => {
      getPathSpaceCalls++;
      return { ok: true, free: 500000, total: 1000000 };
    },
  };

  const state = createMockState({ path: '/storage/drive' });
  const controller = new StatusBarController({
    state,
    api: () => mockApi,
    filteredItems: () => [],
    fmtSize: (n) => `${n} B`,
  });

  try {
    // Initial call triggers path space request and showed FREE n/a initially
    controller.refresh();
    assert.equal(getPathSpaceCalls, 1);

    // Refresh while pending
    controller.refresh();
    assert.ok(statusEl.textContent.includes('FREE …'));

    // Wait for promise resolution
    await new Promise((r) => setTimeout(r, 10));
    assert.ok(statusEl.textContent.includes('FREE 500000 B'));

    // Subsequent refresh within 15 seconds should not re-query
    controller.refresh();
    assert.equal(getPathSpaceCalls, 1);

    // Mismatched path or invalid response handling
    controller.pathSpace.path = '/other';
    controller.pathSpace.free = null;
    controller.refresh();
    assert.ok(statusEl.textContent.includes('FREE n/a'));
  } finally {
    cleanup();
  }
});
