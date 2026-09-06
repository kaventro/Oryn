// src/modules/menu/menuTypes.ts

export const MENU_TYPE_VERSION = '1.0';

export interface MenuItemDef {
  id: string;
  label?: string;
  iconKey?: string;
  shortcut?: string;
  checkmark?: boolean;
  isSeparator?: boolean;
  action?: () => void | Promise<void>;
  submenuBuilder?: (subEl: HTMLElement) => void;
  customRender?: (container: HTMLElement) => void;
}

export interface MenuContext {
  side: 'left' | 'right';
  emptyArea: boolean;
  item: any;
  targetDir: string;
  isFile: boolean;
  isDir: boolean;
  getPath: () => Promise<string | null>;
  isMac: boolean;
}

export interface MenuItemEntry {
  el: HTMLElement;
  fn: () => void;
}

export interface IMenuBuilder {
  build(ctx: MenuContext, deps: any): MenuItemDef[];
}
