//! WebAudio sound playback mirroring Deltarune's snd_* helpers.
#![allow(deprecated)]
//! Sounds are referenced by name ("snd_hurt1"); each play returns a handle.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, GainNode};

pub type SndHandle = i64;

struct Voice {
    name: String,
    src: AudioBufferSourceNode,
    gain: GainNode,
    base_vol: f64,
    vol: f64,
    ended: Rc<RefCell<bool>>,
    paused: bool,
}

pub struct Audio {
    pub ctx: Option<AudioContext>,
    buffers: Rc<RefCell<HashMap<String, AudioBuffer>>>,
    base_vol: HashMap<String, f64>,
    voices: HashMap<SndHandle, Voice>,
    next: SndHandle,
    pub master: f64,
}

impl Audio {
    pub fn new() -> Audio {
        let ctx = AudioContext::new().ok();
        Audio { ctx, buffers: Rc::new(RefCell::new(HashMap::new())), base_vol: HashMap::new(), voices: HashMap::new(), next: 1, master: 1.0 }
    }

    pub fn resume(&self) {
        if self.master == 0.0 {
            return;
        }
        if let Some(c) = &self.ctx {
            let _ = c.resume();
        }
    }

    /// Start loading a sound file; it becomes playable when decoded.
    pub fn load(&mut self, name: &str, url: &str, vol: f64) {
        self.base_vol.insert(name.to_string(), vol);
        let Some(ctx) = self.ctx.clone() else { return };
        let bufs = self.buffers.clone();
        let name = name.to_string();
        let url = url.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let r: Result<(), JsValue> = async {
                let win = web_sys::window().unwrap();
                let resp: web_sys::Response = JsFuture::from(win.fetch_with_str(&url)).await?.dyn_into()?;
                let ab = JsFuture::from(resp.array_buffer()?).await?;
                let buf: AudioBuffer = JsFuture::from(ctx.decode_audio_data(&ab.dyn_into()?)?).await?.dyn_into()?;
                bufs.borrow_mut().insert(name.clone(), buf);
                Ok(())
            }
            .await;
            if let Err(e) = r {
                crate::log(&format!("audio load failed {url}: {e:?}"));
            }
        });
    }

    pub fn loaded_count(&self) -> usize { self.buffers.borrow().len() }

    fn cleanup(&mut self) {
        self.voices.retain(|_, v| !*v.ended.borrow());
    }

    pub fn play_ext(&mut self, name: &str, vol: f64, pitch: f64, looping: bool) -> SndHandle {
        self.cleanup();
        let Some(ctx) = &self.ctx else { return -1 };
        let Some(buf) = self.buffers.borrow().get(name).cloned() else { return -1 };
        let src = ctx.create_buffer_source().unwrap();
        src.set_buffer(Some(&buf));
        src.set_loop(looping);
        src.playback_rate().set_value(pitch.max(0.01) as f32);
        let gain = ctx.create_gain().unwrap();
        let base = *self.base_vol.get(name).unwrap_or(&1.0);
        gain.gain().set_value((base * vol * self.master) as f32);
        src.connect_with_audio_node(&gain).unwrap();
        gain.connect_with_audio_node(&ctx.destination()).unwrap();
        let ended = Rc::new(RefCell::new(false));
        let e2 = ended.clone();
        let cb = Closure::once_into_js(move || {
            *e2.borrow_mut() = true;
        });
        src.set_onended(Some(cb.unchecked_ref()));
        let _ = src.start();
        let h = self.next;
        self.next += 1;
        self.voices.insert(h, Voice { name: name.to_string(), src, gain, base_vol: base, vol, ended, paused: false });
        h
    }

    /// snd_play
    pub fn play(&mut self, name: &str) -> SndHandle { self.play_ext(name, 1.0, 1.0, false) }
    /// snd_play_x(sound, volume, pitch)
    pub fn play_x(&mut self, name: &str, vol: f64, pitch: f64) -> SndHandle { self.play_ext(name, vol, pitch, false) }
    /// snd_play_pitch(sound, pitch)
    pub fn play_pitch(&mut self, name: &str, pitch: f64) -> SndHandle { self.play_ext(name, 1.0, pitch, false) }
    /// snd_loop
    pub fn play_loop(&mut self, name: &str) -> SndHandle { self.play_ext(name, 1.0, 1.0, true) }

    /// Stop every playing instance of a sound name.
    pub fn stop(&mut self, name: &str) {
        let hs: Vec<_> = self.voices.iter().filter(|(_, v)| v.name == name).map(|(h, _)| *h).collect();
        for h in hs {
            self.stop_handle(h);
        }
    }
    pub fn stop_handle(&mut self, h: SndHandle) {
        if let Some(v) = self.voices.remove(&h) {
            let _ = v.src.stop();
            let _ = v.src.disconnect();
        }
    }
    pub fn stop_all(&mut self) {
        let hs: Vec<_> = self.voices.keys().copied().collect();
        for h in hs {
            self.stop_handle(h);
        }
    }
    pub fn is_playing(&mut self, h: SndHandle) -> bool {
        self.cleanup();
        self.voices.contains_key(&h)
    }
    pub fn is_playing_name(&mut self, name: &str) -> bool {
        self.cleanup();
        self.voices.values().any(|v| v.name == name)
    }

    /// snd_volume(handle, volume, frames) — linear fade over `frames` 30fps frames.
    pub fn volume(&mut self, h: SndHandle, vol: f64, frames: f64) {
        let master = self.master;
        let Some(ctx) = &self.ctx else { return };
        if let Some(v) = self.voices.get_mut(&h) {
            let g = v.gain.gain();
            let now = ctx.current_time();
            let _ = g.cancel_scheduled_values(now);
            let cur = (v.base_vol * v.vol * master) as f32;
            g.set_value_at_time(cur, now).ok();
            let target = (v.base_vol * vol * master) as f32;
            if frames <= 0.0 {
                g.set_value_at_time(target, now).ok();
            } else {
                g.linear_ramp_to_value_at_time(target, now + frames / 30.0).ok();
            }
            v.vol = vol;
        }
    }
    /// Volume applied to every instance of a sound name.
    pub fn volume_name(&mut self, name: &str, vol: f64, frames: f64) {
        let hs: Vec<_> = self.voices.iter().filter(|(_, v)| v.name == name).map(|(h, _)| *h).collect();
        for h in hs {
            self.volume(h, vol, frames);
        }
    }
    pub fn pitch(&mut self, h: SndHandle, p: f64) {
        if let Some(v) = self.voices.get(&h) {
            v.src.playback_rate().set_value(p.max(0.01) as f32);
        }
    }
    pub fn pitch_name(&mut self, name: &str, p: f64) {
        for v in self.voices.values().filter(|v| v.name == name) {
            v.src.playback_rate().set_value(p.max(0.01) as f32);
        }
    }
    pub fn pause(&mut self, h: SndHandle) {
        if let Some(v) = self.voices.get_mut(&h) {
            v.paused = true;
            v.src.playback_rate().set_value(0.0001);
            v.gain.gain().set_value(0.0);
        }
    }
}
