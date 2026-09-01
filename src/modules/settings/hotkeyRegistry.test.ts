import test from 'node:test';
import assert from 'node:assert/strict';
import { HotkeyRegistry, ACTION_DEFINITIONS } from './hotkeyRegistry.ts';

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

test('HotkeyRegistry contains newFile action definition', () => {
  const newFileDef = ACTION_DEFINITIONS.find((a) => a.id === 'newFile');
  assert.ok(newFileDef, 'newFile action should be defined');
  assert.equal(newFileDef.defaultKey, 'Shift+F7');
  assert.equal(newFileDef.category, 'File Operations');
  assert.equal(newFileDef.icon, 'file-plus');
});

test('HotkeyRegistry resolves Shift+F7 to newFile and F7 to mkdir', () => {
  const registry = new HotkeyRegistry();

  const eventShiftF7 = {
    key: 'F7',
    shiftKey: true,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
  };
  assert.equal(registry.findActionForEvent(eventShiftF7), 'newFile');

  const eventF7 = {
    key: 'F7',
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
  };
  assert.equal(registry.findActionForEvent(eventF7), 'mkdir');

  const eventCtrlU = {
    key: 'u',
    shiftKey: false,
    ctrlKey: true,
    altKey: false,
    metaKey: false,
  };
  assert.equal(registry.findActionForEvent(eventCtrlU), 'swapPanels');

  const eventCtrlB = {
    key: 'b',
    shiftKey: false,
    ctrlKey: true,
    altKey: false,
    metaKey: false,
  };
  assert.equal(registry.findActionForEvent(eventCtrlB), 'branchView');

  const eventShiftF5 = {
    key: 'F5',
    shiftKey: true,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
  };
  assert.equal(registry.findActionForEvent(eventShiftF5), 'cloneFile');
});
