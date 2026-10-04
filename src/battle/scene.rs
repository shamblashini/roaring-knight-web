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
    /// title option: skip the defeat flash and restart the battle right away
    pub instant_retry: bool,
    /// the current flash is the short instant-retry one
    flash_quick: bool,
    /// white overlay fading out at the start of an instantly retried battle
    battle_fade: f64,
    /// hold-Escape counter (obj_time quit_timer): 30 frames held = quit to the title
    quit_timer: f64,
    /// master volume in 10% steps (0..=10)
    pub volume: i32,
}

const STORAGE_KEY_INSTANT: &str = "rk_instant_retry";
const STORAGE_KEY_VOLUME: &str = "rk_volume";

fn apply_volume(g: &mut Game, steps: i32) {
    g.audio.set_master(steps as f64 / 10.0);
}

fn storage() -> Option<web_sys::Storage> { web_sys::window().and_then(|w| w.local_storage().ok().flatten()) }

impl Scene {
    pub fn new(g: &mut Game) -> Scene {
        let volume = storage()
            .and_then(|st| st.get_item(STORAGE_KEY_VOLUME).ok().flatten())
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(10)
            .clamp(0, 10);
        // ?mute=1 (testing) keeps the audio off regardless of the saved volume
        if g.audio.master > 0.0 {
            apply_volume(g, volume);
        }
        let instant_retry = storage().and_then(|st| st.get_item(STORAGE_KEY_INSTANT).ok().flatten()).as_deref() == Some("1");
        Scene {
            phase: Phase::Title,
            timer: 0.0,
            title_cursor: 0,
            outcome: 0,
            attempts: 0.0,
            loaded_wait: 0.0,
            flash_t: 0.0,
            title_fade: 0.0,
            instant_retry,
            flash_quick: false,
            battle_fade: 0.0,
            quit_timer: 0.0,
            volume,
        }
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

/// Title menu entries, top to bottom.
const MENU_START: usize = 0;
const MENU_VOLUME: usize = 1;
const MENU_INSTANT: usize = 2;
const MENU_HITBOXES: usize = 3;
const MENU_COUNT: usize = 4;

fn muted_by_url() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|q| web_sys::UrlSearchParams::new_with_str(&q).ok())
        .and_then(|p| p.get("mute"))
        .is_some()
}

/// Back to the title screen (fading in from black).
fn to_title(s: &mut Scene, g: &mut Game) {
    reset_world(g);
    s.outcome = 0;
    s.title_cursor = MENU_START;
    s.title_fade = 1.0;
    s.battle_fade = 0.0;
    s.phase = Phase::Title;
    s.timer = 0.0;
}

pub fn scene_update(s: &mut Scene, g: &mut Game) {
    s.timer += 1.0;
    // Hold Escape to quit (obj_time Step_1): +1 per frame held, -2 per frame released, quit at 30 (1 second).
    if s.phase != Phase::Title {
        if g.input.held(Key::Quit) {
            if s.quit_timer < 0.0 {
                s.quit_timer = 0.0;
            }
            s.quit_timer += 1.0;
            if s.quit_timer >= 30.0 {
                s.quit_timer = 0.0;
                to_title(s, g);
                return;
            }
        } else {
            s.quit_timer -= 2.0;
        }
    } else {
        s.quit_timer = 0.0;
    }
    match s.phase {
        Phase::Title => {
            s.loaded_wait += 1.0;
            if s.title_fade > 0.0 {
                // still fading in from the defeat flash: ignore input
                s.title_fade -= 0.05;
                return;
            }
            if g.input.pressed(Key::Up) {
                s.title_cursor = (s.title_cursor + MENU_COUNT - 1) % MENU_COUNT;
                g.snd_play("snd_menumove");
            }
            if g.input.pressed(Key::Down) {
                s.title_cursor = (s.title_cursor + 1) % MENU_COUNT;
                g.snd_play("snd_menumove");
            }
            if s.title_cursor == MENU_VOLUME && (g.input.pressed(Key::Left) || g.input.pressed(Key::Right)) {
                let d = if g.input.pressed(Key::Left) { -1 } else { 1 };
                let v = (s.volume + d).clamp(0, 10);
                if v != s.volume {
                    s.volume = v;
                    if let Some(st) = storage() {
                        let _ = st.set_item(STORAGE_KEY_VOLUME, &v.to_string());
                    }
                    if !muted_by_url() {
                        apply_volume(g, v);
                    }
                    // preview at the new level
                    g.snd_play("snd_menumove");
                }
            }
            if g.input.pressed(Key::B1) && s.title_cursor != MENU_VOLUME {
                g.snd_play("snd_select");
                match s.title_cursor {
                    MENU_INSTANT => {
                        s.instant_retry = !s.instant_retry;
                        if let Some(st) = storage() {
                            let _ = st.set_item(STORAGE_KEY_INSTANT, if s.instant_retry { "1" } else { "0" });
                        }
                    }
                    MENU_HITBOXES => g.show_hitboxes = !g.show_hitboxes,
                    _ => start(s, g),
                }
            }
        }
        Phase::Battle => {
            if s.battle_fade > 0.0 {
                s.battle_fade -= 0.1;
            }
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
                    // defeat: flash straight back to the retry screen (or, with INSTANT RETRY, into a new battle)
                    s.attempts += 1.0;
                    g.audio.stop_all();
                    s.flash_quick = s.instant_retry;
                    if !s.flash_quick {
                        g.snd_play("snd_dtrans_lw");
                    }
                    s.phase = Phase::Flash;
                    s.flash_t = 0.0;
                }
            }
        }
        Phase::Flash if s.flash_quick => {
            // instant retry: a quick white-out, then a fresh battle that fades in from white
            s.flash_t += 1.0;
            if s.flash_t >= QUICK_FLASH {
                s.outcome = 0;
                start(s, g);
                s.battle_fade = 1.0;
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
/// instant retry: frames of white-out before the new battle starts
const QUICK_FLASH: f64 = 8.0;

fn draw_fullscreen(g: &mut Game, col: u32, alpha: f64) {
    g.gfx.draw_set_color(col);
    g.gfx.draw_set_alpha(alpha.clamp(0.0, 1.0));
    g.gfx.draw_rectangle(-10.0, -10.0, 999.0, 999.0, false);
    g.gfx.draw_set_alpha(1.0);
    g.gfx.draw_set_color(C_WHITE);
}

/// "QUITTING..." while Escape is held (obj_time Draw_64).
fn draw_quit_message(g: &mut Game, t: f64) {
    if t >= 1.0 {
        let spr = crate::assets::spr("spr_quitmessage");
        g.draw_sprite_ext(spr, t / 7.0, 4.0, 4.0, 2.0, 2.0, 0.0, C_WHITE, t / 15.0);
    }
}

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

/// `VOLUME ▮▮▮▮▮▮▮□□□ 70%` (with `<` `>` hints while selected), centred on the screen.
fn draw_volume_slider(g: &mut Game, y: f64, steps: i32, col: u32, selected: bool) {
    let (seg, gap) = (9.0, 3.0);
    let bar_w = 10.0 * seg;
    let label = "VOLUME ";
    let pct = format!(" {}%", steps * 10);
    let (lw, pw) = (g.string_width(label), g.string_width(" 100%"));
    let total = lw + bar_w + pw;
    let x0 = (320.0 - total / 2.0).floor();
    g.draw_set_halign(HAlign::Left);
    g.draw_set_valign(VAlign::Top);
    g.draw_text_transformed_color(x0, y, label, 1.0, 1.0, 0.0, [col; 4], 1.0);
    let bx = x0 + lw;
    for k in 0..10 {
        let c = if k < steps { col } else { C_GRAY };
        g.gfx.draw_set_color(c);
        let sx = bx + k as f64 * seg;
        g.gfx.draw_rectangle(sx, y + 4.0, sx + seg - gap, y + 11.0, k >= steps);
    }
    g.gfx.draw_set_color(C_WHITE);
    g.draw_text_transformed_color(bx + bar_w, y, &pct, 1.0, 1.0, 0.0, [col; 4], 1.0);
    if selected {
        g.draw_text_transformed_color(x0 - 18.0, y, "<", 1.0, 1.0, 0.0, [col; 4], 1.0);
        g.draw_text_transformed_color(x0 + total + 8.0, y, ">", 1.0, 1.0, 0.0, [col; 4], 1.0);
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
            let items = [
                (if s.attempts > 0.0 { "TRY AGAIN" } else { "BEGIN" }).to_string(),
                String::new(), // volume slider, drawn below
                (if s.instant_retry { "INSTANT RETRY: ON" } else { "INSTANT RETRY: OFF" }).to_string(),
                (if g.show_hitboxes { "HITBOXES: ON" } else { "HITBOXES: OFF" }).to_string(),
            ];
            for (i, it) in items.iter().enumerate() {
                let col = if i == s.title_cursor { C_YELLOW } else { C_WHITE };
                if i == MENU_VOLUME {
                    draw_volume_slider(g, 342.0 + i as f64 * 20.0, s.volume, col, i == s.title_cursor);
                } else {
                    centered(g, 342.0 + i as f64 * 20.0, it, col, 1.0);
                }
            }
            if s.attempts > 0.0 {
                g.draw_set_font(font("fnt_small"));
                centered(g, 326.0, &format!("ATTEMPTS: {}", s.attempts), C_GRAY, 1.0);
                g.draw_set_font(font("fnt_main"));
            }
            g.draw_set_font(font("fnt_main"));
            centered(g, 430.0, "Z confirm   X cancel   C menu   F fullscreen   hold ESC quit", C_GRAY, 1.0);
            if s.title_fade > 0.0 {
                g.gfx.draw_set_color(0);
                g.gfx.draw_set_alpha(s.title_fade.min(1.0));
                g.gfx.draw_rectangle(-10.0, -10.0, 999.0, 999.0, false);
                g.gfx.draw_set_alpha(1.0);
                g.gfx.draw_set_color(C_WHITE);
            }
        }
        Phase::Battle => {
            if s.battle_fade > 0.0 {
                draw_fullscreen(g, C_WHITE, s.battle_fade);
            }
        }
        Phase::Flash if s.flash_quick => draw_fullscreen(g, C_WHITE, s.flash_t / QUICK_FLASH),
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
    if s.phase != Phase::Title {
        draw_quit_message(g, s.quit_timer);
    }
}

/// scr_damage(): routed to the soul module.
pub fn scr_damage_impl(me: &mut Inst, g: &mut Game) { crate::battle::soul::scr_damage_impl(me, g) }
/// `mytarget` of the knight (set by scr_randomtarget during its turn).
pub fn mytarget(g: &mut Game) -> f64 {
    g.get_first::<crate::battle::KnightEnemy>("obj_knight_enemy").map(|(_, k)| k.mytarget).unwrap_or(4.0)
}
