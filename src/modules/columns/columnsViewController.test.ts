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
