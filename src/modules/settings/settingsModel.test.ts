// src/modules/settings/settingsModel.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { AppSettings } from './settingsModel.ts';

test('AppSettings initializes with enableSftp default to false and default dock icon', () => {
  const settings = new AppSettings();
  assert.equal(settings.enableSftp, false, 'enableSftp should be false by default');
  assert.equal(settings.paneMode, 'dual');
  assert.equal(settings.trayTheme, 'balanced');
  assert.equal(settings.dockIcon, '1');
  assert.equal(settings.confirmDelete, true);
});

test('AppSettings respects enableSftp parameter and clones properly', () => {
  const custom = new AppSettings({ enableSftp: true, trayTheme: 'ocean', dockIcon: '4' });
  assert.equal(custom.enableSftp, true);
  assert.equal(custom.trayTheme, 'ocean');
  assert.equal(custom.dockIcon, '4');

  const cloned = custom.clone();
  assert.equal(cloned.enableSftp, true);
  assert.equal(cloned.trayTheme, 'ocean');
  assert.equal(custom.dockIcon, '4');
  assert.deepEqual(cloned.toJSON(), custom.toJSON());
});
