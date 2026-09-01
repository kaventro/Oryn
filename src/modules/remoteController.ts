// src/modules/remoteController.ts

export interface RemotePathInfo {
  isRemote: boolean;
  profileId: string | null;
  remotePath: string;
}

export function isRemotePath(pathStr?: string | null): RemotePathInfo {
  if (!pathStr || typeof pathStr !== 'string') {
    return { isRemote: false, profileId: null, remotePath: '' };
  }
  const clean = pathStr.trim();
  if (clean.startsWith('sftp://') || clean.startsWith('ssh://')) {
    const withoutScheme = clean.replace(/^sftp:\/\/|^ssh:\/\//, '');
    const slashIdx = withoutScheme.indexOf('/');
    if (slashIdx === -1) {
      return {
        isRemote: true,
        profileId: withoutScheme,
        remotePath: '/',
      };
    }
    const profileId = withoutScheme.slice(0, slashIdx);
    const remotePath = withoutScheme.slice(slashIdx) || '/';
    return {
      isRemote: true,
      profileId,
      remotePath,
    };
  }
  return { isRemote: false, profileId: null, remotePath: pathStr };
}

export function formatRemotePath(profileId: string, remotePath = '/'): string {
  const norm = remotePath.startsWith('/') ? remotePath : `/${remotePath}`;
  return `sftp://${profileId}${norm}`;
}

export interface RemoteProfile {
  id: string;
  name: string;
  host: string;
  port?: number;
  username: string;
  auth_type?: 'Password' | 'Key' | 'Agent' | string;
  password?: string;
  key_path?: string;
  passphrase?: string;
  initial_path?: string;
  accept_unknown_host?: boolean;
  expected_fingerprint?: string;
  [key: string]: any;
}

export interface RemoteControllerDeps {
  api?: any;
  setStatus?: (msg: string) => void;
}

export class RemoteController {
  public api: any;
  public setStatus: (msg: string) => void;
  public profiles: RemoteProfile[];

  constructor(deps: RemoteControllerDeps = {}) {
    this.api = deps.api || (typeof window !== 'undefined' ? (window as any).ow : null);
    this.setStatus = deps.setStatus || console.log;
    this.profiles = [];
  }

  public async loadProfiles(): Promise<RemoteProfile[]> {
    if (!this.api || typeof this.api.remoteListProfiles !== 'function') {
      return [];
    }
    try {
      this.profiles = (await this.api.remoteListProfiles()) || [];
      return this.profiles;
    } catch (e) {
      console.warn('[RemoteController] Failed to load profiles:', e);
      return [];
    }
  }

  public getProfile(id: string): RemoteProfile | null {
    return this.profiles.find((p) => p.id === id) || null;
  }

  public async saveProfile(profile: RemoteProfile): Promise<RemoteProfile[]> {
    const updated = await this.api.remoteSaveProfile(profile);
    this.profiles = updated;
    return updated;
  }

  public async deleteProfile(id: string): Promise<RemoteProfile[]> {
    const updated = await this.api.remoteDeleteProfile(id);
    this.profiles = updated;
    return updated;
  }

  public async testConnection(profile: RemoteProfile): Promise<any> {
    return await this.api.remoteTestConnection(profile);
  }

  public async connect(profile: RemoteProfile): Promise<any> {
    return await this.api.remoteConnect(profile);
  }

  public async disconnect(profileId: string): Promise<any> {
    return await this.api.remoteDisconnect(profileId);
  }

  public async readDir(profileId: string, remotePath: string): Promise<any> {
    return await this.api.remoteReadDir(profileId, remotePath);
  }

  public async readFileText(profileId: string, remotePath: string): Promise<string> {
    return await this.api.remoteReadFileText(profileId, remotePath);
  }

  public async writeFileText(profileId: string, remotePath: string, content: string): Promise<any> {
    return await this.api.remoteWriteFileText(profileId, remotePath, content);
  }

  public async mkdir(profileId: string, remotePath: string): Promise<any> {
    return await this.api.remoteMkdir(profileId, remotePath);
  }

  public async createFile(profileId: string, remotePath: string): Promise<any> {
    return await this.api.remoteCreateFile(profileId, remotePath);
  }

  public async rename(profileId: string, srcPath: string, dstPath: string): Promise<any> {
    return await this.api.remoteRename(profileId, srcPath, dstPath);
  }

  public async delete(profileId: string, remotePath: string, isDir: boolean): Promise<any> {
    return await this.api.remoteDelete(profileId, remotePath, isDir);
  }

  public async download(profileId: string, remotePath: string, localDst: string): Promise<any> {
    return await this.api.remoteDownload(profileId, remotePath, localDst);
  }

  public async upload(profileId: string, localSrc: string, remoteDst: string): Promise<any> {
    return await this.api.remoteUpload(profileId, localSrc, remoteDst);
  }
}
