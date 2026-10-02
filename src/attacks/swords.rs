//! Tracking swords (obj_dbulletcontroller type 151; knight myattackchoice 11/14/15/16/17) and the
//! sword vortex (type 154; myattackchoice 15).
//!
//! Objects: obj_tracking_swords_manager, obj_tracking_sword1, obj_tracking_sword2,
//! obj_tracking_sword_slash, obj_tracking_sword_slash_extra_graze, obj_sword_vortex_manager,
//! obj_sword_vortex, obj_afterimage_grow (scr_afterimage_grow).
//! Scripts: scr_afterimage_grow, gt_minx, gt_miny, scr_armorcheck_equipped_party.

use crate::battle::{self, DBulletController, DbCtrl};
use crate::gfx::{bm, merge_color, C_RED, C_WHITE};
use crate::gm::*;
use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object, NOONE};

// ============================================================================ controller

/// obj_dbulletcontroller Step_0: `if (type == 151)` and `if (type == 154)` blocks.
pub fn ctrl_step(c: &mut DbCtrl, me: &mut Inst, g: &mut Game) {
    if c.typ == 151 && c.made == 0.0 {
        let gx = g.first_inst("obj_growtangle").map(|b| b.x).unwrap_or(0.0);
        let cy = g.cameray();
        let manager = g.instance_create(gx, cy, Box::new(TrackingSwordsManager::default()));
        battle::scr_bullet_inherit(me, manager, g);
        if let Some((i, o)) = g.get::<TrackingSwordsManager>(manager) {
            o.variant = c.difficulty;
            i.damage = me.damage;
        }
        g.user_event(manager, 0);
        c.made = 1.0;
    }
    if c.typ == 154 && c.made == 0.0 {
        let gx = g.first_inst("obj_growtangle").map(|b| b.x).unwrap_or(0.0);
        let cy = g.cameray();
        let manager = g.instance_create(gx, cy, Box::new(SwordVortexManager::default()));
        battle::scr_bullet_inherit(me, manager, g);
        if let Some((i, o)) = g.get::<SwordVortexManager>(manager) {
            o.variant = c.difficulty;
            i.damage = me.damage;
        }
        c.made = 1.0;
    }
}

// ============================================================================ helper scripts

/// gt_minx()
fn gt_minx(g: &mut Game) -> f64 {
    let Some(id) = g.first("obj_growtangle") else { return 0.0 };
    let Some(b) = g.inst(id) else { return 0.0 };
    b.x - g.sprite_width(b) / 2.0
}

/// gt_miny()
fn gt_miny(g: &mut Game) -> f64 {
    let Some(id) = g.first("obj_growtangle") else { return 0.0 };
    let Some(b) = g.inst(id) else { return 0.0 };
    b.y - g.sprite_height(b) / 2.0
}

/// scr_armorcheck_equipped_party(armor): how many party members wear `armor`.
// TODO(battle): Glob has no global.chararmor1/chararmor2; with no armor data this counts 0
// (i.e. the graze factors stay 1, as with no graze-boosting armor equipped).
fn scr_armorcheck_equipped_party(g: &Game, armor: f64) -> f64 {
    let mut total = 0.0;
    for wi in 0..3 {
        let c = g.glob.char[wi];
        if c != 0 {
            // scr_armorcheck_equipped(char, armor): count of matching armor slots
            total += (g.glob.chararmor1[c] == armor) as i32 as f64 + (g.glob.chararmor2[c] == armor) as i32 as f64;
        }
    }
    total
}

/// scr_afterimage_grow(): returns the obj_afterimage_grow instance.
fn scr_afterimage_grow(me: &Inst, g: &mut Game) -> Id {
    let a = g.instance_create(me.x, me.y, Box::new(AfterimageGrow::default()));
    if let Some(i) = g.inst_mut(a) {
        i.sprite_index = me.sprite_index;
        i.image_index = me.image_index;
        i.image_blend = me.image_blend;
        i.image_speed = 0.0;
        i.depth = me.depth;
        i.image_xscale = me.image_xscale;
        i.image_yscale = me.image_yscale;
        i.image_angle = me.image_angle;
    }
    a
}

/// obj_afterimage_grow
pub struct AfterimageGrow {
    pub xrate: f64,
    pub yrate: f64,
    pub fade: f64,
    pub target: Id,
    pub destroytime: f64,
}
impl Default for AfterimageGrow {
    fn default() -> Self { AfterimageGrow { xrate: 0.2, yrate: 0.2, fade: 0.1, target: NOONE, destroytime: -1.0 } }
}
impl Object for AfterimageGrow {
    fn name(&self) -> &'static str { "obj_afterimage_grow" }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.target != NOONE && g.id_exists(self.target) {
            if let Some(t) = g.inst(self.target) {
                me.x = t.x;
                me.y = t.y;
            }
        }
        me.image_alpha -= self.fade;
        me.image_xscale += self.xrate;
        me.image_yscale += self.yrate;
        if me.image_alpha < 0.0 {
            g.destroy_self(me);
        }
        if self.destroytime > -1.0 {
            self.destroytime -= 1.0;
        }
        if self.destroytime == 0.0 {
            g.destroy_self(me);
        }
    }
    obj_vars!(xrate, yrate, fade, destroytime);
}

// ============================================================================ tracking swords

/// obj_tracking_swords_manager (parent obj_regularbullet; no event_inherited)
pub struct TrackingSwordsManager {
    pub timer: f64,
    pub con: f64,
    pub variant: f64,
    pub firstsword: bool,
    pub multiswordmax: f64,
    pub multiswordframes: f64,
    pub multiswordcon: f64,
    pub multiswordcount: f64,
    pub setcount: f64,
    pub setdirection: Vec<f64>,
    pub creatorid: f64,
    pub creator: f64,
    pub hell_surface: i32,
    pub swordcount: f64,
    pub directionprev: Vec<f64>,
    pub rate: f64,
    pub ratedecay: f64,
    pub rateminimum: f64,
    pub maxswords: f64,
    pub inst: Id,
}
impl Default for TrackingSwordsManager {
    fn default() -> Self {
        TrackingSwordsManager {
            timer: 0.0,
            con: 0.0,
            variant: 0.0,
            firstsword: false,
            multiswordmax: 0.0,
            multiswordframes: 0.0,
            multiswordcon: 0.0,
            multiswordcount: 0.0,
            setcount: 0.0,
            setdirection: vec![-1.0; 50],
            creatorid: -1.0,
            creator: -1.0,
            hell_surface: -4,
            swordcount: 0.0,
            directionprev: vec![-1.0; 8],
            rate: 0.0,
            ratedecay: 0.0,
            rateminimum: 0.0,
            maxswords: 0.0,
            inst: NOONE,
        }
    }
}
impl TrackingSwordsManager {
    fn set_dirprev(&mut self, idx: f64, v: f64) {
        let i = idx as usize;
        if i >= self.directionprev.len() {
            self.directionprev.resize(i + 1, 0.0);
        }
        self.directionprev[i] = v;
    }
    /// Values written by `with (obj_tracking_swords_manager)` inside Other_10.
    fn apply_override(&mut self, typ: i32) {
        if typ == 104 {
            self.rate = 55.0;
            self.ratedecay = 0.0;
            self.rateminimum = 24.0;
            self.maxswords = 99.0;
            self.multiswordmax = 0.0;
            self.multiswordframes = 0.0;
        }
        if typ == 154 {
            self.rate = 24.0;
            self.ratedecay = 4.0;
            self.rateminimum = 16.0;
            self.maxswords = 99.0;
            self.multiswordmax = 0.0;
        }
    }
}
impl Object for TrackingSwordsManager {
    fn name(&self) -> &'static str { "obj_tracking_swords_manager" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        // fields initialised in Default (timer..directionprev)
        battle::bullet_init(me);
        self.user(0, me, g);
    }
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 5 {
            battle::collidebullet_hit(me, g);
            return;
        }
        if n != 0 {
            return;
        }
        if self.variant == 0.0 {
            self.rate = 32.0;
            self.ratedecay = 4.0;
            self.rateminimum = 16.0;
            self.maxswords = 99.0;
            self.multiswordmax = 0.0;
        }
        if self.variant == 1.0 {
            self.rate = 50.0;
            self.ratedecay = 10.0;
            self.rateminimum = 6.0;
            self.maxswords = 5.0;
            self.multiswordmax = 0.0;
            self.rate = 24.0;
            self.ratedecay = 0.0;
            self.rateminimum = 24.0;
            self.maxswords = 99.0;
            self.multiswordmax = 0.0;
        }
        if self.variant == 2.0 {
            self.rate = 24.0;
            self.ratedecay = 0.0;
            self.rateminimum = 24.0;
            self.maxswords = 99.0;
            self.multiswordmax = 2.0;
            self.multiswordframes = 4.0;
            let dirs = [0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0, 0.0, 45.0];
            for (k, d) in dirs.iter().enumerate() {
                self.setdirection[k + 1] = *d;
            }
        }
        if self.variant == 3.0 {
            self.rate = 20.0;
            self.ratedecay = 4.0;
            self.rateminimum = 13.0;
            self.maxswords = 99.0;
            self.multiswordmax = 0.0;
        }
        // with (obj_dbulletcontroller) { if (type == 104 / 154) with (obj_tracking_swords_manager) ... }
        // (the controller currently running us is checked out and is type 151 anyway)
        for cid in g.ids_of("obj_dbulletcontroller") {
            let Some(typ) = g.get::<DBulletController>(cid).map(|(_, o)| o.c.typ) else { continue };
            if typ != 104 && typ != 154 {
                continue;
            }
            for mid in g.ids_of("obj_tracking_swords_manager") {
                if mid == me.id {
                    self.apply_override(typ);
                } else if let Some((_, o)) = g.get::<TrackingSwordsManager>(mid) {
                    o.apply_override(typ);
                }
            }
        }
        self.timer = self.rate - 5.0;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if g.glob.turntimer < 70.0 {
            return;
        }
        self.timer += 1.0;
        if (self.timer == self.rate && self.swordcount <= self.maxswords)
            || (self.timer == self.multiswordframes && self.multiswordcon == 1.0)
        {
            let inst = g.instance_create(me.x, me.y, Box::new(TrackingSword1::default()));
            self.inst = inst;
            let d0 = g.choose(&[0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0]);
            let variant = self.variant;
            let damage = me.damage;
            if let Some((i, o)) = g.get::<TrackingSword1>(inst) {
                i.set_direction(d0);
                o.variant = variant;
                i.damage = damage;
                for _ in 0..8 {
                    for k in 0..8 {
                        if i.direction() == self.directionprev[k] {
                            let d = i.direction() + 45.0;
                            i.set_direction(d);
                        }
                    }
                }
                i.image_angle = i.direction() + 180.0;
            }
            let idir = g.inst(inst).map(|i| i.direction()).unwrap_or(0.0);
            self.set_dirprev(self.swordcount, idir);
            for k in 1..4 {
                let mut a = k as f64 + self.swordcount;
                if a > 7.0 {
                    a -= 7.0;
                }
                self.set_dirprev(a, -1.0);
            }
            self.swordcount += 1.0;
            if self.swordcount > self.maxswords && self.variant == 0.0 {
                g.glob.turntimer = 70.0;
            }
            if self.swordcount > self.maxswords && self.variant == 1.0 {
                g.glob.turntimer = 120.0;
            }
            if self.swordcount > 7.0 && self.swordcount < self.maxswords {
                self.swordcount = 0.0;
            }
            self.setcount += 1.0;
            // (GML would error past index 49; treat as unset)
            let sd = self.setdirection.get(self.setcount as usize).copied().unwrap_or(-1.0);
            if sd != -1.0 {
                if let Some(i) = g.inst_mut(inst) {
                    i.set_direction(sd);
                }
            }
            if self.multiswordmax > 0.0 {
                self.multiswordcount += 1.0;
            }
            if self.multiswordcon == 0.0 && self.multiswordmax > 0.0 {
                self.multiswordcon = 1.0;
            }
            if self.multiswordcon == 1.0 && self.multiswordcount == self.multiswordmax {
                self.multiswordcon = 0.0;
                self.multiswordcount = 0.0;
            }
            let (hx, hy) = g.first_inst("obj_heart").map(|h| (h.x, h.y)).unwrap_or((0.0, 0.0));
            if let Some((i, o)) = g.get::<TrackingSword1>(inst) {
                i.x = hx + 10.0 + lengthdir_x(o.len, i.direction());
                i.y = hy + 10.0 + lengthdir_y(o.len, i.direction());
                i.ystart = i.y;
                i.image_angle = i.direction() + 180.0;
            }
            self.rate -= self.ratedecay;
            if self.rate < self.rateminimum {
                self.rate = self.rateminimum;
            }
            self.timer = 0.0;
        }
    }
    fn draw(&mut self, _me: &mut Inst, g: &mut Game) {
        if !g.gfx.surface_exists(self.hell_surface) {
            self.hell_surface = g.gfx.surface_create(150, 150);
        }
        let (mx, my) = (gt_minx(g), gt_miny(g));
        let slashes: Vec<_> = g
            .ids_of("obj_tracking_sword_slash")
            .into_iter()
            .filter_map(|id| g.inst(id))
            .map(|s| (s.sprite_index, s.image_index, s.x, s.y, s.image_xscale, s.image_yscale, s.image_angle, s.image_alpha))
            .collect();
        g.gfx.surface_set_target(self.hell_surface);
        g.gfx.draw_clear_alpha(0, 0.0);
        g.gfx.draw_set_blend_mode(bm::ADD);
        for (s, ii, x, y, xs, ys, ang, a) in slashes {
            g.draw_sprite_ext(s, ii, x - mx, y - my, xs, ys, ang, C_WHITE, a);
        }
        g.gfx.draw_set_blend_mode(bm::NORMAL);
        g.gfx.surface_reset_target();
        g.gfx.draw_surface(self.hell_surface, mx, my);
    }
    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.gfx.surface_exists(self.hell_surface) {
            g.gfx.surface_free(self.hell_surface);
        }
    }
    obj_vars!(
        timer, con, variant, multiswordmax, multiswordframes, multiswordcon, multiswordcount, setcount, creatorid, creator,
        swordcount, rate, ratedecay, rateminimum, maxswords
    );
}

/// obj_tracking_sword1 (parent obj_regularbullet; no event_inherited)
pub struct TrackingSword1 {
    pub timer: f64,
    pub con: f64,
    pub afterimagecon: f64,
    pub targetx: f64,
    pub targety: f64,
    pub variant: f64,
    pub fadetohalftime: f64,
    pub waittime: f64,
    pub fadetofulltime: f64,
    pub flashtime: f64,
    pub len: f64,
    pub lenstart: f64,
    pub afterimage: Id,
    pub slash: Id,
    pub slash2: Id,
}
impl Default for TrackingSword1 {
    fn default() -> Self {
        TrackingSword1 {
            timer: 0.0,
            con: 0.0,
            afterimagecon: 0.0,
            targetx: 0.0,
            targety: 0.0,
            variant: 0.0,
            fadetohalftime: 5.0,
            waittime: 10.0,
            fadetofulltime: 20.0,
            flashtime: 4.0,
            len: 120.0,
            lenstart: 120.0,
            afterimage: NOONE,
            slash: NOONE,
            slash2: NOONE,
        }
    }
}
impl Object for TrackingSword1 {
    fn name(&self) -> &'static str { "obj_tracking_sword1" }
    fn create(&mut self, me: &mut Inst, _g: &mut Game) {
        self.timer = 0.0;
        self.con = 0.0;
        self.afterimagecon = 0.0;
        self.targetx = 0.0;
        self.targety = 0.0;
        self.variant = 0.0;
        me.image_alpha = 0.0;
        battle::bullet_init(me);
        me.element = 5.0;
        self.fadetohalftime = 5.0;
        self.waittime = 10.0;
        self.fadetofulltime = 20.0;
        self.flashtime = 4.0;
        self.len = 120.0;
        self.lenstart = self.len;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.con < 2.0 {
            let (hx, hy) = g.first_inst("obj_heart").map(|h| (h.x, h.y)).unwrap_or((0.0, 0.0));
            let cy = g.cameray();
            me.x = hx + 10.0 + lengthdir_x(self.len, me.direction());
            me.y = clamp(hy + 10.0 + lengthdir_y(self.len, me.direction()), cy + 40.0, cy + 320.0);
        }
        if self.con == 0.0 {
            self.timer += 1.0;
            if self.timer == 1.0 {
                g.snd_play_x("snd_knight_jump_quick", 1.0, 1.3);
            }
            me.image_alpha = lerp(0.0, 0.5, self.timer / self.fadetohalftime);
            if me.image_alpha == 0.5 {
                self.con = 1.0;
                self.timer = 0.0;
            }
        }
        if self.con == 1.0 {
            self.timer += 1.0;
            if self.timer >= self.waittime {
                me.image_alpha = lerp(0.8, 1.0, (self.timer - self.waittime) / self.fadetofulltime);
                self.len = lerp(self.len, self.lenstart + 10.0, (self.timer - self.waittime) / self.fadetofulltime);
                me.image_blend = merge_color(C_WHITE, C_RED, self.timer / 30.0);
            }
            if me.image_alpha == 1.0 {
                self.con = 2.0;
                self.timer = 0.0;
                self.afterimage = scr_afterimage_grow(me, g);
                if let Some((_, a)) = g.get::<AfterimageGrow>(self.afterimage) {
                    a.xrate = 0.2;
                    a.yrate = 0.2;
                    a.fade = 0.3;
                }
            }
        }
        if self.con == 2.0 {
            self.timer += 1.0;
            if self.timer == self.flashtime + 1.0 {
                self.con = 3.0;
                self.timer = 0.0;
            }
        }
        if self.con == 3.0 {
            self.timer += 1.0;
            if self.timer == 1.0 {
                self.afterimagecon = 1.0;
                self.targetx = me.x + lengthdir_x(900.0, me.direction() + 180.0);
                self.targety = me.y + lengthdir_y(900.0, me.direction() + 180.0);
                g.snd_play_x("snd_knight_cut2", 1.0, 1.3);
            }
            if self.timer == 2.0 {
                self.slash = g.instance_create(me.x, me.y, Box::new(TrackingSwordSlash::default()));
                if let Some(s) = g.inst_mut(self.slash) {
                    s.image_angle = me.image_angle;
                    s.set_direction(me.direction());
                    s.damage = me.damage;
                }
                self.slash2 = g.instance_create(me.x, me.y, Box::new(TrackingSwordSlashExtraGraze::default()));
                if let Some(s) = g.inst_mut(self.slash2) {
                    s.image_angle = me.image_angle;
                    s.set_direction(me.direction());
                }
                let a = 200.0;
                let b = 27.0;
                if self.variant == 1.0 {
                    let (gx, gy) = g.first_inst("obj_growtangle").map(|t| (t.x, t.y)).unwrap_or((0.0, 0.0));
                    let mut i = 0.0;
                    while i < b {
                        let c = lerp(1.0, 5.0, i / b);
                        let lx = lerp(me.x, self.targetx, i / b);
                        let ly = lerp(me.y, self.targety, i / b);
                        if lx < gx + a && lx > gx - a && ly > gy - a && ly < gy + a {
                            let ix = lerp(me.x, self.targetx, (i / b) * c);
                            let iy = lerp(me.y, self.targety, (i / b) * c);
                            let inst = g.instance_create(ix, iy, Box::new(TrackingSword2::default()));
                            if let Some(s) = g.inst_mut(inst) {
                                s.damage = me.damage;
                                s.target = me.target;
                            }
                        }
                        i += 1.0;
                    }
                }
            }
            if self.timer == 5.0 {
                g.destroy_self(me);
            }
        }
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.afterimagecon == 0.0 {
            g.draw_self(me);
            if self.con == 2.0 {
                g.gfx.d3d_set_fog(true, C_WHITE);
                g.draw_self(me);
                g.gfx.d3d_set_fog(false, C_WHITE);
            }
        }
        if self.afterimagecon == 1.0 || self.afterimagecon == 2.0 {
            let mult = if self.afterimagecon == 1.0 { 1.0 } else { 0.5 };
            g.draw_sprite_ext(me.sprite_index, me.image_index, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, 0.0025);
            for i in 0..40 {
                let t = i as f64 / 40.0;
                g.draw_sprite_ext(
                    me.sprite_index,
                    me.image_index,
                    lerp(me.x, self.targetx, t),
                    lerp(me.y, self.targety, t),
                    me.image_xscale,
                    me.image_yscale,
                    me.image_angle,
                    me.image_blend,
                    (0.2 + i as f64 / 80.0) * mult,
                );
            }
            self.afterimagecon += 1.0;
        }
    }
    obj_vars!(timer, con, afterimagecon, targetx, targety, variant, fadetohalftime, waittime, fadetofulltime, flashtime, len, lenstart);
}

/// obj_tracking_sword2 (parent obj_regularbullet; no event_inherited; default Draw)
#[derive(Default)]
pub struct TrackingSword2 {
    pub timer: f64,
    pub con: f64,
    pub variant: f64,
}
impl Object for TrackingSword2 {
    fn name(&self) -> &'static str { "obj_tracking_sword2" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer = 0.0;
        self.con = 0.0;
        self.variant = 0.0;
        battle::bullet_init(me);
        me.damage = 1.0;
        me.destroyonhit = 0.0;
        me.grazepoints = 1.0;
        me.active = 0.0;
        me.image_speed = 0.0;
        me.image_index = 1.0;
        me.image_angle = g.irandom(360.0);
        me.set_direction(me.image_angle);
        me.gravity_direction = me.image_angle;
        me.image_xscale = 1.0;
        me.image_yscale = 1.0;
        me.image_alpha = 0.5;
        me.sprite_index = crate::assets::spr("spr_ball_small_pixel_hitbox");
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer += 1.0;
        if self.timer == 30.0 {
            me.gravity = 0.01;
            me.active = 1.0;
            me.image_alpha = 1.0;
        }
        let a = 200.0;
        let (gx, gy) = g.first_inst("obj_growtangle").map(|t| (t.x, t.y)).unwrap_or((0.0, 0.0));
        if me.x < gx - a || me.x > gx + a || me.y < gy - a || me.y > gy + a {
            me.image_alpha -= 0.1;
            if me.image_alpha < 0.1 {
                g.destroy_self(me);
            }
        }
    }
    obj_vars!(timer, con, variant);
}

/// obj_tracking_sword_slash (parent obj_regularbullet). Step = `exit`; Draw only counts down —
/// its pixels are drawn by obj_tracking_swords_manager onto hell_surface.
#[derive(Default)]
pub struct TrackingSwordSlash {
    pub timer: f64,
    pub con: f64,
}
impl Object for TrackingSwordSlash {
    fn name(&self) -> &'static str { "obj_tracking_sword_slash" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer = 0.0;
        self.con = 0.0;
        me.image_xscale = 900.0;
        me.image_yscale = 1.0;
        battle::bullet_init(me);
        me.active = 1.0;
        me.destroyonhit = 0.0;
        me.damage = 1.0;
        me.grazepoints = 4.0;
        if g.exists("obj_sword_vortex_manager") {
            me.grazepoints = 2.0;
        }
        if g.get_first::<TrackingSwordsManager>("obj_tracking_swords_manager").map(|(_, o)| o.variant == 1.0).unwrap_or(false) {
            me.grazepoints = 2.0;
        }
        me.timepoints = 11.0;
    }
    fn step(&mut self, _me: &mut Inst, _g: &mut Game) {}
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer += 1.0;
        if self.timer == 3.0 {
            g.destroy_self(me);
        }
    }
    obj_vars!(timer, con);
}

/// obj_tracking_sword_slash_extra_graze (no parent). Invisible, so its Draw (which would destroy it
/// after 3 frames) never runs: it only goes away when the SOUL touches it while not invulnerable.
pub struct TrackingSwordSlashExtraGraze {
    pub timer: f64,
    pub con: f64,
    pub grazetpfactor: f64,
    pub grazetimefactor: f64,
    pub grazesizefactor: f64,
    pub grazecount: f64,
}
impl Default for TrackingSwordSlashExtraGraze {
    fn default() -> Self {
        TrackingSwordSlashExtraGraze { timer: 0.0, con: 0.0, grazetpfactor: 1.0, grazetimefactor: 1.0, grazesizefactor: 1.0, grazecount: 0.0 }
    }
}
impl Object for TrackingSwordSlashExtraGraze {
    fn name(&self) -> &'static str { "obj_tracking_sword_slash_extra_graze" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer = 0.0;
        self.con = 0.0;
        me.image_xscale = 900.0;
        me.image_yscale = 7.0;
        me.image_blend = C_RED;
        me.visible = false;
        self.grazetpfactor = 1.0;
        self.grazetimefactor = 1.0;
        self.grazesizefactor = 1.0;
        self.grazecount = 0.0;
        self.grazetpfactor += scr_armorcheck_equipped_party(g, 15.0) * 0.1;
        self.grazetpfactor += scr_armorcheck_equipped_party(g, 24.0) * 0.05;
        self.grazetimefactor += scr_armorcheck_equipped_party(g, 14.0) * 0.1;
        if self.grazetimefactor > 3.0 {
            self.grazetimefactor = 3.0;
        }
        if self.grazetpfactor > 3.0 {
            self.grazetpfactor = 3.0;
        }
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if g.glob.inv < 0.0 && g.place_meeting(me, me.x, me.y, "obj_heart") {
            let tpf = self.grazetpfactor;
            let tf = self.grazetimefactor;
            let tracking_v1 =
                g.get_first::<TrackingSwordsManager>("obj_tracking_swords_manager").map(|(_, o)| o.variant == 1.0).unwrap_or(false);
            if tracking_v1 {
                battle::scr_tensionheal(g, 4.0 * tpf);
            } else if g.exists("obj_sword_vortex_manager") {
                battle::scr_tensionheal(g, 4.0 * tpf);
            } else {
                battle::scr_tensionheal(g, 7.0 * tpf);
            }
            if g.glob.turntimer >= 10.0 {
                g.glob.turntimer -= (1.0 / 30.0) * tf;
            }
            g.destroy_self(me);
        }
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        g.draw_self(me);
        self.timer += 1.0;
        if self.timer == 3.0 {
            g.destroy_self(me);
        }
    }
    obj_vars!(timer, con, grazetpfactor, grazetimefactor, grazesizefactor, grazecount);
}

// ============================================================================ sword vortex

/// obj_sword_vortex_manager (parent obj_regularbullet; no event_inherited; no Draw → nothing drawn).
/// Note: Create configures itself with its hardcoded `variant = 3`; the controller's later
/// `variant = difficulty` only reaches the spawned swords' `variant` (as in GML).
pub struct SwordVortexManager {
    pub timer: f64,
    pub siner: f64,
    pub con: f64,
    pub variant: f64,
    pub firstsword: bool,
    pub swordcount: f64,
    pub sinpower: f64,
    pub sinspeed: f64,
    pub startinglen: f64,
    pub shrinkrate: f64,
    pub multiswordmax: f64,
    pub multiswordframes: f64,
    pub multiswordcon: f64,
    pub multiswordcount: f64,
    pub centermoves: f64,
    pub centermovescon: f64,
    pub centermovestimer: f64,
    pub movespeed: f64,
    pub swordcirclecenterx: f64,
    pub swordcirclecentery: f64,
    pub startx: f64,
    pub starty: f64,
    pub targetx: f64,
    pub targety: f64,
    pub setcount: f64,
    pub setdirection: Vec<f64>,
    pub creatorid: f64,
    pub creator: f64,
    pub rate: f64,
    pub ratedecay: f64,
    pub rateminimum: f64,
    pub maxswords: f64,
    pub inst: Id,
}
impl Default for SwordVortexManager {
    fn default() -> Self {
        SwordVortexManager {
            timer: 0.0,
            siner: 0.0,
            con: 0.0,
            variant: 3.0,
            firstsword: false,
            swordcount: 0.0,
            sinpower: 65.0,
            sinspeed: 24.0,
            startinglen: 70.0,
            shrinkrate: 0.0,
            multiswordmax: 0.0,
            multiswordframes: 0.0,
            multiswordcon: 0.0,
            multiswordcount: 0.0,
            centermoves: 0.0,
            centermovescon: 0.0,
            centermovestimer: 0.0,
            movespeed: 60.0,
            swordcirclecenterx: 0.0,
            swordcirclecentery: 0.0,
            startx: 0.0,
            starty: 0.0,
            targetx: 0.0,
            targety: 0.0,
            setcount: 0.0,
            setdirection: vec![-1.0; 50],
            creatorid: -1.0,
            creator: -1.0,
            rate: 0.0,
            ratedecay: 0.0,
            rateminimum: 0.0,
            maxswords: 0.0,
            inst: NOONE,
        }
    }
}
impl Object for SwordVortexManager {
    fn name(&self) -> &'static str { "obj_sword_vortex_manager" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        let (gx, gy) = g.first_inst("obj_growtangle").map(|t| (t.x, t.y)).unwrap_or((0.0, 0.0));
        self.swordcirclecenterx = gx;
        self.swordcirclecentery = gy;
        self.startx = self.swordcirclecenterx;
        self.starty = self.swordcirclecentery;
        battle::bullet_init(me);
        let alt6 = |s: &mut Vec<f64>| {
            for k in 1..=6 {
                s[k] = if k % 2 == 1 { 0.0 } else { 180.0 };
            }
        };
        if self.variant == 0.0 {
            self.rate = 1.0;
            self.ratedecay = 0.0;
            self.rateminimum = 1.0;
            self.maxswords = 2.0;
            self.setdirection[1] = 0.0;
            self.setdirection[2] = 180.0;
            self.multiswordmax = 0.0;
            self.sinpower = 65.0;
            self.sinspeed = 24.0;
            self.startinglen = 70.0;
        }
        if self.variant == 1.0 {
            self.rate = 10.0;
            self.ratedecay = 0.0;
            self.rateminimum = 1.0;
            self.maxswords = 6.0;
            self.multiswordmax = 2.0;
            self.multiswordframes = 1.0;
            self.sinpower = 20.0;
            self.sinspeed = 18.0;
            self.startinglen = 90.0;
            self.shrinkrate = 0.15;
            alt6(&mut self.setdirection);
        }
        if self.variant == 2.0 {
            self.rate = 4.0;
            self.ratedecay = 0.0;
            self.rateminimum = 1.0;
            self.maxswords = 6.0;
            self.multiswordmax = 2.0;
            self.multiswordframes = 1.0;
            self.sinpower = 20.0;
            self.sinspeed = 26.0;
            self.startinglen = 70.0;
            alt6(&mut self.setdirection);
        }
        if self.variant == 3.0 {
            self.rate = 11.0;
            self.ratedecay = 0.0;
            self.rateminimum = 1.0;
            self.maxswords = 6.0;
            self.multiswordmax = 2.0;
            self.multiswordframes = 1.0;
            self.sinpower = 17.0;
            self.sinspeed = 22.0;
            self.startinglen = 80.0;
            alt6(&mut self.setdirection);
            self.centermoves = 1.0;
            self.movespeed = 60.0;
        }
        self.timer = self.rate - 5.0;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer += 1.0;
        self.siner += 1.0;
        if (self.timer == self.rate && self.swordcount < self.maxswords)
            || (self.timer == self.multiswordframes && self.multiswordcon == 1.0)
        {
            let (gx, gy) = g.first_inst("obj_growtangle").map(|t| (t.x, t.y)).unwrap_or((0.0, 0.0));
            let inst = g.instance_create(gx, gy, Box::new(SwordVortex::default()));
            self.inst = inst;
            let d0 = g.choose(&[0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0]);
            if let Some((i, o)) = g.get::<SwordVortex>(inst) {
                o.dir = d0;
                o.variant = self.variant;
                o.sinpower = self.sinpower;
                o.sinspeed = self.sinspeed;
                o.len = self.startinglen;
                o.lenstart = o.len;
                o.shrinkrate = self.shrinkrate;
                i.damage = me.damage;
                i.target = me.target;
            }
            self.swordcount += 1.0;
            self.setcount += 1.0;
            let sd = self.setdirection.get(self.setcount as usize).copied().unwrap_or(-1.0);
            if sd != -1.0 {
                if let Some((_, o)) = g.get::<SwordVortex>(inst) {
                    o.dir = sd;
                }
            }
            if self.multiswordmax > 0.0 {
                self.multiswordcount += 1.0;
            }
            if self.multiswordcon == 0.0 && self.multiswordmax > 0.0 {
                self.multiswordcon = 1.0;
            }
            if self.multiswordcon == 1.0 && self.multiswordcount == self.multiswordmax {
                self.multiswordcon = 0.0;
                self.multiswordcount = 0.0;
            }
            if let Some((i, o)) = g.get::<SwordVortex>(inst) {
                i.x = i.xstart + lengthdir_x(o.len, o.dir);
                i.y = i.ystart + lengthdir_y(o.len, o.dir);
                i.image_angle = o.dir - 90.0;
            }
            self.rate -= self.ratedecay;
            if self.rate < self.rateminimum {
                self.rate = self.rateminimum;
            }
            self.timer = 0.0;
        }
        if self.centermoves == 1.0 {
            if self.centermovescon == 0.0 {
                self.startx = self.swordcirclecenterx;
                self.starty = self.swordcirclecentery;
                let (gx, gy) = g.first_inst("obj_growtangle").map(|t| (t.x, t.y)).unwrap_or((0.0, 0.0));
                self.targetx = (gx - 60.0) + g.irandom(120.0);
                self.targety = (gy - 60.0) + g.irandom(120.0);
                self.centermovescon = 1.0;
            }
            if self.centermovescon == 1.0 {
                self.centermovestimer += 1.0;
                self.swordcirclecenterx = lerp(self.startx, self.targetx, self.centermovestimer / self.movespeed);
                self.swordcirclecentery = lerp(self.starty, self.targety, self.centermovestimer / self.movespeed);
                if self.centermovestimer == self.movespeed {
                    self.centermovestimer = 0.0;
                    self.centermovescon = 0.0;
                }
            }
        }
    }
    obj_vars!(
        timer, siner, con, variant, swordcount, sinpower, sinspeed, startinglen, shrinkrate, multiswordmax, multiswordframes,
        multiswordcon, multiswordcount, centermoves, centermovescon, centermovestimer, movespeed, swordcirclecenterx,
        swordcirclecentery, startx, starty, targetx, targety, setcount, creatorid, creator, rate, ratedecay, rateminimum, maxswords
    );
}

/// obj_sword_vortex (parent obj_regularbullet; no event_inherited)
pub struct SwordVortex {
    pub timer: f64,
    pub con: f64,
    pub dir: f64,
    pub variant: f64,
    pub spinspeed: f64,
    pub speedtowardscenter: f64,
    pub len: f64,
    pub sinpower: f64,
    pub sinspeed: f64,
    pub shrinkrate: f64,
    pub lenstart: f64,
}
impl Default for SwordVortex {
    fn default() -> Self {
        SwordVortex {
            timer: 0.0,
            con: 0.0,
            dir: 0.0,
            variant: 0.0,
            spinspeed: 4.0,
            speedtowardscenter: 0.4,
            len: 70.0,
            sinpower: 65.0,
            sinspeed: 24.0,
            shrinkrate: 0.0,
            lenstart: 70.0,
        }
    }
}
impl Object for SwordVortex {
    fn name(&self) -> &'static str { "obj_sword_vortex" }
    fn create(&mut self, me: &mut Inst, _g: &mut Game) {
        self.timer = 0.0;
        self.con = 0.0;
        self.dir = 0.0;
        me.image_alpha = 0.0;
        battle::bullet_init(me);
        me.destroyonhit = 0.0;
        me.damage = 10.0;
        me.grazepoints = 2.0;
        me.timepoints = 1.0;
        self.spinspeed = 4.0;
        self.speedtowardscenter = 0.4;
        self.len = 70.0;
        self.sinpower = 65.0;
        self.sinspeed = 24.0;
        self.shrinkrate = 0.0;
        self.lenstart = self.len;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        me.image_alpha += 0.1;
        self.dir -= self.spinspeed * lerp(2.0, 1.0, self.len / 120.0);
        me.image_angle = self.dir - 90.0;
        let mgr = g
            .get_first::<SwordVortexManager>("obj_sword_vortex_manager")
            .map(|(_, m)| (m.siner, m.swordcirclecenterx, m.swordcirclecentery));
        if let Some((siner, cx, cy)) = mgr {
            self.len = self.lenstart + (siner / self.sinspeed).sin() * self.sinpower;
            me.x = cx + lengthdir_x(self.len, self.dir);
            me.y = cy + lengthdir_y(self.len, self.dir);
            self.lenstart -= self.shrinkrate;
        }
        self.timer += 1.0;
        if self.timer % 4.0 == 0.0 {
            me.grazed = 0.0;
        }
    }
    obj_vars!(timer, con, dir, variant, spinspeed, speedtowardscenter, len, sinpower, sinspeed, shrinkrate, lenstart);
}
