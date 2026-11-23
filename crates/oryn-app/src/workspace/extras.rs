use std::path::{Path, PathBuf};

use gpui_kit::*;
use oryn_core::batch::BatchKind;
use oryn_core::format::{fmt_size, row_date_text};
use oryn_core::ui::arrange::SortField;

use super::Workspace;
use super::dialog::{Choice, Dialog};
use crate::actions::*;
use crate::tools::ToolKind;

impl Workspace {
    pub(super) fn is_collapsed(&self, key: &str) -> bool {
        self.settings.collapsed.iter().any(|k| k == key)
    }

    pub(super) fn toggle_section(&mut self, key: &str, cx: &mut Context<Self>) {
        if self.is_collapsed(key) {
            self.settings.collapsed.retain(|k| k != key);
        } else {
            self.settings.collapsed.push(key.to_string());
        }
        self.settings.save();
        cx.notify();
    }

    fn report(&mut self, text: String, cx: &mut Context<Self>) {
        self.active_pane()
            .clone()
            .update(cx, |p, cx| p.report(text, cx));
    }

    pub(super) fn read_only(&mut self, cx: &mut Context<Self>) -> bool {
        if self.active_pane().read(cx).in_archive() {
            self.report(
                "This is inside an archive, so it can't be changed. Extract it first.".into(),
                cx,
            );
            return true;
        }
        false
    }

    fn set_favorite(&mut self, path: PathBuf, on: bool, cx: &mut Context<Self>) {
        let name = oryn_core::favorites::Favorites::label(&path);
        let changed = if on {
            self.favorites.add(&path)
        } else {
            self.favorites.remove(&path)
        };
        if changed {
            self.favorites.save();
        }
        let text = match (on, changed) {
            (true, true) => format!("Added \u{201c}{name}\u{201d} to Favourites."),
            (true, false) => format!("\u{201c}{name}\u{201d} is already in Favourites."),
            (false, _) => format!("Removed \u{201c}{name}\u{201d} from Favourites."),
        };
        self.report(text, cx);
        cx.notify();
    }

    pub(super) fn add_current_favorite(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.read_only(cx) {
            return;
        }
        let path = self.active_pane().read(cx).path.clone();
        self.set_favorite(path, true, cx);
    }

    pub(super) fn toggle_current_favorite(
        &mut self,
        _: &FavoriteThisFolder,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.read_only(cx) {
            return;
        }
        let path = self.active_pane().read(cx).path.clone();
        let on = !self.favorites.contains(&path);
        self.set_favorite(path, on, cx);
    }

    pub(super) fn add_favorite(&mut self, _: &AddFavorite, _: &mut Window, cx: &mut Context<Self>) {
        if self.read_only(cx) {
            return;
        }
        let pane = self.active_pane().read(cx);
        let target = match pane.cursor_entry() {
            Some((name, true)) => pane.op_dir().join(name),
            _ => pane.path.clone(),
        };
        let on = !self.favorites.contains(&target);
        self.set_favorite(target, on, cx);
    }

    pub(super) fn remove_favorite(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.set_favorite(path.to_path_buf(), false, cx);
    }

    pub(super) fn remember_paths(&mut self, cx: &mut Context<Self>) {
        let keep = |p: &Path| (p.is_dir()).then(|| p.to_string_lossy().into_owned());
        let left = keep(&self.pane(0).read(cx).path);
        let right = keep(&self.pane(1).read(cx).path);
        let mut changed = false;
        if left.is_some() && left != self.settings.left {
            self.settings.left = left;
            changed = true;
        }
        if right.is_some() && right != self.settings.right {
            self.settings.right = right;
            changed = true;
        }
        if changed {
            self.settings.save();
        }
    }

    pub(super) fn click_tag(&mut self, label: &str, w: &mut Window, cx: &mut Context<Self>) {
        let pane = self.active_pane().read(cx);
        let marked = !pane.selection.is_empty() && !pane.in_archive();
        if marked {
            let targets = pane.targets();
            let mut tags = oryn_core::tools::tags::Tags::load();
            if !targets.iter().all(|t| tags.has(t, label)) {
                tags.toggle(&targets, label);
                tags.save();
                self.refresh_all(None, cx);
            }
        }
        self.open_tool_with(ToolKind::Tags, Some(label.to_string()), w, cx);
    }

    pub(super) fn clear_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.update(cx, |s, cx| s.set_value("", window, cx));
        let pane = self.active_pane().clone();
        pane.update(cx, |p, cx| p.set_filter("", cx));
        self.focus_pane(self.active, window, cx);
    }

    pub(super) fn go_to_path(&mut self, _: &GoToPath, window: &mut Window, cx: &mut Context<Self>) {
        let here = self.active_pane().read(cx).path.display().to_string();
        self.open_dialog(Dialog::GoTo, &here, window, cx);
    }

    pub(super) fn toggle_hidden(
        &mut self,
        _: &ToggleHidden,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let show = !self.settings.show_hidden;
        self.settings.show_hidden = show;
        for pane in self.tabs.iter().flatten().cloned().collect::<Vec<_>>() {
            pane.update(cx, |p, cx| {
                p.show_hidden = show;
                p.refresh(None, cx);
            });
        }
        self.settings.save();
        self.report(
            if show {
                "Showing hidden files."
            } else {
                "Hiding hidden files."
            }
            .into(),
            cx,
        );
    }

    pub(super) fn sort_active(&mut self, field: SortField, w: &mut Window, cx: &mut Context<Self>) {
        self.with_pane(w, cx, |p, cx| p.sort_by(field, cx));
    }

    pub(super) fn show_properties(
        &mut self,
        _: &ShowProperties,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = self.active_pane().read(cx);
        let path = match pane.cursor_entry() {
            Some((name, _)) => pane.op_dir().join(name),
            None => pane.path.clone(),
        };
        let row = pane
            .rows
            .iter()
            .find(|r| Some(r.name.as_ref()) == path.file_name().and_then(|n| n.to_str()))
            .cloned();
        let virt = oryn_core::vfs::is_virtual(&path);
        let mut rows: Vec<(&'static str, String)> = vec![("Where", path.display().to_string())];
        let is_dir = if virt {
            row.as_ref().is_some_and(|r| r.is_dir)
        } else {
            path.is_dir()
        };
        rows.push((
            "Kind",
            if is_dir {
                "Folder".into()
            } else {
                match oryn_core::preview::extension(&path).as_str() {
                    "" => "File".into(),
                    ext => format!("{} file", ext.to_uppercase()),
                }
            },
        ));
        if virt {
            if let Some(r) = &row {
                rows.push(("Size", fmt_size(r.size)));
                rows.push(("Modified", row_date_text(&r.mtime)));
            }
        } else {
            match oryn_core::services::fs_props::stat_props(&path) {
                Ok(props) => {
                    rows.push((
                        "Size",
                        if props.is_dir {
                            "Calculating\u{2026}".into()
                        } else {
                            format!("{} ({} bytes)", fmt_size(Some(props.size)), props.size)
                        },
                    ));
                    rows.push(("Modified", row_date_text(&props.mtime)));
                    if cfg!(unix) {
                        rows.push(("Permissions", props.mode_oct.clone()));
                    }
                }
                Err(e) => rows.push(("Error", e.to_string())),
            }
            if is_dir {
                let target = path.clone();
                let task = cx.background_spawn({
                    let target = target.clone();
                    async move { oryn_core::services::fs_size::path_size(&target) }
                });
                self.info_task = Some(cx.spawn(async move |this, cx| {
                    let sum = task.await;
                    let _ = this.update(cx, |ws, cx| {
                        if let Some(Dialog::Info { path, rows }) = &mut ws.dialog
                            && *path == target
                        {
                            for (k, v) in rows.iter_mut() {
                                if *k == "Size" {
                                    *v = format!(
                                        "{} in {} files and {} folders",
                                        fmt_size(Some(sum.bytes)),
                                        sum.files,
                                        sum.dirs
                                    );
                                }
                            }
                            cx.notify();
                        }
                    });
                }));
            }
        }
        self.open_dialog(Dialog::Info { path, rows }, "", window, cx);
    }

    pub(super) fn open_in_terminal(
        &mut self,
        _: &OpenInTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.read_only(cx) {
            return;
        }
        let pane = self.active_pane().read(cx);
        let dir = match pane.cursor_entry() {
            Some((name, true)) => pane.op_dir().join(name),
            _ => pane.op_dir(),
        };
        self.show_terminal_at(dir, window, cx);
    }

    pub(super) fn swap_panes(
        &mut self,
        _: &SwapPanes,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dual {
            return;
        }
        self.tabs.swap(0, 1);
        self.tab_ix.swap(0, 1);
        self.remember_paths(cx);
        self.focus_pane(self.active, window, cx);
    }

    fn stash(&mut self, kind: BatchKind, cx: &mut Context<Self>) {
        let targets = self.active_pane().read(cx).targets();
        if targets.is_empty() {
            return self.report("Select a file or folder first.".into(), cx);
        }
        if kind == BatchKind::Move && self.read_only(cx) {
            return;
        }
        let text = targets
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        let n = targets.len();
        let what = if n == 1 {
            "1 item".to_string()
        } else {
            format!("{n} items")
        };
        self.report(
            format!(
                "{what} {} Paste with Ctrl+V.",
                if kind == BatchKind::Move {
                    "cut."
                } else {
                    "copied."
                }
            ),
            cx,
        );
        self.clipboard = Some((kind, targets));
    }

    pub(super) fn cut_items(&mut self, _: &CutItems, _: &mut Window, cx: &mut Context<Self>) {
        self.stash(BatchKind::Move, cx);
    }

    pub(super) fn copy_items(&mut self, _: &CopyItems, _: &mut Window, cx: &mut Context<Self>) {
        self.stash(BatchKind::Copy, cx);
    }

    pub(super) fn paste_items(
        &mut self,
        _: &PasteItems,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.read_only(cx) {
            return;
        }
        let Some((kind, sources)) = self.clipboard.clone() else {
            return self.report("Cut or copy something first.".into(), cx);
        };
        let dir = self.active_pane().read(cx).op_dir();
        let sources: Vec<PathBuf> = sources
            .into_iter()
            .filter(|p| p.parent() != Some(dir.as_path()) || kind == BatchKind::Copy)
            .collect();
        if sources.is_empty() {
            return;
        }
        if kind == BatchKind::Move {
            self.clipboard = None;
        }
        if sources.iter().all(|p| p.parent() == Some(dir.as_path())) {
            return self.dispatch_duplicate(window, cx, sources);
        }
        let value = dir.to_string_lossy().into_owned();
        self.open_dialog(
            Dialog::Transfer {
                kind,
                sources,
                conflicts: None,
            },
            &value,
            window,
            cx,
        );
        self.choose(Choice::Confirm, window, cx);
    }

    fn dispatch_duplicate(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        sources: Vec<PathBuf>,
    ) {
        self.duplicate_paths(sources, cx);
        self.focus_pane(self.active, window, cx);
    }

    pub(super) fn reload_profiles(&mut self) {
        self.profiles = oryn_core::services::remote::profile::load_profiles().unwrap_or_default();
    }

    #[cfg(test)]
    pub fn favorite_paths(&self) -> Vec<PathBuf> {
        self.favorites.paths.clone()
    }

    #[cfg(test)]
    pub fn collapsed(&self) -> Vec<String> {
        self.settings.collapsed.clone()
    }

    #[cfg(test)]
    pub fn view_mode(&self) -> crate::pane::ViewMode {
        self.view
    }

    #[cfg(test)]
    pub fn dialog_kind(&self) -> Option<&'static str> {
        self.dialog.as_ref().map(|d| d.kind())
    }

    #[cfg(test)]
    pub fn dialog_rows(&self) -> Vec<(String, String)> {
        match &self.dialog {
            Some(Dialog::Info { rows, .. }) => rows
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            _ => Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn set_dialog_value(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog_input
            .update(cx, |s, cx| s.set_value(text.to_string(), window, cx));
    }

    #[cfg(test)]
    pub fn tool_kind(&self) -> Option<ToolKind> {
        self.tool.as_ref().map(|t| t.kind())
    }

    #[cfg(test)]
    pub fn pane_at(&self, side: usize) -> Entity<crate::pane::FilePane> {
        self.pane(side).clone()
    }

    #[cfg(test)]
    pub fn menu_has(&self, label: &str) -> bool {
        self.has_menu_item(label)
    }
}
