// src/modules/markdownRenderer.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { isMarkdownName, renderMarkdown, safeUrl } from './markdownRenderer.ts';

test('isMarkdownName matches markdown extensions case-insensitively', () => {
  assert.equal(isMarkdownName('README.md'), true);
  assert.equal(isMarkdownName('NOTES.MARKDOWN'), true);
  assert.equal(isMarkdownName('a.mdx'), true);
  assert.equal(isMarkdownName('script.js'), false);
  assert.equal(isMarkdownName('Makefile'), false);
  assert.equal(isMarkdownName(''), false);
  assert.equal(isMarkdownName(null as any), false);
});

test('safeUrl accepts only absolute web and mail targets', () => {
  assert.equal(safeUrl('https://example.com/a?b=1'), 'https://example.com/a?b=1');
  assert.equal(safeUrl('http://example.com'), 'http://example.com');
  assert.equal(safeUrl('mailto:dev@example.com'), 'mailto:dev@example.com');
  // Relative repo links cannot be resolved from the viewer.
  assert.equal(safeUrl('LICENSE'), null);
  assert.equal(safeUrl('./docs/a.md'), null);
  // Anything that would hand the OS shell something dangerous.
  assert.equal(safeUrl('javascript:alert(1)'), null);
  assert.equal(safeUrl('file:///C:/Windows/System32/calc.exe'), null);
  assert.equal(safeUrl('\\\\server\\share'), null);
  assert.equal(safeUrl(''), null);
  assert.equal(safeUrl(null as any), null);
});

test('safeUrl rejects targets carrying whitespace or control characters', () => {
  // A newline inside the URL must not let a payload sneak past the scheme test.
  assert.equal(safeUrl('java\nscript:alert(1)'), null);
  assert.equal(safeUrl('https://a.com/\u0000x'), null);
  assert.equal(safeUrl('https://a.com/ x'), null);
});

test('headings render at the right level', () => {
  const html = renderMarkdown('# One\n\n### Three');
  assert.match(html, /<h1 class="md-h md-h1">One<\/h1>/);
  assert.match(html, /<h3 class="md-h md-h3">Three<\/h3>/);
});

test('emphasis, strikethrough and inline code render', () => {
  const html = renderMarkdown('a **bold** and *italic* and ~~gone~~ and `code`');
  assert.match(html, /<strong>bold<\/strong>/);
  assert.match(html, /<em>italic<\/em>/);
  assert.match(html, /<del>gone<\/del>/);
  assert.match(html, /<code class="md-code-inline">code<\/code>/);
});

test('markup inside a code span is not interpreted', () => {
  const html = renderMarkdown('`**not bold** <b>x</b>`');
  assert.ok(!html.includes('<strong>'), 'code span content must stay literal');
  assert.match(html, /&lt;b&gt;/);
});

test('bold wrapping a code span keeps both, as in **`good first issue`**', () => {
  const html = renderMarkdown('Issues labeled **`good first issue`** are great.');
  assert.match(html, /<strong><code class="md-code-inline">good first issue<\/code><\/strong>/);
});

test('snake_case identifiers are not turned into emphasis', () => {
  const html = renderMarkdown('call some_long_name here');
  assert.ok(!html.includes('<em>'), 'underscores inside a word must not emphasise');
  assert.match(html, /some_long_name/);
});

test('absolute links become anchors carrying the resolved target', () => {
  const html = renderMarkdown('See the [Issues](https://github.com/kaventro/Oryn/issues) tab.');
  assert.match(html, /<a class="md-link" href="https:\/\/github\.com\/kaventro\/Oryn\/issues"/);
  assert.match(html, /data-md-href="https:\/\/github\.com\/kaventro\/Oryn\/issues"/);
  assert.match(html, />Issues<\/a>/);
});

test('a relative link renders as plain text that still shows its target', () => {
  const html = renderMarkdown('the [MIT License](LICENSE) applies');
  assert.ok(!html.includes('<a '), 'relative targets must not be clickable');
  assert.match(html, /<span class="md-link-plain" title="LICENSE">MIT License<\/span>/);
});

test('a javascript: link never becomes an anchor', () => {
  const html = renderMarkdown('[click](javascript:alert(1))');
  assert.ok(!html.includes('<a '), 'javascript: must not produce an anchor');
  assert.ok(!html.includes('href='), 'no href may be emitted for a rejected scheme');
});

test('embedded HTML is escaped rather than executed', () => {
  const html = renderMarkdown('<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>');
  assert.ok(!html.includes('<script'), 'script tags must not survive');
  assert.ok(!html.includes('<img'), 'img tags must not survive');
  assert.match(html, /&lt;script&gt;/);
  assert.match(html, /&lt;img src=x onerror=alert\(1\)&gt;/);
});

test('an image reference degrades to its alt text with the source on hover', () => {
  const html = renderMarkdown('![a badge](https://img.example/b.svg)');
  assert.ok(!html.includes('<img'), 'no network image is emitted');
  assert.match(html, /<span class="md-image" title="https:\/\/img\.example\/b\.svg">a badge<\/span>/);
});

test('fenced code blocks keep their content verbatim and record the language', () => {
  const html = renderMarkdown('```js\nconst a = 1 < 2;\n```');
  assert.match(html, /<div class="md-code-lang">js<\/div>/);
  assert.match(html, /class="tok-kw">const<\/span>/);
  assert.match(html, /&lt;/);
});

test('an unclosed fence still terminates instead of dropping the rest', () => {
  const html = renderMarkdown('```\nstuck');
  assert.match(html, /<pre><code>stuck<\/code><\/pre>/);
});

test('bullet lists render, including nested items', () => {
  const html = renderMarkdown('- one\n- two\n  - inner\n- three');
  assert.match(html, /<ul class="md-list">/);
  assert.equal((html.match(/<ul class="md-list">/g) || []).length, 2);
  assert.match(html, /<li class="md-li">inner<\/li>/);
});

test('ordered lists use ol', () => {
  const html = renderMarkdown('1. first\n2. second');
  assert.match(html, /<ol class="md-list">/);
  assert.match(html, /<li class="md-li">first<\/li>/);
});

test('task list items render as disabled checkboxes', () => {
  const html = renderMarkdown('- [x] done\n- [ ] todo');
  assert.match(html, /<input type="checkbox" disabled checked>/);
  assert.match(html, /<input type="checkbox" disabled>/);
  assert.ok(!html.includes('[x]'), 'the marker itself must be consumed');
});

test('blockquotes nest their content as blocks', () => {
  const html = renderMarkdown('> quoted line');
  assert.match(html, /<blockquote class="md-quote">.*quoted line.*<\/blockquote>/s);
});

test('horizontal rules render', () => {
  assert.match(renderMarkdown('---'), /<hr class="md-hr">/);
  assert.match(renderMarkdown('***'), /<hr class="md-hr">/);
});

test('tables render with header, body and alignment', () => {
  const html = renderMarkdown('| a | b |\n| :-- | --: |\n| 1 | 2 |');
  assert.match(html, /<table class="md-table">/);
  assert.match(html, /<th>a<\/th>/);
  assert.match(html, /<th style="text-align:right">b<\/th>/);
  assert.match(html, /<td>1<\/td>/);
  assert.match(html, /<td style="text-align:right">2<\/td>/);
});

test('paragraphs break on blank lines and keep soft line breaks soft', () => {
  const html = renderMarkdown('one\ntwo\n\nthree');
  assert.equal((html.match(/<p class="md-p">/g) || []).length, 2);
  assert.ok(!html.includes('<br'), 'a soft break must not become a hard break');
});

test('empty and nullish input produce no output', () => {
  assert.equal(renderMarkdown(''), '');
  assert.equal(renderMarkdown(null as any), '');
  assert.equal(renderMarkdown(undefined as any), '');
});

test('a fence carrying an info string beyond the language still parses', () => {
  // Regression guard: a stricter fence regex than the paragraph guard left this
  // line matching neither branch, and the paragraph loop spun forever.
  const html = renderMarkdown('```js title="x"\nbody\n```\n\nafter');
  assert.match(html, /<div class="md-code-lang">js<\/div>/);
  assert.match(html, /<pre><code>body<\/code><\/pre>/);
  assert.match(html, /<p class="md-p">after<\/p>/);
});

test('a nested list sits inside its parent item', () => {
  const html = renderMarkdown('- outer\n  - inner');
  assert.match(html, /<li class="md-li">outer<ul class="md-list"><li class="md-li">inner<\/li><\/ul><\/li>/);
});
