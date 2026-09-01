// src/modules/officeRenderer.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import {
  extensionOf,
  isImageName,
  isLegacyOfficeName,
  isOfficeName,
  renderOffice,
} from './officeRenderer.ts';

test('extensionOf lowercases and tolerates missing dots', () => {
  assert.equal(extensionOf('Report.XLSX'), 'xlsx');
  assert.equal(extensionOf('archive.tar.gz'), 'gz');
  assert.equal(extensionOf('Makefile'), '');
  assert.equal(extensionOf(''), '');
  assert.equal(extensionOf(null), '');
});

test('image, office and legacy office names are told apart', () => {
  assert.equal(isImageName('shot.PNG'), true);
  assert.equal(isImageName('icon.svg'), true);
  assert.equal(isImageName('notes.txt'), false);

  assert.equal(isOfficeName('book.xlsx'), true);
  assert.equal(isOfficeName('macros.xlsm'), true);
  assert.equal(isOfficeName('deck.pptx'), true);
  assert.equal(isOfficeName('letter.docx'), true);

  // The pre-2007 binaries are deliberately not "office" — they route to their
  // own message instead of being handed to the OOXML reader.
  assert.equal(isOfficeName('letter.doc'), false);
  assert.equal(isLegacyOfficeName('letter.doc'), true);
  assert.equal(isLegacyOfficeName('book.xls'), true);
  assert.equal(isLegacyOfficeName('book.xlsx'), false);
});

test('docx paragraphs render as paragraphs', () => {
  const html = renderOffice({ kind: 'docx', paragraphs: ['First', 'Second'], sheets: [] });
  assert.match(html, /<div class="office-doc office-doc--docx">/);
  assert.match(html, /<p class="office-p">First<\/p>/);
  assert.match(html, /<p class="office-p">Second<\/p>/);
});

test('document text is escaped, not interpreted', () => {
  const html = renderOffice({
    kind: 'docx',
    paragraphs: ['<script>alert(1)</script>'],
    sheets: [],
  });
  assert.ok(!html.includes('<script'), 'markup from the file must not survive');
  assert.match(html, /&lt;script&gt;/);
});

test('a pptx slide separator becomes a heading', () => {
  const html = renderOffice({
    kind: 'pptx',
    paragraphs: ['--- Slide 1 ---', 'Title text'],
    sheets: [],
  });
  assert.match(html, /<h3 class="office-slide">Slide 1<\/h3>/);
  assert.match(html, /<p class="office-p">Title text<\/p>/);
});

test('a sheet renders as a table with letter columns and row numbers', () => {
  const html = renderOffice({
    kind: 'xlsx',
    paragraphs: [],
    sheets: [{ name: 'Q3', rows: [['Name', 'Total'], ['Ann', '12']], truncated: false }],
  });
  assert.match(html, /<h3 class="office-sheet-name">Q3<\/h3>/);
  assert.match(html, /<th class="office-col">A<\/th>/);
  assert.match(html, /<th class="office-col">B<\/th>/);
  assert.match(html, /<th class="office-row-num">1<\/th>/);
  assert.match(html, /<td>Ann<\/td>/);
});

test('short rows are padded so columns stay aligned', () => {
  const html = renderOffice({
    kind: 'xlsx',
    paragraphs: [],
    sheets: [{ name: 'S', rows: [['a', 'b', 'c'], ['only']], truncated: false }],
  });
  // The second row must still emit three cells, or the table would skew.
  const secondRow = html.split('<th class="office-row-num">2</th>')[1];
  assert.equal((secondRow.match(/<td>/g) || []).length, 3);
});

test('column labels continue past Z into AA', () => {
  const wide = Array.from({ length: 27 }, (_, i) => `c${i}`);
  const html = renderOffice({
    kind: 'xlsx',
    paragraphs: [],
    sheets: [{ name: 'W', rows: [wide], truncated: false }],
  });
  assert.match(html, /<th class="office-col">Z<\/th>/);
  assert.match(html, /<th class="office-col">AA<\/th>/);
});

test('sheet names are escaped', () => {
  const html = renderOffice({
    kind: 'xlsx',
    paragraphs: [],
    sheets: [{ name: '<img onerror=x>', rows: [['v']], truncated: false }],
  });
  assert.ok(!html.includes('<img'), 'a crafted sheet name must not become a tag');
  assert.match(html, /&lt;img onerror=x&gt;/);
});

test('truncation is stated rather than silently hidden', () => {
  const sheet = renderOffice({
    kind: 'xlsx',
    paragraphs: [],
    sheets: [{ name: 'S', rows: [['a']], truncated: true }],
  });
  assert.match(sheet, /Sheet truncated/);

  const doc = renderOffice({ kind: 'docx', paragraphs: ['x'], sheets: [], truncated: true });
  assert.match(doc, /Document truncated/);
});

test('empty documents and workbooks say so', () => {
  assert.match(renderOffice({ kind: 'docx', paragraphs: [], sheets: [] }), /no extractable text/);
  assert.match(renderOffice({ kind: 'xlsx', paragraphs: [], sheets: [] }), /No sheets/);
  assert.match(
    renderOffice({ kind: 'xlsx', paragraphs: [], sheets: [{ name: 'S', rows: [] }] }),
    /Empty sheet/,
  );
});

test('a missing or malformed document yields nothing rather than throwing', () => {
  assert.equal(renderOffice(null as any), '');
  assert.equal(renderOffice(undefined as any), '');
  assert.equal(renderOffice('nonsense' as any), '');
});
