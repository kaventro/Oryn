/**
 * Tauri IPC bridge — typed interface for `window.ow` (invoke + events).
 */
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

function sanitizeErrorMessage(msg: string): string {
  return msg
    .replace(/(password|passphrase|token|secret)=[^&\s]+/gi, '$1=[REDACTED]')
    .replace(/bearer\s+[a-zA-Z0-9_\-\.]+/gi, 'Bearer [REDACTED]');
}

/** Logs sanitized error to console; throws a short message for UI layers. */
async function ipcInvoke<T = any>(cmd: string, args?: Record<string, any>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e: any) {
    const raw = sanitizeErrorMessage(e?.message ?? String(e));
    console.error(`[ipc] ${cmd} failed:`, raw);
    const short = raw.length > 140 ? `${raw.slice(0, 137)}…` : raw;
    throw new Error(short);
  }
}

async function withChannelPayload<T>(
  channel: string,
  onPayload: ((data: any) => void) | null,
  fn: () => Promise<T>
): Promise<T> {
  let unlisten: UnlistenFn | undefined;
  if (onPayload) {
    unlisten = await listen(channel, (e: any) => onPayload(e.payload));
  }
  try {
    return await fn();
  } finally {
    if (unlisten) unlisten();
  }
}

export interface TauriBridge {
  pathJoin: (a: string, b: string) => Promise<string>;
  pathDirname: (p: string) => Promise<string>;
  pathBasename: (p: string) => Promise<string>;
  pathNormalize: (p: string) => Promise<string>;
  getHome: () => Promise<string>;
  loadConfig: () => Promise<any>;
  readDir: (p: string) => Promise<any>;
  readFlatBranch: (p: string) => Promise<any>;
  mkdir: (p: string) => Promise<any>;
  createFile: (p: string, content?: string) => Promise<any>;
  openPath: (p: string) => Promise<any>;
  showItemInFolder: (p: string) => Promise<any>;
  openVSCode: (p: string) => Promise<any>;
  openTerminal: (p: string) => Promise<any>;
  clipboardWrite: (t: string) => Promise<any>;
  shellExec: (cmd: string, cwd?: string) => Promise<any>;
  statProps: (p: string) => Promise<any>;
  readFileText: (p: string, maxBytes?: number) => Promise<any>;
  writeFileText: (p: string, content: string) => Promise<any>;
  probeText: (p: string) => Promise<any>;
  readOffice: (p: string) => Promise<any>;
  assetUrl: (p: string) => string;
  readMediaDataUrl: (p: string) => Promise<string>;
  rename: (src: string, dst: string) => Promise<any>;
  deletePath: (fullPath: string, useTrash?: boolean) => Promise<any>;
  compressZip: (fullPath: string) => Promise<any>;
  clearSearchCache: () => Promise<any>;
  searchStart: (input: any) => Promise<any>;
  searchPage: (sessionId: string, offset: number, limit: number) => Promise<any>;
  cancelSearch: (clientId: string) => Promise<any>;
  releaseSearch: (sessionId: string) => Promise<any>;
  copy: (src: string, dst: string, policy?: string) => Promise<any>;
  move: (src: string, dst: string, policy?: string) => Promise<any>;
  copyConflicts: (src: string, dst: string) => Promise<any>;
  cancelCopy: () => Promise<any>;
  watchDirs: (paths: string[]) => Promise<any>;
  onFsChange: (cb: (payload: { path: string }) => void) => () => void;
  confirm: (opts: any) => Promise<boolean>;
  info: (opts: any) => Promise<any>;
  onCopyProgress: (cb: (msg: any) => void) => () => void;
  onMenuRefresh: (cb: () => void) => void;
  onMenuCopyPath: (cb: () => void) => void;
  closeWindow: () => Promise<any>;
  minimizeWindow: () => Promise<any>;
  toggleMaximizeWindow: () => Promise<any>;
  setDockIcon: (iconId: string) => Promise<any>;
  getSystemStats: () => Promise<any>;
  getPathSpace: (path: string) => Promise<any>;
  getSystemLocations: () => Promise<any>;
  getDirSize: (path: string) => Promise<any>;
  analyzeDir: (path: string) => Promise<any>;
  scanDuplicates: (path: string, minSizeBytes?: number, maxResults?: number) => Promise<any>;
  fsChecksum: (path: string) => Promise<any>;
  pickFolder: (defaultPath?: string) => Promise<string | null>;
  findReplace: (payload: any, onProgress?: (p: any) => void) => Promise<any>;
  extractZip: (zipPath: string, entryName?: string) => Promise<any>;
  fsCompress: (sources: string[], destination: string) => Promise<any>;
  fsExtract: (archive: string, destination: string) => Promise<any>;
  compareDirs: (leftPath: string, rightPath: string, onUpdate?: (u: any) => void) => Promise<any>;
  compareFiles: (leftPath: string, rightPath: string) => Promise<any>;
  gitIsRepo: (dirPath: string) => Promise<any>;
  gitStatus: (repoPath: string) => Promise<any>;
  gitLog: (repoPath: string, maxCount?: number, filePath?: string) => Promise<any>;
  gitDiff: (repoPath: string, ref1?: string, ref2?: string, filePath?: string) => Promise<any>;
  gitBlame: (repoPath: string, filePath: string, ref?: string) => Promise<any>;
  gitAdd: (repoPath: string, files: string[]) => Promise<any>;
  gitStageFile: (repoPath: string, filePath: string, stage: boolean) => Promise<any>;
  gitRestore: (repoPath: string, filePath: string, staged?: boolean) => Promise<any>;
  gitCommit: (repoPath: string, message: string) => Promise<any>;
  gitPush: (repoPath: string) => Promise<any>;
  gitPull: (repoPath: string) => Promise<any>;
  gitStash: (repoPath: string, message?: string) => Promise<any>;
  gitStashPop: (repoPath: string) => Promise<any>;
  gitStashList: (repoPath: string) => Promise<any>;
  gitBranches: (repoPath: string) => Promise<any>;
  gitCheckout: (repoPath: string, ref: string, isFile?: boolean) => Promise<any>;
  remoteListProfiles: () => Promise<any>;
  remoteSaveProfile: (profile: any) => Promise<any>;
  remoteDeleteProfile: (id: string) => Promise<any>;
  remoteTestConnection: (profile: any) => Promise<any>;
  remoteConnect: (profile: any) => Promise<any>;
  remoteDisconnect: (profileId: string) => Promise<any>;
  remoteReadDir: (profileId: string, path: string) => Promise<any>;
  remoteReadFileText: (profileId: string, path: string) => Promise<any>;
  remoteWriteFileText: (profileId: string, path: string, content: string) => Promise<any>;
  remoteMkdir: (profileId: string, path: string) => Promise<any>;
  remoteCreateFile: (profileId: string, path: string) => Promise<any>;
  remoteRename: (profileId: string, srcPath: string, dstPath: string) => Promise<any>;
  remoteDelete: (profileId: string, path: string, isDir: boolean) => Promise<any>;
  remoteDownload: (profileId: string, remotePath: string, localDst: string) => Promise<any>;
  remoteUpload: (profileId: string, localSrc: string, remoteDst: string) => Promise<any>;
}

export const bridge: TauriBridge = {
  pathJoin: (a: string, b: string) => ipcInvoke('path_join', { a, b }),
  pathDirname: (p: string) => ipcInvoke('path_dirname', { input: { path: p } }),
  pathBasename: (p: string) => ipcInvoke('path_basename', { input: { path: p } }),
  pathNormalize: (p: string) => ipcInvoke('path_normalize', { input: { path: p } }),
  getHome: () => ipcInvoke('app_get_home'),
  loadConfig: () => ipcInvoke('config_load'),
  readDir: (p: string) => ipcInvoke('fs_read_dir', { input: { path: p } }),
  readFlatBranch: (p: string) => ipcInvoke('fs_read_flat_branch', { input: { path: p } }),
  mkdir: (p: string) => ipcInvoke('fs_mkdir', { input: { path: p } }),
  createFile: (p: string, content = '') => ipcInvoke('fs_create_file', { input: { path: p, content } }),
  openPath: (p: string) => ipcInvoke('shell_open_path', { input: { path: p } }),
  showItemInFolder: (p: string) => ipcInvoke('shell_show_in_folder', { input: { path: p } }),
  openVSCode: (p: string) => ipcInvoke('shell_open_vscode', { input: { path: p } }),
  openTerminal: (p: string) => ipcInvoke('shell_open_terminal', { input: { path: p } }),
  clipboardWrite: (t: string) => ipcInvoke('clipboard_write', { input: { text: t } }),
  shellExec: (cmd: string, cwd?: string) => ipcInvoke('shell_exec', { input: { cmd, cwd } }),
  statProps: (p: string) => ipcInvoke('fs_stat_props', { input: { path: p } }),
  readFileText: (p: string, maxBytes?: number) => ipcInvoke('fs_read_file_text', { input: { path: p, maxBytes } }),
  writeFileText: (p: string, content: string) => ipcInvoke('fs_write_file_text', { input: { path: p, content } }),
  probeText: (p: string) => ipcInvoke('fs_probe_text', { input: { path: p } }),
  readOffice: (p: string) => ipcInvoke('fs_read_office', { input: { path: p } }),
  assetUrl: (p: string) => {
    if (!p) return '';
    const norm = typeof p === 'string' ? p.replace(/\\/g, '/') : p;
    return convertFileSrc(norm);
  },
  readMediaDataUrl: (p: string) => ipcInvoke('fs_read_media_data_url', { input: { path: p } }),
  rename: (src: string, dst: string) => ipcInvoke('fs_rename', { input: { src, dst } }),
  deletePath: (fullPath: string, useTrash?: boolean) => ipcInvoke('fs_delete', { input: { fullPath, useTrash } }),
  compressZip: (fullPath: string) => ipcInvoke('fs_compress_zip', { input: { path: fullPath } }),
  clearSearchCache: () => ipcInvoke('search_clear_cache'),
  searchStart: (input: any) => ipcInvoke('search_start', { input }),
  searchPage: (sessionId: string, offset: number, limit: number) =>
    ipcInvoke('search_get_page', { input: { sessionId, offset, limit } }),
  cancelSearch: (clientId: string) => ipcInvoke('search_cancel', { input: { clientId } }),
  releaseSearch: (sessionId: string) => ipcInvoke('search_release', { input: { sessionId } }),
  copy: (src: string, dst: string, policy?: string) => ipcInvoke('fs_copy', { input: { src, dst, policy } }),
  move: (src: string, dst: string, policy?: string) => ipcInvoke('fs_move', { input: { src, dst, policy } }),
  copyConflicts: (src: string, dst: string) => ipcInvoke('fs_copy_conflicts', { input: { src, dst } }),
  cancelCopy: () => ipcInvoke('fs_cancel_copy'),
  watchDirs: (paths: string[]) => ipcInvoke('fs_watch_dirs', { input: { paths } }),
  onFsChange: (cb: (payload: { path: string }) => void) => {
    let unlisten: UnlistenFn | undefined;
    listen('fs:change', (e: any) => cb(e.payload)).then((u) => {
      unlisten = u;
    });
    return () => {
      if (unlisten) unlisten();
    };
  },
  confirm: (opts: any) => ipcInvoke('dialog_confirm', { opts }),
  info: (opts: any) => ipcInvoke('dialog_info', { opts }),
  onCopyProgress: (cb: (msg: any) => void) => {
    let unlisten: UnlistenFn | undefined;
    listen('fs:copyProgress', (e: any) => cb(e.payload)).then((u) => {
      unlisten = u;
    });
    return () => {
      if (unlisten) unlisten();
    };
  },
  onMenuRefresh: (cb: () => void) => {
    listen('menu:refresh', () => cb()).then(() => {});
  },
  onMenuCopyPath: (cb: () => void) => {
    listen('menu:copyPath', () => cb()).then(() => {});
  },
  closeWindow: () => ipcInvoke('window_close'),
  minimizeWindow: () => ipcInvoke('window_minimize'),
  toggleMaximizeWindow: () => ipcInvoke('window_toggle_maximize'),
  setDockIcon: (iconId: string) => ipcInvoke('set_dock_icon', { iconId }),
  getSystemStats: () => ipcInvoke('system_get_stats'),
  getPathSpace: (path: string) => ipcInvoke('system_get_path_space', { input: { path } }),
  getSystemLocations: () => ipcInvoke('system_get_locations'),
  getDirSize: (path: string) => ipcInvoke('fs_get_dir_size', { input: { path } }),
  analyzeDir: (path: string) => ipcInvoke('fs_analyze_dir', { input: { path } }),
  scanDuplicates: (path: string, minSizeBytes?: number, maxResults?: number) =>
    ipcInvoke('fs_scan_duplicates', { input: { path, minSizeBytes, maxResults } }),
  fsChecksum: (path: string) => ipcInvoke('fs_checksum', { input: { path } }),
  pickFolder: (defaultPath?: string) => ipcInvoke('dialog_pick_folder', { input: { defaultPath } }),

  findReplace: (payload: any, onProgress?: (p: any) => void) =>
    withChannelPayload('fs:replaceProgress', onProgress || null, () =>
      ipcInvoke('fs_find_replace', { input: { payload } }),
    ),

  extractZip: (zipPath: string, entryName?: string) => ipcInvoke('zip_extract', { input: { zipPath, entryName } }),
  fsCompress: (sources: string[], destination: string) => ipcInvoke('fs_compress', { input: { sources, destination } }),
  fsExtract: (archive: string, destination: string) => ipcInvoke('fs_extract', { input: { archive, destination } }),

  compareDirs: (leftPath: string, rightPath: string, onUpdate?: (u: any) => void) =>
    withChannelPayload('compare:update', onUpdate || null, () =>
      ipcInvoke('compare_dirs', { input: { leftPath, rightPath } }),
    ),

  compareFiles: (leftPath: string, rightPath: string) =>
    ipcInvoke('compare_files', { input: { leftPath, rightPath } }),

  gitIsRepo: (dirPath: string) => ipcInvoke('git_is_repo', { input: { dirPath } }),
  gitStatus: (repoPath: string) => ipcInvoke('git_status', { input: { repoPath } }),
  gitLog: (repoPath: string, maxCount?: number, filePath?: string) =>
    ipcInvoke('git_log', { input: { repoPath, maxCount, filePath } }),
  gitDiff: (repoPath: string, ref1?: string, ref2?: string, filePath?: string) =>
    ipcInvoke('git_diff', { input: { repoPath, ref1, ref2, filePath } }),
  gitBlame: (repoPath: string, filePath: string, ref?: string) =>
    ipcInvoke('git_blame', { input: { repoPath, filePath, ref } }),
  gitAdd: (repoPath: string, files: string[]) => ipcInvoke('git_add', { input: { repoPath, files } }),
  gitStageFile: (repoPath: string, filePath: string, stage: boolean) =>
    ipcInvoke('git_stage_file', { input: { repoPath, filePath, stage } }),
  gitRestore: (repoPath: string, filePath: string, staged?: boolean) =>
    ipcInvoke('git_restore', { input: { repoPath, filePath, staged } }),
  gitCommit: (repoPath: string, message: string) => ipcInvoke('git_commit', { input: { repoPath, message } }),
  gitPush: (repoPath: string) => ipcInvoke('git_push', { input: { repoPath } }),
  gitPull: (repoPath: string) => ipcInvoke('git_pull', { input: { repoPath } }),
  gitStash: (repoPath: string, message?: string) => ipcInvoke('git_stash', { input: { repoPath, message } }),
  gitStashPop: (repoPath: string) => ipcInvoke('git_stash_pop', { input: { repoPath } }),
  gitStashList: (repoPath: string) => ipcInvoke('git_stash_list', { input: { repoPath } }),
  gitBranches: (repoPath: string) => ipcInvoke('git_branches', { input: { repoPath } }),
  gitCheckout: (repoPath: string, ref: string, isFile?: boolean) =>
    ipcInvoke('git_checkout', { input: { repoPath, ref, isFile } }),

  remoteListProfiles: () => ipcInvoke('remote_list_profiles'),
  remoteSaveProfile: (profile: any) => ipcInvoke('remote_save_profile', { profile }),
  remoteDeleteProfile: (id: string) => ipcInvoke('remote_delete_profile', { id }),
  remoteTestConnection: (profile: any) => ipcInvoke('remote_test_connection', { profile }),
  remoteConnect: (profile: any) => ipcInvoke('remote_connect', { profile }),
  remoteDisconnect: (profileId: string) => ipcInvoke('remote_disconnect', { profileId }),
  remoteReadDir: (profileId: string, path: string) => ipcInvoke('remote_read_dir', { profileId, path }),
  remoteReadFileText: (profileId: string, path: string) => ipcInvoke('remote_read_file_text', { profileId, path }),
  remoteWriteFileText: (profileId: string, path: string, content: string) =>
    ipcInvoke('remote_write_file_text', { profileId, path, content }),
  remoteMkdir: (profileId: string, path: string) => ipcInvoke('remote_mkdir', { profileId, path }),
  remoteCreateFile: (profileId: string, path: string) => ipcInvoke('remote_create_file', { profileId, path }),
  remoteRename: (profileId: string, srcPath: string, dstPath: string) =>
    ipcInvoke('remote_rename', { profileId, srcPath, dstPath }),
  remoteDelete: (profileId: string, path: string, isDir: boolean) =>
    ipcInvoke('remote_delete', { profileId, path, isDir }),
  remoteDownload: (profileId: string, remotePath: string, localDst: string) =>
    ipcInvoke('remote_download', { profileId, remotePath, localDst }),
  remoteUpload: (profileId: string, localSrc: string, remoteDst: string) =>
    ipcInvoke('remote_upload', { profileId, localSrc, remoteDst }),
};

(window as any).ow = bridge;

// Dynamic import so this module assigns `window.ow` before `app.ts` runs `init()`.
import('./app.ts');
