// src/modules/icons/iconRegistry.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { IconRegistry, defaultIconRegistry } from './iconRegistry.ts';

test('IconRegistry resolves default programming language icons', () => {
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'main.swift' }), 'swift');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'server.go' }), 'go');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'main.rs' }), 'rust');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'app.py' }), 'py');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'index.js' }), 'js');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'types.ts' }), 'ts');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'App.jsx' }), 'react');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'Component.tsx' }), 'react');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'style.css' }), 'css');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'schema.sql' }), 'sql');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'database.bak' }), 'backup');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'config.backup' }), 'backup');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'data.old' }), 'backup');
});

test('IconRegistry resolves special exact files and prefixes', () => {
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'Dockerfile' }), 'docker');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'Dockerfile.production' }), 'docker');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'package.json' }), 'pkgJson');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: '.gitignore' }), 'git');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'LICENSE' }), 'license');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'license.md' }), 'license');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'vite.config.ts' }), 'vite');
});

test('IconRegistry resolves folders and parent navigation', () => {
  assert.equal(defaultIconRegistry.resolveIconKey({ base: '..', isDir: true }), 'parent');
  assert.equal(defaultIconRegistry.resolveIconKey({ base: 'src', isDir: true }), 'folder');
});

test('IconRegistry supports OCP: registering custom extensions dynamically', () => {
  const registry = new IconRegistry();
  assert.equal(registry.resolveIconKey({ base: 'model.onnx' }), 'doc');

  // Dynamically register new format without editing core
  registry.registerExtensions(['onnx', 'pt'], 'code');
  assert.equal(registry.resolveIconKey({ base: 'model.onnx' }), 'code');
  assert.equal(registry.resolveIconKey({ base: 'weights.pt' }), 'code');
});
