use std::cell::RefCell;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

use super::{AudioOutput, AudioPlayer};

#[derive(Default)]
pub struct RodioOutput {
    device: RefCell<Option<Rc<MixerDeviceSink>>>,
}

impl RodioOutput {
    fn device(&self) -> Result<Rc<MixerDeviceSink>> {
        if let Some(device) = self.device.borrow().as_ref() {
            return Ok(device.clone());
        }
        let mut sink =
            DeviceSinkBuilder::open_default_sink().context("no audio output is available")?;
        sink.log_on_drop(false);
        let device = Rc::new(sink);
        *self.device.borrow_mut() = Some(device.clone());
        Ok(device)
    }
}

impl AudioOutput for RodioOutput {
    fn open(&self, path: &Path) -> Result<Box<dyn AudioPlayer>> {
        let file = File::open(path).with_context(|| format!("can't open {}", path.display()))?;
        let len = file.metadata()?.len();
        let mut builder = Decoder::builder()
            .with_data(BufReader::new(file))
            .with_byte_len(len)
            .with_seekable(true);
        let ext = oryn_core::preview::extension(path);
        if !ext.is_empty() {
            builder = builder.with_hint(&ext);
        }
        let decoder = builder
            .build()
            .context("this audio format can't be played")?;
        let device = self.device()?;
        let player = Player::connect_new(device.mixer());
        player.pause();
        player.append(decoder);
        Ok(Box::new(RodioPlayer {
            player,
            _device: device,
        }))
    }
}

struct RodioPlayer {
    player: Player,
    _device: Rc<MixerDeviceSink>,
}

impl AudioPlayer for RodioPlayer {
    fn play(&mut self) {
        self.player.play();
    }

    fn pause(&mut self) {
        self.player.pause();
    }

    fn is_playing(&self) -> bool {
        !self.player.is_paused() && !self.player.empty()
    }

    fn is_finished(&self) -> bool {
        self.player.empty()
    }

    fn position(&self) -> Duration {
        self.player.get_pos()
    }

    fn seek(&mut self, to: Duration) -> Result<()> {
        self.player
            .try_seek(to)
            .map_err(|e| anyhow::anyhow!("can't seek in this file: {e}"))
    }
}
