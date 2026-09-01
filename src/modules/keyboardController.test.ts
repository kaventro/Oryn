// src/modules/keyboardController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { KeyboardController } from './keyboardController.ts';
import { AppState } from './stateModels.ts';

test('KeyboardController dynamically resolves active panel from focused element', () => {
  const state = new AppState();
  state.active = 'left';

  let switchedActive = '';
  let updatedClass = false;
  let syncedFilter = false;

  const mockPaneRight = {
    id: 'pane-right',
  };
  const mockListRight = {
    id: 'list-right',
    closest(sel: string) {
      if (sel === '#pane-right') return mockPaneRight;
      return null;
    },
  };

  (globalThis as any).document = {
    activeElement: mockListRight,
    addEventListener: () => {},
    getElementById: () => null,
  };

  const controller = new KeyboardController({
    state,
    pathHeaderController: { isEditing: () => false },
    tabsRenderer: { handleGlobalKeydown: () => false },
    quickViewController: { toggle: () => {} },
    commandsController: { runCommand: () => {}, anyMenuOpen: () => false, closeAllMenus: () => {} },
    searchController: { openOverlay: () => {}, hideOverlay: () => {} },
    propertiesController: { isEditing: () => false },
    gitController: {},
    compareDirsController: {},
    focusActiveList: () => {},
    syncFilterInput: () => { syncedFilter = true; },
    updatePaneClass: () => { updatedClass = true; },
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    refreshAll: () => {},
    browseFolderPicker: () => {},
    focusFilterInput: () => {},
    fileOps: {},
    panelController: {},
    listRenderer: {},
    getFilteredSelection: () => ({ item: null }),
    renderPane: () => {},
    moveCursor: () => {},
  });

  const side = (controller as any)._resolveActiveSide();
  assert.equal(side, 'right');
  assert.equal(state.active, 'right');
  assert.equal(updatedClass, true);
  assert.equal(syncedFilter, true);
});

test('KeyboardController routes ArrowDown, ArrowUp, and Enter to focused panel', () => {
  const state = new AppState();
  state.active = 'left';

  let movedSide = '';
  let movedDelta = 0;
  let openedSide = '';

  const mockPaneRight = { id: 'pane-right' };
  const mockListRight = {
    id: 'list-right',
    closest(sel: string) {
      if (sel === '#pane-right') return mockPaneRight;
      return null;
    },
  };

  (globalThis as any).document = {
    activeElement: mockListRight,
    addEventListener: () => {},
    getElementById: () => null,
  };

  const controller = new KeyboardController({
    state,
    pathHeaderController: { isEditing: () => false },
    tabsRenderer: { handleGlobalKeydown: () => false },
    quickViewController: { toggle: () => {} },
    commandsController: { runCommand: () => {}, anyMenuOpen: () => false, closeAllMenus: () => {} },
    searchController: { openOverlay: () => {}, hideOverlay: () => {} },
    propertiesController: { isEditing: () => false },
    gitController: {},
    compareDirsController: {},
    focusActiveList: () => {},
    syncFilterInput: () => {},
    updatePaneClass: () => {},
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    refreshAll: () => {},
    browseFolderPicker: () => {},
    focusFilterInput: () => {},
    fileOps: {},
    panelController: {
      openSelected: (s: string) => { openedSide = s; },
    },
    listRenderer: {},
    getFilteredSelection: () => ({ item: null }),
    renderPane: () => {},
    moveCursor: (s: string, d: number) => {
      movedSide = s;
      movedDelta = d;
    },
  });

  // Simulate ArrowDown
  const eventDown = {
    key: 'ArrowDown',
    target: mockListRight,
    preventDefault: () => {},
  } as any;
  (controller as any)._onKeydown(eventDown);

  assert.equal(movedSide, 'right', 'ArrowDown should move cursor on the right panel');
  assert.equal(movedDelta, 1);
  assert.equal(state.active, 'right');

  // Simulate ArrowUp
  const eventUp = {
    key: 'ArrowUp',
    target: mockListRight,
    preventDefault: () => {},
  } as any;
  (controller as any)._onKeydown(eventUp);

  assert.equal(movedSide, 'right', 'ArrowUp should move cursor on the right panel');
  assert.equal(movedDelta, -1);

  // Simulate Enter
  const eventEnter = {
    key: 'Enter',
    target: mockListRight,
    preventDefault: () => {},
  } as any;
  (controller as any)._onKeydown(eventEnter);

  assert.equal(openedSide, 'right', 'Enter should open selected on the right panel');
});

test('KeyboardController Tab key switches panel', () => {
  const state = new AppState();
  state.active = 'left';

  let focused = false;

  const controller = new KeyboardController({
    state,
    pathHeaderController: { isEditing: () => false },
    tabsRenderer: { handleGlobalKeydown: () => false },
    quickViewController: { toggle: () => {} },
    commandsController: { runCommand: () => {}, anyMenuOpen: () => false, closeAllMenus: () => {} },
    searchController: { openOverlay: () => {}, hideOverlay: () => {} },
    propertiesController: { isEditing: () => false },
    gitController: {},
    compareDirsController: {},
    focusActiveList: () => { focused = true; },
    syncFilterInput: () => {},
    updatePaneClass: () => {},
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    refreshAll: () => {},
    browseFolderPicker: () => {},
    focusFilterInput: () => {},
    fileOps: {},
    panelController: {},
    listRenderer: {},
    getFilteredSelection: () => ({ item: null }),
    renderPane: () => {},
    moveCursor: () => {},
  });

  const tabEvent = {
    key: 'Tab',
    target: { tagName: 'DIV' },
    preventDefault: () => {},
  } as any;
  (controller as any)._onKeydown(tabEvent);

  assert.equal(state.active, 'right');
  assert.equal(focused, true);
});
