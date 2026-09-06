// src/modules/updaterController.ts — background app updates via tauri-plugin-updater.
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { SettingsStorageService } from './settings/settingsStorage.ts';

/** Download progress as a whole percent; 0 when the server sends no content-length. */
export function downloadPercent(received: number, total: number): number {
  if (!Number.isFinite(total) || total <= 0) return 0;
  const pct = Math.round((received / total) * 100);
  return Math.max(0, Math.min(100, pct));
}

export interface UpdaterDeps {
  check?: () => Promise<Update | null>;
  relaunch?: () => Promise<void>;
  storageService?: SettingsStorageService;
}

const SKIP_KEY = 'oryn.update.skipVersion';

export type UpdaterState = 'idle' | 'prompt' | 'downloading' | 'ready';

export class UpdaterController {
  private readonly check: () => Promise<Update | null>;
  private readonly relaunch: () => Promise<void>;
  private readonly storage: SettingsStorageService;
  private busy = false;
  private state: UpdaterState = 'idle';
  private currentUpdate: Update | null = null;

  constructor(deps: UpdaterDeps = {}) {
    this.check = deps.check ?? (() => check());
    this.relaunch = deps.relaunch ?? relaunch;
    this.storage = deps.storageService ?? new SettingsStorageService();
  }

  public getState(): UpdaterState {
    return this.state;
  }

  public getCurrentUpdate(): Update | null {
    return this.currentUpdate;
  }

  private el<T extends HTMLElement>(id: string): T | null {
    return document.getElementById(id) as T | null;
  }

  private show(
    title: string,
    msg: string,
    opts: { bar?: boolean; action?: string; later?: string } = {}
  ): void {
    const root = this.el('nx-update');
    if (!root) return;
    root.classList.remove('nx-xfer--hidden');
    const t = this.el('nx-update-title');
    if (t) t.textContent = title;
    const m = this.el('nx-update-msg');
    if (m) m.textContent = msg;
    const track = this.el('nx-update-track');
    if (track) track.style.display = opts.bar ? '' : 'none';
    const action = this.el<HTMLButtonElement>('nx-update-action');
    if (action) {
      action.style.display = opts.action ? '' : 'none';
      action.textContent = opts.action ?? '';
      action.disabled = false;
    }
    const later = this.el<HTMLButtonElement>('nx-update-later');
    if (later) {
      later.style.display = opts.later ? '' : 'none';
      later.textContent = opts.later ?? '';
    }
  }

  private hide(): void {
    this.el('nx-update')?.classList.add('nx-xfer--hidden');
  }

  private setProgress(received: number, total: number): void {
    const pct = downloadPercent(received, total);
    const fill = this.el('nx-update-fill');
    if (fill) fill.style.width = `${pct}%`;
    const m = this.el('nx-update-msg');
    if (m) m.textContent = total > 0 ? `Downloading… ${pct}%` : 'Downloading…';
  }

  /** Wires the toast and settings buttons. Call once at startup. */
  attach(): void {
    this.el('nx-update-dismiss')?.addEventListener('click', () => {
      this.dismiss();
    });
    this.el('nx-update-later')?.addEventListener('click', () => {
      this.dismiss();
    });
    this.el('nx-update-action')?.addEventListener('click', () => {
      if (this.state === 'prompt' && this.currentUpdate) {
        void this.startInstall(this.currentUpdate);
      } else if (this.state === 'ready') {
        void this.relaunch();
      }
    });
    this.el('pref-check-updates')?.addEventListener('click', () => {
      void this.run(true);
    });
    this.el('pref-check-updates-btn')?.addEventListener('click', () => {
      void this.run(true);
    });
  }

  /** Dismisses the update prompt for this session without marking it skipped. */
  public dismiss(): void {
    this.hide();
    this.state = 'idle';
    this.currentUpdate = null;
  }

  /**
   * Prompts the user when a new update is found, letting them choose between
   * updating immediately or postponing to next time.
   */
  private promptUpdate(update: Update): void {
    this.currentUpdate = update;
    this.state = 'prompt';
    this.show(
      'Update Available',
      `Oryn ${update.version} is available.`,
      { action: 'Update Now', later: 'Next Time' }
    );
  }

  /**
   * Starts downloading and installing the update in place.
   */
  public async startInstall(update: Update): Promise<void> {
    this.state = 'downloading';
    this.show(`Updating to ${update.version}`, 'Downloading…', { bar: true });
    let received = 0;
    let total = 0;
    try {
      await update.downloadAndInstall((e) => {
        if (e.event === 'Started') {
          total = e.data.contentLength ?? 0;
        } else if (e.event === 'Progress') {
          received += e.data.chunkLength;
          this.setProgress(received, total);
        }
      });
      // The installer replaces the current build in place — no old version is left behind.
      localStorage.setItem(SKIP_KEY, update.version);
      this.state = 'ready';
      this.show(`Oryn ${update.version} installed`, 'Restart to finish.', { action: 'Restart' });
    } catch (err) {
      console.error('[updater]', err);
      this.state = 'idle';
      this.show('Update failed', String((err as Error)?.message ?? err));
    }
  }

  /**
   * Checks for an update. On startup (`manual = false`), respects the `enableAutoUpdate`
   * setting and prompts the user to install now or next time.
   * On manual check (`manual = true`), checks regardless of the setting and surfaces
   * "up to date" / error states.
   */
  async run(manual = false): Promise<void> {
    if (this.busy) return;
    if (!manual && !this.storage.load().enableAutoUpdate) {
      return;
    }
    this.busy = true;
    try {
      const update = await this.check();
      if (!update) {
        if (manual) this.show('Oryn is up to date', 'No newer version available.');
        return;
      }
      if (!manual && localStorage.getItem(SKIP_KEY) === update.version) return;

      this.promptUpdate(update);
    } catch (err) {
      console.error('[updater]', err);
      if (manual) this.show('Update check failed', String((err as Error)?.message ?? err));
    } finally {
      this.busy = false;
    }
  }
}
