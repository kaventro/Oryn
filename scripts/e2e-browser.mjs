#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';

const rootDir = process.cwd();
const required = [
  path.join(rootDir, 'src', 'index.html'),
  path.join(rootDir, 'src', 'app.ts'),
];

for (const file of required) {
  if (!fs.existsSync(file)) {
    console.error(`Frontend smoke check failed: ${path.relative(rootDir, file)} not found`);
    process.exit(1);
  }
}

console.log('✔ Frontend smoke check passed: core entry files present.');
process.exit(0);
