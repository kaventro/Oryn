// src/modules/gitController.ts
import { escHtml, renderDiffContent } from './formatUtils.ts';
import type { AppState } from './stateModels.ts';

export interface GitControllerDeps {
  api: () => any;
  state: any;
  setStatus: (msg: string) => void;
  loadDir: (side: 'left' | 'right') => Promise<void>;
  focusActiveList: () => void;
}

export class GitController {
  public api: () => any;
  public state: AppState;
  public setStatus: (msg: string) => void;
  public loadDir: (side: 'left' | 'right') => Promise<void>;
  public focusActiveList: () => void;

  public _repoPath: string | null;
  public _filePath: string | null;
  public _currentTab: string;
  public _onDocKey: ((e: KeyboardEvent) => void) | null;

  constructor(deps: GitControllerDeps) {
    this.api = deps.api;
    this.state = deps.state;
    this.setStatus = deps.setStatus;
    this.loadDir = deps.loadDir;
    this.focusActiveList = deps.focusActiveList;

    this._repoPath = null;
    this._filePath = null;
    this._currentTab = 'status';
    this._onDocKey = null;
  }

  private _showOverlay(): void {
    const overlay = document.getElementById('git-overlay');
    overlay?.classList.remove('hidden');
    overlay?.setAttribute('aria-hidden', 'false');
    if (!this._onDocKey) {
      this._onDocKey = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
          e.preventDefault();
          this.close();
        }
      };
      document.addEventListener('keydown', this._onDocKey);
    }
  }

  public close(): void {
    const overlay = document.getElementById('git-overlay');
    overlay?.classList.add('hidden');
    overlay?.setAttribute('aria-hidden', 'true');
    if (this._onDocKey) {
      document.removeEventListener('keydown', this._onDocKey);
      this._onDocKey = null;
    }
    if (typeof this.focusActiveList === 'function') {
      this.focusActiveList();
    }
  }

  public hide(): void {
    this.close();
  }

  public async open(filePath: string | null = null): Promise<void> {
    this._showOverlay();
    const pane = this.state[this.state.active];
    const dir = pane.path || '';
    const badge = document.getElementById('git-branch-badge');
    if (badge) badge.textContent = 'Detecting repo…';
    const statusBar = document.getElementById('git-status-bar');
    if (statusBar) statusBar.textContent = '';
    const isRepo = await this.api().gitIsRepo(dir);
    if (!isRepo.ok) {
      if (badge) badge.textContent = 'Not a git repo';
      const statusList = document.getElementById('git-status-list');
      if (statusList) {
        statusList.innerHTML =
          '<div class="git-no-repo">No git repository found in current folder.</div>';
      }
      return;
    }
    this._repoPath = isRepo.root;

    if (filePath) {
      this._filePath = filePath;
    } else {
      const sel = pane?.items?.[pane.cursor];
      if (sel && sel.base && sel.base !== '..' && !sel.isDir) {
        const full = (pane.path ? `${pane.path.replace(/[/\\]$/, '')}/` : '') + sel.base;
        if (this._repoPath && full.startsWith(this._repoPath)) {
          this._filePath = full.slice(this._repoPath.length).replace(/^[/\\]+/, '');
        } else {
          this._filePath = sel.base;
        }
      }
    }

    const fileInput = document.getElementById('git-blame-file') as HTMLInputElement | null;
    if (fileInput && this._filePath) fileInput.value = this._filePath;

    if (badge) badge.textContent = '🔍 Loading…';
    this._switchTab('status');
    await this._refreshStatus();
  }

  public async openDiffForFile(filePath: string, repoPath: string | null = null): Promise<void> {
    this._showOverlay();
    this._filePath = filePath;
    const fileInput = document.getElementById('git-blame-file') as HTMLInputElement | null;
    if (fileInput) fileInput.value = filePath || '';

    if (repoPath) {
      this._repoPath = repoPath;
    } else if (!this._repoPath) {
      const pane = this.state[this.state.active];
      const dir = pane.path || '';
      const isRepo = await this.api().gitIsRepo(dir);
      if (isRepo.ok) {
        this._repoPath = isRepo.root;
      }
    }
    if (!this._repoPath) {
      this.setStatus('Not a git repository.');
      return;
    }
    const badge = document.getElementById('git-branch-badge');
    if (badge) badge.textContent = `Diff: ${filePath}`;
    this._switchTab('diff');
    const r1 = document.getElementById('git-diff-ref1') as HTMLInputElement | null;
    const r2 = document.getElementById('git-diff-ref2') as HTMLInputElement | null;
    if (r1) r1.value = 'HEAD';
    if (r2) r2.value = '';
    const r = await this.api().gitDiff(this._repoPath, 'HEAD', null, filePath);
    if (!r.ok) {
      const statusBar = document.getElementById('git-status-bar');
      if (statusBar) statusBar.textContent = r.error;
      return;
    }
    renderDiffContent(document.getElementById('git-diff-content') as HTMLElement, r.diff);
  }

  public async openLogForFile(filePath: string, repoPath: string | null = null): Promise<void> {
    this._showOverlay();
    this._filePath = filePath;
    const fileInput = document.getElementById('git-blame-file') as HTMLInputElement | null;
    if (fileInput) fileInput.value = filePath || '';

    if (repoPath) {
      this._repoPath = repoPath;
    } else if (!this._repoPath) {
      const pane = this.state[this.state.active];
      const dir = pane.path || '';
      const isRepo = await this.api().gitIsRepo(dir);
      if (isRepo.ok) {
        this._repoPath = isRepo.root;
      }
    }
    if (!this._repoPath) {
      this.setStatus('Not a git repository.');
      return;
    }
    const badge = document.getElementById('git-branch-badge');
    if (badge) badge.textContent = `History: ${filePath}`;
    this._switchTab('log');
    await this._loadLog(filePath);
  }

  public async openBlameForFile(filePath: string, repoPath: string | null = null, ref: string | null = null): Promise<void> {
    this._showOverlay();
    this._filePath = filePath;
    if (repoPath) {
      this._repoPath = repoPath;
    } else if (!this._repoPath) {
      const pane = this.state[this.state.active];
      const dir = pane.path || '';
      const isRepo = await this.api().gitIsRepo(dir);
      if (isRepo.ok) {
        this._repoPath = isRepo.root;
      }
    }
    if (!this._repoPath) {
      this.setStatus('Not a git repository.');
      return;
    }
    const badge = document.getElementById('git-branch-badge');
    if (badge) badge.textContent = `Blame: ${filePath}`;
    const fileInput = document.getElementById('git-blame-file') as HTMLInputElement | null;
    const refInput = document.getElementById('git-blame-ref') as HTMLInputElement | null;
    if (fileInput) fileInput.value = filePath || '';
    if (refInput) refInput.value = ref || '';
    this._switchTab('blame');
    await this._loadBlame(filePath, ref);
  }

  private _switchTab(name: string): void {
    this._currentTab = name;
    const statusBar = document.getElementById('git-status-bar');
    if (statusBar) statusBar.textContent = '';
    ['status', 'log', 'diff', 'blame', 'branches'].forEach((t) => {
      document.getElementById(`git-tab-${t}`)?.classList.toggle('hidden', t !== name);
    });
    document.querySelectorAll('[data-gtab]').forEach((b) => {
      b.classList.toggle('search-tab--active', (b as HTMLElement).dataset.gtab === name);
    });

    const fileInput = document.getElementById('git-blame-file') as HTMLInputElement | null;
    if (fileInput && this._filePath && !fileInput.value) {
      fileInput.value = this._filePath;
    }
  }

  private async _refreshStatus(): Promise<void> {
    if (!this._repoPath) return;
    const badge = document.getElementById('git-branch-badge');
    try {
      const r = await this.api().gitStatus(this._repoPath);
      if (!r.ok) { if (badge) badge.textContent = r.error; return; }

      if (badge) {
        badge.textContent = `⎇ ${r.branch}`;
        if (r.ahead || r.behind) badge.textContent += ` ↑${r.ahead} ↓${r.behind}`;
      }

      const list = document.getElementById('git-status-list');
      if (!list) return;
      list.replaceChildren();
      if (!r.files.length) {
        list.innerHTML = '<div class="git-no-repo" style="color:var(--primary-fixed-dim)">✓ Working tree clean</div>';
        return;
      }
      r.files.forEach((f: any) => {
        const row = document.createElement('div');
        row.className = 'git-status-row';
        const staged = f.index !== ' ' && f.index !== '?';
        const modified = f.worktree !== ' ';
        row.innerHTML = `
          <label class="git-stage-check" title="Click to ${staged ? 'unstage' : 'stage'}">
            <input type="checkbox" class="git-file-check" ${staged ? 'checked' : ''} />
            <span class="git-xy git-xy--${staged ? 'staged' : 'unstaged'}">${escHtml(f.xy)}</span>
            <span class="git-filename">${escHtml(f.file)}</span>
            ${modified ? '<span class="git-modified-dot" title="Modified in worktree">●</span>' : ''}
          </label>`;
        const checkEl = row.querySelector('.git-file-check') as HTMLInputElement | null;
        if (checkEl) checkEl.dataset.path = f.file;
        row.querySelector('.git-file-check')?.addEventListener('change', async (e: Event) => {
          await this.api().gitStageFile(this._repoPath, f.file, (e.target as HTMLInputElement).checked);
          await this._refreshStatus();
        });
        row.addEventListener('dblclick', async () => {
          const r2 = await this.api().gitDiff(this._repoPath, 'HEAD', null, f.file);
          if (!r2.ok) return;
          this._switchTab('diff');
          renderDiffContent(document.getElementById('git-diff-content') as HTMLElement, r2.diff);
        });
        list.appendChild(row);
      });
    } catch (err: any) {
      if (badge) badge.textContent = err?.message || 'Error';
    }
  }

  private async _loadLog(filePath: string | null = null): Promise<void> {
    if (!this._repoPath) return;
    const list = document.getElementById('git-log-list');
    if (!list) return;
    list.innerHTML = '<div class="git-no-repo">Loading log…</div>';
    try {
      const r = await this.api().gitLog(this._repoPath, 100, filePath);
      if (!r.ok) { list.innerHTML = `<div class="git-no-repo">${escHtml(r.error)}</div>`; return; }
      list.replaceChildren();
      if (r.commits.length === 0) {
        list.innerHTML = '<div class="git-no-repo">No commits found.</div>';
        return;
      }
      r.commits.forEach((c: any) => {
        const row = document.createElement('div');
        row.className = 'git-log-row';
        const date = new Date(c.date).toLocaleDateString();
        row.innerHTML = `
          <span class="git-log-hash">${escHtml(c.short)}</span>
          <span class="git-log-subject">${escHtml(c.subject)}</span>
          <span class="git-log-meta">${escHtml(c.author)} · ${date}</span>`;
        row.title = c.hash;
        row.style.cursor = 'pointer';
        row.addEventListener('click', async () => {
          const r2 = await this.api().gitDiff(this._repoPath, `${c.hash}~1`, c.hash, filePath);
          if (!r2.ok) return;
          this._switchTab('diff');
          const d1 = document.getElementById('git-diff-ref1') as HTMLInputElement | null;
          const d2 = document.getElementById('git-diff-ref2') as HTMLInputElement | null;
          if (d1) d1.value = `${c.hash}~1`;
          if (d2) d2.value = c.hash;
          renderDiffContent(document.getElementById('git-diff-content') as HTMLElement, r2.diff);
        });
        list.appendChild(row);
      });
    } catch (err: any) {
      list.innerHTML = `<div class="git-no-repo" style="color:var(--error, #ff7b72)">${escHtml(err?.message || String(err))}</div>`;
    }
  }

  private async _loadBranches(): Promise<void> {
    if (!this._repoPath) return;
    const list = document.getElementById('git-branches-list');
    if (!list) return;
    list.innerHTML = '<div class="git-no-repo">Loading…</div>';
    try {
      const r = await this.api().gitBranches(this._repoPath);
      if (!r.ok) { list.innerHTML = `<div class="git-no-repo">${escHtml(r.error)}</div>`; return; }
      list.replaceChildren();
      r.branches.forEach((b: any) => {
        const row = document.createElement('div');
        row.className = `git-branch-row${b.isCurrent ? ' git-branch-row--current' : ''}`;
        row.innerHTML = `
          <span class="git-branch-icon">${b.isCurrent ? '▶' : '◦'}</span>
          <span class="git-branch-name">${escHtml(b.name)}</span>
          ${b.upstream ? `<span class="git-branch-upstream">${escHtml(b.upstream)}</span>` : ''}`;
        if (!b.isCurrent) {
          row.title = `Checkout ${b.name}`;
          row.style.cursor = 'pointer';
          row.addEventListener('click', async () => {
            const ok = confirm(`Checkout branch "${b.name}"?`);
            if (!ok) return;
            const r2 = await this.api().gitCheckout(this._repoPath, b.name, false);
            if (!r2.ok) { this.setStatus(r2.error); return; }
            await this._loadBranches();
            await this._refreshStatus();
            this._switchTab('status');
          });
        }
        list.appendChild(row);
      });
    } catch (err: any) {
      list.innerHTML = `<div class="git-no-repo" style="color:var(--error, #ff7b72)">${escHtml(err?.message || String(err))}</div>`;
    }
  }

  public async _loadBlame(filePath: string | null = null, ref: string | null = null): Promise<void> {
    if (!this._repoPath) return;
    const fileInput = document.getElementById('git-blame-file') as HTMLInputElement | null;
    const refInput = document.getElementById('git-blame-ref') as HTMLInputElement | null;
    let file = (filePath || fileInput?.value || this._filePath || '').trim();
    if (file && fileInput && !fileInput.value) {
      fileInput.value = file;
    }
    const gitRef = (ref || refInput?.value || '').trim() || null;
    const contentEl = document.getElementById('git-blame-content');
    if (!contentEl) return;

    if (!file) {
      contentEl.innerHTML = '<div class="git-no-repo">Please specify a file path to blame.</div>';
      return;
    }

    this._filePath = file;
    const badge = document.getElementById('git-branch-badge');
    if (badge) badge.textContent = `Blame: ${file}`;

    contentEl.innerHTML = `<div class="git-no-repo">Loading blame for ${escHtml(file)}…</div>`;
    try {
      const r = await this.api().gitBlame(this._repoPath, file, gitRef);
      if (!r || !r.ok) {
        contentEl.innerHTML = `<div class="git-no-repo" style="color:var(--error, #ff7b72)">${escHtml(r?.error || 'Failed to get git blame')}</div>`;
        return;
      }

      if (!r.lines || r.lines.length === 0) {
        contentEl.innerHTML = '<div class="git-no-repo">No blame lines found.</div>';
        return;
      }

      contentEl.replaceChildren();
      const frag = document.createDocumentFragment();

      r.lines.forEach((line: any) => {
        const row = document.createElement('div');
        row.className = 'git-blame-row';

        const dateStr = line.authorTime
          ? new Date(line.authorTime * 1000).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: '2-digit' })
          : '—';

        const gutter = document.createElement('div');
        gutter.className = 'git-blame-gutter';

        const numEl = document.createElement('span');
        numEl.className = 'git-blame-num';
        numEl.textContent = String(line.lineNum);

        const hashEl = document.createElement('span');
        hashEl.className = 'git-blame-hash';
        hashEl.textContent = line.commitShort || (line.commitHash ? line.commitHash.slice(0, 8) : '—');
        hashEl.title = `${line.summary}\nAuthor: ${line.author} <${line.authorMail}>\nDate: ${new Date(line.authorTime * 1000).toLocaleString()}\n\nClick to view commit diff`;

        hashEl.addEventListener('click', async (e) => {
          e.stopPropagation();
          const rDiff = await this.api().gitDiff(this._repoPath, `${line.commitHash}~1`, line.commitHash);
          if (!rDiff.ok) {
            const st = document.getElementById('git-status-bar');
            if (st) st.textContent = rDiff.error;
            return;
          }
          this._switchTab('diff');
          const r1 = document.getElementById('git-diff-ref1') as HTMLInputElement | null;
          const r2 = document.getElementById('git-diff-ref2') as HTMLInputElement | null;
          if (r1) r1.value = `${line.commitHash}~1`;
          if (r2) r2.value = line.commitHash;
          renderDiffContent(document.getElementById('git-diff-content') as HTMLElement, rDiff.diff);
        });

        const authorEl = document.createElement('span');
        authorEl.className = 'git-blame-author';
        authorEl.textContent = line.author;
        authorEl.title = `${line.author} <${line.authorMail}>`;

        const dateEl = document.createElement('span');
        dateEl.className = 'git-blame-date';
        dateEl.textContent = dateStr;

        gutter.append(numEl, hashEl, authorEl, dateEl);

        const codeEl = document.createElement('span');
        codeEl.className = 'git-blame-code';
        codeEl.textContent = line.content;

        row.append(gutter, codeEl);
        frag.appendChild(row);
      });

      contentEl.appendChild(frag);
    } catch (err: any) {
      contentEl.innerHTML = `<div class="git-no-repo" style="color:var(--error, #ff7b72)">${escHtml(err?.message || String(err))}</div>`;
    }
  }

  public setup(): void {
    const overlay = document.getElementById('git-overlay');
    overlay?.addEventListener('click', (e) => {
      if (e.target === overlay) this.close();
    });

    document.getElementById('git-close')?.addEventListener('click', (e) => {
      e.stopPropagation();
      this.close();
    });

    document.querySelectorAll('[data-gtab]').forEach((btn) => {
      btn.addEventListener('click', async () => {
        const tab = (btn as HTMLElement).dataset.gtab;
        if (!tab) return;
        this._switchTab(tab);
        if (tab === 'log') {
          await this._loadLog(this._filePath || null);
        } else if (tab === 'branches') {
          await this._loadBranches();
        } else if (tab === 'status') {
          await this._refreshStatus();
        } else if (tab === 'blame') {
          const fileInput = document.getElementById('git-blame-file') as HTMLInputElement | null;
          if (fileInput && !fileInput.value && this._filePath) {
            fileInput.value = this._filePath;
          }
          const targetFile = fileInput?.value?.trim() || this._filePath || null;
          await this._loadBlame(targetFile);
        } else if (tab === 'diff') {
          if (this._filePath) {
            const badge = document.getElementById('git-branch-badge');
            if (badge) badge.textContent = `Diff: ${this._filePath}`;
            const r1 = (document.getElementById('git-diff-ref1') as HTMLInputElement | null)?.value || 'HEAD';
            const r2 = (document.getElementById('git-diff-ref2') as HTMLInputElement | null)?.value || null;
            const r = await this.api().gitDiff(this._repoPath, r1, r2, this._filePath);
            if (r?.ok) {
              renderDiffContent(document.getElementById('git-diff-content') as HTMLElement, r.diff);
            }
          }
        }
      });
    });

    document.getElementById('git-pull-btn')?.addEventListener('click', async () => {
      if (!this._repoPath) return;
      const st = document.getElementById('git-status-bar');
      if (st) st.textContent = 'Pulling…';
      const r = await this.api().gitPull(this._repoPath);
      if (st) st.textContent = r.ok ? `✓ ${r.output || 'Done'}` : `✗ ${r.error}`;
      if (r.ok) { await this._refreshStatus(); await this.loadDir(this.state.active); }
    });

    document.getElementById('git-push-btn')?.addEventListener('click', async () => {
      if (!this._repoPath) return;
      const st = document.getElementById('git-status-bar');
      if (st) st.textContent = 'Pushing…';
      const r = await this.api().gitPush(this._repoPath);
      if (st) st.textContent = r.ok ? `✓ ${r.output || 'Done'}` : `✗ ${r.error}`;
    });

    document.getElementById('git-stash-btn')?.addEventListener('click', async () => {
      if (!this._repoPath) return;
      const msg = prompt('Enter stash message (optional):') || '';
      const st = document.getElementById('git-status-bar');
      if (st) st.textContent = 'Stashing…';
      const r = await this.api().gitStash(this._repoPath, msg);
      if (st) st.textContent = r.ok ? `✓ ${r.output || 'Stashed'}` : `✗ ${r.error}`;
      if (r.ok) { await this._refreshStatus(); await this.loadDir(this.state.active); }
    });

    document.getElementById('git-pop-btn')?.addEventListener('click', async () => {
      if (!this._repoPath) return;
      const st = document.getElementById('git-status-bar');
      if (st) st.textContent = 'Popping stash…';
      const r = await this.api().gitStashPop(this._repoPath);
      if (st) st.textContent = r.ok ? `✓ ${r.output || 'Stash popped'}` : `✗ ${r.error}`;
      if (r.ok) { await this._refreshStatus(); await this.loadDir(this.state.active); }
    });

    document.getElementById('git-commit-btn')?.addEventListener('click', async () => {
      if (!this._repoPath) return;
      const msg = (document.getElementById('git-commit-msg') as HTMLInputElement | null)?.value?.trim();
      if (!msg) { this.setStatus('Commit message is required.'); return; }
      const st = document.getElementById('git-status-bar');
      if (st) st.textContent = 'Committing…';
      const r = await this.api().gitCommit(this._repoPath, msg);
      if (st) st.textContent = r.ok ? `✓ Committed` : `✗ ${r.error}`;
      if (r.ok) {
        const cMsg = document.getElementById('git-commit-msg') as HTMLInputElement | null;
        if (cMsg) cMsg.value = '';
        await this._refreshStatus();
      }
    });

    document.getElementById('git-diff-run-btn')?.addEventListener('click', async () => {
      if (!this._repoPath) return;
      const ref1 = (document.getElementById('git-diff-ref1') as HTMLInputElement | null)?.value?.trim() || 'HEAD';
      const ref2 = (document.getElementById('git-diff-ref2') as HTMLInputElement | null)?.value?.trim() || null;
      const r = await this.api().gitDiff(this._repoPath, ref1, ref2, this._filePath || null);
      if (!r.ok) {
        const st = document.getElementById('git-status-bar');
        if (st) st.textContent = r.error;
        return;
      }
      renderDiffContent(document.getElementById('git-diff-content') as HTMLElement, r.diff);
    });

    document.getElementById('git-blame-run-btn')?.addEventListener('click', async () => {
      const file = (document.getElementById('git-blame-file') as HTMLInputElement | null)?.value?.trim();
      await this._loadBlame(file || this._filePath || null);
    });

    const triggerBlameOnEnter = (e: KeyboardEvent) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        const file = (document.getElementById('git-blame-file') as HTMLInputElement | null)?.value?.trim();
        void this._loadBlame(file || this._filePath || null);
      }
    };
    document.getElementById('git-blame-file')?.addEventListener('keydown', triggerBlameOnEnter);
    document.getElementById('git-blame-ref')?.addEventListener('keydown', triggerBlameOnEnter);
  }
}
