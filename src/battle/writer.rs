//! Text: obj_writer (typewriter with Deltarune control codes), obj_face, obj_battleblcon (enemy balloons),
//! msgset / msgnext / scr_speaker / scr_anyface(_next) / scr_battletext / scr_battletext_default /
//! scr_enemyblcon / scr_terminate_writer, scr_texttype / scr_textsetup / scr_textsound.
//! OWNER: battle-core agent "writer".
//!
//! English path only. Strings are handled as `Vec<char>` with GML's 1-based indexing.

use crate::assets::{font, spr, FontId};
use crate::gfx::{Color, C_BLACK, C_BLUE, C_DKGRAY, C_LIME, C_MAROON, C_NAVY, C_PURPLE, C_RED, C_WHITE, C_YELLOW};
use crate::input::Key;
use crate::obj_vars;
use crate::rt::{Game, HAlign, Id, Inst, Object, VAlign, NOONE};

// ============================================================================ string helpers (GML semantics)

/// string_char_at(s, i): 1-based. Index < 1 is clamped to 1 (GameMaker runner behaviour); past the end → '\0' ("").
fn char_at(s: &[char], i: i64) -> char {
    if s.is_empty() {
        return '\0';
    }
    let i = i.max(1);
    if i as usize > s.len() { '\0' } else { s[(i - 1) as usize] }
}
/// string_insert(sub, s, index) (1-based; clamped).
fn string_insert(sub: &str, s: &mut Vec<char>, index: i64) {
    let at = (index.max(1) - 1).min(s.len() as i64) as usize;
    for (k, c) in sub.chars().enumerate() {
        s.insert(at + k, c);
    }
}
/// string_delete(s, index, count) (1-based).
fn string_delete(s: &mut Vec<char>, index: i64, count: i64) {
    if index < 1 || index as usize > s.len() {
        return;
    }
    let at = (index - 1) as usize;
    let end = (at + count.max(0) as usize).min(s.len());
    s.drain(at..end);
}

/// `\E<c>` → global.fe
fn face_emotion(c: char) -> Option<f64> {
    let o = c as u32;
    if (48..=57).contains(&o) {
        Some((o - 48) as f64)
    } else if (65..=90).contains(&o) {
        Some(o as f64 - 55.0)
    } else if (97..=122).contains(&o) {
        Some(o as f64 - 61.0)
    } else {
        None
    }
}
/// `\F<c>` → global.fc. `with_b`: the Draw event's table also knows 'b' (Other_15's doesn't).
fn face_char(c: char, with_b: bool) -> Option<f64> {
    Some(match c {
        '0' => 0.0,
        'S' => 1.0,
        'R' => 2.0,
        'N' => 3.0,
        'T' => 4.0,
        'L' => 5.0,
        's' => 6.0,
        'U' => 9.0,
        'A' => 10.0,
        'a' => 11.0,
        'B' => 12.0,
        'b' if with_b => 19.0,
        'r' => 15.0,
        'u' => 18.0,
        'K' => 20.0,
        'Q' => 21.0,
        _ => return None,
    })
}

// ============================================================================ scr_texttype table

/// scr_textsetup arguments: (font, colour, charline, shake, rate, sound ("" = snd_nosound), hspace, vspace, special, y offset)
struct TextSetup {
    font: &'static str,
    color: Color,
    charline: f64,
    shake: f64,
    rate: f64,
    sound: &'static str,
    hspace: f64,
    vspace: f64,
    special: f64,
    yoff: f64,
}

/// Typers reachable in the Roaring Knight fight (battle default 4, battle message 6, anyface \TX = 40,
/// \TS/\TR/\T0 = 47/45/6, balloons 75/50, Tenna 81, scr_speaker variants). Others → None (font_set = false).
fn typer_setup(typer: i64) -> Option<TextSetup> {
    let t = |font, color, charline, shake, rate, sound, hspace, vspace, special| TextSetup {
        font,
        color,
        charline,
        shake,
        rate,
        sound,
        hspace,
        vspace,
        special,
        yoff: 0.0,
    };
    Some(match typer {
        1 => t("fnt_main", C_WHITE, 33.0, 0.0, 1.0, "snd_text", 8.0, 18.0, 0.0),
        2 => t("fnt_main", C_WHITE, 33.0, 0.0, 2.0, "", 8.0, 18.0, 0.0),
        3 => t("fnt_main", C_WHITE, 33.0, 0.0, 2.0, "snd_text", 8.0, 18.0, 1.0),
        4 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 1.0, "snd_text", 16.0, 28.0, 1.0),
        5 => t("fnt_main", C_WHITE, 33.0, 0.0, 1.0, "snd_text", 8.0, 18.0, 0.0),
        6 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 1.0, "snd_text", 16.0, 36.0, 1.0),
        10 | 11 => t("fnt_main", C_WHITE, 33.0, 0.0, 1.0, "snd_txtsus", 8.0, 18.0, 0.0),
        15 | 19 => t("fnt_main", C_WHITE, 33.0, 0.0, 1.0, "snd_text", 8.0, 18.0, 0.0),
        30 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 1.0, "snd_txtsus", 16.0, 36.0, 1.0),
        31 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 1.0, "snd_txtral", 16.0, 36.0, 1.0),
        36 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 1.0, "", 16.0, 36.0, 1.0),
        37 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 3.0, "snd_txtsus", 18.0, 36.0, 1.0),
        40 => t("fnt_main", C_WHITE, 33.0, 0.0, 2.0, "", 8.0, 18.0, 0.0),
        41 => t("fnt_main", C_WHITE, 33.0, 0.0, 3.0, "", 8.0, 18.0, 0.0),
        42 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 2.0, "", 16.0, 36.0, 1.0),
        45 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 1.0, "snd_txtral", 16.0, 28.0, 1.0),
        47 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 1.0, "snd_txtsus", 16.0, 28.0, 1.0),
        50 => t("fnt_dotumche", C_BLACK, 33.0, 0.0, 1.0, "snd_text", 9.0, 20.0, 0.0),
        53 => t("fnt_dotumche", C_BLACK, 33.0, 0.0, 1.0, "snd_txtsus", 9.0, 20.0, 0.0),
        54 => t("fnt_dotumche", C_BLACK, 33.0, 0.0, 2.0, "snd_txtsus", 9.0, 20.0, 0.0),
        60 => t("fnt_main", C_WHITE, 33.0, 0.0, 2.0, "snd_txtral", 12.0, 20.0, 0.0),
        61 => t("fnt_main", C_WHITE, 33.0, 0.0, 2.0, "snd_txtsus", 12.0, 20.0, 0.0),
        74 => t("fnt_dotumche", C_BLACK, 33.0, 0.0, 1.0, "snd_txtral", 9.0, 20.0, 0.0),
        75 => t("fnt_dotumche", C_BLACK, 33.0, 0.0, 1.0, "snd_txtsus", 9.0, 20.0, 0.0),
        78 => t("fnt_mainbig", C_WHITE, 36.0, 0.0, 1.0, "snd_text", 16.0, 36.0, 1.0),
        80 => t("fnt_mainbig", C_WHITE, 33.0, 0.0, 1.0, "snd_tv_voice_short", 16.0, 36.0, 1.0),
        81 => t("fnt_dotumche", C_BLACK, 33.0, 0.0, 1.0, "snd_tv_voice_short", 9.0, 20.0, 0.0),
        _ => return None,
    })
}

const TV_VOICES: [&str; 9] = [
    "snd_tv_voice_short",
    "snd_tv_voice_short_2",
    "snd_tv_voice_short_3",
    "snd_tv_voice_short_4",
    "snd_tv_voice_short_5",
    "snd_tv_voice_short_6",
    "snd_tv_voice_short_7",
    "snd_tv_voice_short_8",
    "snd_tv_voice_short_9",
];

// ============================================================================ obj_writer

/// obj_writer
#[derive(Default)]
pub struct Writer {
    pub reachedend: f64,
    pub skipme: f64,
    pub dialoguer: f64,
    pub facer: f64,
    // ---- Create_0
    pub forcebutton1: f64,
    /// sound asset name; "" = snd_nosound
    pub textsound: &'static str,
    pub charline: f64,
    pub originalcharline: f64,
    pub hspace: f64,
    pub vspace: f64,
    pub rate: f64,
    pub mycolor: Color,
    pub myfont: FontId,
    /// font asset name of `myfont` (GML compares font indices; fnt_comicsans is index 8)
    pub myfont_name: &'static str,
    pub textscale: f64,
    pub textalpha: f64,
    pub textalphagain: f64,
    pub fadeonend: f64,
    pub shake: f64,
    pub special: f64,
    pub shadcolor: Color,
    pub skippable: f64,
    pub automash_timer: f64,
    pub f: f64,
    pub prevent_mash_buffer: f64,
    pub autoaster: f64,
    pub drawaster: f64,
    pub pos: i64,
    pub lineno: f64,
    pub aster: f64,
    pub halt: f64,
    pub xcolor: Color,
    pub wxskip: f64,
    pub msgno: usize,
    pub first_alarm: f64,
    pub firstnoise: f64,
    pub formatted: f64,
    pub colorchange: f64,
    pub sound_played: f64,
    pub sound_timer: f64,
    pub reachedend_sound_play: f64,
    pub reachedend_sound_played: f64,
    pub writingx: f64,
    pub writingy: f64,
    pub siner: f64,
    pub specfade: f64,
    pub autocenter: f64,
    pub miniface_pos: f64,
    pub miniface_current_pos: f64,
    pub specx: [f64; 7],
    pub specy: [f64; 7],
    pub mystring: Vec<char>,
    pub nstring: Vec<String>,
    pub length: i64,
    // ---- Other_15 (formatting) results, read by obj_battleblcon
    pub stringmax: f64,
    pub linecount: f64,
}

impl Writer {
    /// scr_texttype() + scr_textsetup(): apply global.typer's settings.
    fn texttype(&mut self, me: &Inst, g: &Game) {
        self.textscale = 1.0;
        let Some(t) = typer_setup(g.glob.typer as i64) else { return };
        self.myfont = font(t.font);
        self.myfont_name = t.font;
        self.mycolor = t.color;
        self.writingx = me.x;
        self.writingy = me.y + t.yoff;
        self.charline = t.charline;
        self.shake = t.shake;
        self.rate = t.rate;
        self.textsound = t.sound;
        self.hspace = t.hspace;
        self.vspace = t.vspace;
        self.special = t.special;
        self.colorchange = 1.0;
        self.xcolor = self.mycolor;
    }

    fn ch(&self, i: i64) -> char { char_at(&self.mystring, i) }

    fn set_string(&mut self, s: &str) { self.mystring = s.chars().collect(); }

    /// scr_nextmsg()
    fn nextmsg(&mut self, me: &mut Inst) {
        self.msgno += 1;
        self.lineno = 0.0;
        self.aster = 0.0;
        self.halt = 0.0;
        self.pos = 1;
        me.alarm[0] = 1;
        self.drawaster = 1.0;
        self.autoaster = 1.0;
        self.miniface_pos = 0.0;
        self.miniface_current_pos = -1.0;
        let s = self.nstring.get(self.msgno).cloned().unwrap_or_default();
        self.set_string(&s);
        self.formatted = 0.0;
        self.wxskip = 0.0;
        self.sound_played = 0.0;
        self.reachedend = 0.0;
        self.reachedend_sound_played = 0.0;
        self.forcebutton1 = 0.0;
        if self.rate < 3.0 {
            self.firstnoise = 0.0;
            me.alarm[2] = 1;
        }
    }

    /// scr_textsound()
    fn textsound(&mut self, g: &mut Game) {
        let mut playtextsound = true;
        if g.input.held(Key::B2) {
            playtextsound = false;
        }
        if self.skippable == 0.0 {
            playtextsound = true;
        }
        if !playtextsound {
            return;
        }
        let mut getchar = if self.rate <= 2.0 { self.ch(self.pos) } else { self.ch(self.pos - 1) };
        let mut play = true;
        if getchar == '&' || getchar == '\n' {
            if self.rate < 3.0 {
                getchar = self.ch(self.pos + 1);
            } else {
                play = false;
            }
        }
        if matches!(getchar, ' ' | '^' | '!' | '.' | '?' | ',' | ':' | '/' | '\\' | '|' | '*') {
            play = false;
        }
        if !play {
            return;
        }
        if self.textsound == "snd_tv_voice_short" {
            let rand = g.irandom(8.0) as usize + 1;
            if g.glob.flag(1054) <= 0.0 {
                g.glob.set_flag(1054, 1.0);
            }
            let pitchrandom = (0.86 + g.random(0.35)) * g.glob.flag(1054);
            for v in TV_VOICES {
                g.snd_stop(v);
            }
            g.snd_play_x(TV_VOICES[rand - 1], 0.7, pitchrandom);
            self.sound_timer = 3.0;
        } else if !self.textsound.is_empty() {
            g.snd_play(self.textsound);
        }
        g.with_all("obj_face_parent", |_, o, _| {
            if let Some(f) = o.as_any().downcast_mut::<Face>() {
                f.mouthmove = 1.0;
            }
        });
        self.miniface_pos += 1.0;
    }

    /// Other_15 (event_user(5)): word wrap / line formatting.
    pub fn format(&mut self, me: &mut Inst, g: &mut Game) {
        if self.formatted != 0.0 {
            return;
        }
        self.length = self.mystring.len() as i64;
        let mut charpos: f64 = -1.0;
        let mut remspace: i64 = -1;
        let mut remchar: f64 = -1.0;
        self.linecount = 0.0;
        self.stringmax = 0.0;
        self.aster = 0.0;
        let mut i: i64 = 0;
        while i < self.length + 1 {
            let mut skip = false;
            let thischar = self.ch(i);
            if thischar == '`' {
                i += 1;
            } else if thischar == '/' || thischar == '%' {
                if charpos > -1.0 {
                    charpos -= 1.0;
                }
            } else if thischar == '^' {
                if charpos > -1.0 {
                    charpos -= 2.0;
                }
            } else if thischar == '\\' {
                if charpos > -1.0 {
                    charpos -= 3.0;
                }
                if self.dialoguer == 1.0 {
                    let nextchar = self.ch(i + 1);
                    let nextchar2 = self.ch(i + 2);
                    if nextchar == 'E' {
                        if let Some(fe) = face_emotion(nextchar2) {
                            g.glob.fe = fe;
                        }
                    }
                    if nextchar == 'F' {
                        if let Some(fc) = face_char(nextchar2, false) {
                            g.glob.fc = fc;
                        }
                        if g.glob.fc == 0.0 {
                            self.charline = self.originalcharline;
                            self.writingx = me.x;
                        } else {
                            self.charline = 26.0;
                            self.writingx = me.x + 58.0 * self.f;
                        }
                    }
                    if nextchar == 'm' {
                        self.drawaster = 0.0;
                    }
                    if nextchar == 's' && nextchar2 == '0' {
                        self.skippable = 0.0;
                    }
                    // `\O` (writer objects / obj_funnytext) is not used in this fight.
                }
            } else if thischar == '&' || thischar == '\n' {
                if charpos > self.stringmax {
                    self.stringmax = charpos;
                }
                remspace = -1;
                charpos = 0.0;
                self.linecount += 1.0;
                skip = true;
                let nextchar = self.ch(i + 1);
                if self.aster == 1.0 && self.autoaster == 1.0 && nextchar != '*' {
                    charpos = 2.0;
                    self.length += 2;
                    string_insert("||", &mut self.mystring, i + 1);
                    i += 2;
                }
            }
            if !skip {
                if thischar == ' ' {
                    remspace = i;
                    remchar = charpos;
                }
                if thischar == '*' {
                    self.aster = 1.0;
                }
                if charpos >= self.charline {
                    if remspace > 2 {
                        string_delete(&mut self.mystring, remspace, 1);
                        string_insert("&", &mut self.mystring, remspace);
                        i = remspace + 1;
                        if remchar > self.stringmax {
                            self.stringmax = remchar;
                        }
                        remspace = -1;
                        charpos = 1.0;
                        self.linecount += 1.0;
                        self.asterskip(i);
                    } else {
                        if charpos > self.stringmax {
                            self.stringmax = charpos;
                        }
                        string_insert("&", &mut self.mystring, i);
                        self.length += 1;
                        charpos = 1.0;
                        remspace = -1;
                        self.linecount += 1.0;
                        i += 1;
                        self.asterskip(i);
                    }
                } else {
                    charpos += 1.0;
                }
            }
            i += 1;
        }
        if self.autocenter == 1.0 {
            me.x = (g.camerax() + g.camerawidth() / 2.0) - (self.stringmax * self.hspace) / 2.0 + 5.0;
            me.y = (g.cameray() + g.cameraheight() / 2.0) - (self.writingy + (self.linecount + 1.0) * self.vspace) / 2.0 - 10.0;
        }
        if charpos > self.stringmax {
            self.stringmax = charpos;
        }
        self.formatted = 1.0;
    }

    /// scr_asterskip() (uses the caller's loop index `i`)
    fn asterskip(&mut self, i: i64) {
        if self.aster == 1.0 && self.autoaster == 1.0 {
            self.length += 2;
            string_insert("||", &mut self.mystring, i);
        }
        if self.aster == 2.0 {
            self.aster = 1.0;
        }
    }

    /// Draw one character with the current `special` style (inner part of Draw_0).
    fn draw_char(&mut self, g: &mut Game, wx: f64, wy: f64, s: &str, n: i64) {
        if self.special == 0.0 {
            let (ox, oy) = (g.random(self.shake), g.random(self.shake));
            g.draw_text_transformed(wx + ox, wy + oy, s, self.textscale, self.textscale, 0.0);
        }
        if self.special >= 1.0 {
            if self.special == 1.0 {
                let c = g.gfx.draw_get_color();
                if c != 16777215 && c != 0 {
                    let xc = self.xcolor;
                    let (a, b) = (g.random(self.shake), g.random(self.shake));
                    g.draw_text_color(wx + a + 1.0, wy + b + 1.0, s, xc, xc, xc, xc, 0.3);
                    let (a, b) = (g.random(self.shake), g.random(self.shake));
                    g.draw_text_color(wx + a, wy + b, s, C_WHITE, C_WHITE, xc, xc, 1.0);
                } else {
                    let (a, b) = (g.random(self.shake), g.random(self.shake));
                    g.draw_text_color(wx + a + 1.0, wy + b + 1.0, s, C_DKGRAY, C_DKGRAY, C_NAVY, C_NAVY, 1.0);
                    let (a, b) = (g.random(self.shake), g.random(self.shake));
                    g.draw_text(wx + a, wy + b, s);
                }
            }
            if self.special == 2.0 {
                let sf = self.specfade;
                let sn = (self.siner / 14.0).sin();
                g.gfx.draw_set_alpha(sf);
                g.draw_text(wx, wy, s);
                g.gfx.draw_set_alpha((0.3 + sn * 0.1) * sf);
                g.draw_text(wx + 1.0, wy, s);
                g.draw_text(wx - 1.0, wy, s);
                g.draw_text(wx, wy + 1.0, s);
                g.draw_text(wx, wy - 1.0, s);
                g.gfx.draw_set_alpha((0.08 + sn * 0.04) * sf);
                g.draw_text(wx + 1.0, wy + 1.0, s);
                g.draw_text(wx - 1.0, wy - 1.0, s);
                g.draw_text(wx - 1.0, wy + 1.0, s);
                g.draw_text(wx + 1.0, wy - 1.0, s);
                g.gfx.draw_set_alpha(1.0);
            }
            if self.special == 3.0 {
                let si = self.siner;
                g.gfx.draw_set_color(C_WHITE);
                g.gfx.draw_set_alpha(1.0);
                g.draw_text(wx + (si / 4.0).sin(), wy + (si / 4.0).cos(), s);
                g.gfx.draw_set_alpha(0.5);
                g.draw_text(wx + (si / 5.0).sin(), wy + (si / 5.0).cos(), s);
                g.draw_text(wx + (si / 7.0).sin(), wy + (si / 7.0).cos(), s);
                g.draw_text(wx + (si / 9.0).sin(), wy + (si / 9.0).cos(), s);
                for i in 0..7 {
                    let ddir = 315.0 + g.random(15.0);
                    if n == 1 {
                        self.specx[i] += crate::gm::lengthdir_x(2.0, ddir);
                        self.specy[i] += crate::gm::lengthdir_y(2.0, ddir);
                        if self.specx[i] >= 40.0 {
                            self.specx[i] = 0.0;
                            self.specy[i] = 0.0;
                        }
                    }
                    g.gfx.draw_set_alpha(((40.0 - self.specx[i]) / 40.0) * 0.7);
                    g.draw_text(wx + self.specx[i], wy + self.specy[i], s);
                }
                g.gfx.draw_set_alpha(1.0);
            }
            if self.special == 5.0 {
                g.gfx.draw_set_alpha(0.25);
                g.gfx.draw_set_color(self.shadcolor);
                let sh = self.shake;
                let (a, b) = (g.random(sh), g.random(sh));
                g.draw_text(wx + a, wy + 1.0 + b, s);
                let (a, b) = (g.random(sh), g.random(sh));
                g.draw_text(wx + 1.0 + a, wy + b, s);
                let (a, b) = (g.random(sh), g.random(sh));
                g.draw_text(wx + 1.0 + a, wy + 1.0 + b, s);
                g.gfx.draw_set_alpha(1.0);
                g.gfx.draw_set_color(C_BLACK);
                let (a, b) = (g.random(sh), g.random(sh));
                g.draw_text(wx + a, wy + b, s);
            }
        }
    }

    /// `\T<c>` in Draw_0: switch typer.
    fn typer_code(&mut self, c: char, g: &mut Game) -> bool {
        let dz = g.glob.darkzone == 1.0;
        let fighting = g.glob.fighting == 1.0;
        let t = match c {
            '0' => {
                if dz { 6.0 } else { 5.0 }
            }
            '1' => 2.0,
            'A' => 18.0,
            'a' => 20.0,
            'N' => {
                if fighting { 59.0 } else if dz { 56.0 } else { 12.0 }
            }
            'n' => 23.0,
            'B' => {
                if fighting { 77.0 } else if dz { 57.0 } else { 13.0 }
            }
            'S' => {
                if dz {
                    if fighting { 47.0 } else { 30.0 }
                } else {
                    10.0
                }
            }
            'R' => {
                let mut t = 31.0;
                if fighting {
                    t = 45.0;
                }
                if g.glob.flag(30) == 1.0 {
                    t = 6.0;
                }
                t
            }
            'L' => {
                if fighting { 46.0 } else { 32.0 }
            }
            'X' => 40.0,
            'r' => 55.0,
            'T' => 7.0,
            'J' => 35.0,
            'K' => {
                if fighting { 48.0 } else { 33.0 }
            }
            'q' => 62.0,
            'Q' => 58.0,
            's' => 14.0,
            'U' => 17.0,
            'p' => 67.0,
            'v' => {
                if fighting { 81.0 } else { 80.0 }
            }
            _ => return false,
        };
        g.glob.typer = t;
        true
    }
}

impl Object for Writer {
    fn name(&self) -> &'static str { "obj_writer" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.skipme = 0.0;
        self.forcebutton1 = 0.0;
        self.textsound = "snd_text";
        self.charline = 33.0;
        self.originalcharline = self.charline;
        self.hspace = 8.0;
        self.vspace = 18.0;
        self.rate = 1.0;
        self.mycolor = C_WHITE;
        self.myfont = font("fnt_main");
        self.myfont_name = "fnt_main";
        self.textalpha = 1.0;
        self.textalphagain = 0.0;
        self.fadeonend = 0.0;
        self.shake = 0.0;
        self.special = 0.0;
        self.shadcolor = 0x6FD213; // #13D26F
        self.skippable = 1.0;
        self.automash_timer = 0.0;
        if g.glob.flag(6) == 1.0 {
            self.skippable = 0.0;
        }
        self.f = 1.0;
        if g.glob.darkzone == 1.0 {
            self.f = 2.0;
        }
        self.prevent_mash_buffer = 0.0;
        self.texttype(me, g);
        self.autoaster = 1.0;
        self.drawaster = 1.0;
        self.pos = 2;
        self.lineno = 0.0;
        self.aster = 0.0;
        self.halt = 0.0;
        self.reachedend = 0.0;
        self.xcolor = C_BLACK;
        self.wxskip = 0.0;
        self.msgno = 0;
        self.first_alarm = 0.0;
        self.firstnoise = 0.0;
        self.formatted = 0.0;
        self.colorchange = 0.0;
        self.sound_played = 0.0;
        self.sound_timer = 0.0;
        self.reachedend_sound_played = 0.0;
        self.reachedend_sound_play = 0.0;
        self.writingx = me.x;
        self.writingy = me.y;
        self.dialoguer = 0.0;
        self.facer = 0.0;
        self.siner = 0.0;
        self.specfade = 1.0;
        self.autocenter = 0.0;
        self.miniface_current_pos = -1.0;
        self.miniface_pos = 0.0;
        for i in 0..7 {
            self.specx[i] = i as f64 * 6.0;
            self.specy[i] = i as f64 * 6.0;
        }
        let m0 = g.glob.msg.first().cloned().unwrap_or_default();
        self.set_string(&m0);
        self.nstring = (0..100).map(|j| g.glob.msg.get(j).cloned().unwrap_or_default()).collect();
        self.length = self.mystring.len() as i64;
        me.alarm[0] = self.rate as i32;
        if self.rate < 3.0 {
            me.alarm[2] = 1;
        } else {
            self.textsound(g);
        }
    }

    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            0 => {
                let mut playsound = false;
                if self.rate > 2.0 {
                    me.alarm[1] = 1;
                } else {
                    self.sound_timer -= 1.0;
                    if self.first_alarm == 1.0 && self.pos >= 2 && self.sound_timer <= 0.0 {
                        playsound = true;
                    }
                }
                if self.pos <= self.length {
                    me.alarm[0] = self.rate as i32;
                } else {
                    self.reachedend = 1.0;
                }
                if self.first_alarm == 0.0 {
                    if self.ch(1) == '\\' {
                        self.pos += 3;
                    } else {
                        self.pos += 1;
                    }
                    self.first_alarm = 1.0;
                } else {
                    let getchar = self.ch(self.pos);
                    let nextchar = self.ch(self.pos + 1);
                    if getchar == '`' {
                        self.pos += 2;
                        return;
                    }
                    if getchar == '&' || getchar == '\n' {
                        self.pos += 1;
                    }
                    if getchar == '\\' {
                        self.pos += 3;
                    }
                    if getchar == '/' {
                        self.halt = 1.0;
                        if nextchar == '%' {
                            self.halt = 2.0;
                        }
                        me.alarm[0] = -1;
                    }
                    let getchar = self.ch(self.pos);
                    let nextchar = self.ch(self.pos + 1);
                    if getchar == '|' {
                        self.pos += 2;
                    }
                    if getchar == '^' {
                        self.pos += 2;
                        if me.alarm[0] > 0 {
                            me.alarm[0] += match nextchar {
                                '1' => 5,
                                '2' => 10,
                                '3' => 15,
                                '4' => 20,
                                '5' => 30,
                                '6' => 40,
                                '7' => 60,
                                '8' => 90,
                                '9' => 150,
                                _ => 0,
                            };
                        }
                    }
                    self.pos += 1;
                }
                if self.reachedend_sound_play != 0.0 && self.reachedend == 1.0 && self.reachedend_sound_played == 0.0 {
                    playsound = false;
                }
                if playsound {
                    self.textsound(g);
                }
            }
            1 => {
                if self.pos < self.length + 2 {
                    self.textsound(g);
                }
            }
            2 => {
                if self.firstnoise == 0.0 {
                    self.firstnoise = 1.0;
                    self.textsound(g);
                }
            }
            _ => {}
        }
    }

    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 5 {
            self.format(me, g);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        // The runtime's text drawing honours halign/valign; the writer assumes left/top.
        let (rem_h, rem_v) = (g.halign, g.valign);
        g.draw_set_halign(HAlign::Left);
        g.draw_set_valign(VAlign::Top);

        let mut button1 = false;
        let mut button2 = false;
        if g.input.pressed(Key::B1) && self.prevent_mash_buffer <= 0.0 {
            button1 = true;
        }
        if g.input.held(Key::B2) && self.prevent_mash_buffer <= 0.0 {
            button2 = true;
        }
        if g.glob.flag(10) == 1.0 && g.input.held(Key::B3) {
            // auto-mash option (global.flag[10])
            self.prevent_mash_buffer = 3.0;
            self.automash_timer = if self.automash_timer == 0.0 { 1.0 } else { 0.0 };
            if self.automash_timer == 0.0 {
                button1 = true;
            }
            if self.automash_timer == 1.0 {
                button2 = true;
            }
        }
        if self.forcebutton1 != 0.0 {
            button1 = true;
        }
        self.prevent_mash_buffer -= 1.0;
        if self.dialoguer == 1.0 && self.formatted == 0.0 {
            if g.glob.fc == 0.0 {
                self.charline = self.originalcharline;
                self.writingx = me.x;
            } else {
                self.charline = 26.0;
                if g.glob.fc == 22.0 {
                    self.charline = 30.0;
                    self.vspace = 28.0;
                    // i_ex(obj_writer) is always true here (this instance)
                    self.vspace = 30.0;
                }
                self.writingx = me.x + 58.0 * self.f;
            }
        }
        if self.formatted == 0.0 {
            self.format(me, g);
        }
        let mut wx = self.writingx;
        let mut wy = self.writingy;
        self.colorchange = 0.0;
        g.draw_set_font(self.myfont);
        g.gfx.draw_set_color(self.mycolor);
        if self.fadeonend != 0.0 && self.reachedend == 1.0 {
            self.textalphagain = -self.fadeonend.abs();
            if self.textalpha <= 0.0 {
                g.destroy_self(me);
            }
        }
        if self.textalphagain != 0.0 {
            self.textalpha = (self.textalpha + self.textalphagain).clamp(0.0, 1.0);
        }
        if self.textalpha != 1.0 {
            g.gfx.draw_set_alpha(self.textalpha);
        }
        if self.halt == 0.0 && button2 && self.pos < self.length && self.skippable == 1.0 {
            self.skipme = 1.0;
        }
        if self.skipme == 1.0 {
            self.pos = self.mystring.len() as i64 + 1;
            self.reachedend = 1.0;
            me.alarm[0] = -1;
            me.alarm[1] = -1;
        }
        let mut buf = [0u8; 4];
        let mut n: i64 = 1;
        while n < self.pos {
            let mut accept = true;
            let mut mychar = self.ch(n);
            if mychar == '`' {
                n += 1;
                mychar = self.ch(n);
            } else if mychar == '&' || mychar == '\n' {
                accept = false;
                wx = self.writingx;
                if self.wxskip == 1.0 {
                    wx = self.writingx + 58.0;
                }
                wy += self.vspace;
            } else if mychar == '|' {
                accept = false;
                wx += self.hspace;
            } else if mychar == '^' {
                accept = false;
                n += 1;
            } else if mychar == '/' {
                self.halt = 1.0;
                if self.ch(n + 1) == '%' {
                    self.halt = 2.0;
                }
                self.reachedend = 1.0;
                accept = false;
            } else if mychar == '%' {
                accept = false;
                if self.ch(n - 1) == '/' {
                    self.halt = 2.0;
                }
                if self.ch(n + 1) == '%' {
                    g.destroy_self(me);
                } else if self.halt != 2.0 {
                    self.nextmsg(me);
                }
            } else if mychar == '\\' {
                let nextchar = self.ch(n + 1);
                let nextchar2 = self.ch(n + 2);
                match nextchar {
                    'E' => {
                        if let Some(fe) = face_emotion(nextchar2) {
                            g.glob.fe = fe;
                        }
                    }
                    'F' => {
                        if let Some(fc) = face_char(nextchar2, true) {
                            g.glob.fc = fc;
                        }
                        if self.dialoguer == 1.0 {
                            if g.glob.fc == 0.0 {
                                self.charline = self.originalcharline;
                                wx = me.x;
                            } else {
                                self.charline = 26.0;
                                wx = me.x + 58.0 * self.f;
                            }
                        }
                    }
                    'T' => {
                        if self.typer_code(nextchar2, g) {
                            self.texttype(me, g);
                        }
                        if self.dialoguer == 1.0 {
                            if g.glob.fc == 0.0 {
                                self.charline = self.originalcharline;
                                wx = me.x;
                            } else {
                                self.wxskip = 1.0;
                            }
                        }
                    }
                    's' => {
                        if nextchar2 == '0' {
                            self.skippable = 0.0;
                        }
                        if nextchar2 == '1' {
                            self.skippable = 1.0;
                        }
                    }
                    'c' => {
                        self.colorchange = 1.0;
                        match nextchar2 {
                            'R' => self.xcolor = C_RED,
                            'B' => self.xcolor = C_BLUE,
                            'Y' => self.xcolor = C_YELLOW,
                            'G' => self.xcolor = C_LIME,
                            'W' => self.xcolor = C_WHITE,
                            'X' => self.xcolor = C_BLACK,
                            'P' => self.xcolor = C_PURPLE,
                            'M' => self.xcolor = C_MAROON,
                            'S' => self.xcolor = 0xFF80FF,
                            'V' => self.xcolor = 0x80FF80,
                            'I' => self.xcolor = 0xFFC081, // #81C0FF
                            '0' => self.xcolor = self.mycolor,
                            _ => {}
                        }
                    }
                    'M' => {
                        if let Some(d) = nextchar2.to_digit(10) {
                            g.glob.set_flag(20, d as f64);
                        }
                    }
                    'm' => {
                        // mini-face portrait (global.writerimg) — not used in this fight; keep its side effect
                        self.drawaster = 0.0;
                    }
                    // `\f` smallfaces, `\*` button sprites, `\C` choicers, `\S` writer sounds, `\I`/`\O` writer
                    // images/objects: none of these appear in the Roaring Knight battle text.
                    _ => {}
                }
                accept = false;
                n += 2;
            }
            if accept {
                if self.drawaster == 0.0 && mychar == '*' {
                    mychar = ' ';
                }
                if self.colorchange == 1.0 {
                    g.gfx.draw_set_color(self.xcolor);
                }
                if mychar == '#' && self.ch(n - 1) != '`' {
                    mychar = '\n';
                }
                let s: &str = if mychar == '\0' { "" } else { mychar.encode_utf8(&mut buf) };
                let s = s.to_string();
                self.draw_char(g, wx, wy, &s, n);
                wx += self.hspace;
                if self.myfont_name == "fnt_comicsans" {
                    match mychar {
                        'w' => wx += 2.0,
                        'm' => wx += 3.0,
                        'i' => wx -= 2.0,
                        'l' => wx -= 2.0,
                        's' => wx -= 1.0,
                        'j' => wx -= 1.0,
                        _ => {}
                    }
                }
            }
            n += 1;
        }
        // reachedend_sound_play is never enabled in this fight (reachedend_sound = 259).
        if self.halt != 0.0 && button1 && self.siner > 0.0 {
            if self.halt == 1.0 {
                self.nextmsg(me);
                // with (obj_smallface) instance_destroy(); — obj_smallface is never created here
            }
            if self.halt == 2.0 {
                if self.facer == 1.0 {
                    g.destroy_all("obj_face");
                }
                g.destroy_self(me);
            }
        }
        self.skipme = 0.0;
        self.siner += 1.0;
        if self.textalpha != 1.0 {
            g.gfx.draw_set_alpha(1.0);
        }
        g.draw_set_halign(rem_h);
        g.draw_set_valign(rem_v);
    }

    obj_vars!(reachedend, skipme, dialoguer, facer, textalpha, writingx, writingy, halt, skippable);
}

/// instance_create(x, y, obj_writer): reads global.msg / global.typer at creation.
pub fn writer_create(g: &mut Game, x: f64, y: f64) -> Id { g.instance_create(x, y, Box::new(Writer::default())) }

// ============================================================================ obj_face

/// obj_face (party member portrait next to the battle box text). Parent obj_face_parent.
#[derive(Default)]
pub struct Face {
    pub mouthmove: f64,
    pub mouthtimer: f64,
    pub face_index: f64,
    pub nowface: f64,
    pub facechange: f64,
    pub rate: f64,
    pub buffer: f64,
    pub f: f64,
    pub battletimer: f64,
}
impl Object for Face {
    fn name(&self) -> &'static str { "obj_face" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.mouthmove = 0.0;
        self.mouthtimer = 0.0;
        self.face_index = 0.0;
        me.image_speed = 0.0;
        self.nowface = g.glob.fc;
        self.facechange = 2.0;
        self.rate = 1.0;
        self.buffer = 4.0;
        self.f = 1.0;
        if g.glob.darkzone == 1.0 {
            self.f = 2.0;
        }
        self.battletimer = 0.0;
    }
    fn step(&mut self, _me: &mut Inst, _g: &mut Game) {
        self.buffer -= 1.0;
        if self.buffer < 0.0 && self.mouthmove == 1.0 && self.mouthtimer == 0.0 {
            self.mouthtimer = 1.0;
            self.face_index = 1.0;
        }
        if self.mouthtimer > 0.0 {
            self.mouthtimer += self.rate;
        }
        if self.mouthtimer >= 1.0 && self.mouthtimer <= 5.0 {
            self.face_index = 1.0;
        } else {
            self.face_index = 0.0;
        }
        if self.mouthtimer >= 9.0 {
            self.mouthtimer = 0.0;
            self.mouthmove = 0.0;
        }
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        self.battletimer += 1.0;
        self.facechange -= 1.0;
        let fc = g.glob.fc;
        if self.nowface != fc {
            self.facechange = 3.0;
        }
        if self.facechange <= 0.0 {
            let f = self.f;
            // Chapter 3: Susie uses spr_face_susie_alt, Ralsei spr_face_r_nohat, frame = global.fe.
            if fc == 1.0 {
                let mut face = spr("spr_face_susie_alt");
                self.face_index = g.glob.fe;
                if g.glob.fe == 61.0 {
                    face = spr("spr_face_placeholder");
                    self.face_index = 0.0;
                }
                g.draw_sprite_ext(face, self.face_index, me.x - 5.0, me.y, f, f, 0.0, C_WHITE, 1.0);
            }
            if fc == 2.0 {
                let mut face = spr("spr_face_r_nohat");
                self.face_index = g.glob.fe;
                if g.glob.fe == 61.0 {
                    face = spr("spr_face_placeholder");
                    self.face_index = 0.0;
                }
                g.draw_sprite_ext(face, self.face_index, me.x - 15.0, me.y - 10.0, f, f, 0.0, C_WHITE, 1.0);
            }
            // other characters' portraits are not reachable in this fight
        }
        self.nowface = fc;
    }
    obj_vars!(mouthmove, face_index);
}

// ============================================================================ obj_battleblcon

/// obj_battleblcon (enemy / party speech balloon). Owns a writer (`mywriter`).
pub struct BattleBlcon {
    pub mywriter: Id,
    pub auto_length: f64,
    pub side: f64,
    pub xoffset: f64,
    pub init: f64,
    pub reformatted: f64,
    pub remx: f64,
    pub remy: f64,
    pub remmsgno: f64,
    pub initwritingx: f64,
    pub initwritingy: f64,
    pub writingx: f64,
    pub writingy: f64,
    pub stringmax: f64,
    pub hspace: f64,
    pub vspace: f64,
    pub linecount: f64,
    pub balloonwidth: f64,
    pub balloonheight: f64,
}
impl Default for BattleBlcon {
    fn default() -> Self {
        BattleBlcon {
            mywriter: NOONE,
            auto_length: 0.0,
            side: 1.0,
            xoffset: 0.0,
            init: 0.0,
            reformatted: 0.0,
            remx: 0.0,
            remy: 0.0,
            remmsgno: -1.0,
            initwritingx: -1.0,
            initwritingy: -1.0,
            writingx: 0.0,
            writingy: 0.0,
            stringmax: 0.0,
            hspace: 0.0,
            vspace: 0.0,
            linecount: 0.0,
            balloonwidth: 0.0,
            balloonheight: 0.0,
        }
    }
}
impl Object for BattleBlcon {
    fn name(&self) -> &'static str { "obj_battleblcon" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.mywriter = writer_create(g, me.x + 5.0, me.y + 3.0);
        if let Some(w) = g.inst_mut(self.mywriter) {
            w.depth = 9999999.0;
        }
        self.auto_length = 0.0;
        self.side = 1.0;
        self.xoffset = 0.0;
        self.init = 0.0;
        self.reformatted = 0.0;
        self.remx = me.x;
        self.remy = me.y;
        self.remmsgno = -1.0;
        self.initwritingx = -1.0;
        self.initwritingy = -1.0;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if !g.id_exists(self.mywriter) {
            g.destroy_self(me);
        }
    }
    fn user(&mut self, _n: usize, _me: &mut Inst, _g: &mut Game) {
        // Other_10: exit
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let wid = self.mywriter;
        if !g.id_exists(wid) {
            return;
        }
        if let Some(w) = g.inst_mut(wid) {
            w.depth = me.depth - 5.0;
        }
        let first_msgno = g.get_first::<Writer>("obj_writer").map(|(_, w)| w.msgno as f64).unwrap_or(-1.0);
        if self.remmsgno != first_msgno {
            self.reformatted = 0.0;
        }
        if self.auto_length == 0.0 {
            self.init = 1.0;
            self.reformatted = 1.0;
        }
        if self.reformatted == 0.0 {
            let formatted = g.get::<Writer>(wid).map(|(_, w)| w.formatted).unwrap_or(1.0);
            if formatted == 0.0 {
                g.with_t::<Writer, _>(wid, |wi, w, g| w.user(5, wi, g));
            }
            let vals = g.get::<Writer>(wid).map(|(_, w)| (w.formatted, w.writingx, w.writingy, w.stringmax, w.hspace, w.vspace, w.linecount));
            if let Some((formatted, wx, wy, smax, hs, vs, lc)) = vals {
                if formatted == 1.0 {
                    if self.init == 0.0 {
                        self.initwritingx = wx;
                        self.initwritingy = wy;
                        self.init = 1.0;
                    }
                    self.writingx = wx;
                    self.writingy = wy;
                    self.stringmax = smax;
                    self.hspace = hs;
                    self.vspace = vs;
                    self.linecount = lc;
                    self.balloonwidth = self.stringmax * self.hspace + 10.0;
                    self.balloonheight = (self.linecount + 1.0) * self.vspace + 5.0;
                    if self.side == 1.0 {
                        self.writingx = self.initwritingx - (self.balloonwidth + 20.0);
                        self.writingy = self.initwritingy - self.balloonheight / 2.0;
                    }
                    if self.side == -1.0 {
                        self.xoffset = 20.0;
                        self.writingx = self.initwritingx;
                        self.writingy = self.initwritingy - self.balloonheight / 2.0;
                    }
                    if self.side == 2.0 {
                        self.writingx = self.initwritingx - self.balloonwidth / 2.0;
                        self.writingy = self.initwritingy - (self.balloonheight + 20.0);
                    }
                    let (nx, ny, side) = (self.writingx, self.writingy, self.side);
                    if side == 1.0 || side == -1.0 || side == 2.0 {
                        if let Some((_, w)) = g.get::<Writer>(wid) {
                            w.writingx = nx;
                            w.writingy = ny;
                        }
                    }
                }
            }
        }
        if self.auto_length == 0.0 {
            g.draw_self(me);
        }
        if self.auto_length == 1.0 && self.init == 1.0 {
            let mut blconscale = 1.0;
            if self.balloonheight < 40.0 {
                blconscale = 0.5;
            }
            let parts = spr("spr_battleblcon_parts");
            if self.side == 1.0 {
                g.draw_sprite_ext(parts, 4.0, me.x, me.y, 1.0, blconscale, 0.0, C_WHITE, 1.0);
            }
            if self.side == -1.0 {
                g.draw_sprite_ext(parts, 4.0, me.x - self.xoffset, me.y, -1.0, blconscale, 0.0, C_WHITE, 1.0);
            }
            if self.side == 2.0 {
                g.draw_sprite_ext(parts, 4.0, me.x, me.y, 1.0, 1.0, -90.0, C_WHITE, 1.0);
            }
            g.gfx.draw_set_color(C_WHITE);
            let (wx, wy, bw, bh) = (self.writingx, self.writingy, self.balloonwidth, self.balloonheight);
            g.gfx.draw_rectangle(wx - 10.0, wy - 5.0, wx + bw, (wy + bh) - 5.0, false);
            g.gfx.draw_rectangle(wx - 5.0, wy - 10.0, (wx + bw) - 5.0, wy + bh, false);
        }
        if let Some((_, w)) = g.get::<Writer>(wid) {
            self.remmsgno = w.msgno as f64;
        }
    }
    obj_vars!(auto_length, side, xoffset);
}

// ============================================================================ scripts

/// msgset(i, str) / msgsetloc
pub fn msgset(g: &mut Game, i: usize, s: &str) {
    g.glob.msgno = i as f64;
    if g.glob.msg.len() <= i {
        g.glob.msg.resize(i + 1, String::new());
    }
    g.glob.msg[i] = s.to_string();
}
/// msgnext(str) / msgnextloc
pub fn msgnext(g: &mut Game, s: &str) {
    let n = g.glob.msgno as usize + 1;
    msgset(g, n, s);
}

/// scr_speaker(name)
pub fn scr_speaker(g: &mut Game, who: &str) {
    let dz = g.glob.darkzone == 1.0;
    let fighting = g.glob.fighting == 1.0;
    g.glob.typer = 5.0;
    if dz {
        g.glob.typer = 6.0;
    }
    if fighting {
        g.glob.typer = 4.0;
    }
    g.glob.fc = 0.0;
    g.glob.fe = 0.0;
    match who {
        "silent" => g.glob.typer = if dz { 36.0 } else { 2.0 },
        "balloon" | "enemy" => g.glob.typer = 50.0,
        "undyne" | "und" => {
            g.glob.typer = 17.0;
            g.glob.fc = 9.0;
        }
        "temmie" | "tem" => g.glob.typer = 21.0,
        "catti" => g.glob.fc = 13.0,
        "catty" | "caddy" => g.glob.fc = 16.0,
        "bratty" | "bra" => g.glob.fc = 17.0,
        "burgerpants" | "bur" => g.glob.fc = 19.0,
        "sneo" => g.glob.typer = 67.0,
        "susie" | "sus" => {
            g.glob.fc = 1.0;
            g.glob.typer = 10.0;
            if dz {
                g.glob.typer = 30.0;
                if fighting {
                    g.glob.typer = 47.0;
                }
            }
        }
        "ralsei" | "ral" => {
            g.glob.fc = 2.0;
            g.glob.typer = 31.0;
            if fighting {
                g.glob.typer = 45.0;
            }
            if g.glob.flag(30) == 1.0 {
                g.glob.typer = 6.0;
            }
        }
        "toriel" | "tor" => {
            g.glob.fc = 4.0;
            g.glob.typer = 7.0;
        }
        "asgore" | "asg" => {
            g.glob.fc = 10.0;
            g.glob.typer = 18.0;
        }
        "rudy" | "rud" => {
            g.glob.fc = 15.0;
            g.glob.typer = 55.0;
        }
        "alphys" | "alp" => {
            g.glob.fc = 11.0;
            g.glob.typer = 20.0;
        }
        _ => {}
    }
}

/// scr_anyface(name, msgno, emotion): writes the face/typer switch message into global.msg[msgno].
/// `emotion` is the face character as a string ("J", "0", ...).
pub fn scr_anyface(g: &mut Game, who: &str, msgno: usize, emotion: &str) {
    let speaker = who.to_lowercase();
    msgset(g, msgno, &format!("* Face {speaker} not found/"));
    let set = |g: &mut Game, s: String| {
        if g.glob.msg.len() <= msgno {
            g.glob.msg.resize(msgno + 1, String::new());
        }
        g.glob.msg[msgno] = s;
    };
    match speaker.as_str() {
        "susie" | "sus" => set(g, format!("\\TX \\F0 \\E{emotion} \\FS \\TS %")),
        "ralsei" | "ral" => set(g, format!("\\TX \\F0 \\E{emotion} \\FR \\TR %")),
        "alphys" | "alp" => msgset(g, msgno, &format!("\\TX \\F0 \\E{emotion} \\Fa \\Ta %")),
        "none" | "x" | "no name" | "no_name" => set(g, "\\TX \\F0 \\T0 %".to_string()),
        "undyne" | "und" => msgset(g, msgno, &format!("\\TX \\F0 \\E{emotion} \\FU \\TU %")),
        "burgerpants" => msgset(g, msgno, &format!("\\TX \\F0 \\E{emotion} \\Fb \\T0 %")),
        "sneo" => msgset(g, msgno, &format!("\\TX \\F0 \\E{emotion} \\Tp %")),
        // toriel / asgore / rudy (scr_torface & co.) are not used in this fight
        _ => {}
    }
}

/// scr_anyface_next(name, face)
pub fn scr_anyface_next(g: &mut Game, who: &str, face: &str) {
    g.glob.msgno += 1.0;
    let n = g.glob.msgno as usize;
    scr_anyface(g, who, n, face);
}

/// scr_battletext(): writer in the battle box from global.msg (+ obj_face). Returns the writer id.
pub fn scr_battletext(g: &mut Game) -> Id {
    let (xx, yy) = (g.camerax(), g.cameray());
    let battlewriter = writer_create(g, xx + 30.0, yy + 376.0);
    // (the Tenna-convo exception is not reachable) → always a face
    g.instance_create(xx + 26.0, yy + 380.0, Box::new(Face::default()));
    let fc = g.glob.fc;
    if let Some((_, w)) = g.get::<Writer>(battlewriter) {
        w.dialoguer = 1.0;
        w.facer = 1.0;
        // NOTE: dead in practice — Draw resets charline to originalcharline before formatting when fc == 0.
        if fc == 0.0 && w.originalcharline == 33.0 {
            w.charline = 26.0;
        }
    }
    battlewriter
}

/// scr_battletext_default()
pub fn scr_battletext_default(g: &mut Game) -> Id {
    g.glob.fc = 0.0;
    g.glob.typer = 4.0;
    scr_battletext(g)
}

/// scr_enemyblcon(x, y, kind): speech balloon (kind 0 = bare writer). Returns the created instance.
/// Kinds whose sprites aren't used in this fight (2, 4-8, 11, 12.x) fall back to the plain balloon.
pub fn scr_enemyblcon(g: &mut Game, x: f64, y: f64, kind: i32) -> Id {
    if kind == 0 {
        return writer_create(g, x, y);
    }
    let id = g.instance_create(x, y, Box::new(BattleBlcon::default()));
    let (sprite, auto_length, side) = match kind {
        3 => (Some(spr("spr_battleblcon_long")), 0.0, 1.0),
        10 => (Some(spr("spr_battleblcon_long")), 1.0, 1.0),
        13 => (Some(spr("spr_battleblcon_long")), 1.0, 2.0),
        14 => (Some(spr("spr_battleblcon_long")), 1.0, -1.0),
        _ => (None, 0.0, 1.0),
    };
    if let Some((i, b)) = g.get::<BattleBlcon>(id) {
        if let Some(s) = sprite {
            i.sprite_index = s;
        }
        b.auto_length = auto_length;
        b.side = side;
    }
    id
}

/// scr_terminate_writer(): true once no writer is left (destroys them on Z when all reached their end).
pub fn scr_terminate_writer(g: &mut Game) -> bool {
    let mut killed = false;
    let mut killable = false;
    let ids = g.ids_of("obj_writer");
    if !ids.is_empty() {
        let ended = ids.iter().filter(|&&id| g.get::<Writer>(id).map(|(_, w)| w.reachedend == 1.0).unwrap_or(false)).count();
        if ended == ids.len() {
            killable = true;
        }
    } else {
        killed = true;
    }
    if g.input.pressed(Key::B1) && killable {
        g.destroy_all("obj_writer");
        killed = true;
    }
    killed
}
