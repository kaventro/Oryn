// src/modules/settings/settingsStorage.ts
import { AppSettings, type AppSettingsData } from './settingsModel.ts';

const SETTINGS_KEY = 'Oryn.settings';
const THEME_KEY = 'Oryn.trayTheme';
const PANE_MODE_KEY = 'Oryn.paneMode';
const DOCK_ICON_KEY = 'Oryn.dockIcon';

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

export const AVAILABLE_DOCK_ICONS = [
  { id: '1', name: 'Classic Dark Gradient', src: '/dock-icons/thumb_1.png' },
  { id: '2', name: 'Midnight Glow', src: '/dock-icons/thumb_2.png' },
  { id: '3', name: 'Light Lavender', src: '/dock-icons/thumb_3.png' },
  { id: '4', name: 'Frost Violet', src: '/dock-icons/thumb_4.png' },
  { id: '5', name: 'Monochrome Dark', src: '/dock-icons/thumb_5.png' },
  { id: '6', name: 'Electric Cyan', src: '/dock-icons/thumb_6.png' },
  { id: '7', name: 'Emerald Forest', src: '/dock-icons/thumb_7.png' },
  { id: '8', name: 'Amber Sunset', src: '/dock-icons/thumb_8.png' },
  { id: '9', name: 'Crimson Ruby', src: '/dock-icons/thumb_9.png' },
  { id: '10', name: 'Deep Teal Glow', src: '/dock-icons/thumb_10.png' },
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
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) data = parsed;
    } catch { }

    // Legacy sync
    try {
      const theme = localStorage.getItem(THEME_KEY);
      if (theme) data.trayTheme = theme;
      const paneMode = localStorage.getItem(PANE_MODE_KEY);
      if (paneMode) data.paneMode = paneMode;
      const dockIcon = localStorage.getItem(DOCK_ICON_KEY);
      if (dockIcon) data.dockIcon = dockIcon;
    } catch { }

    return new AppSettings(data);
  }

  public save(settings: AppSettings | AppSettingsData): void {
    const s = settings instanceof AppSettings ? settings : new AppSettings(settings);
    try {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(s.toJSON()));
      localStorage.setItem(THEME_KEY, s.trayTheme);
      localStorage.setItem(PANE_MODE_KEY, s.paneMode);
      localStorage.setItem(DOCK_ICON_KEY, s.dockIcon);
    } catch { }

    this.applyTheme(s.trayTheme);
    void this.applyDockIcon(s.dockIcon);
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

  public async applyDockIcon(iconId: string): Promise<void> {
    const validId = String(iconId || '1').replace(/\.png$/, '');
    const api = (window as any).ow;
    if (typeof api?.setDockIcon === 'function') {
      await api.setDockIcon(validId);
    }
    try {
      const link = document.querySelector("link[rel~='icon']") as HTMLLinkElement | null;
      if (link) {
        link.href = `/dock-icons/${validId}.png`;
      }
    } catch {}
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
