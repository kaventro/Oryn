use gpui_kit::*;
use oryn_core::update::{Plan, Release};

use super::Workspace;
use super::dialog::{Choice, Dialog, UpdateStage};

impl Workspace {
    #[cfg(test)]
    pub fn use_updates(
        &mut self,
        source: std::sync::Arc<dyn crate::update::UpdateSource>,
        cx: &mut Context<Self>,
    ) {
        self.updates = source;
        self.look_for_update(cx);
    }

    #[cfg(test)]
    pub fn update_prompt(&self) -> Option<(String, Option<String>)> {
        match &self.dialog {
            Some(Dialog::Update { stage, .. }) => Some((
                format!("{stage:?}"),
                self.dialog_error.as_ref().map(|e| e.to_string()),
            )),
            _ => None,
        }
    }

    pub(super) fn check_updates(&mut self, cx: &mut Context<Self>) {
        if std::env::var_os("ORYN_NO_UPDATE_CHECK").is_none() {
            self.look_for_update(cx);
        }
    }

    fn look_for_update(&mut self, cx: &mut Context<Self>) {
        let source = self.updates.clone();
        let task = cx.background_spawn(async move { source.latest() });
        self._update_task = Some(cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| {
                this.update = found;
                cx.notify();
            });
        }));
    }

    pub(super) fn offer_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(release) = self.update.clone() else {
            return;
        };
        let stage = if self.installing {
            UpdateStage::Working
        } else {
            match self.updates.plan(&release) {
                Plan::Automatic(_) => UpdateStage::Offer,
                Plan::Manual(reason) => UpdateStage::Manual(reason),
            }
        };
        self.open_dialog(Dialog::Update { release, stage }, "", window, cx);
    }

    pub(super) fn choose_update(
        &mut self,
        release: Release,
        stage: UpdateStage,
        choice: Choice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match (choice, stage) {
            (Choice::Browse, _) | (_, UpdateStage::Manual(_)) => {
                self.updates.open_page(&release.url);
                self.close_prompt(window, cx);
            }
            (Choice::Confirm, UpdateStage::Offer | UpdateStage::Failed) => {
                self.install_update(release, cx)
            }
            (_, stage) => {
                self.dialog = Some(Dialog::Update { release, stage });
                cx.notify();
            }
        }
    }

    fn install_update(&mut self, release: Release, cx: &mut Context<Self>) {
        self.installing = true;
        self.dialog_error = None;
        self.dialog = Some(Dialog::Update {
            release: release.clone(),
            stage: UpdateStage::Working,
        });
        cx.notify();
        let job = release.clone();
        let source = self.updates.clone();
        let task =
            cx.background_spawn(async move { source.install(&job).map_err(|e| format!("{e:#}")) });
        self._update_task = Some(cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.installing = false;
                match result {
                    Ok(relaunch) => {
                        cx.set_restart_path(relaunch);
                        cx.restart();
                    }
                    Err(error) => {
                        if matches!(this.dialog, None | Some(Dialog::Update { .. })) {
                            this.dialog = Some(Dialog::Update {
                                release,
                                stage: UpdateStage::Failed,
                            });
                            this.dialog_error = Some(error.into());
                        }
                        cx.notify();
                    }
                }
            });
        }));
    }
}
