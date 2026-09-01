// src/modules/officeRenderer.ts
import { escHtml } from './formatUtils.ts';

/*
  Turns the structure fs_read_office returns into viewer markup.

  Everything the backend extracted came out of a file the user happened to open,
  so it is treated as untrusted text: escHtml runs over every cell, paragraph and
  sheet name, and the only tags here are the ones written below.
*/

const IMAGE_EXTENSIONS = new Set([
  'png',
  'jpg',
  'jpeg',
  'gif',
  'webp',
  'bmp',
  'ico',
  'avif',
  'svg',
]);

const OFFICE_EXTENSIONS = new Set(['docx', 'xlsx', 'xlsm', 'pptx']);

// Rejected up front so the viewer can explain itself instead of showing the
// mojibake that a binary run through a text decoder produces.
const LEGACY_OFFICE_EXTENSIONS = new Set(['doc', 'xls', 'ppt']);

export interface OfficeSheet {
  name?: string;
  rows?: any[][];
  truncated?: boolean;
}

export interface OfficeDoc {
  kind?: string;
  paragraphs?: string[];
  sheets?: OfficeSheet[];
  truncated?: boolean;
}

export function extensionOf(name?: string | null): string {
  const dot = String(name || '').lastIndexOf('.');
  return dot < 0 ? '' : String(name).slice(dot + 1).toLowerCase();
}

export function isImageName(name: string): boolean {
  return IMAGE_EXTENSIONS.has(extensionOf(name));
}

export function isOfficeName(name: string): boolean {
  return OFFICE_EXTENSIONS.has(extensionOf(name));
}

export function isLegacyOfficeName(name: string): boolean {
  return LEGACY_OFFICE_EXTENSIONS.has(extensionOf(name));
}

function renderParagraphs(doc: OfficeDoc): string {
  if (!doc.paragraphs?.length) {
    return '<p class="office-empty">This document holds no extractable text.</p>';
  }
  return doc.paragraphs
    .map((para) => {
      const text = String(para ?? '');
      if (!text.trim()) return '<p class="office-p office-p--blank"></p>';
      // pptx marks slide boundaries with its own separator line.
      if (/^--- Slide \d+ ---$/.test(text)) {
        return `<h3 class="office-slide">${escHtml(text.replace(/^--- | ---$/g, ''))}</h3>`;
      }
      return `<p class="office-p">${escHtml(text)}</p>`;
    })
    .join('');
}

// Excel numbers columns A, B … Z, AA — the header row mirrors that so a cell
// referenced in a formula can still be located by eye.
function columnLabel(index: number): string {
  let n = index + 1;
  let out = '';
  while (n > 0) {
    const rem = (n - 1) % 26;
    out = String.fromCharCode(65 + rem) + out;
    n = Math.floor((n - 1) / 26);
  }
  return out;
}

function renderSheet(sheet: OfficeSheet): string {
  const rows = sheet.rows || [];
  const width = rows.reduce((max, r) => Math.max(max, r.length), 0);
  const name = escHtml(sheet.name || 'Sheet');

  if (!width) {
    return `<section class="office-sheet"><h3 class="office-sheet-name">${name}</h3>` +
      '<p class="office-empty">Empty sheet.</p></section>';
  }

  const head = Array.from(
    { length: width },
    (_, i) => `<th class="office-col">${columnLabel(i)}</th>`,
  ).join('');

  const body = rows
    .map((row, r) => {
      const cells = Array.from({ length: width }, (_, c) => {
        const value = String(row[c] ?? '');
        return `<td>${escHtml(value)}</td>`;
      }).join('');
      return `<tr><th class="office-row-num">${r + 1}</th>${cells}</tr>`;
    })
    .join('');

  const note = sheet.truncated
    ? '<p class="office-note">Sheet truncated — open it in a spreadsheet app for the rest.</p>'
    : '';

  return (
    `<section class="office-sheet"><h3 class="office-sheet-name">${name}</h3>` +
    '<div class="office-table-wrap"><table class="office-table"><thead><tr>' +
    `<th class="office-corner"></th>${head}</tr></thead><tbody>${body}</tbody></table></div>${note}</section>`
  );
}

export function renderOffice(doc: OfficeDoc | null | undefined): string {
  if (!doc || typeof doc !== 'object') return '';

  const body =
    doc.kind === 'xlsx'
      ? (doc.sheets || []).map(renderSheet).join('') ||
        '<p class="office-empty">No sheets in this workbook.</p>'
      : renderParagraphs(doc);

  const note =
    doc.truncated && doc.kind !== 'xlsx'
      ? '<p class="office-note">Document truncated — this is a preview, not the whole file.</p>'
      : '';

  return `<div class="office-doc office-doc--${escHtml(doc.kind || 'unknown')}">${body}${note}</div>`;
}
