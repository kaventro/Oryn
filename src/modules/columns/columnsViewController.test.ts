// src/modules/columns/columnsViewController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { ColumnsViewController } from './columnsViewController.ts';
import { IconRegistry } from '../icons/iconRegistry.ts';

test('ColumnsViewController initial root load and cascading hierarchy', async () => {
  const iconRegistry = new IconRegistry();
  const mockDirs: Record<string, Array<{ name: string; isDir: boolean }>> = {
    '/root': [
      { name: 'src', isDir: true },
      { name: 'README.md', isDir: false },
    ],
    '/root/src': [
      { name: 'index.js', isDir: false },
      { name: 'styles.css', isDir: false },
    ],
  };

  const api = () => ({
    readDir: async (p: string) => mockDirs[p] || [],
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
  });

  const cvc = new ColumnsViewController({
    api,
    rowRenderer: null as any,
    iconRegistry,
  });

  await cvc.loadRoot('left', { path: '/root', cursor: 0 });
  await cvc.selectItem(0, 0, 'left');

  const cols = cvc.getColumns('left');
  assert.equal(cols.length, 2, 'Selecting directory in column 0 expands it into column 1');
  assert.equal(cols[0].path, '/root');
  assert.equal(cols[0].items.length, 2);
  assert.equal(cols[1].path, '/root/src');
  assert.equal(cols[1].items.length, 2);
});

test('ColumnsViewController truncates child columns when selecting sibling file', async () => {
  const iconRegistry = new IconRegistry();
  const mockDirs: Record<string, Array<{ name: string; isDir: boolean }>> = {
    '/root': [
      { name: 'src', isDir: true },
      { name: 'README.md', isDir: false },
    ],
    '/root/src': [
      { name: 'index.js', isDir: false },
    ],
  };

  const api = () => ({
    readDir: async (p: string) => mockDirs[p] || [],
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
  });

  const cvc = new ColumnsViewController({
    api,
    rowRenderer: null as any,
    iconRegistry,
  });

  await cvc.loadRoot('left', { path: '/root', cursor: 0 });
  await cvc.selectItem(0, 0, 'left');
  assert.equal(cvc.getColumns('left').length, 2);

  // Select README.md in column 0
  await cvc.selectItem(0, 1, 'left');
  assert.equal(cvc.getColumns('left').length, 1, 'Child columns truncated when file is selected');
  assert.equal(cvc.getColumns('left')[0].selectedItem?.base, 'README.md');
});

test('ColumnsViewController horizontal and vertical keyboard navigation', async () => {
  const iconRegistry = new IconRegistry();
  const mockDirs: Record<string, Array<{ name: string; isDir: boolean }>> = {
    '/root': [
      { name: 'docs', isDir: true },
      { name: 'src', isDir: true },
    ],
    '/root/docs': [
      { name: 'guide.md', isDir: false },
    ],
    '/root/src': [
      { name: 'app.js', isDir: false },
    ],
  };

  const api = () => ({
    readDir: async (p: string) => mockDirs[p] || [],
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
  });

  const cvc = new ColumnsViewController({
    api,
    rowRenderer: null as any,
    iconRegistry,
  });

  await cvc.loadRoot('left', { path: '/root', cursor: 0 });
  await cvc.selectItem(0, 0, 'left');
  assert.equal(cvc.getActiveColumnIndex('left'), 0);

  // Navigate down in column 0 to 'src'
  await cvc.navigate(0, 1, 'left');
  assert.equal(cvc.getColumns('left')[0].selectedIndex, 1);
  assert.equal(cvc.getColumns('left')[1].path, '/root/src');

  // Step right into column 1
  await cvc.navigate(1, 0, 'left');
  assert.equal(cvc.getActiveColumnIndex('left'), 1);

  // Step left back to column 0
  await cvc.navigate(-1, 0, 'left');
  assert.equal(cvc.getActiveColumnIndex('left'), 0);
});

test('ColumnsViewController forwards context menu requests with item and column path', async () => {
  let ctxMenuArgs: any = null;
  const iconRegistry = new IconRegistry();
  const mockDirs: Record<string, Array<{ name: string; isDir: boolean }>> = {
    '/root': [{ name: 'test.js', isDir: false }],
  };
  const api = () => ({
    readDir: async (p: string) => mockDirs[p] || [],
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
  });

  const cvc = new ColumnsViewController({
    api,
    rowRenderer: null as any,
    iconRegistry,
    showCtxMenu: (x, y, side, emptyArea, item, dirPath) => {
      ctxMenuArgs = { x, y, side, emptyArea, item, dirPath };
    },
  });

  await cvc.loadRoot('left', { path: '/root', cursor: 0 });
  const cols = cvc.getColumns('left');
  assert.equal(cols.length, 1);
  assert.equal(cols[0].items[0].base, 'test.js');

  // Verify showCtxMenu callback is present and callable
  cvc.showCtxMenu?.(100, 200, 'left', false, cols[0].items[0], cols[0].path);
  assert.deepEqual(ctxMenuArgs, {
    x: 100,
    y: 200,
    side: 'left',
    emptyArea: false,
    item: cols[0].items[0],
    dirPath: '/root',
  });
});

test('ColumnsViewController copies gitStatus onto column items and keeps it when expanding', async () => {
  const mockDirs: Record<string, Array<{ name: string; isDir: boolean }>> = {
    '/git-cols-repo': [
      { name: 'src', isDir: true },
      { name: 'README.md', isDir: false },
    ],
    '/git-cols-repo/src': [
      { name: 'app.ts', isDir: false },
    ],
  };
  const api = () => ({
    readDir: async (p: string) => mockDirs[p] || [],
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
    gitIsRepo: async () => ({ ok: true, root: '/git-cols-repo' }),
    gitStatus: async () => ({
      ok: true,
      branch: 'main',
      ahead: 0,
      behind: 0,
      files: [
        { file: 'README.md', index: ' ', worktree: 'M' },
        { file: 'src/app.ts', index: 'A', worktree: ' ' },
      ],
    }),
  });

  const { clearGitStatusCache } = await import('../gitStatusMapper.ts');
  clearGitStatusCache();

  const cvc = new ColumnsViewController({ api });
  await cvc.loadRoot('left', {
    path: '/git-cols-repo',
    items: [
      { base: 'src', isDir: true },
      { base: 'README.md', isDir: false },
    ],
  });

  const rootItems = cvc.getColumns('left')[0].items;
  assert.equal(rootItems.find((i) => i.base === 'README.md')?.gitStatus, 'M');
  assert.equal(rootItems.find((i) => i.base === 'src')?.gitStatus, 'A');

  await cvc.selectItem(0, 0, 'left');
  const srcItems = cvc.getColumns('left')[1].items;
  assert.equal(srcItems.find((i) => i.base === 'app.ts')?.gitStatus, 'A');
});

test('ColumnsViewController triggers onActivateSide with active panel and path', async () => {
  let activatedSide = '';
  let activatedPath = '';
  const mockDirs: Record<string, Array<{ name: string; isDir: boolean }>> = {
    '/rightRoot': [
      { name: 'subfolder', isDir: true },
      { name: 'file.txt', isDir: false },
    ],
    '/rightRoot/subfolder': [
      { name: 'nested.txt', isDir: false },
    ],
  };

  const api = () => ({
    readDir: async (p: string) => mockDirs[p] || [],
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
  });

  const cvc = new ColumnsViewController({
    api,
    onActivateSide: (side, path) => {
      activatedSide = side;
      activatedPath = path || '';
    },
  });

  await cvc.loadRoot('right', { path: '/rightRoot', cursor: 0 });
  await cvc.selectItem(0, 0, 'right'); // Select subfolder in right pane

  assert.equal(activatedSide, 'right', 'Should activate right side');
  assert.equal(activatedPath, '/rightRoot/subfolder', 'Should update path to /rightRoot/subfolder');
});

test('ColumnsViewController goBack and goForward navigate without resetting column view', async () => {
  let activePath = '';
  const mockDirs: Record<string, Array<{ name: string; isDir: boolean }>> = {
    '/users': [
      { name: 'alice', isDir: true },
    ],
    '/users/alice': [
      { name: 'documents', isDir: true },
      { name: 'music', isDir: true },
    ],
    '/users/alice/documents': [
      { name: 'report.pdf', isDir: false },
    ],
  };

  const api = () => ({
    readDir: async (p: string) => mockDirs[p] || [],
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
    pathDirname: async (p: string) => {
      const parts = p.split('/').filter(Boolean);
      return parts.length > 1 ? `/${parts.slice(0, -1).join('/')}` : '/';
    },
  });

  const cvc = new ColumnsViewController({
    api,
    onActivateSide: (_side, p) => {
      activePath = p || '';
    },
  });

  await cvc.loadRoot('left', { path: '/users/alice', cursor: 0 });
  await cvc.selectItem(0, 0, 'left'); // select 'documents'

  const cols = cvc.getColumns('left');
  assert.equal(cols.length, 2);
  assert.equal(activePath, '/users/alice/documents');

  // Go forward into documents column
  await cvc.goForward('left');
  assert.equal(cvc.getActiveColumnIndex('left'), 1);
  assert.equal(cols[1].selectedIndex, 0); // auto-selected report.pdf

  // Go back to previous column
  await cvc.goBack('left');
  assert.equal(cvc.getActiveColumnIndex('left'), 0);
  assert.equal(cols[0].selectedIndex, 0, 'Previous folder documents remains selected');
  assert.equal(activePath, '/users/alice');

  // Go back from column 0: prepends /users
  await cvc.goBack('left');
  const prependedCols = cvc.getColumns('left');
  assert.equal(prependedCols.length, 3);
  assert.equal(prependedCols[0].path, '/users');
  assert.equal(prependedCols[0].selectedItem?.base, 'alice');
  assert.equal(cvc.getActiveColumnIndex('left'), 0);
  assert.equal(activePath, '/users');

  // syncPane with /users/alice should not destroy stack
  await cvc.syncPane('left', { path: '/users/alice' });
  assert.equal(cvc.getColumns('left').length, 3, 'syncPane keeps column stack');
  assert.equal(cvc.getActiveColumnIndex('left'), 1);
});

test('ColumnsViewController render does not inject columns-container when not in columns-mode', () => {
  let removed = false;
  const mockApp = { classList: { contains: (c: string) => c === 'list-mode' } };
  const mockContainer = { remove: () => { removed = true; } };
  const mockDoc = {
    getElementById: (id: string) => (id === 'app' ? mockApp : null),
    querySelector: (sel: string) => (sel.includes('columns-container') ? mockContainer : null),
  };
  (globalThis as any).document = mockDoc;
  try {
    const cvc = new ColumnsViewController({ api: () => ({}) });
    cvc.render('left');
    assert.equal(removed, true, 'existing container is removed and columns are not appended');
  } finally {
    delete (globalThis as any).document;
  }
});

test('ColumnsViewController sorts items by date, size, name, and kind respecting direction', () => {
  const state = {
    left: { sortField: 'date', sortAsc: false },
    right: { sortField: 'name', sortAsc: true },
  };
  const cvc = new ColumnsViewController({
    api: () => ({}),
    state,
  });

  const rawItems = [
    { base: 'old.txt', isDir: false, size: 100, mtime: '2025-01-01T10:00:00Z', ext: 'txt' },
    { base: 'new.txt', isDir: false, size: 50, mtime: '2026-09-02T12:00:00Z', ext: 'txt' },
    { base: 'folderB', isDir: true, size: 0, mtime: '2026-01-01T10:00:00Z', ext: '' },
    { base: 'folderA', isDir: true, size: 0, mtime: '2026-05-01T10:00:00Z', ext: '' },
  ];

  // Test date sort desc (newest first within dirs, then files)
  const sortedByDateDesc = cvc.sortItems(rawItems as any, 'left');
  assert.equal(sortedByDateDesc[0].base, 'folderA', 'Newer folder first');
  assert.equal(sortedByDateDesc[1].base, 'folderB', 'Older folder second');
  assert.equal(sortedByDateDesc[2].base, 'new.txt', 'Newer file first');
  assert.equal(sortedByDateDesc[3].base, 'old.txt', 'Older file second');

  // Switch to size sort asc
  state.left.sortField = 'size';
  state.left.sortAsc = true;
  const sortedBySizeAsc = cvc.sortItems(rawItems as any, 'left');
  assert.equal(sortedBySizeAsc[2].base, 'new.txt', 'Smaller file 50B first');
  assert.equal(sortedBySizeAsc[3].base, 'old.txt', 'Larger file 100B second');
});

test('ColumnsViewController sortItems edge cases (ext, date ties, size ties, parseTime fallbacks)', () => {
  const state = {
    left: { sortField: 'ext', sortAsc: true },
  };
  const cvc = new ColumnsViewController({
    api: () => ({}),
    state,
  });

  const items = [
    { base: 'b.txt', isDir: false, ext: 'txt', size: 100, mtime: 1000 },
    { base: 'a.txt', isDir: false, ext: 'txt', size: 100, mtime: 1000 },
    { base: 'c.doc', isDir: false, ext: 'doc', size: 200, mtime: 2000 },
    { base: 'z_dir', isDir: true, ext: '', size: 0, mtime: 500 },
  ];

  // 1. Sort by ext asc: folder first, then doc, then a.txt, then b.txt (tied ext resolved by name)
  const byExt = cvc.sortItems(items as any, 'left');
  assert.equal(byExt[0].base, 'z_dir');
  assert.equal(byExt[1].base, 'c.doc');
  assert.equal(byExt[2].base, 'a.txt');
  assert.equal(byExt[3].base, 'b.txt');

  // 2. Sort by size with ties (a.txt and b.txt both 100B)
  state.left.sortField = 'size';
  const bySize = cvc.sortItems(items as any, 'left');
  assert.equal(bySize[1].base, 'a.txt');
  assert.equal(bySize[2].base, 'b.txt');

  // 3. Sort by date with ties and parseTime edge cases
  state.left.sortField = 'date';
  const itemsWithVariousDates = [
    { base: 'num_time', isDir: false, mtime: 5000 },
    { base: 'str_time', isDir: false, mtime: '2026-01-01T00:00:00Z' },
    { base: 'invalid_time', isDir: false, mtime: 'not-a-valid-date' },
    { base: 'no_time', isDir: false, mtime: 0 },
    { base: 'num_time_tie', isDir: false, mtime: 5000 },
  ];
  const byDate = cvc.sortItems(itemsWithVariousDates as any, 'left');
  assert.equal(byDate.length, 5);

  // 4. Unknown sort field fallback
  state.left.sortField = 'unknown_field';
  const byUnknown = cvc.sortItems(items as any, 'left');
  assert.equal(byUnknown.length, 4);
});

test('ColumnsViewController getActiveColumn and getActiveColumnIndex', () => {
  const cvc = new ColumnsViewController({ api: () => ({}) });

  // 1. Empty stack returns null
  assert.equal(cvc.getActiveColumn('left'), null);
  assert.equal(cvc.getActiveColumnIndex('left'), 0);

  // 2. Populated stack with selected item
  cvc.paneColumns.left = [
    {
      path: '/root',
      items: [{ base: 'src', isDir: true, size: 0, mtime: 0, ext: '' }],
      selectedIndex: 0,
      selectedItem: { base: 'src', isDir: true, size: 0, mtime: 0, ext: '' },
    },
    {
      path: '/root/src',
      items: [{ base: '..', isDir: true, size: 0, mtime: 0, ext: '' }],
      selectedIndex: 0,
      selectedItem: { base: '..', isDir: true, size: 0, mtime: 0, ext: '' },
    },
  ];
  cvc.activeColIndexes.left = 1;

  // Column 1 has '..' selected, so rightmost search finds Column 0
  const active = cvc.getActiveColumn('left');
  assert.equal(active?.path, '/root');
  assert.equal(active?.colIndex, 0);

  // When neither column has a valid non-parent item selected, fallbacks to activeColIndex
  cvc.paneColumns.left[0].selectedItem = null;
  const fallbackActive = cvc.getActiveColumn('left');
  assert.equal(fallbackActive?.path, '/root/src');
  assert.equal(fallbackActive?.colIndex, 1);
});

test('ColumnsViewController syncPane edge cases (empty stack, mismatched root, and refreshes)', async () => {
  const mockDirs: Record<string, any[]> = {
    '/dirA': [{ name: 'file1.txt', isDir: false }],
    '/dirB': [{ name: 'file2.txt', isDir: false }],
  };
  const api = () => ({
    readDir: async (p: string) => mockDirs[p] || [],
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
  });

  const cvc = new ColumnsViewController({ api });

  // 1. syncPane with empty columns calls loadRoot
  await cvc.syncPane('left', { path: '/dirA', items: [] });
  assert.equal(cvc.getColumns('left').length, 1);
  assert.equal(cvc.getColumns('left')[0].path, '/dirA');

  // 2. syncPane with mismatched root path calls loadRoot
  await cvc.syncPane('left', { path: '/dirB', items: [] });
  assert.equal(cvc.getColumns('left').length, 1);
  assert.equal(cvc.getColumns('left')[0].path, '/dirB');

  // 3. syncPane where root matches and selected item is refreshed
  cvc.getColumns('left')[0].selectedItem = { base: 'file2.txt', isDir: false, size: 0, mtime: 0, ext: 'txt' };
  mockDirs['/dirB'] = [
    { name: 'file2.txt', isDir: false, size: 100 },
    { name: 'new.txt', isDir: false },
  ];
  await cvc.syncPane('left', { path: '/dirB' });
  assert.equal(cvc.getColumns('left')[0].items.length, 2);
  assert.equal(cvc.getColumns('left')[0].selectedItem?.size, 100);

  // 4. syncPane where matching column is at index > 0
  cvc.paneColumns.left = [
    { path: '/dirB', items: [{ base: 'file2.txt', isDir: false, size: 100, mtime: 0, ext: 'txt' }], selectedIndex: -1, selectedItem: null },
    { path: '/dirB/subfolder', items: [], selectedIndex: -1, selectedItem: null },
  ];
  mockDirs['/dirB/subfolder'] = [{ name: 'subfile.txt', isDir: false }];
  await cvc.syncPane('left', { path: '/dirB/subfolder' });
  assert.equal(cvc.getColumns('left')[1].items.length, 1);
  assert.equal(cvc.getActiveColumnIndex('left'), 1);
});

test('ColumnsViewController goForward, goBack, navigate bounds and edge cases', async () => {
  const mockDirs: Record<string, any[]> = {
    '/top': [{ name: 'empty_dir', isDir: true }, { name: 'full_dir', isDir: true }],
    '/top/empty_dir': [],
    '/top/full_dir': [{ name: 'item.txt', isDir: false }],
  };
  const api = () => ({
    readDir: async (p: string) => mockDirs[p] || [],
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
    pathDirname: async (p: string) => (p === '/top' ? '/top' : '/'), // /top is root here
  });

  let activated = '';
  const cvc = new ColumnsViewController({
    api,
    onActivateSide: (_side, p) => { activated = p || ''; },
  });
  await cvc.loadRoot('left', { path: '/top' });

  // 1. goBack on filesystem root (/top -> /top) does not unshift
  await cvc.goBack('left');
  assert.equal(cvc.getColumns('left').length, 1);

  // 2. Step into empty_dir
  await cvc.selectItem(0, 0, 'left');
  assert.equal(cvc.getColumns('left').length, 2);
  assert.equal(cvc.getColumns('left')[1].path, '/top/empty_dir');

  // 3. goForward into empty_dir
  await cvc.goForward('left');
  assert.equal(cvc.getActiveColumnIndex('left'), 1);

  // 4. Navigate vertically on empty column does nothing
  await cvc.navigate(0, 1, 'left');
  assert.equal(cvc.getActiveColumnIndex('left'), 1);

  // 5. Select full_dir in column 0
  await cvc.selectItem(0, 1, 'left');
  assert.equal(cvc.getColumns('left').length, 2);
  // Column 1 is /top/full_dir. goForward auto-selects first item
  await cvc.goForward('left');
  assert.equal(cvc.getActiveColumnIndex('left'), 1);
  assert.equal(cvc.getColumns('left')[1].selectedIndex, 0);

  // 6. Calling goForward when next column already has a selection (selectedIndex !== -1)
  cvc.activeColIndexes.left = 0;
  await cvc.goForward('left');
  assert.equal(cvc.getActiveColumnIndex('left'), 1);

  // 7. Calling goForward when in rightmost column and selected item is a dir: expands subfolder
  cvc.getColumns('left')[1].items.push({ base: 'nested_dir', isDir: true, size: 0, mtime: 0, ext: '' });
  cvc.getColumns('left')[1].selectedIndex = 1;
  cvc.getColumns('left')[1].selectedItem = cvc.getColumns('left')[1].items[1];
  mockDirs['/top/full_dir/nested_dir'] = [{ name: 'leaf.txt', isDir: false }];
  await cvc.goForward('left');
  assert.equal(cvc.getColumns('left').length, 3);
  assert.equal(cvc.getActiveColumnIndex('left'), 2);

  // 8. Calling goForward again when already in rightmost column and selected item is NOT a dir does nothing
  await cvc.goForward('left');
  assert.equal(cvc.getActiveColumnIndex('left'), 2);

  // 9. selectItem with invalid indices
  await cvc.selectItem(-1, 0, 'left');
  await cvc.selectItem(0, 999, 'left');

  // 10. navigate bounds checking
  await cvc.navigate(0, 10, 'left');
  assert.equal(cvc.getColumns('left')[2].selectedIndex, 0);
  await cvc.navigate(0, -10, 'left');
  assert.equal(cvc.getColumns('left')[2].selectedIndex, 0);
});

test('ColumnsViewController fetchDirectory error handling and joinPath fallback', async () => {
  // 1. readDir throws error -> returns empty array
  const failingApi = () => ({
    readDir: async () => { throw new Error('EACCES'); },
  });
  const cvcFail = new ColumnsViewController({ api: failingApi });
  const items = await cvcFail.fetchDirectory('/protected');
  assert.deepEqual(items, []);

  // 2. joinPath fallback when pathJoin is not provided
  const simpleApi = () => ({});
  const cvcSimple = new ColumnsViewController({ api: simpleApi });
  const joinedUnix = await cvcSimple.joinPath('/var/log', 'syslog');
  assert.equal(joinedUnix, '/var/log/syslog');
  const joinedWin = await cvcSimple.joinPath('C:\\Windows', 'System32');
  assert.equal(joinedWin, 'C:\\Windows\\System32');
  const joinedSlash = await cvcSimple.joinPath('/var/log/', 'syslog');
  assert.equal(joinedSlash, '/var/log/syslog');
});

test('ColumnsViewController render in columns-mode with inspector, preview and user interactions', async () => {
  function createMockEl(tag = 'div', className = '', dataset: Record<string, string> = {}) {
    let _className = className;
    const classes = new Set(className ? className.split(/\s+/).filter(Boolean) : []);
    const children: any[] = [];
    const listeners: Record<string, Function[]> = {};
    let _textContent = '';

    const el: any = {
      tagName: tag.toUpperCase(),
      get className() { return _className; },
      set className(val: string) {
        _className = val;
        classes.clear();
        val.split(/\s+/).filter(Boolean).forEach((c) => classes.add(c));
      },
      dataset: { ...dataset },
      style: {},
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
      },
      get textContent() {
        if (children.length > 0) {
          return children.map((c: any) => c.textContent).join(' ');
        }
        return _textContent;
      },
      set textContent(val: string) {
        _textContent = val;
      },
      innerHTML: '',
      children,
      draggable: false,
      scrollTop: 0,
      scrollWidth: 1000,
      replaceChildren: (...nodes: any[]) => {
        children.length = 0;
        if (nodes) children.push(...nodes);
      },
      appendChild: (child: any) => {
        children.push(child);
        return child;
      },
      addEventListener: (ev: string, fn: Function) => {
        if (!listeners[ev]) listeners[ev] = [];
        listeners[ev].push(fn);
      },
      dispatchEvent: (evName: string, evPayload: any = {}) => {
        const e = {
          target: el,
          clientX: 100,
          clientY: 200,
          preventDefault: () => {},
          stopPropagation: () => {},
          ...evPayload,
        };
        (listeners[evName] || []).forEach((fn) => fn(e));
      },
      closest: (sel: string) => {
        if (sel.startsWith('.')) {
          const reqClasses = sel.slice(1).split('.').filter(Boolean);
          if (reqClasses.every((cls) => classes.has(cls))) return el;
        }
        return null;
      },
      querySelector: (sel: string) => {
        if (sel.startsWith('.')) {
          const reqClasses = sel.slice(1).split('.').filter(Boolean);
          for (const c of children) {
            if (reqClasses.every((cls) => c.classList.contains(cls))) return c;
            const found = c.querySelector(sel);
            if (found) return found;
          }
        }
        return null;
      },
      querySelectorAll: (sel: string) => {
        const res: any[] = [];
        if (sel.startsWith('.')) {
          const reqClasses = sel.slice(1).split('.').filter(Boolean);
          for (const c of children) {
            if (reqClasses.every((cls) => c.classList.contains(cls))) res.push(c);
            res.push(...c.querySelectorAll(sel));
          }
        }
        return res;
      },
      scrollIntoView: () => {},
      scrollTo: () => {},
      onclick: null,
      onmousedown: null,
    };
    return el;
  }

  const appEl = createMockEl('div', 'columns-mode');
  const paneBody = createMockEl('div', 'pane-body');
  const docMock = {
    getElementById: (id: string) => (id === 'app' ? appEl : null),
    querySelector: (sel: string) => (sel.includes('#pane-left .pane-body') ? paneBody : null),
    createElement: (tag: string) => createMockEl(tag),
  };

  const origDoc = globalThis.document;
  (globalThis as any).document = docMock;

  let activatedSide = '';
  let openedSelectedPath = '';
  let previewedPath = '';
  let ctxMenuCalled = false;

  const mockApi = {
    readDir: async (p: string) => {
      if (p === '/root') {
        return [
          { name: 'sub', isDir: true, gitStatus: 'M' },
          { name: 'empty_sub', isDir: true },
        ];
      }
      if (p === '/root/sub') {
        return [
          { name: 'photo.png', isDir: false, size: 2048, mtime: 1600000000000 },
          { name: 'notes.txt', isDir: false, size: 500, mtime: 1600000000000 },
        ];
      }
      return [];
    },
    gitIsRepo: async () => ({ ok: true, root: '/root' }),
    gitStatus: async () => ({
      ok: true,
      branch: 'main',
      ahead: 0,
      behind: 0,
      files: [{ file: 'sub', index: 'M', worktree: ' ' }],
    }),
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
    assetUrl: (p: string) => `asset://${p}`,
    readMediaDataUrl: async () => 'data:image/png;base64,123',
  };

  const iconRegistry = {
    resolveIconKey: () => 'file',
    getSvg: () => '<svg></svg>',
  };

  const cvc = new ColumnsViewController({
    api: () => mockApi,
    iconRegistry,
    onActivateSide: (side) => { activatedSide = side; },
    onOpenSelected: (fp) => { openedSelectedPath = fp; },
    onPreviewSelected: (fp) => { previewedPath = fp; },
    showCtxMenu: () => { ctxMenuCalled = true; },
  });

  try {
    // 1. Initial load
    await cvc.loadRoot('left', { path: '/root' });
    const container = paneBody.children[0];
    assert.ok(container);

    // 2. Container onmousedown
    container.onmousedown();
    assert.equal(activatedSide, 'left');

    // 3. Select 'sub' (index 1 after sort) in column 0
    await cvc.selectItem(0, 1, 'left');
    assert.equal(cvc.getColumns('left').length, 2);

    // 4. Select 'photo.png' (image file, index 1 after sort) in column 1 -> triggers inspector rendering with image
    await cvc.selectItem(1, 1, 'left');
    // Wait for async joinPath inside createInspectorElement
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(container.children.length, 3);
    const inspectorEl = container.children[2];
    assert.ok(inspectorEl.classList.contains('columns-preview-column'));

    // Image onerror fallback
    const imgEl = inspectorEl.querySelector('.columns-preview-icon').children[0];
    await imgEl.onerror();
    assert.equal(imgEl.src, 'data:image/png;base64,123');

    // Quick Look preview button click
    const previewBtn = inspectorEl.querySelector('.columns-preview-actions').children[0];
    await previewBtn.onclick();
    assert.equal(previewedPath, '/root/sub/photo.png');

    // 5. Select 'notes.txt' (non-image file, index 0) in column 1
    await cvc.selectItem(1, 0, 'left');
    const textInspector = container.children[2];
    assert.ok(textInspector.textContent.includes('Kind'));
    assert.ok(textInspector.textContent.includes('500 B'));

    // 6. User interactions on column 0:
    const col0 = container.children[0];
    // Click column background outside row
    col0.dispatchEvent('click', { target: col0 });
    // Contextmenu on column background
    col0.dispatchEvent('contextmenu', { target: col0 });
    assert.equal(ctxMenuCalled, true);

    // 7. User interactions on row:
    const row0 = col0.children[0];
    // Row click
    row0.dispatchEvent('click', { target: row0 });
    // Row double click -> triggers onOpenSelected
    row0.dispatchEvent('dblclick', { target: row0 });
    await new Promise((r) => setTimeout(r, 10));
    assert.equal(openedSelectedPath, '/root/empty_sub');

    // Row contextmenu
    row0.dispatchEvent('contextmenu', { target: row0 });
    assert.equal(ctxMenuCalled, true);

    // 8. Select 'empty_sub' in column 0 -> renders Empty Folder in column 1
    await cvc.selectItem(0, 0, 'left');
    const col1 = container.children[1];
    assert.ok(col1.children[0].textContent.includes('Empty Folder'));

    // 9. scrollToColumn & scrollToEnd
    cvc.scrollToColumn('left', 0);
    cvc.scrollToEnd('left');
  } finally {
    globalThis.document = origDoc;
  }
});

test('ColumnsViewController syncPane prunes child column and updates selection when selected folder is deleted', async () => {
  const mockDirs: Record<string, any[]> = {
    '/root': [
      { name: 'folderA', isDir: true },
      { name: 'folderB', isDir: true },
    ],
    '/root/folderA': [
      { name: 'fileInsideA.txt', isDir: false },
    ],
  };

  const api = () => ({
    readDir: async (p: string) => {
      if (mockDirs[p]) return mockDirs[p];
      return { ok: false, error: 'Directory not found' };
    },
    pathJoin: async (p: string, c: string) => `${p}/${c}`,
    pathDirname: async (p: string) => (p === '/root' ? '/' : '/root'),
  });

  const cvc = new ColumnsViewController({ api });

  // Initial load: loadRoot at /root
  await cvc.loadRoot('left', { path: '/root', items: [] });
  assert.equal(cvc.getColumns('left').length, 1);
  assert.equal(cvc.getColumns('left')[0].items.length, 2);

  // User selects folderA (index 0)
  await cvc.selectItem(0, 0, 'left');
  assert.equal(cvc.getColumns('left').length, 2);
  assert.equal(cvc.getColumns('left')[0].selectedItem?.base, 'folderA');
  assert.equal(cvc.getColumns('left')[1].path, '/root/folderA');
  assert.equal(cvc.getColumns('left')[1].items.length, 1);

  // Now simulate folderA being deleted from disk
  mockDirs['/root'] = [{ name: 'folderB', isDir: true }];
  delete mockDirs['/root/folderA'];

  // Refresh via syncPane
  await cvc.syncPane('left', { path: '/root' });

  // Columns after index 0 must be pruned!
  const cols = cvc.getColumns('left');
  assert.equal(cols[0].items.length, 1);
  assert.equal(cols[0].items[0].base, 'folderB');
  // Selected item should update to folderB instead of staying as deleted folderA
  assert.equal(cols[0].selectedItem?.base, 'folderB');
  assert.equal(cols[0].selectedIndex, 0);

  // If folderB has a child column created because folderB is a directory, verify it points to /root/folderB, not /root/folderA!
  for (let i = 1; i < cols.length; i++) {
    assert.notEqual(cols[i].path, '/root/folderA');
  }
});
