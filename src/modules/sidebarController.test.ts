// src/modules/sidebarController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { SidebarController } from './sidebarController.ts';

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

test('SidebarController pins and unpins folders', async () => {
  mockStorage['Oryn.pinnedFolders'] = '[]';
  const statusMessages: string[] = [];
  const controller = new SidebarController({
    api: () => ({
      getSystemLocations: async () => ({
        os: 'windows',
        username: 'TestUser',
        home: 'C:\\Users\\TestUser',
        desktop: 'C:\\Users\\TestUser\\Desktop',
        documents: 'C:\\Users\\TestUser\\Documents',
        downloads: 'C:\\Users\\TestUser\\Downloads',
        drives: [
          { name: 'Local Disk (C:)', mountPoint: 'C:\\', totalSpace: 1000, availableSpace: 500, isRemovable: false, fileSystem: 'NTFS' },
          { name: 'Data (D:)', mountPoint: 'D:\\', totalSpace: 2000, availableSpace: 1500, isRemovable: false, fileSystem: 'NTFS' },
        ],
      }),
    }),
    state: { active: 'left', left: { path: 'C:\\Users\\TestUser\\Projects' } },
    setStatus: (msg: string) => statusMessages.push(msg),
    navigateTo: async () => { },
    focusActiveList: () => { },
  });

  await controller.setup();
  assert.equal(controller.isPinned('C:\\Users\\TestUser\\Projects'), false);

  controller.pinFolder('C:\\Users\\TestUser\\Projects', 'My Projects');
  assert.equal(controller.isPinned('C:\\Users\\TestUser\\Projects'), true);
  assert.equal(controller.pinnedFolders.length, 1);
  assert.equal(controller.pinnedFolders[0].name, 'My Projects');

  // Unpin
  controller.unpinFolder('C:\\Users\\TestUser\\Projects');
  assert.equal(controller.isPinned('C:\\Users\\TestUser\\Projects'), false);
  assert.equal(controller.pinnedFolders.length, 0);
});

test('SidebarController togglePin toggles state', async () => {
  mockStorage['Oryn.pinnedFolders'] = '[]';
  const controller = new SidebarController({
    api: () => ({}),
    state: { active: 'left', left: { path: '/Users/test' } },
    setStatus: () => { },
    navigateTo: async () => { },
    focusActiveList: () => { },
  });

  await controller.setup();
  controller.togglePin('/Users/test/music', 'Music');
  assert.equal(controller.isPinned('/Users/test/music'), true);

  controller.togglePin('/Users/test/music');
  assert.equal(controller.isPinned('/Users/test/music'), false);
});

test('SidebarController setupCollapsibleSections handles collapse and persistence', () => {
  mockStorage['Oryn.sidebar.tags.collapsed'] = 'true';
  const listeners: Record<string, any> = {};
  const classes = new Set<string>();
  const header = {
    addEventListener: (evt: string, fn: any) => { listeners[evt] = fn; },
    closest: () => null,
  };
  const section = {
    dataset: { section: 'tags' },
    classList: {
      add: (c: string) => classes.add(c),
      remove: (c: string) => classes.delete(c),
      toggle: (c: string) => { if (classes.has(c)) classes.delete(c); else classes.add(c); },
      contains: (c: string) => classes.has(c),
    },
    querySelector: (sel: string) => sel === '.sidebar-section-header' ? header : null,
  };

  globalThis.document = {
    querySelectorAll: (sel: string) => sel === '.sidebar-section[data-section]' ? [section] : [],
    getElementById: () => null,
  } as any;

  const controller = new SidebarController({
    api: () => ({}),
    state: { active: 'left', left: { path: '/Users/test' } },
    setStatus: () => { },
    navigateTo: async () => { },
    focusActiveList: () => { },
  });

  controller.setupCollapsibleSections();
  assert.equal(section.classList.contains('collapsed'), true);

  // Click header to expand
  listeners['click']({ target: header });
  assert.equal(section.classList.contains('collapsed'), false);
  assert.equal(mockStorage['Oryn.sidebar.tags.collapsed'], 'false');

  // Click header to collapse again
  listeners['click']({ target: header });
  assert.equal(section.classList.contains('collapsed'), true);
  assert.equal(mockStorage['Oryn.sidebar.tags.collapsed'], 'true');
});
