//! SOUL, bullet board and hero damage: obj_heart, obj_grazebox, obj_growtangle (obj_battlesolid),
//! obj_moveheart, obj_returnheart, obj_heartburst, obj_darkener, obj_shake; scr_damage, scr_dead, scr_moveheart,
//! and the collision events (obj_heart / obj_grazebox vs obj_collidebullet).
//! OWNER: battle-core agent "soul".

use crate::assets::{spr, Spr, NO_SPR};
use crate::battle::controller::BattleController;
use crate::battle::heroes::{self, DmgWriter, Hero};
use crate::battle::KnightEnemy;
use crate::gfx::{merge_color, C_BLACK, C_GREEN, C_LIME, C_WHITE};
use crate::gm;
use crate::input::Key;
use crate::obj_vars;
use crate::rt::{Afterimage, Game, Id, Inst, Object, NOONE};

// ============================================================================ obj_heart

/// obj_heart (the SOUL). `x,y` is its top-left (sprite origin 0,0); centre is x+10,y+10.
/// (Only the red SOUL is ported: `color` modes other than 0 — the yellow shooter — are dropped.)
pub struct Heart {
    pub wspeed: f64,
    pub fly: f64,
    pub darken: f64,
    pub darkamt: f64,
    pub dmgnoise: f64,
    pub canmove: f64,
    pub boundaryup: f64,
    pub color: f64,
    pub disableslow: f64,
    pub siner: f64,
    pub remove_slow_z_buffer: f64,
}
impl Default for Heart {
    fn default() -> Self {
        Heart {
            wspeed: 4.0,
            fly: 0.0,
            darken: 1.0,
            darkamt: 0.0,
            dmgnoise: 0.0,
            canmove: 1.0,
            boundaryup: 0.0,
            color: 0.0,
            disableslow: 0.0,
            siner: 0.0,
            remove_slow_z_buffer: 40.0,
        }
    }
}

impl Object for Heart {
    fn name(&self) -> &'static str { "obj_heart" }

    // obj_heart Create_0
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        g.glob.sp = 4.0;
        self.wspeed = g.glob.sp;
        me.image_speed = 0.0;
        self.fly = 0.0;
        self.darken = 1.0;
        self.darkamt = 0.0;
        self.dmgnoise = 0.0;
        self.canmove = 1.0;
        g.instance_create(me.x + 10.0, me.y + 10.0, Box::new(Grazebox::default()));
        self.boundaryup = 0.0;
        self.color = 0.0;
        self.siner = 0.0;
        self.disableslow = 0.0;
        if g.input.held(Key::B2) {
            self.disableslow = 1.0;
        }
        self.remove_slow_z_buffer = 40.0;
    }

    // obj_heart Step_0
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        const SOLID: &str = "obj_battlesolid";
        let mut press_l = 0.0;
        let mut press_r = 0.0;
        let mut press_d = 0.0;
        let mut press_u = 0.0;
        if g.input.held(Key::Left) {
            press_l = 1.0;
        }
        if g.input.held(Key::Right) {
            press_r = 1.0;
        }
        if g.input.held(Key::Up) {
            press_u = 1.0;
        }
        if g.input.held(Key::Down) {
            press_d = 1.0;
        }
        let mut px = 0.0;
        let mut py = 0.0;
        if self.canmove != 0.0 {
            if press_r == 1.0 {
                px = self.wspeed;
            }
            if press_l == 1.0 {
                px = -self.wspeed;
            }
            if press_d == 1.0 {
                py = self.wspeed;
            }
            if press_u == 1.0 {
                py = -self.wspeed;
            }
            if g.input.held(Key::B2) && g.glob.flag(22) == 0.0 {
                if self.disableslow == 0.0 {
                    px = (px * 0.5).ceil();
                    py = (py * 0.5).ceil();
                }
            } else {
                self.disableslow = 0.0;
            }
        }
        self.remove_slow_z_buffer += 0.5;

        // (xmeet / ymeet / xymeet are written but never read in GML)
        if g.place_meeting(me, me.x + px, me.y, SOLID) {
            // slide around corners vertically
            let mut gg = self.wspeed;
            while gg > 0.0 {
                if press_d == 0.0 && !g.place_meeting(me, me.x + px, me.y - gg, SOLID) {
                    me.y -= gg;
                    py = 0.0;
                    break;
                }
                if press_u == 0.0 && !g.place_meeting(me, me.x + px, me.y + gg, SOLID) {
                    me.y += gg;
                    py = 0.0;
                    break;
                }
                gg -= 1.0;
            }
            let mut bkx = 0.0;
            if px > 0.0 {
                let mut i = px;
                while i >= 0.0 {
                    if !g.place_meeting(me, me.x + i, me.y, SOLID) {
                        px = i;
                        bkx = 1.0;
                        break;
                    }
                    i -= 1.0;
                }
            }
            if px < 0.0 {
                let mut i = px;
                while i <= 0.0 {
                    if !g.place_meeting(me, me.x + i, me.y, SOLID) {
                        px = i;
                        bkx = 1.0;
                        break;
                    }
                    i += 1.0;
                }
            }
            if bkx == 0.0 {
                px = 0.0;
            }
        }
        if g.place_meeting(me, me.x, me.y + py, SOLID) {
            let mut bky = 0.0;
            let mut gg = self.wspeed;
            while gg > 0.0 {
                if press_r == 0.0 && !g.place_meeting(me, me.x - gg, me.y + py, SOLID) {
                    me.x -= gg;
                    px = 0.0;
                    break;
                }
                if press_l == 0.0 && !g.place_meeting(me, me.x + gg, me.y + py, SOLID) {
                    me.x += gg;
                    px = 0.0;
                    break;
                }
                gg -= 1.0;
            }
            if py > 0.0 {
                let mut i = py;
                while i >= 0.0 {
                    if !g.place_meeting(me, me.x, me.y + i, SOLID) {
                        py = i;
                        bky = 1.0;
                        break;
                    }
                    i -= 1.0;
                }
            }
            if py < 0.0 {
                let mut i = py;
                while i <= 0.0 {
                    if !g.place_meeting(me, me.x, me.y + i, SOLID) {
                        py = i;
                        bky = 1.0;
                        break;
                    }
                    i += 1.0;
                }
            }
            if bky == 0.0 {
                py = 0.0;
            }
        }
        if g.place_meeting(me, me.x + px, me.y + py, SOLID) {
            let mut bkxy = 0.0;
            let mut i = px;
            let mut j = py;
            while j != 0.0 || i != 0.0 {
                if !g.place_meeting(me, me.x + i, me.y + j, SOLID) {
                    px = i;
                    py = j;
                    bkxy = 1.0;
                    break;
                }
                if j.abs() >= 1.0 {
                    if j > 0.0 {
                        j -= 1.0;
                    }
                    if j < 0.0 {
                        j += 1.0;
                    }
                } else {
                    j = 0.0;
                }
                if i.abs() >= 1.0 {
                    if i > 0.0 {
                        i -= 1.0;
                    }
                    if i < 0.0 {
                        i += 1.0;
                    }
                } else {
                    i = 0.0;
                }
            }
            if bkxy == 0.0 {
                px = 0.0;
                py = 0.0;
            }
        }
        let (vx, vy) = (g.camerax(), g.cameray());
        let sw = g.sprite_width(me);
        let sh = g.sprite_height(me);
        if me.x + px >= vx + 640.0 - sw {
            px = vx + 640.0 - sw - me.x;
        }
        if me.x + px <= 0.0 {
            px = -me.x;
        }
        if me.y + py <= 0.0 {
            py = -me.y;
        }
        if me.y + py >= vy + 320.0 - sh + self.boundaryup {
            py = (vy + 320.0 - sh - me.y) + self.boundaryup;
        }
        me.x += px;
        me.y += py;
        if self.dmgnoise == 1.0 {
            self.dmgnoise = 0.0;
            g.snd_stop("snd_hurt1");
            g.snd_play("snd_hurt1");
        }
        g.glob.inv -= 1.0;
        if g.glob.inv > 0.0 {
            me.image_speed = 0.25;
        } else {
            me.image_speed = 0.0;
            me.image_index = 0.0;
        }
        g.glob.heartx = me.x + 2.0 - vx;
        g.glob.hearty = me.y + 2.0 - vy;
    }

    // obj_heart Draw_0 (red SOUL: just draw_self). CleanUp_0 only concerns the yellow SOUL's charge sound.
    fn draw(&mut self, me: &mut Inst, g: &mut Game) { g.draw_self(me); }

    obj_vars!(wspeed, canmove, color, boundaryup, disableslow, dmgnoise, darken, darkamt, fly, siner);
}

// ============================================================================ obj_grazebox

/// obj_grazebox
#[derive(Default)]
pub struct Grazebox {
    pub grazetimer: f64,
    pub grazetpfactor: f64,
    pub grazetimefactor: f64,
    pub grazesizefactor: f64,
    pub grazecount: f64,
}

/// scr_armorcheck_equipped(char, armor)
pub fn scr_armorcheck_equipped(g: &Game, ch: usize, armor: f64) -> f64 {
    let mut w = 0.0;
    if g.glob.chararmor1[ch] == armor {
        w += 1.0;
    }
    if g.glob.chararmor2[ch] == armor {
        w += 1.0;
    }
    w
}
/// scr_armorcheck_equipped_party(armor)
pub fn scr_armorcheck_equipped_party(g: &Game, armor: f64) -> f64 {
    let mut t = 0.0;
    for i in 0..3 {
        if g.glob.char[i] != 0 {
            t += scr_armorcheck_equipped(g, g.glob.char[i], armor);
        }
    }
    t
}

impl Object for Grazebox {
    fn name(&self) -> &'static str { "obj_grazebox" }

    // obj_grazebox Create_0
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.grazetimer = 0.0;
        self.grazetpfactor = 1.0;
        self.grazetimefactor = 1.0;
        self.grazesizefactor = 1.0;
        self.grazecount = 0.0;
        self.grazetpfactor += scr_armorcheck_equipped_party(g, 15.0) * 0.1;
        self.grazetpfactor += scr_armorcheck_equipped_party(g, 24.0) * 0.05;
        self.grazetpfactor -= scr_armorcheck_equipped_party(g, 3.0) * 0.2;
        self.grazetpfactor -= scr_armorcheck_equipped_party(g, 9.0) * 0.25;
        self.grazetimefactor += scr_armorcheck_equipped_party(g, 14.0) * 0.1;
        self.grazetimefactor -= scr_armorcheck_equipped_party(g, 3.0) * 0.2;
        self.grazetimefactor -= scr_armorcheck_equipped_party(g, 9.0) * 0.25;
        self.grazesizefactor += scr_armorcheck_equipped_party(g, 3.0) * 0.2;
        self.grazesizefactor += scr_armorcheck_equipped_party(g, 9.0) * 0.25;
        if self.grazetimefactor > 3.0 {
            self.grazetimefactor = 3.0;
        }
        if self.grazetpfactor > 3.0 {
            self.grazetpfactor = 3.0;
        }
        if self.grazesizefactor > 3.0 {
            self.grazesizefactor = 3.0;
        }
        me.image_xscale = self.grazesizefactor;
        me.image_yscale = self.grazesizefactor;
        // (when created from obj_heart's Create the heart is still executing and is found via ids_of)
        if let Some(h) = g.first_inst("obj_heart") {
            let (hx, hy) = (h.x, h.y);
            me.x = hx + 10.0;
            me.y = hy + 10.0;
        }
    }

    // obj_grazebox Step_2
    fn end_step(&mut self, me: &mut Inst, g: &mut Game) {
        if g.exists("obj_heart") {
            if let Some(h) = g.first_inst("obj_heart") {
                let (hx, hy) = (h.x, h.y);
                me.x = hx + 10.0;
                me.y = hy + 10.0;
            }
        } else {
            g.destroy_self(me);
        }
    }

    // obj_grazebox Draw_0
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.grazetimer > 0.0 {
            let a = self.grazetimer / 6.0;
            g.draw_sprite_ext(me.sprite_index, 0.0, me.x, me.y, 1.0, 1.0, 0.0, C_WHITE, a);
            g.draw_sprite_ext(me.sprite_index, 3.0, me.x, me.y, 1.0, 1.0, 0.0, C_WHITE, a - 0.2);
            if me.image_xscale > 1.0 {
                g.draw_sprite_ext(me.sprite_index, 0.0, me.x, me.y, me.image_xscale, me.image_yscale, 0.0, C_WHITE, a);
                g.draw_sprite_ext(me.sprite_index, 3.0, me.x, me.y, me.image_xscale, me.image_yscale, 0.0, C_WHITE, a - 0.2);
            }
        }
        self.grazetimer -= 1.0;
    }

    obj_vars!(grazetimer, grazetpfactor, grazetimefactor, grazesizefactor, grazecount);
}

// ============================================================================ obj_growtangle

/// obj_growtangle (the bullet board). Parent obj_battlesolid.
pub struct Growtangle {
    pub timer: f64,
    pub maxtimer: f64,
    pub growcon: f64,
    pub target_angle: f64,
    pub fullgrow: f64,
    pub keep: f64,
    pub megakeep: f64,
    pub maxxscale: f64,
    pub maxyscale: f64,
    /// GML `customBox`
    pub custom_box: bool,
    pub init: bool,
    pub spr_custom_box: Spr,
    pub sizer: f64,
    pub growscale: f64,
    pub growth: f64,
}
impl Default for Growtangle {
    fn default() -> Self {
        Growtangle {
            timer: 0.0,
            maxtimer: 15.0,
            growcon: 1.0,
            target_angle: 0.0,
            fullgrow: 0.0,
            keep: 0.0,
            megakeep: 0.0,
            maxxscale: 2.0,
            maxyscale: 2.0,
            custom_box: false,
            init: false,
            spr_custom_box: NO_SPR,
            sizer: 0.0,
            growscale: 2.0,
            growth: 0.0,
        }
    }
}

impl Object for Growtangle {
    fn name(&self) -> &'static str { "obj_growtangle" }

    // obj_growtangle Create_0
    fn create(&mut self, me: &mut Inst, _g: &mut Game) {
        me.image_xscale = 0.0;
        me.image_yscale = 0.0;
        me.image_alpha = 0.3;
        self.timer = 0.0;
        self.maxtimer = 15.0;
        self.growcon = 1.0;
        me.image_speed = 0.0;
        me.image_blend = merge_color(C_GREEN, C_LIME, 0.5);
        self.target_angle = 0.0;
        self.fullgrow = 0.0;
        self.keep = 0.0;
        self.megakeep = 0.0;
        self.maxxscale = 2.0;
        self.maxyscale = 2.0;
        self.custom_box = false;
        self.init = false;
        self.spr_custom_box = me.sprite_index;
        self.sizer = 0.0;
        self.growscale = 2.0;
    }

    // obj_growtangle Step_0
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if !self.init {
            if me.visible && (self.maxxscale != 2.0 || self.maxyscale != 2.0) && me.sprite_index == spr("spr_battlebg_0") {
                self.custom_box = true;
                me.sprite_index = spr("spr_battlebg_stretch_hitbox");
                if self.maxxscale % 2.0 != 0.0 {
                    self.maxxscale = gm::round(self.maxxscale * 37.5) / 37.5;
                }
                if self.maxyscale % 2.0 != 0.0 {
                    self.maxyscale = gm::round(self.maxyscale * 37.5) / 37.5;
                }
                me.image_xscale = self.maxxscale / 2.0;
                me.image_yscale = self.maxyscale / 2.0;
                let sw = g.sprite_width(me);
                let sh = g.sprite_height(me);
                let sxo = g.sprite_get_xoffset(me.sprite_index) * me.image_xscale;
                let syo = g.sprite_get_yoffset(me.sprite_index) * me.image_yscale;
                let surf = g.gfx.surface_create(sw as i32, sh as i32);
                g.gfx.surface_set_target(surf);
                g.gfx.draw_clear_alpha(C_BLACK, 0.0);
                g.draw_sprite_ext(spr("spr_battlebg_stretch"), 0.0, sxo, syo, me.image_xscale, me.image_yscale, 0.0, C_WHITE, 1.0);
                // (GML creates the sprite while the surface is still the target; the result is identical)
                g.gfx.surface_reset_target();
                self.spr_custom_box = g.sprite_create_from_surface(surf, 0.0, 0.0, sw, sh, sxo, syo);
                me.sprite_index = spr("spr_battlebg_stretch_hitbox");
                g.gfx.surface_free(surf);
            }
            self.init = true;
        }
        self.growth = 0.0;
        if self.timer < self.maxtimer && self.growcon == 1.0 {
            self.growth = 1.0;
        }
        if self.timer > 0.0 && self.growcon == 3.0 {
            self.growth = 1.0;
        }
        if self.growth == 1.0 {
            if self.growcon == 1.0 {
                self.timer += 1.0;
            }
            if self.growcon == 3.0 {
                self.timer -= 1.0;
            }
            let frac = if self.maxtimer != 0.0 { self.timer / self.maxtimer } else { 1.0 };
            self.sizer = frac;
            me.image_xscale = self.maxxscale * self.sizer;
            me.image_yscale = self.maxyscale * self.sizer;
            me.image_angle = 180.0 + 180.0 * frac + self.target_angle;
            me.image_alpha = 0.5 + frac * 0.5;
            if me.visible {
                let d = g.instance_create(me.x, me.y, Box::new(Afterimage::default()));
                let scale = self.sizer * self.growscale;
                if let Some(a) = g.inst_mut(d) {
                    a.sprite_index = self.spr_custom_box;
                    a.image_xscale = scale;
                    a.image_yscale = scale;
                    a.image_angle = me.image_angle;
                    a.depth = me.depth - 1.0;
                    a.image_blend = me.image_blend;
                    a.image_alpha = (1.0 - me.image_alpha) + 0.1;
                    a.image_speed = 0.0;
                }
            }
            if self.timer >= self.maxtimer && self.growcon == 1.0 {
                self.growcon = 2.0;
                me.image_angle = self.target_angle;
            }
            if self.timer <= 0.0 && self.growcon == 3.0 {
                g.destroy_self(me);
            }
        }
    }

    // obj_growtangle Step_2: keep the SOUL inside a moving box
    fn end_step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.keep == 1.0 && g.exists("obj_heart") {
            // (path_speed is always 0 here)
            if me.speed() != 0.0 || self.megakeep == 1.0 {
                let sw = g.sprite_width(me);
                let sh = g.sprite_height(me);
                let lborder = me.x - sw / 2.0;
                let rborder = me.x + sw / 2.0;
                let uborder = me.y - sh / 2.0;
                let dborder = me.y + sh / 2.0;
                if let Some(h) = g.first_inst("obj_heart") {
                    if h.x < lborder + 5.0 {
                        h.x = lborder + 5.0;
                    }
                    if h.x > rborder - 22.0 {
                        h.x = rborder - 22.0;
                    }
                    if h.y < uborder + 5.0 {
                        h.y = uborder + 5.0;
                    }
                    if h.y > dborder - 22.0 {
                        h.y = dborder - 22.0;
                    }
                }
            }
        }
    }

    // obj_growtangle Draw_0
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        g.draw_sprite_ext(me.sprite_index, 1.0, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, me.image_alpha);
        if self.custom_box && self.growth != 0.0 && self.growcon != 2.0 {
            let scale = self.sizer * self.growscale;
            g.draw_sprite_ext(self.spr_custom_box, 0.0, me.x, me.y, scale, scale, me.image_angle, me.image_blend, me.image_alpha);
        } else {
            g.draw_self(me);
        }
    }

    // obj_growtangle CleanUp_0
    fn cleanup(&mut self, me: &mut Inst, g: &mut Game) {
        if self.custom_box {
            let d = g.instance_create(me.x, me.y, Box::new(BattleCleanup::default()));
            if let Some((_, c)) = g.get::<BattleCleanup>(d) {
                c.custom_box_sprite = self.spr_custom_box;
            }
        }
    }

    obj_vars!(maxxscale, maxyscale, growcon, target_angle, timer, maxtimer, keep, megakeep, sizer, growscale, growth, fullgrow);
}

/// obj_battle_cleanup: frees obj_growtangle's custom box sprite once its afterimages are gone.
pub struct BattleCleanup {
    pub custom_box_sprite: Spr,
}
impl Default for BattleCleanup {
    fn default() -> Self { BattleCleanup { custom_box_sprite: NO_SPR } }
}
impl BattleCleanup {
    fn free(&mut self, me: &mut Inst, g: &mut Game) {
        if self.custom_box_sprite >= 0 {
            g.sprite_delete(self.custom_box_sprite);
            self.custom_box_sprite = NO_SPR;
        }
        g.destroy_self(me);
    }
}
impl Object for BattleCleanup {
    fn name(&self) -> &'static str { "obj_battle_cleanup" }
    fn create(&mut self, me: &mut Inst, _g: &mut Game) { me.alarm[0] = 30; }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if !g.exists("obj_afterimage") {
            self.free(me, g);
        }
    }
    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 0 {
            self.free(me, g);
        }
    }
    fn draw(&mut self, _: &mut Inst, _: &mut Game) {}
    obj_vars!();
}

// ============================================================================ obj_moveheart / obj_returnheart / obj_heartburst

/// obj_moveheart: the SOUL flying from Kris into the box; becomes obj_heart after `flytime` frames.
#[derive(Default)]
pub struct MoveHeart {
    pub burst: f64,
    pub shift: f64,
    pub flytime: f64,
    pub distx: f64,
    pub disty: f64,
    pub dist: f64,
}
impl Object for MoveHeart {
    fn name(&self) -> &'static str { "obj_moveheart" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.burst = 0.0;
        self.shift = 1.0;
        me.image_alpha = 0.0;
        self.flytime = 8.0;
        if let Some(m) = g.first_inst("obj_heartmarker") {
            self.distx = m.x;
            self.disty = m.y;
        } else if !g.exists("obj_growtangle") {
            self.distx = g.camerax() + 310.0;
            self.disty = g.cameray() + 160.0;
        } else if let Some(b) = g.first_inst("obj_growtangle") {
            self.distx = b.x - 10.0;
            self.disty = b.y - 10.0;
        }
        self.dist = gm::point_distance(me.x, me.y, self.distx, self.disty);
        me.move_towards_point(self.distx, self.disty, self.dist / self.flytime);
        me.alarm[0] = self.flytime as i32;
        me.image_speed = 0.0;
        g.instance_create(me.x, me.y, Box::new(HeartBurst::default()));
    }
    fn step(&mut self, me: &mut Inst, _g: &mut Game) { me.image_alpha += 0.334; }
    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 0 {
            return;
        }
        me.x = self.distx;
        me.y = self.disty;
        if !g.exists("obj_heart") {
            let heart = g.instance_create(me.x, me.y, Box::new(Heart::default()));
            if let Some(h) = g.inst_mut(heart) {
                h.sprite_index = me.sprite_index;
                // NOTE: obj_moveheart has no mask, so this sets the SOUL's mask to "same as sprite"
                // (spr_dodgeheart), exactly as the game does.
                h.mask_index = me.mask_index;
            }
        }
        g.destroy_self(me);
    }
    obj_vars!(burst, shift, flytime, distx, disty, dist);
}

/// obj_returnheart: the SOUL flying back to Kris at the end of the enemy turn.
#[derive(Default)]
pub struct ReturnHeart {
    pub burst: f64,
    pub shift: f64,
    pub flytime: f64,
    pub distx: f64,
    pub disty: f64,
    pub dist: f64,
}
impl Object for ReturnHeart {
    fn name(&self) -> &'static str { "obj_returnheart" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.burst = 0.0;
        self.shift = 1.0;
        me.image_alpha = 1.0;
        self.flytime = 8.0;
        let (kx, ky) = g.first_inst("obj_herokris").map(|k| (k.x, k.y)).unwrap_or((0.0, 0.0));
        self.distx = kx + 10.0;
        self.disty = ky + 40.0;
        self.dist = gm::point_distance(me.x, me.y, self.distx, self.disty);
        me.move_towards_point(self.distx, self.disty, self.dist / self.flytime);
        me.alarm[0] = self.flytime as i32;
        me.image_speed = 0.0;
    }
    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 0 {
            return;
        }
        me.x = self.distx;
        me.y = self.disty;
        g.instance_create(me.x, me.y, Box::new(HeartBurst::default()));
        g.destroy_self(me);
    }
    obj_vars!(burst, shift, flytime, distx, disty, dist);
}

/// obj_heartburst: the outline burst when the SOUL appears / returns.
#[derive(Default)]
pub struct HeartBurst {
    pub burst: f64,
}
impl Object for HeartBurst {
    fn name(&self) -> &'static str { "obj_heartburst" }
    fn create(&mut self, _me: &mut Inst, _g: &mut Game) { self.burst = 0.0; }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        self.burst += 1.0;
        let b = self.burst;
        let (x, y) = (me.xstart + 9.0, me.ystart + 9.0);
        g.draw_sprite_ext(spr("spr_heartoutline2"), 0.0, x, y, 0.25 + b, 0.25 + b / 2.0, 0.0, C_WHITE, 0.8 - b / 6.0);
        g.draw_sprite_ext(spr("spr_heartoutline"), 0.0, x, y, 0.25 + b / 1.5, 0.25 + b / 3.0, 0.0, C_WHITE, 1.0 - b / 6.0);
        g.draw_sprite_ext(spr("spr_heartoutline"), 0.0, x, y, 0.2 + b / 2.5, 0.2 + b / 5.0, 0.0, C_WHITE, 1.2 - b / 6.0);
        if self.burst > 10.0 {
            g.destroy_self(me);
        }
    }
    obj_vars!(burst);
}

// ============================================================================ obj_darkener

/// obj_darkener (dims the battle background during the enemy turn)
#[derive(Default)]
pub struct Darkener {
    pub darken: f64,
    pub darkamt: f64,
}
impl Object for Darkener {
    fn name(&self) -> &'static str { "obj_darkener" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        me.depth = 205.0;
        if g.instance_number("obj_darkener") > 1 {
            g.destroy_self(me);
        }
        self.darken = 1.0;
        self.darkamt = 0.0;
    }
    fn destroy(&mut self, _me: &mut Inst, g: &mut Game) {
        for id in g.ids_of("obj_whiteedge") {
            if let Some(w) = g.inst_mut(id) {
                w.image_alpha = 0.0;
            }
        }
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.darken == 1.0 {
            for id in g.ids_of("obj_heroparent") {
                if let Some((_, h)) = g.get::<Hero>(id) {
                    h.darkify = 1.0;
                }
            }
            // (Tenna's darkamt < 8 branch dropped)
            if self.darkamt < 15.0 {
                self.darkamt += 1.0;
            }
            self.whiteedge_alpha(g);
        }
        if self.darken == 0.0 {
            for id in g.ids_of("obj_growtangle") {
                if let Some((_, b)) = g.get::<Growtangle>(id) {
                    b.growcon = 3.0;
                }
            }
            for id in g.ids_of("obj_heroparent") {
                if let Some((_, h)) = g.get::<Hero>(id) {
                    h.darkify = 0.0;
                }
            }
            if self.darkamt > 0.0 {
                self.darkamt -= 1.0;
            }
            self.whiteedge_alpha(g);
            if self.darkamt <= 0.0 {
                g.destroy_self(me);
            }
        }
        let (vx, vy) = (g.camerax(), g.cameray());
        g.gfx.draw_set_alpha(self.darkamt / 20.0);
        g.gfx.draw_set_color(C_BLACK);
        g.gfx.draw_rectangle(vx - 40.0, vy - 40.0, vx + 680.0, vy + 520.0, false);
        g.gfx.draw_set_alpha(1.0);
    }
    obj_vars!(darken, darkamt);
}
impl Darkener {
    fn whiteedge_alpha(&self, g: &mut Game) {
        let a = self.darkamt / 15.0;
        for id in g.ids_of("obj_whiteedge") {
            if let Some(w) = g.inst_mut(id) {
                w.image_alpha = a;
            }
        }
    }
}

// ============================================================================ obj_shake

/// obj_shake (screen shake). GML `active` is stored in `me.active`.
pub struct Shake {
    pub shakex: f64,
    pub shakey: f64,
    pub shakespeed: f64,
    pub camera: f64,
    pub shakesign: f64,
    pub siner: f64,
    pub permashake: f64,
    pub beenset: f64,
    pub mycamerax: f64,
    pub mycameray: f64,
}
impl Default for Shake {
    fn default() -> Self {
        Shake {
            shakex: 4.0,
            shakey: 4.0,
            shakespeed: 1.0,
            camera: 0.0,
            shakesign: 1.0,
            siner: 0.0,
            permashake: 0.0,
            beenset: 0.0,
            mycamerax: 0.0,
            mycameray: 0.0,
        }
    }
}
impl Object for Shake {
    fn name(&self) -> &'static str { "obj_shake" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.camera = 0.0;
        self.shakespeed = 1.0;
        self.shakesign = 1.0;
        self.shakex = 4.0;
        self.shakey = 4.0;
        self.siner = 0.0;
        me.active = 0.0;
        self.permashake = 0.0;
        self.beenset = 0.0;
        if g.instance_number("obj_shake") >= 2 {
            me.active = -1.0;
            g.destroy_self(me);
        }
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if me.active == 0.0 {
            self.beenset = 1.0;
            self.mycamerax = g.gfx.view_x;
            self.mycameray = g.gfx.view_y;
            if g.glob.flag(12) == 0.0 {
                g.gfx.view_x = self.mycamerax + self.shakex;
                g.gfx.view_y = self.mycameray + self.shakey;
            }
            self.shakesign = -self.shakesign;
            me.active = 1.0;
            me.alarm[0] = self.shakespeed as i32;
        }
    }
    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 0 {
            return;
        }
        if g.glob.flag(12) == 0.0 {
            g.gfx.view_x = self.mycamerax + self.shakex * self.shakesign;
            g.gfx.view_y = self.mycameray + self.shakey * self.shakesign;
        }
        if self.permashake == 0.0 {
            if self.shakex > 0.0 {
                self.shakex -= 1.0;
            }
            if self.shakey > 0.0 {
                self.shakey -= 1.0;
            }
        }
        self.shakesign = -self.shakesign;
        me.alarm[0] = self.shakespeed as i32;
        if self.shakex == 0.0 && self.shakey == 0.0 {
            g.destroy_self(me);
        }
    }
    fn destroy(&mut self, _me: &mut Inst, g: &mut Game) {
        if self.beenset != 0.0 {
            g.gfx.view_x = self.mycamerax;
            g.gfx.view_y = self.mycameray;
        }
    }
    fn draw(&mut self, _: &mut Inst, _: &mut Game) {}
    obj_vars!(shakex, shakey, shakespeed, shakesign, permashake);
}

// ============================================================================ collisions

/// The game-specific collision pass (obj_heart and obj_grazebox Collision events), run after motion.
/// GameMaker order: instances in creation order (the SOUL, then its grazebox, which the SOUL creates).
pub fn collision_pass(g: &mut Game) {
    // obj_heart Collision obj_collidebullet: with (other) event_user(5);
    for h in g.ids_of("obj_heart") {
        let hits = g.with(h, |me, _, g| g.instance_place_list(me, me.x, me.y, "obj_collidebullet")).unwrap_or_default();
        for b in hits {
            if !g.id_exists(h) {
                break;
            }
            if g.id_exists(b) {
                g.user_event(b, 5);
            }
        }
    }
    // obj_grazebox Collision obj_collidebullet
    for gb in g.ids_of("obj_grazebox") {
        let hits = g.with(gb, |me, _, g| g.instance_place_list(me, me.x, me.y, "obj_collidebullet")).unwrap_or_default();
        for b in hits {
            if !g.id_exists(gb) {
                break;
            }
            grazebox_collide(g, gb, b);
        }
    }
}

fn grazebox_collide(g: &mut Game, gb: Id, b: Id) {
    let Some(bi) = g.inst(b) else { return };
    let (active, obj, grazed, grazepoints, timepoints) = (bi.active, bi.object, bi.grazed, bi.grazepoints, bi.timepoints);
    if !(active > 0.5) && obj != "obj_sword_tunnel_sword" {
        return;
    }
    let Some((_, me)) = g.get::<Grazebox>(gb) else { return };
    let (tpf, tif) = (me.grazetpfactor, me.grazetimefactor);
    if g.glob.inv < 0.0 {
        if grazed == 1.0 {
            crate::battle::scr_tensionheal(g, (grazepoints / 30.0) * tpf);
            if g.glob.turntimer >= 10.0 {
                g.glob.turntimer -= (timepoints / 30.0) * tif;
            }
            for id in g.ids_of("obj_grazebox") {
                if let Some((_, z)) = g.get::<Grazebox>(id) {
                    if z.grazetimer >= 0.0 && z.grazetimer < 4.0 {
                        z.grazetimer = 3.0;
                    }
                    if z.grazetimer < 2.0 {
                        z.grazetimer = 2.0;
                    }
                }
            }
        }
        if grazed == 0.0 {
            for id in g.ids_of("obj_grazebox") {
                if let Some((_, z)) = g.get::<Grazebox>(id) {
                    z.grazecount += 1.0;
                    // (Tenna's grazed10bullets counter dropped)
                }
            }
            if let Some(bi) = g.inst_mut(b) {
                bi.grazed = 1.0;
            }
            crate::battle::scr_tensionheal(g, grazepoints * tpf);
            if g.glob.turntimer >= 10.0 {
                g.glob.turntimer -= timepoints * tif;
            }
            for id in g.ids_of("obj_battlecontroller") {
                if let Some((_, c)) = g.get::<BattleController>(id) {
                    c.grazenoise = 1.0;
                }
            }
            for id in g.ids_of("obj_grazebox") {
                if let Some((_, z)) = g.get::<Grazebox>(id) {
                    z.grazetimer = 10.0;
                }
            }
        }
    }
}

// ============================================================================ damage

/// scr_element_damage_reduction(element, char). The only elemental armor reachable here is
/// armor 23 ShadowMantle (element 5, amount 0.66); Amber Card / GlowWrist have none.
pub fn scr_element_damage_reduction(g: &Game, element: f64, ch: usize) -> f64 {
    let armor_element = |a: f64| -> (f64, f64) { if a == 23.0 { (5.0, 0.66) } else { (0.0, 0.0) } };
    let mut red = 1.0;
    if element != 0.0 {
        for a in [g.glob.chararmor1[ch], g.glob.chararmor2[ch]] {
            let (el, amt) = armor_element(a);
            if el != 0.0 {
                if el == element {
                    red -= amt;
                }
                if el == 9.0 && (element == 2.0 || element == 8.0) {
                    red -= amt;
                }
                if el == 10.0 {
                    red -= amt;
                }
            }
        }
    }
    if red < 0.25 {
        red = 0.25;
    }
    red
}

/// scr_randomtarget_old(): returns `mytarget` (0-2, or 3 if nobody can be targeted).
fn scr_randomtarget_old(g: &mut Game) -> f64 {
    let ct = g.glob.charcantarget;
    let able = !(ct[0] == 0.0 && ct[1] == 0.0 && ct[2] == 0.0);
    let mut t: f64 = g.choose(&[0.0, 1.0, 2.0]);
    if able {
        while g.glob.charcantarget[t as usize] == 0.0 {
            t = g.choose(&[0.0, 1.0, 2.0]);
        }
    } else {
        t = 3.0;
    }
    if t < 3.0 {
        g.glob.targeted[t as usize] = 1.0;
    }
    t
}

/// scr_party_hpaverage()
fn scr_party_hpaverage(g: &Game) -> f64 {
    let (mut hp, mut maxhp) = (0.0, 0.0);
    for i in 0..3 {
        let c = g.glob.char[i];
        if c > 0 {
            hp += g.glob.hp[c];
            maxhp += g.glob.maxhp[c];
        }
    }
    if hp > 0.0 { (hp / maxhp).floor() } else { 0.0 }
}

fn hp_ratio(g: &Game, slot: f64) -> f64 {
    if !(0.0..3.0).contains(&slot) {
        return 1.0;
    }
    let c = g.glob.char[slot as usize];
    g.glob.hp[c] / g.glob.maxhp[c]
}

/// with (global.charinstance[slot]) { image_blend = c_white; darkify = 0; }
fn undarken_hero(g: &mut Game, slot: f64) {
    if !(0.0..3.0).contains(&slot) {
        return;
    }
    let id = g.glob.charinstance[slot as usize];
    if let Some((i, h)) = g.get::<Hero>(id) {
        i.image_blend = C_WHITE;
        h.darkify = 0.0;
    }
}

fn wears_mantle(g: &Game, ch: usize) -> bool { g.glob.chararmor1[ch] == 23.0 || g.glob.chararmor2[ch] == 23.0 }

/// scr_damage(): see battle::scr_damage. `me` is the bullet.
pub fn scr_damage_impl(me: &mut Inst, g: &mut Game) {
    if g.glob.inv >= 0.0 {
        return;
    }
    // scr_damage_cache() / scr_damage_check() only feed obj_event_manager (achievements): dropped.
    let knight = g.first("obj_knight_enemy");
    for id in g.ids_of("obj_knight_enemy") {
        if let Some((_, k)) = g.get::<KnightEnemy>(id) {
            k.progamer = false;
        }
    }
    let element = me.element;
    if me.target < 3.0 && g.glob.slot_hp(me.target.max(0.0) as usize) <= 0.0 {
        me.target = scr_randomtarget_old(g);
        undarken_hero(g, me.target);
    }
    let mut remtarget = -1.0;
    if me.target == 4.0 {
        remtarget = 4.0;
        me.target = scr_randomtarget_old(g);
        if hp_ratio(g, me.target) < scr_party_hpaverage(g) / 2.0 {
            me.target = scr_randomtarget_old(g);
        }
        if hp_ratio(g, me.target) < scr_party_hpaverage(g) / 2.0 {
            me.target = scr_randomtarget_old(g);
        }
        if me.target == 0.0 && hp_ratio(g, me.target) < 0.35 {
            me.target = scr_randomtarget_old(g);
        }
        undarken_hero(g, me.target);
    }
    let truedamage = g.exists("obj_knight_roaring2");
    let mut chartarget: usize;

    if let (Some(kid), false) = (knight, truedamage) {
        let (aoe, choice) = g.get::<KnightEnemy>(kid).map(|(_, k)| (k.aoedamage, k.myattackchoice)).unwrap_or((true, 0.0));
        if !aoe {
            if me.target == 0.0 {
                if g.glob.hp[2] > 0.0 && g.glob.hp[3] > 0.0 {
                    me.target = g.choose(&[1.0, 2.0]);
                } else if g.glob.hp[2] > 0.0 {
                    me.target = 1.0;
                } else if g.glob.hp[3] > 0.0 {
                    me.target = 2.0;
                }
            }
            if choice != 13.0 && (wears_mantle(g, 1) || wears_mantle(g, 2) || wears_mantle(g, 3)) {
                let counter = g
                    .get::<KnightEnemy>(kid)
                    .map(|(_, k)| {
                        k.damagecounter += 1.0;
                        k.damagecounter
                    })
                    .unwrap_or(0.0);
                if counter < 3.0 {
                    if g.glob.hp[1] > 0.0 && wears_mantle(g, 1) {
                        me.target = 0.0;
                    }
                    if g.glob.hp[2] > 0.0 && wears_mantle(g, 2) {
                        me.target = 1.0;
                    }
                    if g.glob.hp[3] > 0.0 && wears_mantle(g, 3) {
                        me.target = 2.0;
                    }
                } else {
                    me.target = g.choose(&[0.0, 1.0, 2.0]);
                    for _ in 0..2 {
                        if g.glob.hp[1] < 1.0 && me.target == 0.0 {
                            me.target += 1.0;
                        }
                        if g.glob.hp[2] < 1.0 && me.target == 1.0 {
                            me.target += 1.0;
                        }
                        if g.glob.hp[3] < 1.0 && me.target == 2.0 {
                            me.target += 1.0;
                        }
                        if me.target > 2.0 {
                            me.target = 0.0;
                        }
                    }
                    // (operator precedence exactly as in the GML)
                    let a = &g.glob;
                    let t = me.target;
                    let keep = (t == 0.0 && a.chararmor1[1] == 23.0)
                        || a.chararmor2[1] == 23.0
                        || (t == 1.0 && a.chararmor1[2] == 23.0)
                        || a.chararmor2[2] == 23.0
                        || (t == 2.0 && a.chararmor1[3] == 23.0)
                        || a.chararmor2[3] == 23.0;
                    if !keep {
                        if let Some((_, k)) = g.get::<KnightEnemy>(kid) {
                            k.damagecounter = 0.0;
                        }
                    }
                }
            }
        }
    }
    // (Rouxls / Tenna targeting overrides dropped)

    let mut tdamage = me.damage;
    let mut shadowmantlereduction = false;
    if me.target < 3.0 {
        let t = me.target as usize;
        if !truedamage {
            tdamage = crate::battle::scr_damage_calculation(g, tdamage, t);
        }
        chartarget = g.glob.char[t];
        if knight.is_some() && !truedamage {
            for (ch, slot) in [(1, 0), (2, 1), (3, 2)] {
                if wears_mantle(g, ch) && t == slot {
                    tdamage = gm::round(tdamage * 0.33);
                    shadowmantlereduction = true;
                }
            }
        }
        if !truedamage {
            if g.glob.charaction[t] == 10.0 {
                tdamage = ((2.0 * tdamage) / 3.0).ceil();
            }
            if !shadowmantlereduction {
                tdamage = (tdamage * scr_element_damage_reduction(g, element, g.glob.char[t])).ceil();
            }
        }
        if tdamage < 1.0 {
            tdamage = 1.0;
        }
    } else {
        chartarget = 3;
    }
    if !g.exists("obj_shake") {
        g.instance_create(0.0, 0.0, Box::new(Shake::default()));
    }
    if me.target < 3.0 {
        let id = g.glob.charinstance[me.target as usize];
        if let Some((_, h)) = g.get::<Hero>(id) {
            h.hurt = 1.0;
            h.hurttimer = 0.0;
        }
    }
    let mut hpdiff = tdamage;
    for id in g.ids_of("obj_dmgwriter") {
        if let Some((_, w)) = g.get::<DmgWriter>(id) {
            if w.delaytimer >= 1.0 {
                w.killactive = 1.0;
            }
        }
    }
    let mut doomtype = -1.0;
    for id in g.ids_of("obj_heart") {
        if let Some((_, h)) = g.get::<Heart>(id) {
            h.dmgnoise = 1.0;
        }
    }
    if me.target < 3.0 {
        let t = me.target as usize;
        if g.glob.hp[chartarget] <= 0.0 {
            doomtype = 4.0;
            g.glob.hp[chartarget] -= gm::round(tdamage / 4.0);
            hpdiff = gm::round(tdamage / 4.0);
        } else {
            if let Some(kid) = knight {
                let (choice, diff) = g.get::<KnightEnemy>(kid).map(|(_, k)| (k.myattackchoice, k.difficulty)).unwrap_or((0.0, 0.0));
                if choice == 2.0 && (diff == 1.0 || diff == 3.0) {
                    tdamage = gm::round(tdamage * 0.66);
                    hpdiff = tdamage;
                }
            }
            g.glob.hp[chartarget] -= tdamage;
            if g.glob.hp[chartarget] <= 0.0 {
                let maxhp = g.glob.maxhp[chartarget];
                if knight.is_some() && t != 0 {
                    // the Knight kills Susie / Ralsei outright
                    doomtype = 12.0;
                    hpdiff = gm::round(g.glob.hp[chartarget] + 999.0);
                    g.glob.hp[chartarget] = -999.0;
                    scr_dead(g, t);
                } else {
                    hpdiff = (g.glob.hp[chartarget] - maxhp / 2.0).abs();
                    doomtype = 4.0;
                    g.glob.hp[chartarget] = gm::round(-maxhp / 2.0);
                    scr_dead(g, t);
                }
            }
        }
        let hid = g.glob.charinstance[t];
        let pos = g.get::<Hero>(hid).map(|(i, h)| (i.x, i.y + h.myheight - 24.0));
        if let Some((wx, wy)) = pos {
            let w = heroes::dmgwriter(g, wx, wy);
            if let Some((_, dw)) = g.get::<DmgWriter>(w) {
                dw.damage = hpdiff;
                dw.typ = doomtype;
            }
        }
    }
    if me.target == 3.0 {
        for hpi in 0..3 {
            chartarget = g.glob.char[hpi];
            if g.glob.hp[chartarget] >= 0.0 {
                // (tdamage accumulates across members, as in the GML)
                tdamage = crate::battle::scr_damage_calculation(g, tdamage, hpi);
                if knight.is_some() {
                    for (ch, slot) in [(1, 0), (2, 1), (3, 2)] {
                        if wears_mantle(g, ch) && hpi == slot {
                            tdamage = gm::round(tdamage * 0.33);
                            shadowmantlereduction = true;
                        }
                    }
                }
                if !shadowmantlereduction {
                    tdamage = (tdamage * scr_element_damage_reduction(g, element, chartarget)).ceil();
                }
                if g.glob.charaction[hpi] == 10.0 {
                    g.glob.hp[chartarget] -= ((3.0 * tdamage) / 4.0).ceil();
                } else {
                    g.glob.hp[chartarget] -= tdamage;
                }
                if g.glob.hp[chartarget] <= 0.0 {
                    // (global.maxhp[0] in the GML, i.e. 0)
                    g.glob.hp[chartarget] = gm::round(-g.glob.maxhp[0] / 2.0);
                }
            }
        }
    }
    g.glob.inv = g.glob.invc * 30.0;
    // gameover check: always 0 while obj_knight_enemy exists (chapter 3 rule)
    if knight.is_none() {
        let alive = (0..3).any(|i| g.glob.char[i] != 0 && g.glob.hp[g.glob.char[i]] > 0.0);
        if !alive {
            // TODO: scr_gameover() has no port; only reachable once the Knight instance is gone.
            crate::log("scr_damage: party defeated (scr_gameover not ported)");
        }
    }
    if remtarget != -1.0 {
        me.target = remtarget;
    }
}

/// scr_dead(slot)
pub fn scr_dead(g: &mut Game, slot: usize) {
    g.glob.charmove[slot] = 0.0;
    g.glob.charcantarget[slot] = 0.0;
    g.glob.chardead[slot] = 1.0;
    g.glob.charaction[slot] = 0.0;
    g.glob.charspecial[slot] = 0.0;
}

/// scr_moveheart(): the SOUL flies from Kris into the board. Returns the obj_moveheart id.
pub fn scr_moveheart(g: &mut Game) -> Id {
    g.glob.inv = 0.0;
    let Some((kx, ky)) = g.first_inst("obj_herokris").map(|k| (k.x, k.y)) else {
        return g.instance_create(10.0, 40.0, Box::new(MoveHeart::default()));
    };
    g.instance_create(kx + 10.0, ky + 40.0, Box::new(MoveHeart::default()))
}

/// Convenience for obj_battlecontroller's end of the bullet phase:
/// `with (obj_heart) { instance_create(x, y, obj_returnheart); instance_destroy(); }`.
/// Returns the (last) obj_returnheart id, or NOONE if there was no SOUL.
pub fn spawn_returnheart(g: &mut Game) -> Id {
    let mut r = NOONE;
    for h in g.ids_of("obj_heart") {
        if let Some((x, y)) = g.inst(h).map(|i| (i.x, i.y)) {
            r = g.instance_create(x, y, Box::new(ReturnHeart::default()));
            g.destroy(h);
        }
    }
    r
}
