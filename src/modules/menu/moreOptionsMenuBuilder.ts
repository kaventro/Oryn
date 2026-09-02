import type { MenuItemDef, MenuContext } from './menuTypes.ts';

export class MoreOptionsMenuBuilder {
  build(ctx: MenuContext, deps: any, closeMenu: () => void): MenuItemDef[] {
    const items: MenuItemDef[] = [];
    const { side, item, targetDir, getPath, isMac } = ctx;

    const addSep = () => items.push({ id: `sep-${items.length}`, isSeparator: true });

    // 1. View > (As List, As Columns, As Grid)
    items.push({
      id: 'view',
      label: 'View',
      iconKey: 'view',
      submenuBuilder: (subEl) => {
        const currentMode = deps.viewController?.getMode?.() || 'list';
        const modes: { id: string; label: string; shortcut: string }[] = [
          { id: 'list', label: 'As List', shortcut: isMac ? '⌘1' : 'Ctrl+1' },
          { id: 'columns', label: 'As Columns', shortcut: isMac ? '⌘3' : 'Ctrl+3' },
          { id: 'grid', label: 'As Icons / Grid', shortcut: isMac ? '⌘2' : 'Ctrl+2' },
        ];
        modes.forEach((m) => {
          const subItem = document.createElement('div');
          subItem.className = 'ctx-item';
          const left = document.createElement('span');
          left.className = 'ctx-item-left';
          if (currentMode === m.id) {
            const chk = document.createElement('span');
            chk.className = 'ctx-check';
            chk.textContent = '✓';
            left.appendChild(chk);
          }
          const txt = document.createElement('span');
          txt.textContent = m.label;
          left.appendChild(txt);
          subItem.appendChild(left);

          const sc = document.createElement('span');
          sc.className = 'ctx-shortcut';
          sc.textContent = m.shortcut;
          subItem.appendChild(sc);

          subItem.onclick = () => {
            closeMenu();
            deps.viewController?.setMode?.(m.id);
          };
          subEl.appendChild(subItem);
        });
      },
    });

    // 2. Sort By > (Name, Date Modified, Size, Kind)
    items.push({
      id: 'sort',
      label: 'Sort By',
      iconKey: 'sort',
      submenuBuilder: (subEl) => {
        const pane = deps.state[side];
        const curSort = pane?.sortField || 'name';
        const sortOpts: { id: string; label: string }[] = [
          { id: 'name', label: 'Name' },
          { id: 'date', label: 'Date Modified' },
          { id: 'size', label: 'Size' },
          { id: 'ext', label: 'Kind / Type' },
        ];
        sortOpts.forEach((s) => {
          const subItem = document.createElement('div');
          subItem.className = 'ctx-item';
          const left = document.createElement('span');
          left.className = 'ctx-item-left';
          if (curSort === s.id) {
            const chk = document.createElement('span');
            chk.className = 'ctx-check';
            chk.textContent = pane?.sortAsc ? '✓ ▲' : '✓ ▼';
            left.appendChild(chk);
          }
          const txt = document.createElement('span');
          txt.textContent = s.label;
          left.appendChild(txt);
          subItem.appendChild(left);
          subItem.onclick = () => {
            closeMenu();
            if (pane) {
              if (pane.sortField === s.id) {
                pane.sortAsc = !pane.sortAsc;
              } else {
                pane.sortField = s.id;
                pane.sortAsc = true;
              }
              if (deps.loadDir) void deps.loadDir(side);
            }
          };
          subEl.appendChild(subItem);
        });
      },
    });

    addSep();

    // 3. Get Info / Properties
    items.push({
      id: 'properties',
      label: 'Get Info / Properties',
      iconKey: 'info',
      shortcut: isMac ? '⌥↵' : 'Alt+Enter',
      action: async () => {
        deps.state.active = side;
        const fp = await getPath();
        if (deps.showProperties) {
          if (fp) deps.showProperties(side, fp);
          else deps.showProperties(side);
        }
      },
    });

    // 4. Analyze Disk Space…
    items.push({
      id: 'diskSpace',
      label: 'Analyze Disk Space…',
      iconKey: 'disk',
      shortcut: isMac ? '⌘⇧D' : 'Ctrl+Shift+D',
      action: async () => {
        deps.state.active = side;
        const fp = await getPath();
        const target = (item && item.isDir && fp) ? fp : targetDir;
        deps.diskSpaceController?.open?.(target);
      },
    });

    // 5. Find Duplicate Files…
    items.push({
      id: 'duplicateFinder',
      label: 'Find Duplicate Files…',
      iconKey: 'view',
      action: async () => {
        deps.state.active = side;
        const fp = await getPath();
        const target = (item && item.isDir && fp) ? fp : targetDir;
        deps.duplicateFinderController?.open?.(target);
      },
    });

    addSep();

    // 6. New Folder / New File
    items.push({
      id: 'mkdir',
      label: 'New Folder…',
      iconKey: 'folder',
      shortcut: 'F7',
      action: () => {
        deps.state.active = side;
        deps.runCommand?.('mkdir', targetDir);
      },
    });

    items.push({
      id: 'newFile',
      label: 'New File…',
      iconKey: 'filePlus',
      shortcut: '⇧F7',
      action: () => {
        deps.state.active = side;
        deps.runCommand?.('newFile', targetDir);
      },
    });

    addSep();

    // 7. Open in Terminal
    items.push({
      id: 'terminal',
      label: 'Open in Terminal',
      iconKey: 'terminal',
      shortcut: isMac ? '⌃`' : 'Ctrl+`',
      action: async () => {
        deps.state.active = side;
        let target = targetDir;
        if (item && item.isDir) {
          target = (await getPath()) || targetDir;
        }
        deps.openTerminal?.(target);
      },
    });

    // 8. Open in VS Code
    items.push({
      id: 'vscode',
      label: 'Open in VS Code',
      iconKey: 'vscode',
      action: async () => {
        deps.state.active = side;
        const fp = (await getPath()) || targetDir;
        if (!fp) return;
        try {
          await deps.api().openVSCode(fp);
          deps.setStatus('Opened in VS Code');
        } catch (e: any) {
          deps.setStatus(e?.message || 'VS Code failed');
        }
      },
    });

    // 9. Copy Full Path
    items.push({
      id: 'copyPath',
      label: 'Copy Full Path',
      iconKey: 'copy',
      shortcut: isMac ? '⌥⌘C' : 'Ctrl+⇧+C',
      action: async () => {
        deps.state.active = side;
        const fp = (await getPath()) || targetDir;
        if (!fp) return;
        await deps.api().clipboardWrite(fp);
        deps.setStatus('Path copied to clipboard.');
      },
    });

    return items;
  }
}
