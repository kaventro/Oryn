export interface GitFileEntry {
  file?: string;
  index?: string;
  worktree?: string;
}

export interface GitPaneInfo {
  isRepo: true;
  root: string;
  branch: string;
  ahead: number;
  behind: number;
}

export interface GitSnapshot {
  git: GitPaneInfo | null;
  files: GitFileEntry[];
}

const CACHE_TTL_MS = 2500;
const snapshotCache = new Map<string, { at: number; snapshot: GitSnapshot }>();

export function clearGitStatusCache(): void {
  snapshotCache.clear();
}

export function gitTagFromEntry(f: GitFileEntry): string {
  const index = f.index ?? ' ';
  const worktree = f.worktree ?? ' ';
  if (index !== ' ' && index !== '?') {
    return index === 'A' ? 'A' : 'M';
  }
  if (worktree === '?') return '?';
  if (worktree === 'D') return 'D';
  return 'M';
}

export function applyGitStatusToItems(
  dirPath: string,
  items: Array<{ base: string; gitStatus?: string | null }>,
  snapshot: GitSnapshot,
): void {
  if (!snapshot.git) {
    items.forEach((it) => {
      if (it.base !== '..') it.gitStatus = null;
    });
    return;
  }

  const repoRoot = snapshot.git.root;
  const relToRepo = dirPath.startsWith(repoRoot)
    ? dirPath.slice(repoRoot.length).replace(/^[/\\]+/, '')
    : '';
  const prefix = relToRepo ? `${relToRepo.replace(/[/\\]+$/, '')}/` : '';

  const fileStatusMap = new Map<string, string>();
  (snapshot.files || []).forEach((f) => {
    let fPath = String(f.file || '');
    if (!fPath) return;
    if (prefix) {
      if (!fPath.startsWith(prefix)) return;
      fPath = fPath.slice(prefix.length);
    }
    const baseName = fPath.split(/[/\\]/)[0];
    if (!baseName) return;
    fileStatusMap.set(baseName, gitTagFromEntry(f));
  });

  items.forEach((it) => {
    if (it.base !== '..') {
      it.gitStatus = fileStatusMap.get(it.base) || null;
    }
  });
}

export async function fetchGitSnapshot(api: any, dirPath: string): Promise<GitSnapshot> {
  const empty: GitSnapshot = { git: null, files: [] };
  if (!dirPath || typeof api?.gitIsRepo !== 'function') return empty;

  try {
    const repo = await api.gitIsRepo(dirPath);
    if (!repo || !repo.ok || !repo.root) return empty;

    const cached = snapshotCache.get(repo.root);
    if (cached && Date.now() - cached.at < CACHE_TTL_MS) {
      return cached.snapshot;
    }

    if (typeof api.gitStatus !== 'function') return empty;
    const status = await api.gitStatus(repo.root);
    if (!status || !status.ok) return empty;

    const snapshot: GitSnapshot = {
      git: {
        isRepo: true,
        root: repo.root,
        branch: status.branch,
        ahead: status.ahead || 0,
        behind: status.behind || 0,
      },
      files: Array.isArray(status.files) ? status.files : [],
    };
    snapshotCache.set(repo.root, { at: Date.now(), snapshot });
    return snapshot;
  } catch {
    return empty;
  }
}
