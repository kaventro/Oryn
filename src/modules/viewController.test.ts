// src/modules/viewController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { ViewController } from './viewController.ts';

// Mock localStorage
const mockStorage: Record<string, string> = {};
Object.defineProperty(globalThis, 'localStorage', {
  value: {
    getItem: (k: string) => mockStorage[k] || null,
    setItem: (k: string, v: any) => { mockStorage[k] = String(v); },
    removeItem: (k: string) => { delete mockStorage[k]; },
    clear: () => { Object.keys(mockStorage).forEach((k) => delete mockStorage[k]); },
  },
  configurable: true,
  writable: true,
});

test('ViewController initializes with default list mode', () => {
  const vc = new ViewController();
  assert.equal(vc.getMode(), 'list');
});

test('ViewController changes modes and triggers callbacks', () => {
  let changedTo = '';
  const vc = new ViewController({
    onModeChange: (mode: string) => {
      changedTo = mode;
    },
  });

  vc.setMode('grid');
  assert.equal(vc.getMode(), 'grid');
  assert.equal(changedTo, 'grid');

  vc.setMode('columns');
  assert.equal(vc.getMode(), 'columns');
  assert.equal(changedTo, 'columns');

  vc.setMode('list');
  assert.equal(vc.getMode(), 'list');
  assert.equal(changedTo, 'list');

  // Same mode is no-op
  let calledAgain = false;
  vc.onModeChange = () => { calledAgain = true; };
  vc.setMode('list');
  assert.equal(calledAgain, false);
});

test('ViewController setupUI and syncUI manage classes and event listeners', () => {
  const classes = (initial = '') => {
    const set = new Set<string>(initial.split(' ').filter(Boolean));
    return {
      contains: (c: string) => set.has(c),
      add: (c: string) => set.add(c),
      remove: (c: string) => set.delete(c),
      toggle: (c: string, force?: boolean) => {
        const next = force !== undefined ? force : !set.has(c);
        if (next) set.add(c); else set.delete(c);
        return next;
      },
    };
  };

  const listeners: Record<string, Function[]> = {};
  const createBtn = () => ({
    classList: classes(),
    addEventListener: (ev: string, fn: Function) => {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    click: () => {
      (listeners['click'] || []).forEach((fn) => fn({ stopPropagation: () => {} }));
    },
  });

  const appEl = { classList: classes() };
  const listBtn = createBtn();
  const gridBtn = createBtn();
  const colsBtn = createBtn();

  const elements: Record<string, any> = {
    app: appEl,
    'btn-view-list': listBtn,
    'btn-view-grid': gridBtn,
    'btn-view-columns': colsBtn,
  };

  const origDoc = globalThis.document;
  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
  };

  try {
    let modeNotified = '';
    const vc = new ViewController({
      onModeChange: (m) => { modeNotified = m; },
    });

    vc.setupUI();
    assert.equal(modeNotified, 'list');
    assert.equal(appEl.classList.contains('list-mode'), true);
    assert.equal(listBtn.classList.contains('mac-toolbar-btn--active'), true);

    // Switch to grid via UI
    vc.setMode('grid');
    assert.equal(appEl.classList.contains('grid-mode'), true);
    assert.equal(gridBtn.classList.contains('mac-toolbar-btn--active'), true);
    assert.equal(listBtn.classList.contains('mac-toolbar-btn--active'), false);

    // Switch to columns
    vc.setMode('columns');
    assert.equal(appEl.classList.contains('columns-mode'), true);
    assert.equal(colsBtn.classList.contains('mac-toolbar-btn--active'), true);
  } finally {
    globalThis.document = origDoc;
  }
});

test('ViewController persistence reads from Oryn or fallback Oswin keys', () => {
  mockStorage['Oryn.viewMode'] = 'grid';
  const vc1 = new ViewController();
  assert.equal(vc1.getMode(), 'grid');

  delete mockStorage['Oryn.viewMode'];
  mockStorage['Oswin.viewMode'] = 'columns';
  const vc2 = new ViewController();
  assert.equal(vc2.getMode(), 'columns');

  mockStorage['Oswin.viewMode'] = 'invalid-mode';
  const vc3 = new ViewController();
  assert.equal(vc3.getMode(), 'list');
});
