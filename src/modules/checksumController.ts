// src/modules/checksumController.ts
import { fmtBytes, fmtSizeExact } from './formatUtils.ts';
import type { AppState } from './stateModels.ts';

export interface ChecksumControllerDeps {
  api: () => any;
  state: AppState;
  setStatus: (msg: string) => void;
  focusActiveList: () => void;
}

export class ChecksumController {
  public api: () => any;
  public state: AppState;
  public setStatus: (msg: string) => void;
  public focusActiveList: () => void;

  public currentSha256: string;
  public currentMd5: string;

  constructor(deps: ChecksumControllerDeps) {
    this.api = deps.api;
    this.state = deps.state;
    this.setStatus = deps.setStatus;
    this.focusActiveList = deps.focusActiveList;

    this.currentSha256 = '';
    this.currentMd5 = '';
  }

  public async openForPath(filePath?: string): Promise<void> {
    if (!filePath) return;

    const overlay = document.getElementById('checksum-overlay');
    if (overlay) overlay.classList.remove('hidden');

    const fnEl = document.getElementById('checksum-filename');
    const pathEl = document.getElementById('checksum-path');
    const szEl = document.getElementById('checksum-size');
    const shaEl = document.getElementById('checksum-sha256');
    const md5El = document.getElementById('checksum-md5');
    const verifyInputEl = document.getElementById('checksum-verify-input') as HTMLInputElement | null;
    const verifyResEl = document.getElementById('checksum-verify-result');

    if (fnEl) fnEl.textContent = filePath.split(/[/\\]/).pop() || filePath;
    if (pathEl) pathEl.textContent = filePath;
    if (szEl) szEl.textContent = 'Computing…';
    if (shaEl) shaEl.textContent = 'Computing SHA-256…';
    if (md5El) md5El.textContent = 'Computing MD5…';
    if (verifyInputEl) verifyInputEl.value = '';
    if (verifyResEl) {
      verifyResEl.textContent = '';
      verifyResEl.className = 'checksum-verify-result';
    }

    try {
      const res = await this.api().fsChecksum(filePath);
      if (!res || !res.ok) {
        if (shaEl) shaEl.textContent = 'Failed to compute checksum.';
        return;
      }

      this.currentSha256 = res.sha256;
      this.currentMd5 = res.md5;

      if (szEl) szEl.textContent = `${fmtBytes(res.size)} (${fmtSizeExact(res.size)})`;
      if (shaEl) shaEl.textContent = res.sha256;
      if (md5El) md5El.textContent = res.md5;
    } catch (err: any) {
      if (shaEl) shaEl.textContent = err?.message || 'Error computing checksum';
    }
  }

  public hide(): void {
    document.getElementById('checksum-overlay')?.classList.add('hidden');
    if (typeof this.focusActiveList === 'function') {
      this.focusActiveList();
    }
  }

  public verifyInput(inputVal?: string): void {
    const val = (inputVal || '').trim().toLowerCase();
    const resultEl = document.getElementById('checksum-verify-result');
    if (!resultEl) return;

    if (!val) {
      resultEl.textContent = '';
      resultEl.className = 'checksum-verify-result';
      return;
    }

    if (val === this.currentSha256.toLowerCase()) {
      resultEl.textContent = '✓ Matches SHA-256';
      resultEl.className = 'checksum-verify-result checksum-verify-result--match';
    } else if (val === this.currentMd5.toLowerCase()) {
      resultEl.textContent = '✓ Matches MD5';
      resultEl.className = 'checksum-verify-result checksum-verify-result--match';
    } else {
      resultEl.textContent = '✗ Does not match';
      resultEl.className = 'checksum-verify-result checksum-verify-result--mismatch';
    }
  }

  public setup(): void {
    const overlay = document.getElementById('checksum-overlay');
    overlay?.addEventListener('click', (e) => {
      if (e.target === overlay) this.hide();
    });

    document.getElementById('checksum-close')?.addEventListener('click', (e) => {
      e.stopPropagation();
      this.hide();
    });

    document.getElementById('checksum-copy-sha256')?.addEventListener('click', async () => {
      if (this.currentSha256) {
        await this.api().clipboardWrite(this.currentSha256);
        this.setStatus('SHA-256 copied to clipboard.');
      }
    });

    document.getElementById('checksum-copy-md5')?.addEventListener('click', async () => {
      if (this.currentMd5) {
        await this.api().clipboardWrite(this.currentMd5);
        this.setStatus('MD5 copied to clipboard.');
      }
    });

    document.getElementById('checksum-verify-input')?.addEventListener('input', (e) => {
      this.verifyInput((e.target as HTMLInputElement).value);
    });
  }
}
