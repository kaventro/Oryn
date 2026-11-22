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

test('RemoteController resolves a function API for profiles and connection operations', async () => {
  const mockProfiles = [{ id: 'srv1', name: 'Server 1', host: 'example.com', username: 'root' }];
  const calls: string[] = [];
  const mockApi = {
    remoteListProfiles: async () => {
      calls.push('list');
      return mockProfiles;
    },
    remoteSaveProfile: async (profile: any) => {
      calls.push(`save:${profile.id}`);
      return [profile];
    },
    remoteConnect: async (profile: any) => {
      calls.push(`connect:${profile.id}`);
      return { ok: true };
    },
  };
  const controller = new RemoteController({ api: () => mockApi });

  assert.deepEqual(await controller.loadProfiles(), mockProfiles);
  assert.deepEqual(await controller.connect(mockProfiles[0]), { ok: true });
  assert.deepEqual(await controller.saveProfile(mockProfiles[0]), mockProfiles);
  assert.deepEqual(calls, ['list', 'connect:srv1', 'save:srv1']);
});

test('RemoteController forwards every remote operation to the resolved API', async () => {
  const calls: Array<[string, unknown[]]> = [];
  const record = (name: string) => async (...args: unknown[]) => {
    calls.push([name, args]);
    return name === 'remoteDeleteProfile' ? [] : `${name}-result`;
  };
  const mockApi = {
    remoteDeleteProfile: record('remoteDeleteProfile'),
    remoteTestConnection: record('remoteTestConnection'),
    remoteDisconnect: record('remoteDisconnect'),
    remoteReadDir: record('remoteReadDir'),
    remoteReadFileText: record('remoteReadFileText'),
    remoteWriteFileText: record('remoteWriteFileText'),
    remoteMkdir: record('remoteMkdir'),
    remoteCreateFile: record('remoteCreateFile'),
    remoteRename: record('remoteRename'),
    remoteDelete: record('remoteDelete'),
    remoteDownload: record('remoteDownload'),
    remoteUpload: record('remoteUpload'),
  };
  let resolves = 0;
  const controller = new RemoteController({
    api: () => {
      resolves += 1;
      return mockApi;
    },
  });
  const profile = { id: 'srv1', name: 'Server 1', host: 'example.com', username: 'root' } as any;

  assert.deepEqual(await controller.deleteProfile('srv1'), []);
  assert.deepEqual(controller.profiles, []);
  assert.equal(await controller.testConnection(profile), 'remoteTestConnection-result');
  assert.equal(await controller.disconnect('srv1'), 'remoteDisconnect-result');
  assert.equal(await controller.readDir('srv1', '/var'), 'remoteReadDir-result');
  assert.equal(await controller.readFileText('srv1', '/a.txt'), 'remoteReadFileText-result');
  assert.equal(await controller.writeFileText('srv1', '/a.txt', 'hi'), 'remoteWriteFileText-result');
  assert.equal(await controller.mkdir('srv1', '/dir'), 'remoteMkdir-result');
  assert.equal(await controller.createFile('srv1', '/b.txt'), 'remoteCreateFile-result');
  assert.equal(await controller.rename('srv1', '/a.txt', '/c.txt'), 'remoteRename-result');
  assert.equal(await controller.delete('srv1', '/dir', true), 'remoteDelete-result');
  assert.equal(await controller.download('srv1', '/c.txt', '/tmp/c.txt'), 'remoteDownload-result');
  assert.equal(await controller.upload('srv1', '/tmp/d.txt', '/d.txt'), 'remoteUpload-result');

  assert.equal(resolves, 12);
  assert.deepEqual(calls, [
    ['remoteDeleteProfile', ['srv1']],
    ['remoteTestConnection', [profile]],
    ['remoteDisconnect', ['srv1']],
    ['remoteReadDir', ['srv1', '/var']],
    ['remoteReadFileText', ['srv1', '/a.txt']],
    ['remoteWriteFileText', ['srv1', '/a.txt', 'hi']],
    ['remoteMkdir', ['srv1', '/dir']],
    ['remoteCreateFile', ['srv1', '/b.txt']],
    ['remoteRename', ['srv1', '/a.txt', '/c.txt']],
    ['remoteDelete', ['srv1', '/dir', true]],
    ['remoteDownload', ['srv1', '/c.txt', '/tmp/c.txt']],
    ['remoteUpload', ['srv1', '/tmp/d.txt', '/d.txt']],
  ]);
});

test('RemoteController falls back to window.ow when no API is supplied', async () => {
  const origWindow = (globalThis as any).window;
  const profiles = [{ id: 'srv2', name: 'Server 2', host: 'example.org', username: 'deploy' }];
  try {
    (globalThis as any).window = { ow: { remoteListProfiles: async () => profiles } };
    const controller = new RemoteController();
    assert.deepEqual(await controller.loadProfiles(), profiles);

    (globalThis as any).window = {};
    assert.deepEqual(await new RemoteController().loadProfiles(), []);
  } finally {
    (globalThis as any).window = origWindow;
  }
});

test('RemoteController loadProfiles handles no window, empty results and failures', async () => {
  const origWindow = (globalThis as any).window;
  const origWarn = console.warn;
  const warnings: unknown[][] = [];
  try {
    delete (globalThis as any).window;
    assert.deepEqual(await new RemoteController().loadProfiles(), []);

    const empty = new RemoteController({ api: () => ({ remoteListProfiles: async () => null }) });
    assert.deepEqual(await empty.loadProfiles(), []);
    assert.deepEqual(empty.profiles, []);

    console.warn = (...args: unknown[]) => {
      warnings.push(args);
    };
    const failing = new RemoteController({
      api: () => ({
        remoteListProfiles: async () => {
          throw new Error('offline');
        },
      }),
    });
    assert.deepEqual(await failing.loadProfiles(), []);
    assert.equal(warnings.length, 1);
    assert.equal(warnings[0][0], '[RemoteController] Failed to load profiles:');
  } finally {
    console.warn = origWarn;
    (globalThis as any).window = origWindow;
  }
});
