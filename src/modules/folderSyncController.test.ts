// src/modules/folderSyncController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { FolderSyncController } from './folderSyncController.ts';

function createMockElement(tag = 'div'): any {
  const children: any[] = [];
  const classList = {
    _classes: new Set<string>(),
    add(c: string) { this._classes.add(c); },
    remove(c: string) { this._classes.delete(c); },
    contains(c: string) { return this._classes.has(c); },
  };

  const el: any = {
    tagName: tag.toUpperCase(),
    className: '',
    classList,
    checked: false,
    disabled: false,
    textContent: '',
    innerHTML: '',
    children,
    appendChild(c: any) { children.push(c); return c; },
    replaceChildren(...cs: any[]) { children.length = 0; children.push(...cs); },
    querySelector(sel: string) {
      if (sel === '.sync-row-check') {
        const checkbox = createMockElement('input');
        checkbox.type = 'checkbox';
        checkbox.checked = el._checked || false;
        el._checkbox = checkbox;
        return checkbox;
      }
      return null;
    },
    addEventListener(evt: string, fn: any) {
      el._listeners = el._listeners || {};
      el._listeners[evt] = fn;
    },
  };
  return el;
}

test('FolderSyncController open checks folder validity and handles matching/empty paths', async () => {
  let statusMsg = '';
  const mockState: any = {
    left: { path: '/same/path' },
    right: { path: '/same/path' },
  };

  const controller = new FolderSyncController({
    state: mockState,
    api: () => ({}),
    setStatus: (msg) => { statusMsg = msg; },
    loadDir: async () => {},
    focusActiveList: () => {},
  });

  // 1. Same folder on both sides
  await controller.open();
  assert.equal(controller.isOpen, false);
  assert.equal(statusMsg, 'Folder Sync requires two different folders open in left and right panels.');

  // 2. Empty path on one side
  mockState.left.path = '';
  mockState.right.path = '/right/path';
  await controller.open();
  assert.equal(controller.isOpen, false);
});

test('FolderSyncController analyze detects onlyLeft, onlyRight, leftNewer, rightNewer, sizeDiff and same', async () => {
  const elements: Record<string, any> = {};
  const getEl = (id: string) => {
    if (!elements[id]) elements[id] = createMockElement('div');
    return elements[id];
  };

  const origDoc = globalThis.document;
  try {
    (globalThis as any).document = {
      getElementById: (id: string) => getEl(id),
      createElement: (tag: string) => createMockElement(tag),
      createDocumentFragment: () => ({
        children: [],
        appendChild(c: any) { (this as any).children.push(c); return c; },
      }),
    };

    const mockApi = {
      readDir: async (p: string) => {
        if (p === '/left') {
          return {
            ok: true,
            items: [
              { base: '..' },
              { base: 'only_in_left.txt', size: 100, modified: 1000 },
              { base: 'left_newer.txt', size: 200, modified: 10000 },
              { base: 'right_newer.txt', size: 200, modified: 1000 },
              { base: 'size_diff.txt', size: 500, modified: 5000 },
              { base: 'identical.txt', size: 300, modified: 5000 },
            ],
          };
        }
        if (p === '/right') {
          return {
            ok: true,
            items: [
              { base: '..' },
              { base: 'only_in_right.txt', size: 150, modified: 2000 },
              { base: 'left_newer.txt', size: 200, modified: 1000 },
              { base: 'right_newer.txt', size: 200, modified: 10000 },
              { base: 'size_diff.txt', size: 600, modified: 5000 },
              { base: 'identical.txt', size: 300, modified: 5000 },
            ],
          };
        }
        return { ok: false };
      },
    };

    const mockState: any = {
      left: { path: '/left' },
      right: { path: '/right' },
    };

    let focusedList = false;
    const controller = new FolderSyncController({
      state: mockState,
      api: () => mockApi,
      setStatus: () => {},
      loadDir: async () => {},
      focusActiveList: () => { focusedList = true; },
    });

    // Open & analyze
    await controller.open();
    assert.equal(controller.isOpen, true);
    assert.equal(getEl('sync-left-path').textContent, '/left');
    assert.equal(getEl('sync-right-path').textContent, '/right');

    // Analysis results
    assert.equal(controller.analysis.length, 6);
    const byName = new Map(controller.analysis.map((r) => [r.name, r]));

    assert.equal(byName.get('only_in_left.txt')?.status, 'onlyLeft');
    assert.equal(byName.get('only_in_left.txt')?.action, 'copyRight');
    assert.equal(byName.get('only_in_left.txt')?.selected, true);

    assert.equal(byName.get('only_in_right.txt')?.status, 'onlyRight');
    assert.equal(byName.get('only_in_right.txt')?.action, 'copyLeft');
    assert.equal(byName.get('only_in_right.txt')?.selected, true);

    assert.equal(byName.get('left_newer.txt')?.status, 'leftNewer');
    assert.equal(byName.get('left_newer.txt')?.action, 'updateRight');
    assert.equal(byName.get('left_newer.txt')?.selected, true);

    assert.equal(byName.get('right_newer.txt')?.status, 'rightNewer');
    assert.equal(byName.get('right_newer.txt')?.action, 'updateLeft');
    assert.equal(byName.get('right_newer.txt')?.selected, true);

    assert.equal(byName.get('size_diff.txt')?.status, 'sizeDiff');
    assert.equal(byName.get('size_diff.txt')?.action, 'updateRight');
    assert.equal(byName.get('size_diff.txt')?.selected, true);

    assert.equal(byName.get('identical.txt')?.status, 'same');
    assert.equal(byName.get('identical.txt')?.action, 'skip');
    assert.equal(byName.get('identical.txt')?.selected, false);

    // Summary element check
    const summaryText = getEl('sync-summary').textContent;
    assert.ok(summaryText.includes('5 of 6 item(s) selected'));

    // Checkbox toggling in renderResults
    const rows = getEl('sync-results-list').children[0].children;
    assert.ok(rows.length > 0);
    const firstRowCheck = rows[0]._checkbox;
    if (firstRowCheck && firstRowCheck._listeners?.['change']) {
      firstRowCheck._listeners['change']({ target: { checked: false } });
      assert.equal(controller.analysis[0].selected, false);
    }

    // Hide
    controller.hide();
    assert.equal(controller.isOpen, false);
    assert.equal(focusedList, true);

    // Analyze with read failure
    mockState.left.path = '/invalid';
    await controller.analyze();
    assert.ok(getEl('sync-results-list').innerHTML.includes('Failed to read directories'));
  } finally {
    globalThis.document = origDoc;
  }
});

test('FolderSyncController runSync copies selected files in appropriate direction and handles errors', async () => {
  const elements: Record<string, any> = {};
  const getEl = (id: string) => {
    if (!elements[id]) elements[id] = createMockElement('div');
    return elements[id];
  };

  const origDoc = globalThis.document;
  try {
    (globalThis as any).document = {
      getElementById: (id: string) => getEl(id),
      createElement: (tag: string) => createMockElement(tag),
      createDocumentFragment: () => ({
        children: [],
        appendChild(c: any) { (this as any).children.push(c); return c; },
      }),
    };

    const copiedOps: Array<{ src: string; dst: string }> = [];
    const loadedSides: string[] = [];

    const mockApi = {
      pathJoin: async (dir: string, base: string) => `${dir}/${base}`,
      copy: async (src: string, dst: string) => {
        if (src.includes('error')) throw new Error('Copy failed');
        copiedOps.push({ src, dst });
      },
      readDir: async () => ({ ok: true, items: [] }),
    };

    const mockState: any = {
      left: { path: '/left' },
      right: { path: '/right' },
    };

    const controller = new FolderSyncController({
      state: mockState,
      api: () => mockApi,
      setStatus: () => {},
      loadDir: async (side) => { loadedSides.push(side); },
      focusActiveList: () => {},
    });

    controller.analysis = [
      { name: 'copy_to_right.txt', status: 'onlyLeft', action: 'copyRight', selected: true },
      { name: 'copy_to_left.txt', status: 'onlyRight', action: 'copyLeft', selected: true },
      { name: 'error_file.txt', status: 'leftNewer', action: 'updateRight', selected: true },
      { name: 'skipped.txt', status: 'same', action: 'skip', selected: false },
    ];

    // Setup DOM listeners
    controller.setup();

    // Trigger runSync
    await controller.runSync();

    assert.equal(copiedOps.length, 2);
    // left to right
    assert.deepEqual(copiedOps[0], { src: '/left/copy_to_right.txt', dst: '/right' });
    // right to left
    assert.deepEqual(copiedOps[1], { src: '/right/copy_to_left.txt', dst: '/left' });

    assert.ok(loadedSides.includes('left'));
    assert.ok(loadedSides.includes('right'));
    assert.ok(getEl('sync-status-bar').textContent.includes('Synced 2 file(s)'));

    // Event listener: Close button click
    controller.isOpen = true;
    getEl('sync-close-btn')._listeners['click']();
    assert.equal(controller.isOpen, false);

    // Event listener: Backdrop click
    controller.isOpen = true;
    const overlay = getEl('folder-sync-overlay');
    overlay._listeners['click']({ target: overlay });
    assert.equal(controller.isOpen, false);

    // Render empty folders message
    controller.analysis = [];
    controller.renderResults();
    assert.ok(getEl('sync-results-list').innerHTML.includes('Both folders are empty.'));
  } finally {
    globalThis.document = origDoc;
  }
});
