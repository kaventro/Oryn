// src/modules/virtualList/virtualScroller.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { VirtualScroller } from './virtualScroller.ts';

test('VirtualScroller computes visible window slices accurately', () => {
  const scroller = new VirtualScroller({ rowStride: 30, chunkSize: 8, bufferSize: 10 });
  const totalItems = 1000;
  const clientHeight = 600; // ~20 visible items
  const scrollTop = 3000; // ~100 items down

  const calc = scroller.computeWindow(totalItems, clientHeight, scrollTop);
  assert.equal(calc.totalH, 30000);
  assert.ok(calc.start <= 100);
  assert.ok(calc.end >= 120);
  assert.equal(calc.need, calc.end - calc.start);
  assert.equal(calc.offsetY, calc.start * 30);
});

test('VirtualScroller handles empty lists and small boundaries', () => {
  const scroller = new VirtualScroller();
  const calc = scroller.computeWindow(0, 500, 0);
  assert.equal(calc.totalH, 0);
  assert.equal(calc.start, 0);
  assert.equal(calc.end, 0);
  assert.equal(calc.need, 0);
  assert.equal(calc.offsetY, 0);
});
