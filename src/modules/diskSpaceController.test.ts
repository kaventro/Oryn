// src/modules/diskSpaceController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { DiskSpaceController } from './diskSpaceController.ts';

test('DiskSpaceController initializes and manages open/hide and scan', async () => {
  const mockOverlay = {
    classList: {
      _classes: new Set(['hidden']),
      contains(c: string) { return this._classes.has(c); },
      add(c: string) { this._classes.add(c); },
      remove(c: string) { this._classes.delete(c); },
    },
    setAttribute() {},
    addEventListener() {},
  };

  const mockEl = () => ({
    classList: { add() {}, remove() {} },
    replaceChildren() {},
    appendChild(c: any) { (this.children as any[]).push(c); },
    children: [] as any[],
    textContent: '',
    innerHTML: '',
    addEventListener() {},
  });

  (globalThis as any).document = {
    getElementById(id: string) {
      if (id === 'disk-space-overlay') return mockOverlay;
      return mockEl();
    },
    createElement() {
      return {
        className: '',
        textContent: '',
        style: {},
        setAttribute() {},
        appendChild() {},
        append() {},
        addEventListener() {},
      };
    },
  };

  let analyzedPath = '';
  const api = () => ({
    analyzeDir: async (p: string) => {
      analyzedPath = p;
      return {
        ok: true,
        path: p,
        totalSize: 10240,
        totalFiles: 10,
        totalDirs: 2,
        items: [
          { name: 'big.mp4', path: `${p}/big.mp4`, size: 8192, files: 1, dirs: 0, isDir: false },
          { name: 'docs', path: `${p}/docs`, size: 2048, files: 9, dirs: 2, isDir: true },
        ],
      };
    },
  });

  const state = {
    active: 'left',
    left: { path: '/home/user/project' },
    right: { path: '/tmp' },
  };

  const dsc = new DiskSpaceController({
    api,
    state,
    setStatus: () => {},
  });

  assert.equal(dsc.isOpen(), false);
  dsc.open('/home/user/project');
  assert.equal(dsc.isOpen(), true);

  await new Promise((r) => setTimeout(r, 10));
  assert.equal(analyzedPath, '/home/user/project');

  dsc.hide();
  assert.equal(dsc.isOpen(), false);
});
