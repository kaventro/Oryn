// src/modules/ctxMenuController.test.ts
import test from 'node:test';
import assert from 'node:assert/strict';
import { CtxMenuController } from './ctxMenuController.ts';

test('CtxMenuController initializes and manages open/hide state', () => {
  const mockMenu = {
    classList: {
      _classes: new Set(['hidden']),
      contains(c: string) { return this._classes.has(c); },
      add(c: string) { this._classes.add(c); },
      remove(c: string) { this._classes.delete(c); },
      toggle(c: string, force?: boolean) {
        if (force !== undefined) {
          if (force) this._classes.add(c); else this._classes.delete(c);
        } else {
          if (this._classes.has(c)) this._classes.delete(c); else this._classes.add(c);
        }
      },
    },
    children: [] as any[],
    replaceChildren() { this.children = []; },
    appendChild(child: any) { this.children.push(child); return child; },
    style: {} as any,
    getBoundingClientRect: () => ({ left: 10, top: 10, right: 250, bottom: 300, width: 240, height: 290 }),
  };

  (globalThis as any).document = {
    getElementById(id: string) {
      if (id === 'ctx-menu') return mockMenu;
      return null;
    },
    createElement(tag: string) {
      return {
        className: '',
        textContent: '',
        style: {},
        innerHTML: '',
        classList: {
          _classes: new Set<string>(),
          contains(c: string) { return this._classes.has(c); },
          add(c: string) { this._classes.add(c); },
          remove(c: string) { this._classes.delete(c); },
          toggle(c: string, force?: boolean) {
            if (force !== undefined) {
              if (force) this._classes.add(c); else this._classes.delete(c);
            } else {
              if (this._classes.has(c)) this._classes.delete(c); else this._classes.add(c);
            }
          },
        },
        children: [] as any[],
        listeners: {} as Record<string, any>,
        appendChild(child: any) { (this.children as any[]).push(child); return child; },
        append(...children: any[]) { (this.children as any[]).push(...children); },
        setAttribute() {},
        addEventListener(evt: string, fn: any) { this.listeners[evt] = fn; },
        getBoundingClientRect: () => ({ left: 0, top: 0, right: 200, bottom: 200, width: 200, height: 200 }),
      };
    },
  };

  const state = {
    left: { path: '/home/user', sortField: 'name', sortAsc: true },
    right: { path: '/tmp', sortField: 'name', sortAsc: true },
    active: 'left',
  };

  const ctx = new CtxMenuController({
    api: () => ({}),
    state,
    setStatus: () => {},
  });

  assert.equal(ctx.isOpen(), false);
  ctx.hide();
  assert.equal(ctx.isOpen(), false);
});

test('CtxMenuController builds Finder-style submenus and items with shortcut badges and icons', () => {
  let modeSet = '';
  const viewController = {
    getMode: () => 'list',
    setMode: (m: string) => { modeSet = m; },
  };

  let vsCodePath = '';
  const api = () => ({
    openVSCode: async (p: string) => { vsCodePath = p; },
    clipboardWrite: async (t: string) => {},
  });

  const state = {
    left: { path: '/home/user/project', sortField: 'name', sortAsc: true },
    right: { path: '/tmp', sortField: 'name', sortAsc: true },
    active: 'left',
  };

  const createdItems: any[] = [];
  const origCreate = (globalThis as any).document.createElement;
  (globalThis as any).document.createElement = (tag: string) => {
    const el = origCreate(tag);
    createdItems.push(el);
    return el;
  };

  let previewPath = '';
  const ctx = new CtxMenuController({
    api,
    state,
    setStatus: () => {},
    viewController,
    getFilteredSelection: () => ({ item: { base: 'main.rs', isDir: false } }),
    fullPath: async (_side, item) => `/home/user/project/${item.base}`,
    onPreviewSelected: (fp) => { previewPath = fp; },
  });

  ctx.show(100, 100, 'left', false, { base: 'main.rs', isDir: false }, '/home/user/project');
  assert.equal(ctx.isOpen(), true);

  const hasShortcut = createdItems.some((el) => el.className === 'ctx-shortcut' && el.textContent);
  assert.ok(hasShortcut, 'Context menu should render shortcut badges');

  const hasIcon = createdItems.some((el) => el.className === 'ctx-icon' && el.innerHTML.includes('<svg'));
  assert.ok(hasIcon, 'Context menu should render SVG icons');

  const previewItem = createdItems.find((el) => el.textContent === 'Quick Look / Preview');
  assert.ok(previewItem, 'Should have top-level Quick Look / Preview item in context menu');

  // Verify the removed items are NOT in right-click context menu
  const hasView = createdItems.some((el) => el.textContent === 'View');
  const hasSort = createdItems.some((el) => el.textContent === 'Sort By');
  const hasDisk = createdItems.some((el) => el.textContent === 'Analyze Disk Space…');
  const hasDups = createdItems.some((el) => el.textContent === 'Find Duplicate Files…');
  const hasInfo = createdItems.some((el) => el.textContent === 'Get Info / Properties');
  assert.equal(hasView, false, 'Right-click menu should not have View');
  assert.equal(hasSort, false, 'Right-click menu should not have Sort By');
  assert.equal(hasDisk, false, 'Right-click menu should not have Analyze Disk Space');
  assert.equal(hasDups, false, 'Right-click menu should not have Find Duplicate Files');
  assert.equal(hasInfo, false, 'Right-click menu should not have Get Info / Properties');
});

test('CtxMenuController showMoreMenu includes View, Sort By, Get Info, Disk Space, and Duplicate Finder', () => {
  const createdItems: any[] = [];
  const origCreate = (globalThis as any).document.createElement;
  (globalThis as any).document.createElement = (tag: string) => {
    const el = origCreate(tag);
    createdItems.push(el);
    return el;
  };

  const state = {
    left: { path: '/home/user/project', sortField: 'name', sortAsc: true },
    right: { path: '/tmp', sortField: 'name', sortAsc: true },
    active: 'left',
  };

  const ctx = new CtxMenuController({
    api: () => ({}),
    state,
    setStatus: () => {},
    viewController: { getMode: () => 'list', setMode: () => {} },
    getFilteredSelection: () => ({ item: { base: 'main.rs', isDir: false } }),
    fullPath: async (_side, item) => `/home/user/project/${item.base}`,
  });

  ctx.showMoreMenu(100, 100, 'left');
  assert.equal(ctx.isOpen(), true);

  const hasView = createdItems.some((el) => el.textContent === 'View');
  const hasSort = createdItems.some((el) => el.textContent === 'Sort By');
  const hasDisk = createdItems.some((el) => el.textContent === 'Analyze Disk Space…');
  const hasDups = createdItems.some((el) => el.textContent === 'Find Duplicate Files…');
  const hasInfo = createdItems.some((el) => el.textContent === 'Get Info / Properties');

  assert.ok(hasView, 'More Options menu must have View submenu');
  assert.ok(hasSort, 'More Options menu must have Sort By submenu');
  assert.ok(hasDisk, 'More Options menu must have Analyze Disk Space…');
  assert.ok(hasDups, 'More Options menu must have Find Duplicate Files…');
  assert.ok(hasInfo, 'More Options menu must have Get Info / Properties');
});

test('CtxMenuController showMoreMenu prevents dual highlight and flips submenu to left near screen edge', () => {
  const state = {
    left: { path: '/home/user/project', sortField: 'name', sortAsc: true },
    right: { path: '/tmp', sortField: 'name', sortAsc: true },
    active: 'left',
  };

  const ctx = new CtxMenuController({
    api: () => ({}),
    state,
    setStatus: () => {},
    viewController: { getMode: () => 'list', setMode: () => {} },
  });

  ctx.showMoreMenu(1100, 100, 'left');
  assert.equal(ctx.isOpen(), true);
  assert.equal((ctx.el as any)?.style.overflow, 'visible', 'Menu must have overflow visible so submenus are not clipped');

  // Verify selecting an item then hovering over a submenu parent clears selectedIndex
  ctx.navigate(1); // Selects first selectable item
  assert.ok(ctx.selectedIndex >= 0);

  const wrap = (ctx.el as any)?.children.find((c: any) => c.className === 'ctx-item-parent');
  assert.ok(wrap, 'Should find ctx-item-parent');

  // Simulate hover on submenu parent near right edge of screen
  wrap.getBoundingClientRect = () => ({ right: 1150, top: 100, bottom: 130 });
  (globalThis as any).window = { innerWidth: 1200, innerHeight: 800 };

  const mouseenterListener = wrap._listeners?.mouseenter || wrap.listeners?.mouseenter;
  if (mouseenterListener) {
    mouseenterListener();
  }

  assert.equal(ctx.selectedIndex, -1, 'Hovering submenu parent must clear selectedIndex to prevent dual highlight');
  const submenu = wrap.children.find((c: any) => c.className?.includes('ctx-submenu'));
  assert.ok(submenu, 'Must contain submenu');
  assert.ok(submenu.classList.contains('flip-left'), 'Submenu must flip left when near right window boundary');
});

