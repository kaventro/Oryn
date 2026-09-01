import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { downloadPercent } from './updaterController.ts';

test('downloadPercent maps bytes to a clamped whole percent', () => {
  assert.equal(downloadPercent(0, 100), 0);
  assert.equal(downloadPercent(50, 200), 25);
  assert.equal(downloadPercent(200, 200), 100);
  assert.equal(downloadPercent(300, 200), 100, 'over-delivery clamps to 100');
  assert.equal(downloadPercent(-5, 200), 0, 'negative clamps to 0');
});

test('downloadPercent returns 0 when content-length is unknown', () => {
  assert.equal(downloadPercent(1024, 0), 0);
  assert.equal(downloadPercent(1024, Number.NaN), 0);
  assert.equal(downloadPercent(1024, Number.POSITIVE_INFINITY), 0);
});
