// src/modules/settings/settingsStorage.ts
import { AppSettings, type AppSettingsData } from './settingsModel.ts';

const SETTINGS_KEY = 'Oryn.settings';
const THEME_KEY = 'Oryn.trayTheme';
const PANE_MODE_KEY = 'Oryn.paneMode';

export interface ThemeOption {
  id: string;
  name: string;
  color: string;
}

export const AVAILABLE_THEMES: ThemeOption[] = [
  { id: 'balanced', name: 'Graphite Blue', color: '#0a84ff' },
  { id: 'ocean', name: 'Ocean Cyan', color: '#00b4d8' },
  { id: 'ember', name: 'Ember Orange', color: '#ff9f0a' },
  { id: 'emerald', name: 'Emerald Green', color: '#30d158' },
  { id: 'purple', name: 'Purple Glow', color: '#bf5af2' },
  { id: 'mono', name: 'Monochrome', color: '#d1d1d6' },
];

/**
 * Storage & Persistence service for AppSettings.
 * Implements Dependency Inversion & Single Responsibility for I/O operations.
 */
export class SettingsStorageService {
  private _listeners: Set<(settings: AppSettings) => void>;

  constructor() {
    this._listeners = new Set();
  }

  public load(): AppSettings {
    let data: AppSettingsData = {};
    try {
      const raw = localStorage.getItem(SETTINGS_KEY);
      const parsed = raw ? JSON.parse(raw) : null;
      // Must be a plain object; otherwise the property assignments below would
      // throw in strict mode on a primitive.
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) data = parsed;
    } catch { }

    // Legacy sync
    try {
      const theme = localStorage.getItem(THEME_KEY);
      if (theme) data.trayTheme = theme;
      const paneMode = localStorage.getItem(PANE_MODE_KEY);
      if (paneMode) data.paneMode = paneMode;
    } catch { }

    return new AppSettings(data);
  }

  public save(settings: AppSettings | AppSettingsData): void {
    const s = settings instanceof AppSettings ? settings : new AppSettings(settings);
    try {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(s.toJSON()));
      localStorage.setItem(THEME_KEY, s.trayTheme);
      localStorage.setItem(PANE_MODE_KEY, s.paneMode);
    } catch { }

    this.applyTheme(s.trayTheme);
    this._notify(s);
  }

  public applyTheme(themeId: string): void {
    const valid = AVAILABLE_THEMES.some((t) => t.id === themeId);
    const theme = valid ? themeId : 'balanced';
    const root = document.documentElement;
    if (theme === 'balanced') {
      root.removeAttribute('data-tray');
    } else {
      root.setAttribute('data-tray', theme);
    }
  }

  public subscribe(callback: (settings: AppSettings) => void): () => boolean {
    this._listeners.add(callback);
    return () => this._listeners.delete(callback);
  }

  private _notify(settings: AppSettings): void {
    this._listeners.forEach((fn) => {
      try {
        fn(settings);
      } catch (err) {
        console.error('Settings listener error:', err);
      }
    });
  }
}
