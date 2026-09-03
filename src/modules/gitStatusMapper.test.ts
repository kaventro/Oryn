import test from 'node:test';
import assert from 'node:assert/strict';
import {
  applyGitStatusToItems,
  clearGitStatusCache,
  fetchGitSnapshot,
  gitTagFromEntry,
} from './gitStatusMapper.ts';

test('gitTagFromEntry maps staged, untracked, modified, and deleted', () => {
  assert.equal(gitTagFromEntry({ index: 'A', worktree: ' ' }), 'A');
  assert.equal(gitTagFromEntry({ index: 'M', worktree: ' ' }), 'M');
  assert.equal(gitTagFromEntry({ index: ' ', worktree: '?' }), '?');
  assert.equal(gitTagFromEntry({ index: '?', worktree: '?' }), '?');
  assert.equal(gitTagFromEntry({ index: ' ', worktree: 'D' }), 'D');
  assert.equal(gitTagFromEntry({ index: 'D', worktree: ' ' }), 'M');
});

test('applyGitStatusToItems badges nested files onto the parent folder at repo root', () => {
  const items: Array<{ base: string; gitStatus?: string | null }> = [
    { base: 'src' },
    { base: 'README.md' },
    { base: 'clean.txt' },
  ];
  applyGitStatusToItems('/repo', items, {
    git: { isRepo: true, root: '/repo', branch: 'main', ahead: 0, behind: 0 },
    files: [
      { file: 'src/app.ts', index: ' ', worktree: 'M' },
      { file: 'README.md', index: 'A', worktree: ' ' },
    ],
  });
  assert.equal(items[0].gitStatus, 'M');
  assert.equal(items[1].gitStatus, 'A');
  assert.equal(items[2].gitStatus, null);
});

test('applyGitStatusToItems only maps files under the current directory prefix', () => {
  const items: Array<{ base: string; gitStatus?: string | null }> = [
    { base: 'App.swift' },
    { base: 'src' },
  ];
  applyGitStatusToItems('/repo/ios', items, {
    git: { isRepo: true, root: '/repo', branch: 'main', ahead: 1, behind: 0 },
    files: [
      { file: 'README.md', index: ' ', worktree: 'M' },
      { file: 'ios/App.swift', index: ' ', worktree: '?' },
    ],
  });
  assert.equal(items[0].gitStatus, '?');
  assert.equal(items[1].gitStatus, null);
});

test('fetchGitSnapshot returns branch meta and caches by repo root', async () => {
  clearGitStatusCache();
  let statusCalls = 0;
  const api = {
    gitIsRepo: async () => ({ ok: true, root: '/repo' }),
    gitStatus: async () => {
      statusCalls += 1;
      return { ok: true, branch: 'feature', ahead: 2, behind: 0, files: [] };
    },
  };
  const a = await fetchGitSnapshot(api, '/repo/ios');
  const b = await fetchGitSnapshot(api, '/repo');
  assert.equal(a.git?.branch, 'feature');
  assert.equal(a.git?.ahead, 2);
  assert.equal(b.git?.branch, 'feature');
  assert.equal(statusCalls, 1);
});
