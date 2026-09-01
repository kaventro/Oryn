// src/modules/duplicateFinderController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { DuplicateFinderController } from './duplicateFinderController.ts';

test('DuplicateFinderController scans, groups and auto-selects duplicates', async () => {
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
    value: '/test/dir',
    addEventListener() {},
  });

  (globalThis as any).document = {
    getElementById(id: string) {
      if (id === 'duplicates-overlay') return mockOverlay;
      return mockEl();
    },
    createTextNode(t: string) {
      return { nodeType: 3, textContent: t };
    },
    createElement() {
      return {
        className: '',
        textContent: '',
        innerHTML: '',
        style: {},
        setAttribute() {},
        appendChild() {},
        append() {},
        addEventListener() {},
      };
    },
  };

  const mockDuplicatesResult = {
    ok: true,
    totalScanned: 10,
    duplicateGroups: [
      {
        hash: 'hash123',
        size: 5000,
        files: [
          { path: '/test/dir/a.iso', name: 'a.iso', size: 5000, mtime: '2026-01-01T00:00:00Z' },
          { path: '/test/dir/sub/a_copy.iso', name: 'a_copy.iso', size: 5000, mtime: '2026-02-01T00:00:00Z' },
        ],
        totalWasted: 5000,
      },
    ],
    totalWastedBytes: 5000,
    duplicateFilesCount: 2,
  };

  const api = () => ({
    scanDuplicates: async () => mockDuplicatesResult,
    deletePath: async () => {},
  });

  const state = {
    active: 'left',
    left: { path: '/test/dir' },
    right: { path: '/tmp' },
  };

  const dfc = new DuplicateFinderController({
    api,
    state,
    setStatus: () => {},
  });

  assert.equal(dfc.isOpen(), false);
  dfc.open('/test/dir');
  assert.equal(dfc.isOpen(), true);

  await new Promise((r) => setTimeout(r, 10));

  // Test auto-selection strategies
  dfc.selectStrategy('newest'); // keeps newest (2026-02-01), selects older (/test/dir/a.iso)
  dfc.selectStrategy('oldest'); // keeps oldest (2026-01-01), selects newer (/test/dir/sub/a_copy.iso)
  dfc.selectStrategy('none');

  dfc.hide();
  assert.equal(dfc.isOpen(), false);
});

test('DuplicateFinderController safely renders malicious filenames without innerHTML injection', async () => {
  const appendedElements: any[] = [];
  const createdElements: any[] = [];

  const mockGroupsContainer = {
    replaceChildren: () => {},
    appendChild: (el: any) => { appendedElements.push(el); },
  };

  (globalThis as any).document = {
    getElementById(id: string) {
      if (id === 'duplicates-groups') return mockGroupsContainer;
      return {
        classList: { add() {}, remove() {}, contains() { return false; } },
        setAttribute() {},
        replaceChildren() {},
        appendChild() {},
        addEventListener() {},
      };
    },
    createTextNode(t: string) {
      return { nodeType: 3, textContent: t };
    },
    createElement(tag: string) {
      const el = {
        tagName: tag,
        className: '',
        textContent: '',
        innerHTML: '',
        style: {},
        setAttribute() {},
        appendChild: (child: any) => {},
        append: (...children: any[]) => {},
        addEventListener: () => {},
      };
      createdElements.push(el);
      return el;
    },
  };

  const maliciousName = '<img src=x onerror=alert(1)>evil.txt';
  const mockResult = {
    ok: true,
    totalScanned: 2,
    duplicateGroups: [
      {
        hash: 'hash-xss',
        size: 1024,
        files: [
          { path: `/test/${maliciousName}`, name: maliciousName, size: 1024, mtime: '2026-01-01T00:00:00Z' },
          { path: `/test/copy.txt`, name: 'copy.txt', size: 1024, mtime: '2026-01-01T00:00:00Z' },
        ],
        totalWasted: 1024,
      },
    ],
    totalWastedBytes: 1024,
    duplicateFilesCount: 2,
  };

  const dfc = new DuplicateFinderController({
    api: () => ({ scanDuplicates: async () => mockResult }),
    state: { active: 'left', left: { path: '/test' } },
    setStatus: () => {},
  });

  dfc.open('/test');
  await new Promise((r) => setTimeout(r, 10));

  // Verify none of the created elements have innerHTML containing raw unescaped script/img payload
  const hasRawInnerHTML = createdElements.some((el) => el.innerHTML && el.innerHTML.includes('<img src=x'));
  assert.equal(hasRawInnerHTML, false, 'No element should assign raw filename to innerHTML');
});
