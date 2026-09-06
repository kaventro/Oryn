// src/modules/rows/rowRenderer.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { RowRenderer, rowSizeText, type PaneItem, type PaneState } from './rowRenderer.ts';

function createMockElement(tag = 'div'): any {
  const children: any[] = [];
  const dataset: Record<string, string> = {};
  const attrs: Record<string, string> = {};

  const el: any = {
    tagName: tag.toUpperCase(),
    className: '',
    draggable: false,
    textContent: '',
    innerHTML: '',
    dataset,
    children,
    append(...cs: any[]) { children.push(...cs); },
    appendChild(c: any) { children.push(c); return c; },
    replaceChildren(...cs: any[]) { children.length = 0; children.push(...cs); },
    remove() {
      // noop or detach
    },
    removeAttribute(k: string) { delete attrs[k]; delete el[k]; },
    setAttribute(k: string, v: string) { attrs[k] = v; },
    getAttribute(k: string) { return attrs[k] ?? null; },
    querySelector(selector: string) {
      if (selector === '.row-tag-dots') {
        return children.find((c) => c.className === 'row-tag-dots') || null;
      }
      if (selector === '.git-status-tag') {
        return children.find((c) => c.className && c.className.startsWith('git-status-tag')) || null;
      }
      return null;
    },
  };
  return el;
}

function setupDom() {
  const origDoc = globalThis.document;
  (globalThis as any).document = {
    createElement: (tag: string) => createMockElement(tag),
  };
  return () => {
    globalThis.document = origDoc;
  };
}

function createMockIconRegistry() {
  return {
    resolveIconKey: (item: PaneItem) => (item.isDir ? 'folder' : 'file'),
    getSvg: (key: string) => `<svg class="${key}"></svg>`,
  } as any;
}

test('rowSizeText handles parent dir, dir with and without size, and normal files', () => {
  const fmt = (sz: number | null | undefined, isDir?: boolean) => `${sz}b${isDir ? '_d' : ''}`;

  assert.equal(rowSizeText({ base: '..' }, fmt), '');
  assert.equal(rowSizeText({ base: 'folder', isDir: true }, fmt), '<DIR>');
  assert.equal(rowSizeText({ base: 'folder2', isDir: true, size: 4096 }, fmt), '4096b_d');
  assert.equal(rowSizeText({ base: 'file.txt', isDir: false, size: 120 }, fmt), '120b');
});

test('RowRenderer createRow builds row structure and tags/git badges', () => {
  const cleanup = setupDom();
  const iconRegistry = createMockIconRegistry();
  const renderer = new RowRenderer({
    iconRegistry,
    fmtSize: (sz) => `${sz} bytes`,
    rowDateText: (it) => `2026-09-01 ${it.base}`,
  });

  const pane: PaneState = {
    cursor: 0,
    activeTab: { selectedBases: new Set(['selected.txt']) },
  };

  try {
    // Standard file selected by cursor
    const item1: PaneItem = {
      base: 'selected.txt',
      display: 'Selected File.txt',
      isDir: false,
      size: 512,
      tags: ['red', 'work'],
      gitStatus: 'modified',
    };
    const row1 = renderer.createRow(pane, item1, 0);

    assert.ok(row1.className.includes('row'));
    assert.ok(row1.className.includes('user-selected'));
    assert.ok(row1.className.includes('selected'));
    assert.equal(row1.dataset.vidx, '0');
    assert.equal(row1.draggable, true);

    // Check name column
    const nameCol = row1.children[0];
    assert.equal(nameCol.className, 'row-name');
    const iconSpan = nameCol.children[0];
    assert.equal(iconSpan.innerHTML, '<svg class="file"></svg>');
    const nameSpan = nameCol.children[1];
    assert.equal(nameSpan.textContent, 'Selected File.txt');

    // Tags wrap
    const tagsWrap = nameCol.children[2];
    assert.equal(tagsWrap.className, 'row-tag-dots');
    assert.equal(tagsWrap.children.length, 2);
    assert.equal(tagsWrap.children[0].className, 'tag-dot tag-dot--red');
    assert.equal(tagsWrap.children[1].className, 'tag-dot tag-dot--work');

    // Git badge
    const gitBadge = nameCol.children[3];
    assert.equal(gitBadge.className, 'git-status-tag git-status-tag--modified');
    assert.equal(gitBadge.textContent, 'modified');

    // Date & Size columns
    const dateCol = row1.children[1];
    assert.equal(dateCol.textContent, '2026-09-01 selected.txt');
    const sizeCol = row1.children[2] as HTMLElement;
    assert.equal(sizeCol.textContent, '512 bytes');
    assert.equal(sizeCol.title, '512 bytes');

    // Dir row without tags or git
    const dirItem: PaneItem = { base: 'photos', isDir: true };
    const dirRow = renderer.createRow(pane, dirItem, 1);
    assert.ok(dirRow.className.includes('dir'));
    assert.equal(dirRow.className.includes('user-selected'), false);
    assert.equal(dirRow.className.includes('selected'), false);
    assert.equal(dirRow.children[2].textContent, '<DIR>');
  } finally {
    cleanup();
  }
});

test('RowRenderer syncRow efficiently updates row properties', () => {
  const cleanup = setupDom();
  const iconRegistry = createMockIconRegistry();
  const renderer = new RowRenderer({
    iconRegistry,
    fmtSize: (sz) => `${sz} bytes`,
    rowDateText: (it) => `date:${it.base}`,
  });

  const pane: PaneState = {
    cursor: 0,
    activeTab: { selectedBases: new Set() },
  };

  try {
    const initialItem: PaneItem = {
      base: 'test.txt',
      isDir: false,
      size: 100,
    };
    const row = renderer.createRow(pane, initialItem, 0);

    // Sync with updated item: changed display, tags added, git status added, size updated
    const updatedItem: PaneItem = {
      base: 'test.txt',
      display: 'Test Renamed.txt',
      isDir: false,
      size: 200,
      tags: ['green'],
      gitStatus: 'untracked',
    };
    pane.cursor = 1; // cursor moved
    pane.activeTab.selectedBases.add('test.txt'); // now user selected

    renderer.syncRow(row, updatedItem, pane, 0);

    assert.ok(row.className.includes('user-selected'));
    assert.equal(row.className.split(' ').includes('selected'), false); // idx 0 is not cursor 1

    const nameCol = row.children[0];
    const nameText = nameCol.children[1];
    assert.equal(nameText.textContent, 'Test Renamed.txt');

    const tagsWrap = nameCol.querySelector('.row-tag-dots');
    assert.ok(tagsWrap);
    assert.equal(tagsWrap.children.length, 1);
    assert.equal(tagsWrap.children[0].className, 'tag-dot tag-dot--green');

    const gitBadge = nameCol.querySelector('.git-status-tag');
    assert.ok(gitBadge);
    assert.equal(gitBadge.textContent, 'untracked');

    const sizeCol = row.children[2];
    assert.equal(sizeCol.textContent, '200 bytes');

    // Sync again: clear tags and git status
    const clearedItem: PaneItem = {
      base: 'test.txt',
      isDir: false,
      size: null,
      tags: [],
      gitStatus: '',
    };
    renderer.syncRow(row, clearedItem, pane, 0);
    assert.equal((row as any)._tagsKey, '');
    assert.equal((row as any)._gitStatus, '');
  } finally {
    cleanup();
  }
});
