// src/modules/commandsController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { CommandsController } from './commandsController.ts';
import { AppState } from './stateModels.ts';

interface MockElement {
  id: string;
  tagName: string;
  className: string;
  hidden: boolean;
  value: string;
  textContent: string;
  innerHTML: string;
  title: string;
  disabled: boolean;
  selectionStart: number;
  selectionEnd: number;
  style: Record<string, any>;
  dataset: Record<string, string>;
  attributes: Record<string, string>;
  children: MockElement[];
  classList: {
    contains: (c: string) => boolean;
    add: (...c: string[]) => void;
    remove: (...c: string[]) => void;
    toggle: (c: string, force?: boolean) => boolean;
  };
  getAttribute: (k: string) => string | null;
  setAttribute: (k: string, v: string) => void;
  removeAttribute: (k: string) => void;
  focus: () => void;
  replaceChildren: (...nodes: MockElement[]) => void;
  append: (...nodes: MockElement[]) => void;
  appendChild: (child: MockElement) => void;
  addEventListener: (event: string, handler: (e: any) => void) => void;
  removeEventListener: (event: string, handler: (e: any) => void) => void;
  click: (eventPayload?: any) => void;
  closest: (selector: string) => MockElement | null;
  querySelector: (selector: string) => MockElement | null;
  querySelectorAll: (selector: string) => MockElement[];
  onclick: ((e?: any) => void) | null;
  onkeydown: ((e?: any) => void) | null;
  onerror: ((e?: any) => void) | null;
  onload: ((e?: any) => void) | null;
}

function createMockElement(id = '', tagName = 'div', dataset: Record<string, string> = {}): MockElement {
  let _className = '';
  const classes = new Set<string>();
  const listeners: Record<string, ((e: any) => void)[]> = {};
  const attrs: Record<string, string> = {};
  const children: MockElement[] = [];

  const el: MockElement = {
    id,
    tagName: tagName.toUpperCase(),
    get className() { return _className; },
    set className(val: string) {
      _className = val;
      classes.clear();
      val.split(/\s+/).filter(Boolean).forEach((c) => classes.add(c));
    },
    hidden: false,
    value: '',
    textContent: '',
    innerHTML: '',
    title: '',
    disabled: false,
    selectionStart: 0,
    selectionEnd: 0,
    style: {},
    dataset: { ...dataset },
    attributes: attrs,
    children,
    classList: {
      contains: (c: string) => classes.has(c),
      add: (...items: string[]) => {
        items.forEach((c) => classes.add(c));
        _className = Array.from(classes).join(' ');
      },
      remove: (...items: string[]) => {
        items.forEach((c) => classes.delete(c));
        _className = Array.from(classes).join(' ');
      },
      toggle: (c: string, force?: boolean) => {
        const has = classes.has(c);
        const shouldHave = force !== undefined ? force : !has;
        if (shouldHave) classes.add(c);
        else classes.delete(c);
        _className = Array.from(classes).join(' ');
        return shouldHave;
      },
    },
    getAttribute: (k: string) => attrs[k] ?? null,
    setAttribute: (k: string, v: string) => { attrs[k] = v; },
    removeAttribute: (k: string) => { delete attrs[k]; },
    focus: () => {},
    replaceChildren: (...nodes: MockElement[]) => {
      children.length = 0;
      if (nodes && nodes.length) children.push(...nodes);
    },
    append: (...nodes: MockElement[]) => {
      children.push(...nodes);
    },
    appendChild: (child: MockElement) => {
      children.push(child);
    },
    addEventListener: (event: string, handler: (e: any) => void) => {
      if (!listeners[event]) listeners[event] = [];
      listeners[event].push(handler);
    },
    removeEventListener: (event: string, handler: (e: any) => void) => {
      if (listeners[event]) {
        listeners[event] = listeners[event].filter((h) => h !== handler);
      }
    },
    click: (eventPayload: any = {}) => {
      const e = {
        target: el,
        preventDefault: () => {},
        stopPropagation: () => {},
        ...eventPayload,
      };
      if (el.onclick) el.onclick(e);
      (listeners['click'] || []).forEach((fn) => fn(e));
    },
    closest: (selector: string): MockElement | null => {
      if (selector.startsWith('.')) {
        const cls = selector.slice(1);
        if (classes.has(cls)) return el;
      }
      if (selector.startsWith('#')) {
        const targetId = selector.slice(1);
        if (el.id === targetId) return el;
      }
      if (selector === '[data-cmd]' && el.dataset.cmd) return el;
      if (selector.startsWith('a[data-md-href]') && attrs['data-md-href']) return el;
      return null;
    },
    querySelector: (selector: string): MockElement | null => {
      if (selector.startsWith('.')) {
        const cls = selector.slice(1);
        for (const c of children) {
          if (c.classList.contains(cls)) return c;
          const found = c.querySelector(selector);
          if (found) return found;
        }
      }
      if (selector.startsWith('#')) {
        const targetId = selector.slice(1);
        for (const c of children) {
          if (c.id === targetId) return c;
          const found = c.querySelector(selector);
          if (found) return found;
        }
      }
      return null;
    },
    querySelectorAll: (selector: string): MockElement[] => {
      const results: MockElement[] = [];
      if (selector.startsWith('.')) {
        const cls = selector.slice(1);
        for (const c of children) {
          if (c.classList.contains(cls)) results.push(c);
          results.push(...c.querySelectorAll(selector));
        }
      }
      return results;
    },
    onclick: null,
    onkeydown: null,
    onerror: null,
    onload: null,
  };

  return el;
}

function setupDocMock(elements: Record<string, MockElement>, querySelectorAllMap: Record<string, MockElement[]> = {}) {
  const origDoc = globalThis.document;
  const docListeners: Record<string, ((e: any) => void)[]> = {};

  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
    querySelector: (sel: string) => {
      if (sel.startsWith('#')) {
        const id = sel.slice(1);
        return elements[id] || null;
      }
      return null;
    },
    querySelectorAll: (selector: string) => querySelectorAllMap[selector] || [],
    createElement: (tag: string) => createMockElement('', tag),
    addEventListener: (ev: string, fn: (e: any) => void) => {
      if (!docListeners[ev]) docListeners[ev] = [];
      docListeners[ev].push(fn);
    },
    removeEventListener: (ev: string, fn: (e: any) => void) => {
      if (docListeners[ev]) {
        docListeners[ev] = docListeners[ev].filter((h) => h !== fn);
      }
    },
  };

  const dispatchDocEvent = (type: string, payload: any = {}) => {
    const e = {
      preventDefault: () => {},
      stopPropagation: () => {},
      ...payload,
    };
    (docListeners[type] || []).forEach((fn) => fn(e));
  };

  const cleanup = () => {
    globalThis.document = origDoc;
  };

  return { cleanup, dispatchDocEvent };
}

test('CommandsController openViewer shows Edit button for text files and allows saving', async () => {
  const state = new AppState();
  const mockOverlay = createMockElement('viewer-overlay');
  mockOverlay.classList.add('hidden');
  const mockContent = createMockElement('viewer-content');
  const mockEditor = createMockElement('viewer-editor', 'textarea');
  mockEditor.classList.add('hidden');
  const mockTitle = createMockElement('viewer-title');
  const mockClose = createMockElement('viewer-close');
  const mockMode = createMockElement('viewer-mode-btn');
  mockMode.classList.add('hidden');
  const mockEdit = createMockElement('viewer-edit-btn');
  mockEdit.classList.add('hidden');
  const mockSave = createMockElement('viewer-save-btn');
  mockSave.classList.add('hidden');
  const mockCancel = createMockElement('viewer-cancel-edit-btn');
  mockCancel.classList.add('hidden');
  const mockStatus = createMockElement('viewer-status-hint');

  const elements: Record<string, MockElement> = {
    'viewer-overlay': mockOverlay,
    'viewer-content': mockContent,
    'viewer-editor': mockEditor,
    'viewer-title': mockTitle,
    'viewer-close': mockClose,
    'viewer-mode-btn': mockMode,
    'viewer-edit-btn': mockEdit,
    'viewer-save-btn': mockSave,
    'viewer-cancel-edit-btn': mockCancel,
    'viewer-status-hint': mockStatus,
  };

  const { cleanup } = setupDocMock(elements);

  let savedPath = '';
  let savedContent = '';

  const mockApi = {
    readFileText: async (_p: string) => '{\n  "name": "Oswin"\n}',
    writeFileText: async (p: string, content: string) => {
      savedPath = p;
      savedContent = content;
      return { ok: true };
    },
    probeText: async () => ({ isText: true }),
    openPath: async () => {},
    assetUrl: () => '',
  };

  const controller = new CommandsController({
    api: () => mockApi,
    state,
    setStatus: () => {},
    focusActiveList: () => {},
    refreshAll: () => {},
  });

  try {
    // Open JSON file
    await controller.openViewer('/path/to/config.json', { base: 'config.json', isDir: false });

    assert.equal(mockOverlay.classList.contains('hidden'), false, 'Overlay should be visible');
    assert.equal(mockEdit.classList.contains('hidden'), false, 'Edit button should be visible for text/json');

    // Trigger Edit
    mockEdit.onclick!();
    assert.equal(mockContent.classList.contains('hidden'), true, 'Content should be hidden in edit mode');
    assert.equal(mockEditor.classList.contains('hidden'), false, 'Editor textarea should be visible');
    assert.equal(mockEditor.value, '{\n  "name": "Oswin"\n}');

    // Modify content and save
    mockEditor.value = '{\n  "name": "Oswin Updated"\n}';
    await mockSave.onclick!();

    assert.equal(savedPath, '/path/to/config.json');
    assert.equal(savedContent, '{\n  "name": "Oswin Updated"\n}');
    assert.equal(mockEditor.classList.contains('hidden'), true, 'Editor should be hidden after save');
    assert.equal(mockContent.classList.contains('hidden'), false, 'Content should be visible after save');
  } finally {
    cleanup();
  }
});

test('CommandsController openViewer hides Edit button for image files', async () => {
  const state = new AppState();
  const mockOverlay = createMockElement('viewer-overlay');
  mockOverlay.classList.add('hidden');
  const mockContent = createMockElement('viewer-content');
  const mockEditor = createMockElement('viewer-editor', 'textarea');
  const mockTitle = createMockElement('viewer-title');
  const mockClose = createMockElement('viewer-close');
  const mockMode = createMockElement('viewer-mode-btn');
  const mockEdit = createMockElement('viewer-edit-btn');
  mockEdit.classList.add('hidden');
  const mockSave = createMockElement('viewer-save-btn');
  const mockCancel = createMockElement('viewer-cancel-edit-btn');
  const mockStatus = createMockElement('viewer-status-hint');

  const elements: Record<string, MockElement> = {
    'viewer-overlay': mockOverlay,
    'viewer-content': mockContent,
    'viewer-editor': mockEditor,
    'viewer-title': mockTitle,
    'viewer-close': mockClose,
    'viewer-mode-btn': mockMode,
    'viewer-edit-btn': mockEdit,
    'viewer-save-btn': mockSave,
    'viewer-cancel-edit-btn': mockCancel,
    'viewer-status-hint': mockStatus,
  };

  const { cleanup } = setupDocMock(elements);

  const mockApi = {
    readFileText: async () => '',
    writeFileText: async () => {},
    probeText: async () => ({ isText: false }),
    openPath: async () => {},
    assetUrl: () => 'asset://photo.png',
  };

  const controller = new CommandsController({
    api: () => mockApi,
    state,
    setStatus: () => {},
    focusActiveList: () => {},
    refreshAll: () => {},
  });

  try {
    // Open PNG file
    await controller.openViewer('/path/to/photo.png', { base: 'photo.png', isDir: false });

    assert.equal(mockOverlay.classList.contains('hidden'), false, 'Overlay should be visible');
    assert.equal(mockEdit.classList.contains('hidden'), true, 'Edit button MUST be hidden for image files');
  } finally {
    cleanup();
  }
});

test('CommandsController duplicate clones in place instead of copying to the other pane', () => {
  const state = new AppState();
  let cloned = 0;
  let copiedToOther = 0;
  const { cleanup } = setupDocMock({});
  try {
    const controller = new CommandsController({
      api: () => ({}),
      state,
      setStatus: () => {},
      focusActiveList: () => {},
      copyToOther: () => { copiedToOther++; },
      fileOps: { cloneSelection: () => { cloned++; } },
    });

    controller.runCommand('duplicate');
    controller.runCommand('copy');

    assert.equal(cloned, 1);
    assert.equal(copiedToOther, 1);
  } finally {
    cleanup();
  }
});

test('CommandsController rename and delete dispatch to file ops', () => {
  const state = new AppState();
  let renamed = 0;
  let deleted = 0;
  const { cleanup } = setupDocMock({});
  try {
    const controller = new CommandsController({
      api: () => ({}),
      state,
      setStatus: () => {},
      focusActiveList: () => {},
      beginRename: () => { renamed++; },
      beginDelete: () => { deleted++; },
    });

    controller.runCommand('rename');
    controller.runCommand('delete');

    assert.equal(renamed, 1);
    assert.equal(deleted, 1);
  } finally {
    cleanup();
  }
});

test('CommandsController menu and dropdown management (closeAllMenus, anyMenuOpen, hideCtxMenu)', () => {
  const drop1 = createMockElement('drop-file');
  drop1.hidden = false;
  const drop2 = createMockElement('drop-edit');
  drop2.hidden = true;

  const top1 = createMockElement('top-file');
  top1.setAttribute('aria-expanded', 'true');
  const top2 = createMockElement('top-edit');
  top2.setAttribute('aria-expanded', 'false');

  const ctxMenu = createMockElement('ctx-menu');

  const { cleanup } = setupDocMock(
    { 'ctx-menu': ctxMenu },
    {
      '.menu-drop': [drop1, drop2],
      '.menu-top': [top1, top2],
    }
  );

  const controller = new CommandsController({
    api: () => ({}),
    state: new AppState(),
    setStatus: () => {},
    focusActiveList: () => {},
  });

  try {
    assert.equal(controller.anyMenuOpen(), true);

    controller.closeAllMenus();
    assert.equal(drop1.hidden, true);
    assert.equal(top1.getAttribute('aria-expanded'), 'false');
    assert.equal(controller.anyMenuOpen(), false);

    controller.hideCtxMenu();
    assert.equal(ctxMenu.classList.contains('hidden'), true);
  } finally {
    cleanup();
  }
});

test('CommandsController runCommand dispatches general and selection commands', () => {
  let refreshed = 0;
  let pathCopied = 0;
  let quitCalled = 0;
  let moved = 0;
  let xferCancelled = 0;
  let filterFocused = 0;
  let filterToggled = 0;

  let maskDialogArgs: any = null;
  let invertedSide = '';
  let selectedAllSide = '';
  let clearedAllSide = '';
  let extSide = '';

  const mockApi = {
    closeWindow: async () => { quitCalled++; },
    cancelCopy: () => { xferCancelled++; },
  };

  const selectionController = {
    selectByMaskDialog: (side: string, isSelect: boolean) => { maskDialogArgs = { side, isSelect }; },
    invert: (side: string) => { invertedSide = side; },
    selectAll: (side: string) => { selectedAllSide = side; },
    clearAll: (side: string) => { clearedAllSide = side; },
    selectByExtension: (side: string) => { extSide = side; },
  };

  const state = new AppState();
  state.active = 'left';

  const { cleanup } = setupDocMock({});
  const controller = new CommandsController({
    api: () => mockApi,
    state,
    setStatus: () => {},
    focusActiveList: () => {},
    refreshAll: () => { refreshed++; },
    copyPathOnly: () => { pathCopied++; },
    moveToOther: () => { moved++; },
    focusFilterInput: () => { filterFocused++; },
    toggleFilterMode: () => { filterToggled++; },
    selectionController,
  });

  try {
    // 1. General commands
    controller.runCommand('refresh');
    assert.equal(refreshed, 1);

    controller.runCommand('copyPath');
    assert.equal(pathCopied, 1);

    controller.runCommand('quit');
    assert.equal(quitCalled, 1);

    controller.runCommand('move');
    assert.equal(moved, 1);

    controller.runCommand('cancelXfer');
    assert.equal(xferCancelled, 1);

    controller.runCommand('focusFilter');
    assert.equal(filterFocused, 1);

    controller.runCommand('toggleFilter');
    assert.equal(filterToggled, 1);

    // 2. Selection commands
    controller.runCommand('selectByPattern');
    assert.deepEqual(maskDialogArgs, { side: 'left', isSelect: true });

    controller.runCommand('deselectByPattern');
    assert.deepEqual(maskDialogArgs, { side: 'left', isSelect: false });

    controller.runCommand('invertSelection');
    assert.equal(invertedSide, 'left');

    controller.runCommand('selectAll');
    assert.equal(selectedAllSide, 'left');

    controller.runCommand('clearSelection');
    assert.equal(clearedAllSide, 'left');

    controller.runCommand('selectByExtension');
    assert.equal(extSide, 'left');

    // 3. Debounce lock check (synchronous consecutive invocation)
    controller.runCommand('refresh');
    controller.runCommand('refresh');
    // refreshed should only increment by 1 due to lock
    assert.equal(refreshed, 2);
  } finally {
    cleanup();
  }
});

test('CommandsController runCommand: view/preview dispatching for paths, columns and list selection', async () => {
  let openedViewerPath = '';
  let openedViewerItem: any = null;

  const appEl = createMockElement('app');
  const elements = { app: appEl };
  const { cleanup } = setupDocMock(elements);

  const state = new AppState();
  state.active = 'left';

  const columnsViewController = {
    getColumns: (_side: string) => [
      {
        path: '/col/dir',
        selectedItem: { base: 'doc.txt', isDir: false },
      },
    ],
    getActiveColumnIndex: (_side: string) => 0,
    joinPath: async (dir: string, base: string) => `${dir}/${base}`,
  };

  const controller = new CommandsController({
    api: () => ({}),
    state,
    setStatus: () => {},
    focusActiveList: () => {},
    columnsViewController,
    getFilteredSelection: (_side: string) => ({
      item: { base: 'list_file.txt', isDir: false },
    }),
    fullPath: async (_pane: any, item: any) => `/full/${item.base}`,
  });

  // Spy on openViewer
  controller.openViewer = async (fp: string, item?: any) => {
    openedViewerPath = fp;
    openedViewerItem = item;
  };

  try {
    // 1. Direct string payload
    controller.runCommand('view', '/direct/path.md');
    assert.equal(openedViewerPath, '/direct/path.md');

    // 2. Object payload with fp & item
    controller.runCommand('preview', { fp: '/object/image.png', item: { base: 'image.png' } });
    assert.equal(openedViewerPath, '/object/image.png');
    assert.equal(openedViewerItem.base, 'image.png');

    // 3. Columns mode
    appEl.classList.add('columns-mode');
    controller.runCommand('viewFile');
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(openedViewerPath, '/col/dir/doc.txt');

    // 4. Standard list mode
    appEl.classList.remove('columns-mode');
    controller.runCommand('view');
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(openedViewerPath, '/full/list_file.txt');
  } finally {
    cleanup();
  }
});

test('CommandsController newFile dialog interactions (ok, cancel, escape, enter)', async () => {
  const overlay = createMockElement('modal-overlay');
  overlay.classList.add('hidden');
  const title = createMockElement('rename-dialog-title');
  const input = createMockElement('rename-input', 'input');
  const okBtn = createMockElement('rename-ok', 'button');
  const cancelBtn = createMockElement('rename-cancel', 'button');
  const iconEl = createMockElement('rename-dialog-icon');

  const elements = {
    'modal-overlay': overlay,
    'rename-dialog-title': title,
    'rename-input': input,
    'rename-ok': okBtn,
    'rename-cancel': cancelBtn,
    'rename-dialog-icon': iconEl,
  };

  const { cleanup } = setupDocMock(elements);
  const state = new AppState();
  state.active = 'left';
  state.left.path = '/target/folder';

  let createdFile = '';
  let refreshed = 0;
  let statusMsg = '';
  let focused = false;

  let mockApi: any = {
    pathJoin: async (dir: string, file: string) => `${dir}/${file}`,
    createFile: async (fp: string) => {
      createdFile = fp;
      return { ok: true };
    },
  };

  const controller = new CommandsController({
    api: () => mockApi,
    state,
    setStatus: (m) => { statusMsg = m; },
    focusActiveList: () => { focused = true; },
    refreshAll: () => { refreshed++; },
  });

  try {
    // 1. Open newFile dialog and cancel
    controller.runCommand('newFile');
    assert.equal(overlay.classList.contains('hidden'), false);
    assert.equal(title.textContent, 'Create New File');
    assert.equal(okBtn.textContent, 'Create File');

    cancelBtn.click();
    assert.equal(overlay.classList.contains('hidden'), true);
    assert.equal(focused, true);

    // 2. Open and press Escape
    controller.runCommand('newFile');
    input.onkeydown!({ key: 'Escape', preventDefault: () => {} });
    assert.equal(overlay.classList.contains('hidden'), true);

    // 3. Open and submit empty -> closes without creating
    controller.runCommand('newFile');
    input.value = '   ';
    input.onkeydown!({ key: 'Enter', preventDefault: () => {} });
    assert.equal(createdFile, '');

    // 4. Open and submit with filename via Enter
    controller.runCommand('newFile');
    input.value = 'notes.txt';
    input.onkeydown!({ key: 'Enter', preventDefault: () => {} });
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(createdFile, '/target/folder/notes.txt');
    assert.equal(statusMsg, 'Created: notes.txt');
    assert.equal(refreshed, 1);

    // 5. Failure response
    mockApi.createFile = async () => ({ ok: false, error: 'File already exists' });
    controller.runCommand('newFile');
    input.value = 'notes.txt';
    await okBtn.onclick!();
    assert.equal(statusMsg, 'File already exists');

    // 6. Exception handling
    mockApi.createFile = async () => { throw new Error('Disk full'); };
    controller.runCommand('newFile');
    input.value = 'notes2.txt';
    await okBtn.onclick!();
    assert.equal(statusMsg, 'Disk full');
  } finally {
    cleanup();
  }
});

test('CommandsController mkdir dialog interactions', async () => {
  const overlay = createMockElement('modal-overlay');
  overlay.classList.add('hidden');
  const title = createMockElement('rename-dialog-title');
  const input = createMockElement('rename-input', 'input');
  const okBtn = createMockElement('rename-ok', 'button');
  const cancelBtn = createMockElement('rename-cancel', 'button');
  const iconEl = createMockElement('rename-dialog-icon');

  const elements = {
    'modal-overlay': overlay,
    'rename-dialog-title': title,
    'rename-input': input,
    'rename-ok': okBtn,
    'rename-cancel': cancelBtn,
    'rename-dialog-icon': iconEl,
  };

  const { cleanup } = setupDocMock(elements);
  const state = new AppState();
  state.active = 'left';
  state.left.path = '/target/folder';

  let createdDir = '';
  let statusMsg = '';
  let refreshed = 0;

  let mockApi: any = {
    pathJoin: async (dir: string, folder: string) => `${dir}/${folder}`,
    mkdir: async (fp: string) => {
      createdDir = fp;
      return { ok: true };
    },
  };

  const controller = new CommandsController({
    api: () => mockApi,
    state,
    setStatus: (m) => { statusMsg = m; },
    focusActiveList: () => {},
    refreshAll: () => { refreshed++; },
  });

  try {
    // 1. Open mkdir dialog
    controller.runCommand('mkdir');
    assert.equal(overlay.classList.contains('hidden'), false);
    assert.equal(title.textContent, 'Create New Folder');
    assert.equal(okBtn.textContent, 'Create Folder');

    // 2. Submit with Enter
    input.value = 'new_dir';
    input.onkeydown!({ key: 'Enter', preventDefault: () => {} });
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(createdDir, '/target/folder/new_dir');
    assert.equal(statusMsg, 'Created: new_dir');
    assert.equal(refreshed, 1);

    // 3. Failure
    mockApi.mkdir = async () => ({ ok: false, error: 'Permission denied' });
    controller.runCommand('mkdir');
    input.value = 'forbidden';
    await okBtn.onclick!();
    assert.equal(statusMsg, 'Permission denied');

    // 4. Exception
    mockApi.mkdir = async () => { throw new Error('OS Error'); };
    controller.runCommand('mkdir');
    input.value = 'err_dir';
    await okBtn.onclick!();
    assert.equal(statusMsg, 'OS Error');
  } finally {
    cleanup();
  }
});

test('CommandsController openBookmarksOverlay (add, remove, navigate, and persistence)', async () => {
  const overlay = createMockElement('bookmarks-overlay');
  overlay.classList.add('hidden');
  const list = createMockElement('bookmarks-list');
  const btnAdd = createMockElement('bookmarks-add');
  const btnClose = createMockElement('bookmarks-close');
  const filterInput = createMockElement('filter-input', 'input');

  const elements = {
    'bookmarks-overlay': overlay,
    'bookmarks-list': list,
    'bookmarks-add': btnAdd,
    'bookmarks-close': btnClose,
    'filter-input': filterInput,
  };

  const { cleanup } = setupDocMock(elements);

  const storageMap: Record<string, string> = {};
  const origLocalStorage = globalThis.localStorage;
  (globalThis as any).localStorage = {
    getItem: (k: string) => storageMap[k] || null,
    setItem: (k: string, v: string) => { storageMap[k] = v; },
    removeItem: (k: string) => { delete storageMap[k]; },
  };

  const state = new AppState();
  state.active = 'left';
  state.left.path = '/home/user/docs';

  let loadedDir = '';
  let pinnedFolder = '';
  let unpinnedFolder = '';

  const controller = new CommandsController({
    api: () => ({}),
    state,
    setStatus: () => {},
    focusActiveList: () => {},
    loadDir: async (side) => { loadedDir = state[side].path; },
    sidebarController: {
      pinFolder: (p: string) => { pinnedFolder = p; },
      unpinFolder: (p: string) => { unpinnedFolder = p; },
    },
  });

  try {
    // 1. Open with empty bookmarks
    controller.openBookmarksOverlay();
    assert.equal(overlay.classList.contains('hidden'), false);
    assert.ok(list.innerHTML.includes('No bookmarks yet.'));

    // 2. Click Add button
    btnAdd.click();
    assert.equal(pinnedFolder, '/home/user/docs');
    assert.ok(storageMap['Oryn-bookmarks'].includes('/home/user/docs'));
    assert.equal(list.children.length, 1);

    // 3. Click bookmark item to navigate
    const itemEl = list.children[0];
    const pathSpan = itemEl.children[0];
    await pathSpan.onclick!();
    assert.equal(loadedDir, '/home/user/docs');
    assert.equal(overlay.classList.contains('hidden'), true);

    // 4. Reopen and delete bookmark
    controller.openBookmarksOverlay();
    const delSpan = list.children[0].children[1];
    delSpan.click({ stopPropagation: () => {} });
    assert.equal(unpinnedFolder, '/home/user/docs');
    assert.ok(list.innerHTML.includes('No bookmarks yet.'));

    // 5. Escape on list closes
    list.onkeydown!({ key: 'Escape', preventDefault: () => {} });
    assert.equal(overlay.classList.contains('hidden'), true);
  } finally {
    globalThis.localStorage = origLocalStorage;
    cleanup();
  }
});

test('CommandsController setup() wires menubar and document clicks', () => {
  const menubar = createMockElement('menubar');
  const menuTopFile = createMockElement('btn-menu-file', 'button', { menu: 'file' });
  menuTopFile.className = 'menu-top';
  const dropFile = createMockElement('drop-file');
  dropFile.hidden = true;

  const menuItemRefresh = createMockElement('item-refresh', 'div', { cmd: 'refresh' });
  menuItemRefresh.className = 'menu-item';

  menubar.children.push(menuTopFile, menuItemRefresh);

  const otherCmdBtn = createMockElement('other-cmd-btn', 'button', { cmd: 'duplicate' });

  let ranCmd = '';

  const { cleanup, dispatchDocEvent } = setupDocMock(
    {
      menubar,
      'drop-file': dropFile,
    },
    {
      '.menu-top': [menuTopFile],
      '.menu-item': [menuItemRefresh],
    }
  );

  const controller = new CommandsController({
    api: () => ({}),
    state: new AppState(),
    setStatus: () => {},
    focusActiveList: () => {},
  });

  controller.runCommand = (cmd: string) => { ranCmd = cmd; };

  try {
    controller.setup();

    // 1. Click menu-top to expand dropdown
    menuTopFile.click();
    assert.equal(dropFile.hidden, false);
    assert.equal(menuTopFile.getAttribute('aria-expanded'), 'true');

    // 2. Click menu-item to run command
    menuItemRefresh.click();
    assert.equal(ranCmd, 'refresh');

    // 3. Document click on [data-cmd] button outside menubar
    dispatchDocEvent('click', { target: otherCmdBtn });
    assert.equal(ranCmd, 'duplicate');
  } finally {
    cleanup();
  }
});

test('CommandsController openViewer formats: markdown, media, office, binary notices, and keyboard shortcuts', async () => {
  const overlay = createMockElement('viewer-overlay');
  overlay.classList.add('hidden');
  const content = createMockElement('viewer-content');
  const editor = createMockElement('viewer-editor', 'textarea');
  editor.classList.add('hidden');
  const title = createMockElement('viewer-title');
  const btnClose = createMockElement('viewer-close');
  const btnMode = createMockElement('viewer-mode-btn');
  const btnEdit = createMockElement('viewer-edit-btn');
  const btnSave = createMockElement('viewer-save-btn');
  const btnCancel = createMockElement('viewer-cancel-edit-btn');
  const statusHint = createMockElement('viewer-status-hint');

  const elements: Record<string, MockElement> = {
    'viewer-overlay': overlay,
    'viewer-content': content,
    'viewer-editor': editor,
    'viewer-title': title,
    'viewer-close': btnClose,
    'viewer-mode-btn': btnMode,
    'viewer-edit-btn': btnEdit,
    'viewer-save-btn': btnSave,
    'viewer-cancel-edit-btn': btnCancel,
    'viewer-status-hint': statusHint,
  };

  const { cleanup, dispatchDocEvent } = setupDocMock(elements);
  let statusMsg = '';
  let openedPath = '';

  const mockApi = {
    readFileText: async (_fp: string, _max?: number) => '# Header\n[Link](https://oryn.app)',
    writeFileText: async () => {},
    probeText: async () => ({ isText: true }),
    openPath: async (p: string) => { openedPath = p; },
    assetUrl: (p: string) => `asset://${p}`,
    readOffice: async () => ({ type: 'docx', paragraphs: ['Hello Word'] }),
  };

  const controller = new CommandsController({
    api: () => mockApi,
    state: new AppState(),
    setStatus: (m) => { statusMsg = m; },
    focusActiveList: () => {},
  });

  try {
    // 1. Attempt to view directory
    await controller.openViewer('/folder', { base: 'folder', isDir: true });
    assert.equal(statusMsg, 'F3 View: Cannot view a folder. Enter to navigate.');

    // 2. Empty or .. filename
    await controller.openViewer('', { base: '..' });
    assert.equal(overlay.classList.contains('hidden'), true);

    // 3. Markdown file
    await controller.openViewer('/path/README.md', { base: 'README.md', isDir: false });
    assert.equal(overlay.classList.contains('hidden'), false);
    assert.equal(content.classList.contains('viewer-content--md'), true);
    assert.equal(btnMode.textContent, 'Raw');

    // Toggle mode to Raw and back
    btnMode.click();
    assert.equal(btnMode.textContent, 'Rendered');
    btnMode.click();
    assert.equal(btnMode.textContent, 'Raw');

    // Clicking markdown link inside content
    const linkEl = createMockElement('link-1', 'a');
    linkEl.setAttribute('data-md-href', 'https://oryn.app');
    content.click({ target: linkEl });
    assert.equal(openedPath, 'https://oryn.app');

    // 4. Audio file (.mp3)
    await controller.openViewer('/path/song.mp3', { base: 'song.mp3', isDir: false });
    assert.equal(content.children.length, 2);
    assert.equal(content.children[0].tagName, 'AUDIO');

    // 5. Video file (.mp4)
    await controller.openViewer('/path/clip.mp4', { base: 'clip.mp4', isDir: false });
    assert.equal(content.children[0].tagName, 'VIDEO');

    // 6. Modern Office (.docx)
    await controller.openViewer('/path/doc.docx', { base: 'doc.docx', isDir: false });
    assert.equal(content.classList.contains('viewer-content--office'), true);

    // 7. Legacy Office (.doc)
    await controller.openViewer('/path/old.doc', { base: 'old.doc', isDir: false });
    assert.ok(content.textContent.includes('pre-2007 binary Office format'));

    // 8. Non-text extension (.zip)
    await controller.openViewer('/path/archive.zip', { base: 'archive.zip', isDir: false });
    assert.ok(content.textContent.includes("Can't preview archive.zip as text"));

    // 9. Keyboard shortcuts: Close viewer with Escape
    dispatchDocEvent('keydown', { key: 'Escape' });
    assert.equal(overlay.classList.contains('hidden'), true);
  } finally {
    cleanup();
  }
});

test('CommandsController openViewer remote files, editor shortcuts, and write failure', async () => {
  const overlay = createMockElement('viewer-overlay');
  const content = createMockElement('viewer-content');
  const editor = createMockElement('viewer-editor', 'textarea');
  const title = createMockElement('viewer-title');
  const btnClose = createMockElement('viewer-close');
  const btnMode = createMockElement('viewer-mode-btn');
  const btnEdit = createMockElement('viewer-edit-btn');
  const btnSave = createMockElement('viewer-save-btn');
  const btnCancel = createMockElement('viewer-cancel-edit-btn');
  const statusHint = createMockElement('viewer-status-hint');

  const elements: Record<string, MockElement> = {
    'viewer-overlay': overlay,
    'viewer-content': content,
    'viewer-editor': editor,
    'viewer-title': title,
    'viewer-close': btnClose,
    'viewer-mode-btn': btnMode,
    'viewer-edit-btn': btnEdit,
    'viewer-save-btn': btnSave,
    'viewer-cancel-edit-btn': btnCancel,
    'viewer-status-hint': statusHint,
  };

  const { cleanup, dispatchDocEvent } = setupDocMock(elements);

  let remoteSavedProfile = '';
  let remoteSavedPath = '';
  let remoteSavedContent = '';

  const mockApi = {
    remoteReadFileText: async (_prof: string, _path: string) => 'remote log content',
    remoteWriteFileText: async (prof: string, path: string, contentStr: string) => {
      if (contentStr.includes('FAIL')) throw new Error('Remote write failed');
      remoteSavedProfile = prof;
      remoteSavedPath = path;
      remoteSavedContent = contentStr;
    },
  };

  const controller = new CommandsController({
    api: () => mockApi,
    state: new AppState(),
    setStatus: () => {},
    focusActiveList: () => {},
  });

  try {
    // 1. Open remote SFTP file
    await controller.openViewer('sftp://myserver/var/log/sys.log', { base: 'sys.log', isDir: false });
    assert.equal(btnEdit.classList.contains('hidden'), false);

    // 2. Start edit via Ctrl+E keyboard shortcut
    dispatchDocEvent('keydown', { ctrlKey: true, key: 'e' });
    assert.equal(editor.classList.contains('hidden'), false);
    assert.equal(editor.value, 'remote log content');

    // 3. Tab key indentation in editor
    editor.selectionStart = 6;
    editor.selectionEnd = 6;
    editor.onkeydown!({ key: 'Tab', preventDefault: () => {} });
    assert.equal(editor.value, 'remote   log content');
    assert.equal(editor.selectionStart, 8);

    // 4. Save with Ctrl+S
    dispatchDocEvent('keydown', { ctrlKey: true, key: 's' });
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(remoteSavedProfile, 'myserver');
    assert.equal(remoteSavedPath, '/var/log/sys.log');
    assert.equal(remoteSavedContent, 'remote   log content');
    assert.equal(editor.classList.contains('hidden'), true);

    // 5. Edit again and handle save failure
    btnEdit.click();
    editor.value = 'FAIL THIS';
    await btnSave.onclick!();
    assert.ok(statusHint.textContent.includes('Save failed: Remote write failed'));
    assert.equal(btnSave.disabled, false);

    // 6. Cancel edit with Escape
    dispatchDocEvent('keydown', { key: 'Escape' });
    assert.equal(editor.classList.contains('hidden'), true);
  } finally {
    cleanup();
  }
});

test('CommandsController openViewer media callbacks, binary detection, and error fallbacks', async () => {
  const overlay = createMockElement('viewer-overlay');
  const content = createMockElement('viewer-content');
  const editor = createMockElement('viewer-editor', 'textarea');
  const title = createMockElement('viewer-title');
  const btnClose = createMockElement('viewer-close');
  const btnMode = createMockElement('viewer-mode-btn');
  const btnEdit = createMockElement('viewer-edit-btn');
  const btnSave = createMockElement('viewer-save-btn');
  const btnCancel = createMockElement('viewer-cancel-edit-btn');
  const statusHint = createMockElement('viewer-status-hint');

  const elements: Record<string, MockElement> = {
    'viewer-overlay': overlay,
    'viewer-content': content,
    'viewer-editor': editor,
    'viewer-title': title,
    'viewer-close': btnClose,
    'viewer-mode-btn': btnMode,
    'viewer-edit-btn': btnEdit,
    'viewer-save-btn': btnSave,
    'viewer-cancel-edit-btn': btnCancel,
    'viewer-status-hint': statusHint,
  };

  const { cleanup, dispatchDocEvent } = setupDocMock(elements);

  const mockApi = {
    assetUrl: (p: string) => `asset://${p}`,
    readMediaDataUrl: async (_p: string) => 'data:image/png;base64,abc',
    probeText: async (p: string) => {
      if (p.includes('binary_probe')) return { isText: false };
      return { isText: true };
    },
    readFileText: async (p: string) => {
      if (p.includes('binary_text')) return 'some \x00 binary \x01 bytes';
      if (p.includes('throw_error')) throw new Error('I/O read failure');
      return 'normal text';
    },
  };

  const controller = new CommandsController({
    api: () => mockApi,
    state: new AppState(),
    setStatus: () => {},
    focusActiveList: () => {},
  });

  try {
    // 1. Image onload & onerror
    await controller.openViewer('/path/photo.png', { base: 'photo.png', size: 1024, isDir: false });
    const img = content.children[0];
    const meta = content.children[1];
    (img as any).naturalWidth = 800;
    (img as any).naturalHeight = 600;
    img.onload!();
    assert.ok(meta.textContent.includes('800 × 600 px'));

    // Image onerror with readMediaDataUrl
    await img.onerror!();
    assert.equal((img as any).src, 'data:image/png;base64,abc');

    // 2. Audio onerror fallback
    await controller.openViewer('/path/audio.mp3', { base: 'audio.mp3', size: 2048, isDir: false });
    const audio = content.children[0];
    await audio.onerror!();
    assert.equal((audio as any).src, 'data:image/png;base64,abc');

    // 3. Video onerror fallback
    await controller.openViewer('/path/video.mp4', { base: 'video.mp4', size: 4096, isDir: false });
    const video = content.children[0];
    await video.onerror!();
    assert.equal((video as any).src, 'data:image/png;base64,abc');

    // 4. probeText returns isText = false
    await controller.openViewer('/path/binary_probe.dat', { base: 'binary_probe.dat', isDir: false });
    assert.ok(content.textContent.includes("Can't preview binary_probe.dat as text"));

    // 5. looksBinaryText returns true
    await controller.openViewer('/path/binary_text.txt', { base: 'binary_text.txt', isDir: false });
    assert.ok(content.textContent.includes("Can't preview binary_text.txt as text"));

    // 6. Exception in openViewer
    await controller.openViewer('/path/throw_error.txt', { base: 'throw_error.txt', isDir: false });
    assert.ok(content.textContent.includes('Cannot preview file'));
    assert.ok(content.textContent.includes('I/O read failure'));

    // 7. Standalone openViewer without item parameter
    await controller.openViewer('/path/standalone.txt');
    assert.equal(title.textContent, '— standalone.txt');

    // 8. Image onerror when readMediaDataUrl is unavailable / fails
    mockApi.readMediaDataUrl = async () => { throw new Error('decode failed'); };
    await controller.openViewer('/path/broken.png', { base: 'broken.png', isDir: false });
    const brokenImg = content.children[0];
    await brokenImg.onerror!();
    assert.ok(content.textContent.includes('Cannot decode broken.png as an image.'));

    // 9. Keypress while editing that is not Escape or Ctrl+S
    await controller.openViewer('/path/sample.txt', { base: 'sample.txt', isDir: false });
    btnEdit.click();
    dispatchDocEvent('keydown', { key: 'a' });
    assert.equal(editor.classList.contains('hidden'), false);
    dispatchDocEvent('keydown', { key: 'Escape' });

    // 10. Default case for unknown command
    controller.runCommand('nonExistentCommand');
  } finally {
    cleanup();
  }
});


