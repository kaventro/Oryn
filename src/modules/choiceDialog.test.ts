import test from 'node:test';
import assert from 'node:assert/strict';
import { showChoiceDialog } from './choiceDialog.ts';

function makeEl(tag: string): any {
  const el: any = {
    tagName: tag.toUpperCase(),
    className: '',
    id: '',
    style: {} as Record<string, string>,
    textContent: '',
    children: [] as any[],
    onclick: null as any,
    onpointerdown: null as any,
    classList: {
      _set: new Set<string>(),
      add(c: string) { this._set.add(c); el.className = [...this._set].join(' '); },
      remove(c: string) { this._set.delete(c); el.className = [...this._set].join(' '); },
      contains(c: string) { return this._set.has(c); },
    },
    setAttribute() {},
    addEventListener() {},
    querySelector(sel: string) {
      const id = sel.startsWith('#') ? sel.slice(1) : '';
      const walk = (n: any): any => {
        if (n.id === id) return n;
        for (const ch of n.children || []) {
          const hit = walk(ch);
          if (hit) return hit;
        }
        return null;
      };
      return walk(el);
    },
    replaceChildren(...nodes: any[]) { el.children = nodes; },
    appendChild(child: any) { el.children.push(child); return child; },
    append(...nodes: any[]) { el.children.push(...nodes); },
    focus() {},
  };
  return el;
}

test('showChoiceDialog Delete pointerdown confirms and a later overlay click does not cancel', async () => {
  const keyHandlers: Array<[string, EventListener]> = [];
  const bodyChildren: any[] = [];
  (globalThis as any).document = {
    body: {
      appendChild(el: any) { bodyChildren.push(el); return el; },
    },
    createElement: (tag: string) => makeEl(tag),
    addEventListener(type: string, fn: EventListener) { keyHandlers.push([type, fn]); },
    removeEventListener(type: string, fn: EventListener) {
      const i = keyHandlers.findIndex((h) => h[0] === type && h[1] === fn);
      if (i >= 0) keyHandlers.splice(i, 1);
    },
  };

  const pending = showChoiceDialog({
    title: 'Delete',
    message: 'Move "a.txt" to Trash?',
    choices: [
      { label: 'Cancel', value: 'cancel' },
      { label: 'Delete', value: 'ok', primary: true, danger: true },
    ],
  });

  const overlay = bodyChildren[0];
  const actions = overlay.querySelector('#choice-actions');
  const deleteBtn = actions.children.find((b: any) => b.textContent === 'Delete');
  assert.ok(deleteBtn);
  deleteBtn.onpointerdown({ preventDefault() {}, stopPropagation() {} });
  assert.equal(await pending, 'ok');

  overlay.onclick?.({ target: overlay });
});
