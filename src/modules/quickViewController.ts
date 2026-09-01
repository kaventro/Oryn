// src/modules/quickViewController.ts
import { escHtml, renderDiffContent, fmtBytes } from './formatUtils.ts';
import { NON_TEXT_PREVIEW_EXTS, looksBinaryText } from './binaryPreview.ts';
import { renderMarkdown, isMarkdownName } from './markdownRenderer.ts';
import { renderOffice, isOfficeName, isLegacyOfficeName } from './officeRenderer.ts';
import { highlightCode } from './syntaxHighlighter.ts';

export interface QuickViewDeps {
  state: any;
  api: () => any;
  otherSide: (side: 'left' | 'right') => 'left' | 'right';
  getFilteredSelection: (side: 'left' | 'right') => { item: any; count?: number; total?: number; vis?: any[] };
  fullPath: (pane: any, item: any) => Promise<string | null>;
  setStatus?: (msg: string) => void;
  renderMarkdown?: (md: string) => string;
  renderOffice?: (bytes: any, ext: string) => string;
  iconRegistry?: any;
  fmtBytes?: (b: number) => string;
  looksBinaryText?: (s: string) => boolean;
  openSelected?: (side: 'left' | 'right') => void;
  isImageName?: (name: string) => boolean;
  isMarkdownName?: (name: string) => boolean;
  isOfficeName?: (name: string) => boolean;
  isLegacyOfficeName?: (name: string) => boolean;
  isRemotePath?: (p: string) => any;
}

export class QuickViewController {
  private state: any;
  private api: () => any;
  private otherSide: (side: 'left' | 'right') => 'left' | 'right';
  private getFilteredSelection: (side: 'left' | 'right') => { item: any; count?: number; total?: number; vis?: any[] };
  private fullPath: (pane: any, item: any) => Promise<string | null>;
  public mode: 'preview' | 'diff' | 'blame';

  constructor(deps: QuickViewDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.otherSide = deps.otherSide;
    this.getFilteredSelection = deps.getFilteredSelection;
    this.fullPath = deps.fullPath;

    this.mode = 'preview'; // 'preview' | 'diff' | 'blame'
  }

  toggle(side: 'left' | 'right'): boolean {
    const pane = this.state[side];
    if (!pane) return false;
    pane.quickViewActive = !pane.quickViewActive;
    return pane.quickViewActive;
  }

  hide(side: 'left' | 'right'): void {
    if (this.state[side]) {
      this.state[side].quickViewActive = false;
    }
  }

  setMode(mode: 'preview' | 'diff' | 'blame', side: 'left' | 'right'): void {
    this.mode = mode;
    void this.render(side);
  }

  getFileSrc(fullPath: string): string {
    if (!fullPath) return '';
    try {
      const apiObj = typeof this.api === 'function' ? this.api() : this.api;
      if (apiObj?.assetUrl) {
        return apiObj.assetUrl(fullPath);
      }
    } catch (_) {}
    return fullPath;
  }

  async render(side: 'left' | 'right'): Promise<boolean> {
    const pane = this.state[side];
    const hostEl = document.getElementById(`list-${side}`);
    if (!hostEl || !pane) return false;

    if (!pane.quickViewActive) {
      return false;
    }

    const sourceSide = this.otherSide ? this.otherSide(side) : (side === 'left' ? 'right' : 'left');
    const sourcePane = this.state[sourceSide];
    const { item } = this.getFilteredSelection ? this.getFilteredSelection(sourceSide) : { item: null };

    hostEl.replaceChildren();

    const container = document.createElement('div');
    container.className = 'quick-view-container';

    if (!item || item.base === '..' || item.isDir) {
      container.innerHTML = `<div class="quick-view-empty">No file selected for Quick View</div>`;
      hostEl.appendChild(container);
      return true;
    }

    const fp = this.fullPath ? await this.fullPath(sourcePane, item) : null;
    if (!fp) return true;

    // Header with mode toolbar
    const header = document.createElement('div');
    header.className = 'quick-view-header';

    const title = document.createElement('span');
    title.className = 'quick-view-title';
    title.textContent = item.base;
    title.title = fp;

    const tabs = document.createElement('div');
    tabs.className = 'quick-view-tabs';

    const isGit = !!sourcePane.git?.isRepo;

    const makeTab = (modeKey: 'preview' | 'diff' | 'blame', label: string) => {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = `quick-view-tab${this.mode === modeKey ? ' quick-view-tab--active' : ''}`;
      btn.textContent = label;
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        this.setMode(modeKey, side);
      });
      return btn;
    };

    tabs.appendChild(makeTab('preview', 'Preview'));
    if (isGit) {
      tabs.appendChild(makeTab('diff', 'Diff (HEAD)'));
      tabs.appendChild(makeTab('blame', 'Blame'));
    }

    header.append(title, tabs);
    container.appendChild(header);

    const body = document.createElement('div');
    body.className = 'quick-view-body';
    body.innerHTML = `<div class="quick-view-loading">Loading ${escHtml(item.base)}…</div>`;
    container.appendChild(body);
    hostEl.appendChild(container);

    let relPath = item.base;
    if (isGit && sourcePane.git?.root) {
      if (fp.startsWith(sourcePane.git.root)) {
        relPath = fp.slice(sourcePane.git.root.length).replace(/^[/\\]+/, '');
      }
    }

    try {
      if (this.mode === 'diff' && isGit) {
        const r = await this.api().gitDiff(sourcePane.git.root, 'HEAD', null, relPath);
        body.replaceChildren();
        if (!r.ok) {
          body.innerHTML = `<div class="quick-view-error">${escHtml(r.error || 'Failed to load diff')}</div>`;
        } else if (!r.diff || !r.diff.trim()) {
          body.innerHTML = `<div class="quick-view-empty">✓ No changes against HEAD</div>`;
        } else {
          const pre = document.createElement('div');
          pre.className = 'quick-view-diff nx-mono';
          renderDiffContent(pre, r.diff);
          body.appendChild(pre);
        }
      } else if (this.mode === 'blame' && isGit) {
        const r = await this.api().gitBlame(sourcePane.git.root, relPath);
        body.replaceChildren();
        if (!r.ok) {
          body.innerHTML = `<div class="quick-view-error">${escHtml(r.error || 'Failed to load blame')}</div>`;
        } else if (!r.lines || r.lines.length === 0) {
          body.innerHTML = `<div class="quick-view-empty">No blame lines found</div>`;
        } else {
          const pre = document.createElement('div');
          pre.className = 'quick-view-blame nx-mono';
          r.lines.forEach((line: any) => {
            const row = document.createElement('div');
            row.className = 'git-blame-row';
            row.innerHTML = `
              <span class="git-blame-gutter">
                <span class="git-blame-num">${line.lineNum}</span>
                <span class="git-blame-hash" title="${escHtml(line.summary)}">${escHtml(line.commitShort)}</span>
                <span class="git-blame-author" title="${escHtml(line.author)}">${escHtml(line.author)}</span>
              </span>
              <span class="git-blame-code">${escHtml(line.content)}</span>
            `;
            pre.appendChild(row);
          });
          body.appendChild(pre);
        }
      } else {
        // Preview mode: Image / Media / Markdown / Office / Code
        const ext = (item.base.split('.').pop() || '').toLowerCase();
        const imgExts = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'bmp', 'ico'];
        const audioExts = ['mp3', 'wav', 'ogg', 'm4a', 'flac', 'aac'];
        const videoExts = ['mp4', 'webm', 'mov', 'm4v'];

        if (imgExts.includes(ext)) {
          body.replaceChildren();
          const wrap = document.createElement('div');
          wrap.className = 'quick-view-media-wrap';

          const img = document.createElement('img');
          img.className = 'quick-view-img';
          img.src = this.getFileSrc(fp);
          img.alt = item.base;

          const meta = document.createElement('div');
          meta.className = 'quick-view-meta';
          meta.textContent = `${ext.toUpperCase()} Image • ${fmtBytes(item.size || 0)}`;

          img.onload = () => {
            meta.textContent = `${ext.toUpperCase()} Image • ${img.naturalWidth} × ${img.naturalHeight} px • ${fmtBytes(item.size || 0)}`;
          };
          img.onerror = async () => {
            try {
              const apiObj = typeof this.api === 'function' ? this.api() : this.api;
              if (apiObj?.readMediaDataUrl) {
                const dataUrl = await apiObj.readMediaDataUrl(fp);
                img.onerror = () => {
                  body.innerHTML = `<div class="quick-view-error">Cannot preview image ${escHtml(item.base)}</div>`;
                };
                img.src = dataUrl;
                return;
              }
            } catch (_) {}
            body.innerHTML = `<div class="quick-view-error">Cannot preview image ${escHtml(item.base)}</div>`;
          };

          wrap.append(img, meta);
          body.appendChild(wrap);
        } else if (audioExts.includes(ext)) {
          body.replaceChildren();
          const wrap = document.createElement('div');
          wrap.className = 'quick-view-media-wrap';
          const audio = document.createElement('audio');
          audio.controls = true;
          audio.src = this.getFileSrc(fp);
          audio.onerror = async () => {
            try {
              const apiObj = typeof this.api === 'function' ? this.api() : this.api;
              if (apiObj?.readMediaDataUrl) {
                const dataUrl = await apiObj.readMediaDataUrl(fp);
                audio.onerror = null;
                audio.src = dataUrl;
              }
            } catch (_) {}
          };
          const meta = document.createElement('div');
          meta.className = 'quick-view-meta';
          meta.textContent = `Audio (${ext.toUpperCase()}) • ${fmtBytes(item.size || 0)}`;
          wrap.append(audio, meta);
          body.appendChild(wrap);
        } else if (videoExts.includes(ext)) {
          body.replaceChildren();
          const wrap = document.createElement('div');
          wrap.className = 'quick-view-media-wrap';
          const video = document.createElement('video');
          video.controls = true;
          video.style.maxWidth = '100%';
          video.style.maxHeight = '70%';
          video.src = this.getFileSrc(fp);
          video.onerror = async () => {
            try {
              const apiObj = typeof this.api === 'function' ? this.api() : this.api;
              if (apiObj?.readMediaDataUrl) {
                const dataUrl = await apiObj.readMediaDataUrl(fp);
                video.onerror = null;
                video.src = dataUrl;
              }
            } catch (_) {}
          };
          wrap.append(video);
          body.appendChild(wrap);
        } else if (isMarkdownName(item.base)) {
          // Rich Markdown Preview
          const content = await this.api().readFileText(fp, 512000);
          body.replaceChildren();
          const mdWrap = document.createElement('div');
          mdWrap.className = 'quick-view-md-wrap viewer-content--md';
          mdWrap.innerHTML = renderMarkdown(content);
          body.appendChild(mdWrap);
        } else if (isOfficeName(item.base)) {
          // Office (.docx, .xlsx, .pptx) Preview
          const doc = await this.api().readOffice(fp);
          body.replaceChildren();
          const officeWrap = document.createElement('div');
          officeWrap.className = 'quick-view-office-wrap viewer-content--office';
          officeWrap.innerHTML = renderOffice(doc);
          body.appendChild(officeWrap);
        } else if (isLegacyOfficeName(item.base)) {
          body.innerHTML = `<div class="quick-view-empty">Legacy Office binary format (.${ext}) — please open in native application.</div>`;
        } else if (NON_TEXT_PREVIEW_EXTS.has(ext)) {
          body.innerHTML = `<div class="quick-view-empty">Can't preview ${escHtml(item.base)} as text — it's a binary or archive file.</div>`;
        } else {
          // Text / Code preview with Syntax Highlighting and line numbering
          const content = await this.api().readFileText(fp, 512000);
          if (looksBinaryText(content)) {
            body.innerHTML = `<div class="quick-view-empty">Can't preview ${escHtml(item.base)} as text — it looks like a binary file.</div>`;
            return true;
          }
          body.replaceChildren();

          const codeWrap = document.createElement('div');
          codeWrap.className = 'quick-view-code-wrap';

          const lines = content.split('\n');
          const maxLines = 1500;
          const displayLines = lines.slice(0, maxLines);

          const gutter = document.createElement('div');
          gutter.className = 'quick-view-code-gutter nx-mono';
          gutter.textContent = displayLines.map((_: string, i: number) => i + 1).join('\n');

          const codeBody = document.createElement('pre');
          codeBody.className = 'quick-view-code-body nx-mono';
          const highlightedHtml = highlightCode(displayLines.join('\n'), item.base);
          codeBody.innerHTML = highlightedHtml + (lines.length > maxLines ? `\n<span class="tok-com">…[truncated at ${maxLines} lines]</span>` : '');

          codeWrap.append(gutter, codeBody);
          body.appendChild(codeWrap);
        }
      }
    } catch (e: any) {
      body.innerHTML = `<div class="quick-view-error">Cannot preview file: ${escHtml(e?.message || 'Binary or inaccessible')}</div>`;
    }

    return true;
  }
}
