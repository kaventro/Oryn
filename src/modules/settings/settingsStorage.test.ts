// src/modules/settings/settingsStorage.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { AppSettings } from './settingsModel.ts';
import {
  SettingsStorageService,
  AVAILABLE_THEMES,
  AVAILABLE_DOCK_ICONS,
} from './settingsStorage.ts';

// Mock localStorage
const storage: Record<string, string> = {};
(globalThis as any).localStorage = {
  getItem: (k: string) => storage[k] ?? null,
  setItem: (k: string, v: any) => { storage[k] = String(v); },
  removeItem: (k: string) => { delete storage[k]; },
  clear: () => { Object.keys(storage).forEach((k) => delete storage[k]); },
};

test('AVAILABLE_THEMES and AVAILABLE_DOCK_ICONS definitions', () => {
  assert.ok(AVAILABLE_THEMES.length >= 6);
  assert.ok(AVAILABLE_THEMES.some((t) => t.id === 'balanced'));
  assert.ok(AVAILABLE_THEMES.some((t) => t.id === 'ocean'));

  assert.ok(AVAILABLE_DOCK_ICONS.length >= 10);
  assert.ok(AVAILABLE_DOCK_ICONS.some((d) => d.id === '1'));
});

test('SettingsStorageService load handles valid, corrupt, and legacy storage keys', () => {
  const service = new SettingsStorageService();

  // 1. Clean storage returns defaults
  localStorage.clear();
  const def = service.load();
  assert.equal(def.trayTheme, 'balanced');
  assert.equal(def.paneMode, 'dual');

  // 2. Corrupt JSON in Oryn.settings falls back gracefully
  localStorage.setItem('Oryn.settings', '{ invalid JSON');
  const fallback = service.load();
  assert.ok(fallback instanceof AppSettings);

  // 3. Valid JSON in Oryn.settings
  localStorage.setItem(
    'Oryn.settings',
    JSON.stringify({ trayTheme: 'emerald', paneMode: 'single', dockIcon: '3' })
  );
  const loaded = service.load();
  assert.equal(loaded.trayTheme, 'emerald');
  assert.equal(loaded.paneMode, 'single');
  assert.equal(loaded.dockIcon, '3');

  // 4. Legacy storage keys take precedence or populate missing fields
  localStorage.clear();
  localStorage.setItem('Oryn.trayTheme', 'purple');
  localStorage.setItem('Oryn.paneMode', 'single');
  localStorage.setItem('Oryn.dockIcon', '5');
  const legacy = service.load();
  assert.equal(legacy.trayTheme, 'purple');
  assert.equal(legacy.paneMode, 'single');
  assert.equal(legacy.dockIcon, '5');
});

test('SettingsStorageService save persists settings, applies theme, dock icon, and notifies subscribers', async () => {
  const service = new SettingsStorageService();
  localStorage.clear();

  let dockIconCalledWith = '';
  let iconLinkHref = '';
  const rootAttrs: Record<string, string> = {};

  const origDoc = globalThis.document;
  const origWindow = (globalThis as any).window;

  try {
    (globalThis as any).document = {
      documentElement: {
        setAttribute: (k: string, v: string) => { rootAttrs[k] = v; },
        removeAttribute: (k: string) => { delete rootAttrs[k]; },
      },
      querySelector: (sel: string) => {
        if (sel === "link[rel~='icon']") {
          return {
            set href(val: string) { iconLinkHref = val; },
            get href() { return iconLinkHref; },
          };
        }
        return null;
      },
    };

    (globalThis as any).window = {
      ow: {
        setDockIcon: async (id: string) => { dockIconCalledWith = id; },
      },
    };

    let notifiedSettings: any = null;
    let failingListenerRan = false;

    // Subscribe listeners
    const unsubscribe = service.subscribe((s) => { notifiedSettings = s; });
    // Listener that throws error to verify safe error handling
    service.subscribe(() => {
      failingListenerRan = true;
      throw new Error('Listener failure');
    });

    const settingsToSave = new AppSettings({
      trayTheme: 'ocean',
      paneMode: 'single',
      dockIcon: '4.png',
    });

    service.save(settingsToSave);

    // Wait a tick for async applyDockIcon to complete
    await new Promise((resolve) => setTimeout(resolve, 10));

    // Verify localStorage persistence
    assert.equal(storage['Oryn.trayTheme'], 'ocean');
    assert.equal(storage['Oryn.paneMode'], 'single');
    assert.equal(storage['Oryn.dockIcon'], '4.png');
    assert.ok(storage['Oryn.settings']);

    // Verify theme application
    assert.equal(rootAttrs['data-tray'], 'ocean');

    // Verify dock icon application
    assert.equal(dockIconCalledWith, '4');
    assert.equal(iconLinkHref, '/dock-icons/4.png');

    // Verify subscriber notification
    assert.equal(notifiedSettings?.trayTheme, 'ocean');
    assert.equal(failingListenerRan, true);

    // Test saving balanced theme removes data-tray attribute
    service.applyTheme('balanced');
    assert.equal(rootAttrs['data-tray'], undefined);

    // Test saving invalid theme falls back to balanced
    service.applyTheme('nonexistent-theme');
    assert.equal(rootAttrs['data-tray'], undefined);

    // Direct applyDockIcon with error in api.setDockIcon
    (globalThis as any).window.ow.setDockIcon = async () => { throw new Error('API failure'); };
    await service.applyDockIcon('7');
    assert.equal(iconLinkHref, '/dock-icons/7.png');

    // Unsubscribe
    const res = unsubscribe();
    assert.equal(res, true);
    notifiedSettings = null;
    service.save(new AppSettings({ trayTheme: 'emerald' }));
    assert.equal(notifiedSettings, null);
  } finally {
    globalThis.document = origDoc;
    (globalThis as any).window = origWindow;
  }
});
