// src/modules/gitController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { GitController } from './gitController.ts';

test('GitController open, close and hide manage overlay visibility and focus', async () => {
  let focused = false;
  const overlay = {
    classList: {
      _classes: new Set(['hidden']),
      add(c: string) { this._classes.add(c); },
      remove(c: string) { this._classes.delete(c); },
      contains(c: string) { return this._classes.has(c); },
    },
    setAttribute() {},
    removeAttribute() {},
  };

  const elements: Record<string, any> = {
    'git-overlay': overlay,
    'git-branch-badge': { textContent: '' },
    'git-status-bar': { textContent: '' },
    'git-status-list': { innerHTML: '', replaceChildren() {} },
  };

  globalThis.document = {
    getElementById: (id: string) => elements[id] || null,
    querySelectorAll: () => [],
    addEventListener: () => {},
    removeEventListener: () => {},
  } as any;

  const controller = new GitController({
    api: () => ({
      gitIsRepo: async () => ({ ok: false }),
    }),
    state: { active: 'left', left: { path: '/test' } },
    setStatus: () => {},
    loadDir: async () => {},
    focusActiveList: () => { focused = true; },
  });

  await controller.open();
  assert.equal(overlay.classList.contains('hidden'), false);

  controller.close();
  assert.equal(overlay.classList.contains('hidden'), true);
  assert.equal(focused, true);

  focused = false;
  await controller.open();
  assert.equal(overlay.classList.contains('hidden'), false);

  controller.hide();
  assert.equal(overlay.classList.contains('hidden'), true);
  assert.equal(focused, true);
});

test('GitController openDiffForFile preserves filePath and auto-populates blame input', async () => {
  const overlay = {
    classList: {
      _classes: new Set(['hidden']),
      add(c: string) { this._classes.add(c); },
      remove(c: string) { this._classes.delete(c); },
      contains(c: string) { return this._classes.has(c); },
    },
    setAttribute() {},
    removeAttribute() {},
  };

  const fileInput = { value: '' };
  let blamedFile: any = null;

  const elements: Record<string, any> = {
    'git-overlay': overlay,
    'git-branch-badge': { textContent: '' },
    'git-status-bar': { textContent: '' },
    'git-status-list': { innerHTML: '', replaceChildren() {} },
    'git-blame-file': fileInput,
    'git-blame-ref': { value: '' },
    'git-blame-content': { innerHTML: '', replaceChildren() {} },
    'git-diff-ref1': { value: '' },
    'git-diff-ref2': { value: '' },
    'git-diff-content': { replaceChildren() {}, appendChild() {} },
  };

  globalThis.document = {
    getElementById: (id: string) => elements[id] || null,
    querySelectorAll: () => [],
    addEventListener: () => {},
    removeEventListener: () => {},
    createElement: () => ({ appendChild: () => {}, replaceChildren: () => {}, append: () => {}, addEventListener: () => {} }),
    createDocumentFragment: () => ({ appendChild: () => {} }),
  } as any;

  const controller = new GitController({
    api: () => ({
      gitIsRepo: async () => ({ ok: true, root: '/test-repo' }),
      gitDiff: async () => ({ ok: true, diff: '' }),
      gitBlame: async (_repo: string, file: string) => {
        blamedFile = file;
        return { ok: true, lines: [] };
      },
    }),
    state: { active: 'left', left: { path: '/test-repo' } },
    setStatus: () => {},
    loadDir: async () => {},
    focusActiveList: () => {},
  });

  await controller.openDiffForFile('src/index.js', '/test-repo');
  assert.equal(controller._filePath, 'src/index.js');
  assert.equal(fileInput.value, 'src/index.js');

  await controller._loadBlame();
  assert.equal(blamedFile, 'src/index.js');
});
