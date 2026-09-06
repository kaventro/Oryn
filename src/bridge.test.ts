// src/bridge.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

// Setup global mock environment before importing bridge
const origSetTimeout = globalThis.setTimeout;
globalThis.setTimeout = ((fn: Function, delay?: number, ...args: any[]) => {
  if (typeof delay === 'number' && delay >= 1000) {
    return { unref() {} } as any;
  }
  return origSetTimeout(fn, delay, ...args);
}) as any;

globalThis.setInterval = (() => ({ unref() {} })) as any;

(globalThis as any).window = Object.assign(globalThis, {
  addEventListener: () => {},
  removeEventListener: () => {},
});

const mockStorage: Record<string, string> = {
  'Oryn.showSysStats': 'false',
};
(globalThis as any).localStorage = {
  getItem: (k: string) => mockStorage[k] || null,
  setItem: (k: string, v: any) => { mockStorage[k] = String(v); },
  removeItem: (k: string) => { delete mockStorage[k]; },
  clear: () => { Object.keys(mockStorage).forEach((k) => delete mockStorage[k]); },
};

function createMockEl(): any {
  const classes = new Set<string>();
  const children: any[] = [];
  const listeners: Record<string, Function[]> = {};
  return {
    children,
    dataset: {},
    style: {},
    className: '',
    textContent: '',
    value: '',
    removeAttribute(_a: string) {},
    setAttribute(_a: string, _b: string) {},
    classList: {
      add(c: string) { classes.add(c); },
      remove(c: string) { classes.delete(c); },
      contains(c: string) { return classes.has(c); },
      toggle(c: string, f?: boolean) {
        if (f !== undefined) {
          if (f) classes.add(c); else classes.delete(c);
        } else {
          if (classes.has(c)) classes.delete(c); else classes.add(c);
        }
      },
    },
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    removeEventListener(ev: string, fn: Function) {
      if (listeners[ev]) listeners[ev] = listeners[ev].filter((f) => f !== fn);
    },
    dispatchEvent(ev: string, payload: any) {
      (listeners[ev] || []).forEach((fn) => fn(payload));
    },
    appendChild(c: any) { children.push(c); return c; },
    append(...cs: any[]) { children.push(...cs); },
    replaceChildren(...cs: any[]) { children.length = 0; children.push(...cs); },
    get childElementCount() { return children.length; },
    get lastElementChild() { return children[children.length - 1] || null; },
    remove() {},
    focus() {},
    select() {},
    querySelectorAll: () => [],
    querySelector: () => createMockEl(),
  };
}

const mockDocEl = createMockEl();
(globalThis as any).document = {
  documentElement: mockDocEl,
  body: mockDocEl,
  getElementById: () => mockDocEl,
  querySelector: () => mockDocEl,
  querySelectorAll: () => [],
  createElement: () => createMockEl(),
  addEventListener: () => {},
  removeEventListener: () => {},
};

let lastInvokeCmd = '';
let lastInvokeArgs: any = null;
const invokeHistory: string[] = [];
let invokeThrows: Error | null = null;
const eventHandlers: Record<string, Function[]> = {};

(globalThis as any).__TAURI_INTERNALS__ = {
  convertFileSrc: (src: string, protocol = 'asset') => `${protocol}://localhost/${encodeURIComponent(src)}`,
  invoke: async (cmd: string, args: any) => {
    lastInvokeCmd = cmd;
    lastInvokeArgs = args;
    invokeHistory.push(cmd);
    if (invokeThrows) throw invokeThrows;

    if (cmd === 'plugin:event|listen') {
      const eventName = args.event;
      if (!eventHandlers[eventName]) eventHandlers[eventName] = [];
      eventHandlers[eventName].push(args.handler);
      return 42;
    }
    if (cmd === 'plugin:event|unlisten') {
      return null;
    }
    if (cmd === 'app_get_home') {
      if (invokeThrows) throw invokeThrows;
      return '/home/user';
    }
    if (cmd === 'config_load') return {};
    if (cmd === 'fs_read_dir') return { ok: true, items: [] };
    if (cmd === 'system_get_locations') return { locations: [] };
    if (cmd === 'remote_list_profiles') return { profiles: [] };
    if (cmd === 'system_get_path_space') return { ok: true, free: 1000, total: 2000 };
    if (cmd === 'fs_watch_dirs') return { ok: true };
    if (cmd === 'set_dock_icon') return { ok: true };
    if (invokeThrows) throw invokeThrows;
    return { ok: true, cmd, args };
  },
  transformCallback: (cb: any) => cb,
};

(globalThis as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
  unregisterListener: (_event: string, _id: number) => {},
};

// Import bridge module
const { bridge } = await import('./bridge.ts');
await new Promise((r) => setTimeout(r, 50));

test('bridge window.ow assignment', () => {
  assert.equal((globalThis as any).ow, bridge);
});

test('bridge assetUrl formats path or handles empty', () => {
  assert.equal(bridge.assetUrl(''), '');
  const url = bridge.assetUrl('C:\\Users\\admin\\photo.png');
  assert.ok(url.includes('photo.png'));
  assert.ok(!url.includes('\\'));
});

test('bridge error sanitization in ipcInvoke', async () => {
  // 1. Password redaction
  invokeThrows = new Error('Database error with password=super_secret_pwd and token=xyz_token');
  await assert.rejects(
    async () => bridge.getHome(),
    /password=\[REDACTED\] and token=\[REDACTED\]/,
  );

  // 2. Bearer token redaction
  invokeThrows = new Error('Unauthorized Bearer secret_access_token_123456');
  await assert.rejects(
    async () => bridge.getHome(),
    /Bearer \[REDACTED\]/,
  );

  // 3. Length truncation > 140
  const longMsg = 'A'.repeat(200);
  invokeThrows = new Error(longMsg);
  await assert.rejects(
    async () => bridge.getHome(),
    (err: Error) => err.message.length <= 140 && err.message.endsWith('…'),
  );

  invokeThrows = null;
});

test('bridge filesystem and path methods invoke correct backend commands', async () => {
  await bridge.pathJoin('dir', 'file.txt');
  assert.equal(lastInvokeCmd, 'path_join');
  assert.deepEqual(lastInvokeArgs, { a: 'dir', b: 'file.txt' });

  await bridge.pathDirname('/a/b/c.txt');
  assert.equal(lastInvokeCmd, 'path_dirname');
  assert.deepEqual(lastInvokeArgs, { input: { path: '/a/b/c.txt' } });

  await bridge.pathBasename('/a/b/c.txt');
  assert.equal(lastInvokeCmd, 'path_basename');

  await bridge.pathNormalize('/a/b/../c.txt');
  assert.equal(lastInvokeCmd, 'path_normalize');

  await bridge.getHome();
  assert.equal(lastInvokeCmd, 'app_get_home');

  await bridge.loadConfig();
  assert.equal(lastInvokeCmd, 'config_load');

  await bridge.readDir('/test');
  assert.equal(lastInvokeCmd, 'fs_read_dir');

  await bridge.readFlatBranch('/branch');
  assert.equal(lastInvokeCmd, 'fs_read_flat_branch');

  await bridge.mkdir('/newdir');
  assert.equal(lastInvokeCmd, 'fs_mkdir');

  await bridge.createFile('/file.txt');
  assert.equal(lastInvokeCmd, 'fs_create_file');
  assert.equal((lastInvokeArgs.input as any).content, '');

  await bridge.createFile('/file2.txt', 'hello');
  assert.equal((lastInvokeArgs.input as any).content, 'hello');

  await bridge.openPath('/doc.pdf');
  assert.equal(lastInvokeCmd, 'shell_open_path');

  await bridge.showItemInFolder('/doc.pdf');
  assert.equal(lastInvokeCmd, 'shell_show_in_folder');

  await bridge.openVSCode('/workspace');
  assert.equal(lastInvokeCmd, 'shell_open_vscode');

  await bridge.openTerminal('/workspace');
  assert.equal(lastInvokeCmd, 'shell_open_terminal');

  await bridge.clipboardWrite('sample text');
  assert.equal(lastInvokeCmd, 'clipboard_write');

  await bridge.shellExec('ls -la', '/workspace');
  assert.equal(lastInvokeCmd, 'shell_exec');

  await bridge.statProps('/file.txt');
  assert.equal(lastInvokeCmd, 'fs_stat_props');

  await bridge.readFileText('/file.txt', 1024);
  assert.equal(lastInvokeCmd, 'fs_read_file_text');

  await bridge.writeFileText('/file.txt', 'data');
  assert.equal(lastInvokeCmd, 'fs_write_file_text');

  await bridge.probeText('/file.bin');
  assert.equal(lastInvokeCmd, 'fs_probe_text');

  await bridge.readOffice('/doc.docx');
  assert.equal(lastInvokeCmd, 'fs_read_office');

  await bridge.readMediaDataUrl('/image.png');
  assert.equal(lastInvokeCmd, 'fs_read_media_data_url');

  await bridge.rename('/old.txt', '/new.txt');
  assert.equal(lastInvokeCmd, 'fs_rename');

  await bridge.deletePath('/trash.txt', true);
  assert.equal(lastInvokeCmd, 'fs_delete');

  await bridge.compressZip('/archive.zip');
  assert.equal(lastInvokeCmd, 'fs_compress_zip');
});

test('bridge search and transfer methods invoke backend', async () => {
  await bridge.clearSearchCache();
  assert.equal(lastInvokeCmd, 'search_clear_cache');

  await bridge.searchStart({ query: 'hello' });
  assert.equal(lastInvokeCmd, 'search_start');

  await bridge.searchPage('sess_1', 0, 50);
  assert.equal(lastInvokeCmd, 'search_get_page');

  await bridge.cancelSearch('client_1');
  assert.equal(lastInvokeCmd, 'search_cancel');

  await bridge.releaseSearch('sess_1');
  assert.equal(lastInvokeCmd, 'search_release');

  await bridge.copy('/src', '/dst', 'overwrite');
  assert.equal(lastInvokeCmd, 'fs_copy');

  await bridge.move('/src', '/dst');
  assert.equal(lastInvokeCmd, 'fs_move');

  await bridge.copyConflicts('/src', '/dst');
  assert.equal(lastInvokeCmd, 'fs_copy_conflicts');

  await bridge.cancelCopy();
  assert.equal(lastInvokeCmd, 'fs_cancel_copy');

  await bridge.watchDirs(['/dir1', '/dir2']);
  assert.equal(lastInvokeCmd, 'fs_watch_dirs');

  await bridge.confirm({ title: 'Confirm' });
  assert.equal(lastInvokeCmd, 'dialog_confirm');

  await bridge.info({ title: 'Info' });
  assert.equal(lastInvokeCmd, 'dialog_info');

  await bridge.closeWindow();
  assert.equal(lastInvokeCmd, 'window_close');

  await bridge.minimizeWindow();
  assert.equal(lastInvokeCmd, 'window_minimize');

  await bridge.toggleMaximizeWindow();
  assert.equal(lastInvokeCmd, 'window_toggle_maximize');

  await bridge.setDockIcon('icon_a');
  assert.equal(lastInvokeCmd, 'set_dock_icon');

  await bridge.getSystemStats();
  assert.equal(lastInvokeCmd, 'system_get_stats');

  await bridge.getPathSpace('/mount');
  assert.equal(lastInvokeCmd, 'system_get_path_space');

  await bridge.getSystemLocations();
  assert.equal(lastInvokeCmd, 'system_get_locations');

  await bridge.getDirSize('/folder');
  assert.equal(lastInvokeCmd, 'fs_get_dir_size');

  await bridge.analyzeDir('/folder');
  assert.equal(lastInvokeCmd, 'fs_analyze_dir');

  await bridge.scanDuplicates('/folder', 100, 50);
  assert.equal(lastInvokeCmd, 'fs_scan_duplicates');

  await bridge.fsChecksum('/file.txt');
  assert.equal(lastInvokeCmd, 'fs_checksum');

  await bridge.pickFolder('/home');
  assert.equal(lastInvokeCmd, 'dialog_pick_folder');

  await bridge.extractZip('/archive.zip', 'entry.txt');
  assert.equal(lastInvokeCmd, 'zip_extract');

  await bridge.fsCompress(['/a.txt'], '/dest.zip');
  assert.equal(lastInvokeCmd, 'fs_compress');

  await bridge.fsExtract('/dest.zip', '/out');
  assert.equal(lastInvokeCmd, 'fs_extract');

  await bridge.compareFiles('/a.txt', '/b.txt');
  assert.equal(lastInvokeCmd, 'compare_files');
});

test('bridge event listeners and channel callbacks', async () => {
  // onFsChange
  let fsChangePayload: any = null;
  const unlistenFs = bridge.onFsChange((p) => { fsChangePayload = p; });
  assert.equal(typeof unlistenFs, 'function');
  await new Promise((r) => setTimeout(r, 10));
  // Trigger event
  const fsListeners = eventHandlers['fs:change'] || [];
  fsListeners.forEach((fn) => fn({ payload: { path: '/changed.txt' } }));
  assert.deepEqual(fsChangePayload, { path: '/changed.txt' });
  unlistenFs();

  // onCopyProgress
  let copyProgressPayload: any = null;
  const unlistenCopy = bridge.onCopyProgress((p) => { copyProgressPayload = p; });
  await new Promise((r) => setTimeout(r, 10));
  const copyListeners = eventHandlers['fs:copyProgress'] || [];
  copyListeners.forEach((fn) => fn({ payload: { pct: 50 } }));
  assert.deepEqual(copyProgressPayload, { pct: 50 });
  unlistenCopy();

  // onMenuRefresh and onMenuCopyPath
  let menuRefreshCalled = false;
  bridge.onMenuRefresh(() => { menuRefreshCalled = true; });
  await new Promise((r) => setTimeout(r, 10));
  (eventHandlers['menu:refresh'] || []).forEach((fn) => fn({}));
  assert.equal(menuRefreshCalled, true);

  let menuCopyPathCalled = false;
  bridge.onMenuCopyPath(() => { menuCopyPathCalled = true; });
  await new Promise((r) => setTimeout(r, 10));
  (eventHandlers['menu:copyPath'] || []).forEach((fn) => fn({}));
  assert.equal(menuCopyPathCalled, true);

  // findReplace with and without onProgress
  let replaceProgressData: any = null;
  await bridge.findReplace({ find: 'a', replace: 'b' }, (p) => { replaceProgressData = p; });
  assert.ok(invokeHistory.includes('fs_find_replace'));
  (eventHandlers['fs:replaceProgress'] || []).forEach((fn) => fn({ payload: { count: 1 } }));
  assert.deepEqual(replaceProgressData, { count: 1 });

  await bridge.findReplace({ find: 'a', replace: 'b' }); // without progress callback
  assert.equal(lastInvokeCmd, 'fs_find_replace');

  // compareDirs with and without onUpdate
  let compareUpdateData: any = null;
  await bridge.compareDirs('/left', '/right', (u) => { compareUpdateData = u; });
  assert.ok(invokeHistory.includes('compare_dirs'));
  (eventHandlers['compare:update'] || []).forEach((fn) => fn({ payload: { status: 'diff' } }));
  assert.deepEqual(compareUpdateData, { status: 'diff' });

  await bridge.compareDirs('/left', '/right'); // without update callback
  assert.equal(lastInvokeCmd, 'compare_dirs');
});

test('bridge git and remote methods invoke backend', async () => {
  await bridge.gitIsRepo('/repo');
  assert.equal(lastInvokeCmd, 'git_is_repo');

  await bridge.gitStatus('/repo');
  assert.equal(lastInvokeCmd, 'git_status');

  await bridge.gitLog('/repo', 10, 'file.txt');
  assert.equal(lastInvokeCmd, 'git_log');

  await bridge.gitDiff('/repo', 'main', 'feature', 'file.txt');
  assert.equal(lastInvokeCmd, 'git_diff');

  await bridge.gitBlame('/repo', 'file.txt', 'HEAD');
  assert.equal(lastInvokeCmd, 'git_blame');

  await bridge.gitAdd('/repo', ['file.txt']);
  assert.equal(lastInvokeCmd, 'git_add');

  await bridge.gitStageFile('/repo', 'file.txt', true);
  assert.equal(lastInvokeCmd, 'git_stage_file');

  await bridge.gitRestore('/repo', 'file.txt', true);
  assert.equal(lastInvokeCmd, 'git_restore');

  await bridge.gitCommit('/repo', 'commit msg');
  assert.equal(lastInvokeCmd, 'git_commit');

  await bridge.gitPush('/repo');
  assert.equal(lastInvokeCmd, 'git_push');

  await bridge.gitPull('/repo');
  assert.equal(lastInvokeCmd, 'git_pull');

  await bridge.gitStash('/repo', 'stash msg');
  assert.equal(lastInvokeCmd, 'git_stash');

  await bridge.gitStashPop('/repo');
  assert.equal(lastInvokeCmd, 'git_stash_pop');

  await bridge.gitStashList('/repo');
  assert.equal(lastInvokeCmd, 'git_stash_list');

  await bridge.gitBranches('/repo');
  assert.equal(lastInvokeCmd, 'git_branches');

  await bridge.gitCheckout('/repo', 'main', false);
  assert.equal(lastInvokeCmd, 'git_checkout');

  // Remote methods
  await bridge.remoteListProfiles();
  assert.equal(lastInvokeCmd, 'remote_list_profiles');

  await bridge.remoteSaveProfile({ name: 'SSH' });
  assert.equal(lastInvokeCmd, 'remote_save_profile');

  await bridge.remoteDeleteProfile('p1');
  assert.equal(lastInvokeCmd, 'remote_delete_profile');

  await bridge.remoteTestConnection({ name: 'SSH' });
  assert.equal(lastInvokeCmd, 'remote_test_connection');

  await bridge.remoteConnect({ name: 'SSH' });
  assert.equal(lastInvokeCmd, 'remote_connect');

  await bridge.remoteDisconnect('p1');
  assert.equal(lastInvokeCmd, 'remote_disconnect');

  await bridge.remoteReadDir('p1', '/remotedir');
  assert.equal(lastInvokeCmd, 'remote_read_dir');

  await bridge.remoteReadFileText('p1', '/file.txt');
  assert.equal(lastInvokeCmd, 'remote_read_file_text');

  await bridge.remoteWriteFileText('p1', '/file.txt', 'remote-content');
  assert.equal(lastInvokeCmd, 'remote_write_file_text');

  await bridge.remoteMkdir('p1', '/remotedir');
  assert.equal(lastInvokeCmd, 'remote_mkdir');

  await bridge.remoteCreateFile('p1', '/remotefile.txt');
  assert.equal(lastInvokeCmd, 'remote_create_file');

  await bridge.remoteRename('p1', '/old', '/new');
  assert.equal(lastInvokeCmd, 'remote_rename');

  await bridge.remoteDelete('p1', '/remotepath', false);
  assert.equal(lastInvokeCmd, 'remote_delete');

  await bridge.remoteDownload('p1', '/remote.txt', '/local.txt');
  assert.equal(lastInvokeCmd, 'remote_download');

  await bridge.remoteUpload('p1', '/local.txt', '/remote.txt');
  assert.equal(lastInvokeCmd, 'remote_upload');
});

test('app.ts command line input execution and escape', async () => {
  mockDocEl.value = 'cd /newpath';
  mockDocEl.dispatchEvent('keydown', { key: 'Enter', preventDefault: () => {} });

  mockDocEl.value = 'echo hello';
  mockDocEl.dispatchEvent('keydown', { key: 'Enter', preventDefault: () => {} });
  await new Promise((r) => setTimeout(r, 10));

  mockDocEl.value = 'some command';
  mockDocEl.dispatchEvent('keydown', { key: 'Escape', preventDefault: () => {} });
  assert.equal(mockDocEl.value, '');
});
