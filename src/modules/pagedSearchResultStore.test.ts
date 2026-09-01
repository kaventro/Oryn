// src/modules/pagedSearchResultStore.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { PagedSearchResultStore } from './pagedSearchResultStore.ts';

function makePage(sessionId: string, offset: number, pageSize: number, resultCount = 12) {
  const end = Math.min(offset + pageSize, resultCount);
  return {
    ok: true,
    sessionId,
    offset,
    resultCount,
    items: Array.from({ length: end - offset }, (_, index) => `${sessionId}:${offset + index}`),
  };
}

test('deduplicates concurrent fetches for one result page', async () => {
  let resolvePage: any;
  let calls = 0;
  const gateway = {
    getPage: () => {
      calls += 1;
      return new Promise((resolve) => { resolvePage = resolve; });
    },
  };
  const store = new PagedSearchResultStore({ gateway, pageSize: 2 });
  store.reset('session-a', 12, 2);

  const first = store.ensureRange(0, 1);
  const second = store.ensureRange(0, 1);
  assert.equal(calls, 0, 'gateway invocation is deferred to a microtask');
  await Promise.resolve();
  assert.equal(calls, 1);
  resolvePage(makePage('session-a', 0, 2));

  assert.equal(await first, true);
  assert.equal(await second, true);
  assert.equal(store.peek(0), 'session-a:0');
});

test('keeps only the configured LRU page window', async () => {
  const requestedOffsets: number[] = [];
  const gateway = {
    getPage: async (sessionId: string, offset: number, pageSize: number) => {
      requestedOffsets.push(offset);
      return makePage(sessionId, offset, pageSize);
    },
  };
  const store = new PagedSearchResultStore({ gateway, pageSize: 2, maxPages: 2 });
  store.reset('session-a', 12, 2);

  await store.get(0);
  await store.get(2);
  await store.get(4);
  assert.deepEqual([...store.pages.keys()], [2, 4]);

  store.peek(2);
  await store.get(6);
  assert.deepEqual([...store.pages.keys()], [2, 6]);
  assert.deepEqual(requestedOffsets, [0, 2, 4, 6]);
});

test('drops a page response from a replaced search session', async () => {
  let resolvePage: any;
  const gateway = {
    getPage: () => new Promise((resolve) => { resolvePage = resolve; }),
  };
  const store = new PagedSearchResultStore({ gateway, pageSize: 2 });
  store.reset('old-session', 12, 2);
  const request = store.ensureRange(0, 1);
  await Promise.resolve();
  store.reset('new-session', 12, 2);
  resolvePage(makePage('old-session', 0, 2));

  assert.equal(await request, false);
  assert.equal(store.peek(0), null);
  assert.equal(store.sessionId, 'new-session');
});

test('reports page-load errors without rejecting the renderer promise', async () => {
  const errors: any[] = [];
  const gateway = {
    getPage: async () => { throw new Error('session expired'); },
  };
  const store = new PagedSearchResultStore({
    gateway,
    pageSize: 2,
    onError: (error: any, sessionId: string) => errors.push([error.message, sessionId]),
  });
  store.reset('session-a', 12, 2);

  assert.equal(await store.ensureRange(0, 1), false);
  assert.deepEqual(errors, [['session expired', 'session-a']]);
});
