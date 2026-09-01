// src/modules/findReplaceController.ts

export interface FindReplaceDeps {
  api: () => any;
  setStatus: (msg: string) => void;
  getSearchRoot: () => string;
}

export class FindReplaceController {
  public api: () => any;
  public setStatus: (msg: string) => void;
  public getSearchRoot: () => string;

  constructor(deps: FindReplaceDeps) {
    this.api = deps.api;
    this.setStatus = deps.setStatus;
    this.getSearchRoot = deps.getSearchRoot;
  }

  public async run(): Promise<void> {
    const findText = (document.getElementById('search-content') as HTMLInputElement)?.value?.trim();
    if (!findText) { this.setStatus('Enter text to find in "Contains text" field.'); return; }
    const replaceText = (document.getElementById('search-replace-text') as HTMLInputElement)?.value ?? '';
    const dryRun = (document.getElementById('replace-dry-run') as HTMLInputElement)?.checked !== false;
    const useRegex = !!(document.getElementById('replace-regex') as HTMLInputElement)?.checked;
    const caseSensitive = !!(document.getElementById('search-content-case') as HTMLInputElement)?.checked;
    const filePattern = (document.getElementById('search-filename') as HTMLInputElement)?.value?.trim() || '';
    const excludePattern = (document.getElementById('search-exclude') as HTMLInputElement)?.value?.trim() || '';
    const root = this.getSearchRoot();
    const statusEl = document.getElementById('replace-status');
    if (!statusEl) return;

    statusEl.textContent = `${dryRun ? '🔍 Dry run' : '✏️ Replacing'}…`;

    const r = await this.api().findReplace(
      { rootDirs: [root], findText, replaceText, filePattern, caseSensitive, useRegex, dryRun, excludePattern },
      (msg: any) => {
        if (msg.type === 'file') {
          statusEl.textContent += `\n  ${msg.path} (${msg.occurrences}×)`;
        }
      },
    );

    if (!r.ok) { statusEl.textContent = `Error: ${r.error}`; return; }
    const prefix = dryRun ? '[DRY RUN] Would modify' : 'Modified';
    statusEl.textContent = `${prefix} ${r.changed.length} file(s), ${r.count} replacement(s).\n` +
      r.changed.map((f: any) => `  ${f.path} (${f.occurrences}×)`).join('\n');
  }

  public patchSearchButton(): void {
    const startBtn = document.getElementById('search-start-btn');
    if (!startBtn) return;
    startBtn.addEventListener('click', () => {
      const replaceText = (document.getElementById('search-replace-text') as HTMLInputElement)?.value ?? '';
      if (replaceText !== '') void this.run();
    }, { capture: true });
  }
}
