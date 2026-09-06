import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { downloadPercent, UpdaterController } from './updaterController.ts';
import { AppSettings } from './settings/settingsModel.ts';

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

function setupMockDom() {
  const elements: Record<string, any> = {};
  function getOrCreate(id: string) {
    if (!elements[id]) {
      elements[id] = {
        id,
        textContent: '',
        style: {},
        classList: {
          _classes: new Set<string>(),
          add(c: string) { this._classes.add(c); },
          remove(c: string) { this._classes.delete(c); },
          contains(c: string) { return this._classes.has(c); },
        },
        listeners: {} as Record<string, Function[]>,
        addEventListener(event: string, fn: Function) {
          if (!this.listeners[event]) this.listeners[event] = [];
          this.listeners[event].push(fn);
        },
        click() {
          (this.listeners['click'] || []).forEach((fn: Function) => fn());
        },
      };
    }
    return elements[id];
  }
  (globalThis as any).document = {
    getElementById: (id: string) => getOrCreate(id),
  };
  return elements;
}

test('downloadPercent maps bytes to a clamped whole percent', () => {
  assert.equal(downloadPercent(0, 100), 0);
  assert.equal(downloadPercent(50, 200), 25);
  assert.equal(downloadPercent(200, 200), 100);
  assert.equal(downloadPercent(300, 200), 100, 'over-delivery clamps to 100');
  assert.equal(downloadPercent(-5, 200), 0, 'negative clamps to 0');
});

test('downloadPercent returns 0 when content-length is unknown', () => {
  assert.equal(downloadPercent(1024, 0), 0);
  assert.equal(downloadPercent(1024, Number.NaN), 0);
  assert.equal(downloadPercent(1024, Number.POSITIVE_INFINITY), 0);
});

test('UpdaterController skips automatic check when enableAutoUpdate is false', async () => {
  setupMockDom();
  let checkCalled = false;
  const mockStorageService: any = {
    load: () => new AppSettings({ enableAutoUpdate: false }),
  };

  const updater = new UpdaterController({
    check: async () => {
      checkCalled = true;
      return null;
    },
    storageService: mockStorageService,
  });

  // Background startup run
  await updater.run(false);
  assert.equal(checkCalled, false, 'check should not be called when auto-update is disabled');

  // Manual run should still proceed even if auto-update is disabled
  await updater.run(true);
  assert.equal(checkCalled, true, 'manual run should proceed even if auto-update is disabled');
});

test('UpdaterController prompts with Update Now and Next Time when update is found', async () => {
  const dom = setupMockDom();
  mockStorage['oryn.update.skipVersion'] = '';

  const mockUpdate: any = {
    version: '0.0.4',
    downloadAndInstall: async () => {},
  };

  const updater = new UpdaterController({
    check: async () => mockUpdate,
    storageService: { load: () => new AppSettings({ enableAutoUpdate: true }) } as any,
  });

  updater.attach();
  await updater.run(false);

  assert.equal(updater.getState(), 'prompt');
  assert.equal(dom['nx-update-title'].textContent, 'Update Available');
  assert.equal(dom['nx-update-msg'].textContent, 'Oryn 0.0.4 is available.');
  assert.equal(dom['nx-update-action'].textContent, 'Update Now');
  assert.equal(dom['nx-update-later'].textContent, 'Next Time');
  assert.equal(dom['nx-update-action'].style.display, '');
  assert.equal(dom['nx-update-later'].style.display, '');

  // Clicking "Next Time" dismisses prompt without marking as skipped
  dom['nx-update-later'].click();
  assert.equal(updater.getState(), 'idle');
  assert.ok(dom['nx-update'].classList.contains('nx-xfer--hidden'));
  assert.notEqual(mockStorage['oryn.update.skipVersion'], '0.0.4');
});

test('UpdaterController downloads and prompts restart when user clicks Update Now', async () => {
  const dom = setupMockDom();
  let downloadCalled = false;
  let relaunchCalled = false;

  const mockUpdate: any = {
    version: '0.0.5',
    downloadAndInstall: async (onProgress: (e: any) => void) => {
      downloadCalled = true;
      onProgress({ event: 'Started', data: { contentLength: 1000 } });
      onProgress({ event: 'Progress', data: { chunkLength: 500 } });
    },
  };

  const updater = new UpdaterController({
    check: async () => mockUpdate,
    relaunch: async () => { relaunchCalled = true; },
    storageService: { load: () => new AppSettings({ enableAutoUpdate: true }) } as any,
  });

  updater.attach();
  await updater.run(false);
  assert.equal(updater.getState(), 'prompt');

  // Click "Update Now"
  dom['nx-update-action'].click();

  // Wait a tick for async downloadAndInstall
  await new Promise((r) => setTimeout(r, 10));

  assert.equal(downloadCalled, true);
  assert.equal(updater.getState(), 'ready');
  assert.equal(dom['nx-update-title'].textContent, 'Oryn 0.0.5 installed');
  assert.equal(dom['nx-update-action'].textContent, 'Restart');
  assert.equal(mockStorage['oryn.update.skipVersion'], '0.0.5');

  // Click "Restart"
  dom['nx-update-action'].click();
  assert.equal(relaunchCalled, true);
});

test('UpdaterController manual check surfaces "Oryn is up to date" when no update exists', async () => {
  const dom = setupMockDom();

  const updater = new UpdaterController({
    check: async () => null,
    storageService: { load: () => new AppSettings({ enableAutoUpdate: true }) } as any,
  });

  updater.attach();
  await updater.run(true);

  assert.equal(dom['nx-update-title'].textContent, 'Oryn is up to date');
  assert.equal(dom['nx-update-msg'].textContent, 'No newer version available.');
});
