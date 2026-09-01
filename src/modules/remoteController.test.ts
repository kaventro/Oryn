// src/modules/remoteController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { isRemotePath, formatRemotePath, RemoteController } from './remoteController.ts';

test('isRemotePath detects sftp and ssh URLs correctly', () => {
  assert.deepEqual(isRemotePath('sftp://prod-server/var/www'), {
    isRemote: true,
    profileId: 'prod-server',
    remotePath: '/var/www',
  });

  assert.deepEqual(isRemotePath('sftp://myserver/'), {
    isRemote: true,
    profileId: 'myserver',
    remotePath: '/',
  });

  assert.deepEqual(isRemotePath('sftp://myserver'), {
    isRemote: true,
    profileId: 'myserver',
    remotePath: '/',
  });

  assert.deepEqual(isRemotePath('ssh://dev/home/user'), {
    isRemote: true,
    profileId: 'dev',
    remotePath: '/home/user',
  });

  assert.deepEqual(isRemotePath('/local/path/file.txt'), {
    isRemote: false,
    profileId: null,
    remotePath: '/local/path/file.txt',
  });

  assert.deepEqual(isRemotePath('C:\\Users\\me'), {
    isRemote: false,
    profileId: null,
    remotePath: 'C:\\Users\\me',
  });

  assert.deepEqual(isRemotePath(null), {
    isRemote: false,
    profileId: null,
    remotePath: '',
  });
});

test('formatRemotePath formats sftp paths correctly', () => {
  assert.equal(formatRemotePath('prod', '/var/log'), 'sftp://prod/var/log');
  assert.equal(formatRemotePath('prod', 'var/log'), 'sftp://prod/var/log');
  assert.equal(formatRemotePath('prod'), 'sftp://prod/');
});

test('RemoteController handles profiles and operations', async () => {
  const mockProfiles = [{ id: 'srv1', name: 'Server 1', host: 'example.com', port: 22, username: 'root', auth_type: 'Password' }];
  const mockApi = {
    remoteListProfiles: async () => mockProfiles,
    remoteSaveProfile: async (p: any) => [p],
    remoteDeleteProfile: async () => [],
    remoteTestConnection: async () => ({ ok: true }),
    remoteConnect: async () => ({ ok: true, items: [] }),
    remoteReadDir: async () => ({ ok: true, items: [] }),
    remoteReadFileText: async () => 'hello remote',
  };

  const controller = new RemoteController({ api: mockApi as any });
  const loaded = await controller.loadProfiles();
  assert.equal(loaded.length, 1);
  assert.equal(controller.getProfile('srv1')?.name, 'Server 1');
  assert.equal(controller.getProfile('non-existent'), null);

  const text = await controller.readFileText('srv1', '/test.txt');
  assert.equal(text, 'hello remote');
});
