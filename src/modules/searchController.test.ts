// src/modules/searchController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { SearchController } from './searchController.ts';

(globalThis as any).requestAnimationFrame = (fn: any) => setTimeout(fn, 0);
(globalThis as any).cancelAnimationFrame = (id: any) => clearTimeout(id);

if (!(globalThis as any).document) {
  (globalThis as any).document = {};
}
(globalThis as any).document.createElement = (tag: string) => createMockElement(tag);
(globalThis as any).document.createDocumentFragment = () => createMockElement('fragment');

function createMockElement(tag = 'div'): any {
  const children: any[] = [];
  const style: Record<string, any> = {};
  const classList = {
    _classes: new Set<string>(),
    add(c: string) { this._classes.add(c); },
    remove(c: string) { this._classes.delete(c); },
    contains(c: string) { return this._classes.has(c); },
    toggle(c: string, force?: boolean) {
      if (force === undefined) {
        if (this._classes.has(c)) this._classes.delete(c);
        else this._classes.add(c);
      } else if (force) {
        this._classes.add(c);
      } else {
        this._classes.delete(c);
      }
    },
  };

  const el: any = {
    tagName: tag.toUpperCase(),
    className: '',
    classList,
    style,
    dataset: {},
    value: '',
    textContent: '',
    innerHTML: '',
    children,
    append(...cs: any[]) { children.push(...cs); },
    setAttribute(k: string, v: string) { (this as any)[k] = v; },
    appendChild(c: any) { children.push(c); return c; },
    replaceChildren(...cs: any[]) { children.length = 0; children.push(...cs); },
    focus() { el._focused = true; },
    addEventListener(evt: string, fn: any) {
      el._listeners = el._listeners || {};
      el._listeners[evt] = fn;
    },
    closest(sel: string) {
      if (sel === '#search-close' && el.id === 'search-close') return el;
      return null;
    },
  };
  return el;
}

test('SearchController root, tab switching, and overlay open/hide lifecycle', async () => {
  const elements: Record<string, any> = {};
  const getEl = (id: string) => {
    if (!elements[id]) {
      elements[id] = createMockElement('div');
      elements[id].id = id;
    }
    return elements[id];
  };

  const tabButtons: any[] = [
    (() => { const b = createMockElement('button'); b.dataset.tab = 'standard'; return b; })(),
    (() => { const b = createMockElement('button'); b.dataset.tab = 'advanced'; return b; })(),
    (() => { const b = createMockElement('button'); b.dataset.tab = 'results'; return b; })(),
  ];

  const origDoc = globalThis.document;
  const origWindow = (globalThis as any).window;
  try {
    (globalThis as any).window = {
      clearTimeout: () => {},
      setTimeout: (fn: any) => { fn(); return 1; },
    };
    (globalThis as any).document = {
      createElement: (tag: string) => createMockElement(tag),
      createDocumentFragment: () => createMockElement('fragment'),
      getElementById: (id: string) => getEl(id),
      querySelectorAll: (sel: string) => {
        if (sel.includes('[data-tab]')) return tabButtons;
        return [];
      },
    };

    let focusedList = false;
    let openedHit: any = null;

    const mockState: any = {
      active: 'left',
      left: { path: '/home/projects' },
      right: { path: '/home/backup' },
    };

    const controller = new SearchController({
      state: mockState,
      api: () => ({
        cancelSearch: async () => {},
        cancelSearchTask: async () => {},
      }),
      focusActiveList: () => { focusedList = true; },
      openSearchHit: async (hit) => { openedHit = hit; },
    });

    // 1. getCurrentRoot uses active pane path by default
    assert.equal(controller.getCurrentRoot(), '/home/projects');

    // 2. setSearchRoot overrides current root
    controller.setSearchRoot('/custom/root');
    assert.equal(controller.getCurrentRoot(), '/custom/root');
    assert.equal(getEl('search-root-input').value, '/custom/root');

    // 3. switchSearchTab toggles tab classes
    controller.switchSearchTab('advanced');
    assert.ok(getEl('search-tab-advanced').classList.contains('hidden') === false);
    assert.ok(getEl('search-tab-standard').classList.contains('hidden') === true);
    assert.ok(tabButtons[1].classList.contains('search-tab--active') === true);

    // Invalid tab name ignored
    controller.switchSearchTab('invalid-tab');

    // 4. openOverlay resets inputs and shows overlay
    getEl('search-filename').value = 'old query';
    controller.openOverlay();
    assert.equal(getEl('search-filename').value, '');
    assert.equal(getEl('search-overlay').classList.contains('hidden'), false);
    assert.equal(getEl('search-filename')._focused, true);

    // 5. runQuery delegates to queryRunner
    let queryRan = false;
    controller.queryRunner.runQuery = async () => { queryRan = true; };
    await controller.runQuery();
    assert.equal(queryRan, true);

    // 6. hideOverlay hides overlay and restores list focus
    controller.hideOverlay();
    assert.equal(getEl('search-overlay').classList.contains('hidden'), true);
    assert.equal(focusedList, true);
  } finally {
    await new Promise((r) => setTimeout(r, 20));
    if (origDoc) {
      origDoc.createElement = (tag: string) => createMockElement(tag);
      origDoc.createDocumentFragment = () => createMockElement('fragment');
      globalThis.document = origDoc;
    }
    if (origWindow) (globalThis as any).window = origWindow;
  }
});

test('SearchController setupUI wires buttons, inputs, keyboard navigation and browse folder', async () => {
  const elements: Record<string, any> = {};
  const getEl = (id: string) => {
    if (!elements[id]) {
      elements[id] = createMockElement('div');
      elements[id].id = id;
    }
    return elements[id];
  };

  const tabButtons: any[] = [
    (() => { const b = createMockElement('button'); b.dataset.tab = 'standard'; return b; })(),
    (() => { const b = createMockElement('button'); b.dataset.tab = 'advanced'; return b; })(),
  ];

  const origDoc = globalThis.document;
  const origWindow = (globalThis as any).window;
  try {
    (globalThis as any).window = {
      clearTimeout: () => {},
      setTimeout: (fn: any) => { fn(); return 1; },
    };
    (globalThis as any).document = {
      createElement: (tag: string) => createMockElement(tag),
      createDocumentFragment: () => createMockElement('fragment'),
      getElementById: (id: string) => getEl(id),
      querySelectorAll: (sel: string) => {
        if (sel.includes('[data-tab]')) return tabButtons;
        return [];
      },
    };

    let queryRan = false;
    let pickedFolderCalled = false;
    let hitOpened: any = null;

    const mockApi = {
      pickFolder: async (initial?: string) => {
        pickedFolderCalled = true;
        return { ok: true, path: '/picked/folder' };
      },
      searchStart: async () => ({ ok: true, task_id: 't-1' }),
      cancelSearch: async () => {},
      cancelSearchTask: async () => {},
    };

    const mockState: any = {
      active: 'left',
      left: { path: '/home' },
    };

    const controller = new SearchController({
      state: mockState,
      api: () => mockApi,
      focusActiveList: () => {},
      openSearchHit: async (hit) => { hitOpened = hit; },
    });

    controller.setupUI();

    // 1. Close button click hides overlay
    getEl('search-close')._listeners['click']();
    assert.equal(getEl('search-overlay').classList.contains('hidden'), true);

    // 2. Escape key on overlay hides it
    let prevented = false;
    getEl('search-overlay')._listeners['keydown']({
      key: 'Escape',
      preventDefault() { prevented = true; },
    });
    assert.equal(prevented, true);

    // 3. Tab buttons click switches tabs
    tabButtons[1]._listeners['click']();
    assert.ok(tabButtons[1].classList.contains('search-tab--active'));

    // 4. Browse button picks folder
    await getEl('search-browse-btn')._listeners['click']();
    assert.equal(pickedFolderCalled, true);
    assert.equal(controller.getCurrentRoot(), '/picked/folder');

    // 5. Root input change
    getEl('search-root-input')._listeners['change']({ target: { value: '  /typed/root  ' } });
    assert.equal(controller.getCurrentRoot(), '/typed/root');

    // 6. New search button resets inputs
    getEl('search-filename').value = 'query';
    getEl('search-new-btn')._listeners['click']();
    assert.equal(getEl('search-filename').value, '');

    // 7. Filename Enter key triggers queryRunner
    let enterPrevented = false;
    controller.queryRunner.runQuery = async () => { queryRan = true; };
    getEl('search-filename')._listeners['keydown']({
      key: 'Enter',
      preventDefault() { enterPrevented = true; },
    });
    assert.equal(enterPrevented, true);
    assert.equal(queryRan, true);

    // 8. Filename ArrowDown with results switches to results tab
    controller.queryRunner.getSearchResultCount = () => 5;
    let arrowDownPrevented = false;
    getEl('search-filename')._listeners['keydown']({
      key: 'ArrowDown',
      preventDefault() { arrowDownPrevented = true; },
    });
    assert.equal(arrowDownPrevented, true);
    assert.equal(controller.queryRunner.searchResultIndex, 0);

    // 9. Search option select change triggers debounce
    getEl('search-mode')._listeners['change']();

    // 10. Results navigation: ArrowDown, ArrowUp and Enter
    let openResultCalled = false;
    controller.queryRunner.openSelectedResult = async () => { openResultCalled = true; };
    controller.queryRunner.updateSearchHitSelection = () => {};

    // ArrowDown
    getEl('search-results')._listeners['keydown']({
      key: 'ArrowDown',
      preventDefault() {},
    });
    assert.equal(controller.queryRunner.searchResultIndex, 1);

    // ArrowUp back to 0
    getEl('search-results')._listeners['keydown']({
      key: 'ArrowUp',
      preventDefault() {},
    });
    assert.equal(controller.queryRunner.searchResultIndex, 0);

    // ArrowUp past 0 focuses search field and resets index to -1
    getEl('search-results')._listeners['keydown']({
      key: 'ArrowUp',
      preventDefault() {},
    });
    assert.equal(controller.queryRunner.searchResultIndex, -1);
    assert.equal(getEl('search-filename')._focused, true);

    // Enter
    getEl('search-results')._listeners['keydown']({
      key: 'Enter',
      preventDefault() {},
    });
    assert.equal(openResultCalled, true);

    // 11. Search start button click
    let startQueryRan = false;
    controller.queryRunner.runQuery = async () => { startQueryRan = true; };
    getEl('search-start-btn')._listeners['click']();
    assert.equal(startQueryRan, true);

    // 12. Overlay backdrop click closes overlay
    const overlay = getEl('search-overlay');
    overlay.classList.remove('hidden');
    overlay._listeners['click']({ target: overlay, preventDefault() {} });
    assert.ok(overlay.classList.contains('hidden'));
  } finally {
    await new Promise((r) => setTimeout(r, 20));
    if (origDoc) {
      origDoc.createElement = (tag: string) => createMockElement(tag);
      origDoc.createDocumentFragment = () => createMockElement('fragment');
      globalThis.document = origDoc;
    }
    if (origWindow) (globalThis as any).window = origWindow;
  }
});
