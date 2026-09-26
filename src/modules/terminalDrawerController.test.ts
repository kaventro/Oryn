// src/modules/terminalDrawerController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { TerminalDrawerController, ansiToHtml } from './terminalDrawerController.ts';

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

test('ansiToHtml escapes HTML entities and converts ANSI color & styling sequences', () => {
  // 1. Escapes HTML entities
  assert.equal(ansiToHtml('<b>Hello & Goodbye</b>'), '&lt;b&gt;Hello &amp; Goodbye&lt;/b&gt;');

  // 2. Bold and reset
  assert.equal(ansiToHtml('\x1b[1mBold\x1b[0m'), '<span style="font-weight:bold;">Bold</span>');

  // 3. Colors
  assert.equal(ansiToHtml('\x1b[31mRed\x1b[0m'), '<span style="color:#ff453a;">Red</span>');
  assert.equal(ansiToHtml('\x1b[32mGreen\x1b[0m'), '<span style="color:#30d158;">Green</span>');
  assert.equal(ansiToHtml('\x1b[33mYellow\x1b[0m'), '<span style="color:#ffd60a;">Yellow</span>');
  assert.equal(ansiToHtml('\x1b[34mBlue\x1b[0m'), '<span style="color:#0a84ff;">Blue</span>');
  assert.equal(ansiToHtml('\x1b[35mMagenta\x1b[0m'), '<span style="color:#bf5af2;">Magenta</span>');
  assert.equal(ansiToHtml('\x1b[36mCyan\x1b[0m'), '<span style="color:#64d2ff;">Cyan</span>');
  assert.equal(ansiToHtml('\x1b[90mGray\x1b[0m'), '<span style="color:#8e8e93;">Gray</span>');

  // 4. Strips unsupported ANSI codes
  assert.equal(ansiToHtml('\x1b[4mUnderline\x1b[38;5;200m256color\x1b[0m'), 'Underline256color</span>');
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
    style: {} as Record<string, string>,
  };

  const outputEl: any = {
    children: [] as any[],
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
      return { className: '', textContent: '', innerHTML: '', appendChild() {} } as any;
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

test('TerminalDrawerController appendOutput, toggle, copyOutput and openExternalTerminal', async () => {
  let copiedText = '';
  let statusMessage = '';
  let openTerminalPath = '';

  const mockApi: any = {
    clipboardWrite: async (t: string) => { copiedText = t; },
    openTerminal: async (p: string) => { openTerminalPath = p; },
    getHome: async () => '/Users/testuser',
    shellExec: async (_cmd?: string, _dir?: string) => ({ ok: true, code: 0, stdout: 'ok', stderr: '' }),
  };

  const lines: any[] = [];
  const outputEl: any = {
    children: lines,
    appendChild(l: any) { lines.push(l); return l; },
    replaceChildren() { lines.length = 0; },
    innerText: 'Line 1\nLine 2',
    scrollTop: 0,
    scrollHeight: 200,
  };

  const statusEl = { textContent: '' };
  const cwdEl = { textContent: '', title: '' };
  const drawerEl = {
    classList: {
      _classes: new Set(['hidden']),
      add(c: string) { this._classes.add(c); },
      remove(c: string) { this._classes.delete(c); },
      contains(c: string) { return this._classes.has(c); },
    },
    setAttribute() {},
    style: {} as Record<string, string>,
  };

  globalThis.document = {
    getElementById(id: string) {
      if (id === 'terminal-drawer') return drawerEl as any;
      if (id === 'terminal-output') return outputEl as any;
      if (id === 'terminal-status') return statusEl as any;
      if (id === 'terminal-cwd') return cwdEl as any;
      return null;
    },
    createElement(tag: string) {
      return { className: '', textContent: '', innerHTML: '' } as any;
    },
  } as any;

  localStorage.setItem('Oryn.terminalHeight', '350');

  const controller = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/home/dir' } },
    api: () => mockApi as any,
    setStatus: (msg: string) => { statusMessage = msg; },
    focusActiveList: () => {},
  });

  // 1. Toggle when closed -> opens
  controller.toggle();
  assert.equal(controller.isOpen, true);
  assert.equal(drawerEl.style.height, '350px');

  // 2. Toggle when open -> closes
  controller.toggle();
  assert.equal(controller.isOpen, false);

  // 3. appendOutput with ANSI escape codes
  controller.appendOutput('\x1b[32mSuccess\x1b[0m', false, false);
  const ansiLine = lines[lines.length - 1];
  assert.equal(ansiLine.className, 'terminal-line');
  assert.equal(ansiLine.innerHTML, '<span style="color:#30d158;">Success</span>');

  // 4. appendOutput without ANSI, with isErr and isCmd
  controller.appendOutput('Error text', true, true);
  const errLine = lines[lines.length - 1];
  assert.equal(errLine.className, 'terminal-line terminal-line--err terminal-line--cmd');
  assert.equal(errLine.textContent, 'Error text');

  // 5. copyOutput copies to clipboard
  await controller.copyOutput();
  assert.equal(copiedText, 'Line 1\nLine 2');
  assert.equal(statusMessage, 'Terminal output copied to clipboard.');
  assert.equal(statusEl.textContent, '✓ Copied');

  // 6. openExternalTerminal with custom openTerminal
  controller.cwd = '/custom/project';
  await controller.openExternalTerminal();
  assert.equal(openTerminalPath, '/custom/project');
  assert.equal(statusEl.textContent, '✓ Terminal opened');

  // 7. openExternalTerminal with fallback shellExec and ~ cwd
  delete (mockApi as any).openTerminal;
  let shellExecCmd = '';
  let shellExecDir = '';
  mockApi.shellExec = async (cmd: string, dir: string) => {
    shellExecCmd = cmd;
    shellExecDir = dir;
    return { ok: true, code: 0, stdout: '', stderr: '' };
  };
  controller.cwd = '~';
  await controller.openExternalTerminal();
  assert.equal(shellExecCmd, 'open -a Terminal .');
  assert.equal(shellExecDir, '/Users/testuser');

  // 8. openExternalTerminal handles failure
  mockApi.shellExec = async () => { throw new Error('Launch error'); };
  await controller.openExternalTerminal();
  assert.equal(statusEl.textContent, '✗ Launch failed');
  assert.ok(statusMessage.includes('Failed to open external terminal'));

  // 9. openExternalTerminal falls back to / when getHome fails
  mockApi.getHome = async () => { throw new Error('Cannot get home'); };
  let openedFallback = '';
  mockApi.shellExec = async (_cmd: string, dir: string) => {
    openedFallback = dir;
    return { ok: true, code: 0, stdout: '', stderr: '' };
  };
  controller.cwd = '';
  controller.state = { active: 'left', left: { path: '' } } as any;
  await controller.openExternalTerminal();
  assert.equal(openedFallback, '/');

  // 10. updateCwd falls back to ~ when no cwd or state path
  controller.cwd = null;
  controller.state = { active: 'left', left: {} } as any;
  controller.updateCwd();
  assert.equal(controller.cwd, '~');
  assert.equal(cwdEl.textContent, '~');
});

test('TerminalDrawerController runCommand handles help, clear, cd branches, relative paths, and errors', async () => {
  const outputs: string[] = [];
  const statusEl = { textContent: '' };
  let loadDirSide: string | null = null;

  const mockApi: any = {
    getHome: async () => '/home/user',
    statProps: async (p: string) => {
      if (p === '/home/user/docs' || p === '/home/user' || p === '/Users/test' || p === 'D:\\' || p === 'C:\\') {
        return { ok: true, props: { isDir: true } };
      }
      return { ok: false };
    },
    readDir: async (p: string) => {
      if (p === '/fallback/dir') {
        return { ok: true, items: [] };
      }
      return { ok: false };
    },
    shellExec: async (cmd: string) => {
      if (cmd === 'failing-cmd') {
        return { ok: false, code: 127, stdout: '', stderr: 'command not found' };
      }
      if (cmd === 'crash-cmd') {
        throw new Error('Fatal process spawn failure');
      }
      return { ok: true, code: 0, stdout: 'success', stderr: '' };
    },
  };

  const outputEl: any = {
    children: [],
    appendChild(el: any) {
      outputs.push(el.innerHTML || el.textContent || '');
      this.children.push(el);
    },
    replaceChildren() {
      outputs.length = 0;
      this.children.length = 0;
    },
    scrollTop: 0,
    scrollHeight: 100,
  };

  const inputEl = { value: 'initial' };

  globalThis.document = {
    getElementById(id: string) {
      if (id === 'terminal-output') return outputEl;
      if (id === 'terminal-status') return statusEl;
      if (id === 'terminal-input') return inputEl;
      return null;
    },
    createElement(tag: string) {
      return { className: '', textContent: '', innerHTML: '' } as any;
    },
  } as any;

  const controller = new TerminalDrawerController({
    state: { active: 'right', right: { path: '/home/user/docs' } },
    api: () => mockApi,
    setStatus: () => {},
    focusActiveList: () => {},
    loadDir: async (side: 'left' | 'right') => { loadDirSide = side; },
  });

  // 1. Empty command returns early
  await controller.runCommand('');
  await controller.runCommand('   ');

  // 2. clear / cls
  outputs.push('prior line');
  await controller.runCommand('clear');
  assert.equal(outputs.length, 0);

  // 3. help
  await controller.runCommand('help');
  assert.ok(outputs.some((o) => o.includes('Oryn Integrated Shell Commands')));
  assert.equal(statusEl.textContent, '✓ Done');

  // 4. cd - with OLDPWD not set
  await controller.runCommand('cd -');
  assert.ok(outputs.some((o) => o.includes('cd: OLDPWD not set')));
  assert.equal(statusEl.textContent, '✗ Error');

  // 5. cd ~ and cd ~/docs
  await controller.runCommand('cd ~');
  assert.equal(controller.cwd, '/home/user');
  assert.equal(loadDirSide, 'right');
  assert.equal(statusEl.textContent, '✓ Done');

  await controller.runCommand('cd ~/docs');
  assert.equal(controller.cwd, '/home/user/docs');

  // 6. cd - when previousCwd is set
  await controller.runCommand('cd -');
  assert.equal(controller.cwd, '/home/user');

  // 7. cd .. navigation
  controller.cwd = '/home/user/docs';
  await controller.runCommand('cd ..');
  assert.equal(controller.cwd, '/home/user');

  // cd .. from root
  controller.cwd = '/';
  await controller.runCommand('cd ..');
  assert.equal(controller.cwd, '/');

  // cd .. from Windows drive root
  controller.cwd = 'C:\\';
  await controller.runCommand('cd ..');
  assert.equal(controller.cwd, 'C:\\');

  // cd .. from Windows drive parent
  controller.cwd = 'C:\\Users';
  await controller.runCommand('cd ..');
  assert.equal(controller.cwd, 'C:\\');

  // 8. cd to Windows drive letter
  await controller.runCommand('cd D:');
  assert.equal(controller.cwd, 'D:\\');

  // 8b. cd with relative path
  controller.cwd = '/home/user';
  await controller.runCommand('cd docs');
  assert.equal(controller.cwd, '/home/user/docs');

  // 9. cd fallback via readDir
  await controller.runCommand('cd /fallback/dir');
  assert.equal(controller.cwd, '/fallback/dir');

  // 10. cd nonexistent directory
  await controller.runCommand('cd /nonexistent/nowhere');
  assert.ok(outputs.some((o) => o.includes('cd: no such file or directory: /nonexistent/nowhere')));
  assert.equal(statusEl.textContent, '✗ Error');

  // 11. Command with non-zero exit code
  await controller.runCommand('failing-cmd');
  assert.ok(outputs.some((o) => o.includes('command not found')));
  assert.equal(statusEl.textContent, '✗ Exit code 127');

  // 12. Command throwing exception
  await controller.runCommand('crash-cmd');
  assert.ok(outputs.some((o) => o.includes('Fatal process spawn failure')));
  assert.equal(statusEl.textContent, '✗ Error');
});

test('TerminalDrawerController setupResizeHandle, setup DOM listeners and keyboard shortcuts', async () => {
  const listeners: Record<string, Function> = {};
  const inputListeners: Record<string, Function> = {};
  let focusedInput = false;

  const handleEl = {
    classList: {
      _c: new Set<string>(),
      add(c: string) { this._c.add(c); },
      remove(c: string) { this._c.delete(c); },
    },
    addEventListener(evt: string, fn: any) { listeners[evt] = fn; },
  };

  const drawerEl = {
    offsetHeight: 200,
    style: {} as Record<string, string>,
    classList: {
      _c: new Set<string>(),
      add(c: string) { this._c.add(c); },
      remove(c: string) { this._c.delete(c); },
      contains(c: string) { return this._c.has(c); },
    },
    setAttribute() {},
    addEventListener(evt: string, fn: any) { listeners[`drawer_${evt}`] = fn; },
  };

  const inputEl = {
    value: '',
    focus() { focusedInput = true; },
    addEventListener(evt: string, fn: any) { inputListeners[evt] = fn; },
  };

  const buttonListeners: Record<string, Function> = {};
  const createMockButton = (id: string) => ({
    tagName: 'BUTTON',
    addEventListener(evt: string, fn: any) { buttonListeners[`${id}_${evt}`] = fn; },
    closest(sel: string) { return sel === 'button' ? this : null; },
  });

  const outputEl: any = {
    children: [],
    appendChild(c: any) { this.children.push(c); },
    replaceChildren() { this.children = []; },
    innerText: '',
  };

  (globalThis as any).document = {
    body: { style: {} },
    getElementById(id: string) {
      if (id === 'terminal-resize-handle') return handleEl as any;
      if (id === 'terminal-drawer') return drawerEl as any;
      if (id === 'terminal-input') return inputEl as any;
      if (id === 'terminal-output') return outputEl as any;
      if (id === 'terminal-status') return { textContent: '' } as any;
      if (id === 'terminal-cwd') return { textContent: '' } as any;
      if (id === 'terminal-close-btn') return createMockButton('close');
      if (id === 'terminal-clear-btn') return createMockButton('clear');
      if (id === 'terminal-copy-btn') return createMockButton('copy');
      if (id === 'terminal-external-btn') return createMockButton('external');
      return null;
    },
    createElement() { return { className: '', textContent: '', innerHTML: '' }; },
  };

  const windowListeners: Record<string, Function> = {};
  (globalThis as any).window = {
    innerHeight: 1000,
    addEventListener(evt: string, fn: any) { windowListeners[evt] = fn; },
    removeEventListener(evt: string) { delete windowListeners[evt]; },
  };

  const mockApi = {
    readDir: async () => ({
      items: [
        { base: 'notes.txt' },
        { base: 'Project Plan.pdf' },
        { base: 'photo1.png' },
        { base: 'photo2.png' },
      ],
    }),
    shellExec: async () => ({ ok: true, code: 0, stdout: 'done', stderr: '' }),
  };

  const controller = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/test' } },
    api: () => mockApi as any,
    setStatus: () => {},
    focusActiveList: () => {},
  });

  // Call setup
  controller.setup();

  // 1. Test Resize Handle
  listeners['mousedown']({ clientY: 500 });
  assert.ok(handleEl.classList._c.has('resizing'));
  windowListeners['mousemove']({ clientY: 450 }); // delta = 50 -> height 250
  assert.equal(drawerEl.style.height, '250px');
  windowListeners['mouseup']();
  assert.equal(handleEl.classList._c.has('resizing'), false);
  assert.equal(localStorage.getItem('Oryn.terminalHeight'), '200');

  // 2. Buttons click events
  controller.isOpen = true;
  buttonListeners['close_click']();
  assert.equal(controller.isOpen, false);

  buttonListeners['clear_click']();
  assert.equal(outputEl.children.length, 0);

  // 3. Drawer click focuses input
  listeners['drawer_click']({ target: { tagName: 'DIV', closest: () => null } });
  assert.equal(focusedInput, true);

  // 4. Input keydown: Escape hides
  controller.isOpen = true;
  await inputListeners['keydown']({ key: 'Escape', preventDefault() {} });
  assert.equal(controller.isOpen, false);

  // 5. Input keydown: Ctrl+C clears input
  inputEl.value = 'typed text';
  await inputListeners['keydown']({ key: 'c', ctrlKey: true, preventDefault() {} });
  assert.equal(inputEl.value, '');

  // 6. Input keydown: Ctrl+L clears output
  outputEl.children.push('some line');
  await inputListeners['keydown']({ key: 'l', ctrlKey: true, preventDefault() {} });
  assert.equal(outputEl.children.length, 0);

  // 7. Input keydown: Enter runs command and history tracking
  inputEl.value = 'echo 1';
  await inputListeners['keydown']({ key: 'Enter', preventDefault() {} });
  inputEl.value = 'echo 2';
  await inputListeners['keydown']({ key: 'Enter', preventDefault() {} });

  assert.deepEqual(controller.history, ['echo 1', 'echo 2']);

  // History ArrowUp
  await inputListeners['keydown']({ key: 'ArrowUp', preventDefault() {} });
  assert.equal(inputEl.value, 'echo 2');
  await inputListeners['keydown']({ key: 'ArrowUp', preventDefault() {} });
  assert.equal(inputEl.value, 'echo 1');

  // History ArrowDown
  await inputListeners['keydown']({ key: 'ArrowDown', preventDefault() {} });
  assert.equal(inputEl.value, 'echo 2');
  await inputListeners['keydown']({ key: 'ArrowDown', preventDefault() {} });
  assert.equal(inputEl.value, '');

  // 8. Tab completion: single file without space
  inputEl.value = 'cat not';
  await inputListeners['keydown']({ key: 'Tab', preventDefault() {} });
  assert.equal(inputEl.value, 'cat notes.txt');

  // Tab completion: single file with space (quotes added)
  inputEl.value = 'open Proj';
  await inputListeners['keydown']({ key: 'Tab', preventDefault() {} });
  assert.equal(inputEl.value, 'open "Project Plan.pdf"');

  // Tab completion: multiple files
  inputEl.value = 'ls pho';
  await inputListeners['keydown']({ key: 'Tab', preventDefault() {} });
  assert.ok(outputEl.children.some((c: any) => (c.textContent || '').includes('photo1.png')));
});
