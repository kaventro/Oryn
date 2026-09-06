// src/modules/propertiesController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { PropertiesController, type PropertyItem } from './propertiesController.ts';

function createMockElement(tag = 'div'): any {
  const classes = new Set<string>();
  const children: any[] = [];
  const attrs: Record<string, string> = {};
  const listeners: Record<string, Function[]> = {};

  const el: any = {
    tagName: tag.toUpperCase(),
    textContent: '',
    title: '',
    type: '',
    children,
    style: { display: '', background: '' },
    classList: {
      add(c: string) { classes.add(c); el.className = [...classes].join(' '); },
      remove(c: string) { classes.delete(c); el.className = [...classes].join(' '); },
      contains(c: string) { return classes.has(c); },
      toggle(c: string, force?: boolean) {
        const next = force !== undefined ? force : !classes.has(c);
        if (next) classes.add(c); else classes.delete(c);
        el.className = [...classes].join(' ');
        return next;
      },
    },
    setAttribute(k: string, v: string) { attrs[k] = v; },
    getAttribute(k: string) { return attrs[k] ?? null; },
    appendChild(c: any) { children.push(c); return c; },
    append(...cs: any[]) { children.push(...cs); },
    replaceChildren(...cs: any[]) { children.length = 0; children.push(...cs); },
    focus() {},
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    click(target?: any) {
      const ev = { target: target || el, stopPropagation: () => {} };
      (listeners['click'] || []).forEach((fn) => fn(ev));
    },
  };
  return el;
}

function setupDom(elements: Record<string, any>) {
  const origDoc = globalThis.document;
  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
    querySelector: (sel: string) => elements[sel] || null,
    createElement: (tag: string) => createMockElement(tag),
  };
  return () => {
    globalThis.document = origDoc;
  };
}

test('PropertiesController setup and copy button copies path to clipboard', async () => {
  const overlay = createMockElement();
  const modal = createMockElement();
  const pathEl = createMockElement();
  const copyBtn = createMockElement();
  const closeBtn = createMockElement();

  const elements = {
    'properties-overlay': overlay,
    'properties-modal': modal,
    'properties-path': pathEl,
    'properties-copy': copyBtn,
    'properties-close': closeBtn,
  };
  const cleanup = setupDom(elements);

  let clipboardText = '';
  let statusText = '';
  let focused = false;

  const controller = new PropertiesController({
    api: () => ({
      clipboardWrite: async (txt: string) => { clipboardText = txt; },
    }),
    setStatus: (m) => { statusText = m; },
    focusActiveList: () => { focused = true; },
    getFilteredSelection: () => ({ item: null }),
    fullPath: async () => null,
  });

  try {
    pathEl.textContent = '/home/user/document.pdf';
    await copyBtn.click();
    assert.equal(clipboardText, '/home/user/document.pdf');
    assert.equal(statusText, 'Path copied to clipboard.');

    // Close button hides
    closeBtn.click();
    assert.equal(overlay.classList.contains('hidden'), true);
    assert.equal(focused, true);

    // Overlay click outside modal hides
    overlay.classList.remove('hidden');
    overlay.click(overlay);
    assert.equal(overlay.classList.contains('hidden'), true);
  } finally {
    cleanup();
  }
});

test('PropertiesController show populates fields for file and tags', () => {
  const overlay = createMockElement();
  const titleEl = createMockElement();
  const kindEl = createMockElement();
  const pathEl = createMockElement();
  const typeEl = createMockElement();
  const sizeEl = createMockElement();
  const modeEl = createMockElement();
  const mtimeEl = createMockElement();
  const tagsListEl = createMockElement();
  const summaryEl = createMockElement();
  const tagsCard = createMockElement();

  const elements = {
    'properties-overlay': overlay,
    'properties-title': titleEl,
    'properties-kind': kindEl,
    'properties-path': pathEl,
    'properties-type': typeEl,
    'properties-size': sizeEl,
    'properties-mode': modeEl,
    'properties-mtime': mtimeEl,
    'properties-tags-list': tagsListEl,
    'properties-summary': summaryEl,
    '.properties-tags-card': tagsCard,
  };
  const cleanup = setupDom(elements);

  let toggledTag = '';
  const mockTagController = {
    isEnabled: true,
    getAllTags: () => [
      { id: 'red', name: 'Important', color: '#ff0000' },
      { id: 'custom', name: 'Work', color: '#123456' },
    ],
    getTagsForFile: (p: string) => (p.includes('report') ? ['red'] : []),
    toggleTagForFile: (_path: string, tagId: string) => { toggledTag = tagId; },
  };

  let statusMsg = '';
  const controller = new PropertiesController({
    api: () => ({}),
    setStatus: (m) => { statusMsg = m; },
    focusActiveList: () => {},
    getFilteredSelection: () => ({ item: null }),
    fullPath: async () => null,
    tagController: mockTagController,
  });

  const fileProps: PropertyItem = {
    path: '/docs/annual-report.xlsx',
    size: 4096,
    mode: '644',
    modeString: '-rw-r--r--',
    isDir: false,
    mtime: '2026-09-01T12:00:00Z',
  };

  try {
    controller.show(fileProps);

    assert.equal(titleEl.textContent, 'annual-report.xlsx');
    assert.equal(kindEl.textContent, 'FILE');
    assert.equal(typeEl.textContent, 'File');
    assert.ok(sizeEl.textContent.includes('4.0 kB'));
    assert.equal(modeEl.textContent, '-rw-r--r-- (0o644)');
    assert.equal(overlay.classList.contains('hidden'), false);
    assert.equal(summaryEl.textContent, 'Click "Copy path" to copy full file location.');

    // Check rendered tags
    assert.equal(tagsListEl.children.length, 2);
    const redChip = tagsListEl.children[0];
    assert.ok(redChip.className.includes('active')); // file has red tag
    const customChip = tagsListEl.children[1];
    assert.equal(customChip.className.includes('active'), false);

    // Toggle custom tag
    customChip.click();
    assert.equal(toggledTag, 'custom');
    assert.ok(customChip.className.includes('active'));
    assert.equal(statusMsg, 'Toggled tag "Work"');
  } finally {
    cleanup();
  }
});

test('PropertiesController showFor fetches stats and directory size asynchronously', async () => {
  const overlay = createMockElement();
  const sizeEl = createMockElement();
  const summaryEl = createMockElement();

  const elements = {
    'properties-overlay': overlay,
    'properties-size': sizeEl,
    'properties-summary': summaryEl,
  };
  const cleanup = setupDom(elements);

  let statusMsg = '';
  const mockApi = {
    statProps: async (fp: string) => {
      if (fp.includes('notfound')) return { ok: false, error: 'File not found' };
      return {
        ok: true,
        props: {
          path: fp,
          size: 0,
          mode: '755',
          isDir: true,
          mtime: '2026-09-01T10:00:00Z',
        },
      };
    },
    getDirSize: async (fp: string) => {
      if (fp.includes('errordir')) throw new Error('Failed to read');
      return { ok: true, size: 8192, files: 5, dirs: 2 };
    },
  };

  const controller = new PropertiesController({
    api: () => mockApi,
    setStatus: (m) => { statusMsg = m; },
    focusActiveList: () => {},
    getFilteredSelection: () => ({ item: { base: 'projects' } }),
    fullPath: async (_side, item) => `/var/${item.base}`,
  });

  try {
    // Missing selection
    (controller as any).getFilteredSelection = () => ({ item: null });
    await controller.showFor('left');

    // Stat error
    await controller.showFor('left', '/var/notfound');
    assert.equal(statusMsg, 'File not found');

    // Successful directory inspection
    await controller.showFor('left', '/var/projects');
    assert.equal(summaryEl.textContent, 'Directory size includes the recursive contents below it.');
    assert.ok(sizeEl.textContent.includes('8.0 kB'));
    assert.ok(sizeEl.textContent.includes('5 files, 2 dirs'));

    // Directory size failure handling
    await controller.showFor('left', '/var/errordir');
    assert.equal(sizeEl.textContent, 'Size unavailable');
  } finally {
    cleanup();
  }
});
