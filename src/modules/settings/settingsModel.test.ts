// src/modules/settings/settingsModel.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { AppSettings } from './settingsModel.ts';

test('AppSettings initializes with enableSftp default to true, enableAutoUpdate default to true and default dock icon', () => {
  const settings = new AppSettings();
  assert.equal(settings.enableSftp, true, 'enableSftp should be true by default');
  assert.equal(settings.enableTags, true, 'enableTags should be true by default');
  assert.equal(settings.enableAutoUpdate, true, 'enableAutoUpdate should be true by default');
  assert.equal(settings.paneMode, 'dual');
  assert.equal(settings.trayTheme, 'balanced');
  assert.equal(settings.dockIcon, '1');
  assert.equal(settings.confirmDelete, true);
});

test('AppSettings respects enableSftp, enableTags, and enableAutoUpdate parameters and clones properly', () => {
  const custom = new AppSettings({
    enableSftp: false,
    enableTags: false,
    enableAutoUpdate: false,
    trayTheme: 'ocean',
    dockIcon: '4',
  });
  assert.equal(custom.enableSftp, false);
  assert.equal(custom.enableTags, false);
  assert.equal(custom.enableAutoUpdate, false);
  assert.equal(custom.trayTheme, 'ocean');
  assert.equal(custom.dockIcon, '4');

  const cloned = custom.clone();
  assert.equal(cloned.enableSftp, false);
  assert.equal(cloned.enableTags, false);
  assert.equal(cloned.enableAutoUpdate, false);
  assert.equal(cloned.trayTheme, 'ocean');
  assert.equal(custom.dockIcon, '4');
  assert.deepEqual(cloned.toJSON(), custom.toJSON());
});

test('AppSettings.createDefault returns a fresh default AppSettings instance with complete configuration coverage', () => {
  const def = AppSettings.createDefault();
  assert.ok(def instanceof AppSettings);
  assert.equal(def.enableSftp, true);
  assert.equal(def.paneMode, 'dual');

  const customFull = new AppSettings({
    paneMode: 'single',
    showHiddenFiles: true,
    showExtensions: false,
    showStatusBarTerminal: false,
    confirmDelete: false,
    overwritePolicy: 'overwrite',
    defaultDiffRef: 'HEAD~1',
    defaultEditor: 'custom',
    customEditorCmd: 'subl',
    rowDensity: 'comfortable',
    dateFormat: 'iso',
    dualPaneDriveDefaults: false,
    leftDefaultDrive: '/Volumes/Ext1',
    rightDefaultDrive: '/Volumes/Ext2',
  });

  assert.equal(customFull.paneMode, 'single');
  assert.equal(customFull.showHiddenFiles, true);
  assert.equal(customFull.showExtensions, false);
  assert.equal(customFull.showStatusBarTerminal, false);
  assert.equal(customFull.confirmDelete, false);
  assert.equal(customFull.overwritePolicy, 'overwrite');
  assert.equal(customFull.defaultDiffRef, 'HEAD~1');
  assert.equal(customFull.defaultEditor, 'custom');
  assert.equal(customFull.customEditorCmd, 'subl');
  assert.equal(customFull.rowDensity, 'comfortable');
  assert.equal(customFull.dateFormat, 'iso');
  assert.equal(customFull.dualPaneDriveDefaults, false);
  assert.equal(customFull.leftDefaultDrive, '/Volumes/Ext1');
  assert.equal(customFull.rightDefaultDrive, '/Volumes/Ext2');
});

