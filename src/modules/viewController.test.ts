// src/modules/viewController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { ViewController } from './viewController.ts';

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

test('ViewController initializes with default list mode', () => {
  const vc = new ViewController();
  assert.equal(vc.getMode(), 'list');
});

test('ViewController changes modes and triggers callbacks', () => {
  let changedTo = '';
  const vc = new ViewController({
    onModeChange: (mode: string) => {
      changedTo = mode;
    },
  });

  vc.setMode('grid');
  assert.equal(vc.getMode(), 'grid');
  assert.equal(changedTo, 'grid');

  vc.setMode('columns');
  assert.equal(vc.getMode(), 'columns');
  assert.equal(changedTo, 'columns');

  vc.setMode('list');
  assert.equal(vc.getMode(), 'list');
  assert.equal(changedTo, 'list');
});
