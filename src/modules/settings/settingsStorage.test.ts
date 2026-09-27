import test from 'node:test';
import assert from 'node:assert/strict';
import { SettingsStorageService } from './settingsStorage.ts';
import { AppSettings } from './settingsModel.ts';

test('SettingsStorageService load returns default settings when storage is empty', () => {
  const mockStorage: Record<string, string> = {};
  (globalThis as any).localStorage = {
    getItem: (key: string) => mockStorage[key] || null,
    setItem: (key: string, val: string) => { mockStorage[key] = val; },
    removeItem: (key: string) => { delete mockStorage[key]; },
  };

  const service = new SettingsStorageService();
  const settings = service.load();
  assert.equal(settings.dockIcon, '1');
  assert.equal(settings.trayTheme, 'balanced');
  assert.equal(settings.paneMode, 'dual');
});

test('SettingsStorageService load parses saved JSON and respects legacy keys', () => {
  const mockStorage: Record<string, string> = {
    'Oryn.settings': JSON.stringify({ trayTheme: 'ocean', paneMode: 'single', dockIcon: '3' }),
    'Oryn.dockIcon': '5',
  };
  (globalThis as any).localStorage = {
    getItem: (key: string) => mockStorage[key] || null,
    setItem: (key: string, val: string) => { mockStorage[key] = val; },
  };

  const service = new SettingsStorageService();
  const settings = service.load();
  assert.equal(settings.dockIcon, '5');
  assert.equal(settings.paneMode, 'single');
});

test('SettingsStorageService save persists values and notifies subscribers', async () => {
  const mockStorage: Record<string, string> = {};
  (globalThis as any).localStorage = {
    getItem: (key: string) => mockStorage[key] || null,
    setItem: (key: string, val: string) => { mockStorage[key] = val; },
  };

  const service = new SettingsStorageService();
  let notified: AppSettings | null = null;
  const unsubscribe = service.subscribe((s) => { notified = s; });

  const custom = new AppSettings({ dockIcon: '2', trayTheme: 'ember' });
  service.save(custom);

  assert.ok(notified);
  assert.equal((notified as AppSettings).dockIcon, '2');
  assert.equal(mockStorage['Oryn.dockIcon'], '2');
  assert.equal(mockStorage['Oryn.trayTheme'], 'ember');

  unsubscribe();
  notified = null;
  service.save(new AppSettings({ dockIcon: '3' }));
  assert.equal(notified, null);
});

test('SettingsStorageService applyTheme sets or removes data-tray attribute', () => {
  let attrValue: string | null = null;
  (globalThis as any).document = {
    documentElement: {
      setAttribute: (_name: string, val: string) => { attrValue = val; },
      removeAttribute: () => { attrValue = null; },
    },
  };

  const service = new SettingsStorageService();
  service.applyTheme('ocean');
  assert.equal(attrValue, 'ocean');

  service.applyTheme('balanced');
  assert.equal(attrValue, null);

  service.applyTheme('nonexistent');
  assert.equal(attrValue, null);
});

test('SettingsStorageService applyDockIcon updates favicon and invokes setDockIcon', async () => {
  let calledIcon: string | null = null;
  let linkHref: string | null = null;

  (globalThis as any).window = {
    ow: {
      setDockIcon: async (id: string) => { calledIcon = id; },
    },
  };
  (globalThis as any).document = {
    querySelector: () => ({
      set href(val: string) { linkHref = val; },
      get href() { return linkHref || ''; },
    }),
  };

  const service = new SettingsStorageService();
  await service.applyDockIcon('4.png');

  assert.equal(calledIcon, '4');
  assert.equal(linkHref, '/dock-icons/4.png');
});

test('SettingsStorageService applyDockIcon logs, updates favicon and rethrows on failure', async () => {
  let linkHref: string | null = null;
  const originalError = console.error;
  let loggedError: any = null;
  console.error = (...args: any[]) => { loggedError = args; };

  (globalThis as any).window = {
    ow: {
      setDockIcon: async () => { throw new Error('Native window update failed'); },
    },
  };
  (globalThis as any).document = {
    querySelector: () => ({
      set href(val: string) { linkHref = val; },
      get href() { return linkHref || ''; },
    }),
  };

  const service = new SettingsStorageService();
  await assert.rejects(
    async () => {
      await service.applyDockIcon('7');
    },
    { message: 'Native window update failed' },
  );

  assert.equal(linkHref, '/dock-icons/7.png');
  assert.ok(loggedError);
  console.error = originalError;
});

test('SettingsStorageService _notify catches subscriber exceptions gracefully', () => {
  const service = new SettingsStorageService();
  service.subscribe(() => { throw new Error('Boom'); });

  const originalError = console.error;
  let loggedError: any = null;
  console.error = (...args: any[]) => { loggedError = args; };

  service.save(new AppSettings());
  assert.ok(loggedError);
  console.error = originalError;
});

