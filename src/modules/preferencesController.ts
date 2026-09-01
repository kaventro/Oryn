// src/modules/preferencesController.ts
import { SettingsStorageService, AVAILABLE_THEMES } from './settings/settingsStorage.ts';
import { HotkeyRegistry, ACTION_DEFINITIONS } from './settings/hotkeyRegistry.ts';
import { AppSettings } from './settings/settingsModel.ts';

// Global singleton instance for storage
const storageService = new SettingsStorageService();
const hotkeyRegistry = new HotkeyRegistry();

export function applyTrayThemeFromStorage(): void {
  const settings = storageService.load();
  storageService.applyTheme(settings.trayTheme);
}

export function readTrayTheme(): string {
  return storageService.load().trayTheme;
}

export function readPaneMode(): string {
  return storageService.load().paneMode;
}

export function savePaneMode(mode: 'single' | 'dual'): void {
  const s = storageService.load();
  s.paneMode = mode;
  storageService.save(s);
}

/**
 * Per-side default drives for Dual Pane startup.
 */
export function readDrivePaneDefaults(): { enabled: boolean; left: string; right: string } {
  const s = storageService.load();
  return {
    enabled: s.dualPaneDriveDefaults !== false,
    left: s.leftDefaultDrive,
    right: s.rightDefaultDrive,
  };
}

export interface PreferencesControllerDeps {
  focusActiveList?: () => void;
  onPaneModeChange?: (mode: string) => void;
  onSettingsChange?: (settings: AppSettings) => void;
  getDrives?: () => Promise<any>;
  storageService?: SettingsStorageService;
  hotkeyRegistry?: HotkeyRegistry;
}

/**
 * PreferencesController coordinates the Settings UI and domain services.
 * Follows Single Responsibility and Open-Closed principles.
 */
export class PreferencesController {
  public focusActiveList: () => void;
  public onPaneModeChange: (mode: string) => void;
  public onSettingsChange: (settings: AppSettings) => void;
  public getDrives: () => Promise<any>;
  public storage: SettingsStorageService;
  public hotkeyRegistry: HotkeyRegistry;

  public settings: AppSettings;
  private _currentTab: string;
  private _recordingActionId: string | null;
  private _onDocKey: ((e: KeyboardEvent) => void) | null;
  private _didSetup?: boolean;
  private _dom: Record<string, any>;

  constructor(deps: PreferencesControllerDeps = {}) {
    this.focusActiveList = deps.focusActiveList || (() => {});
    this.onPaneModeChange = deps.onPaneModeChange || (() => {});
    this.onSettingsChange = deps.onSettingsChange || (() => {});
    this.getDrives = deps.getDrives || (async () => null);
    this.storage = deps.storageService || storageService;
    this.hotkeyRegistry = deps.hotkeyRegistry || hotkeyRegistry;

    this.settings = this.storage.load();
    this._currentTab = 'appearance';
    this._recordingActionId = null;
    this._onDocKey = null;

    this._dom = {};
  }

  public setup(): void {
    if (this._didSetup) return;

    const overlay = document.getElementById('preferences-overlay');
    if (!overlay) return;
    this._didSetup = true;

    this._dom.overlay = overlay;
    this._dom.modal = document.getElementById('modal-preferences');
    this._dom.closeBtn = document.getElementById('preferences-close');
    this._dom.doneBtn = document.getElementById('pref-done-btn');
    this._dom.navBtns = overlay.querySelectorAll('.mac-pref-nav-item');
    this._dom.panels = overlay.querySelectorAll('.mac-pref-tab-panel');
    this._dom.hotkeysList = document.getElementById('pref-hotkeys-list');
    this._dom.hotkeySearch = document.getElementById('pref-hotkey-search') as HTMLInputElement | null;
    this._dom.hotkeyReset = document.getElementById('pref-hotkeys-reset');
    this._dom.statusHint = document.getElementById('pref-status-hint');

    // Prevent click inside modal from closing
    this._dom.modal?.addEventListener('click', (e: MouseEvent) => e.stopPropagation());

    // Close on overlay backdrop click
    this._dom.overlay.addEventListener('click', (e: MouseEvent) => {
      if (e.target === this._dom.overlay) this.close();
    });

    this._dom.closeBtn?.addEventListener('click', (e: MouseEvent) => {
      e.stopPropagation();
      this.close();
    });

    this._dom.doneBtn?.addEventListener('click', (e: MouseEvent) => {
      e.stopPropagation();
      this.close();
    });

    // Navigation items
    this._dom.navBtns.forEach((btn: HTMLElement) => {
      btn.addEventListener('click', (e: Event) => {
        e.stopPropagation();
        const tab = btn.dataset.ptab;
        if (tab) this.switchTab(tab);
      });
    });

    // Theme Wells
    overlay.querySelectorAll('.mac-theme-well').forEach((well) => {
      well.addEventListener('click', (e: Event) => {
        e.stopPropagation();
        const theme = (well as HTMLElement).dataset.theme;
        if (!theme) return;
        this.settings.trayTheme = theme;
        this._saveAndNotify();
        this._updateThemeWells();
        this._flashStatus(`Theme applied: ${theme}`);
      });
    });

    // Pane Mode Segmented Control
    overlay.querySelectorAll('.mac-segmented-item[data-pane-mode]').forEach((btn) => {
      btn.addEventListener('click', (e: Event) => {
        e.stopPropagation();
        const mode = (btn as HTMLElement).dataset.paneMode as 'single' | 'dual' | undefined;
        if (mode) {
          this.settings.paneMode = mode;
          this._saveAndNotify();
          this._updateSegmented('pane-mode', mode);
          if (typeof this.onPaneModeChange === 'function') {
            this.onPaneModeChange(mode);
          }
          this._flashStatus(`Layout: ${mode === 'dual' ? 'Dual Pane' : 'Single Pane'}`);
        }
      });
    });

    // Native Switches / Checkboxes
    this._bindSwitch('pref-show-hidden', (checked) => {
      this.settings.showHiddenFiles = checked;
    });

    this._bindSwitch('pref-show-ext', (checked) => {
      this.settings.showExtensions = checked;
    });

    this._bindSwitch('pref-show-status-terminal', (checked) => {
      this.settings.showStatusBarTerminal = checked;
    });

    this._bindSwitch('pref-confirm-delete', (checked) => {
      this.settings.confirmDelete = checked;
    });

    this._bindSwitch('pref-enable-sftp', (checked) => {
      this.settings.enableSftp = checked;
    });

    this._bindSwitch('pref-dual-drive-defaults', (checked) => {
      this.settings.dualPaneDriveDefaults = checked;
      this._setDriveSelectsDisabled(!checked);
    });

    document.getElementById('pref-left-drive')?.addEventListener('change', (e: Event) => {
      this.settings.leftDefaultDrive = (e.target as HTMLSelectElement).value;
      this._saveAndNotify();
    });

    document.getElementById('pref-right-drive')?.addEventListener('change', (e: Event) => {
      this.settings.rightDefaultDrive = (e.target as HTMLSelectElement).value;
      this._saveAndNotify();
    });

    // Selects
    document.getElementById('pref-overwrite-policy')?.addEventListener('change', (e: Event) => {
      this.settings.overwritePolicy = (e.target as HTMLSelectElement).value as any;
      this._saveAndNotify();
    });

    document.getElementById('pref-git-diff-ref')?.addEventListener('change', (e: Event) => {
      this.settings.defaultDiffRef = (e.target as HTMLSelectElement).value as any;
      this._saveAndNotify();
    });

    const editorSelect = document.getElementById('pref-default-editor') as HTMLSelectElement | null;
    const customWrap = document.getElementById('pref-custom-editor-wrap');
    editorSelect?.addEventListener('change', (e: Event) => {
      const val = (e.target as HTMLSelectElement).value;
      this.settings.defaultEditor = val as any;
      if (customWrap) customWrap.style.display = val === 'custom' ? 'flex' : 'none';
      this._saveAndNotify();
    });

    document.getElementById('pref-custom-editor-cmd')?.addEventListener('input', (e: Event) => {
      this.settings.customEditorCmd = (e.target as HTMLInputElement).value;
      this._saveAndNotify();
    });

    // Hotkey search & reset
    this._dom.hotkeySearch?.addEventListener('input', (e: Event) => {
      this.renderHotkeysList((e.target as HTMLInputElement).value);
    });

    this._dom.hotkeyReset?.addEventListener('click', (e: MouseEvent) => {
      e.stopPropagation();
      if (confirm('Restore all keyboard shortcuts to system defaults?')) {
        this.hotkeyRegistry.resetDefaults();
        this.renderHotkeysList(this._dom.hotkeySearch?.value || '');
        this._flashStatus('Hotkeys restored to defaults');
      }
    });

    // Window Key Capture during recording
    window.addEventListener('keydown', (e) => {
      if (!this._recordingActionId) return;
      e.preventDefault();
      e.stopPropagation();

      if (e.key === 'Escape') {
        this._recordingActionId = null;
        this.renderHotkeysList(this._dom.hotkeySearch?.value || '');
        return;
      }

      const combo = HotkeyRegistry.eventToCombo(e);
      if (combo) {
        this.hotkeyRegistry.setBinding(this._recordingActionId, combo);
        this._flashStatus(`Saved: ${HotkeyRegistry.formatForDisplay(combo)}`);
        this._recordingActionId = null;
        this.renderHotkeysList(this._dom.hotkeySearch?.value || '');
      }
    }, true);
  }

  private _bindSwitch(id: string, onToggle: (checked: boolean) => void): void {
    const el = document.getElementById(id) as HTMLInputElement | null;
    if (!el) return;
    el.addEventListener('change', (e: Event) => {
      onToggle((e.target as HTMLInputElement).checked);
      this._saveAndNotify();
    });
  }

  private _saveAndNotify(): void {
    this.storage.save(this.settings);
    if (typeof this.onSettingsChange === 'function') {
      this.onSettingsChange(this.settings);
    }
  }

  private _flashStatus(msg: string): void {
    if (this._dom.statusHint) {
      this._dom.statusHint.textContent = `✓ ${msg}`;
      setTimeout(() => {
        if (this._dom.statusHint.textContent === `✓ ${msg}`) {
          this._dom.statusHint.textContent = '';
        }
      }, 2500);
    }
  }

  public switchTab(tabId: string): void {
    this._currentTab = tabId;

    this._dom.panels.forEach((p: HTMLElement) => {
      const match = p.id === `pref-panel-${tabId}`;
      p.style.display = match ? 'block' : 'none';
      p.classList.toggle('active', match);
    });

    this._dom.navBtns.forEach((b: HTMLElement) => {
      b.classList.toggle('active', b.dataset.ptab === tabId);
    });

    if (tabId === 'hotkeys') {
      this.renderHotkeysList(this._dom.hotkeySearch?.value || '');
    }
  }

  private _updateThemeWells(): void {
    document.querySelectorAll('.mac-theme-well').forEach((well) => {
      well.classList.toggle('selected', (well as HTMLElement).dataset.theme === this.settings.trayTheme);
    });
  }

  private _updateSegmented(group: string, activeValue: string): void {
    document.querySelectorAll(`[data-${group}]`).forEach((btn) => {
      btn.classList.toggle('active', btn.getAttribute(`data-${group}`) === activeValue);
    });
  }

  private _sameDrive(a?: string, b?: string): boolean {
    const norm = (p?: string) => String(p || '').replace(/[\\/]+$/, '').toUpperCase();
    return norm(a) === norm(b) && norm(a) !== '';
  }

  private _setDriveSelectsDisabled(disabled: boolean): void {
    const left = document.getElementById('pref-left-drive') as HTMLSelectElement | null;
    const right = document.getElementById('pref-right-drive') as HTMLSelectElement | null;
    if (left) left.disabled = disabled;
    if (right) right.disabled = disabled;
  }

  private async _populateDriveSelects(): Promise<void> {
    const leftSel = document.getElementById('pref-left-drive') as HTMLSelectElement | null;
    const rightSel = document.getElementById('pref-right-drive') as HTMLSelectElement | null;
    if (!leftSel && !rightSel) return;

    let drives: Array<{ value: string; label: string }> = [];
    try {
      const locs = await this.getDrives();
      drives = (locs?.drives || [])
        .map((d: any) => ({ value: d.mountPoint, label: d.name || d.mountPoint }))
        .filter((d: any) => d.value);
    } catch {}

    this._fillDriveSelect(leftSel, drives, this.settings.leftDefaultDrive);
    this._fillDriveSelect(rightSel, drives, this.settings.rightDefaultDrive);
  }

  private _fillDriveSelect(sel: HTMLSelectElement | null, drives: Array<{ value: string; label: string }>, current?: string): void {
    if (!sel) return;
    const opts = [{ value: '', label: 'Automatic (by platform)' }, ...drives];
    if (current && !opts.some((o) => this._sameDrive(o.value, current))) {
      opts.push({ value: current, label: `${current} (not connected)` });
    }

    sel.replaceChildren();
    opts.forEach((o) => {
      const el = document.createElement('option');
      el.value = o.value;
      el.textContent = o.label;
      sel.appendChild(el);
    });

    const match = opts.find((o) => (current ? this._sameDrive(o.value, current) : o.value === ''));
    sel.value = match ? match.value : '';
  }

  public renderHotkeysList(query = ''): void {
    if (!this._dom.hotkeysList) return;
    this._dom.hotkeysList.replaceChildren();

    const q = (query || '').toLowerCase().trim();
    const categories: Record<string, typeof ACTION_DEFINITIONS> = {};

    ACTION_DEFINITIONS.forEach((act) => {
      const cat = act.category || 'General';
      if (!categories[cat]) categories[cat] = [];
      categories[cat].push(act);
    });

    Object.entries(categories).forEach(([category, actions]) => {
      const matching = actions.filter((act) => {
        if (!q) return true;
        const binding = this.hotkeyRegistry.getBinding(act.id);
        const display = HotkeyRegistry.formatForDisplay(binding);
        return act.name.toLowerCase().includes(q) ||
               binding.toLowerCase().includes(q) ||
               display.toLowerCase().includes(q);
      });

      if (matching.length === 0) return;

      const groupEl = document.createElement('div');
      groupEl.className = 'mac-pref-card mac-pref-card--grouped';

      const header = document.createElement('div');
      header.className = 'mac-pref-section-title';
      header.textContent = category;
      groupEl.appendChild(header);

      matching.forEach((act) => {
        const row = document.createElement('div');
        row.className = 'mac-pref-row';

        const left = document.createElement('span');
        left.className = 'mac-pref-row-title';
        left.textContent = act.name;

        const right = document.createElement('div');
        right.className = 'mac-hotkey-controls';

        const isRecording = this._recordingActionId === act.id;
        const currentBinding = this.hotkeyRegistry.getBinding(act.id);
        const displayKey = HotkeyRegistry.formatForDisplay(currentBinding);

        const badge = document.createElement('button');
        badge.type = 'button';
        badge.className = `mac-hotkey-badge${isRecording ? ' recording' : ''}`;
        badge.textContent = isRecording ? 'Type Shortcut…' : displayKey;
        badge.title = isRecording ? 'Press combination or Esc to cancel' : 'Click to rebind';

        badge.addEventListener('click', (e) => {
          e.stopPropagation();
          this._recordingActionId = isRecording ? null : act.id;
          this.renderHotkeysList(query);
        });

        const clearBtn = document.createElement('button');
        clearBtn.type = 'button';
        clearBtn.className = 'mac-hotkey-clear-btn';
        clearBtn.innerHTML = '✕';
        clearBtn.title = 'Clear shortcut';
        clearBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          this.hotkeyRegistry.setBinding(act.id, '');
          this.renderHotkeysList(query);
        });

        right.append(badge, clearBtn);
        row.append(left, right);
        groupEl.appendChild(row);
      });

      this._dom.hotkeysList.appendChild(groupEl);
    });
  }

  public open(): void {
    const overlay = this._dom.overlay || document.getElementById('preferences-overlay');
    if (!overlay) return;

    this.settings = this.storage.load();

    // Populate UI
    this._updateThemeWells();
    this._updateSegmented('pane-mode', this.settings.paneMode);

    const sh = document.getElementById('pref-show-hidden') as HTMLInputElement | null;
    if (sh) sh.checked = Boolean(this.settings.showHiddenFiles);

    const se = document.getElementById('pref-show-ext') as HTMLInputElement | null;
    if (se) se.checked = this.settings.showExtensions !== false;

    const st = document.getElementById('pref-show-status-terminal') as HTMLInputElement | null;
    if (st) st.checked = this.settings.showStatusBarTerminal !== false;

    const cd = document.getElementById('pref-confirm-delete') as HTMLInputElement | null;
    if (cd) cd.checked = this.settings.confirmDelete !== false;

    const sftp = document.getElementById('pref-enable-sftp') as HTMLInputElement | null;
    if (sftp) sftp.checked = Boolean(this.settings.enableSftp);

    const dd = document.getElementById('pref-dual-drive-defaults') as HTMLInputElement | null;
    if (dd) dd.checked = this.settings.dualPaneDriveDefaults !== false;
    this._setDriveSelectsDisabled(this.settings.dualPaneDriveDefaults === false);
    this._populateDriveSelects();

    const op = document.getElementById('pref-overwrite-policy') as HTMLSelectElement | null;
    if (op) op.value = this.settings.overwritePolicy;

    const gd = document.getElementById('pref-git-diff-ref') as HTMLSelectElement | null;
    if (gd) gd.value = this.settings.defaultDiffRef;

    const de = document.getElementById('pref-default-editor') as HTMLSelectElement | null;
    if (de) de.value = this.settings.defaultEditor;

    const ce = document.getElementById('pref-custom-editor-cmd') as HTMLInputElement | null;
    if (ce) ce.value = this.settings.customEditorCmd;

    const customWrap = document.getElementById('pref-custom-editor-wrap');
    if (customWrap) customWrap.style.display = this.settings.defaultEditor === 'custom' ? 'flex' : 'none';

    this.switchTab(this._currentTab);
    overlay.classList.remove('hidden');
    overlay.setAttribute('aria-hidden', 'false');

    this._onDocKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !this._recordingActionId) {
        e.preventDefault();
        this.close();
      }
    };
    document.addEventListener('keydown', this._onDocKey);
  }

  public close(): void {
    const overlay = this._dom.overlay || document.getElementById('preferences-overlay');
    if (!overlay) return;

    overlay.classList.add('hidden');
    overlay.setAttribute('aria-hidden', 'true');
    this._recordingActionId = null;

    if (this._onDocKey) {
      document.removeEventListener('keydown', this._onDocKey);
      this._onDocKey = null;
    }
    this.focusActiveList();
  }
}
