// src/modules/remoteDialog.ts
import { escHtml } from './formatUtils.ts';
import { formatRemotePath, type RemoteProfile, type RemoteController } from './remoteController.ts';
import type { AppState } from './stateModels.ts';

export interface RemoteDialogDeps {
  remoteController: RemoteController;
  navigateTo?: (targetPath: string) => Promise<void>;
  state: AppState;
  setStatus?: (msg: string) => void;
}

export class RemoteDialog {
  public remoteController: RemoteController;
  public navigateTo?: (targetPath: string) => Promise<void>;
  public state: AppState;
  public setStatus: (msg: string) => void;

  public overlay: HTMLElement | null;
  public selectedProfileId: string | null;
  public profiles: RemoteProfile[];

  constructor(deps: RemoteDialogDeps) {
    this.remoteController = deps.remoteController;
    this.navigateTo = deps.navigateTo;
    this.state = deps.state;
    this.setStatus = deps.setStatus || console.log;

    this.overlay = null;
    this.selectedProfileId = null;
    this.profiles = [];
  }

  public async open(): Promise<void> {
    this.profiles = await this.remoteController.loadProfiles();
    if (!this.selectedProfileId && this.profiles.length > 0) {
      this.selectedProfileId = this.profiles[0].id;
    }
    this.render();
  }

  public close(): void {
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
    }
  }

  public render(): void {
    if (this.overlay) {
      this.overlay.remove();
    }

    this.overlay = document.createElement('div');
    this.overlay.className = 'modal-overlay remote-modal-overlay';
    this.overlay.id = 'remote-dialog-overlay';

    const selected: RemoteProfile = this.profiles.find((p) => p.id === this.selectedProfileId) || {
      id: '',
      name: 'New Connection',
      host: '',
      port: 22,
      username: 'root',
      auth_type: 'Password',
      password: '',
      key_path: '',
      passphrase: '',
      initial_path: '/',
    };

    const isNew = !selected.id;

    this.overlay.innerHTML = `
      <div class="modal-dialog remote-modal-dialog" role="dialog" aria-modal="true" aria-labelledby="remote-dialog-title">
        <div class="modal-header">
          <div class="modal-header-title-group">
            <svg class="modal-title-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect>
              <rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect>
              <line x1="6" y1="6" x2="6.01" y2="6"></line>
              <line x1="6" y1="18" x2="6.01" y2="18"></line>
            </svg>
            <h2 id="remote-dialog-title">Remote Connections (SFTP / SSH)</h2>
          </div>
          <button class="modal-close-btn" id="btn-remote-close" title="Close (Esc)">✕</button>
        </div>

        <div class="remote-dialog-body">
          <!-- Server List Sidebar -->
          <div class="remote-sidebar">
            <div class="remote-sidebar-header">
              <span>Saved Servers</span>
              <button class="btn btn-sm btn-accent" id="btn-remote-new" title="Add New Connection">+ Add</button>
            </div>
            <div class="remote-server-list" id="remote-server-list">
              ${this.profiles.length === 0
        ? `<div class="remote-empty-list">No saved servers</div>`
        : this.profiles
          .map(
            (p) => `
                <div class="remote-server-item ${p.id === this.selectedProfileId ? 'active' : ''}" data-id="${escHtml(p.id)}">
                  <div class="remote-server-item-title">${escHtml(p.name || p.host)}</div>
                  <div class="remote-server-item-sub">${escHtml(p.username)}@${escHtml(p.host)}:${p.port || 22}</div>
                </div>`
          )
          .join('')
      }
            </div>
          </div>

          <!-- Server Form -->
          <div class="remote-form-container">
            <form id="remote-profile-form" class="remote-form">
              <div class="remote-form-row">
                <label for="remote-name">Connection Name</label>
                <input type="text" id="remote-name" class="input-text" value="${escHtml(selected.name || '')}" placeholder="e.g. My Ubuntu Server" required />
              </div>

              <div class="remote-form-grid">
                <div class="remote-form-row">
                  <label for="remote-host">Host / IP</label>
                  <input type="text" id="remote-host" class="input-text" value="${escHtml(selected.host || '')}" placeholder="192.168.1.100 or server.com" required />
                </div>
                <div class="remote-form-row remote-port-col">
                  <label for="remote-port">Port</label>
                  <input type="number" id="remote-port" class="input-text" value="${selected.port || 22}" min="1" max="65535" />
                </div>
              </div>

              <div class="remote-form-row">
                <label for="remote-user">Username</label>
                <input type="text" id="remote-user" class="input-text" value="${escHtml(selected.username || 'root')}" placeholder="root or ubuntu" required />
              </div>

              <div class="remote-form-row">
                <label for="remote-auth-type">Authentication</label>
                <select id="remote-auth-type" class="input-select">
                  <option value="Password" ${selected.auth_type === 'Password' ? 'selected' : ''}>Password</option>
                  <option value="Key" ${selected.auth_type === 'Key' ? 'selected' : ''}>SSH Private Key</option>
                  <option value="Agent" ${selected.auth_type === 'Agent' ? 'selected' : ''}>SSH Agent</option>
                </select>
              </div>

              <div class="remote-form-row" id="row-remote-password" style="${selected.auth_type === 'Password' ? '' : 'display:none'}">
                <label for="remote-password">Password</label>
                <input type="password" id="remote-password" class="input-text" value="${escHtml(selected.password || '')}" placeholder="••••••••" />
              </div>

              <div class="remote-form-row" id="row-remote-key" style="${selected.auth_type === 'Key' ? '' : 'display:none'}">
                <label for="remote-key-path">Private Key Path</label>
                <input type="text" id="remote-key-path" class="input-text" value="${escHtml(selected.key_path || '')}" placeholder="~/.ssh/id_rsa or /path/to/key" />
              </div>

              <div class="remote-form-row" id="row-remote-passphrase" style="${selected.auth_type === 'Key' ? '' : 'display:none'}">
                <label for="remote-passphrase">Key Passphrase (optional)</label>
                <input type="password" id="remote-passphrase" class="input-text" value="${escHtml(selected.passphrase || '')}" placeholder="••••••••" />
              </div>

              <div class="remote-form-row">
                <label for="remote-initial-path">Initial Remote Directory</label>
                <input type="text" id="remote-initial-path" class="input-text" value="${escHtml(selected.initial_path || '/')}" placeholder="/ or /home/user or /var/www" />
              </div>

              <div class="remote-form-row" style="margin-top: 6px;">
                <label style="display: flex; align-items: center; gap: 8px; font-size: 12px; cursor: pointer;">
                  <input type="checkbox" id="remote-accept-unknown" ${selected.accept_unknown_host ? 'checked' : ''} />
                  <span>Trust server host key on first connection (TOFU)</span>
                </label>
              </div>

              ${selected.expected_fingerprint ? `
              <div class="remote-form-row" style="margin-top: 6px;">
                <label style="display: flex; align-items: center; gap: 8px; font-size: 12px; cursor: pointer;">
                  <input type="checkbox" id="remote-forget-hostkey" />
                  <span>Forget pinned host key (re-trust on next connect)</span>
                </label>
                <div style="font-size: 11px; opacity: 0.7; margin-top: 4px; word-break: break-all;">Pinned: ${escHtml(selected.expected_fingerprint)}</div>
              </div>
              ` : ''}

              <div id="remote-feedback" class="remote-feedback"></div>
            </form>
          </div>
        </div>

        <div class="modal-footer remote-modal-footer">
          <div class="remote-footer-left">
            ${!isNew ? `<button class="btn btn-danger" id="btn-remote-delete">Delete</button>` : ''}
            <button class="btn btn-secondary" id="btn-remote-test">Test Connection</button>
          </div>
          <div class="remote-footer-right">
            <button class="btn btn-secondary" id="btn-remote-save">Save Profile</button>
            <button class="btn btn-primary btn-connect" id="btn-remote-connect">Connect & Open</button>
          </div>
        </div>
      </div>
    `;

    document.body.appendChild(this.overlay);
    this.bindEvents();
  }

  public bindEvents(): void {
    if (!this.overlay) return;

    const form = this.overlay.querySelector('#remote-profile-form') as HTMLFormElement | null;
    form?.addEventListener('submit', (e) => e.preventDefault());

    const btnClose = this.overlay.querySelector('#btn-remote-close');
    btnClose?.addEventListener('click', () => this.close());

    this.overlay.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        this.close();
      }
    });

    const btnNew = this.overlay.querySelector('#btn-remote-new');
    btnNew?.addEventListener('click', () => {
      this.selectedProfileId = '';
      this.render();
    });

    const items = this.overlay.querySelectorAll('.remote-server-item');
    items.forEach((item) => {
      item.addEventListener('click', () => {
        this.selectedProfileId = (item as HTMLElement).dataset.id || null;
        this.render();
      });
    });

    const authSelect = this.overlay.querySelector('#remote-auth-type') as HTMLSelectElement | null;
    authSelect?.addEventListener('change', (e) => {
      const val = (e.target as HTMLSelectElement).value;
      const rowPass = this.overlay?.querySelector('#row-remote-password') as HTMLElement | null;
      const rowKey = this.overlay?.querySelector('#row-remote-key') as HTMLElement | null;
      const rowPassphrase = this.overlay?.querySelector('#row-remote-passphrase') as HTMLElement | null;
      if (rowPass) rowPass.style.display = val === 'Password' ? '' : 'none';
      if (rowKey) rowKey.style.display = val === 'Key' ? '' : 'none';
      if (rowPassphrase) rowPassphrase.style.display = val === 'Key' ? '' : 'none';
    });

    const btnTest = this.overlay.querySelector('#btn-remote-test');
    btnTest?.addEventListener('click', () => void this.handleTest());

    const btnSave = this.overlay.querySelector('#btn-remote-save');
    btnSave?.addEventListener('click', () => void this.handleSave());

    const btnDelete = this.overlay.querySelector('#btn-remote-delete');
    btnDelete?.addEventListener('click', () => void this.handleDelete());

    const btnConnect = this.overlay.querySelector('#btn-remote-connect');
    btnConnect?.addEventListener('click', () => void this.handleConnect());
  }

  public collectFormData(): RemoteProfile {
    const id = this.selectedProfileId || 'srv_' + Date.now();
    const name = (this.overlay?.querySelector('#remote-name') as HTMLInputElement)?.value.trim() || 'Remote Server';
    const host = (this.overlay?.querySelector('#remote-host') as HTMLInputElement)?.value.trim() || '';
    const port = parseInt((this.overlay?.querySelector('#remote-port') as HTMLInputElement)?.value, 10) || 22;
    const username = (this.overlay?.querySelector('#remote-user') as HTMLInputElement)?.value.trim() || 'root';
    const auth_type = (this.overlay?.querySelector('#remote-auth-type') as HTMLSelectElement)?.value || 'Password';
    const password = (this.overlay?.querySelector('#remote-password') as HTMLInputElement)?.value || null;
    const key_path = (this.overlay?.querySelector('#remote-key-path') as HTMLInputElement)?.value.trim() || null;
    const passphrase = (this.overlay?.querySelector('#remote-passphrase') as HTMLInputElement)?.value || null;
    const initial_path = (this.overlay?.querySelector('#remote-initial-path') as HTMLInputElement)?.value.trim() || '/';
    const accept_unknown_host = (this.overlay?.querySelector('#remote-accept-unknown') as HTMLInputElement)?.checked ?? false;
    const forgetHostKey = (this.overlay?.querySelector('#remote-forget-hostkey') as HTMLInputElement)?.checked ?? false;

    return {
      id,
      name,
      host,
      port,
      username,
      auth_type,
      password: password || undefined,
      key_path: key_path || undefined,
      passphrase: passphrase || undefined,
      initial_path,
      accept_unknown_host,
      expected_fingerprint: forgetHostKey ? '' : undefined,
    };
  }

  public setFeedback(msg: string, isError = false): void {
    const box = this.overlay?.querySelector('#remote-feedback');
    if (box) {
      box.textContent = msg;
      box.className = `remote-feedback ${isError ? 'error' : 'success'}`;
    }
  }

  public async handleTest(): Promise<void> {
    const data = this.collectFormData();
    if (!data.host) {
      this.setFeedback('Please enter host / IP address', true);
      return;
    }
    this.setFeedback('Connecting to test server...');
    try {
      await this.remoteController.testConnection(data);
      this.setFeedback('✓ Connection successful!');
    } catch (e: any) {
      this.setFeedback(`✕ Connection failed: ${e.message || e}`, true);
    }
  }

  public async handleSave(): Promise<void> {
    const data = this.collectFormData();
    if (!data.host) {
      this.setFeedback('Please enter host / IP address', true);
      return;
    }
    try {
      this.profiles = await this.remoteController.saveProfile(data);
      this.selectedProfileId = data.id;
      this.setFeedback('✓ Profile saved successfully!');
      this.render();
    } catch (e: any) {
      this.setFeedback(`✕ Save failed: ${e.message || e}`, true);
    }
  }

  public async handleDelete(): Promise<void> {
    if (!this.selectedProfileId) return;
    if (!confirm('Are you sure you want to delete this connection profile?')) return;
    try {
      this.profiles = await this.remoteController.deleteProfile(this.selectedProfileId);
      this.selectedProfileId = this.profiles[0]?.id || null;
      this.render();
    } catch (e: any) {
      this.setFeedback(`✕ Delete failed: ${e.message || e}`, true);
    }
  }

  public async handleConnect(): Promise<void> {
    const data = this.collectFormData();
    if (!data.host) {
      this.setFeedback('Please enter host / IP address', true);
      return;
    }

    this.setFeedback('Connecting to remote server...');
    try {
      // Save profile first
      this.profiles = await this.remoteController.saveProfile(data);
      this.selectedProfileId = data.id;

      // Connect
      await this.remoteController.connect(data);

      const targetPath = formatRemotePath(data.id, data.initial_path || '/');
      this.close();

      if (this.navigateTo) {
        await this.navigateTo(targetPath);
      }
      this.setStatus(`Connected to ${data.name || data.host}`);
    } catch (e: any) {
      this.setFeedback(`✕ Connect failed: ${e.message || e}`, true);
    }
  }
}
