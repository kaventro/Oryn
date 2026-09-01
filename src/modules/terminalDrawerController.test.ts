// src/modules/terminalDrawerController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { TerminalDrawerController } from './terminalDrawerController.ts';

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

test('TerminalDrawerController open, toggle, and built-ins', async () => {
  let navigatedTo: any = null;
  const executedCommands: Array<{ cmd: string; cwd: string }> = [];

  const mockApi = {
    shellExec: async (cmd: string, cwd: string) => {
      executedCommands.push({ cmd, cwd });
      return { ok: true, code: 0, stdout: 'sample output\n', stderr: '' };
    },
    readDir: async (path: string) => {
      if (path === '/valid/dir') {
        return { ok: true, items: [{ base: 'sub', display: 'sub' }] };
      }
      throw new Error('Not found');
    },
    getHome: async () => '/Users/test',
    clipboardWrite: async () => {},
  };

  const drawerEl = {
    classList: {
      _classes: new Set(['hidden']),
      add(c: string) { this._classes.add(c); },
      remove(c: string) { this._classes.delete(c); },
      contains(c: string) { return this._classes.has(c); },
    },
    setAttribute() {},
    style: {},
  };

  const outputEl: any = {
    children: [],
    appendChild(el: any) { this.children.push(el); },
    replaceChildren() { this.children = []; },
    scrollTop: 0,
    scrollHeight: 100,
    innerText: 'sample output',
  };

  const cwdEl = { textContent: '', title: '' };
  const statusEl = { textContent: '' };
  const inputEl = {
    value: '',
    focus() {},
    select() {},
  };

  globalThis.document = {
    getElementById(id: string) {
      if (id === 'terminal-drawer') return drawerEl as any;
      if (id === 'terminal-output') return outputEl as any;
      if (id === 'terminal-cwd') return cwdEl as any;
      if (id === 'terminal-status') return statusEl as any;
      if (id === 'terminal-input') return inputEl as any;
      return null;
    },
    createElement(tag: string) {
      return { className: '', textContent: '', appendChild() {} } as any;
    },
  } as any;

  let focusedList = false;
  const controller = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/initial/dir' } },
    api: () => mockApi as any,
    setStatus: () => {},
    focusActiveList: () => { focusedList = true; },
    navigateTo: async (side: string, path: string) => { navigatedTo = { side, path }; },
  });

  // Test Show
  controller.show('/initial/dir');
  assert.equal(controller.isOpen, true);
  assert.equal(drawerEl.classList.contains('hidden'), false);
  assert.equal(cwdEl.textContent, '/initial/dir');

  // Test run regular command
  await controller.runCommand('git status');
  assert.equal(executedCommands.length, 1);
  assert.equal(executedCommands[0].cmd, 'git status');
  assert.equal(executedCommands[0].cwd, '/initial/dir');

  // Test built-in cd
  await controller.runCommand('cd /valid/dir');
  assert.equal(controller.cwd, '/valid/dir');
  assert.equal(cwdEl.textContent, '/valid/dir');
  assert.deepEqual(navigatedTo, { side: 'left', path: '/valid/dir' });

  // Test built-in pwd
  await controller.runCommand('pwd');
  assert.equal(outputEl.children.length > 0, true);

  // Test Hide
  controller.hide();
  assert.equal(controller.isOpen, false);
  assert.equal(drawerEl.classList.contains('hidden'), true);
  assert.equal(focusedList, true);
});
