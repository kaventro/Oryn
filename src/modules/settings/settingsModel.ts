// src/modules/settings/settingsModel.ts

export interface AppSettingsData {
  paneMode?: 'single' | 'dual' | string;
  trayTheme?: string;
  showHiddenFiles?: boolean;
  showExtensions?: boolean;
  showStatusBarTerminal?: boolean;
  confirmDelete?: boolean;
  overwritePolicy?: 'prompt' | 'overwrite' | 'skip' | string;
  defaultDiffRef?: 'HEAD' | 'HEAD~1' | string;
  defaultEditor?: 'vscode' | 'cursor' | 'sublime' | 'custom' | string;
  customEditorCmd?: string;
  rowDensity?: 'compact' | 'normal' | 'comfortable' | string;
  dateFormat?: 'iso' | 'relative' | string;
  dualPaneDriveDefaults?: boolean;
  leftDefaultDrive?: string;
  rightDefaultDrive?: string;
  enableSftp?: boolean;
}

/**
 * Domain model for all Oryn application preferences.
 * Adheres to Single Responsibility Principle (SRP) for settings state & validation.
 */
export class AppSettings {
  public paneMode: 'single' | 'dual';
  public trayTheme: string;
  public showHiddenFiles: boolean;
  public showExtensions: boolean;
  public showStatusBarTerminal: boolean;
  public confirmDelete: boolean;
  public overwritePolicy: 'prompt' | 'overwrite' | 'skip';
  public defaultDiffRef: 'HEAD' | 'HEAD~1';
  public defaultEditor: 'vscode' | 'cursor' | 'sublime' | 'custom';
  public customEditorCmd: string;
  public rowDensity: 'compact' | 'normal' | 'comfortable';
  public dateFormat: 'iso' | 'relative';
  public dualPaneDriveDefaults: boolean;
  public leftDefaultDrive: string;
  public rightDefaultDrive: string;
  public enableSftp: boolean;

  constructor(initial: AppSettingsData = {}) {
    this.paneMode = initial.paneMode === 'single' ? 'single' : 'dual';
    this.trayTheme = initial.trayTheme || 'balanced';
    this.showHiddenFiles = Boolean(initial.showHiddenFiles);
    this.showExtensions = initial.showExtensions !== false;
    this.showStatusBarTerminal = initial.showStatusBarTerminal !== false;
    this.confirmDelete = initial.confirmDelete !== false;
    this.enableSftp = Boolean(initial.enableSftp);
    this.overwritePolicy = ['prompt', 'overwrite', 'skip'].includes(initial.overwritePolicy as string)
      ? (initial.overwritePolicy as 'prompt' | 'overwrite' | 'skip')
      : 'prompt';
    this.defaultDiffRef = initial.defaultDiffRef === 'HEAD~1' ? 'HEAD~1' : 'HEAD';
    this.defaultEditor = ['vscode', 'cursor', 'sublime', 'custom'].includes(initial.defaultEditor as string)
      ? (initial.defaultEditor as 'vscode' | 'cursor' | 'sublime' | 'custom')
      : 'vscode';
    this.customEditorCmd = String(initial.customEditorCmd || '');
    this.rowDensity = ['compact', 'normal', 'comfortable'].includes(initial.rowDensity as string)
      ? (initial.rowDensity as 'compact' | 'normal' | 'comfortable')
      : 'normal';
    this.dateFormat = initial.dateFormat === 'iso' ? 'iso' : 'relative';
    // In Dual Pane mode, seed each panel on its own volume at startup when more
    // than one is present. Empty string = "Automatic": the left panel opens on
    // the home volume (C:\ on Windows, / on macOS/Linux) and the right on the
    // next distinct volume. A non-empty value pins a specific mount point.
    this.dualPaneDriveDefaults = initial.dualPaneDriveDefaults !== false;
    this.leftDefaultDrive =
      typeof initial.leftDefaultDrive === 'string' ? initial.leftDefaultDrive : '';
    this.rightDefaultDrive =
      typeof initial.rightDefaultDrive === 'string' ? initial.rightDefaultDrive : '';
  }

  public clone(): AppSettings {
    return new AppSettings(JSON.parse(JSON.stringify(this)));
  }

  public toJSON(): AppSettingsData {
    return {
      paneMode: this.paneMode,
      trayTheme: this.trayTheme,
      showHiddenFiles: this.showHiddenFiles,
      showExtensions: this.showExtensions,
      showStatusBarTerminal: this.showStatusBarTerminal,
      confirmDelete: this.confirmDelete,
      overwritePolicy: this.overwritePolicy,
      defaultDiffRef: this.defaultDiffRef,
      defaultEditor: this.defaultEditor,
      customEditorCmd: this.customEditorCmd,
      rowDensity: this.rowDensity,
      dateFormat: this.dateFormat,
      dualPaneDriveDefaults: this.dualPaneDriveDefaults,
      leftDefaultDrive: this.leftDefaultDrive,
      rightDefaultDrive: this.rightDefaultDrive,
      enableSftp: this.enableSftp,
    };
  }

  public static createDefault(): AppSettings {
    return new AppSettings();
  }
}
