// src/modules/markdownRenderer.ts
import { escHtml } from './formatUtils.ts';
import { highlightCode } from './syntaxHighlighter.ts';

/*
  Markdown → HTML for the F3 viewer and Quick View.

  The viewer opens whatever file the user points at, so the input is untrusted:
  a .md file can carry <script> or an onerror attribute just as easily as a
  heading. This renderer is therefore safe by construction rather than by
  sanitising afterwards — every run of source text goes through escHtml before
  any tag is produced, and the only tags in the output are the ones written
  here. Raw HTML embedded in the markdown is shown as literal text, which is
  the honest thing to do in a file manager anyway.
*/

const MD_EXTENSIONS = new Set(['md', 'markdown', 'mdown', 'mkd', 'mdx']);

// Private-use sentinels that fence a lifted code span. They are stripped from
// the input first, so a file cannot smuggle one in to corrupt the restore pass.
const CODE_OPEN = String.fromCharCode(0xe000);
const CODE_CLOSE = String.fromCharCode(0xe001);

export function isMarkdownName(name: string): boolean {
  const dot = String(name || '').lastIndexOf('.');
  if (dot < 0) return false;
  return MD_EXTENSIONS.has(String(name).slice(dot + 1).toLowerCase());
}

/*
  Only absolute web and mail targets become clickable. Anything else — a
  relative path like (LICENSE), a javascript: payload, a file:// or UNC target —
  is rendered as plain text, because the click handler hands the URL to the OS
  shell and opening an arbitrary local target from a document would be a way to
  launch things the user never asked for.
*/
export function safeUrl(url: any): string | null {
  const t = String(url == null ? '' : url).trim();
  // Reject whitespace and control characters outright: a crafted URL must not
  // be able to smuggle a newline or a tab past the scheme check below.
  if (!t) return null;
  for (let n = 0; n < t.length; n++) {
    const c = t.charCodeAt(n);
    if (c <= 0x20 || c === 0x7f) return null;
  }
  return /^(https?:\/\/|mailto:)/i.test(t) ? t : null;
}

function emphasis(s: string): string {
  return s
    .replace(/\*\*(?=\S)([\s\S]*?\S)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[\s(])__(?=\S)([\s\S]*?\S)__(?=[\s).,;:!?]|$)/g, '$1<strong>$2</strong>')
    .replace(/~~(?=\S)([\s\S]*?\S)~~/g, '<del>$1</del>')
    .replace(/\*(?=\S)([^*]*?\S)\*/g, '<em>$1</em>')
    // `_` only counts at a word boundary so snake_case identifiers survive.
    .replace(/(^|[\s(])_(?=\S)([^_]*?\S)_(?=[\s).,;:!?]|$)/g, '$1<em>$2</em>');
}

function linkTag(text: string, rawHref: string): string {
  const href = safeUrl(rawHref);
  const label = emphasis(text);
  if (!href) {
    // Keep the target visible on hover so a relative link still tells the
    // reader where it pointed.
    return `<span class="md-link-plain" title="${rawHref}">${label}</span>`;
  }
  return `<a class="md-link" href="${href}" data-md-href="${href}">${label}</a>`;
}

function inline(raw: string): string {
  // Code spans are lifted out before anything else so their contents are never
  // treated as markup, then restored last.
  const codes: string[] = [];
  let s = String(raw)
    .split(CODE_OPEN)
    .join('')
    .split(CODE_CLOSE)
    .join('')
    .replace(/`+([^`]+?)`+/g, (_, code) => {
      codes.push(code);
      return CODE_OPEN + (codes.length - 1) + CODE_CLOSE;
    });

  s = escHtml(s);

  // Images resolve to their alt text: the app's CSP blocks remote images, and a
  // relative src cannot be resolved from inside the viewer, so a real <img>
  // would only ever render as a broken icon.
  s = s.replace(/!\[([^\]]*)\]\(([^)]*)\)/g, (_, alt, src) => {
    const target = String(src).trim().split(/\s+/)[0] || '';
    return `<span class="md-image" title="${target}">${alt || 'image'}</span>`;
  });

  s = s.replace(/\[([^\]]*)\]\(([^)]*)\)/g, (_, text, target) => {
    // Strip an optional link title: [x](https://a "title")
    const href = String(target).trim().split(/\s+/)[0] || '';
    return linkTag(text, href);
  });

  // Autolinks: <https://…> survives escaping as &lt;https://…&gt;
  s = s.replace(/&lt;((?:https?:\/\/|mailto:)[^\s&]+)&gt;/g, (_, url) => linkTag(url, url));

  s = emphasis(s);

  return s.replace(
    new RegExp(CODE_OPEN + '(\\d+)' + CODE_CLOSE, 'g'),
    (_, i) => `<code class="md-code-inline">${escHtml(codes[Number(i)])}</code>`,
  );
}

function isBlank(line: string): boolean {
  return !String(line).trim();
}

function listMarker(line: string) {
  const m = String(line).match(/^(\s*)([-*+]|\d{1,9}[.)])\s+(.*)$/);
  if (!m) return null;
  return {
    indent: m[1].replace(/\t/g, '    ').length,
    ordered: /\d/.test(m[2]),
    text: m[3],
  };
}

function isDelimiterRow(line: string): boolean {
  return /^\s*\|?(\s*:?-+:?\s*\|)+\s*:?-*:?\s*\|?\s*$/.test(String(line));
}

function splitRow(line: string): string[] {
  let s = String(line).trim();
  if (s.startsWith('|')) s = s.slice(1);
  if (s.endsWith('|')) s = s.slice(0, -1);
  return s.split('|').map((c) => c.trim());
}

function alignments(delimiter: string): string[] {
  return splitRow(delimiter).map((c) => {
    const left = c.startsWith(':');
    const right = c.endsWith(':');
    if (left && right) return ' style="text-align:center"';
    if (right) return ' style="text-align:right"';
    return '';
  });
}

function renderBlocks(lines: string[]): string {
  const out: string[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    if (isBlank(line)) {
      i++;
      continue;
    }

    // The info string is matched loosely (```js title="x" is common). It must
    // stay in sync with the paragraph guard below, which treats the same prefix
    // as a block start — a stricter regex here would leave such a line matching
    // neither branch, and the paragraph loop would then never advance.
    const fence = line.match(/^\s{0,3}(`{3,}|~{3,})\s*(\S*)/);
    if (fence) {
      const closer = fence[1][0] === '`' ? '`' : '~';
      const body: string[] = [];
      i++;
      while (i < lines.length && !new RegExp('^\\s{0,3}' + closer + '{3,}\\s*$').test(lines[i])) {
        body.push(lines[i]);
        i++;
      }
      i++; // consume the closing fence, or run off the end on an unclosed block
      const lang = fence[2] ? `<div class="md-code-lang">${escHtml(fence[2])}</div>` : '';
      const highlighted = fence[2] ? highlightCode(body.join('\n'), fence[2]) : escHtml(body.join('\n'));
      out.push(`<div class="md-code">${lang}<pre><code>${highlighted}</code></pre></div>`);
      continue;
    }

    const heading = line.match(/^\s{0,3}(#{1,6})\s+(.*)$/);
    if (heading) {
      const level = heading[1].length;
      const text = heading[2].replace(/\s+#+\s*$/, '');
      out.push(`<h${level} class="md-h md-h${level}">${inline(text)}</h${level}>`);
      i++;
      continue;
    }

    // Setext heading: a line underlined with === or ---
    if (i + 1 < lines.length && /^\s{0,3}(={2,}|-{2,})\s*$/.test(lines[i + 1]) && !listMarker(line)) {
      const level = lines[i + 1].trim().startsWith('=') ? 1 : 2;
      out.push(`<h${level} class="md-h md-h${level}">${inline(line.trim())}</h${level}>`);
      i += 2;
      continue;
    }

    if (/^\s{0,3}([-*_])\s*(\1\s*){2,}$/.test(line)) {
      out.push('<hr class="md-hr">');
      i++;
      continue;
    }

    if (/^\s{0,3}>/.test(line)) {
      const inner: string[] = [];
      while (i < lines.length && /^\s{0,3}>/.test(lines[i])) {
        inner.push(lines[i].replace(/^\s{0,3}>\s?/, ''));
        i++;
      }
      out.push(`<blockquote class="md-quote">${renderBlocks(inner)}</blockquote>`);
      continue;
    }

    if (line.includes('|') && i + 1 < lines.length && isDelimiterRow(lines[i + 1])) {
      const align = alignments(lines[i + 1]);
      const head = splitRow(line);
      const rows: string[][] = [];
      i += 2;
      while (i < lines.length && !isBlank(lines[i]) && lines[i].includes('|')) {
        rows.push(splitRow(lines[i]));
        i++;
      }
      const th = head.map((c, n) => `<th${align[n] || ''}>${inline(c)}</th>`).join('');
      const tb = rows
        .map((r) => `<tr>${head.map((_, n) => `<td${align[n] || ''}>${inline(r[n] || '')}</td>`).join('')}</tr>`)
        .join('');
      out.push(
        `<div class="md-table-wrap"><table class="md-table"><thead><tr>${th}</tr></thead><tbody>${tb}</tbody></table></div>`,
      );
      continue;
    }

    if (listMarker(line)) {
      const list = renderList(lines, i);
      out.push(list.html);
      i = list.next;
      continue;
    }

    // Paragraph: runs until a blank line or the start of another block. Soft
    // line breaks stay soft, matching how GitHub renders a README.
    const para: string[] = [];
    while (
      i < lines.length &&
      !isBlank(lines[i]) &&
      !/^\s{0,3}(#{1,6}\s|>|`{3,}|~{3,})/.test(lines[i]) &&
      !listMarker(lines[i]) &&
      !/^\s{0,3}([-*_])\s*(\1\s*){2,}$/.test(lines[i])
    ) {
      para.push(lines[i].trim());
      i++;
    }
    // Safety net: the line reached this branch, so it belongs to a paragraph
    // whatever the guard above thinks. Without this an unforeseen mismatch
    // between the two would spin here forever and hang the window.
    if (!para.length) {
      para.push(lines[i].trim());
      i++;
    }
    out.push(`<p class="md-p">${inline(para.join('\n'))}</p>`);
  }

  return out.join('\n');
}

function renderList(lines: string[], start: number): { html: string; next: number } {
  const first = listMarker(lines[start]);
  if (!first) return { html: '', next: start + 1 };
  const tag = first.ordered ? 'ol' : 'ul';
  const items: string[] = [];
  let i = start;

  while (i < lines.length) {
    const marker = listMarker(lines[i]);
    if (!marker) {
      // A blank line only ends the list if no further item follows it.
      if (isBlank(lines[i]) && listMarker(lines[i + 1] || '')) {
        i++;
        continue;
      }
      break;
    }
    if (marker.indent < first.indent || marker.ordered !== first.ordered) break;

    if (marker.indent > first.indent) {
      const nested = renderList(lines, i);
      // A sublist belongs inside the item above it, not as a sibling of it.
      if (items.length) {
        items[items.length - 1] = items[items.length - 1].replace(/<\/li>$/, nested.html + '</li>');
      } else {
        items.push(nested.html);
      }
      i = nested.next;
      continue;
    }

    const body = [marker.text];
    i++;
    // Lazy continuation lines belong to the item they follow.
    while (i < lines.length && !isBlank(lines[i]) && !listMarker(lines[i])) {
      body.push(lines[i].trim());
      i++;
    }

    const task = body[0].match(/^\[([ xX])\]\s+(.*)$/);
    if (task) {
      const checked = task[1].toLowerCase() === 'x' ? ' checked' : '';
      body[0] = task[2];
      items.push(
        `<li class="md-li md-task"><input type="checkbox" disabled${checked}> ${inline(body.join('\n'))}</li>`,
      );
    } else {
      items.push(`<li class="md-li">${inline(body.join('\n'))}</li>`);
    }
  }

  return { html: `<${tag} class="md-list">${items.join('')}</${tag}>`, next: i };
}

export function renderMarkdown(text: string | null | undefined): string {
  const lines = String(text == null ? '' : text)
    .replace(/\r\n?/g, '\n')
    .split('\n');
  return renderBlocks(lines);
}
