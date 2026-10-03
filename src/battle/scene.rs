//! Top-level flow: title screen → battle → (defeat flash → title | win → outcome screen → title).

use crate::assets::font;
use crate::gfx::{C_GRAY, C_WHITE, C_YELLOW};
use crate::input::Key;
use crate::rt::{Game, HAlign, Inst, VAlign};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Title,
    Battle,
    /// defeat: fountain-seal style light burst, then back to the title (retry) screen
    Flash,
    Outcome,
}

pub struct Scene {
    pub phase: Phase,
    timer: f64,
    title_cursor: usize,
    outcome: i32,
    /// how many times the battle was lost (scr_get_knight_total_attempts)
    pub attempts: f64,
    loaded_wait: f64,
    /// timer of the defeat flash (frames)
    flash_t: f64,
    /// fade-in from black when the title is shown after a defeat
    title_fade: f64,
}

impl Scene {
    pub fn new(_g: &mut Game) -> Scene {
        Scene { phase: Phase::Title, timer: 0.0, title_cursor: 0, outcome: 0, attempts: 0.0, loaded_wait: 0.0, flash_t: 0.0, title_fade: 0.0 }
    }
}

fn reset_world(g: &mut Game) {
    g.audio.stop_all();
    g.clear_all();
    g.glob = crate::rt::Glob::default();
    g.gfx.view_x = 0.0;
    g.gfx.view_y = 0.0;
}

fn start(s: &mut Scene, g: &mut Game) {
    reset_world(g);
    g.glob.set_ex("knight_attempts", s.attempts);
    // debug: ?attack=N&diff=D forces the Knight's attack every turn
    if let Some(q) = web_sys::window().and_then(|w| w.location().search().ok()) {
        if let Ok(p) = web_sys::UrlSearchParams::new_with_str(&q) {
            if let Some(a) = p.get("attack").and_then(|v| v.parse::<f64>().ok()) {
                g.glob.set_ex("debug_attack", a);
            }
            if let Some(d) = p.get("diff").and_then(|v| v.parse::<f64>().ok()) {
                g.glob.set_ex("debug_diff", d);
            }
            if p.get("god").is_some() {
                g.glob.set_ex("debug_god", 1.0);
            }
            if p.get("postroar").is_some() {
                g.glob.set_ex("debug_postroar", 1.0);
            }
            if let Some(h) = p.get("khp").and_then(|v| v.parse::<f64>().ok()) {
                g.glob.set_ex("debug_khp", h);
            }
        }
    }
    crate::battle::controller::start_battle(g);
    if g.glob.ex("debug_khp") > 0.0 {
        g.glob.monsterhp[0] = g.glob.ex("debug_khp");
    }
    if g.glob.ex("debug_postroar") != 0.0 {
        // as if the Roaring already happened: the next hit on the Knight ends the battle
        g.glob.monsterhp[0] = g.glob.monsterhp[0].min(5800.0);
        if let Some((_, k)) = g.get_first::<crate::battle::KnightEnemy>("obj_knight_enemy") {
            k.haveusedroaring = true;
            k.phase = 3.0;
        }
    }
    s.phase = Phase::Battle;
    s.timer = 0.0;
}

pub fn scene_update(s: &mut Scene, g: &mut Game) {
    s.timer += 1.0;
    match s.phase {
        Phase::Title => {
            s.loaded_wait += 1.0;
            if s.title_fade > 0.0 {
                // still fading in from the defeat flash: ignore input
                s.title_fade -= 0.05;
                return;
            }
            if g.input.pressed(Key::Up) || g.input.pressed(Key::Down) {
                s.title_cursor = 1 - s.title_cursor;
                g.snd_play("snd_menumove");
            }
            if g.input.pressed(Key::B1) {
                g.snd_play("snd_select");
                if s.title_cursor == 1 {
                    g.show_hitboxes = !g.show_hitboxes;
                } else {
                    start(s, g);
                }
            }
        }
        Phase::Battle => {
            if g.glob.ex("debug_god") != 0.0 {
                for c in 1..4 {
                    g.glob.hp[c] = g.glob.maxhp[c];
                }
            }
            let outcome = g.glob.ex("battle_outcome") as i32;
            if outcome != 0 {
                if s.outcome == 0 {
                    s.outcome = outcome;
                    s.timer = 0.0;
                }
                if outcome == 2 {
                    // the Knight was struck after the Roaring: let its white fade play, then show the result
                    if s.timer > 150.0 {
                        reset_world(g);
                        s.phase = Phase::Outcome;
                        s.timer = 0.0;
                    }
                } else {
                    // defeat: flash straight back to the retry screen
                    s.attempts += 1.0;
                    g.audio.stop_all();
                    g.snd_play("snd_dtrans_lw");
                    s.phase = Phase::Flash;
                    s.flash_t = 0.0;
                }
            }
        }
        Phase::Flash => {
            s.flash_t += 1.0;
            if s.flash_t == FLASH_CLEAR {
                // the screen is fully white: tear the battle down underneath
                reset_world(g);
            }
            if s.flash_t >= FLASH_END {
                s.outcome = 0;
                s.title_cursor = 0;
                s.title_fade = 1.0;
                s.phase = Phase::Title;
                s.timer = 0.0;
            }
        }
        Phase::Outcome => {
            if s.timer > 30.0 && g.input.pressed(Key::B1) {
                g.snd_play("snd_select");
                s.outcome = 0;
                s.phase = Phase::Title;
                s.timer = 0.0;
            }
        }
    }
}

pub fn scene_draw_under(_s: &mut Scene, _g: &mut Game) {}

// Defeat flash timeline (frames), modelled on obj_darkfountain_event (the fountain seal):
// light bands spread from the centre, the screen fills white, then fades to black.
const FLASH_WHITE: f64 = 25.0;
const FLASH_CLEAR: f64 = 55.0;
const FLASH_BLACK: f64 = 65.0;
const FLASH_END: f64 = 110.0;

/// The seal-style flash overlay drawn over the battle.
fn draw_flash(g: &mut Game, t: f64) {
    let gx = &mut g.gfx;
    // expanding vertical light bands (obj_darkfountain_event Draw, t >= 400)
    let rs = t * 0.6;
    gx.draw_set_color(C_WHITE);
    for i in 1..12 {
        let i = i as f64;
        gx.draw_set_alpha(((rs / 16.0) - (i / 12.0)).clamp(0.0, 1.0));
        gx.draw_rectangle(320.0 - i * i - rs * i, 0.0, 320.0 + i * i + rs * i, 500.0, false);
    }
    // full white
    if t >= FLASH_WHITE {
        gx.draw_set_alpha(((t - FLASH_WHITE) * 0.04).min(1.0));
        gx.draw_rectangle(-10.0, -10.0, 999.0, 999.0, false);
    }
    // fade to black
    if t >= FLASH_BLACK {
        gx.draw_set_color(0);
        gx.draw_set_alpha(((t - FLASH_BLACK) / (FLASH_END - FLASH_BLACK - 10.0)).min(1.0));
        gx.draw_rectangle(-10.0, -10.0, 999.0, 999.0, false);
    }
    gx.draw_set_alpha(1.0);
    gx.draw_set_color(C_WHITE);
}

/// The disclaimer is an HTML element (index.html #disclaimer) so it stays legible and the link is clickable;
/// it is only shown on the title screen.
fn set_disclaimer_visible(visible: bool) {
    use wasm_bindgen::JsCast;
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("disclaimer"))
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = el.style().set_property("display", if visible { "block" } else { "none" });
    }
}

fn centered(g: &mut Game, y: f64, text: &str, col: u32, scale: f64) {
    g.draw_set_halign(HAlign::Center);
    g.draw_set_valign(VAlign::Top);
    let a = g.gfx.draw_alpha;
    g.draw_text_transformed_color(320.0, y, text, scale, scale, 0.0, [col; 4], a);
    g.draw_set_halign(HAlign::Left);
}

pub fn scene_draw_over(s: &mut Scene, g: &mut Game) {
    set_disclaimer_visible(s.phase == Phase::Title && s.title_fade <= 0.0);
    match s.phase {
        Phase::Title => {
            g.gfx.draw_set_alpha(1.0);
            let knight = crate::assets::spr("spr_roaringknight_idle");
            let bob = (s.timer / 16.0).sin() * 6.0;
            g.draw_sprite_ext(knight, 0.0, 268.0, 70.0 + bob, 2.0, 2.0, 0.0, C_WHITE, 1.0);
            g.draw_set_font(font("fnt_mainbig"));
            centered(g, 296.0, "THE ROARING KNIGHT", C_WHITE, 1.0);
            g.draw_set_font(font("fnt_main"));
            let items = [if s.attempts > 0.0 { "TRY AGAIN" } else { "BEGIN" }, if g.show_hitboxes { "HITBOXES: ON" } else { "HITBOXES: OFF" }];
            for (i, it) in items.iter().enumerate() {
                let col = if i == s.title_cursor { C_YELLOW } else { C_WHITE };
                centered(g, 346.0 + i as f64 * 22.0, it, col, 1.0);
            }
            if s.attempts > 0.0 {
                g.draw_set_font(font("fnt_small"));
                centered(g, 326.0, &format!("ATTEMPTS: {}", s.attempts), C_GRAY, 1.0);
                g.draw_set_font(font("fnt_main"));
            }
            g.draw_set_font(font("fnt_main"));
            centered(g, 400.0, "Z confirm   X cancel   C menu   F fullscreen", C_GRAY, 1.0);
            if s.title_fade > 0.0 {
                g.gfx.draw_set_color(0);
                g.gfx.draw_set_alpha(s.title_fade.min(1.0));
                g.gfx.draw_rectangle(-10.0, -10.0, 999.0, 999.0, false);
                g.gfx.draw_set_alpha(1.0);
                g.gfx.draw_set_color(C_WHITE);
            }
        }
        Phase::Battle => {}
        Phase::Flash => draw_flash(g, s.flash_t),
        Phase::Outcome => {
            g.draw_set_font(font("fnt_mainbig"));
            let (title, body) = match s.outcome {
                2 => ("BATTLE OVER", "You struck the Knight after the Roaring."),
                3 => ("GAME OVER", "The party was defeated."),
                _ => ("BATTLE OVER", "The party was defeated."),
            };
            centered(g, 190.0, title, C_WHITE, 1.0);
            g.draw_set_font(font("fnt_main"));
            centered(g, 240.0, body, C_GRAY, 1.0);
            if s.timer > 30.0 {
                centered(g, 400.0, "Press Z", C_WHITE, 1.0);
            }
        }
    }
}

/// scr_damage(): routed to the soul module.
pub fn scr_damage_impl(me: &mut Inst, g: &mut Game) { crate::battle::soul::scr_damage_impl(me, g) }
/// `mytarget` of the knight (set by scr_randomtarget during its turn).
pub fn mytarget(g: &mut Game) -> f64 {
    g.get_first::<crate::battle::KnightEnemy>("obj_knight_enemy").map(|(_, k)| k.mytarget).unwrap_or(4.0)
}
