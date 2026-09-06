// src/modules/checksumController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { ChecksumController } from './checksumController.ts';

function createMockElement(initialProps: Record<string, any> = {}): any {
  const classes = new Set<string>();
  const listeners: Record<string, Function[]> = {};
  return {
    value: '',
    textContent: '',
    className: '',
    ...initialProps,
    classList: {
      add(c: string) { classes.add(c); },
      remove(c: string) { classes.delete(c); },
      contains(c: string) { return classes.has(c); },
    },
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    click(target?: any) {
      (listeners['click'] || []).forEach((fn) => fn({ target: target || this, stopPropagation: () => {} }));
    },
    input(val: string) {
      this.value = val;
      (listeners['input'] || []).forEach((fn) => fn({ target: this }));
    },
  };
}

function setupDom(elements: Record<string, any>) {
  const origDoc = globalThis.document;
  (globalThis as any).document = {
    getElementById: (id: string) => elements[id] || null,
  };
  return () => {
    globalThis.document = origDoc;
  };
}

test('ChecksumController openForPath handles empty and valid paths', async () => {
  const overlay = createMockElement();
  const fnEl = createMockElement();
  const pathEl = createMockElement();
  const szEl = createMockElement();
  const shaEl = createMockElement();
  const md5El = createMockElement();
  const verifyInputEl = createMockElement();
  const verifyResEl = createMockElement();

  const elements = {
    'checksum-overlay': overlay,
    'checksum-filename': fnEl,
    'checksum-path': pathEl,
    'checksum-size': szEl,
    'checksum-sha256': shaEl,
    'checksum-md5': md5El,
    'checksum-verify-input': verifyInputEl,
    'checksum-verify-result': verifyResEl,
  };
  const cleanup = setupDom(elements);

  let checksumResult: any = {
    ok: true,
    size: 2048,
    sha256: 'ABCD1234SHA256',
    md5: 'EF01234MD5',
  };

  const controller = new ChecksumController({
    api: () => ({
      fsChecksum: async (_p: string) => checksumResult,
    }),
    state: {} as any,
    setStatus: () => {},
    focusActiveList: () => {},
  });

  try {
    // Empty path
    await controller.openForPath();
    assert.equal(overlay.classList.contains('hidden'), false); // not touched

    // Valid path
    await controller.openForPath('/path/to/sample.txt');
    assert.equal(fnEl.textContent, 'sample.txt');
    assert.equal(pathEl.textContent, '/path/to/sample.txt');
    assert.equal(controller.currentSha256, 'ABCD1234SHA256');
    assert.equal(controller.currentMd5, 'EF01234MD5');
    assert.equal(shaEl.textContent, 'ABCD1234SHA256');
    assert.equal(md5El.textContent, 'EF01234MD5');
    assert.ok(szEl.textContent.includes('2.0 kB'));

    // Failed response
    checksumResult = { ok: false };
    await controller.openForPath('/path/to/failed.txt');
    assert.equal(shaEl.textContent, 'Failed to compute checksum.');

    // Exception
    checksumResult = null;
    await controller.openForPath('/path/to/error.txt');
    assert.equal(shaEl.textContent, 'Failed to compute checksum.');
  } finally {
    cleanup();
  }
});

test('ChecksumController openForPath handles exception throwing', async () => {
  const shaEl = createMockElement();
  const elements = {
    'checksum-sha256': shaEl,
  };
  const cleanup = setupDom(elements);

  const controller = new ChecksumController({
    api: () => ({
      fsChecksum: async () => {
        throw new Error('Permission denied');
      },
    }),
    state: {} as any,
    setStatus: () => {},
    focusActiveList: () => {},
  });

  try {
    await controller.openForPath('/secret.txt');
    assert.equal(shaEl.textContent, 'Permission denied');
  } finally {
    cleanup();
  }
});

test('ChecksumController hide hides overlay and focuses active list', () => {
  const overlay = createMockElement();
  const cleanup = setupDom({ 'checksum-overlay': overlay });
  let focused = false;

  const controller = new ChecksumController({
    api: () => ({}),
    state: {} as any,
    setStatus: () => {},
    focusActiveList: () => { focused = true; },
  });

  try {
    controller.hide();
    assert.ok(overlay.classList.contains('hidden'));
    assert.equal(focused, true);
  } finally {
    cleanup();
  }
});

test('ChecksumController verifyInput matches SHA-256, MD5, mismatch and empty', () => {
  const verifyResEl = createMockElement();
  const cleanup = setupDom({ 'checksum-verify-result': verifyResEl });

  const controller = new ChecksumController({
    api: () => ({}),
    state: {} as any,
    setStatus: () => {},
    focusActiveList: () => {},
  });
  controller.currentSha256 = 'abc123def';
  controller.currentMd5 = 'md5secret';

  try {
    // Empty
    controller.verifyInput('');
    assert.equal(verifyResEl.textContent, '');
    assert.equal(verifyResEl.className, 'checksum-verify-result');

    // SHA-256 match case insensitive
    controller.verifyInput(' ABC123DEF ');
    assert.equal(verifyResEl.textContent, '✓ Matches SHA-256');
    assert.ok(verifyResEl.className.includes('--match'));

    // MD5 match
    controller.verifyInput('md5secret');
    assert.equal(verifyResEl.textContent, '✓ Matches MD5');
    assert.ok(verifyResEl.className.includes('--match'));

    // Mismatch
    controller.verifyInput('randomstuff');
    assert.equal(verifyResEl.textContent, '✗ Does not match');
    assert.ok(verifyResEl.className.includes('--mismatch'));
  } finally {
    cleanup();
  }
});

test('ChecksumController setup wires close, backdrop, copy, and input', async () => {
  const overlay = createMockElement();
  const closeBtn = createMockElement();
  const copyShaBtn = createMockElement();
  const copyMd5Btn = createMockElement();
  const verifyInput = createMockElement();
  const verifyResult = createMockElement();

  const elements = {
    'checksum-overlay': overlay,
    'checksum-close': closeBtn,
    'checksum-copy-sha256': copyShaBtn,
    'checksum-copy-md5': copyMd5Btn,
    'checksum-verify-input': verifyInput,
    'checksum-verify-result': verifyResult,
  };
  const cleanup = setupDom(elements);

  let clipboardVal = '';
  let statusMsg = '';
  let focused = false;

  const controller = new ChecksumController({
    api: () => ({
      clipboardWrite: async (txt: string) => { clipboardVal = txt; },
    }),
    state: {} as any,
    setStatus: (msg: string) => { statusMsg = msg; },
    focusActiveList: () => { focused = true; },
  });

  controller.currentSha256 = 'sha256_val';
  controller.currentMd5 = 'md5_val';
  controller.setup();

  try {
    // Click close
    closeBtn.click();
    assert.ok(overlay.classList.contains('hidden'));
    assert.equal(focused, true);

    // Click backdrop
    overlay.click(overlay);
    assert.ok(overlay.classList.contains('hidden'));

    // Copy SHA256
    await copyShaBtn.click();
    assert.equal(clipboardVal, 'sha256_val');
    assert.equal(statusMsg, 'SHA-256 copied to clipboard.');

    // Copy MD5
    await copyMd5Btn.click();
    assert.equal(clipboardVal, 'md5_val');
    assert.equal(statusMsg, 'MD5 copied to clipboard.');

    // Input verify
    verifyInput.input('sha256_val');
    assert.equal(verifyResult.textContent, '✓ Matches SHA-256');
  } finally {
    cleanup();
  }
});
