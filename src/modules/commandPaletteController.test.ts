// src/modules/commandPaletteController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { CommandPaletteController } from './commandPaletteController.ts';

test('CommandPaletteController manages open, toggle and filter', () => {
  const mockOverlay = {
    classList: {
      _classes: new Set(['hidden']),
      contains(c: string) { return this._classes.has(c); },
      add(c: string) { this._classes.add(c); },
      remove(c: string) { this._classes.delete(c); },
    },
    setAttribute() {},
    addEventListener() {},
  };

  const mockInput = {
    value: '',
    focus() {},
    addEventListener() {},
  };

  const mockResults = {
    replaceChildren() {},
    appendChild(child: any) { (this.children as any[]).push(child); },
    querySelectorAll: () => [],
    children: [] as any[],
  };

  (globalThis as any).document = {
    getElementById(id: string) {
      if (id === 'command-palette-overlay') return mockOverlay;
      if (id === 'cp-input') return mockInput;
      if (id === 'cp-results-list') return mockResults;
      if (id === 'cp-counter') return { textContent: '' };
      if (id === 'cp-categories') return { addEventListener() {}, querySelectorAll: () => [] };
      return null;
    },
    createElement() {
      return {
        className: '',
        textContent: '',
        setAttribute() {},
        appendChild() {},
        append() {},
        addEventListener() {},
      };
    },
  };

  let executedCmd = '';
  const state = {
    active: 'left',
    left: {
      path: '/home/user/project',
      items: [
        { base: 'src', isDir: true },
        { base: 'package.json', isDir: false },
      ],
    },
    right: { path: '/tmp', items: [] },
  };

  const cp = new CommandPaletteController({
    api: () => ({}),
    state,
    commandsController: {
      runCommand: (cmd: string) => { executedCmd = cmd; },
    },
  });

  assert.equal(cp.isOpen(), false);
  cp.open();
  assert.equal(cp.isOpen(), true);

  cp.filter('new folder');
  cp.execute(0);
  assert.equal(executedCmd, 'mkdir');
  assert.equal(cp.isOpen(), false);
});
