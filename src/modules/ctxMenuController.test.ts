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
    replaceChildren() {},
    appendChild(child: any) {},
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
        appendChild(child: any) { (this.children as any[]).push(child); return child; },
        append(...children: any[]) { (this.children as any[]).push(...children); },
        setAttribute() {},
        addEventListener() {},
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
});
