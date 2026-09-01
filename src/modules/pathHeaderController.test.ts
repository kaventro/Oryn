// src/modules/pathHeaderController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { PathHeaderController } from './pathHeaderController.ts';

test('PathHeaderController.buildCrumbs handles Windows drive paths', () => {
  const controller = new PathHeaderController({
    state: {},
    api: () => ({}),
    setStatus: () => {},
    loadDir: async () => {},
    syncFilterInput: () => {},
    updatePaneClass: () => {},
    focusActiveList: () => {},
  });

  const crumbs = controller.buildCrumbs('C:\\Users\\TestUser\\Desktop');
  assert.deepEqual(crumbs, [
    { label: 'C:', target: 'C:/' },
    { label: 'Users', target: 'C:/Users' },
    { label: 'TestUser', target: 'C:/Users/TestUser' },
    { label: 'Desktop', target: 'C:/Users/TestUser/Desktop' },
  ]);

  const rootDriveCrumbs = controller.buildCrumbs('D:\\');
  assert.deepEqual(rootDriveCrumbs, [
    { label: 'D:', target: 'D:/' },
  ]);
});

test('PathHeaderController.buildCrumbs handles Unix paths', () => {
  const controller = new PathHeaderController({
    state: {},
    api: () => ({}),
    setStatus: () => {},
    loadDir: async () => {},
    syncFilterInput: () => {},
    updatePaneClass: () => {},
    focusActiveList: () => {},
  });

  const crumbs = controller.buildCrumbs('/home/user/docs');
  assert.equal(crumbs[0].target, '/');
  assert.equal(crumbs[1].label, 'home');
  assert.equal(crumbs[1].target, '/home');
  assert.equal(crumbs[2].label, 'user');
  assert.equal(crumbs[2].target, '/home/user');
  assert.equal(crumbs[3].label, 'docs');
  assert.equal(crumbs[3].target, '/home/user/docs');
});

test('PathHeaderController.buildCrumbs handles SFTP remote paths', () => {
  const controller = new PathHeaderController({} as any);
  const crumbs = controller.buildCrumbs('sftp://prod-srv/var/www/html');
  assert.deepEqual(crumbs, [
    { label: 'SFTP: prod-srv', target: 'sftp://prod-srv/' },
    { label: 'var', target: 'sftp://prod-srv/var' },
    { label: 'www', target: 'sftp://prod-srv/var/www' },
    { label: 'html', target: 'sftp://prod-srv/var/www/html' },
  ]);
});
