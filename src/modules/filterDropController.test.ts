// src/modules/filterDropController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { FilterDropController } from './filterDropController.ts';
import type { AppState, Item } from './stateModels.ts';

function createMockElement(tag = 'div', initialProps: Record<string, any> = {}): any {
  const classes = new Set<string>();
  const listeners: Record<string, Function[]> = {};
  const children: any[] = [];
  const dataset: Record<string, string> = {};

  return {
    tagName: tag.toUpperCase(),
    textContent: '',
    value: '',
    dataset,
    children,
    ...initialProps,
    classList: {
      add(c: string) { classes.add(c); },
      remove(c: string) { classes.delete(c); },
      contains(c: string) { return classes.has(c); },
      toggle(c: string, force?: boolean) {
        const has = classes.has(c);
        const next = force !== undefined ? force : !has;
        if (next) classes.add(c);
        else classes.delete(c);
        return next;
      },
    },
    replaceChildren() { children.length = 0; },
    appendChild(c: any) { children.push(c); },
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    dispatchEvent(ev: string, ...args: any[]) {
      (listeners[ev] || []).forEach((fn) => fn(...args));
    },
    scrollIntoView(_opts?: any) {},
    querySelectorAll(selector: string) {
      if (selector === '.fdrop-item') {
        return children.filter((c) => c.className && c.className.includes('fdrop-item'));
      }
      return [];
    },
  };
}

function setupDom(elements: Record<string, any>) {
  const origDoc = globalThis.document;
  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
    createElement: (tag: string) => createMockElement(tag),
    createTextNode: (txt: string) => ({ textContent: txt }),
  };
  return () => {
    globalThis.document = origDoc;
  };
}

function createMockState(filter = '', items: Item[] = []): AppState {
  return {
    active: 'left',
    left: {
      items,
      filter,
      cursor: 0,
      selected: new Set(),
      history: [],
      historyIndex: -1,
      sortCol: 'name',
      sortAsc: true,
      currentPath: '/test',
    } as any,
    right: {
      items: [],
      filter: '',
      cursor: 0,
      selected: new Set(),
      history: [],
      historyIndex: -1,
      sortCol: 'name',
      sortAsc: true,
      currentPath: '/test2',
    } as any,
  } as any;
}

test('FilterDropController hide resets index and adds hidden class', () => {
  const dropdown = createMockElement('div');
  const cleanup = setupDom({ 'filter-dropdown': dropdown });

  const state = createMockState();
  const controller = new FilterDropController({
    state,
    renderPane: () => {},
    focusActiveList: () => {},
  });

  try {
    controller._idx = 3;
    controller.hide();
    assert.equal(controller._idx, -1);
    assert.ok(dropdown.classList.contains('hidden'));
  } finally {
    cleanup();
  }
});

test('FilterDropController render hides when filter is empty or matches nothing', () => {
  const dropdown = createMockElement('div');
  const cleanup = setupDom({ 'filter-dropdown': dropdown });

  // Empty filter
  const state = createMockState('', [{ base: 'file1.txt', isDir: false } as any]);
  const controller = new FilterDropController({
    state,
    renderPane: () => {},
    focusActiveList: () => {},
  });

  try {
    controller.render();
    assert.ok(dropdown.classList.contains('hidden'));

    // Filter with no matches (only '..')
    state.left.filter = 'xyz';
    state.left.items = [{ base: '..', isDir: true } as any];
    controller.render();
    assert.ok(dropdown.classList.contains('hidden'));
  } finally {
    cleanup();
  }
});

test('FilterDropController render populates rows and handles overflow', () => {
  const dropdown = createMockElement('div');
  const cleanup = setupDom({ 'filter-dropdown': dropdown });

  const items: Item[] = [];
  for (let i = 0; i < 15; i++) {
    items.push({
      base: `test_file_${i}.txt`,
      isDir: i % 2 === 0,
    } as any);
  }

  const state = createMockState('test', items);
  let renderedSide = '';
  let focused = false;

  const controller = new FilterDropController({
    state,
    renderPane: (side) => { renderedSide = side; },
    focusActiveList: () => { focused = true; },
  });
  controller._idx = 1;

  try {
    controller.render();
    assert.equal(dropdown.classList.contains('hidden'), false);
    // 10 items + 1 more indicator = 11 children
    assert.equal(dropdown.children.length, 11);

    const firstRow = dropdown.children[0];
    assert.ok(firstRow.className.includes('fdrop-item'));
    assert.ok(firstRow.className.includes('fdrop-item--dir'));

    const secondRow = dropdown.children[1];
    assert.ok(secondRow.classList.contains('fdrop-item--sel'));

    const moreRow = dropdown.children[10];
    assert.equal(moreRow.className, 'fdrop-more');
    assert.equal(moreRow.textContent, '+5 more…');

    // Simulate clicking / mousedown on a row
    let prevented = false;
    firstRow.dispatchEvent('mousedown', {
      preventDefault: () => { prevented = true; },
    });
    assert.equal(prevented, true);
    assert.equal(state.left.filter, 'test_file_0.txt');
    assert.equal(renderedSide, 'left');
    assert.equal(focused, true);
  } finally {
    cleanup();
  }
});

test('FilterDropController moveSelection updates selection index and wraps bounds', () => {
  const dropdown = createMockElement('div');
  const cleanup = setupDom({ 'filter-dropdown': dropdown });

  const items: Item[] = [
    { base: 'apple.txt', isDir: false } as any,
    { base: 'apricot.txt', isDir: false } as any,
    { base: 'banana.txt', isDir: false } as any,
  ];

  const state = createMockState('a', items);
  const controller = new FilterDropController({
    state,
    renderPane: () => {},
    focusActiveList: () => {},
  });

  try {
    // Hidden initially
    dropdown.classList.add('hidden');
    assert.equal(controller.moveSelection(1), false);

    // Rendered
    controller.render();
    assert.equal(controller._idx, -1);

    // Move down
    assert.equal(controller.moveSelection(1), true);
    assert.equal(controller._idx, 0);

    // Move down again
    assert.equal(controller.moveSelection(1), true);
    assert.equal(controller._idx, 1);

    // Move past end
    assert.equal(controller.moveSelection(10), true);
    assert.equal(controller._idx, 2);

    // Move before start
    assert.equal(controller.moveSelection(-10), true);
    assert.equal(controller._idx, 0);
  } finally {
    cleanup();
  }
});

test('FilterDropController applySelection updates filter-input and cursor', () => {
  const dropdown = createMockElement('div');
  const filterInput = createMockElement('input');
  const cleanup = setupDom({
    'filter-dropdown': dropdown,
    'filter-input': filterInput,
  });

  const items: Item[] = [
    { base: 'fileA.txt', isDir: false } as any,
    { base: 'fileB.txt', isDir: false } as any,
  ];

  const state = createMockState('file', items);
  let focused = false;
  let paneRendered = '';

  const controller = new FilterDropController({
    state,
    renderPane: (s) => { paneRendered = s; },
    focusActiveList: () => { focused = true; },
  });

  try {
    controller.applySelection('fileB.txt');
    assert.equal(state.left.cursor, 1);
    assert.equal(state.left.filter, 'fileB.txt');
    assert.equal(filterInput.value, 'fileB.txt');
    assert.equal(paneRendered, 'left');
    assert.equal(focused, true);
    assert.ok(dropdown.classList.contains('hidden'));
  } finally {
    cleanup();
  }
});
