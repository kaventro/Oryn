// src/modules/syntaxHighlighter.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { detectLanguage, highlightCode } from './syntaxHighlighter.ts';

test('detectLanguage resolves standard file extensions correctly', () => {
  assert.equal(detectLanguage('index.ts'), 'ts');
  assert.equal(detectLanguage('main.rs'), 'rust');
  assert.equal(detectLanguage('app.py'), 'python');
  assert.equal(detectLanguage('styles.scss'), 'scss');
  assert.equal(detectLanguage('schema.sql'), 'sql');
  assert.equal(detectLanguage('package.json'), 'json');
  assert.equal(detectLanguage('config.yaml'), 'yaml');
  assert.equal(detectLanguage('deploy.sh'), 'bash');
  assert.equal(detectLanguage('main.go'), 'go');
  assert.equal(detectLanguage('unknown.xyz'), 'plaintext');
});

test('highlightCode highlights keywords, strings, comments and numbers safely', () => {
  const tsCode = `
    // A sample function
    export async function fetchUser(id: number): Promise<User> {
      const url = "https://api.example.com/user/" + id;
      return await api.get(url);
    }
  `;

  const html = highlightCode(tsCode, 'ts');
  assert.ok(html.includes('class="tok-kw"'), 'Should highlight keywords');
  assert.ok(html.includes('class="tok-com"'), 'Should highlight comments');
  assert.ok(html.includes('class="tok-str"'), 'Should highlight strings');
  assert.ok(html.includes('class="tok-type"'), 'Should highlight types');
  assert.ok(html.includes('class="tok-fn"'), 'Should highlight functions');
  assert.ok(!html.includes('<script>'), 'Must not contain raw script tags');
});

test('highlightCode neutralizes XSS payloads in source code', () => {
  const malicious = '<script>alert("xss")</script><img src=x onerror=alert(1)>';
  const html = highlightCode(malicious, 'js');
  assert.ok(!html.includes('<script>'), 'Script tag should be escaped');
  assert.ok(html.includes('&lt;script&gt;') || html.includes('&lt;'), 'Tags must be escaped');
});

test('highlightCode highlights JSON keys and primitives', () => {
  const jsonCode = '{\n  "name": "Oryn",\n  "version": 1,\n  "active": true\n}';
  const html = highlightCode(jsonCode, 'json');
  assert.ok(html.includes('class="tok-prop"'), 'Should highlight property keys');
  assert.ok(html.includes('class="tok-str"'), 'Should highlight string values');
  assert.ok(html.includes('class="tok-num"'), 'Should highlight numbers');
  assert.ok(html.includes('class="tok-kw"'), 'Should highlight booleans');
});
