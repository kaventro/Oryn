// src/modules/sysStatsController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { SysStatsController } from './sysStatsController.ts';

function createMockElement(initial: Record<string, any> = {}): any {
  const attrs: Record<string, string> = {};
  const listeners: Record<string, Function[]> = {};
  return {
    hidden: false,
    textContent: '',
    title: '',
    ...initial,
    setAttribute(k: string, v: string) { attrs[k] = v; },
    getAttribute(k: string) { return attrs[k] ?? null; },
    addEventListener(ev: string, fn: Function) {
      if (!listeners[ev]) listeners[ev] = [];
      listeners[ev].push(fn);
    },
    click() {
      (listeners['click'] || []).forEach((fn) => fn({ stopPropagation: () => {} }));
    },
    trigger(ev: string, ...args: any[]) {
      (listeners[ev] || []).forEach((fn) => fn(...args));
    },
  };
}

function setupDom(elements: Record<string, any>, docHidden = false) {
  const origDoc = globalThis.document;
  const origStorage = globalThis.localStorage;
  const docListeners: Record<string, Function[]> = {};

  const mockStorage: Record<string, string> = {};

  (globalThis as any).localStorage = {
    getItem: (k: string) => mockStorage[k] ?? null,
    setItem: (k: string, v: string) => { mockStorage[k] = String(v); },
    removeItem: (k: string) => { delete mockStorage[k]; },
    clear: () => { Object.keys(mockStorage).forEach((k) => delete mockStorage[k]); },
  };

  (globalThis as any).document = {
    hidden: docHidden,
    getElementById: (id: string) => elements[id] || null,
    addEventListener: (ev: string, fn: Function) => {
      if (!docListeners[ev]) docListeners[ev] = [];
      docListeners[ev].push(fn);
    },
    trigger: (ev: string, ...args: any[]) => {
      (docListeners[ev] || []).forEach((fn) => fn(...args));
    },
  };

  return {
    storage: mockStorage,
    triggerDoc: (ev: string) => (docListeners[ev] || []).forEach((fn) => fn()),
    cleanup: () => {
      globalThis.document = origDoc;
      globalThis.localStorage = origStorage;
    },
  };
}

test('SysStatsController _formatUptime formats seconds into days, hours, or minutes', () => {
  const controller = new SysStatsController({ api: () => ({}) });
  const formatUptime = (controller as any)._formatUptime.bind(controller);

  // Less than an hour
  assert.equal(formatUptime(45), '0m 45s');
  assert.equal(formatUptime(125), '2m 05s');

  // Hours
  assert.equal(formatUptime(3665), '1h 1m 05s');

  // Days
  assert.equal(formatUptime(90065), '1d 1h 1m 05s');
});

test('SysStatsController _storageOn reads from storage key cascade', () => {
  const { storage, cleanup } = setupDom({});
  try {
    const controller = new SysStatsController({ api: () => ({}) });
    const storageOn = (controller as any)._storageOn.bind(controller);

    // Default when no keys present is true (val !== 'false')
    assert.equal(storageOn(), true);

    // Explicit false
    storage['Oryn.showSysStats'] = 'false';
    assert.equal(storageOn(), false);

    // Explicit true
    storage['Oryn.showSysStats'] = 'true';
    assert.equal(storageOn(), true);

    // Fallback key totalshark.showSysStats
    delete storage['Oryn.showSysStats'];
    storage['totalshark.showSysStats'] = 'false';
    assert.equal(storageOn(), false);
  } finally {
    cleanup();
  }
});

test('SysStatsController _tick updates CPU, RAM, and uptime elements', async () => {
  const wrap = createMockElement({ hidden: false });
  const cpuEl = createMockElement();
  const ramEl = createMockElement();
  const upEl = createMockElement();

  const elements = {
    'nx-sys-stats': wrap,
    'nx-stat-cpu': cpuEl,
    'nx-stat-ram': ramEl,
    'nx-stat-up': upEl,
  };
  const { cleanup } = setupDom(elements);

  let statsData: any = {
    cpuPct: 24.6,
    ramUsed: 4 * 1024 * 1024 * 1024,
    ramTotal: 16 * 1024 * 1024 * 1024,
    ramPct: 25,
    uptimeSec: 3600,
  };

  const controller = new SysStatsController({
    api: () => ({
      getSystemStats: async () => statsData,
    }),
  });

  try {
    await (controller as any)._tick();
    assert.equal(cpuEl.textContent, 'CPU 25%');
    assert.equal(ramEl.textContent, 'RAM 25%');
    assert.equal(upEl.textContent, 'UP 1h 0m 00s');

    // Without ramPct (calculated from used / total)
    statsData = {
      cpuPercent: 12,
      ramUsed: 8 * 1024 * 1024 * 1024,
      ramTotal: 16 * 1024 * 1024 * 1024,
      uptimeSec: 60,
    };
    await (controller as any)._tick();
    assert.equal(cpuEl.textContent, 'CPU 12%');
    assert.equal(ramEl.textContent, 'RAM 50%');

    // With fmtSize callback when total is not finite
    const customController = new SysStatsController({
      api: () => ({
        getSystemStats: async () => ({
          cpuPct: 5,
          ramUsed: 1024 * 1024 * 512,
          ramTotal: null,
          uptimeSec: 10,
        }),
      }),
      fmtSize: (sz) => `${sz} bytes`,
    });
    await (customController as any)._tick();
    assert.equal(ramEl.textContent, `RAM ${1024 * 1024 * 512} bytes`);

    // Fallback ramLabel in MB when fmtSize is not provided and total is missing
    const mbController = new SysStatsController({
      api: () => ({
        getSystemStats: async () => ({
          cpuPct: 5,
          ramUsed: 1024 * 1024 * 200,
          ramTotal: 0,
        }),
      }),
    });
    await (mbController as any)._tick();
    assert.equal(ramEl.textContent, 'RAM 200M');

    // Invalid / missing stats payload
    statsData = null;
    await (controller as any)._tick(); // does not throw
  } finally {
    cleanup();
  }
});

test('SysStatsController setVisible and setup handle toggle and visibility changes', () => {
  const btn = createMockElement();
  const wrap = createMockElement();
  const elements = {
    'sys-stats-toggle': btn,
    'nx-sys-stats': wrap,
  };
  const { storage, triggerDoc, cleanup } = setupDom(elements);

  const controller = new SysStatsController({
    api: () => ({
      getSystemStats: async () => ({ cpuPct: 10 }),
    }),
  });

  try {
    controller.setup();
    assert.equal(storage['Oryn.showSysStats'], 'true');
    assert.equal(wrap.hidden, false);
    assert.equal(btn.getAttribute('aria-pressed'), 'true');

    // Toggle button click
    btn.click();
    assert.equal(storage['Oryn.showSysStats'], 'false');
    assert.equal(wrap.hidden, true);
    assert.equal(btn.getAttribute('aria-pressed'), 'false');

    // Toggle again
    btn.click();
    assert.equal(storage['Oryn.showSysStats'], 'true');
    assert.equal(wrap.hidden, false);

    // Document hidden triggers stopPolling
    (globalThis.document as any).hidden = true;
    triggerDoc('visibilitychange');
    assert.equal((controller as any)._timer, null);

    // Document visible again restarts polling
    (globalThis.document as any).hidden = false;
    triggerDoc('visibilitychange');
    assert.notEqual((controller as any)._timer, null);

    // Clean up timer
    controller.setVisible(false);
    assert.equal((controller as any)._timer, null);
  } finally {
    cleanup();
  }
});

test('SysStatsController setup hides elements if API or elements missing', () => {
  const btn = createMockElement();
  const wrap = createMockElement();
  const elements = {
    'sys-stats-toggle': btn,
    'nx-sys-stats': wrap,
  };
  const { cleanup } = setupDom(elements);

  const controller = new SysStatsController({
    api: () => ({}), // no getSystemStats
  });

  try {
    controller.setup();
    assert.equal(btn.hidden, true);
    assert.equal(wrap.hidden, true);
  } finally {
    cleanup();
  }
});

test('SysStatsController tickClock updates clock element', () => {
  const clockEl = createMockElement();
  const { cleanup } = setupDom({ 'nx-clock': clockEl });

  const controller = new SysStatsController({ api: () => ({}) });
  try {
    controller.tickClock();
    assert.match(clockEl.textContent, /^\d{2}:\d{2}:\d{2}$/);
  } finally {
    cleanup();
  }
});
