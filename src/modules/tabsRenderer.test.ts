// src/modules/tabsRenderer.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { TabsRenderer } from './tabsRenderer.ts';
import type { AppState } from './stateModels.ts';

function createMockElement(tag = 'div'): any {
  const children: any[] = [];
  const listeners: Record<string, Function[]> = {};

  const el: any = {
    tagName: tag.toUpperCase(),
    className: '',
    textContent: '',
    innerHTML: '',
    title: '',
    children,
    appendChild(c: any) { children.push(c); return c; },
    replaceChildren(...cs: any[]) { children.length = 0; children.push(...cs); },
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    click(eventPayload: any = {}) {
      const ev = { stopPropagation: () => {}, ...eventPayload };
      (listeners['click'] || []).forEach((fn) => fn(ev));
    },
  };
  return el;
}

function setupDom(elements: Record<string, any>) {
  const origDoc = globalThis.document;
  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
    createElement: (tag: string) => createMockElement(tag),
  };
  return () => {
    globalThis.document = origDoc;
  };
}

function createMockPane(paths: string[], activeIndex = 0): any {
  const tabs = paths.map((p) => ({ path: p }));
  return {
    path: tabs[activeIndex]?.path || '',
    tabs,
    activeTabIndex: activeIndex,
    addTab() {
      tabs.push({ path: this.path });
      this.activeTabIndex = tabs.length - 1;
    },
    closeCurrentTab() {
      if (tabs.length <= 1) return false;
      tabs.splice(this.activeTabIndex, 1);
      this.activeTabIndex = Math.max(0, this.activeTabIndex - 1);
      this.path = tabs[this.activeTabIndex]?.path || '';
      return true;
    },
    nextTab() {
      this.activeTabIndex = (this.activeTabIndex + 1) % tabs.length;
      this.path = tabs[this.activeTabIndex]?.path || '';
    },
    prevTab() {
      this.activeTabIndex = (this.activeTabIndex - 1 + tabs.length) % tabs.length;
      this.path = tabs[this.activeTabIndex]?.path || '';
    },
  };
}

test('TabsRenderer shortName formats path correctly', () => {
  const renderer = new TabsRenderer({
    state: {} as any,
    loadDir: async () => {},
    updatePaneClass: () => {},
    focusActiveList: () => {},
    renderPane: () => {},
    syncFilterInput: () => {},
  });

  assert.equal(renderer.shortName(undefined), '—');
  assert.equal(renderer.shortName(''), '—');
  assert.equal(renderer.shortName('/'), '/');
  assert.equal(renderer.shortName('C:\\'), 'C:');
  assert.equal(renderer.shortName('/home/user/documents'), 'documents');
  assert.equal(renderer.shortName('D:\\Projects\\Oryn'), 'Oryn');
});

test('TabsRenderer render populates tabs, handles click and close', async () => {
  const leftContainer = createMockElement('div');
  const rightContainer = createMockElement('div');
  const folderTitle = createMockElement('div');

  const cleanup = setupDom({
    'tabs-left': leftContainer,
    'tabs-right': rightContainer,
    'folder-title': folderTitle,
  });

  const state: AppState = {
    active: 'left',
    left: createMockPane(['/home/user/docs', '/home/user/pics'], 0),
    right: createMockPane(['/var/log'], 0),
  } as any;

  let loadedSide = '';
  let updatedClasses = false;
  let focused = false;
  let renderedSide = '';

  const renderer = new TabsRenderer({
    state,
    loadDir: async (s) => { loadedSide = s; },
    updatePaneClass: () => { updatedClasses = true; },
    focusActiveList: () => { focused = true; },
    renderPane: (s) => { renderedSide = s; },
    syncFilterInput: () => {},
  });

  try {
    renderer.setup();
    assert.equal(folderTitle.textContent, 'docs');
    assert.equal(leftContainer.children.length, 2);
    assert.equal(rightContainer.children.length, 1);

    // Left pane has 2 tabs: first is active, has close button
    const tab0 = leftContainer.children[0];
    const tab1 = leftContainer.children[1];

    assert.ok(tab0.className.includes('tab-btn--active'));
    assert.equal(tab0.children[1].textContent, 'docs');
    // Tab close button is children[2]
    const closeBtn = tab0.children[2];
    assert.equal(closeBtn.className, 'tab-close');

    // Right pane has 1 tab: no close button
    const rightTab0 = rightContainer.children[0];
    assert.equal(rightTab0.children.length, 2); // icon + name, no close

    // Click tab 1 to activate it
    tab1.click();
    assert.equal(state.left.activeTabIndex, 1);
    await new Promise((r) => setTimeout(r, 0));
    assert.equal(loadedSide, 'left');
    assert.equal(renderedSide, 'left');
    assert.equal(focused, true);

    // Close tab 1
    const newTab1 = leftContainer.children[1];
    const newClose = newTab1.children[2];
    newClose.click();
    assert.equal(state.left.tabs.length, 1);
    assert.equal(state.left.activeTabIndex, 0);
  } finally {
    cleanup();
  }
});

test('TabsRenderer handleGlobalKeydown processes Ctrl+T, Ctrl+W, and Ctrl+Tab', async () => {
  const leftContainer = createMockElement('div');
  const cleanup = setupDom({ 'tabs-left': leftContainer });

  const state: AppState = {
    active: 'left',
    left: createMockPane(['/a', '/b'], 0),
    right: createMockPane(['/c'], 0),
  } as any;

  let loaded = false;
  let focused = false;
  let rendered = false;

  const renderer = new TabsRenderer({
    state,
    loadDir: async () => { loaded = true; },
    updatePaneClass: () => {},
    focusActiveList: () => { focused = true; },
    renderPane: () => { rendered = true; },
    syncFilterInput: () => {},
  });

  try {
    // Unrelated key
    const irrelevantEv = { ctrlKey: false, key: 'x', preventDefault: () => {} } as any;
    assert.equal(renderer.handleGlobalKeydown(irrelevantEv), false);

    // Ctrl+T: new tab
    let prevented = false;
    const ctrlT = { ctrlKey: true, shiftKey: false, key: 't', preventDefault: () => { prevented = true; } } as any;
    assert.equal(renderer.handleGlobalKeydown(ctrlT), true);
    assert.equal(prevented, true);
    assert.equal(state.left.tabs.length, 3);
    await new Promise((r) => setTimeout(r, 0));
    assert.equal(loaded, true);

    // Ctrl+Tab: next tab
    const ctrlTab = { ctrlKey: true, shiftKey: false, key: 'Tab', preventDefault: () => {} } as any;
    assert.equal(renderer.handleGlobalKeydown(ctrlTab), true);
    assert.equal(state.left.activeTabIndex, 0); // wrapped around from 2 to 0

    // Ctrl+Shift+Tab: prev tab
    const ctrlShiftTab = { ctrlKey: true, shiftKey: true, key: 'Tab', preventDefault: () => {} } as any;
    assert.equal(renderer.handleGlobalKeydown(ctrlShiftTab), true);
    assert.equal(state.left.activeTabIndex, 2);

    // Ctrl+W: close current tab
    const ctrlW = { ctrlKey: true, key: 'w', preventDefault: () => {} } as any;
    assert.equal(renderer.handleGlobalKeydown(ctrlW), true);
    assert.equal(state.left.tabs.length, 2);
  } finally {
    cleanup();
  }
});
