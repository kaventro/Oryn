// src/modules/ctxMenuController.ts
import { MenuOverlayView } from './menu/menuOverlayView.ts';
import { ContextMenuBuilder } from './menu/contextMenuBuilder.ts';
import { MoreOptionsMenuBuilder } from './menu/moreOptionsMenuBuilder.ts';
import type { MenuContext } from './menu/menuTypes.ts';

export interface CtxMenuDeps {
  api: () => any;
  state: any;
  setStatus: (msg: string) => void;
  getFilteredSelection?: (side: 'left' | 'right') => { item: any; count?: number; total?: number; vis?: any[] };
  fullPath?: (side: 'left' | 'right' | any, item: any) => Promise<string | null>;
  openSelected?: (side: 'left' | 'right') => Promise<void> | void;
  copyToOther?: () => Promise<void> | void;
  moveToOther?: () => Promise<void> | void;
  beginRename?: (opts?: any) => Promise<void> | void;
  beginDelete?: (opts?: any) => Promise<void> | void;
  loadDir?: (s: 'left' | 'right') => Promise<void> | void;
  renderPane?: (s: 'left' | 'right') => void;
  openGitBlame?: (f: string, r?: any) => Promise<void> | void;
  openGitDiff?: (f: string, r?: any) => Promise<void> | void;
  openGitLog?: (f: string, r?: any) => Promise<void> | void;
  openMultiRename?: () => void;
  openChecksum?: (fp: string) => void;
  openTerminal?: (p?: string) => void;
  tagController?: any;
  sidebarController?: any;
  viewController?: any;
  commandsController?: any;
  tabsRenderer?: any;
  runCommand?: (cmd: string, payload?: any) => void;
  showProperties?: (side: 'left' | 'right', fp?: string) => void;
  onPreviewSelected?: (fp: string, item: any) => void;
}

export class CtxMenuController {
  private overlayView: MenuOverlayView;
  private contextMenuBuilder: ContextMenuBuilder;
  private moreOptionsMenuBuilder: MoreOptionsMenuBuilder;

  public fileOps?: any;
  public state: any;
  public api: () => any;
  public setStatus: (msg: string) => void;
  public getFilteredSelection?: (side: 'left' | 'right') => { item: any; count?: number; total?: number; vis?: any[] };
  public fullPath?: (side: 'left' | 'right' | any, item: any) => Promise<string | null>;
  public openSelected?: (side: 'left' | 'right') => Promise<void> | void;
  public copyToOther?: () => Promise<void> | void;
  public moveToOther?: () => Promise<void> | void;
  public beginRename?: (opts?: any) => Promise<void> | void;
  public beginDelete?: (opts?: any) => Promise<void> | void;
  public loadDir?: (s: 'left' | 'right') => Promise<void> | void;
  public renderPane?: (s: 'left' | 'right') => void;
  public openGitBlame?: (f: string, r?: any) => Promise<void> | void;
  public openGitDiff?: (f: string, r?: any) => Promise<void> | void;
  public openGitLog?: (f: string, r?: any) => Promise<void> | void;
  public openMultiRename?: () => void;
  public openChecksum?: (fp: string) => void;
  public openTerminal?: (p?: string) => void;
  public tagController?: any;
  public sidebarController?: any;
  public viewController?: any;
  public commandsController?: any;
  public diskSpaceController?: any;
  public duplicateFinderController?: any;
  public tabsRenderer?: any;
  public runCommand?: (cmd: string, payload?: any) => void;
  public showProperties?: (side: 'left' | 'right', fp?: string) => void;
  public onPreviewSelected?: (fp: string, item: any) => void;

  constructor(deps: CtxMenuDeps, overlayView?: MenuOverlayView) {
    this.api = deps.api;
    this.state = deps.state;
    this.setStatus = deps.setStatus;
    this.getFilteredSelection = deps.getFilteredSelection;
    this.fullPath = deps.fullPath;
    this.openSelected = deps.openSelected;
    this.copyToOther = deps.copyToOther;
    this.moveToOther = deps.moveToOther;
    this.beginRename = deps.beginRename;
    this.beginDelete = deps.beginDelete;
    this.loadDir = deps.loadDir;
    this.renderPane = deps.renderPane;
    this.openGitBlame = deps.openGitBlame;
    this.openGitDiff = deps.openGitDiff;
    this.openGitLog = deps.openGitLog;
    this.openMultiRename = deps.openMultiRename;
    this.openChecksum = deps.openChecksum;
    this.openTerminal = deps.openTerminal;
    this.tagController = deps.tagController;
    this.sidebarController = deps.sidebarController;
    this.viewController = deps.viewController;
    this.commandsController = deps.commandsController;
    this.tabsRenderer = deps.tabsRenderer;
    this.runCommand = deps.runCommand;
    this.showProperties = deps.showProperties;
    this.onPreviewSelected = deps.onPreviewSelected;

    this.overlayView = overlayView || new MenuOverlayView('ctx-menu');
    this.contextMenuBuilder = new ContextMenuBuilder();
    this.moreOptionsMenuBuilder = new MoreOptionsMenuBuilder();
  }

  // Compatibility getter for tests/inspectors accessing DOM element
  get el(): HTMLElement | null {
    return this.overlayView.el;
  }

  set el(val: HTMLElement | null) {
    this.overlayView.el = val;
  }

  get items() {
    return this.overlayView.items;
  }

  set items(val) {
    this.overlayView.items = val;
  }

  get selectedIndex(): number {
    return this.overlayView.selectedIndex;
  }

  set selectedIndex(val: number) {
    this.overlayView.selectedIndex = val;
  }

  isOpen(): boolean {
    return this.overlayView.isOpen();
  }

  hide(): void {
    this.overlayView.hide();
  }

  navigate(delta: number): void {
    this.overlayView.navigate(delta);
  }

  renderSelection(): void {
    this.overlayView.renderSelection();
  }

  executeSelected(): void {
    this.overlayView.executeSelected();
  }

  show(
    x: number,
    y: number,
    side: 'left' | 'right',
    emptyArea = false,
    explicitItem: any = null,
    explicitDirPath: string | null = null,
  ): void {
    const ctx = this.buildContext(side, emptyArea, explicitItem, explicitDirPath);
    const items = this.contextMenuBuilder.build(ctx, this, () => this.hide());
    this.overlayView.showAt(x, y, items);
  }

  showMoreMenu(x: number, y: number, side: 'left' | 'right'): void {
    const ctx = this.buildContext(side, false, null, null);
    const items = this.moreOptionsMenuBuilder.build(ctx, this, () => this.hide());
    this.overlayView.showAt(x, y, items);
  }

  private buildContext(
    side: 'left' | 'right',
    emptyArea = false,
    explicitItem: any = null,
    explicitDirPath: string | null = null,
  ): MenuContext {
    const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent);
    const { item: rawItem } = this.getFilteredSelection ? this.getFilteredSelection(side) : { item: null };
    const item = emptyArea ? null : (explicitItem || rawItem);
    const targetDir = explicitDirPath || this.state[side]?.path || '';
    const isFile = !!(item && item.base !== '' && item.base !== '..' && !item.isDir);
    const isDir = !!(item && item.base !== '' && item.base !== '..' && item.isDir);

    const getPath = async (): Promise<string | null> => {
      if (!item || item.base === '' || item.base === '..') return null;
      if (explicitDirPath) {
        try {
          const apiObj = typeof this.api === 'function' ? this.api() : this.api;
          if (apiObj?.pathJoin) return await apiObj.pathJoin(explicitDirPath, item.base);
        } catch (_) {}
        const sepChar = explicitDirPath.includes('\\') ? '\\' : '/';
        return explicitDirPath.endsWith(sepChar) ? `${explicitDirPath}${item.base}` : `${explicitDirPath}${sepChar}${item.base}`;
      }
      return this.fullPath ? await this.fullPath(side, item) : null;
    };

    return {
      side,
      emptyArea,
      item,
      targetDir,
      isFile,
      isDir,
      getPath,
      isMac,
    };
  }
}
