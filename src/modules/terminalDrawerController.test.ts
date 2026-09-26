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
  exitListener?.('pty-1');
  assert.equal(controller.isRunning, false);
  assert.equal(status, 'Shell exited');
});
