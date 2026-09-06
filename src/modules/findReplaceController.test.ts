// src/modules/findReplaceController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { FindReplaceController } from './findReplaceController.ts';

function createMockElement(initial: Partial<HTMLInputElement> = {}): any {
  const listeners: Record<string, Function[]> = {};
  return {
    value: '',
    checked: false,
    textContent: '',
    ...initial,
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    click() {
      (listeners['click'] || []).forEach((fn) => fn());
    },
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

test('FindReplaceController.run warns if findText is empty', async () => {
  const elements = {
    'search-content': createMockElement({ value: '   ' }),
  };
  const cleanup = setupDom(elements);

  let statusMsg = '';
  const controller = new FindReplaceController({
    api: () => ({}),
    setStatus: (msg) => { statusMsg = msg; },
    getSearchRoot: () => '/home',
  });

  try {
    await controller.run();
    assert.equal(statusMsg, 'Enter text to find in "Contains text" field.');
  } finally {
    cleanup();
  }
});

test('FindReplaceController.run executes dry run and replacement', async () => {
  const statusEl = createMockElement();
  const elements = {
    'search-content': createMockElement({ value: 'foo' }),
    'search-replace-text': createMockElement({ value: 'bar' }),
    'replace-dry-run': createMockElement({ checked: true }),
    'replace-regex': createMockElement({ checked: true }),
    'search-content-case': createMockElement({ checked: false }),
    'search-filename': createMockElement({ value: '*.txt' }),
    'search-exclude': createMockElement({ value: 'node_modules' }),
    'replace-status': statusEl,
  };
  const cleanup = setupDom(elements);

  let passedParams: any = null;
  let progressFn: any = null;

  const mockApi: any = {
    findReplace: async (params: any, onProgress: any) => {
      passedParams = params;
      progressFn = onProgress;
      onProgress({ type: 'file', path: '/home/a.txt', occurrences: 2 });
      return {
        ok: true,
        count: 2,
        changed: [{ path: '/home/a.txt', occurrences: 2 }],
      };
    },
  };

  const controller = new FindReplaceController({
    api: () => mockApi,
    setStatus: () => {},
    getSearchRoot: () => '/home',
  });

  try {
    // 1. Dry run
    await controller.run();
    assert.equal(passedParams.findText, 'foo');
    assert.equal(passedParams.replaceText, 'bar');
    assert.equal(passedParams.dryRun, true);
    assert.equal(passedParams.useRegex, true);
    assert.ok(statusEl.textContent.includes('[DRY RUN] Would modify 1 file(s), 2 replacement(s).'));

    // 2. Error handling
    mockApi.findReplace = async () => ({ ok: false, error: 'Permission denied' });
    await controller.run();
    assert.equal(statusEl.textContent, 'Error: Permission denied');

    // 3. Live replace (dryRun = false)
    elements['replace-dry-run'].checked = false;
    mockApi.findReplace = async () => ({
      ok: true,
      count: 5,
      changed: [{ path: '/home/b.txt', occurrences: 5 }],
    });
    await controller.run();
    assert.ok(statusEl.textContent.includes('Modified 1 file(s), 5 replacement(s).'));
  } finally {
    cleanup();
  }
});

test('FindReplaceController.patchSearchButton attaches click handler and triggers run', async () => {
  const startBtn = createMockElement();
  const elements = {
    'search-start-btn': startBtn,
    'search-content': createMockElement({ value: 'hello' }),
    'search-replace-text': createMockElement({ value: 'world' }),
    'replace-dry-run': createMockElement({ checked: true }),
    'replace-status': createMockElement(),
  };
  const cleanup = setupDom(elements);

  let runCalled = false;
  const controller = new FindReplaceController({
    api: () => ({
      findReplace: async () => {
        runCalled = true;
        return { ok: true, count: 1, changed: [] };
      },
    }),
    setStatus: () => {},
    getSearchRoot: () => '/',
  });

  try {
    controller.patchSearchButton();
    startBtn.click();
    assert.equal(runCalled, true);
  } finally {
    cleanup();
  }
});
