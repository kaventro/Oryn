// src/modules/fileDiffController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { FileDiffController } from './fileDiffController.ts';

function createMockElement(): any {
  const classes = new Set<string>();
  const listeners: Record<string, Function[]> = {};
  const children: any[] = [];
  return {
    textContent: '',
    children,
    replaceChildren() { children.length = 0; },
    appendChild(c: any) { children.push(c); },
    classList: {
      add(c: string) { classes.add(c); },
      remove(c: string) { classes.delete(c); },
      contains(c: string) { return classes.has(c); },
    },
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    click(target?: any) {
      (listeners['click'] || []).forEach((fn) => fn({ target: target || this, stopPropagation: () => {} }));
    },
  };
}

function setupDom(elements: Record<string, any>) {
  const origDoc = globalThis.document;
  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
    createDocumentFragment: () => createMockElement(),
    createElement: (_tag: string) => createMockElement(),
  };
  return () => {
    globalThis.document = origDoc;
  };
}

test('FileDiffController openWith and hide manage overlay and runDiff', async () => {
  const overlay = createMockElement();
  const leftEl = createMockElement();
  const rightEl = createMockElement();
  const statusEl = createMockElement();
  const contentEl = createMockElement();

  const elements = {
    'diff-overlay': overlay,
    'diff-left-path': leftEl,
    'diff-right-path': rightEl,
    'diff-status': statusEl,
    'diff-content': contentEl,
  };
  const cleanup = setupDom(elements);

  let focused = false;
  let diffParams: any = null;

  const mockApi = {
    compareFiles: async (l: string, r: string) => {
      diffParams = { l, r };
      return { ok: true, same: true, reason: 'identical content' };
    },
  };

  const controller = new FileDiffController({
    api: () => mockApi,
    state: { left: {} as any, right: {} as any, active: 'left' } as any,
    setStatus: () => {},
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    focusActiveList: () => { focused = true; },
  });

  try {
    // 1. openWith with paths
    controller.openWith('/left/a.txt', '/right/a.txt');
    assert.equal(leftEl.textContent, '/left/a.txt');
    assert.equal(rightEl.textContent, '/right/a.txt');

    // Wait a tick for runDiff async
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(diffParams.l, '/left/a.txt');
    assert.equal(diffParams.r, '/right/a.txt');
    assert.ok(statusEl.textContent.includes('✓ Files are identical'));

    // 2. hide
    controller.hide();
    assert.ok(overlay.classList.contains('hidden'));
    assert.equal(focused, true);

    // 3. openWith with empty paths
    controller.openWith('', '');
    assert.equal(leftEl.textContent, '—');
    assert.equal(rightEl.textContent, '—');
  } finally {
    cleanup();
  }
});

test('FileDiffController openForSelected uses selected file or warns', () => {
  const elements = {
    'diff-overlay': createMockElement(),
    'diff-left-path': createMockElement(),
    'diff-right-path': createMockElement(),
    'diff-status': createMockElement(),
    'diff-content': createMockElement(),
  };
  const cleanup = setupDom(elements);

  let statusMsg = '';
  const mockState: any = {
    active: 'left',
    left: {
      path: '/home/user/docs',
      cursor: 0,
      items: [{ base: 'notes.md' }],
    },
    right: {
      path: '/backup/docs',
    },
  };

  const controller = new FileDiffController({
    api: () => ({ compareFiles: async () => ({ ok: true, same: true }) }),
    state: mockState,
    setStatus: (msg) => { statusMsg = msg; },
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    focusActiveList: () => {},
  });

  try {
    // Valid selection
    controller.openForSelected();
    assert.equal(elements['diff-left-path'].textContent, '/home/user/docs/notes.md');
    assert.equal(elements['diff-right-path'].textContent, '/backup/docs/notes.md');

    // Empty selection
    mockState.left.items = [];
    controller.openForSelected();
    assert.equal(statusMsg, 'No file selected for diff.');
  } finally {
    cleanup();
  }
});

test('FileDiffController runDiff handles errors and diff output', async () => {
  const statusEl = createMockElement();
  const contentEl = createMockElement();
  const elements = {
    'diff-status': statusEl,
    'diff-content': contentEl,
  };
  const cleanup = setupDom(elements);

  let compareResult: any = { ok: false, error: 'File not found' };
  const controller = new FileDiffController({
    api: () => ({ compareFiles: async () => compareResult }),
    state: {} as any,
    setStatus: () => {},
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    focusActiveList: () => {},
  });

  try {
    // Error
    await controller.runDiff('/a.txt', '/b.txt');
    assert.equal(statusEl.textContent, 'File not found');

    // Differences
    compareResult = {
      ok: true,
      same: false,
      engine: 'native',
      diff: '--- a\n+++ b\n-old\n+new',
    };
    await controller.runDiff('/a.txt', '/b.txt');
    assert.ok(statusEl.textContent.includes('Files differ: [native]'));
    assert.ok(contentEl.children.length > 0);
  } finally {
    cleanup();
  }
});

test('FileDiffController setup attaches event handlers for close, backdrop and run', () => {
  const overlay = createMockElement();
  const closeBtn = createMockElement();
  const runBtn = createMockElement();
  const leftEl = createMockElement();
  const rightEl = createMockElement();
  const statusEl = createMockElement();
  const contentEl = createMockElement();

  leftEl.textContent = '/home/a.txt';
  rightEl.textContent = '/home/b.txt';

  const elements = {
    'diff-overlay': overlay,
    'diff-close': closeBtn,
    'diff-run-btn': runBtn,
    'diff-left-path': leftEl,
    'diff-right-path': rightEl,
    'diff-status': statusEl,
    'diff-content': contentEl,
  };
  const cleanup = setupDom(elements);

  let ranDiff = false;
  const controller = new FileDiffController({
    api: () => ({
      compareFiles: async () => {
        ranDiff = true;
        return { ok: true, same: true };
      },
    }),
    state: {} as any,
    setStatus: () => {},
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    focusActiveList: () => {},
  });

  try {
    controller.setup();

    // Click backdrop
    overlay.click(overlay);
    assert.ok(overlay.classList.contains('hidden'));

    // Click close
    overlay.classList.remove('hidden');
    closeBtn.click();
    assert.ok(overlay.classList.contains('hidden'));

    // Click run button
    runBtn.click();
    assert.equal(ranDiff, true);
  } finally {
    cleanup();
  }
});
