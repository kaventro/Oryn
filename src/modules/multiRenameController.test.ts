// src/modules/multiRenameController.test.ts
import assert from 'node:assert/strict';
import test from 'node:test';

import { MultiRenameController } from './multiRenameController.ts';
import type { AppState, Item } from './stateModels.ts';

function createMockElement(tag = 'div'): any {
  const children: any[] = [];
  const classList = {
    _classes: new Set<string>(),
    add(c: string) { this._classes.add(c); },
    remove(c: string) { this._classes.delete(c); },
    contains(c: string) { return this._classes.has(c); },
  };

  const el: any = {
    tagName: tag.toUpperCase(),
    className: '',
    classList,
    value: '',
    checked: false,
    disabled: false,
    textContent: '',
    innerHTML: '',
    children,
    appendChild(c: any) { children.push(c); return c; },
    replaceChildren(...cs: any[]) { children.length = 0; children.push(...cs); },
    focus() { el._focused = true; },
    select() { el._selected = true; },
    addEventListener(evt: string, fn: any) {
      el._listeners = el._listeners || {};
      el._listeners[evt] = fn;
    },
  };
  return el;
}

test('MultiRenameController computeNewName handles find/replace, regex, case, numbering and tokens', () => {
  const controller = new MultiRenameController({
    state: {} as any,
    api: () => ({}),
    setStatus: () => {},
    loadDir: async () => {},
  });

  // 1. Plain Find & Replace
  const res1 = controller.computeNewName('vacation_photo.JPG', 0, {
    find: 'photo',
    replace: 'trip',
    isRegex: false,
    prefix: '',
    suffix: '',
    caseTransform: 'none',
    numPos: 'none',
    numStart: 1,
    numDigits: 2,
  });
  assert.equal(res1, 'vacation_trip.JPG');

  // 2. Regex Find & Replace and invalid regex fallback
  const res2 = controller.computeNewName('img_123_test.png', 0, {
    find: '\\d+',
    replace: '999',
    isRegex: true,
    prefix: '',
    suffix: '',
    caseTransform: 'none',
    numPos: 'none',
    numStart: 1,
    numDigits: 2,
  });
  assert.equal(res2, 'img_999_test.png');

  // Invalid regex should not throw and return unchanged
  const resInvalid = controller.computeNewName('test.txt', 0, {
    find: '[unclosed',
    replace: 'rep',
    isRegex: true,
    prefix: '',
    suffix: '',
    caseTransform: 'none',
    numPos: 'none',
    numStart: 1,
    numDigits: 2,
  });
  assert.equal(resInvalid, 'test.txt');

  // 3. Case transformations
  const optsCase = (transform: string) => ({
    find: '',
    replace: '',
    isRegex: false,
    prefix: '',
    suffix: '',
    caseTransform: transform,
    numPos: 'none',
    numStart: 1,
    numDigits: 2,
  });

  assert.equal(controller.computeNewName('hello world.TXT', 0, optsCase('lower')), 'hello world.txt');
  assert.equal(controller.computeNewName('hello world.txt', 0, optsCase('upper')), 'HELLO WORLD.TXT');
  assert.equal(controller.computeNewName('hello world.txt', 0, optsCase('title')), 'Hello World.txt');
  assert.equal(controller.computeNewName('hello WORLD.txt', 0, optsCase('first')), 'Hello world.txt');

  // 4. Numbering: prefix and suffix
  const resNumPre = controller.computeNewName('document.pdf', 2, {
    find: '',
    replace: '',
    isRegex: false,
    prefix: '',
    suffix: '',
    caseTransform: 'none',
    numPos: 'prefix',
    numStart: 10,
    numDigits: 3,
  });
  assert.equal(resNumPre, '012_document.pdf');

  const resNumSuf = controller.computeNewName('document.pdf', 0, {
    find: '',
    replace: '',
    isRegex: false,
    prefix: '',
    suffix: '',
    caseTransform: 'none',
    numPos: 'suffix',
    numStart: 1,
    numDigits: 2,
  });
  assert.equal(resNumSuf, 'document_01.pdf');

  // 5. Tokens in prefix and suffix: [N], [E], [C]
  const resTokens = controller.computeNewName('sample.data.csv', 1, {
    find: '',
    replace: '',
    isRegex: false,
    prefix: 'bak_[N]_',
    suffix: '_v[C]',
    caseTransform: 'none',
    numPos: 'none',
    numStart: 1,
    numDigits: 2,
  });
  assert.equal(resTokens, 'bak_sample.data_sample.data_v02.csv');

  // 6. File without extension
  const resNoExt = controller.computeNewName('Makefile', 0, {
    find: 'Make',
    replace: 'Build',
    isRegex: false,
    prefix: '',
    suffix: '',
    caseTransform: 'none',
    numPos: 'none',
    numStart: 1,
    numDigits: 2,
  });
  assert.equal(resNoExt, 'Buildfile');
});

test('MultiRenameController open, hide, resetInputs, and updatePreview with conflicts', () => {
  const elements: Record<string, any> = {};
  const getEl = (id: string) => {
    if (!elements[id]) elements[id] = createMockElement('div');
    return elements[id];
  };

  const origDoc = globalThis.document;
  try {
    (globalThis as any).document = {
      getElementById: (id: string) => getEl(id),
      createElement: (tag: string) => createMockElement(tag),
      createDocumentFragment: () => {
        const children: any[] = [];
        return {
          children,
          appendChild(c: any) { children.push(c); return c; },
        };
      },
    };

    let statusMsg = '';
    let focusedList = false;

    const mockState: any = {
      active: 'left',
      left: {
        path: '/photos',
        items: [
          { base: '..' },
          { base: 'img1.png' },
          { base: 'img2.png' },
          { base: 'img3.png' },
        ],
        activeTab: {
          selectedBases: new Set(['img1.png', 'img2.png']),
        },
      },
    };

    const controller = new MultiRenameController({
      state: mockState,
      api: () => ({}),
      setStatus: (msg: string) => { statusMsg = msg; },
      loadDir: async () => {},
      focusActiveList: () => { focusedList = true; },
    });

    // 1. Open without path sets error status
    mockState.left.path = '';
    controller.open();
    assert.equal(statusMsg, 'No active directory.');
    assert.equal(controller.isOpen, false);

    // 2. Open with no valid files sets error status
    mockState.left.path = '/photos';
    mockState.left.items = [{ base: '..' }];
    controller.open();
    assert.equal(statusMsg, 'No files to rename.');

    // 3. Open with selected items
    mockState.left.items = [
      { base: '..' },
      { base: 'img1.png' },
      { base: 'img2.png' },
      { base: 'img3.png' },
    ];
    controller.open();
    assert.equal(controller.isOpen, true);
    assert.equal(controller.items.length, 2); // only selected items img1 and img2
    assert.equal(getEl('multi-rename-overlay').classList._classes.has('hidden'), false);
    assert.equal(getEl('mr-find')._focused, true);

    // 3b. Open with empty selectedBases includes all non-parent items
    mockState.left.activeTab.selectedBases.clear();
    controller.open();
    assert.equal(controller.items.length, 3);

    // 4. Update preview with name conflict
    getEl('mr-find').value = 'img.*';
    getEl('mr-regex').checked = true;
    getEl('mr-replace').value = 'duplicate';
    controller.updatePreview();

    const previewList = getEl('mr-preview-list');
    assert.equal(previewList.children.length, 1); // DocumentFragment was appended
    const rows = previewList.children[0].children;
    assert.equal(rows.length, 3);
    // Both new names are same_name.png, so conflict should be marked
    assert.ok(rows[1].className.includes('mr-row--conflict'));
    assert.ok(rows[1].innerHTML.includes('Conflict'));

    // 5. Hide closes overlay and focuses list
    controller.hide();
    assert.equal(controller.isOpen, false);
    assert.equal(getEl('multi-rename-overlay').classList._classes.has('hidden'), true);
    assert.equal(focusedList, true);
  } finally {
    globalThis.document = origDoc;
  }
});

test('MultiRenameController apply renames files and setup registers DOM event listeners', async () => {
  const elements: Record<string, any> = {};
  const getEl = (id: string) => {
    if (!elements[id]) elements[id] = createMockElement('div');
    return elements[id];
  };

  const origDoc = globalThis.document;
  try {
    (globalThis as any).document = {
      getElementById: (id: string) => getEl(id),
      createElement: (tag: string) => createMockElement(tag),
      createDocumentFragment: () => ({
        children: [],
        appendChild(c: any) { (this as any).children.push(c); return c; },
      }),
    };

    const renamedPairs: Array<{ src: string; dst: string }> = [];
    const mockApi = {
      pathJoin: async (dir: string, base: string) => `${dir}/${base}`,
      rename: async (src: string, dst: string) => {
        if (src.includes('error')) throw new Error('Permission denied');
        renamedPairs.push({ src, dst });
      },
    };

    let statusMsg = '';
    let loadedSide = '';

    const mockState: any = {
      active: 'left',
      left: {
        path: '/files',
        items: [
          { base: 'file1.txt' },
          { base: 'error_file.txt' },
        ],
        activeTab: { selectedBases: new Set() },
      },
    };

    const controller = new MultiRenameController({
      state: mockState,
      api: () => mockApi,
      setStatus: (msg: string) => { statusMsg = msg; },
      loadDir: async (side) => { loadedSide = side; },
    });

    controller.items = [
      { base: 'file1.txt' },
      { base: 'error_file.txt' },
    ];

    // Setup DOM listeners
    controller.setup();

    // Set replace options: prefix 'new_'
    getEl('mr-prefix').value = 'new_';

    // Run apply
    await controller.apply();

    assert.equal(renamedPairs.length, 1);
    assert.equal(renamedPairs[0].src, '/files/file1.txt');
    assert.equal(renamedPairs[0].dst, '/files/new_file1.txt');
    assert.equal(loadedSide, 'left');
    assert.ok(statusMsg.includes('1 renamed, 1 errors'));

    // Apply with no changes returns early
    controller.items = [{ base: 'unchanged.txt' }];
    getEl('mr-prefix').value = '';
    await controller.apply();
    assert.equal(controller.isOpen, false);

    // Setup event listeners: close button click
    controller.isOpen = true;
    getEl('mr-close')._listeners['click']();
    assert.equal(controller.isOpen, false);

    // Backdrop click
    controller.isOpen = true;
    const overlay = getEl('multi-rename-overlay');
    overlay._listeners['click']({ target: overlay });
    assert.equal(controller.isOpen, false);
  } finally {
    globalThis.document = origDoc;
  }
});
