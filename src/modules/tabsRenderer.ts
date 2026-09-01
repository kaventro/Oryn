// src/modules/tabsRenderer.ts
import type { AppState } from './stateModels.ts';

export interface TabsRendererDeps {
  state: AppState;
  loadDir: (side: 'left' | 'right') => Promise<void>;
  updatePaneClass: () => void;
  focusActiveList: () => void;
  renderPane: (side: 'left' | 'right') => void;
  syncFilterInput: () => void;
}

export class TabsRenderer {
  public state: AppState;
  public loadDir: (side: 'left' | 'right') => Promise<void>;
  public updatePaneClass: () => void;
  public focusActiveList: () => void;
  public renderPane: (side: 'left' | 'right') => void;
  public syncFilterInput: () => void;

  constructor(deps: TabsRendererDeps) {
    this.state = deps.state;
    this.loadDir = deps.loadDir;
    this.updatePaneClass = deps.updatePaneClass;
    this.focusActiveList = deps.focusActiveList;
    this.renderPane = deps.renderPane;
    this.syncFilterInput = deps.syncFilterInput;
  }

  public setup(): void {
    this.render('left');
    this.render('right');
  }

  public activateSide(side: 'left' | 'right'): void {
    this.state.active = side;
    this.syncFilterInput();
    this.updatePaneClass();
  }

  // Gets the folder name or '*' if unknown
  public shortName(path?: string): string {
    if (!path) return '—';
    const parts = String(path).replace(/\\+/g, '/').split('/').filter(Boolean);
    if (parts.length === 0) return '/';
    return parts[parts.length - 1];
  }

  public render(side: 'left' | 'right'): void {
    const pane = this.state[side];
    const container = document.getElementById(`tabs-${side}`);
    if (!container) return;

    container.replaceChildren();

    if (side === this.state.active) {
      const topTitle = document.getElementById('folder-title');
      if (topTitle) {
        topTitle.textContent = this.shortName(pane.path) || 'Oryn';
      }
    }

    pane.tabs.forEach((tab, index) => {
      const btn = document.createElement('button');
      btn.className = 'tab-btn' + (index === pane.activeTabIndex ? ' tab-btn--active' : '');

      const icon = document.createElement('span');
      icon.className = 'tab-icon';
      icon.innerHTML = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/></svg>`;
      btn.appendChild(icon);

      const name = document.createElement('span');
      name.className = 'tab-name';
      name.textContent = this.shortName(tab.path);
      name.title = tab.path;

      btn.appendChild(name);

      if (pane.tabs.length > 1) {
        const close = document.createElement('span');
        close.className = 'tab-close';
        close.innerHTML = '&times;';
        close.addEventListener('click', (e) => {
          e.stopPropagation();
          this.activateSide(side);
          pane.activeTabIndex = index;
          if (pane.closeCurrentTab()) {
            this.render(side);
            this.loadDir(side).then(() => this.focusActiveList());
          }
        });
        btn.appendChild(close);
      }

      btn.addEventListener('click', () => {
        this.activateSide(side);
        if (pane.activeTabIndex !== index) {
          pane.activeTabIndex = index;
          this.render(side);
          this.loadDir(side).then(() => {
            this.renderPane(side); // Re-render the active tab
            this.focusActiveList();
          });
        }
      });

      container.appendChild(btn);
    });
  }

  public handleGlobalKeydown(e: KeyboardEvent): boolean {
    const side = this.state.active;
    const pane = this.state[side];

    if (e.ctrlKey && !e.shiftKey && e.key.toLowerCase() === 't') {
      e.preventDefault();
      pane.addTab(); // Copies current path
      this.render(side);
      this.loadDir(side).then(() => this.focusActiveList());
      return true;
    }

    if (e.ctrlKey && e.key.toLowerCase() === 'w') {
      e.preventDefault();
      if (pane.closeCurrentTab()) {
        this.render(side);
        this.loadDir(side).then(() => this.focusActiveList());
      }
      return true;
    }

    if (e.ctrlKey && e.key === 'Tab') {
      e.preventDefault();
      if (e.shiftKey) {
        pane.prevTab();
      } else {
        pane.nextTab();
      }
      this.render(side);
      this.loadDir(side).then(() => {
        this.renderPane(side);
        this.focusActiveList();
      });
      return true;
    }

    return false;
  }
}
