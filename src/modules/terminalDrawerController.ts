// src/modules/terminalDrawerController.ts
import type { AppState } from './stateModels.ts';

export interface TerminalDrawerDeps {
  state: any;
  api: () => any;
  setStatus: (msg: string) => void;
  focusActiveList: () => void;
  loadDir?: (side: 'left' | 'right') => Promise<void>;
  navigateTo?: (side: string, path: string) => Promise<void>;
}

export class TerminalDrawerController {
  public state: AppState;
  public api: () => any;
  public setStatus: (msg: string) => void;
  public focusActiveList: () => void;
  public loadDir?: (side: 'left' | 'right') => Promise<void>;
  public navigateTo?: (side: string, path: string) => Promise<void>;

  public isOpen: boolean;
  public history: string[];
  public historyIndex: number;
  public isRunning: boolean;
  public cwd: string | null;
  public previousCwd: string | null;

  constructor(deps: TerminalDrawerDeps) {
    this.state = deps.state;
    this.api = deps.api;
    this.setStatus = deps.setStatus;
    this.focusActiveList = deps.focusActiveList;
    this.loadDir = deps.loadDir;
    this.navigateTo = deps.navigateTo;

    this.isOpen = false;
    this.history = [];
    this.historyIndex = -1;
    this.isRunning = false;
    this.cwd = null;
    this.previousCwd = null;
  }

  public toggle(targetPath?: string): void {
    if (this.isOpen) {
      this.hide();
    } else {
      this.show(targetPath);
    }
  }

  public show(targetPath?: string): void {
    this.isOpen = true;
    const drawer = document.getElementById('terminal-drawer');
    if (!drawer) return;

    drawer.classList.remove('hidden');
    drawer.setAttribute('aria-hidden', 'false');

    // Restore user height if set
    try {
      const savedHeight = localStorage.getItem('Oryn.terminalHeight') || localStorage.getItem('Oswin.terminalHeight');
      if (savedHeight) drawer.style.height = `${savedHeight}px`;
    } catch { }

    if (targetPath) {
      this.cwd = targetPath;
    } else if (!this.cwd) {
      this.cwd = this.state[this.state.active]?.path || null;
    }

    this.updateCwd();

    const input = document.getElementById('terminal-input') as HTMLInputElement | null;
    if (input) {
      input.focus();
      input.select();
    }
  }

  public hide(): void {
    this.isOpen = false;
    const drawer = document.getElementById('terminal-drawer');
    if (drawer) {
      drawer.classList.add('hidden');
      drawer.setAttribute('aria-hidden', 'true');
    }
    if (typeof this.focusActiveList === 'function') {
      this.focusActiveList();
    }
  }

  public updateCwd(): void {
    if (!this.cwd) {
      this.cwd = this.state[this.state.active]?.path || '~';
    }
    const cwdEl = document.getElementById('terminal-cwd');
    if (cwdEl) {
      cwdEl.textContent = this.cwd || '~';
      cwdEl.title = this.cwd || '';
    }
  }

  public appendOutput(text: string, isErr = false, isCmd = false): void {
    const outputEl = document.getElementById('terminal-output');
    if (!outputEl) return;

    const line = document.createElement('div');
    line.className = `terminal-line${isErr ? ' terminal-line--err' : ''}${isCmd ? ' terminal-line--cmd' : ''}`;
    line.textContent = text;

    outputEl.appendChild(line);
    outputEl.scrollTop = outputEl.scrollHeight;
  }

  public clear(): void {
    const outputEl = document.getElementById('terminal-output');
    if (outputEl) outputEl.replaceChildren();
    const statusEl = document.getElementById('terminal-status');
    if (statusEl) statusEl.textContent = '';
  }

  public async copyOutput(): Promise<void> {
    const outputEl = document.getElementById('terminal-output');
    if (!outputEl) return;
    const text = outputEl.innerText || outputEl.textContent || '';
    if (text) {
      await this.api().clipboardWrite(text);
      this.setStatus('Terminal output copied to clipboard.');
      const statusEl = document.getElementById('terminal-status');
      if (statusEl) statusEl.textContent = '✓ Copied';
    }
  }

  public async openExternalTerminal(): Promise<void> {
    let p = this.cwd || this.state[this.state.active]?.path;
    if (!p || p === '~') {
      try {
        p = await this.api().getHome();
      } catch {
        p = '/';
      }
    }
    const statusEl = document.getElementById('terminal-status');
    if (statusEl) statusEl.textContent = 'Launching…';
    try {
      if (typeof this.api().openTerminal === 'function') {
        await this.api().openTerminal(p);
      } else {
        await this.api().shellExec('open -a Terminal .', p);
      }
      this.setStatus(`Launched external terminal at: ${p}`);
      if (statusEl) statusEl.textContent = '✓ Terminal opened';
    } catch (e: any) {
      this.setStatus(`Failed to open external terminal: ${e?.message || e}`);
      if (statusEl) statusEl.textContent = '✗ Launch failed';
    }
  }

  public async runCommand(cmdText?: string): Promise<void> {
    const cmd = (cmdText || '').trim();
    if (!cmd) return;

    // Handle clear built-in
    if (cmd === 'clear' || cmd === 'cls') {
      this.clear();
      return;
    }

    if (!this.cwd) {
      this.cwd = this.state[this.state.active]?.path || null;
    }

    this.history.push(cmd);
    this.historyIndex = this.history.length;

    this.appendOutput(`$ ${cmd}`, false, true);

    const input = document.getElementById('terminal-input') as HTMLInputElement | null;
    if (input) input.value = '';

    const statusEl = document.getElementById('terminal-status');
    if (statusEl) statusEl.textContent = 'Running…';

    // Built-in: help
    if (cmd === 'help') {
      this.appendOutput(
        'Oryn Integrated Shell Commands:\n' +
        '  cd <path>      Change directory (syncs file manager pane in real-time)\n' +
        '  pwd            Print current working directory\n' +
        '  clear / cls    Clear terminal screen\n' +
        '  help           Show this help text\n' +
        '  <any command>  Executed in native login shell with full PATH\n\n' +
        'Hotkeys & Shortcuts:\n' +
        '  Ctrl+` / F9    Toggle terminal drawer\n' +
        '  Tab            Autocomplete file/folder names\n' +
        '  Up / Down      Browse command history\n' +
        '  Ctrl+C         Clear input line\n' +
        '  Ctrl+L         Clear output screen\n' +
        '  Esc            Close terminal & return focus',
        false
      );
      if (statusEl) statusEl.textContent = '✓ Done';
      return;
    }

    // Built-in: pwd
    if (cmd === 'pwd') {
      this.appendOutput(this.cwd || '/', false);
      if (statusEl) statusEl.textContent = '✓ Done';
      return;
    }

    // Built-in: cd
    if (cmd === 'cd' || cmd.startsWith('cd ') || cmd.startsWith('cd\t')) {
      let rawTarget = cmd.slice(2).trim();
      let target = rawTarget
        .replace(/\\ /g, ' ')
        .replace(/^["']|["']$/g, '');

      let home = '/';
      try {
        home = await this.api().getHome();
      } catch { }

      if (!target || target === '~') {
        target = home;
      } else if (target === '~/' || target === '~\\' || target.startsWith('~/') || target.startsWith('~\\')) {
        target = `${home.replace(/[/\\]+$/, '')}/${target.slice(2)}`;
      } else if (target === '-') {
        if (this.previousCwd) {
          target = this.previousCwd;
        } else {
          this.appendOutput('cd: OLDPWD not set', true);
          if (statusEl) statusEl.textContent = '✗ Error';
          return;
        }
      } else if (target === '..') {
        const cur = (this.cwd || '').replace(/[/\\]+$/, '');
        if (cur.match(/^[A-Za-z]:$/) || cur === '/' || cur === '') {
          target = cur.includes(':') ? `${cur}\\` : '/';
        } else {
          const lastSlash = Math.max(cur.lastIndexOf('/'), cur.lastIndexOf('\\'));
          const parent = cur.substring(0, lastSlash);
          if (parent.match(/^[A-Za-z]:$/)) {
            target = `${parent}\\`;
          } else {
            target = parent || (cur.includes(':') ? `${cur.slice(0, 2)}\\` : '/');
          }
        }
      } else if (target.match(/^[A-Za-z]:$/)) {
        target = `${target}\\`;
      } else if (!target.startsWith('/') && !target.match(/^[A-Za-z]:[/\\]/)) {
        // Relative path
        const base = (this.cwd || '').replace(/[/\\]+$/, '');
        const sep = base.includes('\\') ? '\\' : '/';
        target = `${base}${sep}${target}`;
      }

      // Check if target directory exists
      let isDir = false;
      try {
        const stat = await this.api().statProps(target);
        if (stat && stat.ok && stat.props?.isDir) {
          isDir = true;
        }
      } catch { }

      if (!isDir) {
        try {
          const list = await this.api().readDir(target);
          if (list && (list.ok || Array.isArray(list.items))) {
            isDir = true;
          }
        } catch { }
      }

      if (isDir) {
        this.previousCwd = this.cwd;
        this.cwd = target;
        this.updateCwd();
        const activeSide = this.state.active;
        if (typeof this.navigateTo === 'function') {
          await this.navigateTo(activeSide, target);
        } else if (typeof this.loadDir === 'function') {
          this.state[activeSide].path = target;
          await this.loadDir(activeSide);
        }
        if (statusEl) statusEl.textContent = '✓ Done';
        return;
      } else {
        this.appendOutput(`cd: no such file or directory: ${target}`, true);
        if (statusEl) statusEl.textContent = '✗ Error';
        return;
      }
    }

    this.isRunning = true;
    try {
      const res = await this.api().shellExec(cmd, this.cwd);
      if (res.stdout) {
        this.appendOutput(res.stdout.trimEnd(), false);
      }
      if (res.stderr) {
        this.appendOutput(res.stderr.trimEnd(), true);
      }
      if (statusEl) {
        statusEl.textContent = res.code === 0 ? '✓ Done' : `✗ Exit code ${res.code}`;
      }
    } catch (err: any) {
      this.appendOutput(err?.message || String(err), true);
      if (statusEl) statusEl.textContent = '✗ Error';
    } finally {
      this.isRunning = false;
    }
  }

  public setupResizeHandle(): void {
    const handle = document.getElementById('terminal-resize-handle');
    const drawer = document.getElementById('terminal-drawer');
    if (!handle || !drawer) return;

    let isResizing = false;
    let startY = 0;
    let startHeight = 0;

    const onMouseDown = (e: MouseEvent) => {
      isResizing = true;
      startY = e.clientY;
      startHeight = drawer.offsetHeight;
      handle.classList.add('resizing');
      document.body.style.userSelect = 'none';
      document.body.style.cursor = 'ns-resize';

      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    };

    const onMouseMove = (e: MouseEvent) => {
      if (!isResizing) return;
      const delta = startY - e.clientY;
      const maxH = Math.floor(window.innerHeight * 0.85);
      const newHeight = Math.max(140, Math.min(maxH, startHeight + delta));
      drawer.style.height = `${newHeight}px`;
    };

    const onMouseUp = () => {
      if (!isResizing) return;
      isResizing = false;
      handle.classList.remove('resizing');
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);

      try {
        localStorage.setItem('Oryn.terminalHeight', String(drawer.offsetHeight));
        localStorage.setItem('Oswin.terminalHeight', String(drawer.offsetHeight));
      } catch { }
    };

    handle.addEventListener('mousedown', onMouseDown);
  }

  public setup(): void {
    this.setupResizeHandle();

    document.getElementById('terminal-close-btn')?.addEventListener('click', () => this.hide());
    document.getElementById('terminal-clear-btn')?.addEventListener('click', () => this.clear());
    document.getElementById('terminal-copy-btn')?.addEventListener('click', () => void this.copyOutput());
    document.getElementById('terminal-external-btn')?.addEventListener('click', () => void this.openExternalTerminal());

    const drawer = document.getElementById('terminal-drawer');
    const input = document.getElementById('terminal-input') as HTMLInputElement | null;

    if (drawer && input) {
      drawer.addEventListener('click', (e) => {
        const target = e.target as HTMLElement;
        if (target.tagName !== 'BUTTON' && !target.closest('button')) {
          input.focus();
        }
      });
    }

    if (input) {
      input.addEventListener('keydown', async (e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          void this.runCommand(input.value);
        } else if (e.key === 'Tab') {
          e.preventDefault();
          const val = input.value;
          const lastWordMatch = val.match(/([^\s"']+)$/);
          const prefix = lastWordMatch ? lastWordMatch[1] : '';
          if (!prefix) return;

          try {
            const res = await this.api().readDir(this.cwd || '.');
            const items = res?.items || [];
            const matches = items
              .map((en: any) => en.base || en.display || en.name)
              .filter((name: string) => name && name !== '..' && name.toLowerCase().startsWith(prefix.toLowerCase()));

            if (matches.length === 1) {
              const completed = matches[0];
              const suffix = val.slice(0, val.length - prefix.length);
              const needsQuote = completed.includes(' ') && !val.includes('"');
              if (needsQuote) {
                input.value = `${suffix}"${completed}"`;
              } else {
                input.value = `${suffix}${completed}`;
              }
            } else if (matches.length > 1) {
              this.appendOutput(matches.join('   '), false);
            }
          } catch { }
        } else if (e.key === 'ArrowUp') {
          e.preventDefault();
          if (this.history.length > 0) {
            if (this.historyIndex > 0) {
              this.historyIndex -= 1;
            }
            input.value = this.history[this.historyIndex] || '';
          }
        } else if (e.key === 'ArrowDown') {
          e.preventDefault();
          if (this.history.length > 0) {
            if (this.historyIndex < this.history.length - 1) {
              this.historyIndex += 1;
              input.value = this.history[this.historyIndex] || '';
            } else {
              this.historyIndex = this.history.length;
              input.value = '';
            }
          }
        } else if (e.key === 'Escape') {
          e.preventDefault();
          this.hide();
        } else if (e.ctrlKey && (e.key.toLowerCase() === 'c' || e.key === 'c')) {
          e.preventDefault();
          input.value = '';
        } else if (e.ctrlKey && (e.key.toLowerCase() === 'l' || e.key === 'l')) {
          e.preventDefault();
          this.clear();
        }
      });
    }
  }
}
