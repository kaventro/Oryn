// src/modules/features.critical-fixes.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { AppSettings } from './settings/settingsModel.ts';
import { SettingsStorageService } from './settings/settingsStorage.ts';
import { readDefaultEditor } from './preferencesController.ts';
import { CommandsController } from './commandsController.ts';
import { ContextMenuBuilder } from './menu/contextMenuBuilder.ts';
import { MoreOptionsMenuBuilder } from './menu/moreOptionsMenuBuilder.ts';
import { FileOpsController } from './fileOpsController.ts';
import { AppState } from './stateModels.ts';

// Helper to mock localStorage in Node.js test environment
function setupMockLocalStorage() {
  const store: Record<string, string> = {};
  (globalThis as any).localStorage = {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, val: string) => { store[key] = String(val); },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { Object.keys(store).forEach((k) => delete store[k]); },
  };
  return store;
}

function makeMockEl(tag: string): any {
  const el: any = {
    tagName: tag.toUpperCase(),
    className: '',
    id: '',
    style: {} as Record<string, string>,
    textContent: '',
    children: [] as any[],
    dataset: {} as Record<string, string>,
    onclick: null as any,
    onpointerdown: null as any,
    classList: {
      _set: new Set<string>(),
      add(c: string) { this._set.add(c); el.className = [...this._set].join(' '); },
      remove(c: string) { this._set.delete(c); el.className = [...this._set].join(' '); },
      contains(c: string) { return this._set.has(c); },
      toggle(c: string, force?: boolean) {
        if (force === undefined) {
          if (this._set.has(c)) this._set.delete(c); else this._set.add(c);
        } else if (force) this._set.add(c); else this._set.delete(c);
        el.className = [...this._set].join(' ');
      },
    },
    setAttribute(k: string, v: string) { el.dataset[k] = v; },
    getAttribute(k: string) { return el.dataset[k] ?? null; },
    removeAttribute(k: string) { delete el.dataset[k]; },
    addEventListener(type: string, fn: any) {
      if (type === 'click') el.onclick = fn;
    },
    removeEventListener() {},
    querySelector(sel: string) {
      const id = sel.startsWith('#') ? sel.slice(1) : '';
      const walk = (n: any): any => {
        if (n.id === id) return n;
        for (const ch of n.children || []) {
          const hit = walk(ch);
          if (hit) return hit;
        }
        return null;
      };
      return walk(el);
    },
    querySelectorAll() { return []; },
    replaceChildren(...nodes: any[]) { el.children = nodes; },
    appendChild(child: any) { el.children.push(child); return child; },
    append(...nodes: any[]) { el.children.push(...nodes); },
    focus() {},
  };
  return el;
}

function setupMockDocument() {
  const bodyChildren: any[] = [];
  const root = makeMockEl('html');
  const body = makeMockEl('body');
  (globalThis as any).document = {
    documentElement: root,
    body: {
      ...body,
      appendChild(el: any) { bodyChildren.push(el); return el; },
    },
    createElement: (tag: string) => makeMockEl(tag),
    getElementById: (id: string) => {
      const walk = (n: any): any => {
        if (n.id === id) return n;
        for (const ch of n.children || []) {
          const hit = walk(ch);
          if (hit) return hit;
        }
        return null;
      };
      for (const el of bodyChildren) {
        const found = walk(el);
        if (found) return found;
      }
      return null;
    },
    querySelector: () => null,
    querySelectorAll: () => [],
    addEventListener: () => {},
    removeEventListener: () => {},
  };
}

test('AppSettings supports zed and all preferred code editors', () => {
  const sDefault = new AppSettings({});
  assert.equal(sDefault.defaultEditor, 'vscode');

  const sCursor = new AppSettings({ defaultEditor: 'cursor' });
  assert.equal(sCursor.defaultEditor, 'cursor');

  const sZed = new AppSettings({ defaultEditor: 'zed' });
  assert.equal(sZed.defaultEditor, 'zed');

  const sSublime = new AppSettings({ defaultEditor: 'sublime' });
  assert.equal(sSublime.defaultEditor, 'sublime');

  const sCustom = new AppSettings({ defaultEditor: 'custom', customEditorCmd: 'nvim' });
  assert.equal(sCustom.defaultEditor, 'custom');
  assert.equal(sCustom.customEditorCmd, 'nvim');

  const sInvalid = new AppSettings({ defaultEditor: 'invalid-editor' });
  assert.equal(sInvalid.defaultEditor, 'vscode');
});

test('readDefaultEditor returns stored editor preference', () => {
  setupMockLocalStorage();
  setupMockDocument();
  const storage = new SettingsStorageService();

  // Save cursor
  const settings = new AppSettings({ defaultEditor: 'cursor' });
  storage.save(settings);

  const res = readDefaultEditor();
  assert.equal(res.editor, 'cursor');

  // Save zed
  settings.defaultEditor = 'zed';
  storage.save(settings);
  assert.equal(readDefaultEditor().editor, 'zed');

  // Save custom
  settings.defaultEditor = 'custom';
  settings.customEditorCmd = 'zed --wait';
  storage.save(settings);
  const customRes = readDefaultEditor();
  assert.equal(customRes.editor, 'custom');
  assert.equal(customRes.customCmd, 'zed --wait');
});

test('CommandsController runCommand(openEditor / editVSCode) calls openEditor with preferred editor', async () => {
  setupMockLocalStorage();
  setupMockDocument();
  const storage = new SettingsStorageService();
  storage.save(new AppSettings({ defaultEditor: 'cursor' }));

  let openedPath = '';
  let openedEditor = '';
  let openedCustomCmd = '';

  const mockApi = () => ({
    openEditor: async (path: string, editor?: string, customCmd?: string) => {
      openedPath = path;
      openedEditor = editor || '';
      openedCustomCmd = customCmd || '';
      return { ok: true };
    },
    openVSCode: async (_path: string) => {
      assert.fail('Should have called openEditor, not openVSCode fallback');
    },
  });

  const state: any = {
    active: 'left',
    left: { path: '/home/project' },
    right: { path: '/home/other' },
  };

  const ctrl = new CommandsController({
    api: mockApi,
    state,
    setStatus: () => {},
    focusActiveList: () => {},
    fullPath: async (_pane: any, item: any) => `/home/project/${item.base}`,
    getFilteredSelection: () => ({ item: { base: 'main.rs', isDir: false } }),
  });

  // Test openEditor
  ctrl.runCommand('openEditor');
  await new Promise((r) => setTimeout(r, 10));

  assert.equal(openedPath, '/home/project/main.rs');
  assert.equal(openedEditor, 'cursor');

  // Test editVSCode alias (shortcut Shift+F4)
  storage.save(new AppSettings({ defaultEditor: 'custom', customEditorCmd: 'subl -n' }));
  ctrl.runCommand('editVSCode', '/direct/path/script.py');
  await new Promise((r) => setTimeout(r, 10));

  assert.equal(openedPath, '/direct/path/script.py');
  assert.equal(openedEditor, 'custom');
  assert.equal(openedCustomCmd, 'subl -n');
});

test('ContextMenuBuilder dynamically updates label to match configured editor and triggers openEditor', async () => {
  setupMockLocalStorage();
  setupMockDocument();
  const storage = new SettingsStorageService();
  storage.save(new AppSettings({ defaultEditor: 'cursor' }));

  let openedPath = '';
  let openedEditor = '';

  const mockApi = () => ({
    openEditor: async (path: string, editor?: string) => {
      openedPath = path;
      openedEditor = editor || '';
      return { ok: true };
    },
  });

  const builder = new ContextMenuBuilder();
  const ctx: any = {
    side: 'left',
    item: { base: 'app.ts', isDir: false },
    targetDir: '/src',
    isFile: true,
    isDir: false,
    getPath: async () => '/src/app.ts',
    isMac: true,
  };

  const deps: any = {
    state: { active: 'left' },
    api: mockApi,
    setStatus: () => {},
  };

  const items = builder.build(ctx, deps, () => {});
  const editorItem = items.find((i) => i.id === 'openEditor');
  assert.ok(editorItem, 'Editor menu item should be present');
  assert.equal(editorItem.label, 'Open in Cursor');

  await editorItem.action?.();
  assert.equal(openedPath, '/src/app.ts');
  assert.equal(openedEditor, 'cursor');
});

test('MoreOptionsMenuBuilder dynamically displays configured editor label and invokes openEditor', async () => {
  setupMockLocalStorage();
  setupMockDocument();
  const storage = new SettingsStorageService();
  storage.save(new AppSettings({ defaultEditor: 'zed' }));

  let openedPath = '';
  let openedEditor = '';

  const mockApi = () => ({
    openEditor: async (path: string, editor?: string) => {
      openedPath = path;
      openedEditor = editor || '';
      return { ok: true };
    },
  });

  const builder = new MoreOptionsMenuBuilder();
  const ctx: any = {
    side: 'right',
    item: { base: 'index.html', isDir: false },
    targetDir: '/web',
    getPath: async () => '/web/index.html',
    isMac: true,
  };

  const deps: any = {
    state: { active: 'right' },
    api: mockApi,
    setStatus: () => {},
  };

  const items = builder.build(ctx, deps, () => {});
  const editorItem = items.find((i) => i.id === 'openEditor');
  assert.ok(editorItem, 'Editor menu item should exist in More Options menu');
  assert.equal(editorItem.label, 'Open in Zed');

  await editorItem.action?.();
  assert.equal(openedPath, '/web/index.html');
  assert.equal(openedEditor, 'zed');
});

test('FileOpsController: when trash delete fails, prompts user; if cancelled, file is NOT deleted', async () => {
  setupMockLocalStorage();
  setupMockDocument();

  let deleteCallCount = 0;
  let lastUsedTrash: boolean | undefined = undefined;

  const mockApi = () => ({
    deletePath: async (_p: string, useTrash?: boolean) => {
      deleteCallCount++;
      lastUsedTrash = useTrash;
      if (useTrash) {
        // Trash fails! (e.g. external USB, permission error, AppleScript -1743)
        return { ok: false, error: 'Failed to move to trash: Operation not permitted' };
      }
      return { ok: true };
    },
    pathDirname: async (p: string) => p.split('/').slice(0, -1).join('/'),
  });

  const state = new AppState();
  state.active = 'left';
  state.config.useTrash = true;

  const controller = new FileOpsController({
    state,
    api: mockApi,
    setStatus: () => {},
    refreshAll: async () => {},
    focusActiveList: () => {},
    getFilteredSelection: () => ({ vis: [] }),
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    loadDir: async () => {},
    fullPath: async (_p, item) => item?.base || '',
  });

  // Mock dialogs: confirm initial delete, but reject permanent deletion fallback
  (controller as any).confirmDelete = async () => true;
  let fallbackPromptShown = false;
  (controller as any).askPermanentFallback = async (summary: string) => {
    fallbackPromptShown = true;
    assert.ok(summary.includes('photo.jpg'));
    return false; // User clicks "Cancel"
  };

  await controller.beginDelete({
    targetPath: '/external-usb/photo.jpg',
    targetItem: { base: 'photo.jpg', isDir: false },
    permanent: false,
  });

  // Verified:
  // 1. Fallback prompt was shown to user
  assert.equal(fallbackPromptShown, true, 'User must be asked for confirmation before permanent delete');
  // 2. Only one delete attempt was made (with useTrash = true)
  assert.equal(deleteCallCount, 1);
  assert.equal(lastUsedTrash, true);
  // 3. File was NOT permanently deleted
});

test('FileOpsController: when trash delete fails and user confirms permanent fallback, deletes permanently', async () => {
  setupMockLocalStorage();
  setupMockDocument();

  const deleteCalls: { path: string; useTrash?: boolean }[] = [];

  const mockApi = () => ({
    deletePath: async (p: string, useTrash?: boolean) => {
      deleteCalls.push({ path: p, useTrash });
      if (useTrash) {
        return { ok: false, error: 'Failed to move to trash: Disk does not support trash' };
      }
      return { ok: true };
    },
    pathDirname: async (p: string) => p.split('/').slice(0, -1).join('/'),
  });

  const state = new AppState();
  state.active = 'left';
  state.config.useTrash = true;

  const controller = new FileOpsController({
    state,
    api: mockApi,
    setStatus: () => {},
    refreshAll: async () => {},
    focusActiveList: () => {},
    getFilteredSelection: () => ({ vis: [] }),
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    loadDir: async () => {},
    fullPath: async (_p, item) => item?.base || '',
  });

  (controller as any).confirmDelete = async () => true;
  let fallbackPromptShown = false;
  (controller as any).askPermanentFallback = async (summary: string) => {
    fallbackPromptShown = true;
    assert.ok(summary.includes('report.docx'));
    return true; // User clicks "Permanently Delete"
  };

  await controller.beginDelete({
    targetPath: '/network-drive/report.docx',
    targetItem: { base: 'report.docx', isDir: false },
    permanent: false,
  });

  // Verified:
  assert.equal(fallbackPromptShown, true);
  assert.equal(deleteCalls.length, 2);
  assert.equal(deleteCalls[0].useTrash, true, 'First attempt must use trash');
  assert.equal(deleteCalls[1].useTrash, false, 'Second attempt must be permanent fallback after confirmation');
});

test('FileOpsController: batch deletion prompts for permanent fallback if trash fails; cancel preserves files', async () => {
  setupMockLocalStorage();
  setupMockDocument();

  const deleteCalls: { path: string; useTrash?: boolean }[] = [];

  const mockApi = () => ({
    deletePath: async (p: string, useTrash?: boolean) => {
      deleteCalls.push({ path: p, useTrash });
      if (useTrash) {
        return { ok: false, error: 'Failed to move to trash' };
      }
      return { ok: true };
    },
    pathJoin: async (dir: string, base: string) => `${dir}/${base}`,
  });

  const state = new AppState();
  state.active = 'left';
  state.left.path = '/mnt/external';
  state.config.useTrash = true;
  state.left.activeTab.selectedBases = new Set(['file1.txt', 'file2.txt']);

  const controller = new FileOpsController({
    state,
    api: mockApi,
    setStatus: () => {},
    refreshAll: async () => {},
    focusActiveList: () => {},
    getFilteredSelection: () => ({ vis: [] }),
    otherSide: (s) => (s === 'left' ? 'right' : 'left'),
    loadDir: async () => {},
    fullPath: async (_p, item) => item?.base || '',
  });

  (controller as any).confirmDelete = async () => true;
  let fallbackPromptShown = false;
  (controller as any).askPermanentFallback = async (summary: string) => {
    fallbackPromptShown = true;
    assert.ok(summary.includes('2 items'));
    return false; // User cancels fallback
  };

  await controller.beginDelete({ permanent: false });

  assert.equal(fallbackPromptShown, true);
  // Both attempted trash, both failed, user canceled fallback, so no permanent unlink calls
  assert.equal(deleteCalls.length, 2);
  assert.ok(deleteCalls.every((c) => c.useTrash === true), 'Should only have attempted trash, never permanent unlink');
});
