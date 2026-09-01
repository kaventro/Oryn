// src/modules/settings/hotkeyRegistry.ts

export interface HotkeyAction {
  id: string;
  name: string;
  defaultKey: string;
  category: string;
  icon: string;
}

export const ACTION_DEFINITIONS: HotkeyAction[] = [
  // Navigation & Lister
  { id: 'viewFile', name: 'Quick Look / Lister', defaultKey: 'F3', category: 'Navigation & View', icon: 'eye' },
  { id: 'quickView', name: 'Toggle Quick View Pane', defaultKey: 'Ctrl+Q', category: 'Navigation & View', icon: 'layout' },
  { id: 'switchPanel', name: 'Switch Active Panel', defaultKey: 'Tab', category: 'Navigation & View', icon: 'columns' },
  { id: 'swapPanels', name: 'Swap Panels (Left ↔ Right)', defaultKey: 'Ctrl+U', category: 'Navigation & View', icon: 'repeat' },
  { id: 'goRoot', name: 'Go to Drive Root Directory', defaultKey: 'Ctrl+\\', category: 'Navigation & View', icon: 'home' },
  { id: 'branchView', name: 'Branch View (Flat List)', defaultKey: 'Ctrl+B', category: 'Navigation & View', icon: 'list' },
  { id: 'openNewTab', name: 'Open in New Tab', defaultKey: '', category: 'Navigation & View', icon: 'plus-square' },
  { id: 'changeDriveLeft', name: 'Change Drive (Left Pane)', defaultKey: 'Alt+F1', category: 'Navigation & View', icon: 'hard-drive' },
  { id: 'changeDriveRight', name: 'Change Drive (Right Pane)', defaultKey: 'Alt+F2', category: 'Navigation & View', icon: 'hard-drive' },
  { id: 'goBack', name: 'History Back', defaultKey: 'Alt+ArrowLeft', category: 'Navigation & View', icon: 'arrow-left' },
  { id: 'goFwd', name: 'History Forward', defaultKey: 'Alt+ArrowRight', category: 'Navigation & View', icon: 'arrow-right' },
  { id: 'refresh', name: 'Reload Panes', defaultKey: 'Ctrl+R', category: 'Navigation & View', icon: 'refresh' },
  { id: 'editPath', name: 'Edit Address / Path', defaultKey: 'Ctrl+L', category: 'Navigation & View', icon: 'edit' },
  { id: 'calcDirSize', name: 'Calculate Directory Size', defaultKey: 'Space', category: 'Navigation & View', icon: 'database' },

  // File Operations
  { id: 'rename', name: 'Rename Item', defaultKey: 'F2', category: 'File Operations', icon: 'edit-3' },
  { id: 'editVSCode', name: 'Open in Code Editor', defaultKey: 'F4', category: 'File Operations', icon: 'code' },
  { id: 'copy', name: 'Copy to Other Pane', defaultKey: 'F5', category: 'File Operations', icon: 'copy' },
  { id: 'cloneFile', name: 'Clone / Copy in Same Folder', defaultKey: 'Shift+F5', category: 'File Operations', icon: 'copy' },
  { id: 'move', name: 'Move to Other Pane', defaultKey: 'F6', category: 'File Operations', icon: 'move' },
  { id: 'compressArchive', name: 'Compress Selection to ZIP', defaultKey: 'Alt+F5', category: 'File Operations', icon: 'archive' },
  { id: 'extractArchive', name: 'Extract Archive to Other Pane', defaultKey: 'Alt+F9', category: 'File Operations', icon: 'folder' },
  { id: 'newFile', name: 'Create New File', defaultKey: 'Shift+F7', category: 'File Operations', icon: 'file-plus' },
  { id: 'mkdir', name: 'Create New Folder', defaultKey: 'F7', category: 'File Operations', icon: 'folder-plus' },
  { id: 'delete', name: 'Delete Item', defaultKey: 'F8', category: 'File Operations', icon: 'trash-2' },
  { id: 'multiRename', name: 'Batch Rename (Multi-Rename)', defaultKey: 'Ctrl+M', category: 'File Operations', icon: 'file-text' },
  { id: 'copyPath', name: 'Copy Path to Clipboard', defaultKey: '', category: 'File Operations', icon: 'clipboard' },
  { id: 'properties', name: 'Properties', defaultKey: 'Alt+Enter', category: 'File Operations', icon: 'info' },
  { id: 'checksum', name: 'Checksum / Hash', defaultKey: '', category: 'File Operations', icon: 'hash' },

  // Git
  { id: 'openGit', name: 'Git Repository Panel', defaultKey: 'Ctrl+G', category: 'Git', icon: 'git-branch' },
  { id: 'gitDiffFile', name: 'Diff File with HEAD', defaultKey: '', category: 'Git', icon: 'git-diff' },
  { id: 'gitBlameFile', name: 'Blame File', defaultKey: '', category: 'Git', icon: 'users' },
  { id: 'gitFileHistory', name: 'File History', defaultKey: '', category: 'Git', icon: 'clock' },
  { id: 'gitStageFile', name: 'Stage / Unstage File', defaultKey: '', category: 'Git', icon: 'plus-circle' },
  { id: 'gitDiscardChanges', name: 'Discard Local Changes', defaultKey: '', category: 'Git', icon: 'rotate-ccw' },

  { id: 'commandPalette', name: 'Command Palette / Quick Jump', defaultKey: 'Ctrl+P', category: 'Tools & Overlays', icon: 'command' },
  { id: 'diskSpace', name: 'Disk Space Analyzer (Treemap)', defaultKey: 'Ctrl+Shift+D', category: 'Tools & Overlays', icon: 'pie-chart' },
  { id: 'duplicateFinder', name: 'Duplicate File Finder (Cleaner)', defaultKey: '', category: 'Tools & Overlays', icon: 'copy' },
  { id: 'openSearch', name: 'Find / Search Files', defaultKey: 'Alt+F7', category: 'Tools & Overlays', icon: 'search' },
  { id: 'focusFilter', name: 'Quick Filter Bar', defaultKey: 'Ctrl+F', category: 'Tools & Overlays', icon: 'filter' },
  { id: 'openCompare', name: 'Directory Compare', defaultKey: 'Ctrl+D', category: 'Tools & Overlays', icon: 'git-diff' },
  { id: 'toggleTerminal', name: 'Integrated Terminal', defaultKey: 'Ctrl+`', category: 'Tools & Overlays', icon: 'terminal' },
  { id: 'openTerminalHere', name: 'Open Terminal Here (External)', defaultKey: '', category: 'Tools & Overlays', icon: 'terminal' },
  { id: 'openBookmarks', name: 'Bookmarks', defaultKey: '', category: 'Tools & Overlays', icon: 'bookmark' },
  { id: 'openRemote', name: 'Remote Server Connections (SFTP / SSH)', defaultKey: 'Ctrl+Shift+S', category: 'Tools & Overlays', icon: 'server' },
  { id: 'openPreferences', name: 'Settings & Preferences', defaultKey: 'Ctrl+,', category: 'Tools & Overlays', icon: 'settings' },

  // Selection
  { id: 'selectAll', name: 'Select All', defaultKey: 'Ctrl+A', category: 'Selection', icon: 'check-square' },
  { id: 'selectByPattern', name: 'Select by Mask / Pattern', defaultKey: 'Alt+S', category: 'Selection', icon: 'plus-square' },
  { id: 'selectByExtension', name: 'Select by Extension', defaultKey: '', category: 'Selection', icon: 'file-plus' },
  { id: 'deselectByPattern', name: 'Deselect by Mask', defaultKey: 'Alt+D', category: 'Selection', icon: 'minus-square' },
  { id: 'invertSelection', name: 'Invert Selection', defaultKey: 'Alt+I', category: 'Selection', icon: 'shuffle' },
  { id: 'clearSelection', name: 'Deselect All', defaultKey: 'Alt+A', category: 'Selection', icon: 'square' },
];

const STORAGE_KEY = 'Oryn.customHotkeys';

/**
 * HotkeyRegistry manages action bindings, event resolution, and conflict tracking.
 */
export class HotkeyRegistry {
  private _keymap: Map<string, string>;

  constructor() {
    this._keymap = new Map();
    this.load();
  }

  public load(): void {
    this._keymap.clear();
    ACTION_DEFINITIONS.forEach((act) => {
      this._keymap.set(act.id, act.defaultKey);
    });

    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const custom = JSON.parse(raw);
        if (custom && typeof custom === 'object') {
          Object.entries(custom).forEach(([id, combo]) => {
            if (combo) this._keymap.set(id, combo as string);
            else this._keymap.delete(id);
          });
        }
      }
    } catch {}
  }

  public save(): void {
    try {
      const obj: Record<string, string> = {};
      this._keymap.forEach((combo, id) => {
        obj[id] = combo;
      });
      localStorage.setItem(STORAGE_KEY, JSON.stringify(obj));
    } catch {}
  }

  public getBinding(actionId: string): string {
    return this._keymap.get(actionId) || '';
  }

  public setBinding(actionId: string, combo?: string): void {
    if (!actionId) return;
    if (!combo) {
      this._keymap.delete(actionId);
    } else {
      this._keymap.set(actionId, combo);
    }
    this.save();
  }

  public resetDefaults(): void {
    this._keymap.clear();
    ACTION_DEFINITIONS.forEach((act) => {
      this._keymap.set(act.id, act.defaultKey);
    });
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {}
  }

  public findActionForEvent(e: { key: string; ctrlKey?: boolean; metaKey?: boolean; altKey?: boolean; shiftKey?: boolean }): string | null {
    const eventCombo = HotkeyRegistry.eventToCombo(e);
    if (!eventCombo) return null;

    const lowerEvent = eventCombo.toLowerCase();
    for (const [id, combo] of this._keymap.entries()) {
      if (combo && combo.toLowerCase() === lowerEvent) {
        return id;
      }
    }
    return null;
  }

  /**
   * Converts KeyboardEvent into normalized combo e.g. "Ctrl+Shift+T" or "Alt+ArrowLeft"
   */
  public static eventToCombo(e: { key: string; ctrlKey?: boolean; metaKey?: boolean; altKey?: boolean; shiftKey?: boolean }): string {
    const parts: string[] = [];
    if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');

    let key = e.key;
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(key)) {
      return '';
    }

    if (key === ' ') key = 'Space';
    else if (key === '`' || key === '~') key = '`';
    else if (key === ',') key = ',';
    else if (key.length === 1) key = key.toUpperCase();

    parts.push(key);
    return parts.join('+');
  }

  /**
   * Formats a combo into Apple-style glyphs: ⌘, ⌥, ⇧, ⌃
   */
  public static formatForDisplay(combo: string): string {
    if (!combo) return '—';
    return combo
      .replace(/\bCtrl\b/g, '⌃')
      .replace(/\bAlt\b/g, '⌥')
      .replace(/\bShift\b/g, '⇧')
      .replace(/\bMeta\b/g, '⌘')
      .replace(/\+/g, ' ');
  }
}
