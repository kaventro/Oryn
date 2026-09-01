// src/modules/stateModels.ts

export interface Item {
  base: string;
  display?: string;
  isDir?: boolean;
  size?: number | null;
  modified?: number | string | null;
  created?: number | string | null;
  tags?: string[] | null;
  gitStatus?: string | null;
  fullPath?: string;
  sizing?: boolean;
  [key: string]: any;
}

export type SortField = 'name' | 'ext' | 'size' | 'date';

export class TabState {
  public id: string;
  public path: string;
  public items: Item[];
  public filter: string;
  public cursor: number;
  public listSerial: number;
  public sortField: SortField;
  public sortAsc: boolean;
  public selectedBases: Set<string>;
  public navHistory: string[];
  public navFwd: string[];

  constructor(initialPath = '') {
    this.id = Math.random().toString(36).substring(2, 9);
    this.path = initialPath;
    this.items = [];
    this.filter = '';
    this.cursor = 0;
    this.listSerial = 0;
    this.sortField = 'name';
    this.sortAsc = true;
    this.selectedBases = new Set();
    this.navHistory = [];
    this.navFwd = [];
  }

  public pushHistory(path?: string): void {
    if (path && path !== this.path) {
      this.navHistory.push(path);
      this.navFwd = [];
    }
  }

  public popHistory(): string | null {
    return this.navHistory.pop() || null;
  }

  public pushFwd(path?: string): void {
    if (path) this.navFwd.push(path);
  }

  public popFwd(): string | null {
    return this.navFwd.pop() || null;
  }

  public get isFilterActive(): boolean {
    return this.filter.trim().length > 0;
  }

  public select(base: string): void {
    if (base === '..') return;
    this.selectedBases.add(base);
  }

  public deselect(base: string): void {
    this.selectedBases.delete(base);
  }

  public toggleSelection(base: string): void {
    if (base === '..') return; // Cannot select parent dir
    if (this.selectedBases.has(base)) {
      this.selectedBases.delete(base);
    } else {
      this.selectedBases.add(base);
    }
  }

  public clearSelection(): void {
    this.selectedBases.clear();
  }
}

export class PaneState {
  public tabs: TabState[];
  public activeTabIndex: number;
  public isBranchView?: boolean;
  public isFlatView?: boolean;
  [key: string]: any;

  constructor(initialPath = '') {
    this.tabs = [new TabState(initialPath)];
    this.activeTabIndex = 0;
  }

  public get activeTab(): TabState {
    return this.tabs[this.activeTabIndex];
  }

  // --- Proxies to Active Tab for Backward Compatibility & Convenience ---
  public get path(): string { return this.activeTab.path; }
  public set path(v: string) { this.activeTab.path = v; }

  public get items(): Item[] { return this.activeTab.items; }
  public set items(v: Item[]) { this.activeTab.items = v; }

  public get filter(): string { return this.activeTab.filter; }
  public set filter(v: string) { this.activeTab.filter = v; }

  public get cursor(): number { return this.activeTab.cursor; }
  public set cursor(v: number) { this.activeTab.cursor = v; }

  public get listSerial(): number { return this.activeTab.listSerial; }
  public set listSerial(v: number) { this.activeTab.listSerial = v; }

  public get sortField(): SortField { return this.activeTab.sortField; }
  public set sortField(v: SortField) { this.activeTab.sortField = v; }

  public get sortAsc(): boolean { return this.activeTab.sortAsc; }
  public set sortAsc(v: boolean) { this.activeTab.sortAsc = v; }
  // 

  public addTab(path?: string): TabState {
    const newTab = new TabState(path || this.activeTab.path);
    this.tabs.push(newTab);
    this.activeTabIndex = this.tabs.length - 1;
    return newTab;
  }

  public closeCurrentTab(): boolean {
    if (this.tabs.length <= 1) return false;
    this.tabs.splice(this.activeTabIndex, 1);
    if (this.activeTabIndex >= this.tabs.length) {
      this.activeTabIndex = this.tabs.length - 1;
    }
    return true;
  }

  public nextTab(): void {
    if (this.tabs.length <= 1) return;
    this.activeTabIndex = (this.activeTabIndex + 1) % this.tabs.length;
  }

  public prevTab(): void {
    if (this.tabs.length <= 1) return;
    this.activeTabIndex = (this.activeTabIndex - 1 + this.tabs.length) % this.tabs.length;
  }
}

export interface AppConfig {
  useTrash: boolean;
  deletionLog: boolean;
  vimNavigation: boolean;
  [key: string]: any;
}

export class AppState {
  public left: PaneState;
  public right: PaneState;
  public active: 'left' | 'right';
  public config: AppConfig;
  public copyInProgress: boolean;
  [key: string]: any;

  constructor() {
    this.left = new PaneState();
    this.right = new PaneState();
    this.active = 'left';
    this.config = { useTrash: true, deletionLog: true, vimNavigation: false };
    this.copyInProgress = false;
  }

  public get activePane(): PaneState {
    return this[this.active];
  }

  public get activeTab(): TabState {
    return this.activePane.activeTab;
  }

  public getPane(side: 'left' | 'right'): PaneState {
    return this[side];
  }
}
