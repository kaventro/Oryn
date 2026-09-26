// src/modules/stateModels.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { TabState, PaneState, AppState } from './stateModels.ts';

test('TabState initialization and history navigation', () => {
  const tab = new TabState('/initial/path');
  assert.equal(tab.path, '/initial/path');
  assert.equal(tab.cursor, 0);
  assert.equal(tab.filter, '');
  assert.equal(tab.isFilterActive, false);
  assert.equal(tab.sortField, 'name');
  assert.equal(tab.sortAsc, true);
  assert.equal(tab.selectedBases.size, 0);

  // Filter active test
  tab.filter = '  ';
  assert.equal(tab.isFilterActive, false);
  tab.filter = 'test';
  assert.equal(tab.isFilterActive, true);

  // History pushing and popping
  assert.equal(tab.popHistory(), null);
  tab.pushHistory('/initial/path'); // same as current path -> ignored
  assert.equal(tab.popHistory(), null);

  tab.pushHistory('/first/path');
  tab.pushHistory('/second/path');
  assert.equal(tab.popHistory(), '/second/path');
  assert.equal(tab.popHistory(), '/first/path');
  assert.equal(tab.popHistory(), null);

  // Forward navigation
  assert.equal(tab.popFwd(), null);
  tab.pushFwd('/fwd/path');
  assert.equal(tab.popFwd(), '/fwd/path');
  assert.equal(tab.popFwd(), null);

  // Pushing new history clears navFwd
  tab.pushFwd('/fwd/path');
  tab.pushHistory('/another/path');
  assert.equal(tab.popFwd(), null);
});

test('TabState selection methods ignore parent dir and track selected bases', () => {
  const tab = new TabState('/path');

  // Select
  tab.select('..');
  assert.equal(tab.selectedBases.size, 0);
  tab.select('file1.txt');
  assert.equal(tab.selectedBases.has('file1.txt'), true);

  // Deselect
  tab.deselect('file1.txt');
  assert.equal(tab.selectedBases.has('file1.txt'), false);

  // Toggle selection
  tab.toggleSelection('..');
  assert.equal(tab.selectedBases.size, 0);

  tab.toggleSelection('file2.txt');
  assert.equal(tab.selectedBases.has('file2.txt'), true);
  tab.toggleSelection('file2.txt');
  assert.equal(tab.selectedBases.has('file2.txt'), false);

  // Clear selection
  tab.select('a.txt');
  tab.select('b.txt');
  assert.equal(tab.selectedBases.size, 2);
  tab.clearSelection();
  assert.equal(tab.selectedBases.size, 0);
});

test('PaneState tab management and proxy getters/setters', () => {
  const pane = new PaneState('/start');
  assert.equal(pane.tabs.length, 1);
  assert.equal(pane.activeTabIndex, 0);
  assert.equal(pane.path, '/start');

  // Proxies
  pane.path = '/updated';
  assert.equal(pane.activeTab.path, '/updated');

  pane.items = [{ base: 'item.txt' }];
  assert.deepEqual(pane.items, [{ base: 'item.txt' }]);

  pane.filter = 'search';
  assert.equal(pane.filter, 'search');

  pane.cursor = 5;
  assert.equal(pane.cursor, 5);

  pane.listSerial = 42;
  assert.equal(pane.listSerial, 42);

  pane.sortField = 'size';
  assert.equal(pane.sortField, 'size');

  pane.sortAsc = false;
  assert.equal(pane.sortAsc, false);

  // Adding tabs
  const tab2 = pane.addTab('/tab2');
  assert.equal(pane.tabs.length, 2);
  assert.equal(pane.activeTabIndex, 1);
  assert.equal(pane.path, '/tab2');

  const tab3 = pane.addTab(); // uses activeTab.path fallback
  assert.equal(pane.tabs.length, 3);
  assert.equal(pane.activeTabIndex, 2);
  assert.equal(pane.path, '/tab2');

  // Next and prev tab
  pane.nextTab(); // wraps to 0
  assert.equal(pane.activeTabIndex, 0);
  pane.prevTab(); // wraps to 2
  assert.equal(pane.activeTabIndex, 2);
  pane.prevTab(); // wraps to 1
  assert.equal(pane.activeTabIndex, 1);

  // Closing tabs
  assert.equal(pane.closeCurrentTab(), true); // closes tab at index 1
  assert.equal(pane.tabs.length, 2);
  assert.equal(pane.activeTabIndex, 1);

  assert.equal(pane.closeCurrentTab(), true); // closes tab at index 1
  assert.equal(pane.tabs.length, 1);
  assert.equal(pane.activeTabIndex, 0);

  // Cannot close last remaining tab
  assert.equal(pane.closeCurrentTab(), false);
  assert.equal(pane.tabs.length, 1);

  // Single tab next/prev do nothing
  pane.nextTab();
  assert.equal(pane.activeTabIndex, 0);
  pane.prevTab();
  assert.equal(pane.activeTabIndex, 0);
});

test('AppState activePane, activeTab and getPane', () => {
  const app = new AppState();
  assert.equal(app.active, 'left');
  assert.equal(app.activePane, app.left);
  assert.equal(app.activeTab, app.left.activeTab);
  assert.equal(app.getPane('right'), app.right);
  assert.equal(app.copyInProgress, false);
  assert.equal(app.config.useTrash, true);

  app.active = 'right';
  assert.equal(app.activePane, app.right);
  assert.equal(app.activeTab, app.right.activeTab);
});
