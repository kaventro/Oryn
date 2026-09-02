// src/modules/keyboardController.ts
// Handles all global keyboard shortcuts and delegates to the appropriate controllers.
import { fmtBytes } from './formatUtils.ts';

export interface KeyboardControllerDeps {
  state: any;
  pathHeaderController: any;
  tabsRenderer: any;
  quickViewController: any;
  commandsController: any;
  searchController: any;
  propertiesController: any;
  gitController: any;
  compareDirsController: any;
  ctxMenuController?: any;
  multiRenameController?: any;
  terminalDrawerController?: any;
  folderSyncController?: any;
  checksumController?: any;
  preferencesController?: any;
  commandPaletteController?: any;
  diskSpaceController?: any;
  duplicateFinderController?: any;
  hotkeyRegistry?: any;
  focusActiveList: () => void;
  syncFilterInput: () => void;
  updatePaneClass: () => void;
  otherSide: (side: 'left' | 'right') => 'left' | 'right';
  refreshAll: () => Promise<void> | void;
  browseFolderPicker: () => Promise<void> | void;
  focusFilterInput: () => void;
  fileOps: any;
  panelController: any;
  listRenderer: any;
  getFilteredSelection: (side: 'left' | 'right') => any;
  paintVirtualPane?: (side: 'left' | 'right', full?: boolean) => void;
  renderPane: (side: 'left' | 'right') => void;
  moveCursor: (side: 'left' | 'right', delta: number) => void;
  setStatus?: (msg: string) => void;
  selectionController?: any;
  drivePopupController?: any;
  viewController?: any;
  columnsViewController?: any;
}

export class KeyboardController {
  public state: any;
  public pathHeaderController: any;
  public tabsRenderer: any;
  public quickViewController: any;
  public commandsController: any;
  public searchController: any;
  public propertiesController: any;
  public gitController: any;
  public compareDirsController: any;
  public ctxMenuController: any;
  public multiRenameController: any;
  public terminalDrawerController: any;
  public folderSyncController: any;
  public checksumController: any;
  public preferencesController: any;
  public commandPaletteController: any;
  public diskSpaceController: any;
  public duplicateFinderController: any;
  public hotkeyRegistry: any;
  public focusActiveList: () => void;
  public syncFilterInput: () => void;
  public updatePaneClass: () => void;
  public otherSide: (side: 'left' | 'right') => 'left' | 'right';
  public refreshAll: () => Promise<void> | void;
  public browseFolderPicker: () => Promise<void> | void;
  public focusFilterInput: () => void;
  public fileOps: any;
  public panelController: any;
  public listRenderer: any;
  public getFilteredSelection: (side: 'left' | 'right') => any;
  public paintVirtualPane?: (side: 'left' | 'right', full?: boolean) => void;
  public renderPane: (side: 'left' | 'right') => void;
  public moveCursor: (side: 'left' | 'right', delta: number) => void;
  public setStatus: (msg: string) => void;
  public selectionController: any;
  public drivePopupController: any;
  public viewController: any;
  public columnsViewController: any;

  private _sizing = new Set<string>();
  private _typeSearchBuf = '';
  private _typeSearchLastChar = '';
  private _typeSearchTimer: any = null;

  constructor(deps: KeyboardControllerDeps) {
    this.state = deps.state;
    this.pathHeaderController = deps.pathHeaderController;
    this.tabsRenderer = deps.tabsRenderer;
    this.quickViewController = deps.quickViewController;
    this.commandsController = deps.commandsController;
    this.searchController = deps.searchController;
    this.propertiesController = deps.propertiesController;
    this.gitController = deps.gitController;
    this.compareDirsController = deps.compareDirsController;
    this.ctxMenuController = deps.ctxMenuController;
    this.multiRenameController = deps.multiRenameController;
    this.terminalDrawerController = deps.terminalDrawerController;
    this.folderSyncController = deps.folderSyncController;
    this.checksumController = deps.checksumController;
    this.preferencesController = deps.preferencesController;
    this.commandPaletteController = deps.commandPaletteController;
    this.diskSpaceController = deps.diskSpaceController;
    this.duplicateFinderController = deps.duplicateFinderController;
    this.hotkeyRegistry = deps.hotkeyRegistry || deps.preferencesController?.hotkeyRegistry;
    this.focusActiveList = deps.focusActiveList;
    this.syncFilterInput = deps.syncFilterInput;
    this.updatePaneClass = deps.updatePaneClass;
    this.otherSide = deps.otherSide;
    this.refreshAll = deps.refreshAll;
    this.browseFolderPicker = deps.browseFolderPicker;
    this.focusFilterInput = deps.focusFilterInput;
    this.fileOps = deps.fileOps;
    this.panelController = deps.panelController;
    this.listRenderer = deps.listRenderer;
    this.getFilteredSelection = deps.getFilteredSelection;
    this.paintVirtualPane = deps.paintVirtualPane;
    this.renderPane = deps.renderPane;
    this.moveCursor = deps.moveCursor;
    this.setStatus = deps.setStatus || (() => {});
    this.selectionController = deps.selectionController;
    this.drivePopupController = deps.drivePopupController;
    this.viewController = deps.viewController;
    this.columnsViewController = deps.columnsViewController;
  }

  private _sizingSuffix(): string {
    if (!this._sizing.size) return '';
    const names = [...this._sizing];
    const head = names.slice(0, 2).join(', ');
    const rest = names.length > 2 ? ` +${names.length - 2}` : '';
    return ` | calculating: ${head}${rest}…`;
  }

  private _computeDirSize(side: 'left' | 'right', item: any): void {
    const state = this.state;
    item.sizing = true;
    this._sizing.add(item.base);
    state[side].listSerial += 1;
    this.renderPane(side);
    this.setStatus(`Calculating size: ${item.base}…${this._sizing.size > 1 ? ` (${this._sizing.size} running)` : ''}`);
    void this.panelController.fullPath(state[side], item).then((path: string | null) => {
      if (!path) {
        item.sizing = false;
        this._sizing.delete(item.base);
        return;
      }
      (window as any).ow.getDirSize(path).then((res: any) => {
        item.sizing = false;
        this._sizing.delete(item.base);
        if (res?.ok && res.size !== undefined) {
          item.size = res.size;
          const extra = res.files !== undefined ? ` — ${res.files} files, ${res.dirs} dirs` : '';
          this.setStatus(`${item.base}: ${fmtBytes(res.size)} (${res.size.toLocaleString()} bytes)${extra}${this._sizingSuffix()}`);
        }
        state[side].listSerial += 1;
        this.renderPane(side);
      }).catch((err: any) => {
        item.sizing = false;
        this._sizing.delete(item.base);
        state[side].listSerial += 1;
        this.renderPane(side);
        this.setStatus(`Size failed: ${err?.message || err}${this._sizingSuffix()}`);
      });
    });
  }

  private async _gitTarget(side: 'left' | 'right'): Promise<{ rel: string; root: string; item: any } | null> {
    const pane = this.state[side];
    if (!pane?.git?.isRepo) {
      this.setStatus('Not a git repository.');
      return null;
    }
    const { item } = this.getFilteredSelection(side);
    if (!item || item.base === '..') return null;
    const fp = await this.panelController.fullPath(pane, item);
    if (!fp) return null;
    let rel = fp;
    if (rel.startsWith(pane.git.root)) {
      rel = rel.slice(pane.git.root.length).replace(/^[/\\]+/, '');
    }
    return { rel, root: pane.git.root, item };
  }

  private async _runGitFileAction(actionId: string, side: 'left' | 'right'): Promise<void> {
    const target = await this._gitTarget(side);
    if (!target) return;
    const { rel, root, item } = target;

    if (actionId === 'gitDiffFile') {
      await this.gitController.openDiffForFile(rel, root);
    } else if (actionId === 'gitBlameFile') {
      await this.gitController.openBlameForFile(rel, root);
    } else if (actionId === 'gitFileHistory') {
      await this.gitController.openLogForFile(rel, root);
    } else if (actionId === 'gitStageFile') {
      const isStaged = item.gitStatus === 'A';
      await (window as any).ow.gitStageFile(root, rel, !isStaged);
      this.setStatus(isStaged ? `Unstaged ${item.base}` : `Staged ${item.base}`);
      await this.panelController.loadDir(side);
    } else if (actionId === 'gitDiscardChanges') {
      if (!window.confirm(`Discard all uncommitted changes in "${item.base}"?`)) return;
      await (window as any).ow.gitRestore(root, rel, false);
      this.setStatus(`Restored ${item.base}`);
      await this.panelController.loadDir(side);
    }
  }

  private _resolveActiveSide(): 'left' | 'right' {
    const activeEl = typeof document !== 'undefined' ? document.activeElement : null;
    if (activeEl) {
      if (activeEl.id === 'list-right' || activeEl.closest?.('#pane-right')) {
        if (this.state.active !== 'right') {
          this.state.active = 'right';
          this.syncFilterInput();
          this.updatePaneClass();
        }
        return 'right';
      }
      if (activeEl.id === 'list-left' || activeEl.closest?.('#pane-left')) {
        if (this.state.active !== 'left') {
          this.state.active = 'left';
          this.syncFilterInput();
          this.updatePaneClass();
        }
        return 'left';
      }
    }
    return (this.state.active as 'left' | 'right') || 'left';
  }

  public executeAction(actionId: string): void {
    const side = this._resolveActiveSide();
    const opp = this.otherSide(side);

    switch (actionId) {
      case 'commandPalette':
        this.commandPaletteController?.toggle();
        break;
      case 'diskSpace':
        this.diskSpaceController?.toggle();
        break;
      case 'duplicateFinder':
        this.duplicateFinderController?.toggle();
        break;
      case 'viewFile':
        this.commandsController.runCommand('viewFile');
        break;
      case 'quickView':
        this.quickViewController.toggle(opp);
        this.renderPane(opp);
        break;
      case 'switchPanel': {
        const nextSide = this.otherSide(this.state.active);
        this.state.active = nextSide;
        this.syncFilterInput();
        this.updatePaneClass();
        this.paintVirtualPane?.('left', false);
        this.paintVirtualPane?.('right', false);
        this.focusActiveList();
        break;
      }
      case 'swapPanels':
        void this.fileOps.swapPanels();
        break;
      case 'goRoot':
        void this.fileOps.goRoot(side);
        break;
      case 'branchView':
        void this.fileOps.toggleBranchView(side);
        break;
      case 'cloneFile':
        void this.fileOps.cloneSelection(side);
        break;
      case 'properties': {
        const { item } = this.getFilteredSelection(side);
        void this.propertiesController?.open(side, item);
        break;
      }
      case 'goBack':
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          void this.columnsViewController.goBack(side);
        } else {
          void this.panelController.goBack(side);
        }
        break;
      case 'goFwd':
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          void this.columnsViewController.goForward(side);
        } else {
          void this.panelController.goFwd(side);
        }
        break;
      case 'goParent':
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          void this.columnsViewController.goBack(side);
        } else {
          void this.pathHeaderController.goParent(side);
        }
        break;
      case 'refresh':
        void this.refreshAll();
        break;
      case 'editPath':
        this.pathHeaderController.beginEdit(side);
        break;
      case 'calcDirSize': {
        const { item } = this.getFilteredSelection(side);
        if (item && item.base !== '..' && item.isDir && !item.sizing) {
          this._computeDirSize(side, item);
        }
        break;
      }
      case 'rename':
        void this.fileOps.beginRename();
        break;
      case 'editVSCode':
        this.commandsController.runCommand('editVSCode');
        break;
      case 'copy':
        this.commandsController.runCommand('copy');
        break;
      case 'move':
        this.commandsController.runCommand('move');
        break;
      case 'newFile':
        this.commandsController.runCommand('newFile');
        break;
      case 'mkdir':
        this.commandsController.runCommand('mkdir');
        break;
      case 'changeDriveLeft':
        this.drivePopupController?.open('left');
        break;
      case 'changeDriveRight':
        this.drivePopupController?.open('right');
        break;
      case 'compressArchive':
        void this.fileOps.compressSelection(side);
        break;
      case 'extractArchive':
        void this.fileOps.extractArchive(side);
        break;
      case 'delete':
        void this.fileOps.beginDelete();
        break;
      case 'multiRename':
        this.multiRenameController?.open();
        break;
      case 'openSearch':
        this.searchController.openOverlay();
        break;
      case 'focusFilter':
        this.focusFilterInput();
        break;
      case 'openGit':
        void this.gitController.open();
        break;
      case 'openCompare':
        this.compareDirsController.open();
        break;
      case 'toggleTerminal':
        this.terminalDrawerController?.toggle();
        break;
      case 'openPreferences':
        this.preferencesController?.open();
        break;
      case 'selectAll':
        this.commandsController.runCommand('selectAll');
        break;
      case 'selectByPattern':
        this.commandsController.runCommand('selectByPattern');
        break;
      case 'deselectByPattern':
        this.commandsController.runCommand('deselectByPattern');
        break;
      case 'invertSelection':
        this.commandsController.runCommand('invertSelection');
        break;
      case 'clearSelection':
        this.commandsController.runCommand('clearSelection');
        break;
      case 'openNewTab': {
        void (async () => {
          const { item } = this.getFilteredSelection(side);
          if (!item || item.base === '..') return;
          const fp = await this.panelController.fullPath(this.state[side], item);
          if (!fp) return;
          this.state[side].addTab(fp);
          this.tabsRenderer?.render(side);
          await this.panelController.loadDir(side);
        })();
        break;
      }
      case 'openTerminalHere': {
        void (async () => {
          const { item } = this.getFilteredSelection(side);
          let target = this.state[side]?.path;
          if (item && item.isDir && item.base !== '..') {
            target = await this.panelController.fullPath(this.state[side], item);
          }
          if (target) await (window as any).ow.openTerminal(target);
        })();
        break;
      }
      case 'properties':
        void this.propertiesController?.showFor(side);
        break;
      case 'checksum': {
        void (async () => {
          const { item } = this.getFilteredSelection(side);
          if (!item || item.isDir || item.base === '..') return;
          const fp = await this.panelController.fullPath(this.state[side], item);
          if (fp) await this.checksumController?.openForPath(fp);
        })();
        break;
      }
      case 'gitDiffFile':
      case 'gitBlameFile':
      case 'gitFileHistory':
      case 'gitStageFile':
      case 'gitDiscardChanges':
        void this._runGitFileAction(actionId, side);
        break;
      default:
        this.commandsController.runCommand(actionId);
        break;
    }
  }

  public setup(): void {
    document.addEventListener('keydown', (e) => this._onKeydown(e), true);
  }

  private _onKeydown(e: KeyboardEvent): void {
    const state = this.state;
    const inTerminal = !!(e.target as HTMLElement)?.closest?.('#terminal-drawer');
    const inFilter = (e.target as HTMLElement)?.id === 'filter-input';
    const inRename = (e.target as HTMLElement)?.id === 'rename-input';
    const inCmdLine = (e.target as HTMLElement)?.id === 'nx-cmd-input';
    const inPathEdit = this.pathHeaderController.isEditing();
    const isInputField = !!(e.target && ((e.target as HTMLElement).tagName === 'INPUT' || (e.target as HTMLElement).tagName === 'TEXTAREA' || (e.target as HTMLElement).isContentEditable));

    const cpOverlay = document.getElementById('command-palette-overlay');
    const cpOpen = cpOverlay && !cpOverlay.classList.contains('hidden');

    const searchOverlay = document.getElementById('search-overlay');
    const searchOpen = searchOverlay && !searchOverlay.classList.contains('hidden');
    const focusInFindSearch = searchOverlay?.contains(e.target as Node);
    const propertiesOverlay = document.getElementById('properties-overlay');
    const propertiesOpen = propertiesOverlay && !propertiesOverlay.classList.contains('hidden');

    // Terminal Drawer has full priority over its own input and typing
    if (inTerminal) {
      if (((e.ctrlKey || e.metaKey) && (e.key === '`' || e.key === '~' || e.code === 'Backquote')) || e.key === 'F9') {
        e.preventDefault();
        this.terminalDrawerController?.toggle();
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        this.terminalDrawerController?.hide();
        return;
      }
      return;
    }

    // Command Palette hotkey: Ctrl+P / Cmd+P or Ctrl+K / Cmd+K
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key.toLowerCase() === 'p' || e.key.toLowerCase() === 'k')) {
      e.preventDefault();
      this.commandPaletteController?.toggle();
      return;
    }

    if (cpOpen) {
      // Handled internally by command palette
      return;
    }

    if (inRename || inPathEdit || inCmdLine) return;

    // Generic text inputs (search inputs, checksum verify, modal inputs)
    if (isInputField && !inFilter) {
      if (e.key !== 'Escape') {
        return;
      }
    }

    // Generic dialogs (overwrite choice, mask prompt) own the keyboard while open.
    const choiceOv = document.getElementById('choice-overlay');
    const promptOv = document.getElementById('prompt-overlay');
    if ((choiceOv && !choiceOv.classList.contains('hidden')) ||
        (promptOv && !promptOv.classList.contains('hidden'))) {
      return;
    }

    // Open Preferences with Cmd+, or Ctrl+,
    if ((e.ctrlKey || e.metaKey) && (e.key === ',' || e.code === 'Comma')) {
      e.preventDefault();
      this.preferencesController?.open();
      return;
    }

    // Standard File Operations: Copy (Ctrl+C/Cmd+C), Cut (Ctrl+X/Cmd+X), Paste (Ctrl+V/Cmd+V), Duplicate (Ctrl+D/Cmd+D)
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !isInputField) {
      if (e.code === 'KeyC') {
        e.preventDefault();
        void this.fileOps.copySelectionToClipboard(state.active);
        return;
      }
      if (e.code === 'KeyX') {
        e.preventDefault();
        void this.fileOps.cutSelectionToClipboard(state.active);
        return;
      }
      if (e.code === 'KeyV') {
        e.preventDefault();
        void this.fileOps.pasteFromClipboard(state.active);
        return;
      }
      if (e.code === 'KeyD' && !e.shiftKey) {
        e.preventDefault();
        void this.fileOps.cloneSelection(state.active);
        return;
      }
      if (e.code === 'KeyN') {
        e.preventDefault();
        if (e.shiftKey) {
          this.commandsController.runCommand('mkdir');
        } else {
          this.commandsController.runCommand('newFile');
        }
        return;
      }
    }

    // Dynamic Hotkeys Matching via HotkeyRegistry
    if (this.hotkeyRegistry && !inRename && !inPathEdit && !inCmdLine && !searchOpen && !propertiesOpen) {
      const matchedActionId = this.hotkeyRegistry.findActionForEvent(e);
      if (matchedActionId) {
        const binding = this.hotkeyRegistry.getBinding(matchedActionId);
        if (inFilter && !e.ctrlKey && !e.altKey && !e.metaKey && matchedActionId !== 'switchPanel' && !binding.startsWith('F')) {
          // ignore plain letters in filter
        } else {
          e.preventDefault();
          this.executeAction(matchedActionId);
          return;
        }
      }
    }

    if (this.tabsRenderer.handleGlobalKeydown(e)) return;

    if (e.altKey && (e.key === 'F4' || e.code === 'F4')) {
      e.preventDefault();
      this.commandsController.runCommand('quit');
      return;
    }

    if (e.altKey && !e.ctrlKey && !e.metaKey) {
      const altSelectionMap: Record<string, string> = {
        KeyS: 'selectByPattern',
        KeyD: 'deselectByPattern',
        KeyI: 'invertSelection',
        KeyE: 'selectByExtension',
        KeyA: 'clearSelection',
      };
      const cmd = altSelectionMap[e.code];
      if (cmd) {
        e.preventDefault();
        this.commandsController.runCommand(cmd);
        return;
      }
    }

    if ((e.ctrlKey || e.metaKey) && !e.altKey && e.code === 'KeyA'
        && (document.activeElement as HTMLElement)?.tagName !== 'INPUT') {
      e.preventDefault();
      this.commandsController.runCommand('selectAll');
      return;
    }

    if (searchOpen && e.key === 'Escape') {
      e.preventDefault();
      this.searchController.hideOverlay();
      return;
    }

    if (propertiesOpen && e.key === 'Escape') {
      e.preventDefault();
      this.propertiesController.hide();
      return;
    }

    if (propertiesOpen && propertiesOverlay?.contains(e.target as Node)) return;

    if (
      searchOpen && focusInFindSearch &&
      ((e.target as HTMLElement)?.id === 'search-filename' ||
        (e.target as HTMLElement)?.id === 'search-content' ||
        (e.target as HTMLElement)?.id === 'search-not-content' ||
        (e.target as HTMLElement)?.id === 'search-exclude' ||
        (e.target as HTMLElement)?.id === 'search-root-input' ||
        (e.target as HTMLElement)?.id === 'search-replace-text' ||
        (e.target as HTMLElement)?.id === 'search-results' ||
        (e.target as HTMLElement)?.closest?.('#search-overlay select') ||
        (e.target as HTMLElement)?.closest?.('#search-overlay textarea'))
    ) {
      return;
    }

    if (e.key === 'Escape') {
      if (this.commandPaletteController?.isOpen()) {
        e.preventDefault();
        this.commandPaletteController.hide();
        return;
      }
      if (this.diskSpaceController?.isOpen()) {
        e.preventDefault();
        this.diskSpaceController.hide();
        return;
      }
      if (this.duplicateFinderController?.isOpen()) {
        e.preventDefault();
        this.duplicateFinderController.hide();
        return;
      }
      if (this.ctxMenuController && this.ctxMenuController.isOpen()) {
        e.preventDefault();
        this.ctxMenuController.hide();
        return;
      }
      if (this.multiRenameController?.isOpen) {
        e.preventDefault();
        this.multiRenameController.hide();
        return;
      }
      if (this.terminalDrawerController?.isOpen) {
        e.preventDefault();
        this.terminalDrawerController.hide();
        return;
      }
      if (this.folderSyncController?.isOpen) {
        e.preventDefault();
        this.folderSyncController.hide();
        return;
      }
      const gitOv = document.getElementById('git-overlay');
      if (gitOv && !gitOv.classList.contains('hidden')) {
        e.preventDefault();
        this.gitController?.close();
        return;
      }
      const csOv = document.getElementById('checksum-overlay');
      if (csOv && !csOv.classList.contains('hidden')) {
        e.preventDefault();
        this.checksumController?.hide();
        return;
      }
      const cmpOv = document.getElementById('compare-overlay');
      if (cmpOv && !cmpOv.classList.contains('hidden')) {
        e.preventDefault();
        this.compareDirsController?.hide();
        return;
      }
      const diffOv = document.getElementById('diff-overlay');
      if (diffOv && !diffOv.classList.contains('hidden')) {
        e.preventDefault();
        diffOv.classList.add('hidden');
        this.focusActiveList();
        return;
      }
      const propOv = document.getElementById('properties-overlay');
      if (propOv && !propOv.classList.contains('hidden')) {
        e.preventDefault();
        this.propertiesController?.hide();
        return;
      }
      const prefOv = document.getElementById('preferences-overlay');
      if (prefOv && !prefOv.classList.contains('hidden')) {
        e.preventDefault();
        this.preferencesController?.close();
        return;
      }
      if (this.commandsController.anyMenuOpen()) {
        e.preventDefault();
        this.commandsController.closeAllMenus();
        return;
      }
      if (state.copyInProgress) {
        e.preventDefault();
        this.commandsController.runCommand('cancelXfer');
        return;
      }
      if (inFilter) {
        e.preventDefault();
        this.focusActiveList();
        return;
      }
      return;
    }

    if (((e.ctrlKey || e.metaKey) && (e.key === '`' || e.key === '~' || e.code === 'Backquote')) || e.key === 'F9') {
      e.preventDefault();
      this.terminalDrawerController?.toggle();
      return;
    }

    if (e.ctrlKey && e.key.toLowerCase() === 'm') {
      e.preventDefault();
      this.multiRenameController?.open();
      return;
    }

    if (e.ctrlKey && e.key.toLowerCase() === 'g') {
      e.preventDefault();
      void this.gitController.open();
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 'd') {
      e.preventDefault();
      this.diskSpaceController?.toggle();
      return;
    }

    if (e.ctrlKey && e.key.toLowerCase() === 'd') {
      e.preventDefault();
      this.compareDirsController.open();
      return;
    }

    if (e.ctrlKey && e.key.toLowerCase() === 'q') {
      e.preventDefault();
      const opp = this.otherSide(state.active);
      this.quickViewController.toggle(opp);
      this.renderPane(opp);
      return;
    }

    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === 't') {
      e.preventDefault();
      this.searchController.openOverlay();
      return;
    }

    if (e.ctrlKey && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      this.focusFilterInput();
      return;
    }

    if (e.ctrlKey && e.key.toLowerCase() === 'l') {
      e.preventDefault();
      this.pathHeaderController.beginEdit(state.active);
      return;
    }

    if ((e.metaKey || e.ctrlKey) && (e.key === '1' || e.key === '2' || e.key === '3')) {
      e.preventDefault();
      const mode = e.key === '1' ? 'list' : e.key === '2' ? 'grid' : 'columns';
      if (this.viewController) {
        this.viewController.setMode(mode);
        this.setStatus(`View mode: ${mode}`);
      }
      return;
    }

    if (e.ctrlKey && e.key.toLowerCase() === 'r') {
      e.preventDefault();
      void this.refreshAll();
      return;
    }

    if (e.altKey && e.key === 'ArrowLeft') {
      e.preventDefault();
      void this.panelController.goBack(state.active);
      return;
    }

    if (e.altKey && e.key === 'ArrowRight') {
      e.preventDefault();
      void this.panelController.goFwd(state.active);
      return;
    }

    if (e.ctrlKey && e.key.toLowerCase() === 'b') {
      e.preventDefault();
      void this.browseFolderPicker();
      return;
    }

    if (e.key === 'Tab' && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      const nextSide = this.otherSide(state.active);
      state.active = nextSide;
      this.syncFilterInput();
      this.updatePaneClass();
      this.paintVirtualPane?.('left', false);
      this.paintVirtualPane?.('right', false);
      this.focusActiveList();
      return;
    }

    if (inFilter) {
      if (e.key === 'F3') { e.preventDefault(); this.commandsController.runCommand('viewFile'); return; }
      if ((e.key === 'F2' || e.code === 'F2') && !e.altKey) {
        e.preventDefault();
        void this.fileOps.beginRename();
        return;
      }
      if ((e.key === 'F4' || e.code === 'F4') && !e.altKey && !e.shiftKey) {
        e.preventDefault();
        void this.fileOps.beginRename();
        return;
      }
      if ((e.key === 'F4' || e.code === 'F4') && e.shiftKey) {
        e.preventDefault();
        this.commandsController.runCommand('editVSCode');
        return;
      }
      if (e.key === 'F7') {
        e.preventDefault();
        this.commandsController.runCommand(e.shiftKey ? 'newFile' : 'mkdir');
        return;
      }
      if (e.key === 'F8') { e.preventDefault(); void this.fileOps.beginDelete(); return; }
      if (e.key === 'Delete' || e.key === 'Backspace') {
        const fi = e.target as HTMLInputElement;
        const v = String(fi.value ?? '');
        const start = fi.selectionStart ?? 0;
        const end = fi.selectionEnd ?? 0;
        const hasSel = start !== end;
        if (e.key === 'Backspace' && (hasSel || (v.length > 0 && start > 0))) return;
        if (e.key === 'Delete' && (hasSel || (v.length > 0 && start < v.length))) return;
        e.preventDefault();
        void this.fileOps.beginDelete();
        return;
      }
      return;
    }

    const side = this._resolveActiveSide();

    switch (e.key) {
      case 'F3':
        e.preventDefault();
        this.commandsController.runCommand('viewFile');
        break;
      case 'F4':
        e.preventDefault();
        this.commandsController.runCommand('editVSCode');
        break;
      case 'F2':
        e.preventDefault();
        void this.fileOps.beginRename();
        break;
      case 'F5':
        e.preventDefault();
        if (e.shiftKey) {
          void this.fileOps.cloneSelection(side);
        } else {
          void this.fileOps.copyToOther();
        }
        break;
      case 'F6':
        e.preventDefault();
        if (e.shiftKey) {
          void this.fileOps.beginRename();
        } else {
          void this.fileOps.moveToOther();
        }
        break;
      case 'F7':
        e.preventDefault();
        this.commandsController.runCommand(e.shiftKey ? 'newFile' : 'mkdir');
        break;
      case 'Insert': {
        e.preventDefault();
        const { item } = this.getFilteredSelection(side);
        if (item && item.base !== '..') {
          state[side].activeTab.toggleSelection(item.base);
          this.selectionController?.updateIndicator(side);
        }
        this.moveCursor(side, 1);
        break;
      }
      case ' ':
      case 'Space': {
        const overlay = document.getElementById('viewer-overlay');
        if (overlay && !overlay.classList.contains('hidden')) {
          e.preventDefault();
          overlay.classList.add('hidden');
          this.focusActiveList();
          return;
        }
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          const cols = this.columnsViewController.getColumns(side);
          const activeIdx = this.columnsViewController.getActiveColumnIndex(side);
          const col = cols[activeIdx];
          const item = col?.selectedItem;
          if (item && !item.isDir) {
            e.preventDefault();
            void this.columnsViewController.joinPath(col.path, item.base).then((fp: string) => {
              void this.commandsController.openViewer(fp, item);
            });
            return;
          }
        }
        const { item } = this.getFilteredSelection ? this.getFilteredSelection(side) : { item: null };
        if (item && item.base !== '..') {
          e.preventDefault();
          if (item.isDir) {
            this._computeDirSize(side, item);
          } else {
            this.commandsController.runCommand('viewFile');
          }
        }
        break;
      }
      case 'j':
        if (state.config.vimNavigation) { e.preventDefault(); this.moveCursor(side, 1); }
        break;
      case 'k':
        if (state.config.vimNavigation) { e.preventDefault(); this.moveCursor(side, -1); }
        break;
      case 'l':
        if (state.config.vimNavigation) {
          e.preventDefault();
          void this.panelController.openSelected(side);
        }
        break;
      case 'h':
        if (state.config.vimNavigation) {
          e.preventDefault();
          const labelEl = document.querySelector(`#pane-${side} .pane-path-label`) as HTMLElement;
          if (labelEl) labelEl.click();
        }
        break;
      case 'ArrowDown':
        if (this.ctxMenuController && this.ctxMenuController.isOpen()) {
          e.preventDefault();
          this.ctxMenuController.navigate(1);
          return;
        }
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          e.preventDefault();
          void this.columnsViewController.navigate(0, 1);
          return;
        }
        e.preventDefault();
        this.moveCursor(side, 1);
        break;
      case 'ArrowUp':
        if (this.ctxMenuController && this.ctxMenuController.isOpen()) {
          e.preventDefault();
          this.ctxMenuController.navigate(-1);
          return;
        }
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          e.preventDefault();
          void this.columnsViewController.navigate(0, -1);
          return;
        }
        e.preventDefault();
        this.moveCursor(side, -1);
        break;
      case 'ArrowRight':
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          e.preventDefault();
          void this.columnsViewController.navigate(1, 0);
          return;
        }
        break;
      case 'ArrowLeft':
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          e.preventDefault();
          void this.columnsViewController.navigate(-1, 0);
          return;
        }
        break;
      case 'Enter':
        if (this.ctxMenuController && this.ctxMenuController.isOpen()) {
          e.preventDefault();
          this.ctxMenuController.executeSelected();
          return;
        }
        e.preventDefault();
        void this.panelController.openSelected(side);
        break;
      case 'F5':
        e.preventDefault();
        void this.fileOps.copyToOther();
        break;
      case 'F6':
        e.preventDefault();
        void this.fileOps.moveToOther();
        break;
      case '+':
        if (e.location === 3) { e.preventDefault(); this.commandsController.runCommand('selectByPattern'); }
        break;
      case '-':
        if (e.location === 3) { e.preventDefault(); this.commandsController.runCommand('deselectByPattern'); }
        break;
      case '*':
        if (e.location === 3) { e.preventDefault(); this.commandsController.runCommand('invertSelection'); }
        break;
      case 'F8':
      case 'Delete':
        e.preventDefault();
        void this.fileOps.beginDelete({ permanent: e.shiftKey });
        break;
      case 'Backspace':
        if (inFilter || (document.activeElement as HTMLElement)?.tagName === 'INPUT') break;
        e.preventDefault();
        if (document.getElementById('app')?.classList.contains('columns-mode') && this.columnsViewController) {
          void this.columnsViewController.goBack(side);
        } else {
          void this.pathHeaderController.goParent(side);
        }
        break;
      default:
        if (e.key && e.key.length === 1 && e.key !== ' ' && !e.ctrlKey && !e.altKey && !e.metaKey && (document.activeElement as HTMLElement)?.tagName !== 'INPUT' && (document.activeElement as HTMLElement)?.tagName !== 'TEXTAREA') {
          if (!state.config.vimNavigation || !['j', 'k', 'h', 'l'].includes(e.key)) {
            e.preventDefault();
            this._handleTypeToJump(side, e.key);
          }
        }
        break;
    }
  }

  private _handleTypeToJump(side: 'left' | 'right', char: string): void {
    const lowerChar = char.toLowerCase();
    clearTimeout(this._typeSearchTimer);
    this._typeSearchTimer = setTimeout(() => {
      this._typeSearchBuf = '';
      this._typeSearchLastChar = '';
    }, 850);

    const pane = this.state[side];
    const vis = this.listRenderer
      ? this.listRenderer.filteredItems(pane)
      : (pane.items || []);

    if (lowerChar === this._typeSearchLastChar && this._typeSearchBuf.length <= 1) {
      this._typeSearchBuf = lowerChar;
      const currentIdx = pane.cursor;
      for (let i = 1; i <= vis.length; i++) {
        const idx = (currentIdx + i) % vis.length;
        const it = vis[idx];
        if (it.base !== '..' && it.base.toLowerCase().startsWith(lowerChar)) {
          pane.cursor = idx;
          this.renderPane(side);
          this.setStatus(`Quick Search: "${lowerChar}" (${idx + 1})`);
          return;
        }
      }
      this.setStatus(`Quick Search: "${lowerChar}" (no more)`);
      return;
    }

    this._typeSearchBuf += lowerChar;
    this._typeSearchLastChar = lowerChar;
    const query = this._typeSearchBuf;

    let foundIdx = vis.findIndex((it: any) => it.base !== '..' && it.base.toLowerCase().startsWith(query));
    if (foundIdx === -1) {
      foundIdx = vis.findIndex((it: any) => it.base !== '..' && it.base.toLowerCase().includes(query));
    }

    if (foundIdx !== -1) {
      pane.cursor = foundIdx;
      this.renderPane(side);
      this.setStatus(`Quick Search: "${this._typeSearchBuf}"`);
    } else {
      this.setStatus(`Quick Search: "${this._typeSearchBuf}" (no match)`);
    }
  }
}
