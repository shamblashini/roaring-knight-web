//! Sprite/font metadata and texture loading.

use crate::gfx::Gfx;
use base64::Engine;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Sprite handle. -1 = no sprite (GameMaker's -1 / noone).
pub type Spr = i32;
pub const NO_SPR: Spr = -1;

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub tex: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub offx: f32,
    pub offy: f32,
}

pub struct Sprite {
    pub name: String,
    pub w: f64,
    pub h: f64,
    pub ox: f64,
    pub oy: f64,
    pub bl: f64,
    pub br: f64,
    pub bt: f64,
    pub bb: f64,
    /// 0 = axis-aligned rectangle, 1 = precise, 2 = rotated rectangle
    pub kind: i32,
    pub speed: f64,
    pub speedtype: i32,
    pub frames: Vec<Option<Frame>>,
    /// bit-packed rows, MSB = leftmost pixel; one entry per frame or a single shared one
    pub masks: Vec<Vec<u8>>,
    pub mask_w: usize,
    pub mask_h: usize,
    /// GameMaker nine-slice borders [left, top, right, bottom] (unscaled when the sprite is stretched)
    pub nineslice: Option<[f64; 4]>,
}

impl Sprite {
    /// Nine-slice mapping of a destination coordinate `u` (0..dest) back to source pixels (0..src).
    pub fn nineslice_map(u: f64, dest: f64, src: f64, a: f64, b: f64) -> f64 {
        if u < a {
            u
        } else if u > dest - b {
            src - (dest - u)
        } else {
            let mid_d = (dest - a - b).max(0.0001);
            a + (u - a) * (src - a - b) / mid_d
        }
    }
    pub fn frame_count(&self) -> usize { self.frames.len().max(1) }
    pub fn mask_at(&self, frame: usize, px: i64, py: i64) -> bool {
        if px < 0 || py < 0 || px as usize >= self.mask_w || py as usize >= self.mask_h {
            return false;
        }
        if self.masks.is_empty() {
            return true;
        }
        let m = &self.masks[frame.min(self.masks.len() - 1)];
        let stride = (self.mask_w + 7) / 8;
        let idx = py as usize * stride + (px as usize >> 3);
        idx < m.len() && (m[idx] & (0x80 >> (px & 7))) != 0
    }
}

#[derive(Deserialize)]
struct RawMask {
    w: usize,
    h: usize,
    masks: Vec<String>,
}
#[derive(Deserialize)]
struct RawSprite {
    w: f64,
    h: f64,
    ox: f64,
    oy: f64,
    bl: f64,
    br: f64,
    bt: f64,
    bb: f64,
    kind: i32,
    speed: f64,
    speedtype: i32,
    frames: Vec<Option<[f64; 7]>>,
    mask: Option<RawMask>,
    ns: Option<[f64; 4]>,
}
#[derive(Deserialize)]
struct RawFont {
    size: f64,
    #[allow(dead_code)]
    ascender: f64,
    glyphs: Vec<[i32; 7]>,
}
#[derive(Deserialize)]
pub struct RawSound {
    pub file: String,
    pub vol: f64,
}
#[derive(Deserialize)]
struct RawAssets {
    pages: Vec<String>,
    sprites: HashMap<String, RawSprite>,
    fonts: HashMap<String, RawFont>,
    sounds: HashMap<String, RawSound>,
}

#[derive(Clone, Copy)]
pub struct Glyph {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub shift: f32,
    pub offset: f32,
}
pub struct Font {
    pub tex: usize,
    pub size: f64,
    pub glyphs: HashMap<u32, Glyph>,
    pub line_h: f32,
}

pub type FontId = i32;

#[derive(Default)]
pub struct Assets {
    pub sprites: Vec<Sprite>,
    pub sprite_ids: HashMap<String, Spr>,
    pub fonts: Vec<Font>,
    pub font_ids: HashMap<String, FontId>,
    pub sounds: HashMap<String, RawSound>,
}

thread_local! {
    static SPRITE_IDS: RefCell<HashMap<String, Spr>> = RefCell::new(HashMap::new());
    static FONT_IDS: RefCell<HashMap<String, FontId>> = RefCell::new(HashMap::new());
}

/// Look up a sprite by its GameMaker name (e.g. `spr("spr_heart")`).
pub fn spr(name: &str) -> Spr {
    SPRITE_IDS.with(|m| {
        let m = m.borrow();
        match m.get(name) {
            Some(&s) => s,
            None => {
                if !m.is_empty() {
                    crate::log(&format!("missing sprite {name}"));
                }
                NO_SPR
            }
        }
    })
}
pub fn sprite_name(s: Spr) -> String {
    SPRITE_IDS.with(|m| m.borrow().iter().find(|(_, &v)| v == s).map(|(k, _)| k.clone()).unwrap_or_default())
}
pub fn font(name: &str) -> FontId { FONT_IDS.with(|m| *m.borrow().get(name).unwrap_or(&0)) }

async fn fetch_text(url: &str) -> Result<String, JsValue> {
    let win = web_sys::window().unwrap();
    let resp: web_sys::Response = JsFuture::from(win.fetch_with_str(url)).await?.dyn_into()?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!("failed to fetch {url}")));
    }
    let t = JsFuture::from(resp.text()?).await?;
    Ok(t.as_string().unwrap())
}

pub async fn load_image(url: &str) -> Result<web_sys::HtmlImageElement, JsValue> {
    let img = web_sys::HtmlImageElement::new()?;
    let p = js_sys::Promise::new(&mut |res, rej| {
        img.set_onload(Some(&res));
        img.set_onerror(Some(&rej));
    });
    img.set_src(url);
    JsFuture::from(p).await?;
    Ok(img)
}

impl Assets {
    pub async fn load(gfx: &mut Gfx, base: &str) -> Result<Assets, JsValue> {
        let txt = fetch_text(&format!("{base}/assets.json")).await?;
        let raw: RawAssets = serde_json::from_str(&txt).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut page_tex = vec![];
        for p in &raw.pages {
            let img = load_image(&format!("{base}/{p}")).await?;
            page_tex.push(gfx.add_texture_image(&img));
        }
        let mut a = Assets::default();
        let mut names: Vec<_> = raw.sprites.keys().cloned().collect();
        names.sort();
        for name in names {
            let r = &raw.sprites[&name];
            let frames = r
                .frames
                .iter()
                .map(|f| {
                    f.map(|f| Frame {
                        tex: page_tex[f[0] as usize],
                        x: f[1] as f32,
                        y: f[2] as f32,
                        w: f[3] as f32,
                        h: f[4] as f32,
                        offx: f[5] as f32,
                        offy: f[6] as f32,
                    })
                })
                .collect();
            let (masks, mw, mh) = match &r.mask {
                Some(m) => (
                    m.masks.iter().map(|s| base64::engine::general_purpose::STANDARD.decode(s).unwrap_or_default()).collect(),
                    m.w,
                    m.h,
                ),
                None => (vec![], r.w as usize, r.h as usize),
            };
            let id = a.sprites.len() as Spr;
            a.sprite_ids.insert(name.clone(), id);
            a.sprites.push(Sprite {
                name,
                w: r.w,
                h: r.h,
                ox: r.ox,
                oy: r.oy,
                bl: r.bl,
                br: r.br,
                bt: r.bt,
                bb: r.bb,
                kind: r.kind,
                speed: r.speed,
                speedtype: r.speedtype,
                frames,
                masks,
                mask_w: mw,
                mask_h: mh,
                nineslice: r.ns,
            });
        }
        let mut fnames: Vec<_> = raw.fonts.keys().cloned().collect();
        fnames.sort();
        for name in fnames {
            let f = &raw.fonts[&name];
            let img = load_image(&format!("{base}/fnt/{name}.png")).await?;
            let tex = gfx.add_texture_image(&img);
            let mut glyphs = HashMap::new();
            let mut line_h: f32 = 0.0;
            for g in &f.glyphs {
                glyphs.insert(
                    g[0] as u32,
                    Glyph { x: g[1] as f32, y: g[2] as f32, w: g[3] as f32, h: g[4] as f32, shift: g[5] as f32, offset: g[6] as f32 },
                );
                line_h = line_h.max(g[4] as f32);
            }
            let id = a.fonts.len() as FontId;
            a.font_ids.insert(name, id);
            a.fonts.push(Font { tex, size: f.size, glyphs, line_h });
        }
        a.sounds = raw.sounds;
        SPRITE_IDS.with(|m| *m.borrow_mut() = a.sprite_ids.clone());
        FONT_IDS.with(|m| *m.borrow_mut() = a.font_ids.clone());
        Ok(a)
    }

    pub fn get(&self, s: Spr) -> Option<&Sprite> {
        if s < 0 { None } else { self.sprites.get(s as usize) }
    }
}
