import assert from 'node:assert/strict';
import test from 'node:test';

import { TerminalDrawerController, ansiToHtml } from './terminalDrawerController.ts';

test('ansiToHtml escapes markup and converts supported color sequences', () => {
  assert.equal(ansiToHtml('<b>A & B</b>'), '&lt;b&gt;A &amp; B&lt;/b&gt;');
  assert.equal(ansiToHtml('\x1b[31mred\x1b[0m'), '<span style="color:#ff453a;">red</span>');
});

test('terminal drawer streams a persistent PTY session and forwards terminal input', async () => {
  const writes: string[] = [];
  const sent: Array<{ sessionId: string; data: string }> = [];
  const resized: Array<{ sessionId: string; cols: number; rows: number }> = [];
  let outputListener: ((id: string, data: string) => void) | undefined;
  let exitListener: ((id: string) => void) | undefined;
  let inputListener: ((data: string) => void) | undefined;
  let clipboard = '';
  let status = '';
  let fitCount = 0;
  const terminal = {
    cols: 82,
    rows: 26,
    open() {},
    loadAddon() {},
    onData(callback: (data: string) => void) { inputListener = callback; },
    write(data: string) { writes.push(data); },
    writeln(data: string) { writes.push(`${data}\n`); },
    focus() {},
    clear() { writes.length = 0; },
    getSelection: () => 'selected text',
    buffer: { active: { length: 0, getLine: () => undefined } },
  } as any;
  const fitAddon = { fit() { fitCount += 1; } } as any;
  const classes = new Set(['hidden']);
  const drawer = {
    classList: { add: (name: string) => classes.add(name), remove: (name: string) => classes.delete(name) },
    setAttribute() {},
    style: {} as Record<string, string>,
    offsetHeight: 280,
    addEventListener() {},
  };
  const host = { addEventListener() {} };
  const cwd = { textContent: '', title: '' };
  const statusEl = { textContent: '' };
  const handle = { classList: { add() {}, remove() {} }, addEventListener() {} };
  (globalThis as any).document = {
    getElementById(id: string) {
      return ({
        'terminal-drawer': drawer,
        'terminal-output': host,
        'terminal-cwd': cwd,
        'terminal-status': statusEl,
        'terminal-resize-handle': handle,
      } as Record<string, any>)[id] || null;
    },
    body: { style: {} },
  };
  (globalThis as any).window = { addEventListener() {}, removeEventListener() {}, innerHeight: 800 };
  (globalThis as any).ResizeObserver = class { observe() {} };
  (globalThis as any).localStorage = { getItem: () => null, setItem() {} };

  const api = {
    terminalListen: async (onOutput: typeof outputListener, onExit: typeof exitListener) => {
      outputListener = onOutput;
      exitListener = onExit;
      return () => {};
    },
    terminalStart: async (path: string, cols: number, rows: number) => {
      assert.equal(path, '/workspace');
      assert.equal(cols, 82);
      assert.equal(rows, 26);
      return 'pty-1';
    },
    terminalWrite: async (sessionId: string, data: string) => { sent.push({ sessionId, data }); },
    terminalResize: async (sessionId: string, cols: number, rows: number) => { resized.push({ sessionId, cols, rows }); },
    clipboardWrite: async (value: string) => { clipboard = value; },
  };
  const controller = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/workspace' } } as any,
    api: () => api,
    setStatus: (message) => { status = message; },
    focusActiveList() {},
    terminalFactory: () => ({ terminal, fitAddon }),
  });

  await controller.runCommand('top');
  assert.equal(controller.isRunning, true, `${status}: ${writes.join('')}`);
  assert.equal(sent.at(-1)?.data, 'top\r');
  inputListener?.('q');
  await Promise.resolve();
  assert.deepEqual(sent.at(-1), { sessionId: 'pty-1', data: 'q' });

  outputListener?.('pty-1', 'live output');
  assert.ok(writes.includes('live output'));
  controller.show();
  assert.equal(fitCount > 0, true);
  assert.deepEqual(resized.at(-1), { sessionId: 'pty-1', cols: 82, rows: 26 });

  await controller.copyOutput();
  assert.equal(clipboard, 'selected text');

  controller.appendOutput('info log');
  controller.appendOutput('error log', true);
  controller.appendOutput('cmd log', false, true);
  assert.ok(writes.some(w => w.includes('info log')));
  assert.ok(writes.some(w => w.includes('error log')));
  assert.ok(writes.some(w => w.includes('cmd log')));

  controller.clear();
  assert.equal(writes.length, 0);

  controller.hide();
  assert.equal(controller.isOpen, false);
  assert.ok(classes.has('hidden'));

  controller.toggle('/custom/dir');
  assert.equal(controller.isOpen, true);
  assert.equal(controller.cwd, '/custom/dir');
  assert.equal(cwd.textContent, '/custom/dir');

  controller.toggle();
  assert.equal(controller.isOpen, false);

  exitListener?.('pty-1');
  assert.equal(controller.isRunning, false);
  assert.equal(status, 'Shell exited');
});

test('terminal drawer handles external terminal launch and setup buttons', async () => {
  let externalLaunched = '';
  let statusMsg = '';
  const registeredEvents: Record<string, Function> = {};
  const btnClose = { addEventListener(ev: string, fn: Function) { registeredEvents['close:' + ev] = fn; } };
  const btnClear = { addEventListener(ev: string, fn: Function) { registeredEvents['clear:' + ev] = fn; } };
  const btnCopy = { addEventListener(ev: string, fn: Function) { registeredEvents['copy:' + ev] = fn; } };
  const btnExternal = { addEventListener(ev: string, fn: Function) { registeredEvents['external:' + ev] = fn; } };
  const host = { addEventListener(ev: string, fn: Function) { registeredEvents['host:' + ev] = fn; } };

  (globalThis as any).document = {
    getElementById(id: string) {
      if (id === 'terminal-close-btn') return btnClose;
      if (id === 'terminal-clear-btn') return btnClear;
      if (id === 'terminal-copy-btn') return btnCopy;
      if (id === 'terminal-external-btn') return btnExternal;
      if (id === 'terminal-output') return host;
      if (id === 'terminal-status') return { textContent: '' };
      return null;
    },
    body: { style: {} },
  };

  const api = {
    openTerminal: async (path: string) => { externalLaunched = path; },
    getHome: async () => '/home/user',
  };

  const controller = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/home/user/project' } } as any,
    api: () => api,
    setStatus: (msg) => { statusMsg = msg; },
    focusActiveList() {},
  });

  controller.setup();
  assert.ok(registeredEvents['close:click']);
  assert.ok(registeredEvents['clear:click']);
  assert.ok(registeredEvents['copy:click']);
  assert.ok(registeredEvents['external:click']);

  await controller.openExternalTerminal();
  assert.equal(externalLaunched, '/home/user/project');
  assert.ok(statusMsg.includes('Launched external terminal'));

  // Test fallback to shellExec when openTerminal is not a function
  let shellExecCmd = '';
  let shellExecCwd = '';
  controller.api = () => ({
    shellExec: async (cmd: string, cwd: string) => {
      shellExecCmd = cmd;
      shellExecCwd = cwd;
    },
    getHome: async () => '/home/fallback',
  });
  controller.cwd = '~';
  await controller.openExternalTerminal();
  assert.equal(shellExecCmd, 'open -a Terminal .');
  assert.equal(shellExecCwd, '/home/fallback');

  // Test error handling when getHome and launch fail
  controller.api = () => ({
    getHome: async () => { throw new Error('Home not found'); },
    shellExec: async () => { throw new Error('Shell launch failed'); },
  });
  controller.cwd = '~';
  await controller.openExternalTerminal();
  assert.ok(statusMsg.includes('Failed to open external terminal'));

  // Test empty command early return
  await controller.runCommand('');
  await controller.runCommand('   ');
});

test('terminal drawer copyOutput falls back to readVisibleBuffer when selection is empty', async () => {
  let copiedText = '';
  const lines = ['First line', 'Second line'];
  const terminal = {
    cols: 80,
    rows: 24,
    open() {},
    loadAddon() {},
    onData() {},
    write() {},
    focus() {},
    getSelection: () => '',
    buffer: {
      active: {
        length: lines.length,
        getLine: (i: number) => ({ translateToString: () => lines[i] }),
      },
    },
  } as any;
  const statusEl = { textContent: '' };
  (globalThis as any).document = {
    getElementById(id: string) {
      if (id === 'terminal-status') return statusEl;
      if (id === 'terminal-drawer') return { classList: { remove() {}, add() {} }, setAttribute() {}, style: {} };
      if (id === 'terminal-output') return {};
      return null;
    },
  };

  const controller = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/home' } } as any,
    api: () => ({
      clipboardWrite: async (t: string) => { copiedText = t; },
    }),
    setStatus: () => {},
    focusActiveList() {},
    terminalFactory: () => ({ terminal, fitAddon: { fit() {} } as any }),
  });

  controller.show();
  await controller.copyOutput();
  assert.equal(copiedText, 'First line\nSecond line');
  assert.equal(statusEl.textContent, '✓ Copied');
});

test('terminal drawer handles unavailable PTY backend and start errors', async () => {
  const writes: string[] = [];
  let statusText = '';
  const terminal = {
    cols: 80,
    rows: 24,
    open() {},
    loadAddon() {},
    onData() {},
    write: (d: string) => { writes.push(d); },
    writeln: (d: string) => { writes.push(d); },
    focus() {},
  } as any;

  (globalThis as any).document = {
    getElementById(id: string) {
      if (id === 'terminal-output') return {};
      if (id === 'terminal-drawer') return { classList: { remove() {}, add() {} }, setAttribute() {}, style: {} };
      return null;
    },
  };

  // 1. Backend without terminalStart
  const controllerUnavailable = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/home' } } as any,
    api: () => ({}),
    setStatus: (s) => { statusText = s; },
    focusActiveList() {},
    terminalFactory: () => ({ terminal, fitAddon: { fit() {} } as any }),
  });
  controllerUnavailable.show();
  await new Promise(r => setTimeout(r, 20));
  assert.ok(writes.some(w => w.includes('PTY is unavailable')));

  // 2. terminalStart throws error
  const controllerError = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/home' } } as any,
    api: () => ({
      terminalListen: async () => () => {},
      terminalStart: async () => { throw new Error('Spawn failed'); },
    }),
    setStatus: (s) => { statusText = s; },
    focusActiveList() {},
    terminalFactory: () => ({ terminal, fitAddon: { fit() {} } as any }),
  });
  controllerError.show();
  await new Promise(r => setTimeout(r, 20));
  assert.equal(statusText, 'Terminal error');
});

test('terminal drawer handles pending events and resize drag handle', async () => {
  let outputCallback: Function = () => {};
  let exitCallback: Function = () => {};
  const writes: string[] = [];
  let status = '';
  const terminal = {
    cols: 80,
    rows: 24,
    open() {},
    loadAddon() {},
    onData() {},
    write: (d: string) => { writes.push(d); },
    focus() {},
  } as any;

  let resizeMouseDown: Function = () => {};
  const windowListeners: Record<string, Function> = {};
  const drawerEl = {
    classList: { remove() {}, add() {} },
    setAttribute() {},
    style: { height: '200px' },
    offsetHeight: 200,
  };

  (globalThis as any).document = {
    getElementById(id: string) {
      if (id === 'terminal-output') return {};
      if (id === 'terminal-drawer') return drawerEl;
      if (id === 'terminal-resize-handle') return {
        classList: { add() {}, remove() {} },
        addEventListener: (ev: string, fn: Function) => {
          if (ev === 'mousedown') resizeMouseDown = fn;
        },
      };
      return null;
    },
    body: { style: {} },
  };
  (globalThis as any).window = {
    addEventListener: (ev: string, fn: Function) => { windowListeners[ev] = fn; },
    removeEventListener: (ev: string) => { delete windowListeners[ev]; },
    innerHeight: 1000,
  };

  const controller = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/home' } } as any,
    api: () => ({
      terminalListen: async (out: Function, ext: Function) => {
        outputCallback = out;
        exitCallback = ext;
        // emit output and exit BEFORE terminalStart resolves
        out('sess-queued', 'early output');
        ext('sess-queued');
        return () => {};
      },
      terminalStart: async () => 'sess-queued',
      terminalResize: async () => {},
    }),
    setStatus: (s) => { status = s; },
    focusActiveList() {},
    terminalFactory: () => ({ terminal, fitAddon: { fit() {} } as any }),
  });

  controller.setupResizeHandle();
  assert.ok(typeof resizeMouseDown === 'function');

  // Trigger drag resize
  resizeMouseDown({ clientY: 500 } as MouseEvent);
  assert.ok(windowListeners['mousemove']);
  assert.ok(windowListeners['mouseup']);

  windowListeners['mousemove']({ clientY: 450 } as MouseEvent);
  windowListeners['mouseup']({} as MouseEvent);

  controller.show();
  await new Promise(r => setTimeout(r, 20));
  assert.ok(writes.includes('early output'));
  assert.equal(status, 'Shell exited');
});

test('startTerminal cleans up listener and pending queues when PTY startup fails without active session', async () => {
  let unlistenCalled = false;
  const writes: string[] = [];
  const terminal = {
    cols: 80,
    rows: 24,
    open() {},
    loadAddon() {},
    onData() {},
    write: (d: string) => { writes.push(d); },
    writeln: (d: string) => { writes.push(d); },
    focus() {},
  } as any;
  (globalThis as any).document = {
    getElementById: (id: string) => (id === 'terminal-output' ? {} : null),
    body: { style: {} },
  };
  const controller = new TerminalDrawerController({
    state: { active: 'left', left: { path: '/home' } } as any,
    api: () => ({
      terminalListen: async (out: Function, ext: Function) => {
        out('other-id', 'stray output');
        ext('other-id');
        return () => { unlistenCalled = true; };
      },
      terminalStart: async () => { throw new Error('PTY spawn failed'); },
    }),
    setStatus: () => {},
    focusActiveList: () => {},
    terminalFactory: () => ({ terminal, fitAddon: { fit() {} } as any }),
  });

  (globalThis as any).window = { addEventListener() {}, removeEventListener() {}, innerHeight: 800 };
  (globalThis as any).ResizeObserver = class { observe() {} disconnect() {} };
  await controller.runCommand('test');

  assert.equal(unlistenCalled, true);
  assert.equal((controller as any).unlisten, null);
  assert.deepEqual((controller as any).pendingOutput, []);
  assert.equal((controller as any).pendingExits.size, 0);
  assert.ok(writes.some(w => w.includes('PTY spawn failed')));
});

