// src/modules/drivePopupController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { DrivePopupController, type DriveItem } from './drivePopupController.ts';
import type { AppState } from './stateModels.ts';

function createMockElement(tag = 'div'): any {
  const classes = new Set<string>();
  const children: any[] = [];
  const listeners: Record<string, Function[]> = {};

  const el: any = {
    id: '',
    tagName: tag.toUpperCase(),
    textContent: '',
    innerHTML: '',
    children,
    style: { top: '', left: '' },
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
    appendChild(c: any) { children.push(c); return c; },
    replaceChildren(...cs: any[]) { children.length = 0; children.push(...cs); },
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    click() {
      (listeners['click'] || []).forEach((fn) => fn({ stopPropagation: () => {} }));
    },
    mouseenter() {
      (listeners['mouseenter'] || []).forEach((fn) => fn({ stopPropagation: () => {} }));
    },
    getBoundingClientRect() {
      return { top: 100, left: 200, width: 400, height: 600 };
    },
    querySelectorAll(selector: string) {
      if (selector === '.drive-popup-item') {
        const list = children.find((c) => c.className === 'drive-popup-list');
        return list ? list.children : [];
      }
      return [];
    },
  };
  return el;
}

function setupDom() {
  const origDoc = globalThis.document;
  const elements: Record<string, any> = {};
  const docListeners: Record<string, Function[]> = {};

  const doc = {
    body: {
      appendChild: (c: any) => { elements[c.id] = c; },
    },
    getElementById: (id: string) => elements[id] || null,
    createElement: (tag: string) => createMockElement(tag),
    addEventListener: (ev: string, fn: Function) => {
      if (!docListeners[ev]) docListeners[ev] = [];
      docListeners[ev].push(fn);
    },
    removeEventListener: (ev: string, fn: Function) => {
      if (docListeners[ev]) {
        docListeners[ev] = docListeners[ev].filter((f) => f !== fn);
      }
    },
    triggerKeydown: (e: any) => {
      const ev = { preventDefault: () => {}, stopPropagation: () => {}, ...e };
      (docListeners['keydown'] || []).forEach((fn) => fn(ev));
    },
  };

  (globalThis as any).document = doc;

  return {
    elements,
    triggerKeydown: doc.triggerKeydown,
    cleanup: () => {
      globalThis.document = origDoc;
    },
  };
}

function createMockState(): AppState {
  return {
    active: 'left',
    left: { path: '/old/left' } as any,
    right: { path: '/old/right' } as any,
  } as any;
}

test('DrivePopupController open and close handles empty, error, and valid locations', async () => {
  const { elements, cleanup } = setupDom();
  const state = createMockState();

  const paneLeft = createMockElement('div');
  paneLeft.id = 'pane-left';
  elements['pane-left'] = paneLeft;

  let statusMsg = '';
  let focused = false;
  let loadedSide = '';

  let mockLocations: any = { locations: [] };
  let shouldThrow = false;

  const controller = new DrivePopupController({
    state,
    api: () => ({
      getSystemLocations: async () => {
        if (shouldThrow) throw new Error('API offline');
        return mockLocations;
      },
    }),
    loadDir: async (s) => { loadedSide = s; },
    focusActiveList: () => { focused = true; },
    setStatus: (m) => { statusMsg = m; },
  });

  try {
    // 1. Empty locations
    await controller.open('left');
    assert.equal(statusMsg, 'No system drives detected.');
    assert.equal(controller.isOpen, false);

    // 2. Error throwing
    shouldThrow = true;
    await controller.open('left');
    assert.ok(statusMsg.startsWith('Drive detection error:'));

    // 3. Valid locations
    shouldThrow = false;
    mockLocations = {
      locations: [
        { kind: 'drive', path: 'C:\\', name: 'Local Disk (C:)' },
        { kind: 'volume', path: 'D:\\', name: 'Data (D:)' },
        { kind: 'network', path: '\\\\server\\share', name: 'Share' },
      ],
    };

    await controller.open('left');
    assert.equal(controller.isOpen, true);
    assert.equal(controller.drives.length, 2); // only drive and volume
    assert.equal(controller.targetSide, 'left');
    assert.equal(state.active, 'left');

    const popup = elements['drive-selection-popup'];
    assert.ok(popup);
    assert.equal(popup.classList.contains('hidden'), false);
    assert.equal(popup.style.top, '136px'); // 100 + 36
    assert.equal(popup.style.left, '212px'); // 200 + 12

    // Close popup
    controller.close();
    assert.equal(controller.isOpen, false);
    assert.equal(popup.classList.contains('hidden'), true);
    assert.equal(focused, true);
  } finally {
    cleanup();
  }
});

test('DrivePopupController render and click selects drive', async () => {
  const { elements, cleanup } = setupDom();
  const state = createMockState();
  const paneRight = createMockElement('div');
  paneRight.id = 'pane-right';
  elements['pane-right'] = paneRight;

  let loadedSide = '';
  let status = '';

  const controller = new DrivePopupController({
    state,
    api: () => ({
      getSystemLocations: async () => ({
        locations: [
          { kind: 'drive', path: 'C:\\', name: 'System' },
          { kind: 'drive', path: 'E:\\', name: 'USB' },
        ],
      }),
    }),
    loadDir: async (s) => { loadedSide = s; },
    focusActiveList: () => {},
    setStatus: (m) => { status = m; },
  });

  try {
    await controller.open('right');

    const popup = elements['drive-selection-popup'];
    const list = popup.children[1];
    assert.equal(list.children.length, 2);

    // Mouseenter changes selection
    list.children[1].mouseenter();
    assert.equal(controller.selectedIndex, 1);

    // Click item selects drive
    list.children[1].click();
    assert.equal(controller.isOpen, false);
    assert.equal(state.right.path, 'E:\\');
    assert.equal(loadedSide, 'right');
    assert.equal(status, 'Switched right pane to E:\\');
  } finally {
    cleanup();
  }
});

test('DrivePopupController keyboard navigation handles arrows, enter, escape, and hotkeys', async () => {
  const { triggerKeydown, cleanup } = setupDom();
  const state = createMockState();

  let loadedSide = '';

  const controller = new DrivePopupController({
    state,
    api: () => ({
      getSystemLocations: async () => ({
        locations: [
          { kind: 'drive', path: 'C:\\', name: 'Drive C' },
          { kind: 'drive', path: 'D:\\', name: 'Drive D' },
          { kind: 'drive', path: 'E:\\', name: 'Drive E' },
        ],
      }),
    }),
    loadDir: async (s) => { loadedSide = s; },
    focusActiveList: () => {},
    setStatus: () => {},
  });

  try {
    await controller.open('left');
    assert.equal(controller.selectedIndex, 0);

    // ArrowDown
    triggerKeydown({ key: 'ArrowDown' });
    assert.equal(controller.selectedIndex, 1);

    triggerKeydown({ key: 'ArrowDown' });
    assert.equal(controller.selectedIndex, 2);

    // ArrowDown wraps to 0
    triggerKeydown({ key: 'ArrowDown' });
    assert.equal(controller.selectedIndex, 0);

    // ArrowUp wraps to 2
    triggerKeydown({ key: 'ArrowUp' });
    assert.equal(controller.selectedIndex, 2);

    // Enter selects index 2 (E:\)
    triggerKeydown({ key: 'Enter' });
    assert.equal(controller.isOpen, false);
    assert.equal(state.left.path, 'E:\\');

    // Reopen and test direct drive letter key (e.g. 'D')
    await controller.open('left');
    triggerKeydown({ key: 'd' });
    assert.equal(controller.isOpen, false);
    assert.equal(state.left.path, 'D:\\');

    // Reopen and test Escape
    await controller.open('left');
    assert.equal(controller.isOpen, true);
    triggerKeydown({ key: 'Escape' });
    assert.equal(controller.isOpen, false);
  } finally {
    cleanup();
  }
});
