// src/modules/formatUtils.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import {
  escHtml,
  safeColor,
  fuzzyScore,
  filteredItems,
  fmtSize,
  fmtBytes,
  formatIsoLocal,
  fileExtFromBase,
  fileExtFullFromBase,
  fmtSizeExact,
  rowDateText,
  shortPath,
  renderDiffContent,
} from './formatUtils.ts';

test('escHtml escapes all HTML-significant characters', () => {
  assert.equal(escHtml('<script>'), '&lt;script&gt;');
  assert.equal(escHtml('a & b'), 'a &amp; b');
  assert.equal(escHtml('"'), '&quot;');
  assert.equal(escHtml("'"), '&#39;');
});

test('escHtml neutralizes attribute-breakout payloads', () => {
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
  assert.equal(safeColor('rgba(10, 20, 30, 0.5)'), 'rgba(10, 20, 30, 0.5)');
  assert.equal(safeColor('hsl(120, 100%, 50%)'), 'hsl(120, 100%, 50%)');
  assert.equal(safeColor('rebeccapurple'), 'rebeccapurple');
  // Injection attempts fall back.
  assert.equal(safeColor('url(https://evil/x)'), 'transparent');
  assert.equal(safeColor('red; background: url(x)'), 'transparent');
  assert.equal(safeColor('expression(alert(1))'), 'transparent');
  assert.equal(safeColor(null as any), 'transparent');
  assert.equal(safeColor('#fff', 'black'), '#fff');
  assert.equal(safeColor('nope!!', 'black'), 'black');
});

test('fuzzyScore calculates prefix, substring, subsequence, and zero matches', () => {
  assert.equal(fuzzyScore('test.ts', ''), 1);
  assert.equal(fuzzyScore('apple', 'app'), 1000 - 5);
  assert.equal(fuzzyScore('pineapple', 'app'), 500 - 4);
  assert.equal(fuzzyScore('axpxpxlxe', 'apple'), 100);
  assert.equal(fuzzyScore('apple', 'banana'), 0);
});

test('filteredItems filters with fuzzy scoring and handles parent dir', () => {
  const items = [
    { base: '..' },
    { base: 'app.ts', isDir: false },
    { base: 'apple.png', isDir: false },
    { base: 'zebra.txt', isDir: false },
  ];

  // With filter
  const filtered = filteredItems({ filter: 'app', items });
  assert.equal(filtered[0].base, '..');
  assert.equal(filtered.length, 3);
  assert.equal(filtered[1].base, 'app.ts');
  assert.equal(filtered[2].base, 'apple.png');
});

test('filteredItems sorts items by name, ext, size, and date with dirs first', () => {
  const items = [
    { base: 'z_folder', isDir: true, size: 50, mtime: 1000 },
    { base: 'a_folder', isDir: true, size: 10, mtime: 5000 },
    { base: 'file_b.png', isDir: false, size: 300, mtime: '2026-01-01' },
    { base: 'file_a.txt', isDir: false, size: 100, mtime: '2026-02-01' },
    { base: 'file_c.doc', isDir: false, size: 200, mtime: null },
    { base: '..' },
  ];

  // Sort by name asc
  const byNameAsc = filteredItems({ items, sortField: 'name', sortAsc: true });
  assert.equal(byNameAsc[0].base, '..');
  assert.equal(byNameAsc[1].base, 'a_folder');
  assert.equal(byNameAsc[2].base, 'z_folder');
  assert.equal(byNameAsc[3].base, 'file_a.txt');
  assert.equal(byNameAsc[4].base, 'file_b.png');
  assert.equal(byNameAsc[5].base, 'file_c.doc');

  // Sort by name desc
  const byNameDesc = filteredItems({ items, sortField: 'name', sortAsc: false });
  assert.equal(byNameDesc[0].base, '..');
  assert.equal(byNameDesc[1].base, 'z_folder');
  assert.equal(byNameDesc[2].base, 'a_folder');
  assert.equal(byNameDesc[3].base, 'file_c.doc');

  // Sort by ext asc
  const byExt = filteredItems({ items, sortField: 'ext', sortAsc: true });
  assert.equal(byExt[0].base, '..');
  assert.equal(byExt[3].base, 'file_c.doc'); // .doc < .png < .txt
  assert.equal(byExt[4].base, 'file_b.png');
  assert.equal(byExt[5].base, 'file_a.txt');

  // Sort by size asc
  const bySize = filteredItems({ items, sortField: 'size', sortAsc: true });
  assert.equal(bySize[0].base, '..');
  assert.equal(bySize[3].base, 'file_a.txt'); // 100 < 200 < 300
  assert.equal(bySize[4].base, 'file_c.doc');
  assert.equal(bySize[5].base, 'file_b.png');

  // Sort by date asc
  const byDate = filteredItems({ items, sortField: 'date', sortAsc: true });
  assert.equal(byDate[0].base, '..');
  assert.equal(byDate[3].base, 'file_c.doc'); // null date = 0
  assert.equal(byDate[4].base, 'file_b.png'); // 2026-01-01 < 2026-02-01
  assert.equal(byDate[5].base, 'file_a.txt');
});

test('fmtSize formats bytes correctly across units', () => {
  assert.equal(fmtSize(null), '');
  assert.equal(fmtSize(500), '500 B');
  assert.equal(fmtSize(2048), '2 kB');
  assert.equal(fmtSize(5 * 1024 * 1024), '5.0 MB');
  assert.equal(fmtSize(3.5 * 1024 * 1024 * 1024), '3.50 GB');
});

test('fmtBytes formats bytes or returns fallback on invalid values', () => {
  assert.equal(fmtBytes(null), '—');
  assert.equal(fmtBytes('abc' as any), '—');
  assert.equal(fmtBytes(500), '500 B');
  assert.equal(fmtBytes(1536), '1.5 kB');
  assert.equal(fmtBytes(2.5 * 1024 * 1024), '2.5 MB');
  assert.equal(fmtBytes(4.5 * 1024 * 1024 * 1024), '4.50 GB');
});

test('formatIsoLocal formats dates or handles invalid inputs', () => {
  assert.equal(formatIsoLocal(null), '—');
  assert.equal(formatIsoLocal('invalid-date'), 'invalid-date');
  const d = new Date('2026-06-15T12:00:00Z');
  const formatted = formatIsoLocal(d);
  assert.ok(formatted.length > 3);
});

test('fileExtFromBase and fileExtFullFromBase extract extensions safely', () => {
  assert.equal(fileExtFromBase(null), '—');
  assert.equal(fileExtFromBase({ base: '..', isDir: false }), '—');
  assert.equal(fileExtFromBase({ base: 'folder', isDir: true }), '—');
  assert.equal(fileExtFromBase({ base: 'noextension', isDir: false }), '—');
  assert.equal(fileExtFromBase({ base: '.hidden', isDir: false }), '—');
  assert.equal(fileExtFromBase({ base: 'trailing.', isDir: false }), '—');
  assert.equal(fileExtFromBase({ base: 'photo.JPG', isDir: false }), 'jpg');
  assert.equal(fileExtFromBase({ base: 'archive.verylongextensionname', isDir: false }), 'verylongexte');

  assert.equal(fileExtFullFromBase(null), '');
  assert.equal(fileExtFullFromBase({ base: '..', isDir: false }), '');
  assert.equal(fileExtFullFromBase({ base: 'folder', isDir: true }), '');
  assert.equal(fileExtFullFromBase({ base: 'noext', isDir: false }), '');
  assert.equal(fileExtFullFromBase({ base: '.dot', isDir: false }), '');
  assert.equal(fileExtFullFromBase({ base: 'doc.PDF', isDir: false }), 'pdf');
  assert.equal(fileExtFullFromBase({ base: 'archive.verylongextensionname', isDir: false }), 'verylongextensionname');
});

test('fmtSizeExact formats byte counts', () => {
  assert.equal(fmtSizeExact(null), '');
  assert.equal(fmtSizeExact(1024), '1,024 bytes');
});

test('rowDateText formats mtime or handles invalid/missing values', () => {
  assert.equal(rowDateText(null), '—');
  assert.equal(rowDateText({ base: '..' }), '—');
  assert.equal(rowDateText({ base: 'file.txt', mtime: null as any }), '—');
  assert.equal(rowDateText({ base: 'file.txt', mtime: 'invalid' }), '—');
  const text = rowDateText({ base: 'file.txt', mtime: '2026-05-10T14:30:00' });
  assert.ok(text.includes('10 May 2026'));
});

test('shortPath truncates long paths with ellipsis', () => {
  assert.equal(shortPath('C:\\short\\path'), 'C:\\short\\path');
  const longP = 'C:\\Users\\admin\\VeryLongDirectoryNameThatExceedsTheFiftyTwoCharacterLimit\\file.txt';
  const shortened = shortPath(longP);
  assert.ok(shortened.startsWith('…'));
  assert.equal(shortened.length, 52);
});

test('renderDiffContent handles empty diff, regular diff lines, and line truncation', () => {
  function makeMockContainer(): any {
    const children: any[] = [];
    return {
      children,
      replaceChildren() { children.length = 0; },
      appendChild(c: any) {
        if (c.isFrag) {
          children.push(...c.children);
        } else {
          children.push(c);
        }
      },
    };
  }

  function makeMockElement(cls: string): any {
    const classes = new Set<string>();
    if (cls) classes.add(cls);
    return {
      className: cls,
      classList: {
        add(c: string) { classes.add(c); },
        contains(c: string) { return classes.has(c); },
      },
      textContent: '',
    };
  }

  const origDoc = globalThis.document;
  (globalThis as any).document = {
    createElement: (tag: string) => makeMockElement(''),
    createDocumentFragment: () => {
      const children: any[] = [];
      return {
        isFrag: true,
        children,
        appendChild(c: any) { children.push(c); },
      };
    },
  };

  try {
    const container = makeMockContainer();

    // Empty diff
    renderDiffContent(container, '');
    assert.equal(container.children.length, 1);
    assert.equal(container.children[0].textContent, 'No changes.');

    // Normal diff
    const diff = `--- a/file.txt\n+++ b/file.txt\n@@ -1,2 +1,2 @@\n-old line\n+new line\n unchanged`;
    renderDiffContent(container, diff);
    assert.equal(container.children.length, 6);
    assert.ok(container.children[0].classList.contains('diff-line--file'));
    assert.ok(container.children[1].classList.contains('diff-line--file'));
    assert.ok(container.children[2].classList.contains('diff-line--hunk'));
    assert.ok(container.children[3].classList.contains('diff-line--del'));
    assert.ok(container.children[4].classList.contains('diff-line--add'));

    // Huge diff > 2500 lines
    const bigLines = Array.from({ length: 2600 }, (_, i) => `line ${i}`).join('\n');
    renderDiffContent(container, bigLines);
    assert.equal(container.children.length, 2501); // 1 truncation note + 2500 lines
    assert.ok(container.children[0].textContent.includes('UI shows first 2500'));
  } finally {
    globalThis.document = origDoc;
  }
});
