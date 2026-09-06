// src/modules/searchSessionGateway.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';
import { TauriSearchSessionGateway } from './searchSessionGateway.ts';

test('TauriSearchSessionGateway generates clientId and forwards calls to api', async () => {
  const calls: { method: string; args: any[] }[] = [];

  const mockApi = {
    searchStart: async (args: any) => {
      calls.push({ method: 'searchStart', args: [args] });
      return { sessionId: 'sess-123' };
    },
    searchPage: async (id: string, off: number, lim: number) => {
      calls.push({ method: 'searchPage', args: [id, off, lim] });
      return { items: [], total: 0 };
    },
    cancelSearch: async (clientId: string) => {
      calls.push({ method: 'cancelSearch', args: [clientId] });
      return { ok: true };
    },
    releaseSearch: async (sessionId: string) => {
      calls.push({ method: 'releaseSearch', args: [sessionId] });
      return { ok: true };
    },
  };

  const gateway = new TauriSearchSessionGateway(() => mockApi);
  assert.ok(gateway.clientId);

  // 1. start
  const query = { path: '/home', pattern: '*.ts' };
  const res = await gateway.start(query);
  assert.equal(res.sessionId, 'sess-123');
  assert.equal(calls[0].method, 'searchStart');
  assert.equal(calls[0].args[0].clientId, gateway.clientId);
  assert.equal(calls[0].args[0].pattern, '*.ts');

  // 2. getPage
  await gateway.getPage('sess-123', 0, 50);
  assert.equal(calls[1].method, 'searchPage');
  assert.deepEqual(calls[1].args, ['sess-123', 0, 50]);

  // 3. cancel
  await gateway.cancel();
  assert.equal(calls[2].method, 'cancelSearch');
  assert.deepEqual(calls[2].args, [gateway.clientId]);

  // 4. release
  await gateway.release('sess-123');
  assert.equal(calls[3].method, 'releaseSearch');
  assert.deepEqual(calls[3].args, ['sess-123']);
});

test('TauriSearchSessionGateway falls back when randomUUID is unavailable', () => {
  const origDescriptor = Object.getOwnPropertyDescriptor(globalThis.crypto, 'randomUUID');
  try {
    Object.defineProperty(globalThis.crypto, 'randomUUID', {
      value: undefined,
      configurable: true,
      writable: true,
    });
    const gateway = new TauriSearchSessionGateway(() => ({}));
    assert.ok(gateway.clientId.startsWith('search-client-'));
  } finally {
    if (origDescriptor) {
      Object.defineProperty(globalThis.crypto, 'randomUUID', origDescriptor);
    }
  }
});
