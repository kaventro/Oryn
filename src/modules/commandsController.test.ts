// src/modules/commandsController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { CommandsController } from './commandsController.ts';
import { AppState } from './stateModels.ts';

function createMockElement(id: string) {
  const classes = new Set<string>();
  return {
    id,
    classList: {
      contains: (c: string) => classes.has(c),
      add: (c: string) => classes.add(c),
      remove: (c: string) => classes.delete(c),
      toggle: (c: string, force?: boolean) => {
        if (force !== undefined) {
          if (force) classes.add(c); else classes.delete(c);
        } else {
          if (classes.has(c)) classes.delete(c); else classes.add(c);
        }
      },
    },
    textContent: '',
    innerHTML: '',
    value: '',
    style: {},
    focus: () => {},
    replaceChildren: () => {},
    append: () => {},
    appendChild: () => {},
    onclick: null as any,
    onkeydown: null as any,
  };
}

test('CommandsController openViewer shows Edit button for text files and allows saving', async () => {
  const state = new AppState();
  const mockOverlay = createMockElement('viewer-overlay');
  mockOverlay.classList.add('hidden');
  const mockContent = createMockElement('viewer-content');
  const mockEditor = createMockElement('viewer-editor');
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

  const elements: Record<string, any> = {
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

  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
    addEventListener: () => {},
    removeEventListener: () => {},
  };

  let savedPath = '';
  let savedContent = '';

  const mockApi = {
    readFileText: async (p: string) => '{\n  "name": "Oswin"\n}',
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

  // Open JSON file
  await controller.openViewer('/path/to/config.json', { base: 'config.json', isDir: false });

  assert.equal(mockOverlay.classList.contains('hidden'), false, 'Overlay should be visible');
  assert.equal(mockEdit.classList.contains('hidden'), false, 'Edit button should be visible for text/json');

  // Trigger Edit
  mockEdit.onclick();
  assert.equal(mockContent.classList.contains('hidden'), true, 'Content should be hidden in edit mode');
  assert.equal(mockEditor.classList.contains('hidden'), false, 'Editor textarea should be visible');
  assert.equal(mockEditor.value, '{\n  "name": "Oswin"\n}');

  // Modify content and save
  mockEditor.value = '{\n  "name": "Oswin Updated"\n}';
  await mockSave.onclick();

  assert.equal(savedPath, '/path/to/config.json');
  assert.equal(savedContent, '{\n  "name": "Oswin Updated"\n}');
  assert.equal(mockEditor.classList.contains('hidden'), true, 'Editor should be hidden after save');
  assert.equal(mockContent.classList.contains('hidden'), false, 'Content should be visible after save');
});

test('CommandsController openViewer hides Edit button for image files', async () => {
  const state = new AppState();
  const mockOverlay = createMockElement('viewer-overlay');
  mockOverlay.classList.add('hidden');
  const mockContent = createMockElement('viewer-content');
  const mockEditor = createMockElement('viewer-editor');
  const mockTitle = createMockElement('viewer-title');
  const mockClose = createMockElement('viewer-close');
  const mockMode = createMockElement('viewer-mode-btn');
  const mockEdit = createMockElement('viewer-edit-btn');
  mockEdit.classList.add('hidden');
  const mockSave = createMockElement('viewer-save-btn');
  const mockCancel = createMockElement('viewer-cancel-edit-btn');
  const mockStatus = createMockElement('viewer-status-hint');

  const elements: Record<string, any> = {
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

  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
    createElement: (tag: string) => createMockElement(tag),
    addEventListener: () => {},
    removeEventListener: () => {},
  };

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

  // Open PNG file
  await controller.openViewer('/path/to/photo.png', { base: 'photo.png', isDir: false });

  assert.equal(mockOverlay.classList.contains('hidden'), false, 'Overlay should be visible');
  assert.equal(mockEdit.classList.contains('hidden'), true, 'Edit button MUST be hidden for image files');
});
