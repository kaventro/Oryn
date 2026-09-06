import type { MenuItemDef, MenuItemEntry } from './menuTypes.ts';
import { MENU_ICONS } from './menuIcons.ts';

export class MenuOverlayView {
  public el: HTMLElement | null = null;
  public items: MenuItemEntry[] = [];
  public selectedIndex = -1;

  constructor(elementId = 'ctx-menu') {
    this.el = typeof document !== 'undefined' ? document.getElementById(elementId) : null;
  }

  isOpen(): boolean {
    return !!this.el && !this.el.classList.contains('hidden');
  }

  hide(): void {
    if (this.el) {
      this.el.classList.add('hidden');
    }
    this.selectedIndex = -1;
    this.items = [];
  }

  navigate(delta: number): void {
    if (this.items.length === 0) return;
    let next = this.selectedIndex;
    for (let i = 0; i < this.items.length; i++) {
      next += delta;
      if (next < 0) next = this.items.length - 1;
      if (next >= this.items.length) next = 0;
      if (!this.items[next].el.classList.contains('ctx-sep')) break;
    }
    this.selectedIndex = next;
    this.renderSelection();
  }

  renderSelection(): void {
    this.items.forEach((item, idx) => {
      item.el.classList.toggle('selected', idx === this.selectedIndex);
    });
  }

  executeSelected(): void {
    if (this.selectedIndex >= 0 && this.selectedIndex < this.items.length) {
      const fn = this.items[this.selectedIndex].fn;
      this.hide();
      fn();
    }
  }

  render(definitions: MenuItemDef[]): void {
    if (!this.el) return;
    this.hide();
    this.el.replaceChildren();

    definitions.forEach((def) => {
      if (def.customRender) {
        def.customRender(this.el!);
        return;
      }

      if (def.isSeparator) {
        const sep = document.createElement('div');
        sep.className = 'ctx-sep';
        sep.setAttribute('role', 'separator');
        this.el!.appendChild(sep);
        this.items.push({ el: sep, fn: () => {} });
        return;
      }

      if (def.submenuBuilder) {
        this.renderSubmenuItem(def);
        return;
      }

      this.renderRegularItem(def);
    });
  }

  private renderRegularItem(def: MenuItemDef): HTMLElement {
    const d = document.createElement('div');
    d.className = 'ctx-item';

    const left = document.createElement('span');
    left.className = 'ctx-item-left';

    if (def.checkmark) {
      const chk = document.createElement('span');
      chk.className = 'ctx-check';
      chk.textContent = '✓';
      left.appendChild(chk);
    } else if (def.iconKey && MENU_ICONS[def.iconKey]) {
      const icon = document.createElement('span');
      icon.className = 'ctx-icon';
      icon.innerHTML = MENU_ICONS[def.iconKey];
      left.appendChild(icon);
    }

    const txt = document.createElement('span');
    txt.textContent = def.label || '';
    left.appendChild(txt);
    d.appendChild(left);

    if (def.shortcut) {
      const sc = document.createElement('span');
      sc.className = 'ctx-shortcut';
      sc.textContent = def.shortcut;
      d.appendChild(sc);
    }

    d.onclick = () => {
      this.hide();
      def.action?.();
    };

    const idx = this.items.length;
    d.addEventListener('mouseenter', () => {
      this.selectedIndex = idx;
      this.renderSelection();
    });

    this.el!.appendChild(d);
    this.items.push({ el: d, fn: () => def.action?.() });
    return d;
  }

  private renderSubmenuItem(def: MenuItemDef): HTMLElement {
    const wrap = document.createElement('div');
    wrap.className = 'ctx-item-parent';

    const parentItem = document.createElement('div');
    parentItem.className = 'ctx-item';

    const left = document.createElement('span');
    left.className = 'ctx-item-left';

    if (def.iconKey && MENU_ICONS[def.iconKey]) {
      const icon = document.createElement('span');
      icon.className = 'ctx-icon';
      icon.innerHTML = MENU_ICONS[def.iconKey];
      left.appendChild(icon);
    }

    const txt = document.createElement('span');
    txt.textContent = def.label || '';
    left.appendChild(txt);

    const arrow = document.createElement('span');
    arrow.className = 'ctx-arrow';
    arrow.textContent = '▶';

    parentItem.append(left, arrow);
    wrap.appendChild(parentItem);

    const submenu = document.createElement('div');
    submenu.className = 'ctx-submenu';
    def.submenuBuilder?.(submenu);
    wrap.appendChild(submenu);

    wrap.addEventListener('mouseenter', () => {
      this.selectedIndex = -1;
      this.renderSelection();

      const winW = typeof window !== 'undefined' ? window.innerWidth : 1200;
      const winH = typeof window !== 'undefined' ? window.innerHeight : 800;
      const wrapRect = wrap.getBoundingClientRect
        ? wrap.getBoundingClientRect()
        : { right: 500, top: 100, bottom: 120 };

      if (wrapRect.right + 200 > winW - 10) {
        submenu.classList.add('flip-left');
      } else {
        submenu.classList.remove('flip-left');
      }

      if (wrapRect.top + 220 > winH - 10) {
        const overflow = (wrapRect.top + 220) - (winH - 10);
        submenu.style.top = `${Math.max(-120, -4 - overflow)}px`;
      } else {
        submenu.style.top = '-4px';
      }
    });

    this.el!.appendChild(wrap);
    return wrap;
  }

  position(x: number, y: number): void {
    if (!this.el) return;
    this.el.classList.remove('hidden');
    const winW = typeof window !== 'undefined' ? window.innerWidth : 1200;
    const winH = typeof window !== 'undefined' ? window.innerHeight : 800;
    const pad = 10;

    this.el.style.maxHeight = '';
    const r = this.el.getBoundingClientRect
      ? this.el.getBoundingClientRect()
      : { left: x, top: y, right: x + 220, bottom: y + 300, width: 220, height: 300 };
    const menuW = r.width || (this.el as any).offsetWidth || 230;
    const menuH = (this.el as any).scrollHeight || r.height || (this.el as any).offsetHeight || 300;

    let nx = x;
    if (nx + menuW > winW - pad) {
      nx = Math.max(pad, winW - pad - menuW);
    }
    if (nx < pad) nx = pad;

    let ny = y;
    const maxAvailableH = winH - 2 * pad;

    if (ny + menuH > winH - pad) {
      const upwardY = winH - pad - menuH;
      if (upwardY >= pad) {
        ny = upwardY;
        this.el.style.maxHeight = `${menuH + 4}px`;
        this.el.style.overflow = 'visible';
      } else {
        ny = pad;
        this.el.style.maxHeight = `${maxAvailableH}px`;
        this.el.style.overflowY = 'auto';
        this.el.style.overflowX = 'hidden';
      }
    } else {
      this.el.style.maxHeight = `${winH - ny - pad}px`;
      this.el.style.overflow = 'visible';
    }

    if (ny < pad) ny = pad;

    this.el.style.left = `${Math.round(nx)}px`;
    this.el.style.top = `${Math.round(ny)}px`;
  }

  showAt(x: number, y: number, definitions: MenuItemDef[]): void {
    this.selectedIndex = -1;
    this.render(definitions);
    this.position(x, y);
  }
}
