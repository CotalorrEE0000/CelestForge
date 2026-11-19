use std::path::{Path, PathBuf};

use rodio::{Decoder, Player, Source};

pub struct AudioSystem {
    sink: Option<rodio::MixerDeviceSink>,
    bgm: Option<Player>,
    voice: Option<Player>,
    bgm_volume: f32,
    se_volume: f32,
    voice_volume: f32,
    asset_root: PathBuf,
}

impl AudioSystem {
    pub fn new(asset_root: PathBuf) -> Self {
        let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut s) => {
                s.log_on_drop(false);
                Some(s)
            }
            Err(e) => {
                eprintln!("[AUDIO] device unavailable, audio disabled: {e}");
                None
            }
        };

        Self {
            sink,
            bgm: None,
            voice: None,
            bgm_volume: 0.6,
            se_volume: 0.8,
            voice_volume: 1.0,
            asset_root,
        }
    }

    fn resolve(&self, asset_id: &str) -> PathBuf {
        self.asset_root.join(asset_id)
    }

    fn decode(path: &Path) -> Result<Decoder<std::io::BufReader<std::fs::File>>, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("open {path:?}: {e}"))?;
        Decoder::try_from(file).map_err(|e| format!("decode {path:?}: {e}"))
    }

    pub fn play_bgm(&mut self, asset_id: &str) {
        let Some(sink) = &self.sink else { return };
        let src = match Self::decode(&self.resolve(asset_id)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[AUDIO] bgm {asset_id}: {e}");
                return;
            }
        };
        let player = Player::connect_new(sink.mixer());
        player.append(src.repeat_infinite());
        player.set_volume(self.bgm_volume);
        self.bgm = Some(player);
    }

    pub fn stop_bgm(&mut self) {
        self.bgm = None;
    }

    pub fn play_voice(&mut self, who: &str, asset_id: &str) {
        let Some(sink) = &self.sink else { return };
        let src = match Self::decode(&self.resolve(asset_id)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[AUDIO] voice {who}/{asset_id}: {e}");
                return;
            }
        };
        let player = Player::connect_new(sink.mixer());
        player.append(src);
        player.set_volume(self.voice_volume);
        self.voice = Some(player);
    }

    pub fn stop_voice(&mut self) {
        self.voice = None;
    }

    pub fn play_se(&mut self, asset_id: &str) {
        let Some(sink) = &self.sink else { return };
        let src = match Self::decode(&self.resolve(asset_id)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[AUDIO] se {asset_id}: {e}");
                return;
            }
        };
        let player = Player::connect_new(sink.mixer());
        player.append(src);
        player.set_volume(self.se_volume);
        player.detach();
    }

    pub fn stop_se(&mut self) {}

    pub fn set_bgm_volume(&mut self, v: f32) {
        self.bgm_volume = v.clamp(0.0, 1.0);
        if let Some(p) = &self.bgm {
            p.set_volume(self.bgm_volume);
        }
    }

    pub fn set_se_volume(&mut self, v: f32) {
        self.se_volume = v.clamp(0.0, 1.0);
    }

    pub fn set_voice_volume(&mut self, v: f32) {
        self.voice_volume = v.clamp(0.0, 1.0);
        if let Some(p) = &self.voice {
            p.set_volume(self.voice_volume);
        }
    }

    pub fn bgm_volume(&self) -> f32 {
        self.bgm_volume
    }

    pub fn se_volume(&self) -> f32 {
        self.se_volume
    }
}
