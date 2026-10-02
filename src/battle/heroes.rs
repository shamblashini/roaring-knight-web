//! obj_heroparent (obj_herokris / obj_herosusie / obj_heroralsei), obj_attackpress (FIGHT bars),
//! obj_burstbolt, obj_basicattack, obj_dmgwriter (damage numbers), obj_tensionbar (TP bar),
//! scr_damage_enemy, scr_monsterdefeat, scr_retarget, scr_monsterpop, and the sprite fonts
//! (global.damagefont / global.damagefontgold / global.hpfont).
//! OWNER: battle-core agent "heroes".

use crate::assets::{font, spr, Spr, NO_SPR};
use crate::battle::KnightEnemy;
use crate::gfx::{merge_color, Color, C_AQUA, C_BLACK, C_BLUE, C_DKGRAY, C_FUCHSIA, C_GREEN, C_LIME, C_LTGRAY, C_NAVY, C_ORANGE, C_PURPLE, C_RED, C_WHITE, C_YELLOW};
use crate::gm::{ceil, floor, round};
use crate::input::Key;
use crate::obj_vars;
use crate::rt::{Afterimage, Game, HAlign, Id, Inst, Object, NOONE};
use std::cell::Cell;

// ============================================================================ hooks into other modules

thread_local! {
    static SCR_WINCOMBAT_HOOK: Cell<Option<fn(&mut Game)>> = const { Cell::new(None) };
}

/// Register `scr_wincombat()` (controller). Only reachable if every monster is gone.
pub fn set_scr_wincombat_hook(f: fn(&mut Game)) { SCR_WINCOMBAT_HOOK.with(|h| h.set(Some(f))); }

fn call_scr_wincombat(g: &mut Game) {
    match SCR_WINCOMBAT_HOOK.with(|h| h.get()) {
        Some(f) => f(g),
        None => crate::log("heroes: scr_wincombat hook not registered"),
    }
}

/// global.charauto[char] (no auto-battling party members in this fight; stored in glob.extra).
fn charauto(g: &Game, c: usize) -> f64 {
    match c {
        1 => g.glob.ex("charauto1"),
        2 => g.glob.ex("charauto2"),
        3 => g.glob.ex("charauto3"),
        4 => g.glob.ex("charauto4"),
        _ => 0.0,
    }
}

/// scr_monsterpop()
pub fn scr_monsterpop(g: &Game) -> f64 { g.glob.monster[0] + g.glob.monster[1] + g.glob.monster[2] }

// ============================================================================ sprite fonts

/// A font made with font_add_sprite_ext(sprite, map, prop, sep).
#[derive(Clone, Copy)]
pub struct SpriteFont {
    pub spr: &'static str,
    pub map: &'static str,
    pub prop: bool,
    pub sep: f64,
}
/// global.damagefont = font_add_sprite_ext(spr_numbersfontbig, "0123456789+-%", 20 (proportional), 0)
pub const DAMAGEFONT: SpriteFont = SpriteFont { spr: "spr_numbersfontbig", map: "0123456789+-%", prop: true, sep: 0.0 };
/// global.damagefontgold = font_add_sprite_ext(spr_numbersfontbig_gold, "0123456789+-%", 20, 0)
pub const DAMAGEFONTGOLD: SpriteFont = SpriteFont { spr: "spr_numbersfontbig_gold", map: "0123456789+-%", prop: true, sep: 0.0 };
/// global.hpfont = font_add_sprite_ext(spr_numbersfontsmall, "0123456789-+", 0 (monospace), 2)
pub const HPFONT: SpriteFont = SpriteFont { spr: "spr_numbersfontsmall", map: "0123456789-+", prop: false, sep: 2.0 };

/// (subimage, x offset to draw the frame at, advance) of one character, or None if unmapped.
fn spritefont_glyph(g: &Game, f: &SpriteFont, s: Spr, ch: char) -> Option<(f64, f64, f64)> {
    let sw = g.sprite_get_width(s);
    let Some(i) = f.map.chars().position(|c| c == ch) else {
        // characters not in the map: only a space advances (GameMaker gives sprite fonts a space glyph)
        return if ch == ' ' { Some((-1.0, 0.0, sw + f.sep)) } else { None };
    };
    if !f.prop {
        return Some((i as f64, 0.0, sw + f.sep));
    }
    // proportional: glyph width = the frame's opaque pixel extent (frames are bbox-trimmed in the atlas)
    let fr = g.sprite(s).and_then(|sp| sp.frames.get(i).copied().flatten());
    Some(match fr {
        Some(fr) => (i as f64, -(fr.offx as f64), fr.w as f64 + f.sep),
        None => (i as f64, 0.0, sw + f.sep),
    })
}

/// string_width() for a sprite font.
pub fn spritefont_string_width(g: &Game, f: &SpriteFont, text: &str) -> f64 {
    let s = spr(f.spr);
    text.chars().filter_map(|c| spritefont_glyph(g, f, s, c)).map(|gl| gl.2).sum()
}

/// draw_text_transformed_colour with a sprite font (valign top). Text is tinted by `col`.
pub fn draw_spritefont_ext(
    g: &mut Game, f: &SpriteFont, x: f64, y: f64, text: &str, xs: f64, ys: f64, rot: f64, col: Color, alpha: f64, halign: HAlign,
) {
    let s = spr(f.spr);
    let w = spritefont_string_width(g, f, text);
    let mut cx = match halign {
        HAlign::Left => 0.0,
        HAlign::Center => -(w / 2.0).floor(),
        HAlign::Right => -w,
    };
    let (sn, cs) = (rot.to_radians().sin(), rot.to_radians().cos());
    for ch in text.chars() {
        let Some((idx, off, shift)) = spritefont_glyph(g, f, s, ch) else { continue };
        if idx >= 0.0 {
            let lx = (cx + off) * xs;
            g.draw_sprite_ext(s, idx, x + lx * cs, y - lx * sn, xs, ys, rot, col, alpha);
        }
        cx += shift;
    }
}

/// Text in global.hpfont (the char-box HP numbers) using the current draw colour/alpha.
/// `halign_right` = draw_set_halign(fa_right), otherwise fa_left.
pub fn draw_hpfont_text(g: &mut Game, x: f64, y: f64, text: &str, halign_right: bool) {
    let (c, a) = (g.gfx.draw_color, g.gfx.draw_alpha);
    let h = if halign_right { HAlign::Right } else { HAlign::Left };
    draw_spritefont_ext(g, &HPFONT, x, y, text, 1.0, 1.0, 0.0, c, a, h);
}

/// Text in global.damagefont (or damagefontgold) using the current draw colour/alpha.
pub fn draw_damagefont_text(g: &mut Game, x: f64, y: f64, text: &str, xs: f64, ys: f64, rot: f64, halign_right: bool, gold: bool) {
    let (c, a) = (g.gfx.draw_color, g.gfx.draw_alpha);
    let h = if halign_right { HAlign::Right } else { HAlign::Left };
    let f = if gold { DAMAGEFONTGOLD } else { DAMAGEFONT };
    draw_spritefont_ext(g, &f, x, y, text, xs, ys, rot, c, a, h);
}

// ============================================================================ obj_heroparent

/// obj_heroparent instance. `obj` is "obj_herokris", "obj_herosusie" or "obj_heroralsei".
pub struct Hero {
    pub obj: &'static str,
    pub char: f64,
    pub myself: f64,
    pub state: f64,
    pub hurt: f64,
    pub hurttimer: f64,
    pub attacktimer: f64,
    pub attacked: f64,
    pub index: f64,
    pub itemed: f64,
    pub tu: f64,
    pub myheight: f64,
    pub mywidth: f64,
    pub flash: f64,
    pub fsiner: f64,
    pub becomeflash: f64,
    pub darkify: f64,
    pub normalsprite: Spr,
    pub idlesprite: Spr,
    pub defendsprite: Spr,
    pub hurtsprite: Spr,
    pub attackreadysprite: Spr,
    pub attacksprite: Spr,
    pub itemsprite: Spr,
    pub itemreadysprite: Spr,
    pub actreadysprite: Spr,
    pub actsprite: Spr,
    pub spellreadysprite: Spr,
    pub spellsprite: Spr,
    pub defeatsprite: Spr,
    pub victorysprite: Spr,
    // ---- the rest of obj_heroparent Create
    pub points: f64,
    pub siner: f64,
    pub combatdarken: f64,
    pub darkentimer: f64,
    pub specdraw: f64,
    pub is_auto_susie: f64,
    pub poisonamount: f64,
    pub poisontimer: f64,
    pub maxframes: f64,
    pub force: f64,
    pub victoryanim: f64,
    pub actframes: f64,
    pub victoryframes: f64,
    pub defendframes: f64,
    pub itemframes: f64,
    pub attackframes: f64,
    pub attackspeed: f64,
    pub actreturnframes: f64,
    pub spellframes: f64,
    pub hurtindex: f64,
    pub acttimer: f64,
    pub defendtimer: f64,
    pub finishattacktimer: f64,
    pub spelltimer: f64,
    pub gotupcon: f64,
    pub remspellframes: f64,
    pub remspellsprite: Spr,
    pub thissprite: Spr,
    pub caster: f64,
    // ---- set by scripts run on the hero
    pub cancelattack: f64,
    pub thistarget: f64,
    /// last obj_dmgwriter made by scr_damage_enemy (GML `dm`)
    pub dm: Id,
    /// last obj_basicattack (GML `attack`)
    pub attack: Id,
}
impl Default for Hero {
    fn default() -> Self {
        Hero {
            obj: "obj_herokris",
            char: 1.0,
            myself: 0.0,
            state: 0.0,
            hurt: 0.0,
            hurttimer: 0.0,
            attacktimer: 0.0,
            attacked: 0.0,
            index: 0.0,
            itemed: 0.0,
            tu: 0.0,
            myheight: 37.0,
            mywidth: 34.0,
            flash: 0.0,
            fsiner: 0.0,
            becomeflash: 0.0,
            darkify: 0.0,
            normalsprite: NO_SPR,
            idlesprite: NO_SPR,
            defendsprite: NO_SPR,
            hurtsprite: NO_SPR,
            attackreadysprite: NO_SPR,
            attacksprite: NO_SPR,
            itemsprite: NO_SPR,
            itemreadysprite: NO_SPR,
            actreadysprite: NO_SPR,
            actsprite: NO_SPR,
            spellreadysprite: NO_SPR,
            spellsprite: NO_SPR,
            defeatsprite: NO_SPR,
            victorysprite: NO_SPR,
            points: 0.0,
            siner: 0.0,
            combatdarken: 1.0,
            darkentimer: 0.0,
            specdraw: 0.0,
            is_auto_susie: 0.0,
            poisonamount: 0.0,
            poisontimer: 0.0,
            maxframes: 0.0,
            force: 0.0,
            victoryanim: 0.0,
            actframes: 7.0,
            victoryframes: 9.0,
            defendframes: 1.0,
            itemframes: 3.0,
            attackframes: 3.0,
            attackspeed: 0.5,
            actreturnframes: 10.0,
            spellframes: 10.0,
            hurtindex: 0.0,
            acttimer: 0.0,
            defendtimer: 0.0,
            finishattacktimer: 0.0,
            spelltimer: 0.0,
            gotupcon: 0.0,
            remspellframes: 10.0,
            remspellsprite: NO_SPR,
            thissprite: NO_SPR,
            caster: 0.0,
            cancelattack: 0.0,
            thistarget: 0.0,
            dm: NOONE,
            attack: NOONE,
        }
    }
}

impl Hero {
    pub fn new(obj: &'static str) -> Hero { Hero { obj, ..Default::default() } }

    fn is(&self, o: &str) -> bool { self.obj == o }

    /// scr_retarget(slot): run on the hero (sets its thistarget / cancelattack).
    fn scr_retarget(&mut self, g: &mut Game, slot: usize) {
        self.thistarget = g.glob.chartarget[slot];
        self.cancelattack = 0.0;
        if self.thistarget == 0.0 && g.glob.monster[0] == 0.0 {
            self.thistarget = 1.0;
        }
        if self.thistarget == 1.0 && g.glob.monster[1] == 0.0 {
            self.thistarget = 2.0;
        }
        if self.thistarget == 2.0 {
            if g.glob.monster[2] == 0.0 {
                self.thistarget = 3.0;
            }
            if self.thistarget == 3.0 && g.glob.monster[0] == 1.0 {
                self.thistarget = 0.0;
            }
            if self.thistarget == 3.0 && g.glob.monster[1] == 1.0 {
                self.thistarget = 1.0;
            }
            if self.thistarget == 3.0 {
                self.cancelattack = 1.0;
            }
        }
        g.glob.chartarget[slot] = self.thistarget;
    }

    /// The knight's (blocking, damagereduction) if it exists (GML reads obj_knight_enemy.*).
    fn knight_reduction(g: &mut Game) -> Option<(f64, f64)> {
        g.get_first::<KnightEnemy>("obj_knight_enemy").map(|(_, k)| (k.blocking, k.damagereduction))
    }

    /// Damage of the physical attack (shared by Step_0 and Other_10).
    fn attack_damage(&self, g: &mut Game, slot: usize, kn: Option<(f64, f64)>) -> f64 {
        let t = (g.glob.chartarget[slot] as usize).min(2);
        let mut damage = round(((g.glob.battleat[slot] * self.points) / 20.0) - (g.glob.monsterdf[t] * 3.0));
        if let Some((_, reduction)) = kn {
            damage = ceil(damage * reduction);
            if self.is("obj_herokris") {
                if g.glob.hp[2] < 0.0 && g.glob.hp[3] < 0.0 {
                    damage *= 2.0;
                } else if g.glob.hp[2] < 0.0 || g.glob.hp[3] < 0.0 {
                    damage *= 1.0;
                } else {
                    damage = round(damage * 0.5);
                }
            }
        }
        damage
    }

    /// The obj_basicattack slash on the target (+ Susie's impact).
    fn make_basicattack(&mut self, g: &mut Game, slot: usize) {
        let t = (g.glob.chartarget[slot] as usize).min(2);
        let ax = g.glob.monsterx[t] + g.random(6.0);
        let ay = g.glob.monstery[t] + g.random(6.0);
        let attack = g.instance_create(ax, ay, Box::new(BasicAttack::default()));
        self.attack = attack;
        let weapon = g.glob.charweapon[g.glob.char[slot].min(4)];
        let is_susie = self.is("obj_herosusie");
        let is_ralsei = self.is("obj_heroralsei");
        if let Some((a, b)) = g.get::<BasicAttack>(attack) {
            if is_susie {
                a.sprite_index = spr("spr_attack_mash2");
                a.image_speed = 0.5;
                b.maxindex = 4.0;
            }
            if is_ralsei {
                a.sprite_index = spr("spr_attack_slap1");
                b.maxindex = 4.0;
                a.image_speed = 0.5;
            }
            if weapon == 26.0 {
                a.sprite_index = spr("spr_attack_shard");
                a.image_speed = 0.334;
            }
            if self.points == 150.0 {
                a.image_xscale = 2.5;
                a.image_yscale = 2.5;
            }
        }
        if is_susie {
            g.snd_play("snd_impact");
            g.instance_create(0.0, 0.0, Box::new(crate::battle::soul::Shake::default()));
        }
    }
}

impl Object for Hero {
    fn name(&self) -> &'static str { self.obj }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        let o = self.obj;
        *self = Hero::new(o);
        self.char = 0.0;
        self.myself = 0.0;
        me.image_xscale = 2.0;
        me.image_yscale = 2.0;
        self.normalsprite = spr("spr_krisr_dark");
        self.idlesprite = spr("spr_krisb_idle");
        self.actreadysprite = spr("spr_krisb_actready");
        self.actsprite = spr("spr_krisb_act");
        self.hurtsprite = spr("spr_krisb_hurt");
        self.defendsprite = spr("spr_krisb_attackready");
        self.attackreadysprite = spr("spr_krisb_attackready");
        self.attacksprite = spr("spr_krisb_attack");
        self.itemsprite = spr("spr_krisb_item");
        self.itemreadysprite = spr("spr_krisb_itemready");
        self.spellreadysprite = spr("spr_ralseib_spellready");
        self.spellsprite = spr("spr_ralseib_spell");
        self.defeatsprite = spr("spr_krisb_defeat");
        self.victorysprite = spr("spr_krisb_victory");
        if o == "obj_herokris" {
            self.normalsprite = spr("spr_krisr_dark");
            self.idlesprite = spr("spr_krisb_idle");
            self.defendsprite = spr("spr_krisb_defend");
            self.hurtsprite = spr("spr_krisb_hurt");
            self.attackreadysprite = spr("spr_krisb_attackready");
            self.attacksprite = spr("spr_krisb_attack");
            self.itemsprite = spr("spr_krisb_item");
            self.actreadysprite = spr("spr_krisb_actready");
            self.actsprite = spr("spr_krisb_act");
            self.itemreadysprite = spr("spr_krisb_itemready");
            self.spellreadysprite = spr("spr_krisb_actready");
            self.spellsprite = spr("spr_krisb_act");
            self.defeatsprite = spr("spr_krisb_defeat");
            self.victorysprite = spr("spr_krisb_victory");
            self.actframes = 7.0;
            self.actreturnframes = 10.0;
            self.attackframes = 6.0;
            self.itemframes = 6.0;
            self.defendframes = 5.0;
            self.spellframes = 10.0;
            self.attackspeed = 0.5;
            self.victoryframes = g.sprite_get_number(self.victorysprite);
            self.mywidth = 68.0;
            self.myheight = 74.0;
        }
        if o == "obj_herosusie" {
            self.attackframes = 5.0;
            self.itemframes = 5.0;
            self.defendframes = 5.0;
            self.actframes = 7.0;
            self.actreturnframes = 10.0;
            self.spellframes = 8.0;
            self.attackspeed = 0.5;
            self.normalsprite = spr("spr_susier_dark");
            self.idlesprite = spr("spr_susieb_idle");
            self.defendsprite = spr("spr_susieb_defend");
            self.hurtsprite = spr("spr_susieb_hurt");
            self.actreadysprite = spr("spr_susieb_actready");
            self.actsprite = spr("spr_susieb_act");
            self.attackreadysprite = spr("spr_susieb_attackready");
            self.attacksprite = spr("spr_susieb_attack");
            if g.glob.charweapon[2] == 0.0 {
                self.idlesprite = spr("spr_susieb_idle_unarmed");
                self.attackreadysprite = spr("spr_susieb_attackready_unarmed");
                self.attacksprite = spr("spr_susieb_attack_unarmed");
            }
            self.itemsprite = spr("spr_susieb_item");
            self.itemreadysprite = spr("spr_susieb_itemready");
            self.spellreadysprite = spr("spr_susieb_spellready");
            self.spellsprite = spr("spr_susieb_spell");
            self.defeatsprite = spr("spr_susieb_defeat");
            if g.glob.encounterno == 115.0 {
                self.defeatsprite = spr("spr_susie_dw_fell");
            }
            self.victorysprite = spr("spr_susieb_victory");
            self.victoryframes = g.sprite_get_number(self.victorysprite);
            self.mywidth = 70.0;
            self.myheight = 82.0;
        }
        if o == "obj_heroralsei" {
            self.attackframes = 6.0;
            self.itemframes = 6.0;
            self.defendframes = 7.0;
            self.actframes = 7.0;
            self.actreturnframes = 10.0;
            self.attackspeed = 0.5;
            self.normalsprite = spr("spr_ralsei_walk_right");
            self.idlesprite = spr("spr_ralsei_idle");
            self.defendsprite = spr("spr_ralsei_defend");
            self.hurtsprite = spr("spr_ralsei_hurt_fixed");
            self.attackreadysprite = spr("spr_ralsei_attackready");
            self.attacksprite = spr("spr_ralsei_attack");
            self.itemsprite = spr("spr_ralsei_item");
            self.itemreadysprite = spr("spr_ralsei_itemready");
            self.spellreadysprite = spr("spr_ralsei_spellready");
            self.spellsprite = spr("spr_ralsei_spell");
            self.defeatsprite = spr("spr_ralsei_defeat");
            self.victorysprite = spr("spr_ralsei_victory");
            self.actreadysprite = spr("spr_ralsei_actready");
            self.actsprite = spr("spr_ralsei_act");
            self.victoryframes = g.sprite_get_number(self.victorysprite);
            self.mywidth = 52.0;
            self.myheight = 86.0;
        }
        self.remspellframes = self.spellframes;
        self.remspellsprite = self.spellsprite;
        self.thissprite = self.idlesprite;
        self.caster = 0.0;
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        let my = (self.myself as usize).min(2);
        let c = g.glob.char[my].min(4);
        if g.glob.hp[c] > 0.0 {
            if g.glob.myfight == 3.0 && g.glob.faceaction[my] == 6.0 && self.state != 8.0 {
                self.state = 6.0;
            }
            if self.state == 0.0 && self.hurt == 0.0 {
                self.acttimer = 0.0;
                self.thissprite = self.idlesprite;
                if g.glob.faceaction[my] == 1.0 {
                    self.thissprite = self.attackreadysprite;
                }
                if g.glob.faceaction[my] == 3.0 {
                    self.thissprite = self.itemreadysprite;
                }
                if g.glob.faceaction[my] == 2.0 {
                    self.thissprite = self.spellreadysprite;
                }
                if g.glob.faceaction[my] == 6.0 {
                    self.thissprite = self.actreadysprite;
                }
                if g.glob.charcond[my] == 5.0 {
                    self.thissprite = self.defeatsprite;
                    g.glob.faceaction[my] = 9.0;
                }
                if g.glob.faceaction[my] == 4.0 {
                    self.thissprite = self.defendsprite;
                    self.index = self.defendtimer;
                    if self.defendtimer < self.defendframes {
                        self.defendtimer += 0.5;
                    }
                } else {
                    self.defendtimer = 0.0;
                    self.index = self.siner / 5.0;
                }
                self.siner += 1.0;
            }
            if self.state == 1.0 && self.hurt == 0.0 {
                self.siner += 1.0;
                if self.attacked == 0.0 {
                    g.snd_stop("snd_laz_c");
                    if self.is("obj_herokris") {
                        g.snd_play("snd_laz_c");
                    }
                    if self.is("obj_heroralsei") {
                        g.snd_play_pitch("snd_laz_c", 1.15);
                    }
                    if self.is("obj_herosusie") {
                        g.snd_play_pitch("snd_laz_c", 0.9);
                    }
                    if self.points == 150.0 {
                        g.snd_stop("snd_criticalswing");
                        g.snd_play("snd_criticalswing");
                        for _ in 0..3 {
                            let ax = me.x + self.mywidth + g.random(50.0);
                            let ay = me.y + 30.0 + g.random(30.0);
                            let anim = g.instance_create(ax, ay, Box::new(Afterimage::default()));
                            let hs = 2.0 + g.random(4.0);
                            if let Some(a) = g.inst_mut(anim) {
                                a.sprite_index = spr("spr_lightfairy");
                                a.image_speed = 0.25;
                                a.depth = -20.0;
                                a.image_xscale = 2.0;
                                a.image_yscale = 2.0;
                                a.set_hspeed(hs);
                                a.friction = -0.25;
                            }
                        }
                    }
                    self.attacked = 1.0;
                    self.finishattacktimer = 11.0;
                }
                if self.attacktimer < self.attackframes {
                    me.image_index = self.attacktimer;
                } else {
                    me.image_index = self.attackframes;
                }
                self.thissprite = self.attacksprite;
                self.index = me.image_index;
                self.attacktimer += self.attackspeed;
                if self.force == 1.0 && me.image_index == self.attackframes {
                    self.force = 0.0;
                    self.state = 0.0;
                    self.attacktimer = 0.0;
                    self.attacked = 0.0;
                }
            }
            if self.state == 2.0 && self.hurt == 0.0 {
                self.siner += 1.0;
                if self.itemed == 0.0 {
                    self.itemed = 1.0;
                    self.spelltimer = 16.0;
                }
                if self.attacktimer < self.spellframes && self.spellframes != 0.0 {
                    me.image_index = self.attacktimer;
                } else {
                    me.image_index = self.spellframes;
                }
                if scr_monsterpop(g) == 0.0 {
                    self.attacktimer = 0.0;
                }
                self.thissprite = self.spellsprite;
                self.index = me.image_index;
                self.attacktimer += 0.5;
                if self.force == 1.0 && me.image_index == self.attackframes {
                    self.force = 0.0;
                    self.state = 0.0;
                    self.attacktimer = 0.0;
                    self.itemed = 0.0;
                }
            }
            if self.state == 4.0 && self.hurt == 0.0 {
                self.siner += 1.0;
                if self.itemed == 0.0 {
                    self.itemed = 1.0;
                    self.spelltimer = 16.0;
                }
                if self.attacktimer < self.itemframes {
                    me.image_index = self.attacktimer;
                } else {
                    me.image_index = self.itemframes;
                }
                if scr_monsterpop(g) == 0.0 {
                    self.attacktimer = 0.0;
                }
                self.index = me.image_index;
                self.thissprite = self.itemsprite;
                self.attacktimer += 0.5;
            }
            if self.state == 6.0 {
                if g.glob.myfight == 3.0 {
                    if self.acttimer < self.actframes {
                        self.acttimer += 0.5;
                    }
                } else {
                    self.acttimer += 0.5;
                }
                self.thissprite = self.actsprite;
                self.index = self.acttimer;
                if self.acttimer >= self.actreturnframes {
                    self.acttimer = 0.0;
                    self.state = 0.0;
                    g.glob.faceaction[my] = 0.0;
                }
            }
            if self.state == 7.0 {
                self.hurt = 0.0;
                self.hurttimer = 0.0;
                if self.victoryanim < self.victoryframes {
                    self.thissprite = self.victorysprite;
                    self.index = self.victoryanim;
                    self.victoryanim += 0.334;
                } else {
                    self.thissprite = self.normalsprite;
                    self.index = 0.0;
                }
            }
            if self.state == 8.0 && (me.image_index + me.image_speed) >= self.maxframes && self.maxframes != 0.0 {
                self.state = 0.0;
                self.hurt = 0.0;
                self.attacktimer = 0.0;
                self.maxframes = 0.0;
            }
            if self.hurt == 1.0 && g.glob.hp[c] > 0.0 {
                self.hurtindex = self.hurttimer / 2.0;
                if self.hurtindex > 2.0 {
                    self.hurtindex = 2.0;
                }
                if g.glob.charcond[my] == 5.0 {
                    g.glob.faceaction[my] = 5.0;
                    g.glob.charmove[my] = 1.0;
                    g.glob.charcond[my] = 0.0;
                }
                if g.glob.faceaction[my] == 0.0 {
                    g.glob.faceaction[my] = 5.0;
                }
                if self.hurttimer > 15.0 {
                    self.hurttimer = 0.0;
                    self.hurt = 0.0;
                    if g.glob.faceaction[my] == 5.0 {
                        g.glob.faceaction[my] = 0.0;
                    }
                }
                self.hurttimer += 1.0;
            }
            if self.gotupcon == 1.0 {
                self.gotupcon = 0.0;
            }
        } else {
            g.glob.charcond[my] = 0.0;
            self.hurttimer = 0.0;
            self.hurt = 0.0;
            if self.is("obj_herosusie") && g.glob.encounterno == 115.0 {
                self.defeatsprite = spr("spr_susie_dw_fell");
            }
            self.thissprite = self.defeatsprite;
            self.index = 0.0;
            self.siner += 1.0;
        }
        if g.glob.targeted[my] != 1.0 && self.combatdarken == 1.0 && g.exists("obj_darkener") && self.darkify == 1.0 {
            if self.darkentimer < 15.0 {
                self.darkentimer += 1.0;
            }
            me.image_blend = merge_color(C_WHITE, C_BLACK, self.darkentimer / 30.0);
        }
        if self.darkify == 0.0 {
            if self.darkentimer > 0.0 {
                self.darkentimer -= 3.0;
            }
            me.image_blend = merge_color(C_WHITE, C_BLACK, self.darkentimer / 30.0);
        }
        if self.poisonamount > 0.0 {
            self.poisontimer += 1.0;
            if self.poisontimer >= 10.0 {
                if g.glob.hp[c] > 1.0 {
                    g.glob.hp[c] -= 1.0;
                    self.poisonamount -= 1.0;
                } else {
                    self.poisonamount = 0.0;
                }
                self.poisontimer = 0.0;
            }
        }
        if self.finishattacktimer > 0.0 {
            self.finishattacktimer -= 1.0;
            if self.finishattacktimer == 0.0 {
                g.glob.faceaction[my] = 0.0;
                self.scr_retarget(g, my);
                let kn = Hero::knight_reduction(g);
                let mut knightblock = false;
                if let Some((blocking, reduction)) = kn {
                    if blocking == 1.0 && reduction < 0.1 {
                        knightblock = true;
                    }
                }
                if self.cancelattack == 0.0 {
                    let damage = self.attack_damage(g, my, kn);
                    me.damage = damage;
                    let t = (g.glob.chartarget[my] as usize).min(2);
                    let dm = scr_damage_enemy_dm(g, self.caster, t, damage);
                    self.dm = dm;
                    let ch = self.char;
                    if let Some((_, w)) = g.get::<DmgWriter>(dm) {
                        w.typ = ch - 1.0;
                        if ch == 4.0 {
                            w.typ = 6.0;
                        }
                        w.delay = 8.0;
                    }
                    if damage > 0.0 {
                        scr_tensionheal_round(g, self.points / 10.0);
                        if knightblock {
                            if let Some((_, k)) = g.get_first::<KnightEnemy>("obj_knight_enemy") {
                                k.blockanim = 1.0;
                            }
                        } else {
                            self.make_basicattack(g, my);
                        }
                    }
                }
            }
        }
        if self.spelltimer > 0.0 {
            self.spelltimer -= 1.0;
            if self.spelltimer == 0.0 {
                if self.spellframes > 0.0 {
                    g.glob.faceaction[my] = 0.0;
                }
                if scr_monsterpop(g) > 0.0 {
                    let sp = g.glob.charspecial[my];
                    self.caster = my as f64;
                    crate::battle::spells::scr_spell(g, sp, my, me, self);
                }
                self.state = 0.0;
                self.attacktimer = 0.0;
            }
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let scale = 2.0;
        let my = (self.myself as usize).min(2);
        let c = g.glob.char[my].min(4);
        if self.hurt == 1.0 && self.state != 8.0 && g.glob.hp[c] > 0.0 {
            if g.glob.faceaction[my] != 4.0 {
                self.specdraw = 1.0;
                g.draw_sprite_ext(self.hurtsprite, self.hurtindex, (me.x - 20.0) + (self.hurtindex * 10.0), me.y, scale, scale, 0.0, me.image_blend, me.image_alpha);
            } else {
                self.specdraw = 1.0;
                self.thissprite = self.defendsprite;
                self.index = self.defendtimer;
                g.draw_sprite_ext(self.defendsprite, self.defendtimer, (me.x - 20.0) + (self.hurtindex * 10.0), me.y, scale, scale, 0.0, me.image_blend, me.image_alpha);
            }
        }
        if self.specdraw == 0.0 && self.state != 8.0 {
            me.sprite_index = self.thissprite;
            me.image_index = self.index;
            me.image_blend = C_WHITE;
            g.draw_sprite_ext(self.thissprite, self.index, me.x, me.y, scale, scale, 0.0, me.image_blend, me.image_alpha);
            if self.flash == 1.0 {
                self.fsiner += 1.0;
                g.gfx.d3d_set_fog(true, C_WHITE);
                g.draw_sprite_ext(self.thissprite, self.index, me.x, me.y, scale, scale, 0.0, me.image_blend, (-(self.fsiner / 5.0).cos() * 0.4) + 0.6);
                g.gfx.d3d_set_fog(false, C_BLACK);
            }
        }
        if self.state == 8.0 {
            g.draw_sprite_ext(me.sprite_index, me.image_index, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, me.image_alpha);
        }
        self.specdraw = 0.0;
        if self.becomeflash == 0.0 {
            self.flash = 0.0;
        }
        self.becomeflash = 0.0;
        // (spr_chartarget is only drawn in chapter 1)
    }

    /// Other_10 (event_user(0)): the older attack-resolution path (creates the dmgwriter itself).
    /// Nothing in the knight fight calls it, but it's ported for completeness.
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 0 || self.finishattacktimer <= 0.0 {
            return;
        }
        let my = (self.myself as usize).min(2);
        self.finishattacktimer -= 1.0;
        if self.finishattacktimer != 0.0 {
            return;
        }
        g.glob.faceaction[my] = 0.0;
        self.scr_retarget(g, my);
        let kn = Hero::knight_reduction(g);
        let knightblock = matches!(kn, Some((b, r)) if b == 1.0 && r < 0.1);
        if self.cancelattack != 0.0 {
            return;
        }
        let t = (g.glob.chartarget[my] as usize).min(2);
        let dm = dmgwriter(g, g.glob.monsterx[t], (g.glob.monstery[t] - (g.glob.hittarget[t] * 20.0)) + 20.0);
        self.dm = dm;
        let ch = self.char;
        if let Some((_, w)) = g.get::<DmgWriter>(dm) {
            w.typ = ch - 1.0;
            if ch == 4.0 {
                w.typ = 6.0;
            }
            w.delay = 8.0;
        }
        let mut damage = self.attack_damage(g, my, kn);
        if g.glob.monstertype[t] == 19.0 {
            damage = ceil(damage * 0.3);
        }
        if damage < 0.0 {
            damage = 0.0;
        }
        me.damage = damage;
        if damage == 0.0 {
            if let Some((_, w)) = g.get::<DmgWriter>(dm) {
                w.delay = 2.0;
            }
            monster_try_dodge(g, t);
        }
        if let Some((_, w)) = g.get::<DmgWriter>(dm) {
            w.damage = damage;
        }
        g.glob.hittarget[t] += 1.0;
        g.glob.monsterhp[t] -= damage;
        if g.glob.monsterhp[t] <= 0.0 {
            g.glob.monster[t] = 0.0;
        }
        if damage > 0.0 {
            if g.glob.monstertype[0] != 20.0 {
                scr_tensionheal_round(g, self.points / 10.0);
            } else {
                scr_tensionheal_round(g, self.points / 15.0);
            }
            if knightblock {
                if let Some((_, k)) = g.get_first::<KnightEnemy>("obj_knight_enemy") {
                    k.blockanim = 1.0;
                }
            }
            if !knightblock {
                self.make_basicattack(g, my);
                monster_hurt(g, t, damage);
            }
            set_monster_hurtamt(g, t, damage);
        }
    }

    obj_vars!(
        state, hurt, hurttimer, flash, darkify, char, myself, attacktimer, attacked, index, itemed, tu, fsiner, becomeflash, points,
        siner, combatdarken, darkentimer, specdraw, poisonamount, maxframes, force, victoryanim, hurtindex, acttimer, defendtimer,
        finishattacktimer, spelltimer, caster
    );
}

/// scr_tensionheal(round(x))
fn scr_tensionheal_round(g: &mut Game, x: f64) { crate::battle::scr_tensionheal(g, round(x)); }

/// Create Kris, Susie and Ralsei at global.heromakex/y, filling global.charinstance
/// (obj_battlecontroller Create: myself = slot, char = id, depth = 200 - slot*20).
pub fn create_heroes(g: &mut Game) {
    for i in 0..3 {
        let c = g.glob.char[i];
        let obj = match c {
            1 => "obj_herokris",
            2 => "obj_herosusie",
            3 => "obj_heroralsei",
            _ => continue,
        };
        let (x, y) = (g.glob.heromakex[i], g.glob.heromakey[i]);
        let id = g.instance_create(x, y, Box::new(Hero::new(obj)));
        g.glob.charinstance[i] = id;
        if let Some((inst, h)) = g.get::<Hero>(id) {
            h.myself = i as f64;
            h.char = c as f64;
            inst.depth = 200.0 - (i as f64 * 20.0);
        }
    }
}

// ============================================================================ scr_damage_enemy & co

/// `with (global.monsterinstance[t]) { shakex = 9; state = 3; hurttimer = 30; ... }`
fn monster_hurt(g: &mut Game, t: usize, damage: f64) {
    let mi = g.glob.monsterinstance[t];
    let knight_exists = g.exists("obj_knight_enemy");
    if let Some((_, k)) = g.get::<KnightEnemy>(mi) {
        k.shakex = 9.0;
        k.state = 3.0;
        k.hurttimer = 30.0;
        if knight_exists && damage >= 100.0 {
            k.stronghurtanim = true;
        }
        return;
    }
    if g.id_exists(mi) {
        g.set_var(mi, "shakex", 9.0);
        g.set_var(mi, "state", 3.0);
        g.set_var(mi, "hurttimer", 30.0);
        if knight_exists && damage >= 100.0 {
            if let Some((_, k)) = g.get_first::<KnightEnemy>("obj_knight_enemy") {
                k.stronghurtanim = true;
            }
        }
    }
}

/// `global.monsterinstance[t].hurtamt = v` (only if the monster exposes that variable).
fn set_monster_hurtamt(g: &mut Game, t: usize, v: f64) {
    let mi = g.glob.monsterinstance[t];
    if g.get_var(mi, "hurtamt").is_some() {
        g.set_var(mi, "hurtamt", v);
    }
}

/// `if (hurttimer <= 15 && candodge == 1) { dodgetimer = 0; state = 4; }` on the monster.
/// (The knight's candodge is 0 from scr_enemy_object_init, so this never fires for it.)
fn monster_try_dodge(g: &mut Game, t: usize) {
    let mi = g.glob.monsterinstance[t];
    let candodge = g.get_var(mi, "candodge").unwrap_or(0.0);
    let hurttimer = g.get::<KnightEnemy>(mi).map(|(_, k)| k.hurttimer).or_else(|| g.get_var(mi, "hurttimer")).unwrap_or(0.0);
    if hurttimer <= 15.0 && candodge == 1.0 {
        g.set_var(mi, "dodgetimer", 0.0);
        if let Some((_, k)) = g.get::<KnightEnemy>(mi) {
            k.state = 4.0;
        } else {
            g.set_var(mi, "state", 4.0);
        }
    }
}

/// scr_damage_enemy(target, damage) called by party member `caster` (party slot 0-2; 5 = special).
/// In GML `caster` is the calling instance's variable (obj_heroparent sets it to 0 in Create; scr_spell
/// sets it to the casting slot).
pub fn scr_damage_enemy(g: &mut Game, caster: f64, target: usize, damage: f64) { scr_damage_enemy_dm(g, caster, target, damage); }

/// scr_damage_enemy, returning the obj_dmgwriter it made (GML leaves it in the caller's `dm`).
pub fn scr_damage_enemy_dm(g: &mut Game, caster: f64, target: usize, damage: f64) -> Id {
    let t = target.min(2);
    let dm = dmgwriter(g, g.glob.monsterx[t], (g.glob.monstery[t] + 20.0) - (g.glob.hittarget[t] * 20.0));
    let mut typ = None;
    if caster < 4.0 {
        let c = g.glob.char.get(caster.max(0.0) as usize).copied().unwrap_or(0);
        typ = Some(if c == 4 { 6.0 } else { c as f64 - 1.0 });
    }
    if caster == 5.0 {
        typ = Some(5.0);
    }
    if let Some((_, w)) = g.get::<DmgWriter>(dm) {
        if let Some(t) = typ {
            w.typ = t;
        }
        w.damage = damage;
    }
    g.glob.monsterhp[t] -= damage;
    if damage > 0.0 {
        monster_hurt(g, t, damage);
        set_monster_hurtamt(g, t, damage);
    }
    g.glob.hittarget[t] += 1.0;
    if damage == 0.0 {
        set_monster_hurtamt(g, t, 0.0);
        monster_try_dodge(g, t);
    }
    if g.glob.monsterhp[t] <= 0.0 {
        let mi = g.glob.monsterinstance[t];
        if g.id_exists(mi) {
            scr_monsterdefeat(g, t);
        }
    }
    dm
}

/// scr_monsterdefeat() run on the monster in slot `myself`. The knight fight's ending is driven by
/// obj_knight_enemy at 80% HP, so this is only reached if its HP is somehow taken to 0.
pub fn scr_monsterdefeat(g: &mut Game, myself: usize) {
    let m = myself.min(2);
    if g.glob.monster[m] != 1.0 {
        return;
    }
    let gold3 = g.glob.ex("monstergold3") + g.glob.ex(&format!("monstergold{m}"));
    g.glob.set_ex("monstergold3", gold3);
    let exp3 = g.glob.ex("monsterexp3") + g.glob.ex(&format!("monsterexp{m}"));
    g.glob.set_ex("monsterexp3", exp3);
    g.glob.monster[m] = 0.0;
    let fl = 51 + m as i32;
    if g.glob.flag(fl) == 0.0 {
        g.glob.set_flag(fl, 2.0);
        if g.glob.monsterhp[m] <= 0.0 {
            g.glob.set_flag(fl, 1.0);
        }
    }
    let mi = g.glob.monsterinstance[m];
    let add = |g: &mut Game, f: i32, v: f64| {
        let o = g.glob.flag(f);
        g.glob.set_flag(f, o + v);
    };
    match g.glob.flag(fl) as i32 {
        1 => {
            add(g, 40, 1.0);
            if g.get_var(mi, "fatal").unwrap_or(0.0) == 1.0 {
                add(g, 44, 1.0);
            }
        }
        2 => add(g, 41, 1.0),
        3 => add(g, 42, 1.0),
        5 => add(g, 43, 1.0),
        6 => {
            add(g, 45, 1.0);
            let gold3 = g.glob.ex("monstergold3") + 24.0;
            g.glob.set_ex("monstergold3", gold3);
        }
        _ => {}
    }
    if scr_monsterpop(g) == 0.0 {
        let (mut frozened, mut violenced, mut spared, mut pacified) = (0, 0, 0, 0);
        for d in 0..3 {
            match g.glob.flag(51 + d) as i32 {
                1 => violenced += 1,
                2 => spared += 1,
                3 => pacified += 1,
                6 => frozened += 1,
                _ => {}
            }
        }
        if frozened > 0 {
            g.glob.set_flag(50, 6.0);
        }
        if pacified > 0 {
            g.glob.set_flag(50, 3.0);
        }
        if spared > 0 {
            g.glob.set_flag(50, 2.0);
        }
        if violenced > 0 {
            g.glob.set_flag(50, 1.0);
        }
        if g.glob.flag(50) == 6.0 {
            add(g, 926, 1.0);
        }
        if g.glob.flag(54) != 0.0 {
            let f54 = g.glob.flag(54) as i32;
            let f50 = g.glob.flag(50);
            g.glob.set_flag(f54, f50);
            g.glob.set_flag(54, 0.0);
        }
    }
    // event_user(11): obj_knight_enemy / obj_monsterparent define no Other_21
    g.user_event(mi, 11);
}

// ============================================================================ obj_basicattack

/// obj_basicattack: the slash/impact sprite on the enemy.
pub struct BasicAttack {
    pub maxindex: f64,
    pub critical: f64,
}
impl Default for BasicAttack {
    fn default() -> Self { BasicAttack { maxindex: 3.0, critical: 0.0 } }
}
impl Object for BasicAttack {
    fn name(&self) -> &'static str { "obj_basicattack" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        me.image_xscale = 2.0;
        me.image_yscale = 2.0;
        me.damage = 100.0;
        me.image_speed = 0.334;
        // with (obj_battlecontroller) damagenoise = 1;
        if let Some(bc) = g.first("obj_battlecontroller") {
            if g.get_var(bc, "damagenoise").is_some() {
                g.set_var(bc, "damagenoise", 1.0);
            } else {
                // controller doesn't expose damagenoise: play its sound (snd_damage) directly
                g.snd_play("snd_damage");
            }
        }
        self.maxindex = 3.0;
        self.critical = 0.0;
        if self.critical == 1.0 {
            me.image_xscale += 0.1;
            me.image_yscale += 0.1;
        }
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if me.image_index >= self.maxindex {
            g.destroy_self(me);
        }
    }
    obj_vars!(maxindex, critical);
}

// ============================================================================ obj_burstbolt

/// obj_burstbolt: the flash when a FIGHT bolt is hit.
pub struct BurstBolt {
    pub mag: f64,
}
impl Default for BurstBolt {
    fn default() -> Self { BurstBolt { mag: 0.1 } }
}
impl Object for BurstBolt {
    fn name(&self) -> &'static str { "obj_burstbolt" }
    fn create(&mut self, _me: &mut Inst, _g: &mut Game) { self.mag = 0.1; }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        me.image_alpha -= 0.1;
        me.image_xscale += self.mag;
        me.image_yscale += self.mag;
        let (sw, sh) = (g.sprite_width(me), g.sprite_height(me));
        me.x += ((1.0 - sw) * self.mag) / 2.7;
        me.y += ((1.0 - sh) * self.mag) / 2.5;
        if me.image_alpha < 0.0 {
            g.destroy_self(me);
        }
    }
    obj_vars!(mag);
}

// ============================================================================ obj_dmgwriter

/// obj_dmgwriter
pub struct DmgWriter {
    pub damage: f64,
    /// GML `type`
    pub typ: f64,
    pub delay: f64,
    pub delaytimer: f64,
    pub specialmessage: f64,
    pub killactive: f64,
    pub active: f64,
    pub spec: f64,
    pub bounces: f64,
    pub mercytimer: f64,
    pub stretch: f64,
    pub stretchgo: f64,
    pub lightf: Color,
    pub lightb: Color,
    pub lightg: Color,
    pub lighty: Color,
    pub aqcolor: Color,
    pub init: f64,
    pub kill: f64,
    pub killtimer: f64,
    pub stayincamera: f64,
    pub xx: f64,
    pub message_sprite: Spr,
    pub vstart: f64,
    pub flip: f64,
    pub message: f64,
    pub damagemessage: String,
}
impl Default for DmgWriter {
    fn default() -> Self {
        DmgWriter {
            damage: 0.0,
            typ: -1.0,
            delay: 2.0,
            delaytimer: 0.0,
            specialmessage: 0.0,
            killactive: 0.0,
            active: 0.0,
            spec: 0.0,
            bounces: 0.0,
            mercytimer: 0.0,
            stretch: 0.2,
            stretchgo: 1.0,
            lightf: C_WHITE,
            lightb: C_WHITE,
            lightg: C_WHITE,
            lighty: C_WHITE,
            aqcolor: C_WHITE,
            init: 0.0,
            kill: 0.0,
            killtimer: 0.0,
            stayincamera: 1.0,
            xx: 0.0,
            message_sprite: NO_SPR,
            vstart: 0.0,
            flip: 0.0,
            message: 0.0,
            damagemessage: String::new(),
        }
    }
}
impl Object for DmgWriter {
    fn name(&self) -> &'static str { "obj_dmgwriter" }
    fn create(&mut self, _me: &mut Inst, g: &mut Game) {
        self.spec = 0.0;
        self.delaytimer = 0.0;
        self.delay = 2.0;
        self.active = 0.0;
        self.damage = round(g.random(600.0));
        self.bounces = 0.0;
        self.typ = -1.0;
        self.mercytimer = 0.0;
        self.stretch = 0.2;
        self.stretchgo = 1.0;
        self.lightf = merge_color(C_PURPLE, C_WHITE, 0.6);
        self.lightb = merge_color(C_AQUA, C_WHITE, 0.5);
        self.lightg = merge_color(C_LIME, C_WHITE, 0.5);
        self.lighty = merge_color(C_YELLOW, C_WHITE, 0.3);
        self.aqcolor = merge_color(C_AQUA, C_BLUE, 0.3);
        self.init = 0.0;
        self.kill = 0.0;
        self.killtimer = 0.0;
        self.killactive = 0.0;
        for id in g.ids_of("obj_dmgwriter") {
            if let Some((_, w)) = g.get::<DmgWriter>(id) {
                if w.typ != 3.0 {
                    w.killtimer = 0.0;
                }
            }
        }
        self.specialmessage = 0.0;
        self.stayincamera = 1.0;
        self.xx = g.camerax();
        self.message_sprite = spr("spr_battlemsg");
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.delaytimer < self.delay {
            for id in g.ids_of("obj_dmgwriter") {
                if let Some((_, w)) = g.get::<DmgWriter>(id) {
                    w.killtimer = 0.0;
                }
            }
            self.killtimer = 0.0;
        }
        self.delaytimer += 1.0;
        self.mercytimer += 1.0;
        if self.delaytimer == self.delay {
            let vs = -5.0 - g.random(2.0);
            me.set_vspeed(vs);
            me.set_hspeed(10.0);
            self.vstart = me.vspeed();
            self.flip = 90.0;
        }
        if self.delaytimer >= self.delay {
            let t = self.typ;
            g.gfx.draw_set_color(C_WHITE);
            if t == 0.0 {
                g.gfx.draw_set_color(self.lightb);
            }
            if t == 1.0 {
                g.gfx.draw_set_color(self.lightf);
            }
            if t == 2.0 {
                g.gfx.draw_set_color(self.lightg);
            }
            if t == 3.0 {
                g.gfx.draw_set_color(C_LIME);
            }
            if t == 4.0 {
                g.gfx.draw_set_color(C_RED);
            }
            if t == 5.0 && self.damage < 0.0 {
                g.gfx.draw_set_color(C_LTGRAY);
            }
            if t == 6.0 {
                g.gfx.draw_set_color(self.lighty);
            }
            self.message = self.specialmessage;
            if self.damage == 0.0 {
                self.message = 1.0;
            }
            if t == 4.0 {
                self.message = 2.0;
            }
            if t == 5.0 && self.damage == 100.0 {
                self.message = 5.0;
            }
            if t == 12.0 {
                self.message = 10.0;
            }
            if t == 13.0 {
                self.message = 13.0;
                g.gfx.draw_set_color(self.aqcolor);
            }
            let fnt = if t == 5.0 { DAMAGEFONTGOLD } else { DAMAGEFONT };
            if me.hspeed() > 0.0 {
                me.set_hspeed(me.hspeed() - 1.0);
            }
            if me.hspeed() < 0.0 {
                me.set_hspeed(me.hspeed() + 1.0);
            }
            if me.hspeed().abs() < 1.0 {
                me.set_hspeed(0.0);
            }
            if self.init == 0.0 {
                self.damagemessage = gml_string(self.damage);
                if t == 5.0 {
                    self.damagemessage = format!("+{}%", gml_string(self.damage));
                }
                if t == 5.0 && self.damage < 0.0 {
                    self.damagemessage = format!("{}%", gml_string(self.damage));
                }
                self.init = 1.0;
            }
            let (xs, ys) = (2.0 - self.stretch, self.stretch + self.kill);
            let a = 1.0 - self.kill;
            if self.message == 0.0 {
                g.gfx.draw_set_alpha(a);
                let col = g.gfx.draw_color;
                let rot = if self.spec == 1.0 { 90.0 } else { 0.0 };
                if self.spec == 0.0 || self.spec == 1.0 {
                    let msg = self.damagemessage.clone();
                    draw_spritefont_ext(g, &fnt, me.x + 30.0, me.y, &msg, xs, ys, rot, col, a, HAlign::Right);
                }
                g.draw_set_halign(HAlign::Left);
                g.gfx.draw_set_alpha(1.0);
            } else {
                let ms = self.message_sprite;
                let (x, y) = (me.x + 30.0, me.y);
                let dc = g.gfx.draw_color;
                let (sub, col) = match self.message as i32 {
                    1 => (0.0, dc),
                    2 => (1.0, C_RED),
                    3 => (2.0, C_LIME),
                    4 => (3.0, C_LIME),
                    5 => (5.0, C_LIME),
                    6 => (8.0, C_WHITE),
                    7 => (9.0, C_WHITE),
                    8 => (10.0, C_WHITE),
                    9 => (11.0, C_WHITE),
                    10 => (13.0, C_RED),
                    13 => (14.0, self.aqcolor),
                    _ => (-1.0, C_WHITE),
                };
                if sub >= 0.0 {
                    g.draw_sprite_ext(ms, sub, x, y, xs, ys, 0.0, col, a);
                }
            }
            if self.bounces < 2.0 {
                me.set_vspeed(me.vspeed() + 1.0);
            }
            if me.y > me.ystart && self.bounces < 2.0 && self.killactive == 0.0 {
                me.y = me.ystart;
                me.set_vspeed(self.vstart / 2.0);
                self.bounces += 1.0;
            }
            if self.bounces >= 2.0 && self.killactive == 0.0 {
                me.set_vspeed(0.0);
                me.y = me.ystart;
            }
            if self.stretchgo == 1.0 {
                self.stretch += 0.4;
            }
            if self.stretch >= 1.2 {
                self.stretch = 1.0;
                self.stretchgo = 0.0;
            }
            self.killtimer += 1.0;
            if self.killtimer > 35.0 {
                self.killactive = 1.0;
            }
            if self.killactive == 1.0 {
                self.kill += 0.08;
                me.y -= 4.0;
            }
            if self.kill > 1.0 {
                g.destroy_self(me);
            }
        }
        if g.glob.fighting == 1.0 && self.stayincamera == 1.0 && me.x >= (self.xx + 600.0) {
            me.x = self.xx + 600.0;
        }
    }
    obj_vars!(damage, typ, delay, delaytimer, specialmessage, killactive, active, spec, kill, killtimer, stretch, bounces);
}

/// GML string(real): integers print without decimals, others with up to 2 decimals.
fn gml_string(v: f64) -> String {
    if v == v.trunc() {
        format!("{}", v as i64)
    } else {
        let s = format!("{:.2}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// instance_create(x, y, obj_dmgwriter) (Create event runs; set damage/typ afterwards via g.get::<DmgWriter>).
pub fn dmgwriter(g: &mut Game, x: f64, y: f64) -> Id { g.instance_create(x, y, Box::new(DmgWriter::default())) }

// ============================================================================ obj_tensionbar

/// obj_tensionbar
pub struct TensionBar {
    pub tsiner: f64,
    pub apparent: f64,
    pub current: f64,
    pub change: f64,
    pub changetimer: f64,
    pub red: f64,
    pub redtimer: f64,
    pub xx: f64,
    pub yy: f64,
    pub flashsiner: f64,
    pub maxed: f64,
    pub healthbar_surf: i32,
    pub yoffset: f64,
    pub tamt: f64,
}
impl Default for TensionBar {
    fn default() -> Self {
        TensionBar {
            tsiner: 0.0,
            apparent: 0.0,
            current: 0.0,
            change: 0.0,
            changetimer: 15.0,
            red: 0.0,
            redtimer: 0.0,
            xx: 0.0,
            yy: 0.0,
            flashsiner: 0.0,
            maxed: 0.0,
            healthbar_surf: -1,
            yoffset: 0.0,
            tamt: 0.0,
        }
    }
}
impl Object for TensionBar {
    fn name(&self) -> &'static str { "obj_tensionbar" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.tsiner = 0.0;
        g.glob.tensionselect = 0.0;
        self.apparent = g.glob.tension;
        self.current = g.glob.tension;
        self.change = 0.0;
        self.changetimer = 15.0;
        self.red = 0.0;
        self.redtimer = 0.0;
        self.xx = g.camerax();
        self.yy = g.cameray();
        me.y = self.yy + 40.0;
        me.x = self.xx - 40.0;
        me.set_hspeed(13.0);
        me.friction = 1.0;
        self.flashsiner = 0.0;
        self.maxed = 0.0;
        self.healthbar_surf = g.gfx.surface_create(96, 250);
        self.yoffset = 0.0;
    }
    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 5 {
            g.destroy_self(me);
        }
    }
    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.gfx.surface_exists(self.healthbar_surf) {
            g.gfx.surface_free(self.healthbar_surf);
        }
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let ended = g.get_first::<KnightEnemy>("obj_knight_enemy").map(|(_, k)| k.end_cutscene_version > 0.0).unwrap_or(false);
        if ended {
            return;
        }
        if !g.gfx.surface_exists(self.healthbar_surf) {
            self.healthbar_surf = g.gfx.surface_create(96, 250);
        }
        let surf = self.healthbar_surf;
        g.gfx.surface_set_target(surf);
        g.gfx.draw_clear_alpha(C_WHITE, 0.0);
        // (obj_battlecontroller.rouxlsgridenabled is never set in this fight)
        self.yoffset = crate::gm::lerp(self.yoffset, 0.0, 0.25);
        self.yy = g.cameray();
        me.y = self.yy + 40.0 + self.yoffset;
        let tb = spr("spr_tensionbar");
        let sw = g.sprite_width(me);
        let sh = g.sprite_height(me);
        g.draw_sprite(tb, 1.0, 0.0, 0.0);
        let tension = g.glob.tension;
        let maxt = g.glob.maxtension;
        if (self.apparent - tension).abs() < 20.0 {
            self.apparent = tension;
        }
        if self.apparent < tension {
            self.apparent += 20.0;
        }
        if self.apparent > tension {
            self.apparent -= 20.0;
        }
        if self.apparent != self.current {
            self.changetimer += 1.0;
            if self.changetimer > 15.0 {
                let d = |s: &Self| s.apparent - s.current;
                if d(self) > 0.0 {
                    self.current += 2.0;
                }
                if d(self) > 10.0 {
                    self.current += 2.0;
                }
                if d(self) > 25.0 {
                    self.current += 3.0;
                }
                if d(self) > 50.0 {
                    self.current += 4.0;
                }
                if d(self) > 100.0 {
                    self.current += 5.0;
                }
                if d(self) < 0.0 {
                    self.current -= 2.0;
                }
                if d(self) < -10.0 {
                    self.current -= 2.0;
                }
                if d(self) < -25.0 {
                    self.current -= 3.0;
                }
                if d(self) < -50.0 {
                    self.current -= 4.0;
                }
                if d(self) < -100.0 {
                    self.current -= 5.0;
                }
                if d(self).abs() < 3.0 {
                    self.current = self.apparent;
                }
            }
        }
        let maxcol = merge_color(C_YELLOW, C_ORANGE, 0.5);
        let bar = |g: &mut Game, v: f64| {
            g.gfx.draw_rectangle(3.0, (0.0 + sh) - 1.0, (0.0 + sw) - 1.0, (0.0 + sh) - ((v / maxt) * sh), false);
        };
        if self.current > 0.0 {
            if self.apparent < self.current {
                g.gfx.draw_set_color(C_RED);
                bar(g, self.current);
                g.gfx.draw_set_color(C_ORANGE);
                bar(g, self.apparent);
            }
            if self.apparent > self.current {
                g.gfx.draw_set_color(C_WHITE);
                bar(g, self.apparent);
                g.gfx.draw_set_color(C_ORANGE);
                if self.maxed == 1.0 {
                    g.gfx.draw_set_color(maxcol);
                }
                bar(g, self.current);
            }
            if self.apparent == self.current {
                g.gfx.draw_set_color(C_ORANGE);
                if self.maxed == 1.0 {
                    g.gfx.draw_set_color(maxcol);
                }
                bar(g, self.current);
            }
        }
        if g.glob.tensionselect > 0.0 {
            self.tsiner += 1.0;
            g.gfx.draw_set_color(C_WHITE);
            g.gfx.draw_set_alpha(((self.tsiner / 8.0).sin() * 0.5).abs() + 0.2);
            let theight = (0.0 + sh) - ((self.current / maxt) * sh);
            let mut theight2 = theight + ((g.glob.tensionselect / maxt) * sh);
            if theight2 > ((0.0 + sh) - 1.0) {
                theight2 = (0.0 + sh) - 1.0;
                g.gfx.draw_set_color(C_DKGRAY);
                g.gfx.draw_set_alpha(0.7);
            }
            g.gfx.draw_rectangle(3.0, theight2, (0.0 + sw) - 1.0, theight, false);
            g.gfx.draw_set_alpha(1.0);
        } else {
            self.tsiner += 1.0;
        }
        if self.apparent > 20.0 && self.apparent < maxt {
            g.draw_sprite(spr("spr_tensionmarker"), 0.0, 3.0, (0.0 + sh) - ((self.current / maxt) * sh));
        }
        g.draw_sprite(tb, 0.0, 0.0, 0.0);
        g.gfx.gpu_set_blendmode(crate::gfx::bm::SUBTRACT);
        g.draw_sprite_ext(spr("spr_tensionbar_cutout"), 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, C_WHITE, 1.0);
        g.gfx.gpu_set_blendmode(crate::gfx::bm::NORMAL);
        g.gfx.surface_reset_target();
        g.gfx.draw_surface(surf, me.x, me.y);
        g.draw_sprite(spr("spr_tplogo"), 0.0, me.x - 30.0, me.y + 30.0);
        g.gfx.draw_set_color(C_WHITE);
        g.draw_set_font(font("fnt_mainbig"));
        self.flashsiner += 1.0;
        self.tamt = floor((self.apparent / maxt) * 100.0);
        self.maxed = 0.0;
        if self.tamt < 100.0 {
            let s = gml_string(floor((self.apparent / maxt) * 100.0));
            g.draw_text(me.x - 30.0, me.y + 70.0, &s);
            g.draw_text(me.x - 25.0, me.y + 95.0, "%");
        }
        if self.tamt >= 100.0 {
            self.maxed = 1.0;
            g.gfx.draw_set_color(C_YELLOW);
            g.draw_text(me.x - 28.0, me.y + 70.0, "M");
            g.draw_text(me.x - 24.0, me.y + 90.0, "A");
            g.draw_text(me.x - 20.0, me.y + 110.0, "X");
        }
    }
    obj_vars!(tsiner, apparent, current, changetimer, yoffset, maxed);
}
pub fn create_tensionbar(g: &mut Game) -> Id { g.instance_create(0.0, 0.0, Box::new(TensionBar::default())) }

// ============================================================================ obj_attackpress

/// obj_attackpress (FIGHT timing bars). All its logic is in its Draw event, as in GML.
#[derive(Default)]
pub struct AttackPress {
    pub active: f64,
    pub fastmode: f64,
    pub goahead: f64,
    pub linespeed: f64,
    pub linex: f64,
    pub spelluse: f64,
    pub spelldelay: [f64; 3],
    pub maxdelay: f64,
    pub maxdelaytimer: f64,
    pub havechar: [f64; 3],
    pub charitem: [f64; 3],
    pub charspell: [f64; 3],
    pub fade: f64,
    pub fadeamt: f64,
    pub fakefade: f64,
    pub bcolor: Color,
    pub charcolor: [Color; 3],
    pub target: f64,
    pub boltcolor: [Color; 3],
    pub imagetimer: f64,
    pub posttimer: f64,
    pub timermax: f64,
    pub boltorder: [f64; 3],
    pub boltgap: f64,
    pub boltspeed: f64,
    pub boltx: f64,
    pub points: [f64; 3],
    pub pressbuffer: [f64; 4],
    pub charbolt: [f64; 3],
    pub attacked: [f64; 3],
    pub bolttotal: f64,
    pub boltxoff: f64,
    pub my_method: f64,
    pub boltnum: f64,
    pub boltuse: [f64; 3],
    pub lastbolt: f64,
    pub boltchar: Vec<f64>,
    pub boltalive: Vec<f64>,
    pub boltred: Vec<f64>,
    pub boltframe: Vec<f64>,
    pub boltcount: [f64; 3],
    pub diff: f64,
    pub haveauto: f64,
    pub autoed: f64,
    pub sus: usize,
    pub techwon: f64,
}

fn vset(v: &mut Vec<f64>, i: usize, x: f64) {
    if v.len() <= i {
        v.resize(i + 1, 0.0);
    }
    v[i] = x;
}

impl AttackPress {
    fn bolt_x(&self, me: &Inst, i: usize) -> f64 { (me.x + 80.0 + (self.boltframe[i] * self.boltspeed)) - (self.boltx * self.boltspeed) }

    /// Score a hit on bolt `i` (shared body of scr_boltcheck / scr_boltcheck_onebutton).
    fn hit_bolt(&mut self, me: &Inst, g: &mut Game, i: usize, bc: usize, topclose: f64) {
        let p = topclose.abs();
        let bx = self.bolt_x(me, i);
        let burst = g.instance_create(bx, me.y + (38.0 * bc as f64), Box::new(BurstBolt::default()));
        let mut blend = None;
        let mut mag = None;
        if p == 0.0 {
            self.points[bc] += 150.0;
            blend = Some(C_YELLOW);
            mag = Some(0.2);
        }
        if p == 1.0 {
            self.points[bc] += 120.0;
        }
        if p == 2.0 {
            self.points[bc] += 110.0;
        }
        if p >= 3.0 {
            self.points[bc] += 100.0 - (topclose.abs() * 2.0);
            blend = Some(self.boltcolor[bc]);
        }
        if p >= 15.0 {
            blend = Some(self.charcolor[bc]);
        }
        if let Some((bi, b)) = g.get::<BurstBolt>(burst) {
            if let Some(c) = blend {
                bi.image_blend = c;
            }
            if let Some(m) = mag {
                b.mag = m;
            }
        }
        self.boltalive[i] = 0.0;
    }

    /// scr_boltcheck_onebutton()
    fn scr_boltcheck_onebutton(&mut self, me: &Inst, g: &mut Game) {
        let mut dualbolt = -1.0;
        let mut dualboltid: i64 = -1;
        self.pressbuffer = [5.0; 4];
        let mut qualifybolt: i64 = -1;
        let mut topclose = 999.0;
        for i in 0..self.bolttotal as usize {
            if self.boltalive[i] == 1.0 {
                let close = self.boltframe[i] - self.boltx;
                if close < 15.0 && close > -5.0 {
                    if close == topclose {
                        dualbolt = 1.0;
                        dualboltid = i as i64;
                    }
                    if close < topclose {
                        topclose = close;
                        qualifybolt = i as i64;
                    }
                }
            }
        }
        if qualifybolt != -1 {
            let q = qualifybolt as usize;
            let bc = self.boltchar[q] as usize;
            self.hit_bolt(me, g, q, bc, topclose);
            if dualbolt == 1.0 && dualboltid >= 0 {
                let d = dualboltid as usize;
                let bc = self.boltchar[d] as usize;
                self.hit_bolt(me, g, d, bc, topclose);
            }
        }
    }

    /// scr_boltcheck(slot) (the 3-button control scheme, global.flag[13] == 1)
    fn scr_boltcheck(&mut self, me: &Inst, g: &mut Game, slot: usize) {
        let pb = g.glob.char[slot].min(3);
        self.pressbuffer[pb] = 5.0;
        let mut qualifybolt: i64 = -1;
        let mut topclose = 99.0;
        for i in 0..self.bolttotal as usize {
            if self.boltchar[i] == slot as f64 && self.boltalive[i] == 1.0 {
                let close = self.boltframe[i] - self.boltx;
                if close < 15.0 && close > -5.0 && close < topclose {
                    topclose = close;
                    qualifybolt = i as i64;
                }
            }
        }
        if qualifybolt != -1 {
            self.hit_bolt(me, g, qualifybolt as usize, slot, topclose);
        }
    }

    fn set_hero<F: FnOnce(&mut Inst, &mut Hero)>(g: &mut Game, slot: usize, f: F) {
        let id = g.glob.charinstance[slot];
        if let Some((i, h)) = g.get::<Hero>(id) {
            f(i, h);
        }
    }
}

impl Object for AttackPress {
    fn name(&self) -> &'static str { "obj_attackpress" }

    fn create(&mut self, _me: &mut Inst, g: &mut Game) {
        self.active = 0.0;
        self.fastmode = 1.0;
        if self.fastmode == 1.0 {
            self.active = 1.0;
        }
        self.goahead = 0.0;
        self.linespeed = 2.0;
        self.linex = -10.0;
        self.spelluse = 0.0;
        self.spelldelay = [10.0; 3];
        self.maxdelay = 0.0;
        self.maxdelaytimer = 0.0;
        for xyz in 0..3 {
            self.havechar[xyz] = 0.0;
            self.charitem[xyz] = 0.0;
            self.charspell[xyz] = 0.0;
            let ca = g.glob.charaction[xyz];
            if ca == 1.0 {
                self.havechar[xyz] = 1.0;
            }
            if ca == 4.0 || ca == 2.0 {
                if self.maxdelay == 0.0 {
                    self.maxdelay = 25.0;
                }
                self.maxdelay += 15.0;
                if xyz == 2 && self.spelluse == 1.0 {
                    if self.spelldelay[1] == 25.0 {
                        self.spelldelay[2] = 45.0;
                    } else {
                        self.spelldelay[2] = 25.0;
                    }
                }
                if xyz == 1 && self.spelluse == 1.0 {
                    self.spelldelay[1] = 25.0;
                }
                self.spelluse = 1.0;
                if ca == 4.0 {
                    self.charitem[xyz] = 1.0;
                } else {
                    self.charspell[xyz] = 1.0;
                }
            }
        }
        self.spelluse = 0.0;
        self.fade = 0.0;
        self.fadeamt = 0.0;
        self.fakefade = 0.0;
        self.bcolor = C_NAVY;
        self.charcolor = [16776960, 16711935, 65280];
        self.target = 0.0;
        g.glob.hittarget[0] = g.glob.chartarget[0];
        g.glob.hittarget[1] = g.glob.chartarget[1];
        g.glob.hittarget[2] = g.glob.chartarget[2];
        self.boltcolor = [merge_color(C_AQUA, C_WHITE, 0.5), merge_color(C_FUCHSIA, C_WHITE, 0.5), merge_color(C_LIME, C_WHITE, 0.5)];
        self.imagetimer = 0.0;
        self.posttimer = 0.0;
        self.timermax = 50.0;
        let hc = self.havechar;
        if hc[0] == 0.0 && hc[1] == 0.0 && hc[2] == 0.0 {
            self.timermax = 3.0;
            if self.spelluse == 1.0 && self.fastmode == 1.0 {
                self.timermax += self.maxdelay;
            }
        }
        // bolt order (only used by my_method 2, but it consumes the RNG like the original)
        self.boltorder[0] = g.choose(&[0.0, 1.0, 2.0]);
        if hc[1] == 0.0 && hc[2] == 0.0 {
            self.boltorder[0] = 0.0;
        }
        if self.boltorder[0] == 2.0 {
            self.boltorder[1] = g.choose(&[0.0, 1.0]);
        }
        if self.boltorder[0] == 1.0 {
            self.boltorder[1] = g.choose(&[0.0, 2.0]);
        }
        if self.boltorder[0] == 0.0 {
            self.boltorder[1] = g.choose(&[1.0, 2.0]);
        }
        let (b0, b1) = (self.boltorder[0], self.boltorder[1]);
        if b1 == 2.0 && b0 == 0.0 {
            self.boltorder[2] = 1.0;
        }
        if b1 == 0.0 && b0 == 2.0 {
            self.boltorder[2] = 1.0;
        }
        if b1 == 1.0 && b0 == 0.0 {
            self.boltorder[2] = 2.0;
        }
        if b1 == 0.0 && b0 == 1.0 {
            self.boltorder[2] = 2.0;
        }
        if b1 == 2.0 && b0 == 1.0 {
            self.boltorder[2] = 0.0;
        }
        if b1 == 1.0 && b0 == 2.0 {
            self.boltorder[2] = 0.0;
        }
        if hc[1] == 1.0 && hc[2] == 0.0 {
            self.boltorder[0] = g.choose(&[0.0, 1.0]);
            self.boltorder[2] = if self.boltorder[0] == 1.0 { 0.0 } else { 1.0 };
        }
        self.boltgap = 20.0;
        self.boltspeed = 8.0;
        self.boltx = 0.0;
        self.points = [0.0; 3];
        self.pressbuffer = [0.0; 4];
        self.charbolt = [1.0; 3];
        for i in 0..3 {
            if hc[i] == 0.0 {
                self.charbolt[i] = 0.0;
            }
        }
        self.attacked = [0.0; 3];
        self.bolttotal = self.charbolt[0] + self.charbolt[1] + self.charbolt[2];
        self.boltxoff = 0.0;
        self.my_method = 1.0;
        self.boltnum = 1.0;
        self.boltuse = [0.0; 3];
        self.lastbolt = -1.0;
        self.boltchar = vec![-1.0];
        self.diff = 10.0;
        if g.glob.flag(13) == 0.0 {
            self.diff += 2.0;
        }
        let n = self.bolttotal as usize;
        self.boltalive = vec![0.0; n];
        self.boltred = vec![0.0; n];
        self.boltframe = vec![0.0; n];
        // my_method == 1
        let pick = |g: &mut Game, hc: &[f64; 3]| -> usize {
            let mut c = g.choose(&[0usize, 1, 2]);
            while hc[c] == 0.0 {
                c = g.choose(&[0usize, 1, 2]);
            }
            c
        };
        for i in 0..n {
            self.boltalive[i] = 1.0;
            let mut c = pick(g, &hc);
            while self.boltuse[c] >= self.charbolt[c] {
                c = pick(g, &hc);
            }
            vset(&mut self.boltchar, i, c as f64);
            self.boltuse[c] += 1.0;
        }
        let diff = self.diff;
        for i in 0..n {
            self.boltred[i] = 0.0;
            self.boltxoff += self.lastbolt;
            self.boltframe[i] = 30.0 + self.boltxoff;
            if i + 1 < n {
                if self.lastbolt != 0.0 && self.boltchar[i] != self.boltchar[i + 1] {
                    self.lastbolt = g.choose(&[0.0, diff, diff * 1.5]);
                    self.boltred[i] = 1.0;
                } else {
                    self.lastbolt = g.choose(&[diff, diff * 1.5]);
                }
            } else {
                self.lastbolt = g.choose(&[diff, diff * 1.5]);
            }
        }
        self.haveauto = 0.0;
        self.autoed = 0.0;
        if charauto(g, 2) == 1.0 && (g.glob.char[0] == 2 || g.glob.char[1] == 2 || g.glob.char[2] == 2) {
            self.sus = 0;
            if g.glob.char[1] == 2 {
                self.sus = 1;
            }
            if g.glob.char[2] == 2 {
                self.sus = 2;
            }
            if g.glob.hp[2] >= 0.0 && g.glob.charmove[self.sus] == 1.0 {
                self.haveauto = 1.0;
                if self.timermax == 3.0 {
                    self.timermax = 50.0;
                }
            }
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let ended = g.get_first::<KnightEnemy>("obj_knight_enemy").map(|(_, k)| k.end_cutscene_version > 0.0).unwrap_or(false);
        if ended {
            return;
        }
        self.maxdelaytimer += 1.0;
        if self.spelluse == 1.0 && scr_monsterpop(g) > 0.0 {
            for xyz in 0..3 {
                if self.maxdelaytimer == self.spelldelay[xyz] {
                    if self.charitem[xyz] == 1.0 {
                        AttackPress::set_hero(g, xyz, |_, h| h.state = 4.0);
                    }
                    if self.charspell[xyz] == 1.0 {
                        AttackPress::set_hero(g, xyz, |_, h| h.state = 2.0);
                    }
                }
            }
        }
        if self.maxdelaytimer >= self.maxdelay {
            self.active = 1.0;
        }
        if self.active != 1.0 {
            return;
        }
        let (x, y) = (me.x, me.y);
        for i in 0..3 {
            let fi = i as f64;
            if self.havechar[0] == 1.0 || self.havechar[1] == 1.0 || self.havechar[2] == 1.0 {
                g.gfx.draw_set_color(self.bcolor);
                if i == 1 || i == 2 {
                    g.gfx.draw_rectangle(x + 77.0, y + (38.0 * fi), x + 300.0, y + (38.0 * fi) + 1.0, false);
                }
            }
            let j = g.glob.char[i];
            if j != 0 && charauto(g, j) == 0.0 && self.havechar[i] == 1.0 {
                g.gfx.draw_set_color(self.bcolor);
                let pb = &self.pressbuffer;
                if j == 1 {
                    g.gfx.draw_set_color(C_BLUE);
                    if pb[1] > 0.0 {
                        g.gfx.draw_set_color(merge_color(C_BLUE, C_WHITE, pb[1] / 5.0));
                    }
                }
                if j == 2 {
                    g.gfx.draw_set_color(C_PURPLE);
                    if pb[2] > 0.0 {
                        g.gfx.draw_set_color(merge_color(C_PURPLE, C_WHITE, pb[2] / 5.0));
                    }
                }
                if j == 3 {
                    g.gfx.draw_set_color(C_GREEN);
                    if pb[3] > 0.0 {
                        g.gfx.draw_set_color(merge_color(C_GREEN, C_WHITE, pb[3] / 5.0));
                    }
                }
                if j == 4 {
                    g.gfx.draw_set_color(C_YELLOW);
                    if pb[2] > 0.0 {
                        g.gfx.draw_set_color(merge_color(C_YELLOW, C_WHITE, pb[2] / 5.0));
                    }
                }
                let bs = self.boltspeed;
                g.gfx.draw_rectangle(x + 78.0, y + (38.0 * fi), x + 80.0 + (15.0 * bs), y + (38.0 * fi) + 36.0, true);
                g.gfx.draw_rectangle(x + 79.0, y + (38.0 * fi) + 2.0, (x + 80.0 + (15.0 * bs)) - 1.0, y + (38.0 * fi) + 35.0, true);
                g.draw_sprite(spr("spr_pressfront"), j as f64 - 1.0, x, y + (38.0 * fi));
                if g.glob.flag(13) == 0.0 {
                    g.draw_sprite(spr("spr_pressfront_b"), 0.0, x, y + (38.0 * fi));
                }
                if g.glob.flag(13) == 1.0 {
                    g.draw_sprite(spr("spr_pressfront_b"), fi, x, y + (38.0 * fi));
                }
                g.draw_sprite(spr("spr_pressspot"), j as f64 - 1.0, x + 80.0, y + (38.0 * fi));
            }
        }
        self.boltcount = [0.0; 3];
        if self.my_method == 1.0 {
            let spot = spr("spr_attackspot");
            for i in 0..self.bolttotal as usize {
                let offset = self.boltchar[i];
                let rel = self.boltframe[i] - self.boltx;
                if rel < -5.0 {
                    self.boltalive[i] = 0.0;
                }
                let mut boltalpha = 1.0;
                let bx = self.bolt_x(me, i);
                if rel < 0.0 {
                    boltalpha = 1.0 + (rel / 3.0);
                } else if self.imagetimer == 0.0 && self.boltalive[i] == 1.0 {
                    let img = g.instance_create(bx, y + (38.0 * offset), Box::new(Afterimage::default()));
                    if let Some(a) = g.inst_mut(img) {
                        a.sprite_index = spot;
                        a.image_alpha = 0.4;
                    }
                }
                if self.boltalive[i] == 1.0 {
                    g.draw_sprite_ext(spot, 0.0, bx, y + (38.0 * offset), 1.0, 1.0, 0.0, C_WHITE, boltalpha);
                    let bc = (self.boltchar[i] as usize).min(2);
                    self.boltcount[bc] += 1.0;
                }
            }
            for i in 0..3 {
                if self.boltcount[i] == 0.0 && self.havechar[i] == 1.0 && self.attacked[i] == 0.0 {
                    self.attacked[i] = 1.0;
                    self.target = i as f64;
                    self.user(1, me, g);
                }
            }
        }
        if scr_monsterpop(g) > 0.0 {
            if g.glob.flag(13) == 1.0 {
                if g.input.pressed(Key::B1) && self.havechar[0] == 1.0 {
                    self.scr_boltcheck(me, g, 0);
                }
                if g.input.pressed(Key::B2) && self.havechar[1] == 1.0 {
                    self.scr_boltcheck(me, g, 1);
                }
                if g.input.pressed(Key::B3) && self.havechar[2] == 1.0 {
                    self.scr_boltcheck(me, g, 2);
                }
            } else if g.input.pressed(Key::B1) {
                self.scr_boltcheck_onebutton(me, g);
            }
        } else {
            self.fakefade = 1.0;
            if self.posttimer < (self.timermax - 35.0) {
                self.posttimer = self.timermax - 34.0;
            }
        }
        self.imagetimer += 1.0;
        self.boltx += 1.0;
        for p in self.pressbuffer.iter_mut() {
            *p -= 1.0;
        }
        if self.imagetimer > 1.0 {
            self.imagetimer = 0.0;
        }
        self.goahead = 0.0;
        if (self.attacked[0] == 1.0 || self.havechar[0] == 0.0)
            && (self.attacked[1] == 1.0 || self.havechar[1] == 0.0)
            && (self.attacked[2] == 1.0 || self.havechar[2] == 0.0)
        {
            self.goahead = 1.0;
        }
        if scr_monsterpop(g) == 0.0 {
            self.goahead = 1.0;
        }
        if self.goahead == 1.0 {
            self.posttimer += 1.0;
            if self.posttimer > (self.timermax - 35.0) && self.haveauto == 1.0 && self.autoed == 0.0 && scr_monsterpop(g) > 0.0 {
                let sus = self.sus;
                let tgt = (g.glob.chartarget[sus] as usize).min(2);
                let miss = g.glob.automiss[tgt] == 1.0;
                AttackPress::set_hero(g, sus, |_, h| {
                    h.points = if miss { 0.0 } else { 160.0 };
                    h.state = 1.0;
                    h.attacktimer = 0.0;
                    h.is_auto_susie = 1.0;
                });
                self.posttimer -= 25.0;
                self.autoed = 1.0;
            }
            if self.posttimer > self.timermax {
                self.fade = 1.0;
                for id in g.ids_of("obj_heroparent") {
                    if let Some((_, h)) = g.get::<Hero>(id) {
                        if h.state == 1.0 {
                            h.state = 0.0;
                        }
                        h.attacked = 0.0;
                        h.itemed = 0.0;
                    }
                }
                self.techwon = 0.0;
                if scr_monsterpop(g) == 0.0 {
                    self.techwon = 1.0;
                }
                if self.techwon == 1.0 {
                    call_scr_wincombat(g);
                }
                if self.techwon == 0.0 {
                    if g.glob.mnfight != 1.5 && g.glob.mnfight != 2.0 {
                        g.glob.mnfight = 1.0;
                    }
                    g.glob.myfight = -1.0;
                }
            }
        }
        if self.fade == 1.0 || self.fakefade == 1.0 {
            self.fadeamt += 0.08;
            g.gfx.draw_set_color(C_BLACK);
            g.gfx.draw_set_alpha(self.fadeamt);
            g.gfx.draw_rectangle(x - 1.0, y, x + 640.0, y + 300.0, false);
            g.gfx.draw_set_alpha(1.0);
            if self.fade == 1.0 && self.fadeamt > 1.0 {
                g.destroy_self(me);
            }
        }
    }

    /// Other_11 (event_user(1)): hand the bar's points to the hero and start its attack.
    fn user(&mut self, n: usize, _me: &mut Inst, g: &mut Game) {
        if n != 1 || scr_monsterpop(g) <= 0.0 {
            return;
        }
        for i in 0..3 {
            if self.target == i as f64 {
                let pts = self.points[i];
                AttackPress::set_hero(g, i, |_, h| {
                    h.points = pts;
                    h.state = 1.0;
                    h.attacktimer = 0.0;
                });
            }
        }
    }

    obj_vars!(active, fade, fadeamt, fakefade, posttimer, timermax, boltx, target, goahead);
}
