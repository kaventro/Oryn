import type { MenuItemDef, MenuContext } from './menuTypes.ts';
import { safeColor } from '../formatUtils.ts';
import { MENU_ICONS } from './menuIcons.ts';

export class ContextMenuBuilder {
  build(ctx: MenuContext, deps: any, closeMenu: () => void): MenuItemDef[] {
    const items: MenuItemDef[] = [];
    const { side, item, targetDir, isFile, isDir, getPath, isMac } = ctx;

    const addSep = () => items.push({ id: `sep-${items.length}`, isSeparator: true });

    // 1. New Folder / New File
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

    // 2. Open in Terminal
    items.push({
      id: 'terminal',
      label: 'Open in Terminal',
      iconKey: 'terminal',
      shortcut: isMac ? '⌃`' : 'Ctrl+`',
      action: async () => {
        deps.state.active = side;
        let target = targetDir;
        if (isDir) {
          target = (await getPath()) || targetDir;
        }
        deps.openTerminal?.(target);
      },
    });

    // 3. Open in VS Code
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

    // 4. Copy Full Path
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

    // 5. Open With >
    items.push({
      id: 'openWith',
      label: 'Open With',
      iconKey: 'openWith',
      submenuBuilder: (subEl) => {
        const makeSub = (lbl: string, clickFn: () => void, iconKey?: string, shortcut?: string) => {
          const s = document.createElement('div');
          s.className = 'ctx-item';
          const left = document.createElement('span');
          left.className = 'ctx-item-left';
          if (iconKey && MENU_ICONS[iconKey]) {
            const icon = document.createElement('span');
            icon.className = 'ctx-icon';
            icon.innerHTML = MENU_ICONS[iconKey];
            left.appendChild(icon);
          }
          const txt = document.createElement('span');
          txt.textContent = lbl;
          left.appendChild(txt);
          s.appendChild(left);

          if (shortcut) {
            const sc = document.createElement('span');
            sc.className = 'ctx-shortcut';
            sc.textContent = shortcut;
            s.appendChild(sc);
          }

          s.onclick = () => {
            closeMenu();
            clickFn();
          };
          subEl.appendChild(s);
        };

        makeSub(
          'Default Application',
          async () => {
            const fp = (await getPath()) || targetDir;
            if (fp) void deps.api().openPath(fp);
          },
          'open',
        );

        makeSub(
          'Visual Studio Code',
          async () => {
            const fp = (await getPath()) || targetDir;
            if (fp) {
              try {
                await deps.api().openVSCode(fp);
                deps.setStatus('Opened in VS Code');
              } catch (e: any) {
                deps.setStatus(e?.message || 'VS Code failed');
              }
            }
          },
          'vscode',
        );

        makeSub(
          'Terminal',
          async () => {
            const fp = (await getPath()) || targetDir;
            if (fp && deps.openTerminal) deps.openTerminal(fp);
          },
          'terminal',
        );

        if (isFile) {
          makeSub(
            'Quick Look / Preview',
            async () => {
              const fp = await getPath();
              if (fp && deps.onPreviewSelected) {
                deps.onPreviewSelected(fp, item);
              } else if (fp && deps.commandsController?.openViewer) {
                void deps.commandsController.openViewer(fp, item);
              }
            },
            'eye',
            'F3 / ␣',
          );
        }
      },
    });

    addSep();

    if (isFile || isDir) {
      items.push({
        id: 'open',
        label: 'Open',
        iconKey: 'open',
        shortcut: '↵',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (fp) {
            try {
              const apiObj = typeof deps.api === 'function' ? deps.api() : deps.api;
              if (apiObj?.openPath) await apiObj.openPath(fp);
            } catch (_) {}
          } else {
            deps.openSelected?.(side);
          }
        },
      });
    }

    if (isFile) {
      items.push({
        id: 'preview',
        label: 'Quick Look / Preview',
        iconKey: 'eye',
        shortcut: 'F3 / ␣',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (fp) {
            if (deps.onPreviewSelected) {
              deps.onPreviewSelected(fp, item);
            } else if (deps.commandsController?.openViewer) {
              void deps.commandsController.openViewer(fp, item);
            }
          }
        },
      });
    }

    if (isDir) {
      items.push({
        id: 'openTab',
        label: 'Open in New Tab',
        iconKey: 'tab',
        shortcut: isMac ? '⌘T' : 'Ctrl+T',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          const pane = deps.state[side];
          pane.addTab(fp);
          deps.tabsRenderer?.render(side);
          if (deps.loadDir) await deps.loadDir(side);
        },
      });
    }

    // Pin to Favorites
    if (isDir && deps.sidebarController) {
      const curFpPromise = getPath();
      curFpPromise.then((fp) => {
        if (!fp) return;
        const pinned = deps.sidebarController.isPinned(fp);
        items.push({
          id: 'pin',
          label: pinned ? 'Unpin from Favorites' : 'Pin to Favorites',
          iconKey: 'star',
          action: () => deps.sidebarController.togglePin(fp, item?.base),
        });
      });
    } else if (!isDir && !isFile && deps.sidebarController && targetDir) {
      const pinned = deps.sidebarController.isPinned(targetDir);
      items.push({
        id: 'pinFolder',
        label: pinned ? 'Unpin Current Folder' : 'Pin Current Folder',
        iconKey: 'star',
        action: () => deps.sidebarController.togglePin(targetDir),
      });
    }

    // Tags Row
    if (item && item.base !== '' && item.base !== '..' && deps.tagController && deps.tagController.isEnabled !== false) {
      items.push({
        id: 'tagsRow',
        customRender: (container: HTMLElement) => {
          const tagRow = document.createElement('div');
          tagRow.className = 'ctx-tags-row';

          const tagLabel = document.createElement('span');
          tagLabel.className = 'ctx-tags-label';
          tagLabel.textContent = 'Tags:';
          tagRow.appendChild(tagLabel);

          const dotsWrap = document.createElement('div');
          dotsWrap.className = 'ctx-tags-dots';

          const allTags = deps.tagController.getAllTags();
          void getPath().then((fp) => {
            if (!fp) return;
            const assigned = deps.tagController.getTagsForFile(fp);

            allTags.forEach((t: any) => {
              const dot = document.createElement('button');
              dot.type = 'button';
              dot.className = `ctx-tag-dot tag-dot--${t.id}${assigned.includes(t.id) ? ' active' : ''}`;
              if (t.color && !t.id.match(/^(red|orange|yellow|green|blue|purple|gray)$/)) {
                dot.style.background = safeColor(t.color);
              }
              dot.title = `Tag: ${t.name}`;
              dot.addEventListener('click', (e) => {
                e.stopPropagation();
                deps.tagController.toggleTagForFile(fp, t.id);
                dot.classList.toggle('active');
                deps.setStatus(`Toggled tag "${t.name}" on ${item.base}`);
              });
              dotsWrap.appendChild(dot);
            });
          });

          tagRow.appendChild(dotsWrap);
          container.appendChild(tagRow);
        },
      });
    }

    if (isFile) {
      items.push({
        id: 'checksum',
        label: 'Checksum / Hash…',
        iconKey: 'hash',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (fp) deps.openChecksum?.(fp);
        },
      });
    }

    // Git Actions
    const pane = deps.state[side];
    if (pane?.git?.isRepo && isFile) {
      addSep();
      items.push({
        id: 'gitDiff',
        label: 'Git Diff with HEAD',
        iconKey: 'git',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          let rel = fp;
          if (rel.startsWith(pane.git.root)) {
            rel = rel.slice(pane.git.root.length).replace(/^[/\\]+/, '');
          }
          await deps.openGitDiff?.(rel, pane.git.root);
        },
      });
      items.push({
        id: 'gitBlame',
        label: 'Git Blame',
        iconKey: 'git',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          let rel = fp;
          if (rel.startsWith(pane.git.root)) {
            rel = rel.slice(pane.git.root.length).replace(/^[/\\]+/, '');
          }
          await deps.openGitBlame?.(rel, pane.git.root);
        },
      });
      items.push({
        id: 'gitLog',
        label: 'Git File History',
        iconKey: 'git',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          let rel = fp;
          if (rel.startsWith(pane.git.root)) {
            rel = rel.slice(pane.git.root.length).replace(/^[/\\]+/, '');
          }
          await deps.openGitLog?.(rel, pane.git.root);
        },
      });
    }

    // File Operations
    if (isFile || isDir) {
      addSep();

      items.push({
        id: 'cut',
        label: 'Cut',
        iconKey: 'copyTo',
        shortcut: isMac ? '⌘X' : 'Ctrl+X',
        action: () => {
          deps.state.active = side;
          deps.commandsController?.cutSelection?.();
        },
      });

      items.push({
        id: 'copy',
        label: 'Copy',
        iconKey: 'copy',
        shortcut: isMac ? '⌘C' : 'Ctrl+C',
        action: () => {
          deps.state.active = side;
          deps.commandsController?.copySelection?.();
        },
      });

      items.push({
        id: 'duplicate',
        label: 'Duplicate',
        iconKey: 'copy',
        shortcut: isMac ? '⌘D' : 'Ctrl+D',
        action: () => {
          deps.state.active = side;
          deps.commandsController?.duplicate?.();
        },
      });

      items.push({
        id: 'rename',
        label: 'Rename…',
        iconKey: 'rename',
        shortcut: 'F2',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (fp && deps.beginRename) void deps.beginRename({ targetPath: fp, targetItem: item });
          else if (deps.beginRename) void deps.beginRename();
        },
      });

      addSep();

      items.push({
        id: 'multiRename',
        label: 'Multi-Rename…',
        iconKey: 'rename',
        shortcut: isMac ? '⌘M' : 'Ctrl+M',
        action: () => {
          deps.state.active = side;
          deps.openMultiRename?.();
        },
      });

      items.push({
        id: 'copyToOther',
        label: 'Copy to other panel',
        iconKey: 'copyTo',
        shortcut: 'F5',
        action: () => {
          deps.state.active = side;
          if (deps.copyToOther) void deps.copyToOther();
        },
      });

      items.push({
        id: 'moveToOther',
        label: 'Move to other panel',
        iconKey: 'moveTo',
        shortcut: 'F6',
        action: () => {
          deps.state.active = side;
          if (deps.moveToOther) void deps.moveToOther();
        },
      });

      items.push({
        id: 'compress',
        label: 'Compress to ZIP',
        iconKey: 'archive',
        shortcut: '⌥Z',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (!fp) return;
          try {
            const r = await deps.api().compressZip(fp);
            deps.setStatus(`Created ${r.zipPath}`);
          } catch (e: any) {
            deps.setStatus(e?.message || 'ZIP failed');
          }
          if (deps.loadDir) await deps.loadDir(side);
        },
      });

      items.push({
        id: 'share',
        label: 'Share',
        iconKey: 'share',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (fp) await deps.api().showItemInFolder(fp);
        },
      });

      items.push({
        id: 'trash',
        label: 'Move to Trash',
        iconKey: 'trash',
        shortcut: isMac ? '⌘⌫' : 'F8 / Del',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (fp && deps.beginDelete) {
            void deps.beginDelete({ targetPath: fp, targetItem: item, useTrash: true });
          } else if (deps.beginDelete) {
            void deps.beginDelete({ useTrash: true });
          }
        },
      });

      items.push({
        id: 'delete',
        label: 'Delete…',
        iconKey: 'trash',
        shortcut: isMac ? '⌘⌫' : 'F8 / Del',
        action: async () => {
          deps.state.active = side;
          const fp = await getPath();
          if (fp && deps.beginDelete) {
            void deps.beginDelete({ targetPath: fp, targetItem: item });
          } else if (deps.beginDelete) {
            void deps.beginDelete();
          }
        },
      });
    }

    return items;
  }
}
