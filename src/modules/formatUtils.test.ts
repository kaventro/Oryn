// src/modules/formatUtils.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { escHtml, safeColor } from './formatUtils.ts';

test('escHtml escapes all HTML-significant characters', () => {
  assert.equal(escHtml('<script>'), '&lt;script&gt;');
  assert.equal(escHtml('a & b'), 'a &amp; b');
  // Quotes must be escaped so the output is safe inside HTML attribute values.
  assert.equal(escHtml('"'), '&quot;');
  assert.equal(escHtml("'"), '&#39;');
});

test('escHtml neutralizes attribute-breakout payloads', () => {
  // Simulates a malicious git author name / commit summary rendered into
  // title="${escHtml(...)}". Without quote escaping this would inject an
  // onmouseover handler; the escaped form must contain no raw double quote.
  const payload = 'x" onmouseover="alert(1)';
  const escaped = escHtml(payload);
  assert.ok(!escaped.includes('"'), 'escaped output must not contain a raw double quote');
  assert.equal(escaped, 'x&quot; onmouseover=&quot;alert(1)');
});

test('escHtml coerces non-string input', () => {
  assert.equal(escHtml(42 as any), '42');
  assert.equal(escHtml(null as any), 'null');
});

test('safeColor passes valid colors and blocks CSS-injection payloads', () => {
  assert.equal(safeColor('#8e8e93'), '#8e8e93');
  assert.equal(safeColor('#fff'), '#fff');
  assert.equal(safeColor('rgb(10, 20, 30)'), 'rgb(10, 20, 30)');
  assert.equal(safeColor('rebeccapurple'), 'rebeccapurple');
  // Injection attempts fall back.
  assert.equal(safeColor('url(https://evil/x)'), 'transparent');
  assert.equal(safeColor('red; background: url(x)'), 'transparent');
  assert.equal(safeColor('expression(alert(1))'), 'transparent');
  assert.equal(safeColor(null as any), 'transparent');
  assert.equal(safeColor('#fff', 'black'), '#fff');
  assert.equal(safeColor('nope!!', 'black'), 'black');
});
