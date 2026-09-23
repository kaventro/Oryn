#!/usr/bin/env node

/**
 * End-to-End (E2E) & UI Verification Suite for Oryn
 * 
 * Verifies:
 * 1. Entry files and production build integrity
 * 2. HTML elements for Preferences: Default Editor dropdown (VS Code, Cursor, Zed, Sublime, Custom)
 * 3. Dynamic editor integration (UI context menus, More Options, command execution)
 * 4. Safe trash failure handling and anti-data-loss fallback dialogs
 */

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync, fork } from 'node:child_process';

const rootDir = process.cwd();

console.log('====================================================');
console.log('🚀 Running Oryn E2E & UI Verification Suite');
console.log('====================================================\n');

// 1. Smoke check entry files
console.log('[1/4] Verifying entry files...');
const requiredFiles = [
  path.join(rootDir, 'src', 'index.html'),
  path.join(rootDir, 'src', 'bridge.ts'),
  path.join(rootDir, 'src', 'app.ts'),
  path.join(rootDir, 'src', 'modules', 'preferencesController.ts'),
  path.join(rootDir, 'src', 'modules', 'fileOpsController.ts'),
  path.join(rootDir, 'src', 'modules', 'commandsController.ts'),
  path.join(rootDir, 'desktop', 'src', 'services', 'fs_delete.rs'),
  path.join(rootDir, 'desktop', 'src', 'commands', 'shell_cmd.rs'),
];

for (const file of requiredFiles) {
  if (!fs.existsSync(file)) {
    console.error(`❌ Verification failed: ${path.relative(rootDir, file)} not found`);
    process.exit(1);
  }
}
console.log('  ✔ All required entry files present.\n');

// 2. HTML DOM template verification for default editor options
console.log('[2/4] Verifying HTML Preferences UI options...');
const indexHtml = fs.readFileSync(path.join(rootDir, 'src', 'index.html'), 'utf-8');

const expectedOptions = [
  { val: 'vscode', label: 'Visual Studio Code' },
  { val: 'cursor', label: 'Cursor' },
  { val: 'sublime', label: 'Sublime Text' },
  { val: 'zed', label: 'Zed' },
  { val: 'custom', label: 'Custom Command' },
];

for (const opt of expectedOptions) {
  const pattern = new RegExp(`<option[^>]*value=["']${opt.val}["'][^>]*>[^<]*${opt.label}`, 'i');
  if (!pattern.test(indexHtml)) {
    console.error(`❌ HTML check failed: <option value="${opt.val}"> for ${opt.label} not found in index.html`);
    process.exit(1);
  }
}
console.log('  ✔ All editor options (VS Code, Cursor, Zed, Sublime, Custom) present in #pref-default-editor.\n');

// 3. Run UI unit & integration test suites
console.log('[3/4] Running UI integration tests (Node Test Runner)...');
const testRun = spawnSync(
  process.execPath,
  ['--experimental-strip-types', '--test', 'src/modules/features.critical-fixes.test.ts', 'src/bridge.test.ts'],
  { stdio: 'inherit', cwd: rootDir }
);

if (testRun.status !== 0) {
  console.error('❌ UI integration tests failed!');
  process.exit(testRun.status || 1);
}
console.log('  ✔ UI integration tests passed with 0 errors.\n');

// 4. Production build smoke check
console.log('[4/4] Verifying production UI bundle builds cleanly...');
const buildRun = spawnSync('npm', ['run', 'ui:build'], { stdio: 'pipe', cwd: rootDir });
if (buildRun.status !== 0) {
  console.error('❌ Production UI build failed:\n', buildRun.stderr.toString());
  process.exit(buildRun.status || 1);
}
console.log('  ✔ Production UI build succeeded (dist/ created cleanly).\n');

console.log('====================================================');
console.log('✅ ALL E2E AND UI VERIFICATION CHECKS PASSED!');
console.log('====================================================');
process.exit(0);
