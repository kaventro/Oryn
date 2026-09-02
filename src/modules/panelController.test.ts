// src/modules/panelController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { PanelController } from './panelController.ts';
import { AppState } from './stateModels.ts';

test('PanelController resets cursor to 0 when loading new directory', async () => {
  const state = new AppState();
  state.left.path = '/folderA';
  state.left.cursor = 4;

  let watched: string[] = [];
  const mockApi = {
    readDir: async (p: string) => ({
      ok: true,
      items: [
        { base: '..', isDir: true },
        { base: 'file1.txt', isDir: false },
        { base: 'file2.txt', isDir: false },
      ],
    }),
    pathJoin: async (a: string, b: string) => `${a}/${b}`,
    pathDirname: async (p: string) => p.split('/').slice(0, -1).join('/') || '/',
    pathBasename: async (p: string) => p.split('/').pop() || '',
    watchDirs: async (paths: string[]) => { watched = paths; },
  };

  let rendered = false;
  const controller = new PanelController({
    state,
    api: () => mockApi,
    setStatus: () => {},
    renderPane: () => { rendered = true; },
    updatePaneClass: () => {},
    focusActiveList: () => {},
  });

  await controller.loadDir('left');
  assert.equal(state.left.cursor, 0);
  assert.equal(rendered, true);
  assert.deepEqual(watched, ['/folderA']);
});

test('PanelController preserves selected item on auto-refresh', async () => {
  const state = new AppState();
  state.left.path = '/folderA';
  state.left.items = [
    { base: '..', isDir: true },
    { base: 'beta.txt', isDir: false },
    { base: 'gamma.txt', isDir: false },
  ];
  state.left.cursor = 2; // Selected gamma.txt

  const mockApi = {
    readDir: async (p: string) => ({
      ok: true,
      items: [
        { base: '..', isDir: true },
        { base: 'alpha.txt', isDir: false }, // Newly inserted before beta.txt and gamma.txt
        { base: 'beta.txt', isDir: false },
        { base: 'gamma.txt', isDir: false },
      ],
    }),
    pathJoin: async (a: string, b: string) => `${a}/${b}`,
    watchDirs: async () => {},
  };

  const controller = new PanelController({
    state,
    api: () => mockApi,
    setStatus: () => {},
    renderPane: () => {},
    updatePaneClass: () => {},
    focusActiveList: () => {},
  });

  await controller.loadDir('left', { preserveCursor: true });
  // gamma.txt is now at index 3 in sorted filtered list
  const { item, index } = controller.getFilteredSelection('left');
  assert.equal(item?.base, 'gamma.txt');
  assert.equal(index, 3);
  assert.equal(state.left.cursor, 3);
});

test('PanelController openSelected enters subdirectory and resets cursor', async () => {
  const state = new AppState();
  state.left.path = '/folderA';
  state.left.items = [
    { base: '..', isDir: true },
    { base: 'subfolder', isDir: true },
    { base: 'file.txt', isDir: false },
  ];
  state.left.cursor = 1; // selected subfolder

  let focused = false;
  const mockApi = {
    readDir: async (p: string) => ({
      ok: true,
      items: [
        { base: '..', isDir: true },
        { base: 'nested.txt', isDir: false },
      ],
    }),
    pathJoin: async (a: string, b: string) => `${a.replace(/\/+$/, '')}/${b}`,
    pathNormalize: async (p: string) => p,
    watchDirs: async () => {},
  };

  const controller = new PanelController({
    state,
    api: () => mockApi,
    setStatus: () => {},
    renderPane: () => {},
    updatePaneClass: () => {},
    focusActiveList: () => { focused = true; },
  });

  await controller.openSelected('left');
  assert.equal(state.left.path, '/folderA/subfolder');
  assert.equal(state.left.cursor, 0);
  assert.equal(focused, true);
});

test('PanelController loadDir attaches git branch and per-item status badges', async () => {
  const { clearGitStatusCache } = await import('./gitStatusMapper.ts');
  clearGitStatusCache();

  const state = new AppState();
  state.left.path = '/git-pane-repo/ios';

  const mockApi = {
    readDir: async () => ({
      ok: true,
      items: [
        { base: 'App.swift', isDir: false },
        { base: 'clean.txt', isDir: false },
      ],
    }),
    gitIsRepo: async () => ({ ok: true, root: '/git-pane-repo' }),
    gitStatus: async () => ({
      ok: true,
      branch: 'feature/ui',
      ahead: 1,
      behind: 0,
      files: [{ file: 'ios/App.swift', index: ' ', worktree: 'M' }],
    }),
    watchDirs: async () => {},
  };

  const controller = new PanelController({
    state,
    api: () => mockApi,
    setStatus: () => {},
    renderPane: () => {},
    updatePaneClass: () => {},
    focusActiveList: () => {},
  });

  await controller.loadDir('left');
  assert.equal(state.left.git?.branch, 'feature/ui');
  assert.equal(state.left.git?.ahead, 1);
  assert.equal(state.left.items.find((i) => i.base === 'App.swift')?.gitStatus, 'M');
  assert.equal(state.left.items.find((i) => i.base === 'clean.txt')?.gitStatus, null);
});

test('PanelController refreshAll skips syncing columnsViewController when not in columns-mode', async () => {
  const state = new AppState();
  state.left.path = '/folderA';
  state.right.path = '/folderB';

  const mockApi = {
    readDir: async () => ({ ok: true, items: [] }),
    watchDirs: async () => {},
  };

  let synced = false;
  const mockColumnsVC = {
    syncPane: async () => { synced = true; },
  };

  const mockApp = { classList: { contains: (c: string) => c === 'list-mode' } };
  const mockDoc = {
    getElementById: (id: string) => (id === 'app' ? mockApp : null),
  };
  (globalThis as any).document = mockDoc;

  try {
    const controller = new PanelController({
      state,
      api: () => mockApi,
      setStatus: () => {},
      renderPane: () => {},
      updatePaneClass: () => {},
      focusActiveList: () => {},
      columnsViewController: mockColumnsVC,
    });

    await controller.refreshAll({ force: true });
    assert.equal(synced, false, 'columnsViewController should not be synced in list mode');
  } finally {
    delete (globalThis as any).document;
  }
});


