//! "Flurry" (knight myattackchoice 2, obj_dbulletcontroller type 99).
//!
//! obj_roaringknight_boxsplitter_attack spawns obj_roaringknight_splitslash lines across the box; when a
//! slash lands it creates/reuses obj_knight_split_growtangle, which splits the bullet board in two halves,
//! pushes the SOUL with its half and fires obj_roaringknight_fountain_bullet rows from the crack.
//!
//! Also ported (not reachable from Flurry, but requested): obj_roaringknight_slash, obj_fake_gt,
//! obj_knight_warp, and the legacy user events 2/3 of obj_knight_split_growtangle.
//!
//! Decompiler note: several GML object references were decompiled as raw object indices:
//! `knight = 345` (obj_knight_enemy), `growtangle = 1517` (obj_growtangle), `_splitter = 182`
//! (obj_knight_split_growtangle), `split_bullet = 793` (obj_roaringknight_fountain_bullet).

use crate::assets::{spr, Spr, NO_SPR};
use crate::battle::{self, DbCtrl, Growtangle, Heart, KnightEnemy, RegVars};
use crate::gfx::{bm, merge_color, Color, C_BLACK, C_GRAY, C_RED, C_WHITE};
use crate::gm::*;
use crate::obj_vars;
use crate::rt::{Afterimage, Game, Id, Inst, Marker, Object, NOONE};

// ============================================================================ controller

/// obj_dbulletcontroller Step_0, `if (type == 99)`.
pub fn ctrl_step(c: &mut DbCtrl, me: &mut Inst, g: &mut Game) {
    if c.made == 0.0 {
        if let Some(k) = g.inst_mut(c.creatorid) {
            k.image_alpha = 0.0;
        }
        let (kx, ky) = g.inst(c.creatorid).map(|k| (k.x, k.y)).unwrap_or((0.0, 0.0));
        let slasher = g.instance_create(kx, ky, Box::new(BoxSplitterAttack::default()));
        battle::scr_bullet_inherit(me, slasher, g);
        let difficulty = c.difficulty;
        if let Some((_, o)) = g.get::<BoxSplitterAttack>(slasher) {
            o.difficulty = difficulty;
        }
        g.with_t::<BoxSplitterAttack, _>(slasher, |sme, o, g| {
            o.turn_type = "full";
            o.user(0, sme, g);
        });
        c.made = 1.0;
    }
}

// ============================================================================ helper scripts

/// clamp01()
fn clamp01(v: f64) -> f64 { clamp(v, 0.0, 1.0) }

/// inverselerp() / scr_inverselerp()
fn inverselerp(a: f64, b: f64, v: f64) -> f64 {
    if b == a {
        return 0.0;
    }
    (v - a) / (b - a)
}

/// remap_clamped(a0, a1, b0, b1, v)
fn remap_clamped(a0: f64, a1: f64, b0: f64, b1: f64, v: f64) -> f64 {
    let r = lerp(b0, b1, inverselerp(a0, a1, v));
    clamp(r, b0.min(b1), b0.max(b1))
}

/// randomsign()
fn randomsign(g: &mut Game) -> f64 { g.irandom(1.0) * 2.0 - 1.0 }

/// safe_delete(inst)
fn safe_delete(g: &mut Game, id: Id) {
    if g.id_exists(id) {
        g.destroy(id);
    }
}

/// scr_dark_marker(x, y, sprite)
fn scr_dark_marker(g: &mut Game, x: f64, y: f64, sprite: Spr) -> Id {
    let id = g.instance_create(x, y, Box::new(Marker));
    if let Some(m) = g.inst_mut(id) {
        m.sprite_index = sprite;
        m.image_speed = 0.0;
        m.image_xscale = 2.0;
        m.image_yscale = 2.0;
    }
    id
}

/// scr_bulletparent_count(): instances whose object_index is exactly obj_bulletparent.
fn scr_bulletparent_count(g: &mut Game) -> usize {
    let ids = g.ids_of("obj_bulletparent");
    ids.into_iter().filter(|&id| g.inst(id).map(|i| i.object == "obj_bulletparent").unwrap_or(false)).count()
}

/// Builtins of the first obj_growtangle we need (copied out to release the borrow on `g`).
#[derive(Clone, Copy)]
struct GtInfo {
    id: Id,
    xstart: f64,
    ystart: f64,
    depth: f64,
    image_blend: Color,
    image_xscale: f64,
    image_yscale: f64,
    sprite_index: Spr,
}
fn gt_info(g: &mut Game) -> Option<GtInfo> {
    let id = g.first("obj_growtangle")?;
    let i = g.inst(id)?;
    Some(GtInfo {
        id,
        xstart: i.xstart,
        ystart: i.ystart,
        depth: i.depth,
        image_blend: i.image_blend,
        image_xscale: i.image_xscale,
        image_yscale: i.image_yscale,
        sprite_index: i.sprite_index,
    })
}

/// (x, y) of the first obj_heart (top-left).
fn heart_xy(g: &mut Game) -> (f64, f64) { g.first_inst("obj_heart").map(|h| (h.x, h.y)).unwrap_or((0.0, 0.0)) }

/// (x, y) of an instance by id (for GML `inst.x` on another instance).
fn inst_xy(g: &Game, id: Id) -> (f64, f64) { g.inst(id).map(|i| (i.x, i.y)).unwrap_or((0.0, 0.0)) }

/// scr_heartclamp(a0, a1)
fn scr_heartclamp(g: &mut Game, a0: f64, a1: f64) {
    let Some(gt) = gt_info(g) else { return };
    let xthick = gt.image_xscale * 2.0 + 1.0;
    let ythick = gt.image_yscale * 2.0 + 1.0;
    let (b0, b1, b2, b3) = (battle::scr_get_box(g, 0), battle::scr_get_box(g, 1), battle::scr_get_box(g, 2), battle::scr_get_box(g, 3));
    for h in g.ids_of("obj_heart") {
        if let Some(h) = g.inst_mut(h) {
            h.x = clamp(h.x, b2 + xthick + a0, b0 - (20.0 + xthick + a0));
            h.y = clamp(h.y, b1 + ythick + a1, b3 - (20.0 + ythick + a1));
        }
    }
}

/// Does `me`'s collision mask cover world point (px, py)? (mirror of the runtime's private `covers`)
fn self_covers(g: &Game, me: &Inst, px: f64, py: f64, bb: &[f64; 4]) -> bool {
    let Some(sp) = g.sprite(g.mask_of(me)) else { return false };
    if sp.kind == 0 {
        return px >= bb[0] && px <= bb[2] + 1.0 && py >= bb[1] && py <= bb[3] + 1.0;
    }
    let (sn, cs) = (me.image_angle.to_radians().sin(), me.image_angle.to_radians().cos());
    let dx = px - me.x;
    let dy = py - me.y;
    let lx = dx * cs - dy * sn;
    let ly = dx * sn + dy * cs;
    if me.image_xscale == 0.0 || me.image_yscale == 0.0 {
        return false;
    }
    let lx = lx / me.image_xscale + sp.ox;
    let ly = ly / me.image_yscale + sp.oy;
    let (mx, my) = (lx.floor() as i64, ly.floor() as i64);
    if sp.kind == 2 || sp.masks.is_empty() {
        return mx as f64 >= sp.bl && mx as f64 <= sp.br && my as f64 >= sp.bt && my as f64 <= sp.bb;
    }
    let n = sp.frame_count() as i64;
    let fi = (me.image_index.floor() as i64).rem_euclid(n.max(1)) as usize;
    sp.mask_at(fi, mx, my)
}

/// collision_rectangle(x1, y1, x2, y2, id /* self */, precise) — the runtime's version skips the
/// executing instance, so this checks `me` directly with the same sampling.
fn self_collision_rectangle(g: &Game, me: &Inst, x1: f64, y1: f64, x2: f64, y2: f64) -> bool {
    let (l, r) = (x1.min(x2), x1.max(x2));
    let (t, b) = (y1.min(y2), y1.max(y2));
    let Some(bb) = g.bbox(me) else { return false };
    let il = l.max(bb[0]);
    let ir = r.min(bb[2]);
    let it = t.max(bb[1]);
    let ib = b.min(bb[3]);
    if il > ir || it > ib {
        return false;
    }
    let mut yy = it.floor();
    while yy <= ib {
        let mut xx = il.floor();
        while xx <= ir {
            if self_covers(g, me, xx + 0.5, yy + 0.5, &bb) {
                return true;
            }
            xx += 1.0;
        }
        yy += 1.0;
    }
    false
}

/// scr_precise_hit(size = 3, circle = false)
fn scr_precise_hit(g: &mut Game, me: &Inst, size: f64) -> bool {
    let r = size / 2.0;
    let (hx, hy) = heart_xy(g);
    let (hx, hy) = (hx + 10.0, hy + 10.0);
    if r <= 0.0 {
        let Some(bb) = g.bbox(me) else { return false };
        return hx >= bb[0] && hx <= bb[2] + 1.0 && hy >= bb[1] && hy <= bb[3] + 1.0 && self_covers(g, me, hx, hy, &bb);
    }
    self_collision_rectangle(g, me, hx - r, hy - r, hx + r, hy + r)
}

/// scr_randomtarget_old() — `global.charcantarget[i]` approximated as "slot has a living member".
// TODO(battle): use global.charcantarget / global.targeted once the battle core exposes them.
fn scr_randomtarget_old(g: &mut Game) -> f64 {
    let can = |g: &Game, i: usize| g.glob.char[i] != 0 && g.glob.hp[g.glob.char[i]] > 0.0;
    let able = (0..3).any(|i| can(g, i));
    let mut t = g.choose(&[0usize, 1, 2]);
    if able {
        while !can(g, t) {
            t = g.choose(&[0usize, 1, 2]);
        }
        t as f64
    } else {
        3.0
    }
}

/// scr_damage_maxhp(fraction, ignore_defend, nonlethal) — the HP arithmetic of the game's script.
// TODO(battle): scr_damage_cache, chararmor 23 (damagecounter / halved damage), obj_shake, hero hurt
// animation (global.charinstance), obj_dmgwriter, scr_dead, scr_battle_sprite_set("kris", fallen),
// scr_element_damage_reduction, scr_damage_check and scr_gameover belong to the battle core. The
// desired shared API is `battle::scr_damage_maxhp(me, g, frac, ignore_defend, nonlethal)`.
fn scr_damage_maxhp(me: &mut Inst, g: &mut Game, frac: f64, ignore_defend: bool, nonlethal: bool) {
    if g.glob.inv >= 0.0 {
        return;
    }
    let charid = |g: &Game, slot: f64| -> usize { if (0.0..3.0).contains(&slot) { g.glob.char[slot as usize] } else { 0 } };
    if me.target < 3.0 && g.glob.hp[charid(g, me.target)] <= 0.0 {
        me.target = scr_randomtarget_old(g);
    }
    let mut remtarget = -1.0;
    if me.target == 4.0 {
        remtarget = 4.0;
        me.target = scr_randomtarget_old(g);
        // TODO(battle): the scr_party_hpaverage() re-rolls
    }
    let knight_aoe = g.get_first::<KnightEnemy>("obj_knight_enemy").map(|(_, k)| k.aoedamage);
    if let Some(aoedamage) = knight_aoe {
        if !g.exists("obj_knight_roaring2") && !aoedamage && me.target == 0.0 {
            if g.glob.hp[2] > 0.0 && g.glob.hp[3] > 0.0 {
                me.target = g.choose(&[1.0, 2.0]);
            } else if g.glob.hp[2] > 0.0 {
                me.target = 1.0;
            } else if g.glob.hp[3] > 0.0 {
                me.target = 2.0;
            }
        }
    }
    let chartarget = charid(g, me.target);
    let mut tdamage = (g.glob.maxhp[chartarget] * frac).ceil();
    if me.target < 3.0 && g.glob.charaction[me.target as usize] == 10.0 && !ignore_defend {
        tdamage = (tdamage / 1.5).ceil();
    }
    if nonlethal {
        tdamage = clamp(tdamage, 1.0, g.glob.hp[chartarget] - 1.0);
    }
    if let Some((_, h)) = g.get_first::<Heart>("obj_heart") {
        h.dmgnoise = 1.0;
    }
    if me.target < 3.0 {
        if g.glob.hp[chartarget] <= 0.0 {
            g.glob.hp[chartarget] -= round(tdamage / 4.0);
        } else {
            g.glob.hp[chartarget] -= tdamage;
            if g.glob.hp[chartarget] <= 0.0 {
                if g.exists("obj_knight_enemy") {
                    if g.glob.hp[1] < 0.0 {
                        g.glob.hp[1] = 1.0;
                    } else {
                        g.glob.hp[chartarget] = -999.0;
                    }
                } else {
                    g.glob.hp[chartarget] = round(-g.glob.maxhp[chartarget] / 2.0);
                }
            }
        }
    }
    if me.target == 3.0 {
        for hpi in 0..3 {
            let ct = g.glob.char[hpi];
            if g.glob.hp[ct] >= 0.0 {
                tdamage = battle::scr_damage_calculation(g, tdamage, hpi);
                tdamage = tdamage.ceil();
                if g.glob.charaction[hpi] == 10.0 {
                    g.glob.hp[ct] -= (3.0 * tdamage / 4.0).ceil();
                } else {
                    g.glob.hp[ct] -= tdamage;
                }
                if g.glob.hp[ct] <= 0.0 {
                    g.glob.hp[ct] = round(-g.glob.maxhp[0] / 2.0);
                }
            }
        }
    }
    g.glob.inv = g.glob.invc * 30.0;
    if remtarget != -1.0 {
        me.target = remtarget;
    }
}

/// draw_sprite_general with GameMaker's handling of negative part sizes (the quad extends backwards
/// from (x, y) and the texels run backwards too, i.e. the part is drawn unmirrored, ending at x/y).
fn draw_sprite_general_signed(
    g: &mut Game, s: Spr, sub: f64, mut l: f64, mut t: f64, mut w: f64, mut h: f64, mut x: f64, mut y: f64, xs: f64, ys: f64, rot: f64, col: Color,
    alpha: f64,
) {
    let (sn, cs) = (rot.to_radians().sin(), rot.to_radians().cos());
    if w < 0.0 {
        l += w;
        let dx = w * xs;
        x += dx * cs;
        y -= dx * sn;
        w = -w;
    }
    if h < 0.0 {
        t += h;
        let dy = h * ys;
        x += dy * sn;
        y += dy * cs;
        h = -h;
    }
    g.draw_sprite_general(s, sub, l, t, w, h, x, y, xs, ys, rot, [col; 4], alpha);
}

/// draw_sprite_part_ext_rot(sprite, sub, left, top, width, height, x, y, xscale, yscale, rot, col, alpha)
fn draw_sprite_part_ext_rot(
    g: &mut Game, s: Spr, sub: f64, mut left: f64, mut top: f64, w: f64, h: f64, x: f64, y: f64, xs: f64, ys: f64, rot: f64, col: Color, alpha: f64,
) {
    if left < 0.0 {
        left = 0.0;
    }
    if top < 0.0 {
        top = 0.0;
    }
    let xo = g.sprite_get_xoffset(s) * xs;
    let yo = g.sprite_get_yoffset(s) * ys;
    let nx = left * xs;
    let ny = top * ys;
    let theta = point_direction(xo, yo, nx, ny) + rot;
    let radius = point_distance(xo, yo, nx, ny);
    let xx = x + lengthdir_x(radius, theta);
    let yy = y + lengthdir_y(radius, theta);
    draw_sprite_general_signed(g, s, sub, left, top, w / xs - left, h / ys - top, xx, yy, xs, ys, rot, col, alpha);
}

/// Store into a GML array slot (arrays grow with 0-filled gaps).
fn arr_set(v: &mut Vec<Id>, i: f64, id: Id) {
    let i = i.max(0.0) as usize;
    if i >= v.len() {
        v.resize(i + 1, 0);
    }
    v[i] = id;
}

// ============================================================================ obj_roaringknight_boxsplitter_attack

/// obj_roaringknight_boxsplitter_attack (parent obj_bulletparent): the Knight's stand-in that spawns
/// the split slashes and ends the turn.
#[derive(Default)]
pub struct BoxSplitterAttack {
    pub hell_surface: i32,
    pub spawn_speed: f64,
    pub spawn_range: f64,
    pub min_angle: f64,
    pub max_angle: f64,
    pub timer: f64,
    pub slash_count: f64,
    pub animtimer: f64,
    pub count: f64,
    pub aetimer: f64,
    pub recoil: f64,
    pub final_slash_anim: bool,
    pub slash_anim_count: f64,
    pub flip: bool,
    pub flipped: f64,
    pub forward: f64,
    pub auto: bool,
    pub flip_mode: bool,
    pub knight: Id,
    pub turn_segment: f64,
    pub local_turntimer: f64,
    pub next_up: f64,
    pub next_next_up: f64,
    pub splitbox: Id,
    pub anchor_x: f64,
    pub anchor_y: f64,
    pub done: bool,
    pub omae_wa_con: f64,
    pub omae_wa_timer: f64,
    pub vertical: f64,
    pub difficulty: f64,
    /// obj_growtangle at first, then the obj_knight_split_growtangle instance
    pub growtangle: Id,
    pub init: bool,
    pub force_swap: f64,
    pub first_vertical: bool,
    pub diagonal: f64,
    pub force_oneside: f64,
    pub turn_type: &'static str,
}

impl Object for BoxSplitterAttack {
    fn name(&self) -> &'static str { "obj_roaringknight_boxsplitter_attack" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::bullet_init(me);
        self.hell_surface = -4;
        self.spawn_speed = 40.0;
        self.spawn_range = 4.0;
        self.min_angle = 145.0;
        self.max_angle = 215.0;
        self.timer = 200.0;
        self.slash_count = 0.0;
        me.image_alpha = 1.0;
        me.image_xscale = 2.0;
        me.image_yscale = 2.0;
        if let Some(h) = g.first_inst("obj_heart") {
            me.depth = h.depth + 1.0;
        }
        me.image_speed = 0.0;
        me.image_index = 1.0;
        self.animtimer = 5.0;
        self.count = 3.0;
        self.aetimer = 0.0;
        self.recoil = 0.0;
        self.final_slash_anim = false;
        self.slash_anim_count = 0.0;
        self.flip = false;
        self.flipped = -1.0;
        self.forward = 0.0;
        self.auto = false;
        self.flip_mode = true;
        self.knight = NOONE;
        self.turn_segment = -1.0;
        self.local_turntimer = 330.0;
        self.next_up = -1.0;
        self.next_next_up = -1.0;
        self.auto = true;
        self.splitbox = NOONE;
        self.knight = NOONE;
        if g.exists("obj_knight_enemy") {
            self.knight = g.first("obj_knight_enemy").unwrap_or(NOONE);
        }
        self.anchor_x = me.x;
        self.anchor_y = me.y;
        self.done = false;
        self.omae_wa_con = 0.0;
        self.omae_wa_timer = 0.0;
        self.vertical = 0.0;
        self.difficulty = 2.0;
        self.growtangle = g.first("obj_growtangle").unwrap_or(NOONE);
        self.init = false;
        self.force_swap = -1.0;
        self.first_vertical = false;
        self.diagonal = 0.0;
        self.force_oneside = g.irandom(1.0);
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.local_turntimer -= 1.0;
        if !self.init {
            if self.difficulty == 0.0 {
                self.spawn_speed = 50.0;
            } else if self.difficulty == 1.0 {
                self.spawn_speed = 46.0;
                self.force_swap = g.irandom(2.0) + 1.0;
            } else if self.difficulty == 2.0 {
                self.spawn_speed = 31.0;
            }
            self.init = true;
            self.vertical = g.irandom(1.0);
        }
        if !self.auto {
            return;
        }
        if self.knight == NOONE {
            self.knight = g.first("obj_knight_enemy").unwrap_or(NOONE);
        }
        if self.local_turntimer <= 30.0 {
            let idle = spr("spr_roaringknight_idle");
            if self.local_turntimer <= 10.0 && me.sprite_index != idle {
                if me.image_xscale < 0.0 {
                    me.x -= 220.0;
                }
                me.image_xscale = me.image_xscale.abs();
                me.sprite_index = idle;
                me.image_index = 0.0;
            } else if self.local_turntimer < 22.0 && me.image_xscale < 0.0 {
                me.image_index = 4.0;
            }
            let kx = g.first_inst("obj_knight_enemy").map(|k| k.x).unwrap_or(me.x);
            if me.x < kx {
                me.x += 1.0;
            }
            let ltt = self.local_turntimer.max(0.0);
            let ky = g.inst(self.knight).map(|k| k.y).unwrap_or(me.y);
            me.y = lerp(me.y, ky, (50.0 - ltt) / 50.0);
            // (GML would error if obj_knight_split_growtangle were missing; treat that as split == false)
            let split = g.get_first::<SplitGrowtangle>("obj_knight_split_growtangle").map(|(_, s)| s.split).unwrap_or(false);
            if self.local_turntimer < 0.0 && !split {
                g.glob.turntimer = 0.0;
                g.destroy_self(me);
            }
            return;
        }
        self.timer += 1.0;
        if self.timer >= self.spawn_speed {
            self.timer = 0.0;
            self.vertical = g.irandom(1.0);
            if self.difficulty == 0.0 {
                self.vertical = self.force_oneside;
            }
            let (gx, gy) = inst_xy(g, self.growtangle);
            let slash = g.instance_create(gx, gy, Box::new(SplitSlash::default()));
            let vertical = self.vertical;
            if let Some((_, s)) = g.get::<SplitSlash>(slash) {
                s.vertical = vertical;
            }
            if self.difficulty == 3.0 {
                let diagonal = self.diagonal;
                if let Some((_, s)) = g.get::<SplitSlash>(slash) {
                    s.diagonal = diagonal;
                }
                if truthy(self.diagonal) {
                    self.timer = -4.0;
                    self.diagonal = 0.0;
                } else {
                    self.diagonal = g.irandom(1.0);
                }
            }
            self.slash_count += 1.0;
            if self.difficulty <= 2.0 && self.spawn_speed > 40.0 {
                self.spawn_speed = scr_movetowards(self.spawn_speed, 40.0, 3.0);
            }
            self.spawn_range = scr_approach(self.spawn_range, 60.0, 3.0);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if me.image_alpha == 1.0 {
            if self.animtimer < 4.0 {
                self.animtimer += 1.0;
            } else if me.image_index == 1.0 || me.image_index == 4.0 {
                me.image_index += 1.0;
            }
            g.draw_self(me);
            self.aetimer += 1.0;
            if self.aetimer % 4.0 == 0.0 && me.image_alpha != 0.0 {
                let fade = g.afterimage(me);
                let gtx = g.first_inst("obj_growtangle").map(|b| b.x).unwrap_or(0.0);
                let (sprite, depth, x) = (me.sprite_index, me.depth, me.x);
                if let Some((fi, fo)) = g.get::<Afterimage>(fade) {
                    fi.sprite_index = sprite;
                    fi.image_alpha = 0.6;
                    fo.fade_speed = 0.02;
                    fi.set_hspeed(2.0 * sign(x - gtx));
                    fi.depth = if x < gtx { depth + 50.0 } else { depth + 100.0 };
                }
            }
        }
        if !g.gfx.surface_exists(self.hell_surface) {
            self.hell_surface = g.gfx.surface_create(142, 142);
        }
        g.gfx.draw_set_blend_mode(bm::NORMAL);
        g.gfx.surface_set_target(self.hell_surface);
        g.gfx.draw_clear_alpha(C_BLACK, 0.0);
        // (var _gtx = gt_minx() + 5; var _gty = gt_miny() + 5; — computed but unused in GML)
        struct S {
            timer: f64,
            flip: f64,
            blend: Color,
            xoffset: f64,
            yoffset: f64,
            angle: f64,
            angleoffset: f64,
        }
        let mut list = vec![];
        for id in g.ids_of("obj_roaringknight_splitslash") {
            if let Some((i, s)) = g.get::<SplitSlash>(id) {
                if !s.slash {
                    list.push(S {
                        timer: s.timer,
                        flip: s.flip,
                        blend: i.image_blend,
                        xoffset: s.xoffset,
                        yoffset: s.yoffset,
                        angle: i.image_angle,
                        angleoffset: s.angleoffset,
                    });
                }
            }
        }
        let px = spr("spr_pxwhite10_center");
        let flow = spr("spr_knight_bullet_flow");
        for s in list {
            let ease = scr_ease_out(clamp01(s.timer / 30.0), 3);
            let spin = (ease * 15.0 - 15.0) * s.flip;
            let backing = merge_color(C_BLACK, s.blend, 0.5);
            let size = lerp(4.0, 0.0, ease);
            let length = clamp01(s.timer / 30.0) * 90.0;
            g.draw_sprite_ext(px, 0.0, 71.0 + s.xoffset, 71.0 + s.yoffset, length, size, spin + s.angle + s.angleoffset, backing, 1.0);
            g.gfx.gpu_set_blendmode_ext(bm::DEST_ALPHA, bm::DEST_ALPHA);
            g.draw_sprite_tiled_ext(flow, 2.0, s.timer, s.timer, 0.25, 0.25, s.blend, 1.0);
            g.draw_sprite_tiled_ext(flow, 2.0, -s.timer + 40.0, -s.timer + 40.0, 0.25, 0.25, s.blend, 1.0);
            g.gfx.gpu_set_blendmode(bm::NORMAL);
        }
        g.gfx.surface_reset_target();
        g.gfx.draw_set_blend_mode(bm::ADD);
        let (gx, gy) = g.first_inst("obj_growtangle").map(|b| (b.x, b.y)).unwrap_or((0.0, 0.0));
        g.gfx.draw_surface(self.hell_surface, gx - 71.0, gy - 71.0);
        g.gfx.draw_set_blend_mode(bm::NORMAL);
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.gfx.surface_exists(self.hell_surface) {
            g.gfx.surface_free(self.hell_surface);
        }
        if self.turn_type != "start" && self.turn_type != "short start" && self.turn_type != "short mid" && scr_bulletparent_count(g) < 2 {
            if let Some(k) = g.inst_mut(self.knight) {
                k.image_alpha = 1.0;
            }
            g.glob.turntimer = -1.0;
        }
    }

    // event_user(0): no Other_10 on this object or its parents — nothing to run.

    obj_vars!(timer, local_turntimer, animtimer, spawn_speed);
}

// ============================================================================ obj_roaringknight_splitslash

/// obj_roaringknight_splitslash (parent obj_collidebullet).
#[derive(Default)]
pub struct SplitSlash {
    pub timer: f64,
    pub slash: bool,
    pub thickness: f64,
    pub trailthickness: f64,
    pub xdir: f64,
    pub ydir: f64,
    pub xdraw: f64,
    pub ydraw: f64,
    pub init: bool,
    pub flip: f64,
    pub slashmarker: Id,
    pub vertical: f64,
    pub memheartx: f64,
    pub memhearty: f64,
    pub playerstrike: f64,
    pub xoffset: f64,
    pub yoffset: f64,
    pub angleoffset: f64,
    pub startdepth: f64,
    pub difficulty: f64,
    pub slice_delay: f64,
    pub hurt_delay: f64,
    pub diagonal: f64,
    pub cuty: f64,
}

impl Object for SplitSlash {
    fn name(&self) -> &'static str { "obj_roaringknight_splitslash" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::bullet_init(me);
        me.active = 0.0;
        self.timer = 0.0;
        me.image_alpha = 1.0;
        me.image_speed = 0.0;
        self.slash = false;
        me.destroyonhit = 0.0;
        self.thickness = 10.0;
        me.image_blend = C_BLACK;
        self.trailthickness = 10.0;
        self.xdir = 0.0;
        self.ydir = 0.0;
        self.xdraw = 250.0;
        self.ydraw = 250.0;
        self.init = false;
        self.flip = g.choose(&[-1.0, 1.0]);
        self.timer = 0.0;
        me.damage = 206.0;
        me.element = 5.0;
        me.grazepoints = 10.0;
        self.slashmarker = NOONE;
        self.vertical = 0.0;
        self.memheartx = 0.0;
        self.memhearty = 0.0;
        self.playerstrike = 0.0;
        self.xoffset = 0.0;
        self.yoffset = 0.0;
        self.angleoffset = 0.0;
        self.startdepth = me.depth;
        self.difficulty = 0.0;
        self.slice_delay = 5.0;
        self.hurt_delay = 15.0;
        self.diagonal = 0.0;
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer += 1.0;
        if !self.init {
            let gtdepth = g.first_inst("obj_growtangle").map(|b| b.depth).unwrap_or(0.0);
            self.startdepth = me.depth;
            me.depth = gtdepth + 10.0;
            self.init = true;
            self.slashmarker = scr_dark_marker(g, me.x, me.y, spr("spr_rk_quickslash_upper"));
            if let Some(m) = g.inst_mut(self.slashmarker) {
                m.depth = gtdepth + 50.0;
                m.image_speed = 0.0;
                m.image_alpha = 0.0;
            }
            self.angleoffset = g.random_range(-12.0, 12.0);
            let slash_count =
                g.get_first::<BoxSplitterAttack>("obj_roaringknight_boxsplitter_attack").map(|(_, b)| b.slash_count).unwrap_or(0.0);
            let odd = slash_count % 2.0 == 1.0;
            if truthy(self.diagonal) {
                me.set_direction(if odd { -45.0 } else { 45.0 });
                self.vertical = if odd { 1.0 } else { 0.0 };
                me.image_angle = me.direction();
                self.xoffset = g.random_range(-2.0, 2.0) * 2.0;
                self.yoffset = g.random_range(-2.0, 2.0) * 2.0;
            } else if truthy(self.vertical) {
                me.set_direction(if odd { -90.0 } else { 90.0 });
                me.image_angle = me.direction();
                self.xoffset = g.random_range(-8.0, 8.0) * 2.0;
            } else {
                self.yoffset = g.random_range(-8.0, 8.0) * 2.0;
            }
            let ang = me.image_angle;
            if let Some(m) = g.inst_mut(self.slashmarker) {
                m.image_angle = ang;
            }
        }
        if me.image_alpha < 1.0 && !self.slash {
            me.image_alpha += 0.05;
        }
        if !self.slash {
            me.image_blend = merge_color(C_BLACK, C_RED, clamp01(self.timer / 20.0));
        }
        if !self.slash {
            if let Some(m) = g.inst_mut(self.slashmarker) {
                m.image_alpha = 0.0;
            }
        } else if let Some(m) = g.inst_mut(self.slashmarker) {
            m.x = me.x;
            m.y = me.y;
            m.image_index = me.image_index;
            m.image_blend = me.image_blend;
            m.image_alpha = me.image_alpha;
        }
        if self.timer <= 15.0 {
            self.thickness = lerp(10.0, 1.0, scr_ease_out(self.timer / 15.0, 4));
        }
        if self.timer == 29.0 {
            me.depth = self.startdepth;
        }
        if self.timer == 30.0 {
            self.timer_30(me, g);
        }
        if self.timer == 34.0 {
            me.active = 0.0;
        }
        if self.timer == 35.0 + self.hurt_delay && self.playerstrike == 1.0 {
            self.playerstrike = 0.0;
            if let Some(h) = g.first_inst("obj_heart") {
                h.image_alpha = 1.0;
            }
            if me.target != 3.0 {
                scr_damage_maxhp(me, g, 0.66, false, true);
            }
            g.glob.inv = g.glob.invc * 30.0;
            g.destroy_self(me);
        }
    }

    fn end_step(&mut self, _me: &mut Inst, g: &mut Game) {
        if self.playerstrike == 1.0 {
            let (mx, my) = (self.memheartx, self.memhearty);
            if let Some(h) = g.first_inst("obj_heart") {
                if h.x != mx {
                    h.x = mx + sign(h.x - mx);
                }
                if h.y != my {
                    h.y = my + sign(h.y - my);
                }
                self.memheartx = h.x;
                self.memhearty = h.y;
            }
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.slash {
            g.draw_self(me);
        } else {
            g.gfx.gpu_set_blendmode(bm::ADD);
            let ease = scr_ease_out(clamp01(self.timer / 30.0), 3);
            let spin = (ease * 15.0 - 15.0) * self.flip;
            let backing = merge_color(C_BLACK, C_RED, 0.5);
            let gtid = g.get_first::<BoxSplitterAttack>("obj_roaringknight_boxsplitter_attack").map(|(_, b)| b.growtangle).unwrap_or(NOONE);
            let (gx, gy) = inst_xy(g, gtid);
            let size = lerp(4.0, 0.0, ease);
            let length = ease * 180.0;
            g.draw_sprite_ext(
                spr("spr_pxwhite10_center"),
                0.0,
                gx + self.xoffset,
                gy + self.yoffset,
                length,
                size,
                spin + me.image_angle + self.angleoffset,
                backing,
                1.0,
            );
            g.gfx.gpu_set_blendmode(bm::NORMAL);
        }
        if self.playerstrike == 1.0 {
            for hid in g.ids_of("obj_heart") {
                let Some((hs, hi, hx, hy)) = g.inst(hid).map(|h| (h.sprite_index, h.image_index, h.x, h.y)) else { continue };
                let xx = g.irandom(2.0) - 1.0;
                let yy = g.irandom(2.0) - 1.0;
                let fade = remap_clamped(45.0, 55.0, 1.0, 0.0, self.timer);
                g.draw_sprite(hs, hi, hx + xx, hy + yy);
                g.draw_sprite_ext(spr("spr_rk_slash_heartslice"), self.cuty, hx + xx, hy + yy, 1.0, 1.0, 0.0, C_WHITE, fade);
            }
        }
    }

    /// Other_15: the SOUL touched the slash.
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 5 {
            return;
        }
        if me.active == 1.0 && scr_precise_hit(g, me, 3.0) {
            self.playerstrike = 1.0;
            me.active = 0.0;
            let (hx, hy) = heart_xy(g);
            self.memheartx = hx;
            self.memhearty = hy;
            if let Some(h) = g.first_inst("obj_heart") {
                h.image_alpha = 0.0;
            }
            g.glob.inv = -1.0;
            self.cuty = round(remap_clamped(-16.0, 16.0, 1.0, 14.0, hy - (me.y - 8.0)));
            let mut split_wait = 0.0;
            if let Some((_, s)) = g.get_first::<SplitGrowtangle>("obj_knight_split_growtangle") {
                s.split_delay = 5.0;
                split_wait = s.split_wait;
            }
            if let Some((_, b)) = g.get_first::<BoxSplitterAttack>("obj_roaringknight_boxsplitter_attack") {
                b.timer -= 5.0;
                b.local_turntimer += 5.0;
            }
            self.hurt_delay = split_wait;
        }
    }

    /// Other_7
    fn animation_end(&mut self, me: &mut Inst, g: &mut Game) {
        if self.slash && me.visible {
            if self.playerstrike == 0.0 {
                g.destroy_self(me);
            } else {
                me.image_alpha = 0.0;
            }
        }
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) { safe_delete(g, self.slashmarker); }

    obj_vars!(timer, thickness, flip, xoffset, yoffset, angleoffset, playerstrike, hurt_delay);
}

impl SplitSlash {
    /// `if (timer == 30)`: the slash lands.
    fn timer_30(&mut self, me: &mut Inst, g: &mut Game) {
        me.x = me.xstart;
        me.y = me.ystart;
        if me.image_angle == 90.0 {
            me.image_yscale *= -1.0;
        }
        me.image_angle += self.angleoffset;
        me.x += self.xoffset;
        me.y += self.yoffset;
        me.image_blend = C_WHITE;
        me.active = 1.0;
        self.slash = true;
        let splitter;
        if !g.exists("obj_knight_split_growtangle") {
            let (gx, gy) = g.first_inst("obj_growtangle").map(|b| (b.x, b.y)).unwrap_or((0.0, 0.0));
            splitter = g.instance_create(gx, gy, Box::new(SplitGrowtangle::default()));
            battle::scr_bullet_inherit(me, splitter, g);
            if let Some(s) = g.inst_mut(splitter) {
                s.grazepoints = 5.0;
            }
            for b in g.ids_of("obj_roaringknight_boxsplitter_attack") {
                let mut diff = None;
                if let Some((_, bo)) = g.get::<BoxSplitterAttack>(b) {
                    bo.growtangle = splitter;
                    diff = Some(bo.difficulty);
                }
                if let (Some(d), Some((_, so))) = (diff, g.get::<SplitGrowtangle>(splitter)) {
                    so.difficulty = d;
                }
            }
        } else {
            splitter = g.first("obj_knight_split_growtangle").unwrap_or(NOONE);
        }
        let (xo, yo, ao, vert, diag) = (self.xoffset, self.yoffset, self.angleoffset, self.vertical, self.diagonal);
        if let Some((_, s)) = g.get::<SplitGrowtangle>(splitter) {
            s.xoffset = xo;
            s.yoffset = yo;
            s.angle = ao;
        }
        if let Some((_, s)) = g.get_first::<SplitGrowtangle>("obj_knight_split_growtangle") {
            s.vertical = vert;
            s.diagonal = diag;
            s.con = 1.0;
            s.timer = 0.0;
        }
        me.sprite_index = spr("spr_rk_quickslash");
        me.image_speed = 1.0;
        me.image_index = 0.0;
        me.image_yscale *= 2.0;
        for b in g.ids_of("obj_roaringknight_boxsplitter_attack") {
            if let Some((bi, bo)) = g.get::<BoxSplitterAttack>(b) {
                if bi.image_index >= 4.0 {
                    bi.image_index = 1.0;
                } else {
                    bi.image_index = 4.0;
                }
                bo.animtimer = 0.0;
            }
        }
        g.snd_stop("snd_wideslash_low");
        g.snd_stop("snd_knight_hurtb");
        let pitch = 0.9 + g.random(4.0) / 10.0;
        g.snd_play_x("snd_wideslash_low", 0.8, pitch);
        let mut angle = me.image_angle;
        if me.image_xscale < 0.0 {
            angle += 180.0;
        }
        let dirx = lengthdir_x(60.0, angle);
        let diry = lengthdir_y(60.0, angle);
        let slash_mark = spr("spr_knight_slash_mark");
        for i in 0..16 {
            let debris = g.instance_create(me.xstart + self.xoffset, me.ystart + self.yoffset, Box::new(Afterimage::default()));
            // with (_debris) { ... } — random calls in GML order
            let mut spd = g.random_range(10.0, 20.0);
            let rs = randomsign(g);
            let mut dir = (me.image_angle + ((20.0 - spd) * rs) / 2.0 + 180.0).rem_euclid(360.0);
            spd += g.random_range(-2.0, 2.0);
            let even = i % 2 == 0;
            if even {
                dir = (dir - 180.0).rem_euclid(360.0);
                spd *= 0.75;
            }
            let fade_add = g.random(0.02);
            let mut xscale = 0.0;
            if let Some((d, a)) = g.get::<Afterimage>(debris) {
                d.set_direction(dir);
                d.set_speed(spd);
                if even {
                    d.x += dirx;
                    d.y += diry;
                } else {
                    d.x -= dirx;
                    d.y -= diry;
                }
                d.image_angle = d.direction();
                d.sprite_index = slash_mark;
                d.image_alpha = 1.0;
                d.image_blend = C_WHITE;
                d.image_xscale = d.speed() / 10.0;
                d.image_yscale = 0.1;
                d.friction = 0.5;
                a.fade_speed += fade_add;
                xscale = d.image_xscale;
            }
            g.lerpvar_instance(debris, "image_xscale", xscale, 0.05, 5.0, 0, "out");
        }
    }
}

// ============================================================================ obj_knight_split_growtangle

/// obj_knight_split_growtangle (parent obj_bulletparent): the bullet board split in two halves.
pub struct SplitGrowtangle {
    pub effect: Id,
    pub con: f64,
    pub timer: f64,
    pub distance: f64,
    pub old_distance: f64,
    pub heart_y: f64,
    pub heart_x: f64,
    pub split_dist: f64,
    pub slow: f64,
    pub fast: f64,
    pub child_bullet: Vec<Id>,
    pub count: f64,
    pub flame_index: f64,
    pub marker: [Id; 2],
    pub split: bool,
    pub vertical: f64,
    pub diagonal: f64,
    pub launch_force: f64,
    pub launch_dir: (f64, f64),
    pub open_time: f64,
    pub boxgone: bool,
    pub max_distance: f64,
    pub split_delay: f64,
    pub vshift: f64,
    pub hshift: f64,
    pub xoffset: f64,
    pub yoffset: f64,
    pub angle: f64,
    pub h_change: f64,
    pub v_change: f64,
    pub update_box: bool,
    pub source_surf: i32,
    pub box_sprite: Spr,
    pub half_box_a: i32,
    pub half_box_b: i32,
    pub difficulty: f64,
    pub split_wait: f64,
    pub split_hold: f64,
    pub init: bool,
    pub bullet_count: f64,
    pub bullet_range: f64,
    pub disable_on_close: bool,
}

impl Default for SplitGrowtangle {
    fn default() -> Self {
        SplitGrowtangle {
            effect: NOONE,
            con: 0.0,
            timer: 0.0,
            distance: 0.0,
            old_distance: 0.0,
            heart_y: 0.0,
            heart_x: 0.0,
            split_dist: 50.0,
            slow: 4.0,
            fast: 8.0,
            child_bullet: vec![NOONE],
            count: 0.0,
            flame_index: 0.0,
            marker: [NOONE; 2],
            split: false,
            vertical: 0.0,
            diagonal: 0.0,
            launch_force: 0.0,
            launch_dir: (0.0, 0.0),
            open_time: 45.0,
            boxgone: false,
            max_distance: 70.0,
            split_delay: 0.0,
            vshift: 0.0,
            hshift: 0.0,
            xoffset: 0.0,
            yoffset: 0.0,
            angle: 0.0,
            h_change: 0.0,
            v_change: 0.0,
            update_box: false,
            source_surf: -4,
            box_sprite: NO_SPR,
            half_box_a: -4,
            half_box_b: -4,
            difficulty: 0.0,
            split_wait: 5.0,
            split_hold: 30.0,
            init: false,
            bullet_count: 13.0,
            bullet_range: 144.0,
            disable_on_close: true,
        }
    }
}

impl Object for SplitGrowtangle {
    fn name(&self) -> &'static str { "obj_knight_split_growtangle" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        *self = SplitGrowtangle::default();
        let gt = gt_info(g);
        if let Some(gt) = gt {
            me.image_blend = gt.image_blend;
            me.image_xscale = gt.image_xscale;
            me.image_yscale = gt.image_yscale;
            me.depth = gt.depth + 100.0;
        }
        self.effect = NOONE;
        self.con = 0.0;
        self.timer = 0.0;
        self.distance = 0.0;
        self.old_distance = 0.0;
        if let Some(gt) = gt {
            if let Some(b) = g.inst_mut(gt.id) {
                b.visible = false;
            }
            me.sprite_index = gt.sprite_index;
        }
        self.heart_y = 0.0;
        self.heart_x = 0.0;
        let flame = spr("spr_rk_split_flame_big");
        let pos = [(me.x + 2.0, me.y - 1.0, 180.0), (me.x, me.y + 2.0, 0.0)];
        for (k, &(mx, my, ang)) in pos.iter().enumerate() {
            let m = g.instance_create(mx, my, Box::new(Marker));
            if let Some(mi) = g.inst_mut(m) {
                mi.sprite_index = flame;
                mi.image_speed = 0.5;
                mi.image_xscale = 2.0;
                mi.image_yscale = 2.0;
                mi.image_angle = ang;
                mi.image_blend = C_GRAY;
                mi.depth = me.depth + 10.0;
            }
            self.marker[k] = m;
        }
        // remaining fields: see Default (split=false, open_time=45, max_distance=70, split_wait=5,
        // split_hold=30, split_bullet=obj_roaringknight_fountain_bullet, bullet_count=13, bullet_range=144, ...)
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if !self.init {
            if self.difficulty == 2.0 {
                self.split_wait = 4.0;
                self.split_hold = 26.0;
            }
            self.init = true;
        }
        self.timer += 1.0;
        self.old_distance = self.distance;
        if self.con == 1.0 {
            if self.timer <= 1.0 {
                self.effect = g.instance_create(me.x, me.y, Box::new(SplitGrowtangleEffect::default()));
                let (a, d, xo, yo, dep, v) = (self.angle, self.diagonal, self.xoffset, self.yoffset, me.depth - 100.0, self.vertical);
                if let Some((ei, e)) = g.get::<SplitGrowtangleEffect>(self.effect) {
                    e.angle = a;
                    e.diagonal = d;
                    e.xoffset = xo;
                    e.yoffset = yo;
                    ei.depth = dep;
                    e.vertical = v;
                }
            }
            if self.timer >= self.split_wait + self.split_delay {
                self.split_open(me, g);
            }
        }
        let hold = if truthy(self.diagonal) { self.split_hold + 2.0 } else { self.split_hold };
        if self.con == 2.0 {
            self.split = true;
            if self.timer == 7.0 {
                let d = me.depth - 10.0;
                for i in 0..(self.count.max(0.0) as usize) {
                    let id = self.child_bullet.get(i).copied().unwrap_or(0);
                    if g.id_exists(id) {
                        if let Some(b) = g.inst_mut(id) {
                            b.depth = d;
                            b.active = 1.0;
                            b.grazed = 0.0;
                        }
                    }
                }
            }
            if self.timer <= hold / 2.0 {
                self.distance = scr_ease_out(self.timer / (self.split_hold / 2.0), 3) * self.max_distance;
                let dd = self.distance - self.old_distance;
                let (hxm, hym, diag, vert) = (self.heart_x, self.heart_y, truthy(self.diagonal), truthy(self.vertical));
                if let Some(h) = g.first_inst("obj_heart") {
                    if diag {
                        h.x += dd * hxm * 1.0;
                        h.y += dd * hym * 1.0;
                    } else if vert {
                        h.x += dd * hxm * 1.25;
                    } else {
                        h.y += dd * hym * 1.25;
                    }
                }
            } else {
                self.user(0, me, g);
            }
        }
        if self.con == 3.0 {
            self.distance = self.max_distance - scr_ease_in(self.timer / (self.split_hold / 2.0), 3) * self.max_distance;
            if self.timer >= hold / 2.0 {
                if truthy(self.vertical) || truthy(self.diagonal) {
                    self.vshift = g.irandom_range(-3.0, 3.0);
                } else {
                    self.hshift = g.irandom_range(-3.0, 3.0);
                }
                if truthy(self.diagonal) {
                    self.hshift = self.vshift;
                }
                self.user(0, me, g);
            }
        }
        if self.con == 4.0 {
            self.distance = scr_movetowards(self.distance, 0.0, 12.0);
            if self.distance == 0.0 {
                self.con = 0.0;
                self.split = false;
                if self.difficulty == 3.0 {
                    if self.split_wait > 3.0 {
                        self.split_wait -= 1.0;
                    }
                    if self.split_hold > 26.0 {
                        self.split_hold -= 2.0;
                    }
                } else {
                    if self.split_wait > 5.0 {
                        self.split_wait -= 1.0;
                    }
                    if self.split_hold > 30.0 {
                        self.split_hold -= 2.0;
                    }
                }
                g.snd_play("snd_locker");
            }
        }
        let dist = round(self.distance);
        let (x, y, xo, yo) = (me.x, me.y, self.xoffset, self.yoffset);
        // (angle0, angle1, x0, x1, y0, y1)
        let m = if truthy(self.diagonal) {
            let (a0, a1) = if truthy(self.vertical) { (-45.0, 135.0) } else { (225.0, 45.0) };
            let sd = 0.5f64.sqrt() * dist;
            (a0, a1, x - sd - 1.0 + xo, x + sd + 3.0 + xo, y - sd - 1.0 + yo, y + sd + 3.0 + yo)
        } else if truthy(self.vertical) {
            (-90.0, 90.0, x - dist - 1.0 + xo, x + dist + 3.0 + xo, y - 1.0 + yo, y + 3.0 + yo)
        } else {
            (180.0, 0.0, x - 1.0 + xo, x + 3.0 + xo, y - dist - 1.0 + yo, y + dist + 3.0 + yo)
        };
        if let Some(mi) = g.inst_mut(self.marker[0]) {
            mi.image_angle = m.0;
            mi.x = m.2;
            mi.y = m.4;
        }
        if let Some(mi) = g.inst_mut(self.marker[1]) {
            mi.image_angle = m.1;
            mi.x = m.3;
            mi.y = m.5;
        }
        let distance = self.distance;
        if let Some(b) = g.first_inst("obj_growtangle") {
            if distance > 0.0 {
                b.x = -9999.0;
            } else {
                b.x = b.xstart;
            }
        }
    }

    fn end_step(&mut self, _me: &mut Inst, g: &mut Game) {
        let mut dist = round(self.distance);
        if self.con == 0.0 {
            dist = 0.0;
        }
        let vert = truthy(self.vertical);
        let diag = truthy(self.diagonal);
        let mut sw = if vert { dist } else { 0.0 };
        let mut sh = if vert { 0.0 } else { dist };
        if diag {
            sw = 0.5f64.sqrt() * dist;
            sh = 0.5f64.sqrt() * dist;
        }
        let Some(gt) = gt_info(g) else { return };
        let tl = (gt.xstart - 70.0 - sw, gt.ystart - 70.0 - sh);
        let br = (gt.xstart + 52.0 + sw, gt.ystart + 52.0 + sh);
        let dist_change = 0.5f64.sqrt() * (dist - round(self.old_distance));
        let Some(h) = g.first_inst("obj_heart") else { return };
        let heart_start = (h.x, h.y);
        if h.x < tl.0 {
            h.x = tl.0;
        }
        if h.x > br.0 {
            h.x = br.0;
        }
        if h.y < tl.1 {
            h.y = tl.1;
        }
        if h.y > br.1 {
            h.y = br.1;
        }
        if diag && dist_change != 0.0 {
            let cx = clamp(h.x - heart_start.0, -dist_change.abs(), dist_change.abs());
            let cy = clamp(h.y - heart_start.1, -dist_change.abs(), dist_change.abs());
            if cx != 0.0 {
                if !vert {
                    h.y += cx;
                } else {
                    h.y -= cx;
                }
            }
            if cy != 0.0 {
                if !vert {
                    h.x += cy;
                } else {
                    h.x -= cy;
                }
            }
        }
        h.x = round(h.x);
        h.y = round(h.y);
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let vert = truthy(self.vertical);
        let mut splid = if vert { round(self.distance) } else { 0.0 };
        let mut dist = if vert { 0.0 } else { round(self.distance) };
        let mut angle = self.angle + if vert { 90.0 } else { 0.0 };
        if truthy(self.diagonal) {
            splid = 0.5f64.sqrt() * self.distance;
            dist = splid;
            angle += 45.0;
        }
        let mut rnd = || if self.distance > 0.0 { g.irandom_range(-1.0, 1.0) } else { 0.0 };
        let xx = rnd();
        let yy = rnd();
        let xx2 = rnd();
        let yy2 = rnd();
        let (x, y) = (me.x, me.y);
        let (xo, yo) = (self.xoffset, self.yoffset);
        if !g.gfx.surface_exists(self.source_surf) {
            self.source_surf = g.gfx.surface_create(170, 170);
            g.gfx.surface_set_target(self.source_surf);
            g.gfx.draw_clear_alpha(C_BLACK, 0.0);
            g.draw_sprite_ext(me.sprite_index, 1.0, 85.0, 85.0, 2.0, 2.0, 0.0, C_WHITE, 1.0);
            g.draw_sprite_ext(me.sprite_index, 0.0, 85.0, 85.0, 2.0, 2.0, 0.0, C_WHITE, 1.0);
            g.gfx.surface_reset_target();
            self.half_box_a = g.gfx.surface_create(170, 170);
            self.half_box_b = g.gfx.surface_create(170, 170);
        }
        if self.distance == 0.0 {
            g.gfx.draw_surface_ext(self.source_surf, x - 85.0, y - 85.0, 1.0, 1.0, 0.0, me.image_blend, 1.0);
            self.update_box = true;
        }
        if self.distance != 0.0 && self.update_box {
            let xmul = lengthdir_x(1.0, angle);
            let ymul = lengthdir_y(1.0, angle);
            let xchange = xmul * 400.0;
            let ychange = ymul * 400.0;
            let xend = if vert { 400.0 } else { 0.0 };
            let yend = if vert { 0.0 } else { 400.0 };
            g.gfx.draw_set_color(C_WHITE);
            g.gfx.surface_set_target(self.half_box_a);
            g.gfx.draw_clear_alpha(C_BLACK, 0.0);
            g.gfx.draw_surface(self.source_surf, 0.0, 0.0);
            g.gfx.gpu_set_blendmode(bm::SUBTRACT);
            let (xnudge, ynudge) = if vert { (1.0, 0.0) } else { (0.0, 1.0) };
            g.gfx.draw_triangle(85.0 + xo - xchange, 85.0 + yo - ychange, 85.0 + xo + xchange, 85.0 + yo + ychange, 85.0 + xo + xend, 85.0 + yo + yend, false);
            g.gfx.gpu_set_blendmode(bm::NORMAL);
            g.gfx.surface_reset_target();
            g.gfx.surface_set_target(self.half_box_b);
            g.gfx.draw_clear_alpha(C_BLACK, 0.0);
            g.gfx.draw_surface(self.source_surf, 0.0, 0.0);
            g.gfx.gpu_set_blendmode(bm::SUBTRACT);
            g.gfx.draw_triangle(
                85.0 + xo - xchange - xnudge,
                85.0 + yo - ychange - ynudge,
                85.0 + xo + xchange - xnudge,
                85.0 + yo + ychange - ynudge,
                85.0 + xo - xend,
                85.0 + yo - yend,
                false,
            );
            g.gfx.gpu_set_blendmode(bm::NORMAL);
            g.gfx.surface_reset_target();
            let change = g.choose(&[-2.0, -1.0, 1.0, 2.0]);
            let deviation;
            if vert {
                deviation = change - self.v_change;
                self.v_change = change;
            } else {
                deviation = change - self.h_change;
                self.h_change = change;
            }
            g.gfx.surface_set_target(self.source_surf);
            g.gfx.draw_clear_alpha(C_BLACK, 0.0);
            g.gfx.draw_surface(self.half_box_a, -xmul * deviation, -ymul * deviation);
            g.gfx.draw_surface(self.half_box_b, xmul * deviation, ymul * deviation);
            g.gfx.draw_set_color(merge_color(C_BLACK, C_WHITE, 0.25));
            let abs_dev = 85.0 + deviation.abs();
            g.gfx.gpu_set_blendmode_ext(bm::DEST_ALPHA, bm::ZERO);
            g.gfx.draw_line(85.0 + xo - xmul * abs_dev, 85.0 + yo - ymul * abs_dev, 85.0 + xo + xmul * abs_dev, 85.0 + yo + ymul * abs_dev);
            g.gfx.gpu_set_blendmode(bm::NORMAL);
            g.gfx.surface_reset_target();
            self.update_box = false;
        }
        if self.distance > 0.0 {
            if truthy(self.diagonal) && vert {
                g.gfx.draw_surface_ext(self.half_box_a, x - splid - 85.0 + xx, y + dist - 85.0 + yy, 1.0, 1.0, 0.0, me.image_blend, 1.0);
                g.gfx.draw_surface_ext(self.half_box_b, x + splid - 85.0 + xx2, y - dist - 85.0 + yy2, 1.0, 1.0, 0.0, me.image_blend, 1.0);
            } else {
                g.gfx.draw_surface_ext(self.half_box_a, x - splid - 85.0 + xx, y - dist - 85.0 + yy, 1.0, 1.0, 0.0, me.image_blend, 1.0);
                g.gfx.draw_surface_ext(self.half_box_b, x + splid - 85.0 + xx2, y + dist - 85.0 + yy2, 1.0, 1.0, 0.0, me.image_blend, 1.0);
            }
        }
        g.gfx.draw_set_color(C_WHITE);
        if self.con > 0.0 {
            if round(self.distance) == 0.0 {
                let wp = spr("spr_whitepixel");
                if vert {
                    g.draw_sprite_ext(wp, 0.0, x + xo, y - 74.0, dist * 2.0, splid * 2.0, self.angle, C_WHITE, 1.0);
                } else {
                    g.draw_sprite_ext(wp, 0.0, x - 74.0, y + yo, splid * 2.0, dist * 2.0, self.angle, C_WHITE, 1.0);
                }
            } else {
                let edge = spr("spr_rk_split_flame_edge");
                let fi = self.flame_index;
                if vert {
                    g.draw_sprite_ext(edge, fi, x - dist - 1.0, y + 2.0, 2.0, 2.0, angle, C_GRAY, 1.0);
                    g.draw_sprite_ext(edge, fi, x + dist + 2.0, y, 2.0, 2.0, angle, C_GRAY, 1.0);
                } else {
                    g.draw_sprite_ext(edge, fi, x + 2.0, y - dist - 1.0, 2.0, 2.0, angle + 185.0, C_GRAY, 1.0);
                    g.draw_sprite_ext(edge, fi, x, y + dist + 2.0, 2.0, 2.0, angle, C_GRAY, 1.0);
                }
            }
        }
        self.flame_index += 0.5;
    }

    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            // Other_10
            0 => {
                self.timer = 0.0;
                self.con += 1.0;
            }
            // Other_11: restore obj_growtangle with a custom box sprite made from the current halves
            1 => {
                self.boxgone = true;
                let Some(gtid) = g.first("obj_growtangle") else { return };
                let custom = g.get::<Growtangle>(gtid).map(|(_, b)| b.custom_box).unwrap_or(false);
                if custom {
                    return;
                }
                let mut sprite = NO_SPR;
                if g.gfx.surface_exists(self.source_surf) {
                    g.gfx.surface_set_target(self.source_surf);
                    g.gfx.draw_set_color(C_BLACK);
                    // ossafe_fill_rectangle(0, 169, 1, 170)
                    g.gfx.draw_rectangle(0.0, 169.0, 1.0, 170.0, false);
                    g.gfx.draw_set_color(C_WHITE);
                    g.gfx.surface_reset_target();
                    // NOTE: GML passes removeback=true (bottom-left pixel colour, forced to black above,
                    // becomes transparent); the runtime's sprite_create_from_surface has no removeback.
                    sprite = g.sprite_create_from_surface_ext(self.source_surf, 0.0, 0.0, 170.0, 170.0, true, 85.0, 85.0);
                }
                if let Some((bi, bo)) = g.get::<Growtangle>(gtid) {
                    bi.visible = true;
                    bi.x = bi.xstart;
                    bi.y = bi.ystart;
                    bo.custom_box = true;
                    bo.spr_custom_box = sprite;
                    bo.growscale = 1.0;
                } else if let Some(bi) = g.inst_mut(gtid) {
                    bi.visible = true;
                    bi.x = bi.xstart;
                    bi.y = bi.ystart;
                }
            }
            // Other_12 / Other_13: older fountain spawners (not called by the current Step)
            2 => self.legacy_split(me, g, false),
            3 => self.legacy_split(me, g, true),
            _ => {}
        }
    }

    fn cleanup(&mut self, me: &mut Inst, g: &mut Game) {
        self.user(1, me, g);
        g.destroy(self.marker[0]);
        g.destroy(self.marker[1]);
        if g.gfx.surface_exists(self.source_surf) {
            g.gfx.surface_free(self.source_surf);
        }
        if g.gfx.surface_exists(self.half_box_a) {
            g.gfx.surface_free(self.half_box_a);
        }
        if g.gfx.surface_exists(self.half_box_b) {
            g.gfx.surface_free(self.half_box_b);
        }
    }

    obj_vars!(con, timer, distance, old_distance, split_delay, split_wait, split_hold, xoffset, yoffset, angle, flame_index);
}

impl SplitGrowtangle {
    /// Step_0, con == 1 && timer >= split_wait + split_delay: the box breaks open and fires the bullets.
    fn split_open(&mut self, me: &mut Inst, g: &mut Game) {
        if self.disable_on_close {
            for b in g.ids_of("obj_roaringknight_fountain_bullet") {
                if let Some(b) = g.inst_mut(b) {
                    b.active = 0.0;
                }
            }
            self.child_bullet = vec![];
            self.count = 0.0;
        }
        g.snd_play_x("snd_knight_boxbreak", 1.0, 1.1);
        self.user(0, me, g);
        let (hx, hy) = heart_xy(g);
        if truthy(self.diagonal) {
            let heartdir = point_direction(me.x + self.xoffset, me.y + self.yoffset, hx + 10.0, hy + 10.0);
            let a = self.angle + if truthy(self.vertical) { 45.0 } else { -45.0 };
            if angle_difference(a, heartdir).abs() < 90.0 {
                self.heart_x = 1.0;
                self.heart_y = if truthy(self.vertical) { -1.0 } else { 1.0 };
            } else {
                self.heart_x = -1.0;
                self.heart_y = if truthy(self.vertical) { 1.0 } else { -1.0 };
            }
        } else {
            self.heart_x = if hx + 10.0 < me.x + self.xoffset { -1.0 } else { 1.0 };
            self.heart_y = if hy + 10.0 < me.y + self.yoffset { -1.0 } else { 1.0 };
        }
        if self.split_delay > 0.0 {
            g.snd_play_pitch("snd_chargeshot_fire", 0.5);
        }
        g.snd_play("snd_chargeshot_fire");
        self.split_delay = 0.0;
        let range = self.bullet_range;
        let mut total = self.bullet_count;
        let mut odd = false;
        if self.bullet_count % 2.0 == 1.0 {
            odd = true;
            total += 1.0;
        }
        let mut flip = g.choose(&[true, false]);
        let trueangle = if truthy(self.vertical) { self.angle + 90.0 } else { self.angle };
        let xrange = lengthdir_x(range, trueangle);
        let yrange = lengthdir_y(range, trueangle);
        let xshift = xrange / (total / 2.0 - 1.0);
        let yshift = yrange / (total / 2.0 - 1.0);
        let mut xstart = me.x - xrange / 2.0;
        let mut ystart = me.y - yrange / 2.0;
        let mut weight: f64 = 0.0;
        let mut direction = 0.0;
        let mut i = 0.0;
        while i < self.bullet_count {
            if !truthy(self.diagonal) && i == total / 2.0 {
                xstart = me.x - xrange / 2.0;
                ystart = me.y - yrange / 2.0;
                if odd {
                    xstart += xshift / 2.0;
                    ystart += yshift / 2.0;
                }
                weight = 0.0;
                flip = !flip;
            }
            if weight == 0.0 {
                weight = g.choose(&[-2.0, -1.0, 1.0, 2.0]);
            }
            let speed = inverselerp(-1.0, 1.0, sign(-weight));
            let (bx, by) = if truthy(self.diagonal) { (me.x, me.y) } else { (xstart, ystart) };
            let b = g.instance_create(bx, by, Box::new(FountainBullet::default()));
            let topspeed = if speed == 1.0 { 4.0 } else { 2.0 };
            let top_speed = topspeed + g.random_range(-0.2, 0.2);
            if truthy(self.diagonal) {
                direction += 360.0 / self.bullet_count;
            } else if truthy(self.vertical) {
                direction = if flip { 180.0 } else { 0.0 };
            } else {
                direction = if flip { 90.0 } else { -90.0 };
            }
            let depth = me.depth + 1.0;
            if let Some((bi, bo)) = g.get::<FountainBullet>(b) {
                bi.friction = if speed == 1.0 { -0.2 } else { -0.05 };
                bo.top_speed = top_speed;
                bi.image_speed = 0.5;
                bi.depth = depth;
                bi.image_xscale = 2.0;
                bi.image_yscale = 2.0;
                bi.active = 0.0;
                bi.set_speed(0.0);
                bi.set_direction(direction);
                bi.image_angle = direction;
            }
            battle::scr_bullet_inherit(me, b, g);
            if let Some(bi) = g.inst_mut(b) {
                bi.grazed = -1.0;
            }
            arr_set(&mut self.child_bullet, self.count, b);
            self.count += 1.0;
            if weight.abs() == 1.0 {
                weight = g.choose(&[1.0, 2.0]) * sign(-weight);
            } else {
                weight = scr_movetowards(weight, 0.0, 1.0);
            }
            xstart += xshift;
            ystart += yshift;
            i += 1.0;
        }
    }

    /// Other_12 ("old called") and Other_13 ("new called"): legacy horizontal splitter.
    /// `only_slow` = Other_13, which only spawns the speed-4 bullets and no extra speed-2 ones.
    fn legacy_split(&mut self, me: &mut Inst, g: &mut Game, only_slow: bool) {
        self.timer += 1.0;
        if self.con == 0.0 {
            if self.timer == 20.0 {
                g.snd_play_x("snd_knight_boxbreak", 1.0, 1.1);
                let mut balance_top = 0.0;
                let mut balance_bottom = 0.0;
                self.user(0, me, g);
                self.user(1, me, g);
                let (_, hy) = heart_xy(g);
                let gty = g.first_inst("obj_growtangle").map(|b| b.y).unwrap_or(0.0);
                self.heart_y = if hy + 10.0 < gty { -1.0 } else { 1.0 };
                g.snd_play("snd_chargeshot_fire");
                let mut xstart = g.first_inst("obj_growtangle").map(|b| b.x).unwrap_or(0.0) - 66.0;
                for i in 0..14 {
                    let balance = if i % 2 == 0 { balance_top } else { balance_bottom };
                    let change = if balance >= 3.0 {
                        -1.0
                    } else if balance <= -3.0 {
                        1.0
                    } else {
                        g.choose(&[1.0, -1.0])
                    };
                    // `_change ? 8 : 4` — GML truthiness: -1 is false, so speed is 4 when change == -1
                    let speed = if truthy(change) { 8.0 } else { 4.0 };
                    if i % 2 == 0 {
                        balance_top += change;
                    } else {
                        balance_bottom += change;
                    }
                    if !only_slow || speed != 8.0 {
                        let ts = speed + g.random_range(-0.1, 0.1);
                        self.legacy_bullet(me, g, xstart, ts, speed / 8.0, i % 2 == 1);
                    }
                    if !only_slow && truthy(change) {
                        let ts = 2.0 + g.random_range(-0.1, 0.1);
                        self.legacy_bullet(me, g, xstart, ts, 0.25, i % 2 == 0);
                    }
                    if i % 2 == 1 {
                        xstart += 22.0;
                    }
                }
            }
        } else if self.con == 1.0 {
            if self.timer == 15.0 {
                let d = g.first_inst("obj_growtangle").map(|b| b.depth).unwrap_or(0.0) - 10.0;
                for i in 0..(self.count.max(0.0) as usize) {
                    let id = self.child_bullet.get(i).copied().unwrap_or(0);
                    if g.id_exists(id) {
                        if let Some(b) = g.inst_mut(id) {
                            b.depth = d;
                        }
                    }
                }
            }
            if self.timer <= 30.0 {
                let old = self.distance;
                let sd = if only_slow { 50.0 } else { self.split_dist };
                self.distance = scr_ease_out(self.timer / 30.0, 6) * sd;
                let dd = (self.distance - old) * self.heart_y * 1.25;
                if let Some(h) = g.first_inst("obj_heart") {
                    h.y += dd;
                }
            } else {
                self.user(0, me, g);
            }
        }
        if self.con >= 1.0 {
            let quarterbox = 37.5;
            let boxy = me.y + (self.distance + quarterbox) * self.heart_y;
            if let Some(h) = g.first_inst("obj_heart") {
                if h.y < boxy - quarterbox + 4.0 {
                    h.y = boxy - quarterbox + 4.0;
                }
                if h.y > boxy + quarterbox - 24.0 {
                    h.y = boxy + quarterbox - 24.0;
                }
            }
        }
        let dist = round(self.distance);
        let y = me.y;
        if let Some(m) = g.inst_mut(self.marker[0]) {
            m.y = y - dist - 1.0;
        }
        if let Some(m) = g.inst_mut(self.marker[1]) {
            m.y = y + dist + 3.0;
        }
    }

    fn legacy_bullet(&mut self, me: &mut Inst, g: &mut Game, x: f64, top_speed: f64, image_speed: f64, down: bool) {
        let b = g.instance_create(x, me.y, Box::new(FountainBullet::default()));
        let depth = me.depth + 1.0;
        if let Some((bi, bo)) = g.get::<FountainBullet>(b) {
            bi.set_speed(0.0);
            bo.top_speed = top_speed;
            bi.image_speed = image_speed;
            bi.depth = depth;
            bi.image_xscale = 1.0;
            bi.image_yscale = 1.0;
            if down {
                bi.set_direction(-90.0);
                bi.image_angle = 180.0;
            } else {
                bi.set_direction(90.0);
            }
        }
        battle::scr_bullet_inherit(me, b, g);
        arr_set(&mut self.child_bullet, self.count, b);
        self.count += 1.0;
    }
}

// ============================================================================ obj_knight_split_growtangle_effect

/// obj_knight_split_growtangle_effect (no parent): the flash of the box being cut.
pub struct SplitGrowtangleEffect {
    pub timer: f64,
    pub surf: i32,
    pub vertical: f64,
    pub angle: f64,
    pub xmul: f64,
    pub ymul: f64,
    pub diagonal: f64,
    pub xoffset: f64,
    pub yoffset: f64,
}
impl Default for SplitGrowtangleEffect {
    fn default() -> Self {
        SplitGrowtangleEffect { timer: 0.0, surf: -4, vertical: 0.0, angle: 0.0, xmul: 1.0, ymul: 1.0, diagonal: 0.0, xoffset: 0.0, yoffset: 0.0 }
    }
}

impl Object for SplitGrowtangleEffect {
    fn name(&self) -> &'static str { "obj_knight_split_growtangle_effect" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer = 0.0;
        if let Some(gt) = gt_info(g) {
            me.image_blend = gt.image_blend;
            me.image_xscale = gt.image_xscale;
            me.image_yscale = gt.image_yscale;
        }
        self.surf = -4;
        self.vertical = 0.0;
        self.angle = 0.0;
        self.xmul = 1.0;
        self.ymul = 1.0;
        self.diagonal = 0.0;
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if !g.gfx.surface_exists(self.surf) {
            self.surf = g.gfx.surface_create(640, 480);
            let app = g.gfx.app_surface;
            g.gfx.surface_copy(self.surf, 0.0, 0.0, app);
            self.xmul = lengthdir_x(1.0, self.angle);
            self.ymul = lengthdir_y(1.0, self.angle);
        }
        self.timer += 1.0;
        let vert = truthy(self.vertical);
        let fade = (10.0 - self.timer) / 10.0;
        let htimer = (if vert { 0.0 } else { self.timer }) * self.xmul;
        let vtimer = (if vert { self.timer } else { 0.0 }) * self.ymul;
        let sw = g.sprite_width(me);
        let sh = g.sprite_height(me);
        let mut splitwidth = sw;
        let mut splitheight = sh;
        let mut splitleft = 0.0;
        let mut splittop = 0.0;
        if vert {
            splitleft = sw / 2.0;
            splitwidth /= 2.0;
        } else {
            splittop = sh / 2.0;
            splitheight /= 2.0;
        }
        let (s, x, y, xs, ys, col) = (me.sprite_index, me.x, me.y, me.image_xscale, me.image_yscale, me.image_blend);
        for (k, a) in [(8.0, clamp01(fade)), (6.0, clamp01(fade)), (4.0, fade)] {
            draw_sprite_part_ext_rot(g, s, 0.0, 0.0, 0.0, splitwidth, splitheight, x - htimer * k, y - vtimer * k, xs, ys, 0.0, col, a);
        }
        for (k, a) in [(8.0, clamp01(fade)), (6.0, clamp01(fade)), (4.0, fade)] {
            draw_sprite_part_ext_rot(g, s, 0.0, splitleft, splittop, splitwidth, splitheight, x + htimer * k, y + vtimer * k, xs, ys, 0.0, col, a);
        }
        let sx = x - g.camerax();
        let sy = y - g.cameray();
        let (cx, cy) = (g.camerax(), g.cameray());
        if vert {
            g.gfx.draw_surface_part_ext(self.surf, 0.0, 0.0, sx, 480.0, cx, cy - self.timer * 8.0, 1.0, 1.0, C_WHITE, fade / 2.0);
            g.gfx.draw_surface_part_ext(self.surf, sx, 0.0, 640.0 - sx, 480.0, x, cy + self.timer * 8.0, 1.0, 1.0, C_WHITE, fade / 2.0);
        } else {
            g.gfx.draw_surface_part_ext(self.surf, 0.0, 0.0, 640.0, sy, cx - self.timer * 8.0, cy, 1.0, 1.0, C_WHITE, fade / 2.0);
            g.gfx.draw_surface_part_ext(self.surf, 0.0, sy, 640.0, 480.0 - sy, cx + self.timer * 8.0, y, 1.0, 1.0, C_WHITE, fade / 2.0);
        }
        g.gfx.draw_set_color(C_WHITE);
        let mut angle = self.angle;
        if vert {
            angle += 90.0;
        }
        if truthy(self.diagonal) {
            angle += 45.0;
        }
        let px = spr("spr_pxwhite10_center");
        g.draw_sprite_ext(px, 0.0, x + self.xoffset, y + self.yoffset, 50.0, fade, angle, C_WHITE, 1.0);
        g.draw_sprite_ext(px, 0.0, x + self.xoffset, y + self.yoffset, 50.0, fade * 1.4, angle, C_WHITE, 0.5);
        if self.timer == 10.0 {
            g.destroy_self(me);
        }
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.gfx.surface_exists(self.surf) {
            g.gfx.surface_free(self.surf);
        }
    }

    obj_vars!(timer, angle, xoffset, yoffset);
}

// ============================================================================ obj_roaringknight_fountain_bullet

/// obj_roaringknight_fountain_bullet (parent obj_regularbullet).
#[derive(Default)]
pub struct FountainBullet {
    pub rb: RegVars,
    pub speed_mult: f64,
    pub top_speed: f64,
    /// (sic) the game sets `destroy_on_hit`, not `destroyonhit`, so these bullets still die on hit
    pub destroy_on_hit: bool,
}

impl Object for FountainBullet {
    fn name(&self) -> &'static str { "obj_roaringknight_fountain_bullet" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::regularbullet_create(me, &mut self.rb, g);
        me.element = 5.0;
        self.speed_mult = 0.0;
        self.top_speed = 0.0;
        me.image_xscale = 1.0;
        me.image_yscale = 1.0;
        me.active = 0.0;
        self.destroy_on_hit = false;
        me.grazepoints = 5.0;
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        battle::regularbullet_step(me, &mut self.rb, g);
        if self.speed_mult < 1.0 {
            self.speed_mult += 0.2;
            if me.active == 0.0 && self.speed_mult >= 0.1 {
                me.active = 1.0;
            }
            me.set_speed(self.speed_mult * self.top_speed);
        }
    }

    obj_vars!(speed_mult, top_speed);
}

// ============================================================================ obj_roaringknight_slash

/// obj_roaringknight_slash (parent obj_collidebullet): a screen-wide AOE slash line.
/// Not used by Flurry (created by the rotating-slash / roaring / tunnel attacks).
#[derive(Default)]
pub struct RkSlash {
    pub width: f64,
    pub aoe: bool,
    pub slashdir: f64,
}

impl Object for RkSlash {
    fn name(&self) -> &'static str { "obj_roaringknight_slash" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::bullet_init(me);
        // event_inherited(): obj_collidebullet has no Create
        me.active = 1.0;
        me.element = 5.0;
        self.width = 24.0;
        me.grazepoints = 50.0;
        self.aoe = true;
        me.alarm[0] = 1;
        me.alarm[1] = 3;
        me.image_index = 2.0;
        me.image_speed = 0.0;
        me.image_yscale = 0.1;
        self.slashdir = g.choose(&[-1.0, 1.0]);
        me.destroyonhit = 0.0;
    }

    fn alarm(&mut self, n: usize, me: &mut Inst, _g: &mut Game) {
        // Alarm_0 is just `exit` (it exists so alarm[0] counts down)
        if n == 1 {
            me.mask_index = spr("spr_nomask");
        }
    }

    fn end_step(&mut self, me: &mut Inst, g: &mut Game) {
        me.damage = 206.0;
        me.grazepoints = 50.0;
        // `!alarm[0]`: GML truthiness (alarm rests at -1 after firing)
        if !truthy(me.alarm[0] as f64) {
            self.width *= 0.66;
            me.image_alpha *= 0.66;
        }
        if self.width < 12.0 {
            me.active = 0.0;
        }
        if self.width < 0.5 {
            g.destroy_self(me);
        }
        if self.width > 4.0 {
            for b in g.ids_of("obj_growtangle") {
                let dx = g.choose(&[-2.0, -1.0, 0.0, 1.0, 2.0]);
                let dy = g.choose(&[-2.0, -1.0, 0.0, 1.0, 2.0]);
                if let Some(b) = g.inst_mut(b) {
                    b.x = b.xstart + dx;
                    b.y = b.ystart + dy;
                }
            }
            scr_heartclamp(g, 0.0, 0.0);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let dir = me.direction();
        let hx = lengthdir_x(640.0, dir);
        let hy = lengthdir_y(640.0, dir);
        let hxoff = lengthdir_x(self.width, dir + 90.0);
        let hyoff = lengthdir_y(self.width, dir + 90.0);
        let a = me.image_alpha;
        let color = crate::gfx::make_color_rgb(255.0, (1.0 - a) * 255.0, (1.0 - a) * 255.0);
        let (x, y) = (me.x, me.y);
        g.gfx.draw_set_alpha(a * 2.0);
        // `if (slashdir)`: GML truthiness, so -1 takes the else branch
        if truthy(self.slashdir) {
            g.gfx.draw_triangle_color(x - hx * a, y - hy * a, x + hx + hxoff, y + hy + hyoff, x + hx - hxoff, y + hy - hyoff, color, color, color, false);
        } else {
            g.gfx.draw_triangle_color(x + hx * a, y + hy * a, x - hx + hxoff, y - hy + hyoff, x - hx - hxoff, y - hy - hyoff, color, color, color, false);
        }
        g.gfx.draw_set_alpha(1.0);
    }

    /// Other_15
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 5 {
            return;
        }
        me.damage = 206.0;
        if self.aoe {
            me.damage = 75.0;
            me.target = 3.0;
            for k in g.ids_of("obj_knight_enemy") {
                if let Some((_, ko)) = g.get::<KnightEnemy>(k) {
                    ko.aoedamage = true;
                }
            }
        }
        if me.active == 1.0 {
            if me.target != 3.0 {
                battle::scr_damage(me, g);
            }
            if me.target == 3.0 {
                battle::scr_damage_all(me, g);
            }
            if me.destroyonhit == 1.0 {
                g.destroy_self(me);
            }
        }
        for k in g.ids_of("obj_knight_enemy") {
            if let Some((_, ko)) = g.get::<KnightEnemy>(k) {
                ko.aoedamage = false;
            }
        }
    }

    obj_vars!(width, slashdir);
}

// ============================================================================ obj_fake_gt

/// obj_fake_gt (parent obj_bulletparent): draws obj_growtangle at an offset while hiding the real one.
/// Not used by Flurry.
#[derive(Default)]
pub struct FakeGt {
    pub xoffset: f64,
    pub yoffset: f64,
}

impl Object for FakeGt {
    fn name(&self) -> &'static str { "obj_fake_gt" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.xoffset = 0.0;
        self.yoffset = 0.0;
        if let Some(b) = g.first_inst("obj_growtangle") {
            b.visible = false;
            me.depth = b.depth;
        }
    }

    fn end_step(&mut self, me: &mut Inst, g: &mut Game) {
        if let Some(b) = g.first_inst("obj_growtangle") {
            me.x = b.x;
            me.y = b.y;
            me.image_xscale = b.image_xscale;
            me.image_yscale = b.image_yscale;
        }
    }

    fn draw(&mut self, _me: &mut Inst, g: &mut Game) {
        let (xo, yo) = (self.xoffset, self.yoffset);
        for id in g.ids_of("obj_growtangle") {
            let Some(b) = g.inst(id) else { continue };
            let (s, ii, x, y, xs, ys, ang, col, a) =
                (b.sprite_index, b.image_index, b.x, b.y, b.image_xscale, b.image_yscale, b.image_angle, b.image_blend, b.image_alpha);
            let gtv = g.get::<Growtangle>(id).map(|(_, o)| (o.custom_box, o.growth, o.growcon, o.maxxscale, o.maxyscale, o.spr_custom_box));
            g.draw_sprite_ext(s, 1.0, x + xo, y + yo, xs, ys, ang, col, a);
            match gtv {
                Some((true, growth, growcon, mx, my, sc)) if truthy(growth) && growcon != 2.0 => {
                    g.draw_sprite_ext(sc, 0.0, x + xo, y + yo, xs / (mx / 2.0), ys / (my / 2.0), ang, col, a);
                }
                _ => g.draw_sprite_ext(s, ii, x + xo, y + yo, xs, ys, ang, col, a),
            }
        }
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if let Some(b) = g.first_inst("obj_growtangle") {
            b.visible = true;
        }
    }

    obj_vars!(xoffset, yoffset);
}

// ============================================================================ obj_knight_warp

/// obj_knight_warp (no parent): the Knight's teleport flash. Not used by Flurry.
pub struct KnightWarp {
    pub master: Id,
    pub master_xoffset: f64,
    pub master_yoffset: f64,
}
impl Default for KnightWarp {
    fn default() -> Self { KnightWarp { master: NOONE, master_xoffset: 0.0, master_yoffset: 0.0 } }
}

impl Object for KnightWarp {
    fn name(&self) -> &'static str { "obj_knight_warp" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.master = NOONE;
        self.master_xoffset = 0.0;
        self.master_yoffset = 0.0;
        // scr_darksize()
        me.image_xscale = 2.0;
        me.image_yscale = 2.0;
        me.image_speed = 0.0;
        g.snd_play("snd_knight_teleport");
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.master != NOONE {
            if let Some(m) = g.inst(self.master) {
                me.x = m.x + self.master_xoffset;
                me.y = m.y + self.master_yoffset;
            }
        }
    }

    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            0 => {
                if self.master != NOONE {
                    if let Some(m) = g.inst_mut(self.master) {
                        m.image_alpha = 1.0;
                    }
                }
                g.destroy_self(me);
            }
            1 => g.destroy_self(me),
            _ => {}
        }
    }

    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            0 => {
                if self.master != NOONE {
                    if let Some(m) = g.inst_mut(self.master) {
                        m.image_alpha = 0.0;
                    }
                }
                me.image_index = 6.0;
                g.lerpvar(me, "image_index", 5.0, 8.0, 4.0, 0, "out");
                me.alarm[0] = 4;
            }
            1 => {
                if self.master != NOONE {
                    if let Some(m) = g.inst_mut(self.master) {
                        m.image_alpha = 0.0;
                    }
                }
                me.image_index = 8.0;
                g.lerpvar(me, "image_index", 8.0, 5.0, 4.0, 0, "out");
                me.alarm[1] = 4;
            }
            _ => {}
        }
    }

    obj_vars!(master_xoffset, master_yoffset);
}
