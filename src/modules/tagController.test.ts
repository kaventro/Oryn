// src/modules/tagController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { TagController } from './tagController.ts';

test('TagController manages file tags and respects isEnabled toggle', () => {
  const storage: Record<string, string> = {};
  (globalThis as any).localStorage = {
    getItem: (k: string) => storage[k] || null,
    setItem: (k: string, v: string) => { storage[k] = v; },
    removeItem: (k: string) => { delete storage[k]; },
  };

  const tc = new TagController({
    api: () => ({}),
    state: { left: {} as any, right: {} as any, active: 'left' } as any,
    setStatus: () => {},
    renderPane: () => {},
  });

  // By default, tags are enabled
  assert.equal(tc.isEnabled, true);
  assert.ok(tc.getAllTags().length > 0);

  // Add a tag to a file
  tc.addTagToFile('/path/to/test.txt', 'red');
  assert.deepEqual(tc.getTagsForFile('/path/to/test.txt'), ['red']);

  // Disable tags
  tc.isEnabled = false;
  assert.deepEqual(tc.getTagsForFile('/path/to/test.txt'), []);
  assert.deepEqual(tc.getAllTags(), []);

  // Re-enable tags
  tc.isEnabled = true;
  assert.deepEqual(tc.getTagsForFile('/path/to/test.txt'), ['red']);
  assert.ok(tc.getAllTags().length > 0);

  delete (globalThis as any).localStorage;
});
