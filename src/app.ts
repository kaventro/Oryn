// src/app.ts — orchestrator. Instantiates and wires all controllers.
import { AppState } from './modules/stateModels.ts';
import { PathHeaderController } from './modules/pathHeaderController.ts';
import { ListRenderer } from './modules/listRenderer.ts';
import { TabsRenderer } from './modules/tabsRenderer.ts';
import { QuickViewController } from './modules/quickViewController.ts';
import { SearchController } from './modules/searchController.ts';
import { CommandsController } from './modules/commandsController.ts';
import { PanelController } from './modules/panelController.ts';
import { FilterDropController } from './modules/filterDropController.ts';
import { PropertiesController } from './modules/propertiesController.ts';
import { SysStatsController } from './modules/sysStatsController.ts';
import { FileOpsController } from './modules/fileOpsController.ts';
import { CtxMenuController } from './modules/ctxMenuController.ts';
import { FileDiffController } from './modules/fileDiffController.ts';
import { CompareDirsController } from './modules/compareDirsController.ts';
import { GitController } from './modules/gitController.ts';
import { MultiRenameController } from './modules/multiRenameController.ts';
import { TerminalDrawerController } from './modules/terminalDrawerController.ts';
import { FolderSyncController } from './modules/folderSyncController.ts';
import { ChecksumController } from './modules/checksumController.ts';
import { TagController } from './modules/tagController.ts';
import { FindReplaceController } from './modules/findReplaceController.ts';
import { DrivePopupController } from './modules/drivePopupController.ts';
import { KeyboardController } from './modules/keyboardController.ts';
import { CommandPaletteController } from './modules/commandPaletteController.ts';
import { DiskSpaceController } from './modules/diskSpaceController.ts';
import { DuplicateFinderController } from './modules/duplicateFinderController.ts';
import { StatusBarController } from './modules/statusBarController.ts';
import { SelectionController } from './modules/selectionController.ts';
import { SidebarController } from './modules/sidebarController.ts';
import { ViewController } from './modules/viewController.ts';
import { ColumnsViewController } from './modules/columns/columnsViewController.ts';
import { RemoteController } from './modules/remoteController.ts';
import { RemoteDialog } from './modules/remoteDialog.ts';
import { UpdaterController } from './modules/updaterController.ts';
import {
  PreferencesController,
  applyTrayThemeFromStorage,
  readPaneMode,
  savePaneMode,
  readDrivePaneDefaults,
} from './modules/preferencesController.ts';
import {
  filteredItems, fmtSize, rowDateText, shortPath,
} from './modules/formatUtils.ts';

declare const __APP_VERSION__: string;

function api(): any { return (window as any).ow; }

const state = new AppState();
const statusBarController = new StatusBarController({
  state,
  api,
  filteredItems,
  fmtSize,
  shortPath,
});

function otherSide(side: 'left' | 'right'): 'left' | 'right' { return side === 'left' ? 'right' : 'left'; }

/**
 * Normalize a path for comparison: lowercase, `\`→`/`, drop a trailing slash
 * (but keep a lone root "/"). Works for Windows drive letters and POSIX mounts
 * alike — `C:\` → `c:`, `/Volumes/Data/` → `/volumes/data`, `/` → `/`.
 */
function normDrive(p?: string | null): string {
  const s = String(p || '').toLowerCase().replace(/\\/g, '/');
  return s.length > 1 ? s.replace(/\/+$/, '') : s;
}

/** Return `wanted` if it matches an available mount, otherwise `fallback`. */
function pickDrive(mounts: string[], wanted?: string, fallback: string = ''): string {
  if (!wanted) return fallback;
  const w = normDrive(wanted);
  const hit = mounts.find((m) => normDrive(m) === w);
  return hit || fallback;
}

/** The mount whose path is the longest prefix of `home` (the volume home lives on). */
function volumeContaining(mounts: string[], home: string): string {
  const h = normDrive(home);
  if (!h) return '';
  let best = '';
  let bestLen = -1;
  mounts.forEach((m) => {
    const nm = normDrive(m);
    if (nm && h.startsWith(nm) && nm.length > bestLen) {
      best = m;
      bestLen = nm.length;
    }
  });
  return best;
}

/**
 * Platform-neutral per-side defaults for Dual Pane. There are no drive letters
 * on macOS/Linux, so rather than assume "C:"/"D:" we open the left panel on the
 * volume that holds the home directory (typically C:\ on Windows, / elsewhere)
 * and the right panel on the next distinct volume in the list.
 */
function autoSideDrives(mounts: string[], home: string): { left: string; right: string } {
  const left = volumeContaining(mounts, home) || mounts[0] || home;
  const right = mounts.find((m) => normDrive(m) !== normDrive(left)) || mounts[1] || home;
  return { left, right };
}

/**
 * Decide each panel's startup directory. In Dual Pane, when more than one volume
 * is present and "Open Drives by Side" is on, each panel opens on its own volume:
 * the saved choice when set, otherwise the platform-neutral automatic default.
 * Anything missing or unavailable falls back to the OS home directory.
 */
async function resolveStartupPaths(home: string): Promise<{ left: string; right: string }> {
  let left = home;
  let right = home;
  try {
    const prefs = readDrivePaneDefaults();
    if (readPaneMode() === 'dual' && prefs.enabled) {
      const locs = await api().getSystemLocations();
      const mounts: string[] = (locs?.drives || []).map((d: any) => d.mountPoint).filter(Boolean);
      if (mounts.length > 1) {
        const auto = autoSideDrives(mounts, locs?.home || home);
        left = pickDrive(mounts, prefs.left, auto.left);
        right = pickDrive(mounts, prefs.right, auto.right);
      }
    }
  } catch { }
  return { left, right };
}

function setStatus(t?: string): void {
  statusBarController.setMessage(t || '');
  const term = document.getElementById('nx-terminal-output');
  if (term) term.textContent = (t || 'ready');
}

function applyPaneMode(mode: string): void {
  const appEl = document.getElementById('app');
  const paneToggleBtn = document.getElementById('btn-pane-toggle');
  const isDual = mode === 'dual';
  if (isDual) {
    appEl?.classList.remove('single-pane-mode');
    appEl?.classList.add('dual-pane-mode');
    paneToggleBtn?.classList.add('active');
  } else {
    appEl?.classList.remove('dual-pane-mode');
    appEl?.classList.add('single-pane-mode');
    paneToggleBtn?.classList.remove('active');
    state.active = 'left';
  }
  savePaneMode(mode as 'single' | 'dual');
  paintVirtualPane('left', false);
  paintVirtualPane('right', false);
  updatePaneClass();
  focusActiveList();
}

function updatePaneClass(): void {
  const isLeft = state.active === 'left';
  document.getElementById('pane-left')?.classList.toggle('active', isLeft);
  document.getElementById('pane-right')?.classList.toggle('active', !isLeft);

  const tabsLeft = document.getElementById('tabs-left');
  const tabsRight = document.getElementById('tabs-right');
  if (tabsLeft) tabsLeft.style.display = isLeft ? 'flex' : 'none';
  if (tabsRight) tabsRight.style.display = !isLeft ? 'flex' : 'none';

  const titleLeft = document.getElementById('title-left');
  const titleRight = document.getElementById('title-right');
  if (titleLeft) titleLeft.style.display = isLeft ? 'flex' : 'none';
  if (titleRight) titleRight.style.display = !isLeft ? 'flex' : 'none';

  pathHeaderControllerInst?.renderTitle('left', state.left?.path);
  pathHeaderControllerInst?.renderTitle('right', state.right?.path);

  const activePane = state[state.active];
  if (activePane && activePane.path) {
    const name = activePane.path.split(/[/\\]/).filter(Boolean).pop() || 'Oryn';
    const folderTitle = document.getElementById('folder-title');
    if (folderTitle) folderTitle.textContent = name;
    const fi = document.getElementById('filter-input') as HTMLInputElement | null;
    if (fi && !fi.value) fi.placeholder = `Search ${name}…`;
  }

  sidebarControllerInst?.updateActiveHighlight();
  statusBarController.refresh();
}

function focusActiveList(): void {
  (document.getElementById(state.active === 'left' ? 'list-left' : 'list-right') as HTMLElement | null)?.focus({ preventScroll: true });
}

function syncFilterInput(): void {
  const fi = document.getElementById('filter-input') as HTMLInputElement | null;
  if (fi) fi.value = state[state.active].filter;
}

function focusFilterInput(): void {
  (document.getElementById('filter-input') as HTMLInputElement | null)?.focus();
}

// Controller refs — populated in init() before any callbacks are invoked.
let pathHeaderControllerInst: PathHeaderController;
let listRendererInst: ListRenderer;
let quickViewControllerInst: QuickViewController;
let panelControllerInst: PanelController;
let filterDropControllerInst: FilterDropController;
let selectionControllerInst: SelectionController;
let sidebarControllerInst: SidebarController;

function renderPane(side: 'left' | 'right'): void {
  pathHeaderControllerInst?.renderTitle(side, state[side]?.path);
  quickViewControllerInst.render(side).then((handled) => {
    if (!handled) {
      if (document.getElementById('app')?.classList.contains('columns-mode') && listRendererInst?.columnsViewController) {
        void listRendererInst.columnsViewController.syncPane(side, state[side]);
      } else {
        listRendererInst.paintVirtualPane(side, false);
      }
    }
    statusBarController.refresh();
    selectionControllerInst?.updateIndicator(side);
  });
}

function paintVirtualPane(side: 'left' | 'right', align?: boolean): void {
  if (document.getElementById('app')?.classList.contains('columns-mode') && listRendererInst?.columnsViewController) {
    listRendererInst.columnsViewController.render(side);
  } else {
    listRendererInst.paintVirtualPane(side, align);
  }
}

function moveCursor(side: 'left' | 'right', delta: number): void {
  listRendererInst.moveCursor(side, delta);
  const opp = otherSide(side);
  if (state[opp].quickViewActive) quickViewControllerInst.render(opp);
  statusBarController.refresh();
}

async function init(): Promise<void> {
  const ow = api();
  if (!ow || typeof ow.readDir !== 'function') {
    setStatus('Bridge not ready (window.ow). Restart the app.');
    return;
  }

  applyTrayThemeFromStorage();

  // Core controllers
  const tabsRenderer = new TabsRenderer({
    state,
    loadDir: (s) => panelControllerInst.loadDir(s),
    updatePaneClass,
    focusActiveList,
    renderPane,
    syncFilterInput,
  });

  pathHeaderControllerInst = new PathHeaderController({
    state,
    api,
    setStatus,
    loadDir: (s) => panelControllerInst.loadDir(s),
    syncFilterInput,
    updatePaneClass,
    focusActiveList,
    openGit: () => gitController.open(),
  });
  const pathHeaderController = pathHeaderControllerInst;

  quickViewControllerInst = new QuickViewController({
    state,
    api,
    otherSide,
    getFilteredSelection: (side) => panelControllerInst.getFilteredSelection(side),
    fullPath: (pane, item) => panelControllerInst.fullPath(pane, item),
  });

  const tagController = new TagController({
    api,
    state,
    setStatus,
    renderPane,
  });

  const propertiesController = new PropertiesController({
    api,
    setStatus,
    focusActiveList,
    getFilteredSelection: (side) => panelControllerInst.getFilteredSelection(side),
    fullPath: (side, item) => panelControllerInst.fullPath(state[side], item),
    tagController,
  });

  const fileOps = new FileOpsController({
    state,
    api,
    setStatus,
    otherSide,
    getFilteredSelection: (side) => panelControllerInst.getFilteredSelection(side),
    fullPath: (side, item) => panelControllerInst.fullPath(state[side], item),
    loadDir: (s) => panelControllerInst.loadDir(s),
    refreshAll: (opts) => panelControllerInst.refreshAll(opts),
    focusActiveList,
  });

  const remoteController = new RemoteController({
    api: () => api(),
    setStatus,
  });

  const remoteDialog = new RemoteDialog({
    remoteController,
    navigateTo: (p) => panelControllerInst.navigateTo(state.active, p),
    state,
    setStatus,
  });

  sidebarControllerInst = new SidebarController({
    api,
    state,
    setStatus,
    navigateTo: (side, p) => panelControllerInst.navigateTo(side, p),
    focusActiveList,
    remoteController,
    openRemoteDialog: () => remoteDialog.open(),
  });

  const ctxMenuController = new CtxMenuController({
    api,
    state,
    setStatus,
    getFilteredSelection: (side) => panelControllerInst.getFilteredSelection(side),
    fullPath: (side, item) => panelControllerInst.fullPath(state[side], item),
    openSelected: (side) => panelControllerInst.openSelected(side),
    copyToOther: () => fileOps.copyToOther(),
    moveToOther: () => fileOps.moveToOther(),
    beginRename: (opts) => fileOps.beginRename(opts),
    beginDelete: (opts) => fileOps.beginDelete(opts),
    loadDir: (s) => panelControllerInst.loadDir(s),
    openGitBlame: (f, r) => gitController.openBlameForFile(f, null, r),
    openGitDiff: (f) => gitController.openDiffForFile(f),
    openGitLog: (f) => gitController.openLogForFile(f),
    openMultiRename: () => multiRenameController.open(),
    openChecksum: (fp) => checksumController.openForPath(fp),
    tagController,
    sidebarController: sidebarControllerInst,
    runCommand: (c, payload) => commandsController.runCommand(c, payload),
  });
  ctxMenuController.showProperties = (side, fp) => propertiesController.showFor(side, fp);

  listRendererInst = new ListRenderer({
    state,
    filteredItems,
    pathHeaderController,
    updatePaneClass,
    focusActiveList,
    openSelected: (side) => panelControllerInst.openSelected(side),
    showCtxMenu: (x, y, side) => ctxMenuController.show(x, y, side),
    fmtSize,
    rowDateText,
    onDrop: (sourceSide, targetSide, data, isCopy) => {
      if (sourceSide === targetSide) return;
      state.active = sourceSide;
      const pane = state[sourceSide];
      const items = Array.isArray(data?.items) ? data.items : (data?.base ? [data.base] : []);
      if (items.length > 0) {
        pane.activeTab.clearSelection();
        items.forEach((baseName: string) => pane.activeTab.select(baseName));
        selectionControllerInst?.updateIndicator(sourceSide);
      }
      if (isCopy) void fileOps.copyToOther();
      else void fileOps.moveToOther();
    }
  });

  const searchController = new SearchController({
    state,
    api,
    focusActiveList,
    openSearchHit: (fp) => panelControllerInst.openSearchHit(fp),
  });

  filterDropControllerInst = new FilterDropController({
    state,
    renderPane,
    focusActiveList,
  });

  panelControllerInst = new PanelController({
    state,
    api,
    setStatus,
    renderPane,
    updatePaneClass,
    focusActiveList,
    tabsRenderer,
    syncFilterInput,
    hideSearchOverlay: () => searchController.hideOverlay(),
    hideFilterDrop: () => filterDropControllerInst.hide(),
    tagController,
  });

  // Feature controllers
  const sysStatsController = new SysStatsController({ api, fmtSize });

  const fileDiffController = new FileDiffController({
    api,
    state,
    setStatus,
    otherSide,
    focusActiveList,
  });

  const compareDirsController = new CompareDirsController({
    api,
    state,
    setStatus,
    openFileDiff: (l, r) => fileDiffController.openWith(l, r),
    focusActiveList,
  });

  const gitController = new GitController({
    api,
    state,
    setStatus,
    loadDir: (s) => panelControllerInst.loadDir(s),
    focusActiveList,
  });

  const multiRenameController = new MultiRenameController({
    state,
    api,
    setStatus,
    loadDir: (s) => panelControllerInst.loadDir(s),
    getFilteredSelection: (side) => panelControllerInst.getFilteredSelection(side),
    fullPath: (pane, item) => panelControllerInst.fullPath(pane, item),
    focusActiveList,
  });

  const terminalDrawerController = new TerminalDrawerController({
    state,
    api,
    setStatus,
    focusActiveList,
    loadDir: (s) => panelControllerInst.loadDir(s),
    navigateTo: (s, p) => panelControllerInst.navigateTo(s as any, p),
  });
  ctxMenuController.openTerminal = (p) => terminalDrawerController.show(p);

  const folderSyncController = new FolderSyncController({
    state,
    api,
    setStatus,
    loadDir: (s) => panelControllerInst.loadDir(s),
    focusActiveList,
  });

  const checksumController = new ChecksumController({
    api,
    state,
    setStatus,
    focusActiveList,
  });

  const findReplaceController = new FindReplaceController({
    api,
    setStatus,
    getSearchRoot: () => searchController.getCurrentRoot(),
  });

  const applySettingsToUI = (settings: any) => {
    if (!settings) return;
    const statusTermBtn = document.getElementById('btn-status-terminal-toggle');
    if (statusTermBtn) {
      statusTermBtn.style.display = settings.showStatusBarTerminal !== false ? 'inline-flex' : 'none';
    }
    const isSftpEnabled = Boolean(settings.enableSftp);
    const remoteSection = document.getElementById('sidebar-remote-section');
    if (remoteSection) {
      remoteSection.style.display = isSftpEnabled ? 'block' : 'none';
    }
    const remoteToggleBtn = document.getElementById('btn-remote-toggle');
    if (remoteToggleBtn) {
      remoteToggleBtn.style.display = isSftpEnabled ? 'inline-flex' : 'none';
    }
  };

  const preferencesController = new PreferencesController({
    focusActiveList,
    getDrives: () => api().getSystemLocations(),
    onPaneModeChange: (mode) => applyPaneMode(mode),
    onSettingsChange: (settings) => {
      applySettingsToUI(settings);
      panelControllerInst.refreshAll();
    },
  });
  preferencesController.setup();
  applySettingsToUI(preferencesController.settings);

  selectionControllerInst = new SelectionController({
    state,
    renderPane,
    setStatus,
    focusActiveList,
    getFilteredSelection: (side) => panelControllerInst.getFilteredSelection(side),
  });

  const commandsController = new CommandsController({
    selectionController: selectionControllerInst,
    api,
    state,
    otherSide,
    fileOps,
    syncFilterInput,
    updatePaneClass,
    focusActiveList,
    refreshAll: () => panelControllerInst.refreshAll(),
    copyPathOnly: () => fileOps.copyPathOnly(),
    copyToOther: () => fileOps.copyToOther(),
    moveToOther: () => fileOps.moveToOther(),
    beginRename: () => fileOps.beginRename(),
    beginDelete: () => fileOps.beginDelete(),
    focusFilterInput,
    openSearchOverlay: () => searchController.openOverlay(),
    browseFolderPicker: () => panelControllerInst.browseFolderPicker(),
    openGitOverlay: () => gitController.open(),
    openCompareOverlay: () => compareDirsController.open(),
    openPreferences: () => preferencesController.open(),
    openRemoteDialog: () => remoteDialog.open(),
    openMultiRename: () => multiRenameController.open(),
    openFolderSync: () => folderSyncController.open(),
    toggleTerminal: () => terminalDrawerController.toggle(),
    getFilteredSelection: (side: 'left' | 'right') => panelControllerInst.getFilteredSelection(side),
    fullPath: (pane, item) => panelControllerInst.fullPath(pane, item),
    openSelected: (side: 'left' | 'right') => panelControllerInst.openSelected(side),
    loadDir: (s: 'left' | 'right') => panelControllerInst.loadDir(s),
    setStatus,
    hideCtxMenu: () => ctxMenuController.hide(),
    sidebarController: sidebarControllerInst,
  });

  const drivePopupController = new DrivePopupController({
    state,
    api,
    loadDir: (s) => panelControllerInst.loadDir(s),
    focusActiveList,
    setStatus,
  });

  const columnsViewController = new ColumnsViewController({
    api,
    rowRenderer: listRendererInst.rowRenderer,
    iconRegistry: listRendererInst.iconRegistry,
    showCtxMenu: (x, y, side, emptyArea, item, dirPath) => ctxMenuController.show(x, y, side, emptyArea, item, dirPath),
    onOpenSelected: (path, isDir) => {
      if (isDir) {
        void panelControllerInst.navigateTo(state.active, path);
      } else {
        void ow.openPath(path);
      }
    },
    onPreviewSelected: (fp, item) => {
      void commandsController.openViewer(fp, item);
    },
    onActivateSide: (side, activePath) => {
      state.active = side;
      if (activePath) {
        state[side].path = activePath;
        void panelControllerInst.refreshGitMeta(side, { annotateItems: false }).then(() => {
          pathHeaderControllerInst.renderTitle(side, state[side]?.path);
          statusBarController.refresh();
        });
      }
      syncFilterInput();
      updatePaneClass();
      focusActiveList();
    },
    setStatus,
  });
  listRendererInst.columnsViewController = columnsViewController;
  commandsController.columnsViewController = columnsViewController;
  fileOps.columnsViewController = columnsViewController;
  panelControllerInst.columnsViewController = columnsViewController;

  const viewController = new ViewController({
    state,
    onModeChange: (mode) => {
      if (mode === 'columns') {
        void columnsViewController.loadRoot('left', state.left);
        void columnsViewController.loadRoot('right', state.right);
      } else {
        document.querySelectorAll('.columns-container').forEach((el) => el.remove());
        listRendererInst.paintVirtualPane('left', true);
        listRendererInst.paintVirtualPane('right', true);
      }
    },
  });
  ctxMenuController.viewController = viewController;
  ctxMenuController.commandsController = commandsController;
  ctxMenuController.onPreviewSelected = (fp, item) => commandsController.openViewer(fp, item);

  const diskSpaceController = new DiskSpaceController({
    api,
    state,
    setStatus,
    navigateTo: (side, p) => panelControllerInst.navigateTo(side, p),
    focusActiveList,
    fileOps,
  });

  const duplicateFinderController = new DuplicateFinderController({
    api,
    state,
    setStatus,
    navigateTo: (side, p) => panelControllerInst.navigateTo(side, p),
    focusActiveList,
    openViewer: (fp, item) => commandsController.openViewer(fp, item),
  });

  const commandPaletteController = new CommandPaletteController({
    api,
    state,
    setStatus,
    commandsController,
    viewController,
    fileOps,
    gitController,
    searchController,
    compareDirsController,
    terminalDrawerController,
    quickViewController: quickViewControllerInst,
    propertiesController,
    multiRenameController,
    checksumController,
    preferencesController,
    diskSpaceController,
    duplicateFinderController,
    panelController: panelControllerInst,
    navigateTo: (side, p) => panelControllerInst.navigateTo(side, p),
    openSelected: (side) => panelControllerInst.openSelected(side),
    refreshAll: () => panelControllerInst.refreshAll(),
  });

  ctxMenuController.diskSpaceController = diskSpaceController;
  ctxMenuController.duplicateFinderController = duplicateFinderController;

  const keyboardController = new KeyboardController({
    state,
    pathHeaderController,
    tabsRenderer,
    quickViewController: quickViewControllerInst,
    commandsController,
    searchController,
    propertiesController,
    gitController,
    compareDirsController,
    ctxMenuController,
    multiRenameController,
    terminalDrawerController,
    folderSyncController,
    checksumController,
    preferencesController,
    commandPaletteController,
    diskSpaceController,
    duplicateFinderController,
    drivePopupController,
    viewController,
    columnsViewController,
    focusActiveList,
    syncFilterInput,
    updatePaneClass,
    otherSide,
    refreshAll: () => panelControllerInst.refreshAll(),
    browseFolderPicker: () => panelControllerInst.browseFolderPicker(),
    focusFilterInput,
    fileOps,
    panelController: panelControllerInst,
    listRenderer: listRendererInst,
    getFilteredSelection: (side) => panelControllerInst.getFilteredSelection(side),
    paintVirtualPane,
    renderPane,
    moveCursor,
    setStatus,
    selectionController: selectionControllerInst,
  });

  // Boot
  state.config = await ow.loadConfig();
  const home = await ow.getHome();
  const { left: leftStart, right: rightStart } = await resolveStartupPaths(home);
  state.left.path = leftStart;
  state.right.path = rightStart;
  const fInput = document.getElementById('filter-input') as HTMLInputElement | null;
  if (fInput) fInput.value = '';
  state.left.filter = '';
  state.right.filter = '';
  await panelControllerInst.loadDir('left');
  await panelControllerInst.loadDir('right');

  // Apply saved or default (dual) pane mode
  applyPaneMode(readPaneMode());

  // Setup Sidebar & UI Components
  await sidebarControllerInst.setup();

  // Setup Tags Controller
  tagController.setup();

  // Settings & Preferences Button
  document.getElementById('btn-settings-toggle')?.addEventListener('click', () => {
    preferencesController.open();
  });

  // Terminal Toggle Buttons (Header & Status Bar)
  document.getElementById('btn-terminal-toggle')?.addEventListener('click', () => {
    terminalDrawerController.toggle();
  });
  document.getElementById('btn-status-terminal-toggle')?.addEventListener('click', () => {
    terminalDrawerController.toggle();
  });

  // Sidebar Toggle Helper
  const sidebarEl = document.getElementById('sidebar');
  const appEl = document.getElementById('app');
  function toggleSidebar(forceState?: boolean): void {
    const willCollapse = typeof forceState === 'boolean'
      ? forceState
      : !sidebarEl?.classList.contains('collapsed');

    sidebarEl?.classList.toggle('collapsed', willCollapse);
    appEl?.classList.toggle('sidebar-collapsed', willCollapse);
    try {
      localStorage.setItem('Oryn.sidebarCollapsed', willCollapse ? '1' : '0');
      localStorage.setItem('Oswin.sidebarCollapsed', willCollapse ? '1' : '0');
    } catch (_) { }

    setTimeout(() => {
      paintVirtualPane('left', false);
      paintVirtualPane('right', false);
    }, 220);
  }

  // Restore sidebar state
  try {
    if (localStorage.getItem('Oryn.sidebarCollapsed') === '1' || localStorage.getItem('Oswin.sidebarCollapsed') === '1' || localStorage.getItem('totalshark.sidebarCollapsed') === '1') {
      toggleSidebar(true);
    }
  } catch (_) { }

  document.getElementById('sidebar-toggle')?.addEventListener('click', () => toggleSidebar());
  document.getElementById('sidebar-toggle-header')?.addEventListener('click', () => toggleSidebar());

  // Global shortcut Cmd+B / Ctrl+B or Cmd+\ / Ctrl+\ to toggle sidebar
  window.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && (e.key.toLowerCase() === 'b' || e.key === '\\')) {
      const activeEl = document.activeElement;
      if (activeEl && (activeEl.tagName === 'INPUT' || activeEl.tagName === 'TEXTAREA')) return;
      e.preventDefault();
      toggleSidebar();
    }
  });

  // Version from package.json
  const versionEl = document.getElementById('pref-app-version');
  if (versionEl && typeof __APP_VERSION__ === 'string') {
    versionEl.textContent = `Oryn ${__APP_VERSION__} • macOS & Windows`;
  }

  // Detect OS platform
  const isMac = navigator.userAgent.includes('Macintosh') || navigator.platform.includes('Mac');
  if (isMac) {
    document.body.classList.add('platform-mac');
  } else {
    document.body.classList.add('platform-windows');
  }

  // Window Caption Controls (Windows / Linux)
  document.getElementById('win-btn-min')?.addEventListener('click', () => {
    void ow.minimizeWindow();
  });
  document.getElementById('win-btn-max')?.addEventListener('click', () => {
    void ow.toggleMaximizeWindow();
  });
  document.getElementById('win-btn-close')?.addEventListener('click', () => {
    void ow.closeWindow();
  });

  // Pane Layout Toggle (Single vs Dual Pane)
  const paneToggleBtn = document.getElementById('btn-pane-toggle');
  paneToggleBtn?.addEventListener('click', () => {
    const isCurrentlyDual = appEl?.classList.contains('dual-pane-mode');
    applyPaneMode(isCurrentlyDual ? 'single' : 'dual');
  });

  // Add Tab Button
  document.getElementById('btn-add-tab')?.addEventListener('click', () => {
    const pane = state[state.active];
    pane.addTab();
    tabsRenderer.render(state.active);
    panelControllerInst.loadDir(state.active).then(() => focusActiveList());
  });

  // Navigation Toolbar Buttons
  document.getElementById('btn-nav-up')?.addEventListener('click', () => {
    const isColumns = document.getElementById('app')?.classList.contains('columns-mode');
    if (isColumns && columnsViewController) {
      void columnsViewController.goBack(state.active);
    } else {
      void pathHeaderController.goParent(state.active);
    }
  });
  document.getElementById('btn-nav-back')?.addEventListener('click', () => {
    const isColumns = document.getElementById('app')?.classList.contains('columns-mode');
    if (isColumns && columnsViewController) {
      void columnsViewController.goBack(state.active);
    } else {
      const pane = state[state.active];
      if (pane && typeof pane.historyBack === 'function' && pane.canHistoryBack?.()) {
        pane.historyBack();
        void panelControllerInst.loadDir(state.active);
      } else {
        void pathHeaderController.goParent(state.active);
      }
    }
  });
  document.getElementById('btn-nav-forward')?.addEventListener('click', () => {
    const isColumns = document.getElementById('app')?.classList.contains('columns-mode');
    if (isColumns && columnsViewController) {
      void columnsViewController.goForward(state.active);
    } else {
      const pane = state[state.active];
      if (pane && typeof pane.historyForward === 'function' && pane.canHistoryForward?.()) {
        pane.historyForward();
        void panelControllerInst.loadDir(state.active);
      }
    }
  });

  // More Menu Button
  document.getElementById('btn-more-menu')?.addEventListener('click', (e) => {
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    ctxMenuController.showMoreMenu(rect.left, rect.bottom + 4, state.active);
  });

  // Setup all UI
  viewController.setupUI();
  searchController.setupUI();
  pathHeaderController.setup();
  tabsRenderer.setup();
  sysStatsController.setup();
  fileDiffController.setup();
  compareDirsController.setup();
  gitController.setup();
  multiRenameController.setup();
  terminalDrawerController.setup();
  folderSyncController.setup();
  checksumController.setup();
  findReplaceController.patchSearchButton();
  commandsController.setup();
  preferencesController.setup();
  keyboardController.setup();

  // Clock
  sysStatsController.tickClock();
  setInterval(() => sysStatsController.tickClock(), 1000);

  // Prevent default drag and drop behavior on window to avoid accidentally loading files
  document.addEventListener('dragover', (e) => e.preventDefault());
  document.addEventListener('drop', (e) => e.preventDefault());

  // Prevent contextmenu defaults on empty surfaces
  document.addEventListener('contextmenu', (e) => {
    if ((e.target as HTMLElement)?.closest?.('input, textarea, [contenteditable="true"]')) return;
    e.preventDefault();
  });

  // Resize
  let resizeTimer = 0;
  window.addEventListener('resize', () => {
    window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => {
      paintVirtualPane('left', false);
      paintVirtualPane('right', false);
    }, 120);
  });

  // Filter input
  const fi = document.getElementById('filter-input') as HTMLInputElement | null;
  if (fi) {
    fi.addEventListener('input', () => {
      state[state.active].filter = fi.value;
      state[state.active].cursor = 0;
      renderPane(state.active);
      filterDropControllerInst.render();
    });
    fi.addEventListener('focus', () => {
      if (fi.value.trim()) filterDropControllerInst.render();
    });
    fi.addEventListener('blur', () => {
      setTimeout(() => filterDropControllerInst.hide(), 150);
    });
    fi.addEventListener('keydown', (e) => {
      const el = document.getElementById('filter-dropdown');
      const dropVisible = el && !el.classList.contains('hidden');
      if (e.key === 'ArrowDown') {
        if (dropVisible) { e.preventDefault(); e.stopPropagation(); filterDropControllerInst.moveSelection(1); return; }
        if (fi.value.trim()) {
          e.preventDefault(); e.stopPropagation();
          filterDropControllerInst.render();
          filterDropControllerInst.moveSelection(1);
          return;
        }
      }
      if (e.key === 'ArrowUp' && dropVisible) {
        e.preventDefault(); e.stopPropagation();
        filterDropControllerInst.moveSelection(-1);
        return;
      }
      if (e.key === 'Enter' && dropVisible && filterDropControllerInst._idx >= 0) {
        e.preventDefault(); e.stopPropagation();
        const rows = [...(el?.querySelectorAll('.fdrop-item') ?? [])] as HTMLElement[];
        const base = rows[filterDropControllerInst._idx]?.dataset.base;
        if (base) filterDropControllerInst.applySelection(base);
        return;
      }
      if (e.key === 'Escape' && dropVisible) {
        e.preventDefault(); e.stopPropagation();
        filterDropControllerInst.hide();
        return;
      }
    });
  }

  // Pane activation & focus tracking
  const activateSide = (side: 'left' | 'right') => {
    if (state.active !== side) {
      state.active = side;
      syncFilterInput();
      updatePaneClass();
      paintVirtualPane('left', false);
      paintVirtualPane('right', false);
    }
  };

  document.getElementById('pane-left')?.addEventListener('click', () => {
    activateSide('left');
    focusActiveList();
  });
  document.getElementById('pane-right')?.addEventListener('click', () => {
    activateSide('right');
    focusActiveList();
  });
  document.getElementById('pane-left')?.addEventListener('focusin', () => {
    activateSide('left');
  });
  document.getElementById('pane-right')?.addEventListener('focusin', () => {
    activateSide('right');
  });

  // Filesystem live changes auto-refresh
  let fsDebounceTimer: any = null;
  if (typeof ow.onFsChange === 'function') {
    ow.onFsChange((payload: any) => {
      clearTimeout(fsDebounceTimer);
      fsDebounceTimer = setTimeout(() => {
        const changed = (payload?.path || '').replace(/[/\\]+$/, '');
        const leftP = (state.left?.path || '').replace(/[/\\]+$/, '');
        const rightP = (state.right?.path || '').replace(/[/\\]+$/, '');
        if (leftP && (leftP === changed || !changed)) {
          void panelControllerInst.loadDir('left', { preserveCursor: true });
        }
        if (rightP && (rightP === changed || !changed)) {
          void panelControllerInst.loadDir('right', { preserveCursor: true });
        }
      }, 120);
    });
  }

  // Auto-refresh when window regains focus
  window.addEventListener('focus', () => {
    if (!state.copyInProgress) {
      if (state.left?.path && !state.left.path.startsWith('sftp://')) {
        void panelControllerInst.loadDir('left', { preserveCursor: true });
      }
      if (state.right?.path && !state.right.path.startsWith('sftp://')) {
        void panelControllerInst.loadDir('right', { preserveCursor: true });
      }
    }
  });

  // Column sorting
  (['left', 'right'] as const).forEach(side => {
    const head = document.querySelector(`#pane-${side} .pane-col-head`);
    if (head) {
      head.addEventListener('click', (e) => {
        const span = (e.target as HTMLElement).closest('[data-sort]') as HTMLElement | null;
        if (!span) return;
        const field = span.dataset.sort as any;
        const pane = state[side];
        if (pane.sortField === field) {
          pane.sortAsc = !pane.sortAsc;
        } else {
          pane.sortField = field;
          pane.sortAsc = true;
        }
        state.active = side;
        updatePaneClass();
        renderPane(side);
        focusActiveList();
      });
    }
  });

  // Command line
  const cmdInput = document.getElementById('nx-cmd-input') as HTMLInputElement | null;
  if (cmdInput) {
    cmdInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        const cmd = cmdInput.value.trim();
        if (!cmd) return;
        const cwd = state[state.active].path || undefined;
        cmdInput.value = '';

        // cd navigates the active panel instead of running in subprocess
        const cdMatch = cmd.match(/^cd\s+(.+)$/);
        if (cdMatch) {
          const target = cdMatch[1].trim().replace(/^['"]|['"]$/g, '');
          const resolved = target.startsWith('/')
            ? target
            : (cwd ? cwd + '/' + target : target);
          void panelControllerInst.navigateTo(state.active, resolved);
          focusActiveList();
          return;
        }

        setStatus(`Running: ${cmd}`);
        ow.shellExec(cmd, cwd).then((r: any) => {
          const out = (r.stdout || '').trim() || (r.stderr || '').trim();
          const short = out.length > 200 ? out.slice(0, 197) + '…' : out;
          if (r.ok) setStatus(short || `Done (exit 0)`);
          else setStatus(`[exit ${r.code}] ${short || r.stderr || 'failed'}`);
          panelControllerInst.refreshAll();
        }).catch((err: any) => setStatus(err?.message || String(err)));
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        cmdInput.value = '';
        focusActiveList();
      }
    });
  }

  // Menu callbacks
  ow.onMenuRefresh(() => panelControllerInst.refreshAll());
  ow.onMenuCopyPath(() => fileOps.copyPathOnly());
  document.getElementById('browse-folder-btn')?.addEventListener('click', () => searchController.openOverlay());
  document.getElementById('btn-remote-toggle')?.addEventListener('click', () => remoteDialog.open());

  // Load saved remote profiles on launch for sidebar
  remoteController.loadProfiles().then(() => {
    sidebarControllerInst?.render();
  }).catch(() => { });
}

// Background updates run outside init() so a failed startup can still self-heal.
const updater = new UpdaterController();
updater.attach();
setTimeout(() => { void updater.run(); }, 5000);

init().catch((err) => {
  console.error('[init] failed:', err);
  const msg = err?.message || String(err);
  setStatus(msg.length > 180 ? `${msg.slice(0, 177)}…` : msg);
});
