use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use oryn_core::batch::{self, BatchKind, BatchReport, Progress};
use oryn_core::format::fmt_size;
use oryn_core::services::fs_transfer::{OverwritePolicy, copy};
use oryn_core::ui::rename::duplicate_name;

use super::Workspace;
use super::dialog::Dialog;
use crate::actions::*;
use crate::theme::{Palette, black_alpha, hex, white_alpha};

fn copy_out_of_archive(
    sources: &[PathBuf],
    dest: &std::path::Path,
    policy: OverwritePolicy,
    p: &Progress,
) -> BatchReport {
    use oryn_core::vfs::{VfsRouter, VirtualFileSystem};
    let router = VfsRouter::new();
    let mut report = BatchReport::default();
    p.total.store(sources.len() as u64, Ordering::Relaxed);
    for src in sources {
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let target = dest.join(&name);
        if target.symlink_metadata().is_ok() && policy == OverwritePolicy::Skip {
            report.done += 1;
            continue;
        }
        match router.extract_to(&src.to_string_lossy(), &target) {
            Ok(()) => report.done += 1,
            Err(e) => report.failed.push((name, e.to_string())),
        }
    }
    report
}

pub struct Job {
    label: String,
    progress: Arc<Progress>,
}

impl Workspace {
    pub(super) fn refresh_all(&mut self, focus: Option<&str>, cx: &mut Context<Self>) {
        let active = self.active;
        for side in 0..2 {
            let pane = self.pane(side).clone();
            let f = if side == active { focus } else { None };
            pane.update(cx, |p, cx| p.refresh(f, cx));
        }
    }

    fn notify_active(&mut self, text: String, cx: &mut Context<Self>) {
        self.active_pane()
            .clone()
            .update(cx, |p, cx| p.report(text, cx));
    }

    fn busy(&mut self, cx: &mut Context<Self>) -> bool {
        if self.job.is_some() {
            self.notify_active("Wait for the current operation to finish.".into(), cx);
            return true;
        }
        false
    }

    fn run_job(
        &mut self,
        label: String,
        kind: BatchKind,
        work: impl FnOnce(&Progress) -> BatchReport + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let progress = Arc::new(Progress::default());
        self.job = Some(Job {
            label,
            progress: progress.clone(),
        });
        cx.notify();
        let task = cx.background_spawn({
            let progress = progress.clone();
            async move { work(&progress) }
        });
        cx.spawn(async move |this, cx| {
            let ticker = {
                let this = this.clone();
                let cx = cx.clone();
                async move {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(120))
                            .await;
                        if this.update(&mut cx.clone(), |_, cx| cx.notify()).is_err() {
                            break;
                        }
                    }
                }
            };
            let report = futures::future::select(task, Box::pin(ticker)).await;
            let report = match report {
                futures::future::Either::Left((r, _)) => r,
                futures::future::Either::Right(_) => return,
            };
            let _ = this.update(cx, |ws, cx| {
                ws.job = None;
                ws.active_pane().clone().update(cx, |p, _| {
                    if !report.cancelled && report.failed.is_empty() {
                        p.clear_marks();
                    }
                });
                ws.refresh_all(None, cx);
                ws.notify_active(report.summary(kind), cx);
            });
        })
        .detach();
    }

    pub(super) fn start_batch(
        &mut self,
        kind: BatchKind,
        sources: Vec<PathBuf>,
        dest: Option<PathBuf>,
        policy: OverwritePolicy,
        cx: &mut Context<Self>,
    ) {
        if self.busy(cx) {
            return;
        }
        if sources.iter().any(|s| oryn_core::vfs::is_virtual(s)) {
            if kind != BatchKind::Copy {
                return self
                    .notify_active("Files inside an archive can only be copied out.".into(), cx);
            }
            let Some(dest) = dest else {
                return;
            };
            return self.run_job(
                "Copying".into(),
                kind,
                move |p| copy_out_of_archive(&sources, &dest, policy, p),
                cx,
            );
        }
        let label = kind.verb().to_string();
        self.run_job(
            label,
            kind,
            move |p| batch::run(kind, &sources, dest.as_deref(), policy, p),
            cx,
        );
    }

    pub(super) fn other_dir(&self, cx: &App) -> PathBuf {
        let side = if self.dual {
            1 - self.active
        } else {
            self.active
        };
        self.pane(side).read(cx).op_dir()
    }

    fn sources(&mut self, cx: &mut Context<Self>) -> Option<Vec<PathBuf>> {
        let sources = self.active_pane().read(cx).targets();
        if sources.is_empty() {
            self.notify_active("Select a file or folder first.".into(), cx);
            return None;
        }
        Some(sources)
    }

    fn transfer(&mut self, kind: BatchKind, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy(cx) || (kind == BatchKind::Move && self.read_only(cx)) {
            return;
        }
        let Some(sources) = self.sources(cx) else {
            return;
        };
        let dest = self.other_dir(cx).to_string_lossy().into_owned();
        self.open_dialog(
            Dialog::Transfer {
                kind,
                sources,
                conflicts: None,
            },
            &dest,
            window,
            cx,
        );
    }

    pub(super) fn copy_to_other(
        &mut self,
        _: &CopyToOther,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transfer(BatchKind::Copy, w, cx);
    }

    pub(super) fn move_to_other(
        &mut self,
        _: &MoveToOther,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transfer(BatchKind::Move, w, cx);
    }

    fn delete(&mut self, permanent: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy(cx) || self.read_only(cx) {
            return;
        }
        let Some(sources) = self.sources(cx) else {
            return;
        };
        let permanent = permanent || !self.settings.use_trash;
        if !self.settings.confirm_delete {
            let kind = if permanent {
                BatchKind::Delete
            } else {
                BatchKind::Trash
            };
            return self.start_batch(kind, sources, None, OverwritePolicy::Overwrite, cx);
        }
        self.open_dialog(Dialog::Delete { sources, permanent }, "", window, cx);
    }

    pub(super) fn delete_items(&mut self, _: &DeleteItems, w: &mut Window, cx: &mut Context<Self>) {
        self.delete(false, w, cx);
    }

    pub(super) fn delete_permanently(
        &mut self,
        _: &DeletePermanently,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.delete(true, w, cx);
    }

    pub(super) fn rename_item(&mut self, _: &RenameItem, w: &mut Window, cx: &mut Context<Self>) {
        if self.read_only(cx) {
            return;
        }
        let pane = self.active_pane().read(cx);
        let dir = pane.op_dir();
        let Some((old, _)) = pane.cursor_entry() else {
            return self.notify_active("Put the cursor on a file first.".into(), cx);
        };
        let value = old.clone();
        self.open_dialog(Dialog::Rename { dir, old }, &value, w, cx);
    }

    pub(super) fn new_folder(&mut self, _: &NewFolder, w: &mut Window, cx: &mut Context<Self>) {
        if self.read_only(cx) {
            return;
        }
        let dir = self.active_pane().read(cx).op_dir();
        self.open_dialog(Dialog::NewFolder { dir }, "untitled folder", w, cx);
    }

    pub(super) fn new_file(&mut self, _: &NewFile, w: &mut Window, cx: &mut Context<Self>) {
        if self.read_only(cx) {
            return;
        }
        let dir = self.active_pane().read(cx).op_dir();
        self.open_dialog(Dialog::NewFile { dir }, "untitled.txt", w, cx);
    }

    pub(super) fn duplicate_items(
        &mut self,
        _: &DuplicateItems,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy(cx) || self.read_only(cx) {
            return;
        }
        let Some(sources) = self.sources(cx) else {
            return;
        };
        self.duplicate_paths(sources, cx);
    }

    pub(super) fn duplicate_paths(&mut self, sources: Vec<PathBuf>, cx: &mut Context<Self>) {
        if self.busy(cx) {
            return;
        }
        self.run_job(
            "Duplicating".into(),
            BatchKind::Copy,
            move |p| {
                let mut report = BatchReport::default();
                p.total.store(sources.len() as u64, Ordering::Relaxed);
                for src in &sources {
                    let (Some(dir), Some(name)) = (src.parent(), src.file_name()) else {
                        continue;
                    };
                    let name = name.to_string_lossy().into_owned();
                    let target = duplicate_name(&name, src.is_dir(), |n| {
                        dir.join(n).symlink_metadata().is_ok()
                    });
                    let out = copy(p, src, &dir.join(&target), OverwritePolicy::Skip);
                    if out.ok {
                        report.done += 1;
                    } else if out.cancelled == Some(true) {
                        report.cancelled = true;
                        break;
                    } else {
                        report
                            .failed
                            .push((name, out.error.unwrap_or_else(|| "failed".into())));
                    }
                }
                report
            },
            cx,
        );
    }

    pub(super) fn compress_zip(&mut self, _: &CompressZip, _: &mut Window, cx: &mut Context<Self>) {
        if self.read_only(cx) {
            return;
        }
        let Some(sources) = self.sources(cx) else {
            return;
        };
        let dir = self.active_pane().read(cx).op_dir();
        self.start_batch(
            BatchKind::Compress,
            sources,
            Some(dir),
            OverwritePolicy::Skip,
            cx,
        );
    }

    pub(super) fn extract_here(&mut self, _: &ExtractHere, _: &mut Window, cx: &mut Context<Self>) {
        if self.read_only(cx) {
            return;
        }
        let Some(sources) = self.sources(cx) else {
            return;
        };
        let archives: Vec<_> = sources
            .into_iter()
            .filter(|s| batch::is_archive(s))
            .collect();
        if archives.is_empty() {
            self.notify_active("Select a .zip, .tar or .tar.gz archive first.".into(), cx);
            return;
        }
        let dir = self.active_pane().read(cx).op_dir();
        self.start_batch(
            BatchKind::Extract,
            archives,
            Some(dir),
            OverwritePolicy::Skip,
            cx,
        );
    }

    pub(super) fn refresh_panes(
        &mut self,
        _: &RefreshPanes,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.refresh_all(None, cx);
    }

    pub(super) fn render_job(&self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let job = self.job.as_ref()?;
        let fraction = job.progress.fraction();
        let done = job.progress.done.load(Ordering::Relaxed);
        let total = job.progress.total.load(Ordering::Relaxed);
        let current = job.progress.current.lock().clone();
        let cancelling = job.progress.is_cancelled();
        let bytes_total = job.progress.bytes_total.load(Ordering::Relaxed);
        let amount = if bytes_total > 0 {
            format!(
                "{} of {}",
                fmt_size(Some(job.progress.bytes_done().min(bytes_total))),
                fmt_size(Some(bytes_total))
            )
        } else {
            format!("{done} of {total}")
        };
        Some(
            div()
                .id("job")
                .role(Role::Status)
                .aria_label(SharedString::from(format!(
                    "{} {done} of {total}",
                    job.label
                )))
                .absolute()
                .right(px(20.0))
                .bottom(px(40.0))
                .w(px(340.0))
                .p(px(14.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .rounded(px(12.0))
                .bg(hex(0x242428).opacity(0.97))
                .border_1()
                .border_color(white_alpha(0.12))
                .shadow(vec![BoxShadow {
                    color: black_alpha(0.6),
                    offset: point(px(0.0), px(12.0)),
                    blur_radius: px(36.0),
                    spread_radius: px(0.0),
                    inset: false,
                }])
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.text_strong)
                                .child(format!("{}\u{2026}", job.label)),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.text_muted)
                                .child(amount),
                        ),
                )
                .child(
                    div()
                        .h(px(6.0))
                        .w_full()
                        .rounded_full()
                        .bg(white_alpha(0.08))
                        .overflow_hidden()
                        .child(
                            div()
                                .h_full()
                                .rounded_full()
                                .bg(p.accent)
                                .w(relative(fraction.max(0.02))),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(11.5))
                                .text_color(p.text_secondary)
                                .child(current),
                        )
                        .child(
                            div()
                                .id("job-cancel")
                                .role(Role::Button)
                                .aria_label("Cancel")
                                .px(px(10.0))
                                .py(px(3.0))
                                .rounded(px(6.0))
                                .text_size(px(12.0))
                                .bg(white_alpha(0.08))
                                .border_1()
                                .border_color(white_alpha(0.1))
                                .text_color(hex(0xd1d1d6))
                                .cursor_pointer()
                                .hover(|s| s.bg(white_alpha(0.14)))
                                .when(cancelling, |d| d.opacity(0.5))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(job) = &this.job {
                                        job.progress.cancel();
                                    }
                                    cx.notify();
                                }))
                                .child(if cancelling { "Cancelling" } else { "Cancel" }),
                        ),
                )
                .into_any_element(),
        )
    }
}
