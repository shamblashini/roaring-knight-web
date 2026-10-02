//! Sword tunnel (knight myattackchoice 13, obj_dbulletcontroller type 153).
//!
//! Ports obj_sword_tunnel_manager, obj_sword_tunnel_sword, obj_sword_tunnel_hitbox,
//! obj_knight_swordtunnelanim, obj_afterimage_grow and the scripts scr_anglechange /
//! scr_afterimage_grow / scr_darksize.
//!
//! TODO(battle): obj_knight_enemy Draw_0 does `siner2++` and then `if (i_ex(obj_knight_swordtunnelanim)) exit;`
//!   (the knight is not drawn while the tunnel animation exists).
//! TODO(battle): obj_grazebox Collision_obj_collidebullet: `if (!other.active && other.object_index != obj_sword_tunnel_sword) exit;`
//!   (tunnel swords can be grazed even while inactive).
//! TODO(battle): the sword's slash sets obj_heart.mask_index = spr_dodgeheart_smallmask and this attack never restores it.

use crate::assets::spr;
use crate::battle::{self, DbCtrl, KnightEnemy};
use crate::gfx::{merge_color, C_RED, C_WHITE};
use crate::gm::*;
use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object, NOONE};

/// obj_dbulletcontroller Step_0 block for this attack's type(s).
pub fn ctrl_step(c: &mut DbCtrl, me: &mut Inst, g: &mut Game) {
    if c.typ == 153 && c.made == 0.0 {
        let bx = g.first_inst("obj_growtangle").map(|b| b.x).unwrap_or(0.0);
        let cy = g.cameray();
        let manager = g.instance_create(bx, cy, Box::new(SwordTunnelManager::default()));
        battle::scr_bullet_inherit(&*me, manager, g);
        if let Some((mi, mo)) = g.get::<SwordTunnelManager>(manager) {
            mo.difficulty = c.difficulty;
            mi.damage = me.damage;
        }
        g.user_event(manager, 0);
        c.made = 1.0;
    }
}

// ============================================================================ helpers

/// scr_anglechange(angle, target, speed)
fn scr_anglechange(a: f64, b: f64, spd: f64) -> f64 {
    // median(-spd, spd, angle_difference(b, a))
    let d = angle_difference(b, a);
    let (lo, hi) = if -spd <= spd { (-spd, spd) } else { (spd, -spd) };
    d.max(lo).min(hi)
}

/// scr_afterimage_grow()
fn scr_afterimage_grow(me: &Inst, g: &mut Game) -> Id {
    let id = g.instance_create(me.x, me.y, Box::new(AfterimageGrow::default()));
    if let Some(a) = g.inst_mut(id) {
        a.sprite_index = me.sprite_index;
        a.image_index = me.image_index;
        a.image_blend = me.image_blend;
        a.image_speed = 0.0;
        a.depth = me.depth;
        a.image_xscale = me.image_xscale;
        a.image_yscale = me.image_yscale;
        a.image_angle = me.image_angle;
    }
    id
}

fn heart_xy(g: &mut Game) -> (f64, f64) { g.first_inst("obj_heart").map(|h| (h.x, h.y)).unwrap_or((0.0, 0.0)) }

fn growtangle_y(g: &mut Game) -> f64 { g.first_inst("obj_growtangle").map(|b| b.y).unwrap_or(0.0) }

// ============================================================================ obj_sword_tunnel_manager

/// obj_sword_tunnel_manager (parent obj_regularbullet; no event_inherited anywhere).
#[derive(Default)]
pub struct SwordTunnelManager {
    pub timer: f64,
    pub finishtimer: f64,
    pub finishtimermax: f64,
    pub con: f64,
    pub swordx: f64,
    pub swordy: f64,
    pub swordxrel: f64,
    pub swordyrel: f64,
    pub sworddirection: f64,
    pub swordcount: f64,
    pub setcount: f64,
    pub waitsetcount: f64,
    pub movedirection: &'static str,
    pub tobymode: f64,
    pub difficulty: f64,
    pub tobyvolleymode: f64,
    pub tobyvolleycount: f64,
    pub tobyvolleyamount: f64,
    pub tobyvolleymodeinitspeed: f64,
    pub stopsfxtimer: f64,
    pub tobytimer: f64,
    // Other_10
    pub rate: f64,
    pub gapsize: f64,
    pub verticalchange: f64,
    pub maxswords: f64,
}

impl SwordTunnelManager {
    /// Spawn one obj_sword_tunnel_sword.
    fn make_sword(&self, x: f64, y: f64, g: &mut Game) -> Id { g.instance_create(x, y, Box::new(SwordTunnelSword::default())) }
}

impl Object for SwordTunnelManager {
    fn name(&self) -> &'static str { "obj_sword_tunnel_manager" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer = -40.0 + g.irandom(10.0);
        self.finishtimer = 0.0;
        self.finishtimermax = 230.0;
        let kdiff = g.get_first::<KnightEnemy>("obj_knight_enemy").map(|(_, k)| k.difficulty).unwrap_or(0.0);
        if kdiff == 3.0 {
            self.finishtimermax = 250.0;
        }
        self.con = 0.0;
        self.swordx = g.camerax() + g.camerawidth() + 20.0;
        self.swordy = growtangle_y(g);
        self.swordxrel = 340.0;
        self.swordyrel = 0.0;
        self.sworddirection = 180.0;
        self.swordcount = 0.0;
        self.setcount = g.choose(&[2.0, 3.0, 4.0]);
        self.waitsetcount = g.choose(&[1.0, 2.0, 3.0]);
        self.movedirection = g.choose(&["up", "down"]);
        self.tobymode = 0.0;
        self.difficulty = 0.0;
        self.tobyvolleymode = 0.0;
        self.tobyvolleycount = 0.0;
        self.tobyvolleyamount = 10.0 + g.irandom(6.0);
        self.tobyvolleymodeinitspeed = 1.0;
        self.stopsfxtimer = 0.0;
        self.user(0, me, g);
        let (kx, ky) = g.first_inst("obj_knight_enemy").map(|k| (k.x, k.y)).unwrap_or((0.0, 0.0));
        g.instance_create(kx, ky, Box::new(KnightSwordtunnelAnim::default()));
    }

    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            0 => {
                self.rate = 6.0;
                self.gapsize = 50.0;
                self.verticalchange = 15.0;
                self.tobymode = 0.0;
                self.maxswords = 999.0;
                if self.difficulty == 0.0 {
                    self.rate = 4.0;
                    self.gapsize = 45.0;
                    self.verticalchange = 10.0;
                    self.tobymode = 0.0;
                    self.maxswords = 999.0;
                }
                if self.difficulty == 1.0 {
                    self.rate = 4.0;
                    self.gapsize = 45.0;
                    self.verticalchange = 10.0;
                    self.tobymode = 1.0;
                    self.tobytimer = 0.0;
                    self.maxswords = 999.0;
                }
                if self.difficulty == 2.0 {
                    self.rate = 4.0;
                    self.gapsize = 45.0;
                    self.verticalchange = 7.0;
                    self.tobymode = 2.0;
                    self.tobytimer = 0.0;
                    self.maxswords = 999.0;
                }
                if self.difficulty == 3.0 {
                    self.rate = 4.0;
                    self.gapsize = 45.0;
                    self.verticalchange = 7.0;
                    self.tobymode = 3.0;
                    self.tobytimer = 0.0;
                    self.maxswords = 999.0;
                }
                if self.difficulty == 4.0 {
                    self.rate = 4.0;
                    self.gapsize = 40.0;
                    self.verticalchange = 10.0;
                    self.tobymode = 0.0;
                    self.maxswords = 999.0;
                }
            }
            5 => battle::collidebullet_hit(me, g),
            _ => {}
        }
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer += 1.0;
        self.finishtimer += 1.0;
        let mut xx = g.camerax() + 320.0;
        let mut yy = g.cameray() + 180.0;
        if let Some(b) = g.first_inst("obj_growtangle") {
            xx = b.x;
            yy = b.y;
        }
        if self.finishtimer == self.finishtimermax {
            self.con = 1.0;
            for id in g.ids_of("obj_sword_tunnel_sword") {
                if let Some((_, s)) = g.get::<SwordTunnelSword>(id) {
                    s.con = 1.0;
                }
            }
        }
        if self.timer >= self.rate && self.con == 0.0 {
            let dmg = me.damage;
            if self.tobymode == 0.0 {
                let s = self.make_sword(self.swordx, self.swordy - 50.0 - self.gapsize / 2.0, g);
                if let Some(i) = g.inst_mut(s) {
                    i.image_angle = 270.0;
                    i.damage = dmg;
                }
                let s = self.make_sword(self.swordx, self.swordy + 50.0 + self.gapsize / 2.0, g);
                if let Some(i) = g.inst_mut(s) {
                    i.image_angle = 90.0;
                    i.damage = dmg;
                }
            } else if self.tobymode == 1.0 {
                self.tobytimer += 1.0;
                let s = self.make_sword(self.swordx, self.swordy - 50.0 - self.gapsize / 2.0, g);
                let add = (self.tobytimer / 6.0).sin() * 16.0 + g.random(1.0);
                if let Some((i, o)) = g.get::<SwordTunnelSword>(s) {
                    i.image_angle = 270.0;
                    o._speed += add;
                    i.damage = dmg;
                }
                let s = self.make_sword(self.swordx, self.swordy + 50.0 + self.gapsize / 2.0, g);
                let add = (self.tobytimer / 6.0).sin() * 16.0 + g.random(1.0);
                self.timer = max(0.0, (self.tobytimer / 6.0).sin() * 2.0);
                if let Some((i, o)) = g.get::<SwordTunnelSword>(s) {
                    i.image_angle = 90.0;
                    o._speed += add;
                    i.damage = dmg;
                }
            } else if self.tobymode == 2.0 || self.tobymode == 3.0 {
                let mode3 = self.tobymode == 3.0;
                if mode3 {
                    self.tobytimer += 1.0;
                    if self.tobyvolleymode == 0.0 {
                        self.verticalchange = (self.tobytimer / 8.0).sin().abs() * 5.0;
                        self.gapsize = 34.0 + self.verticalchange * 1.4;
                    }
                }
                let gy = growtangle_y(g);
                let sx = lengthdir_x(self.swordxrel, self.sworddirection + 180.0);
                let sy = lengthdir_y(self.swordxrel, self.sworddirection + 180.0);
                let syaddy = lengthdir_y(self.swordy - gy, self.sworddirection + 270.0);
                let syaddx = lengthdir_x(self.swordy - gy, self.sworddirection + 270.0);
                let sgapy = lengthdir_y(self.gapsize, self.sworddirection + 270.0) * 2.0;
                let sgapx = lengthdir_x(self.gapsize, self.sworddirection + 270.0) * 2.0;
                self.tobytimer += 1.0;
                let speedproportion = lerp(1.0, 0.8, lengthdir_y(1.0, self.sworddirection + 180.0).abs());
                let grav = if mode3 {
                    ((2.0 * speedproportion) - (self.verticalchange / 15.0)) * self.tobyvolleymodeinitspeed
                } else {
                    2.0 * speedproportion
                };
                let sd = self.sworddirection;
                let s = self.make_sword(((xx + sx) - sgapx) + syaddx, ((yy + sy) - sgapy) + syaddy, g);
                if let Some((i, o)) = g.get::<SwordTunnelSword>(s) {
                    i.image_angle = sd + 270.0;
                    o.mydirection = sd;
                    o._speed = -8.0 * speedproportion;
                    o._gravity = grav;
                    i.damage = dmg;
                }
                let s = self.make_sword(xx + sx + sgapx + syaddx, yy + sy + sgapy + syaddy, g);
                if let Some((i, o)) = g.get::<SwordTunnelSword>(s) {
                    i.image_angle = sd + 90.0;
                    o.mydirection = sd;
                    o._speed = -8.0 * speedproportion;
                    o._gravity = grav;
                    i.damage = dmg;
                }
                self.sworddirection += if mode3 { 8.0 } else { 4.0 };
            }
            if self.movedirection == "up" {
                self.swordy -= self.verticalchange;
            }
            if self.movedirection == "down" {
                self.swordy += self.verticalchange;
            }
            self.swordcount += 1.0;
            if (self.setcount == self.swordcount && (self.movedirection == "down" || self.movedirection == "up"))
                || (self.waitsetcount == self.swordcount && self.movedirection == "none")
            {
                self.swordcount = 0.0;
                self.setcount = g.choose(&[2.0, 3.0, 4.0]);
                self.waitsetcount = g.choose(&[1.0, 2.0, 3.0]);
                if self.tobymode == 2.0 {
                    self.setcount = g.choose(&[4.0, 6.0, 8.0]);
                    self.waitsetcount = g.choose(&[2.0, 4.0, 6.0]);
                    if self.tobyvolleymode != 0.0 {
                        self.setcount = g.choose(&[6.0, 10.0, 12.0]);
                        self.waitsetcount = g.choose(&[6.0, 8.0, 12.0]);
                    }
                }
                if self.movedirection == "none" {
                    self.movedirection = g.choose(&["up", "down"]);
                } else {
                    self.movedirection = "none";
                }
                let gy = growtangle_y(g);
                if self.movedirection == "up" && self.swordy < gy - 20.0 {
                    self.movedirection = "down";
                }
                if self.movedirection == "down" && self.swordy > gy + 20.0 {
                    self.movedirection = "up";
                }
            }
        }
        if self.timer >= self.rate && self.stopsfxtimer < 3.0 {
            if self.con == 1.0 {
                self.stopsfxtimer += 1.0;
            }
            g.snd_play_x("snd_heavy_passing", 0.3, 1.2);
            self.timer = 0.0;
        }
        if self.swordcount >= self.maxswords {
            g.destroy_self(me);
        }
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        g.snd_stop("snd_object_passing");
        g.snd_stop("snd_heavy_passing");
    }

    obj_vars!(timer, finishtimer, con, swordx, swordy, difficulty);
}

// ============================================================================ obj_sword_tunnel_sword

/// obj_sword_tunnel_sword (parent obj_regularbullet; Other_15 inherited from obj_collidebullet).
#[derive(Default)]
pub struct SwordTunnelSword {
    pub timer: f64,
    pub con: f64,
    pub _speed: f64,
    pub _gravity: f64,
    pub _maxspeed: f64,
    pub create_2nd_hitbox: bool,
    pub mydirection: f64,
    pub randx: f64,
    pub randy: f64,
    pub targetangle: f64,
    pub anglespeed: f64,
    pub telegraph: f64,
    pub telegraphalpha: f64,
    pub hitbox: Id,
    pub inst: Id,
}

impl Object for SwordTunnelSword {
    fn name(&self) -> &'static str { "obj_sword_tunnel_sword" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::bullet_init(me);
        me.grazepoints = 0.8;
        me.destroyonhit = 0.0;
        self.timer = 0.0;
        self.con = 0.0;
        self._speed = 10.0;
        self._gravity = 2.0;
        self._maxspeed = 30.0;
        me.active = 0.0;
        self.create_2nd_hitbox = false;
        me.image_index = 2.0;
        me.image_speed = 0.0;
        self.mydirection = 180.0;
        self._speed = 6.0;
        self._gravity = 1.0;
        me.image_yscale = 0.0;
        self.randx = -20.0 + g.irandom(40.0);
        self.randy = -20.0 + g.irandom(40.0);
        self.targetangle = 0.0;
        self.anglespeed = 8.0;
        self.telegraph = 0.0;
        self.telegraphalpha = 0.0;
        self.hitbox = NOONE;
        self.inst = NOONE;
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self._speed += self._gravity;
        if self._speed > self._maxspeed {
            self._speed = self._maxspeed;
        }
        let mut xadd = lengthdir_x(1.0, self.mydirection);
        let mut yadd = lengthdir_y(1.0, self.mydirection);
        if self.con == 1.0 {
            self.timer += 1.0;
            let c = 10.0;
            if self.timer == 1.0 {
                self._gravity = 0.0;
                self.telegraph = 1.0;
            }
            if self.timer < 10.0 + c / 2.0 {
                self.anglespeed = lerp(8.0, 0.0, self.timer / (10.0 + c / 2.0));
                let (hx, hy) = heart_xy(g);
                let pd = point_direction(me.x, me.y, hx + 10.0 + self.randx, hy + 10.0 + self.randy);
                me.image_angle += scr_anglechange(me.image_angle, pd, self.anglespeed);
                self.targetangle += self.anglespeed;
                // show_debug_message(targetangle) omitted
            }
            me.set_direction(me.image_angle);
            if self.timer < 10.0 + c {
                self._speed = lerp(self._speed, 0.0, self.timer / 10.0);
            }
            if self.timer >= 11.0 + c && self.timer < 15.0 + c {
                if !g.audio.is_playing_name("snd_knight_jump") {
                    g.snd_play_x("snd_knight_jump", 1.0, 0.8);
                }
                self._speed = 2.0;
                xadd = lengthdir_x(2.0, me.image_angle + 180.0);
                yadd = lengthdir_y(2.0, me.image_angle + 180.0);
            }
            if self.timer >= 15.0 + c && self.timer < 20.0 + c {
                xadd = 0.0;
                yadd = 0.0;
            }
            if self.timer == 20.0 + c {
                let inst = scr_afterimage_grow(me, g);
                if let Some((_, a)) = g.get::<AfterimageGrow>(inst) {
                    a.xrate = 0.4;
                    a.yrate = 0.4;
                    a.fade = 0.2;
                }
                self.inst = inst;
            }
            if self.timer >= 20.0 + c {
                if !g.audio.is_playing_name("snd_knight_cut") {
                    g.snd_play_x("snd_knight_cut", 1.0, 0.8);
                }
                self.telegraph = 0.0;
                me.damage = 160.0;
                self._speed = 80.0;
                xadd = lengthdir_x(1.0, me.image_angle);
                yadd = lengthdir_y(1.0, me.image_angle);
            }
        }
        me.image_blend = C_WHITE;
        let (hx, hy) = heart_xy(g);
        if me.x > hx - 80.0 && me.x < hx + 80.0 && me.y < hy + 80.0 && me.y > hy - 80.0 {
            me.image_blend = C_RED;
            let remx = me.x;
            let remy = me.y;
            let n = max(floor(self._speed / 8.0), 1.0) as i64;
            for _ in 0..n {
                me.x += xadd * 8.0;
                me.y += yadd * 8.0;
                me.active = 1.0;
                if self.con == 1.0 && self._speed == 80.0 && !self.create_2nd_hitbox {
                    let hb = g.instance_create_depth(me.x, me.y, me.depth - 1.0, Box::new(SwordTunnelHitbox::default()));
                    if let Some(h) = g.inst_mut(hb) {
                        h.image_angle = me.image_angle;
                        h.image_yscale = 0.4;
                        h.image_xscale = 999.0;
                    }
                    self.hitbox = hb;
                    let small = spr("spr_dodgeheart_smallmask");
                    for hid in g.ids_of("obj_heart") {
                        if let Some(h) = g.inst_mut(hid) {
                            h.mask_index = small;
                        }
                    }
                    self.create_2nd_hitbox = true;
                } else if !self.create_2nd_hitbox
                    && g
                        .collision_line(
                            me.x,
                            me.y,
                            me.x + lengthdir_x(37.0, me.image_angle),
                            me.y + lengthdir_y(37.0, me.image_angle),
                            "obj_heart",
                            None,
                        )
                        .is_some()
                {
                    self.user(5, me, g);
                }
                me.active = 0.0;
            }
            me.x = remx;
            me.y = remy;
        }
        me.x += xadd * self._speed;
        me.y += yadd * self._speed;
        let ai = g.afterimage(me);
        let con = self.con;
        if let Some(a) = g.inst_mut(ai) {
            a.image_alpha = 0.4;
            a.x = (me.x + me.xprevious) / 2.0;
            a.y = (me.y + me.yprevious) / 2.0;
            if con > 0.0 {
                a.image_blend = C_WHITE;
            }
        }
        let (cx, cy) = (g.camerax(), g.cameray());
        if me.x <= cx - 100.0 {
            g.destroy_self(me);
        }
        if me.x >= cx + 740.0 {
            g.destroy_self(me);
        }
        if me.y >= cy + 600.0 {
            g.destroy_self(me);
        }
        if me.y <= cy - 250.0 {
            g.destroy_self(me);
        }
        if self.con == 0.0 {
            me.image_yscale = lerp(me.image_yscale, self._speed / 20.0, 0.1);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.telegraph == 0.0 && self.telegraphalpha > 0.0 {
            self.telegraphalpha -= 0.1;
        }
        if self.telegraph == 1.0 && self.telegraphalpha < 0.5 {
            self.telegraphalpha += 0.05;
        }
        if self.telegraphalpha > 0.0 {
            g.draw_sprite_ext(spr("spr_lasergun_laser_telegraph"), 0.0, me.x, me.y, 999.0, 0.4, me.image_angle, C_RED, self.telegraphalpha);
        }
        if self.con > 0.0 {
            let mut t = self.timer;
            if t > 10.0 {
                t = 10.0;
            }
            if me.image_blend == C_RED {
                me.image_blend = merge_color(C_RED, C_WHITE, t / 10.0);
            }
        }
        for i in 0..10 {
            let f = i as f64 / 10.0;
            g.draw_sprite_ext(
                me.sprite_index,
                me.image_index,
                lerp(me.xprevious, me.x, f),
                lerp(me.yprevious, me.y, f),
                me.image_xscale,
                me.image_yscale,
                me.image_angle,
                me.image_blend,
                f,
            );
        }
        g.draw_self(me);
    }

    obj_vars!(timer, con, _speed, _gravity, mydirection, telegraphalpha);
}

// ============================================================================ obj_sword_tunnel_hitbox

/// obj_sword_tunnel_hitbox (parent obj_regularbullet). Destroy_0 is `exit` (no-op).
#[derive(Default)]
pub struct SwordTunnelHitbox {
    pub timer: f64,
    pub con: f64,
}

impl Object for SwordTunnelHitbox {
    fn name(&self) -> &'static str { "obj_sword_tunnel_hitbox" }

    fn create(&mut self, me: &mut Inst, _g: &mut Game) {
        battle::bullet_init(me);
        me.grazepoints = 0.8;
        me.destroyonhit = 0.0;
        self.timer = 0.0;
        self.con = 0.0;
        me.damage = 160.0;
        me.active = 0.0;
        me.visible = false;
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.timer += 1.0;
        if self.timer == 2.0 {
            me.active = 1.0;
        }
        if self.timer == 3.0 {
            g.destroy_self(me);
        }
    }

    obj_vars!(timer, con);
}

// ============================================================================ obj_knight_swordtunnelanim

/// obj_knight_swordtunnelanim: the Knight pointing / dashing away while the tunnel runs.
#[derive(Default)]
pub struct KnightSwordtunnelAnim {
    pub con: f64,
    pub timer: f64,
    pub siner: f64,
    pub animindex: f64,
    pub drawtrail: bool,
    pub shadowtimer: f64,
    pub dir: f64,
    pub fadeaudio: f64,
    pub fadeaudio2: f64,
    pub shinkafade: f64,
    pub leafpitch: f64,
    pub endtimer: f64,
}

impl Object for KnightSwordtunnelAnim {
    fn name(&self) -> &'static str { "obj_knight_swordtunnelanim" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.con = 0.0;
        self.timer = 0.0;
        self.siner = 0.0;
        self.animindex = 0.0;
        // scr_darksize()
        me.image_xscale = 2.0;
        me.image_yscale = 2.0;
        me.sprite_index = spr("spr_roaringknight_point_ol");
        me.image_speed = 0.0;
        if let Some(b) = g.first_inst("obj_growtangle") {
            me.depth = b.depth - 1.0;
        }
        self.drawtrail = true;
        self.shadowtimer = 0.0;
        self.dir = 4.0;
        self.fadeaudio = 0.0;
        self.fadeaudio2 = 0.0;
        self.shinkafade = 0.0;
        self.leafpitch = 1.0;
        self.endtimer = 0.0;
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.con == 0.0 {
            if self.timer < 60.0 && self.timer % 3.0 == 0.0 {
                // sound 226 = snd_leaf_dodge
                let p = 1.0 + g.random(0.2);
                g.snd_play_x("snd_leaf_dodge", 0.8 * self.fadeaudio, p);
                let p = 1.0 + g.random(0.2);
                g.snd_play_x("snd_leaf_dodge", 0.8 * self.fadeaudio, p);
            }
            self.timer += 1.0;
            if self.timer == 1.0 {
                g.lerpvar(me, "image_index", 0.0, 4.0, 10.0, 0, "out");
                g.lerpvar(me, "dir", 4.0, -18.0, 40.0, 2, "in");
            }
            if self.timer == 15.0 {
                g.lerpvar(me, "fadeaudio", 0.0, 1.0, 5.0, 0, "out");
            }
            if self.timer == 20.0 {
                g.lerpvar(me, "fadeaudio", self.fadeaudio, 0.0, 20.0, 0, "out");
                g.lerpvar(me, "image_alpha", 1.0, 0.0, 10.0, 0, "out");
                me.set_hspeed(-4.0);
            }
            if self.timer == 26.0 {
                self.drawtrail = false;
            }
            if self.timer == 60.0 {
                self.timer = 0.0;
                self.con = 1.0;
            }
        }
        if self.con == 1.0 {
            self.timer += 1.0;
            if self.timer == 1.0 {
                let leafpitchtime = 120.0;
                let shinkafadeintime = 20.0;
                g.lerpvar(me, "shinkafade", 0.0, 1.0, shinkafadeintime, 0, "out");
                g.lerpvar(me, "leafpitch", 0.4, 1.0, leafpitchtime, 0, "out");
            }
            if self.timer % 3.0 == 0.0 {
                // sound 226 = snd_leaf_dodge
                let vol = 0.4 * self.shinkafade + self.leafpitch / 10.0;
                let p = self.leafpitch + g.random_range(0.0, 0.2);
                g.snd_play_x("snd_leaf_dodge", vol, p);
                let p = self.leafpitch + g.random_range(0.0, 0.2);
                g.snd_play_x("snd_leaf_dodge", vol, p);
            }
            if self.timer % 11.0 == 0.0 {
                g.snd_play_x("snd_shinka_ambience", 1.0 * self.shinkafade, 1.0);
            }
            let endtime = 120.0;
            if self.timer == endtime {
                g.lerpvar(me, "shinkafade", 1.0, 0.0, 5.0, 0, "out");
            }
        }
        if g.glob.turntimer < 10.0 {
            self.endtimer += 1.0;
            me.image_alpha = 1.0;
            if let Some(k) = g.first_inst("obj_knight_enemy") {
                me.x = k.x;
            }
            if self.endtimer == 1.0 {
                me.sprite_index = spr("spr_roaringknight_ball_transition_sword");
                me.image_index = 5.0;
                me.image_speed = 0.5;
            }
            if self.endtimer == 8.0 {
                g.destroy_self(me);
            }
        }
        self.siner += 1.0;
        me.y = me.ystart + (self.siner / 16.0).sin() * 8.0;
        if self.drawtrail {
            self.shadowtimer += 1.0;
            if self.shadowtimer % 2.0 == 0.0 {
                let ai = g.afterimage(me);
                let gdepth = g.first_inst("obj_growtangle").map(|b| b.depth);
                let (dir, st) = (self.dir, self.shadowtimer);
                if let Some(a) = g.inst_mut(ai) {
                    a.image_alpha = 1.0;
                    a.set_hspeed(dir);
                    if let Some(d) = gdepth {
                        a.depth = d - 1000.0;
                    }
                    a.depth += st;
                }
            }
        }
    }

    obj_vars!(con, timer, siner, dir, fadeaudio, fadeaudio2, shinkafade, leafpitch, endtimer);
}

// ============================================================================ obj_afterimage_grow

/// obj_afterimage_grow: an afterimage that grows and fades.
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
