import type { Terminal } from '@xterm/xterm';
import type { FitAddon } from '@xterm/addon-fit';
import type { AppState } from './stateModels.ts';

export interface TerminalDrawerDeps {
  state: any;
  api: () => any;
  setStatus: (msg: string) => void;
  focusActiveList: () => void;
  loadDir?: (side: 'left' | 'right') => Promise<void>;
  navigateTo?: (side: string, path: string) => Promise<void>;
  terminalFactory?: (host: HTMLElement) => { terminal: Terminal; fitAddon: FitAddon };
}

export function ansiToHtml(raw: string): string {
  const safe = raw.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  return safe
    .replace(/\x1b\[0m/g, '</span>')
    .replace(/\x1b\[1m/g, '<span style="font-weight:bold;">')
    .replace(/\x1b\[31m/g, '<span style="color:#ff453a;">')
    .replace(/\x1b\[32m/g, '<span style="color:#30d158;">')
    .replace(/\x1b\[33m/g, '<span style="color:#ffd60a;">')
    .replace(/\x1b\[34m/g, '<span style="color:#0a84ff;">')
    .replace(/\x1b\[35m/g, '<span style="color:#bf5af2;">')
    .replace(/\x1b\[36m/g, '<span style="color:#64d2ff;">')
    .replace(/\x1b\[90m/g, '<span style="color:#8e8e93;">')
    .replace(/\x1b\[[0-9;]*m/g, '');
}

export class TerminalDrawerController {
  public state: AppState;
  public api: () => any;
  public setStatus: (msg: string) => void;
  public focusActiveList: () => void;
  public loadDir?: (side: 'left' | 'right') => Promise<void>;
  public navigateTo?: (side: string, path: string) => Promise<void>;
  public terminalFactory?: TerminalDrawerDeps['terminalFactory'];
  public isOpen = false;
  public isRunning = false;
  public cwd: string | null = null;
  public history: string[] = [];
  public historyIndex = -1;

  private terminal: Terminal | null = null;
  private fitAddon: FitAddon | null = null;
  private sessionId: string | null = null;
  private unlisten: (() => void) | null = null;
  private resizeObserver: ResizeObserver | null = null;
  private starting: Promise<void> | null = null;
  private pendingOutput: Array<{ sessionId: string; data: string }> = [];
  private pendingExits = new Set<string>();

  constructor(deps: TerminalDrawerDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.setStatus = deps.setStatus;
    this.focusActiveList = deps.focusActiveList;
    this.loadDir = deps.loadDir;
    this.navigateTo = deps.navigateTo;
    this.terminalFactory = deps.terminalFactory;
  }

  public toggle(targetPath?: string): void {
    if (this.isOpen) this.hide();
    else this.show(targetPath);
  }

  public show(targetPath?: string): void {
    this.isOpen = true;
    const drawer = document.getElementById('terminal-drawer');
    if (!drawer) return;
    drawer.classList.remove('hidden');
    drawer.setAttribute('aria-hidden', 'false');
    try {
      const savedHeight = localStorage.getItem('Oryn.terminalHeight') || localStorage.getItem('Oswin.terminalHeight');
      if (savedHeight) drawer.style.height = `${savedHeight}px`;
    } catch { }

    if (targetPath) this.cwd = targetPath;
    else if (!this.cwd) this.cwd = this.state[this.state.active]?.path || null;
    this.updateCwd();
    void this.ensureTerminal();
  }

  public hide(): void {
    this.isOpen = false;
    const drawer = document.getElementById('terminal-drawer');
    drawer?.classList.add('hidden');
    drawer?.setAttribute('aria-hidden', 'true');
    this.focusActiveList();
  }

  public updateCwd(): void {
    if (!this.cwd) this.cwd = this.state[this.state.active]?.path || '~';
    const cwdEl = document.getElementById('terminal-cwd');
    if (cwdEl) {
      cwdEl.textContent = this.cwd || '~';
      cwdEl.title = this.cwd || '';
    }
  }

  private async ensureTerminal(): Promise<void> {
    if (this.starting) return this.starting;
    if (this.sessionId) {
      this.fitAndResize();
      this.terminal?.focus();
      return;
    }
    this.starting = this.startTerminal();
    try { await this.starting; }
    catch (error) { this.reportError(error); }
    finally { this.starting = null; }
  }

  private async startTerminal(): Promise<void> {
    const host = document.getElementById('terminal-output');
    if (!host) return;
    if (!this.terminal) {
      const createXterm = async () => {
        const [{ Terminal }, { FitAddon }] = await Promise.all([
          import('@xterm/xterm'),
          import('@xterm/addon-fit'),
        ]);
        return { terminal: new Terminal({
        cursorBlink: true,
        fontFamily: 'var(--font-mono), Menlo, monospace',
        fontSize: 12,
        convertEol: true,
        scrollback: 5000,
        theme: { background: '#111214', foreground: '#e0e0e0', cursor: '#7ee787', selectionBackground: '#34516f' },
        }), fitAddon: new FitAddon() };
      };
      const { terminal, fitAddon } = this.terminalFactory
        ? this.terminalFactory(host)
        : await createXterm();
      this.terminal = terminal;
      this.fitAddon = fitAddon;
      this.terminal.loadAddon(this.fitAddon);
      this.terminal.open(host);
      this.terminal.onData((data) => {
        if (this.sessionId) void this.api().terminalWrite(this.sessionId, data).catch((e: any) => this.reportError(e));
      });
      this.resizeObserver = new ResizeObserver(() => this.fitAndResize());
      this.resizeObserver.observe(host);
      window.addEventListener('resize', this.fitAndResize);
    }
    this.fitAddon?.fit();
    const cols = this.terminal.cols || 80;
    const rows = this.terminal.rows || 24;
    const api = this.api();
    if (typeof api.terminalListen !== 'function' || typeof api.terminalStart !== 'function') {
      this.terminal.write('Terminal PTY is unavailable in this backend build.\r\n');
      return;
    }
    this.unlisten ??= await api.terminalListen(
      (sessionId: string, data: string) => {
        if (sessionId === this.sessionId) this.terminal?.write(data);
        else if (!this.sessionId) this.pendingOutput.push({ sessionId, data });
      },
      (sessionId: string) => {
        if (sessionId === this.sessionId) {
          this.sessionId = null;
          this.isRunning = false;
          this.setStatus('Shell exited');
        } else if (!this.sessionId) this.pendingExits.add(sessionId);
      },
    );
    try {
      const sessionId: string = await api.terminalStart(this.cwd, cols, rows);
      this.sessionId = sessionId;
      for (const event of this.pendingOutput) {
        if (event.sessionId === sessionId) this.terminal?.write(event.data);
      }
      this.pendingOutput = [];
      if (this.pendingExits.delete(sessionId)) {
        this.sessionId = null;
        this.isRunning = false;
        this.setStatus('Shell exited');
        return;
      }
      this.isRunning = true;
      this.setStatus('Shell ready');
      this.terminal?.focus();
    } catch (error) {
      this.reportError(error);
    }
  }

  private fitAndResize = (): void => {
    if (!this.terminal || !this.fitAddon) return;
    try {
      this.fitAddon.fit();
      if (this.sessionId) void this.api().terminalResize(this.sessionId, this.terminal.cols, this.terminal.rows).catch(() => {});
    } catch { }
  };

  private reportError(error: any): void {
    const message = error?.message || String(error);
    this.terminal?.writeln(`\r\n\x1b[31m${message}\x1b[0m`);
    this.setStatus('Terminal error');
  }

  public clear(): void {
    this.terminal?.clear();
    this.terminal?.focus();
  }

  public appendOutput(text: string, isErr = false, isCmd = false): void {
    if (!this.terminal) return;
    if (isCmd) this.terminal.write('\x1b[1;34m');
    if (isErr) this.terminal.write('\x1b[31m');
    this.terminal.write(text);
    if (isErr || isCmd) this.terminal.write('\x1b[0m');
    this.terminal.write('\r\n');
  }

  public async copyOutput(): Promise<void> {
    const selection = this.terminal?.getSelection() || '';
    const text = selection || this.readVisibleBuffer();
    if (text) {
      await this.api().clipboardWrite(text);
      this.setStatus(selection ? 'Selection copied.' : 'Terminal output copied to clipboard.');
      const statusEl = document.getElementById('terminal-status');
      if (statusEl) statusEl.textContent = '✓ Copied';
    }
  }

  private readVisibleBuffer(): string {
    const buffer = this.terminal?.buffer.active;
    if (!buffer) return '';
    const lines: string[] = [];
    for (let i = 0; i < buffer.length; i++) lines.push(buffer.getLine(i)?.translateToString(true) || '');
    return lines.join('\n').trimEnd();
  }

  public async openExternalTerminal(): Promise<void> {
    let path = this.cwd || this.state[this.state.active]?.path;
    if (!path || path === '~') {
      try { path = await this.api().getHome(); }
      catch { path = '/'; }
    }
    const statusEl = document.getElementById('terminal-status');
    if (statusEl) statusEl.textContent = 'Launching…';
    try {
      if (typeof this.api().openTerminal === 'function') await this.api().openTerminal(path);
      else await this.api().shellExec('open -a Terminal .', path);
      this.setStatus(`Launched external terminal at: ${path}`);
      if (statusEl) statusEl.textContent = '✓ Terminal opened';
    } catch (e: any) {
      this.setStatus(`Failed to open external terminal: ${e?.message || e}`);
      if (statusEl) statusEl.textContent = '✗ Launch failed';
    }
  }

  public async runCommand(command?: string): Promise<void> {
    if (!command?.trim()) return;
    if (!this.cwd) this.cwd = this.state[this.state.active]?.path || null;
    this.history.push(command.trim());
    this.historyIndex = this.history.length;
    await this.ensureTerminal();
    if (this.sessionId) await this.api().terminalWrite(this.sessionId, `${command}\r`);
  }

  public setupResizeHandle(): void {
    const handle = document.getElementById('terminal-resize-handle');
    const drawer = document.getElementById('terminal-drawer');
    if (!handle || !drawer) return;
    let resizing = false;
    let startY = 0;
    let startHeight = 0;
    const onMouseDown = (e: MouseEvent) => {
      resizing = true;
      startY = e.clientY;
      startHeight = drawer.offsetHeight;
      handle.classList.add('resizing');
      document.body.style.userSelect = 'none';
      document.body.style.cursor = 'ns-resize';
      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    };
    const onMouseMove = (e: MouseEvent) => {
      if (!resizing) return;
      drawer.style.height = `${Math.max(140, Math.min(Math.floor(window.innerHeight * 0.85), startHeight + startY - e.clientY))}px`;
      this.fitAndResize();
    };
    const onMouseUp = () => {
      if (!resizing) return;
      resizing = false;
      handle.classList.remove('resizing');
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
      try { localStorage.setItem('Oryn.terminalHeight', String(drawer.offsetHeight)); } catch { }
    };
    handle.addEventListener('mousedown', onMouseDown);
  }

  public setup(): void {
    this.setupResizeHandle();
    document.getElementById('terminal-close-btn')?.addEventListener('click', () => this.hide());
    document.getElementById('terminal-clear-btn')?.addEventListener('click', () => this.clear());
    document.getElementById('terminal-copy-btn')?.addEventListener('click', () => void this.copyOutput());
    document.getElementById('terminal-external-btn')?.addEventListener('click', () => void this.openExternalTerminal());
    document.getElementById('terminal-output')?.addEventListener('click', () => this.terminal?.focus());
  }
}
