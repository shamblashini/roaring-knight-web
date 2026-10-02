//! The Knight's "rotating slash" attack (myattackchoice 5 and 16, obj_dbulletcontroller type 104).
//!
//! Ported objects:
//! * `obj_knight_rotating_slash` (Create, Step, Draw, Alarm 1/2/3, CleanUp, Other_10) — the Knight double that
//!   aims rotating slash markers at the SOUL and then cuts along them.
//! * `obj_roaringknight_slash` (parent obj_collidebullet: Create, Alarm 0/1, Draw, End Step, Other_15) — the
//!   actual damaging cut.
//! * `obj_knight_circle` (Create, Step, Draw, CleanUp) — the red flash circle when a marker appears.
//! * `obj_particle_generic` (Create, Step, Alarm 0, Other_40) — slash marks / debris.
//! * `obj_afterimage_screen` (Create, Step, Draw, Draw End, CleanUp) — whole-screen afterimage of the final flurry.
//! * `obj_knight_warp` (Create, Step, Alarm 0/1, Other_10/11) — teleport puff (only used by combination turns).
//!
//! Helper scripts ported locally: scr_darksize, scr_heartclamp, scr_bulletparent_count, ds_list_shuffle, mean.

use crate::assets::spr;
use crate::battle::{self, DbCtrl, KnightEnemy};
use crate::gfx::{bm, make_color_rgb, Color, C_BLACK, C_RED, C_WHITE};
use crate::gm::*;
use crate::obj_vars;
use crate::rt::{Game, Id, Inst, LerpVar, Object, NOONE};

/// obj_dbulletcontroller Step_0, `if (type == 104)` block.
pub fn ctrl_step(c: &mut DbCtrl, me: &mut Inst, g: &mut Game) {
    if c.made == 0.0 {
        g.glob.turntimer = 999999.0;
        c.made = 1.0;
        let (kx, ky) = match g.inst_mut(c.creatorid) {
            Some(k) => {
                k.image_alpha = 0.0;
                (k.x, k.y)
            }
            None => (me.x, me.y),
        };
        let id = g.instance_create(kx, ky, Box::new(KnightRotatingSlash::default()));
        if let Some((_, o)) = g.get::<KnightRotatingSlash>(id) {
            o.difficulty = c.difficulty;
        }
        g.user_event(id, 0);
        battle::scr_bullet_inherit(me, id, g);
        // GML scr_bullet_inherit also copies creatorid/creator when called from obj_dbulletcontroller;
        // battle::scr_bullet_inherit doesn't, so do it here.
        if let Some((_, o)) = g.get::<KnightRotatingSlash>(id) {
            o.creatorid = c.creatorid;
            o.creator = c.creator;
        }
    }
}

// ============================================================================ helpers

fn bx(g: &mut Game, n: i32) -> f64 { battle::scr_get_box(g, n) }

/// scr_darksize()
fn scr_darksize(me: &mut Inst) {
    me.image_xscale = 2.0;
    me.image_yscale = 2.0;
}

/// mean(a, b)
fn mean(a: f64, b: f64) -> f64 { (a + b) / 2.0 }

/// scr_heartclamp(0, 0)
fn scr_heartclamp(g: &mut Game) {
    let Some(gt) = g.first("obj_growtangle") else { return };
    let Some((gxs, gys)) = g.inst(gt).map(|b| (b.image_xscale, b.image_yscale)) else { return };
    for hid in g.ids_of("obj_heart") {
        let xthick = gxs * 2.0 + 1.0;
        let ythick = gys * 2.0 + 1.0;
        let (b0, b1, b2, b3) = (bx(g, 0), bx(g, 1), bx(g, 2), bx(g, 3));
        if let Some(h) = g.inst_mut(hid) {
            h.x = clamp(h.x, b2 + xthick, b0 - (20.0 + xthick));
            h.y = clamp(h.y, b1 + ythick, b3 - (20.0 + ythick));
        }
    }
}

/// scr_bulletparent_count(): instances whose object_index is exactly obj_bulletparent.
fn scr_bulletparent_count(g: &mut Game) -> usize {
    let ids = g.ids_of("obj_bulletparent");
    ids.into_iter().filter(|&id| g.inst(id).map(|i| i.object == "obj_bulletparent").unwrap_or(false)).count()
}

/// ds_list_shuffle
fn ds_list_shuffle(g: &mut Game, list: &mut [f64]) {
    let n = list.len();
    if n < 2 {
        return;
    }
    for i in (1..n).rev() {
        let j = g.irandom(i as f64) as usize;
        list.swap(i, j);
    }
}

fn set_sprite_delayed(g: &mut Game, target: Id, s: &'static str, delay: i32) {
    g.script_delayed(target, delay, move |g| {
        if let Some(i) = g.inst_mut(target) {
            i.sprite_index = spr(s);
        }
    });
}

// ============================================================================ obj_knight_rotating_slash

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum St {
    #[default]
    Intro,
    Aim,
    Slash,
    Cooldown,
    Return,
}

/// obj_knight_rotating_slash (parent obj_bulletparent)
#[derive(Default)]
pub struct KnightRotatingSlash {
    pub difficulty: f64,
    pub slash_number: f64,
    pub slashes_done: bool,
    pub rotation: f64,
    pub rotation_base: f64,
    pub rotation_change: f64,
    pub rotation_goal: f64,
    pub timer: f64,
    state: St,
    /// "full", "start", "end", "short start", "short mid", "short end"
    pub turn_type: &'static str,
    pub turn_segment: f64,
    pub next_up: f64,
    pub next_next_up: f64,
    pub local_turntimer: f64,
    pub aim_direction: f64,
    pub spin: f64,
    pub random_offset: f64,
    pub slash_array: [f64; 6],
    pub slash_counter: f64,
    pub final_counter: f64,
    pub slash_base: f64,
    pub slash_offset: f64,
    pub speed_gain: f64,
    pub cooldown_time: f64,
    pub slash_timer: f64,
    pub aim_type: f64,
    pub anchor_x: f64,
    pub anchor_y: f64,
    pub aim_x: f64,
    pub aim_y: f64,
    /// GML r / g / b
    pub cr: f64,
    pub cg: f64,
    pub cb: f64,
    pub line_width: f64,
    slash_list: Vec<f64>,
    pub movebox_x: f64,
    pub movebox_y: f64,
    pub line2: f64,
    pub line3: f64,
    pub do_final: bool,
    my_surface: i32,
    pub turn_limit_4: f64,
    pub debug: bool,
    pub circle_size: f64,
    pub creatorid: Id,
    pub creator: f64,
}

impl KnightRotatingSlash {
    /// Creates one obj_roaringknight_slash (+ its slash-mark particles) along `dir`.
    fn make_slash(&mut self, me: &Inst, g: &mut Game, dir: f64) {
        let slashid = g.instance_create(self.aim_x, self.aim_y, Box::new(RoaringknightSlash::default()));
        let (sx, sy, sang) = match g.get::<RoaringknightSlash>(slashid) {
            Some((si, so)) => {
                si.set_direction(dir);
                si.image_xscale = 2.0;
                si.image_angle = si.direction();
                si.visible = false;
                so.width *= 2.0;
                so.aoe = 1.0;
                (si.x, si.y, si.image_angle)
            }
            None => (self.aim_x, self.aim_y, dir),
        };
        let markscalex = 5.0 + g.random(3.0);
        let markscaley = 2.0 + g.random(1.0);
        // red mark
        {
            let p = g.instance_create(sx, sy, Box::new(ParticleGeneric::default()));
            if let Some(pi) = g.inst_mut(p) {
                pi.depth += 100.0;
                pi.sprite_index = spr("spr_knight_slash_mark");
                pi.image_blend = C_RED;
                pi.image_angle = sang;
                pi.image_xscale = markscalex;
                pi.image_yscale = markscaley;
            }
            g.lerpvar_instance(p, "image_xscale", markscalex, 0.0, 4.0, 0, "out");
            g.lerpvar_instance(p, "image_yscale", markscaley, 0.0, 4.0, 0, "out");
            g.doom(p, 4);
        }
        // black mark (sic: both scales use markscaley)
        {
            let p = g.instance_create(sx, sy, Box::new(ParticleGeneric::default()));
            let s = markscaley * 0.85;
            if let Some(pi) = g.inst_mut(p) {
                pi.depth += 99.0;
                pi.sprite_index = spr("spr_knight_slash_mark");
                pi.image_blend = C_BLACK;
                pi.image_angle = sang;
                pi.image_xscale = s;
                pi.image_yscale = s;
            }
            g.lerpvar_instance(p, "image_xscale", s, 0.0, 4.0, 0, "out");
            g.lerpvar_instance(p, "image_yscale", s, 0.0, 4.0, 0, "out");
            g.doom(p, 4);
        }
        // debris on both sides
        for side in [0.0, 180.0] {
            let n = 4.0 + g.irandom(3.0);
            let mut k = 0.0;
            while k < n {
                k += 1.0;
                let faade = 12.0 + g.irandom(8.0);
                let diist = -20.0 + g.random(60.0);
                let p = g.instance_create(sx, sy, Box::new(ParticleGeneric::default()));
                let spd = 6.0 + g.random(4.0);
                let ang = sang + side + g.random_range(-10.0, 10.0);
                let (mut x, mut y) = (sx, sy);
                for _ in 0..50 {
                    x += lengthdir_x(5.0, ang);
                    y += lengthdir_y(5.0, ang);
                    if x < bx(g, 2) || x > bx(g, 0) || y < bx(g, 1) || y > bx(g, 3) {
                        break;
                    }
                }
                x += lengthdir_x(diist, ang);
                y += lengthdir_y(diist, ang);
                if let Some(pi) = g.inst_mut(p) {
                    pi.set_speed(spd);
                    pi.image_angle = ang;
                    pi.set_direction(ang);
                    pi.x = x;
                    pi.y = y;
                    pi.depth += 101.0;
                    pi.sprite_index = spr("spr_knight_slash_mark");
                    pi.image_blend = C_RED;
                    pi.image_xscale = 0.3;
                    pi.image_yscale = 0.15;
                }
                g.lerpvar_instance(p, "image_xscale", 0.3, 0.0, faade, 0, "out");
                g.lerpvar_instance(p, "image_yscale", 0.15, 0.0, faade, 0, "out");
                g.doom(p, faade as i32);
            }
        }
        battle::scr_bullet_inherit(me, slashid, g);
        g.user_event(slashid, 0);
    }

    fn lerp_to_movebox(&mut self, me: &Inst, g: &mut Game) {
        let t = (self.slash_base + self.slash_offset) - 8.0;
        let tx = (bx(g, 0) - 20.0) + self.movebox_x;
        g.lerpvar(me, "x", me.x, tx, t, 1, "out");
        let ty = (bx(g, 1) - 20.0) + self.movebox_y;
        g.lerpvar(me, "y", me.y, ty, t, 1, "out");
    }

    fn step_movebox(&mut self, g: &mut Game) {
        self.movebox_x += 20.0 + g.irandom(40.0);
        self.movebox_y += 30.0 + g.irandom(60.0);
    }
}

impl Object for KnightRotatingSlash {
    fn name(&self) -> &'static str { "obj_knight_rotating_slash" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::bullet_init(me);
        scr_darksize(me);
        me.image_speed = 0.0;
        self.difficulty = 2.0;
        self.slash_number = 1.0;
        self.slashes_done = false;
        self.rotation = 16.0;
        self.rotation_base = 16.0;
        self.rotation_change = 1.0;
        self.rotation_goal = 2.0;
        self.timer = 0.0;
        self.state = St::Intro;
        self.turn_type = "full";
        self.turn_segment = -1.0;
        self.next_up = -1.0;
        self.next_next_up = -1.0;
        self.local_turntimer = 0.0;
        self.aim_direction = 0.0;
        self.spin = g.choose(&[-1.0, 1.0]);
        self.random_offset = g.irandom(360.0);
        self.slash_array = [1.0, 2.0, 2.0, 3.0, 3.0, 4.0];
        self.slash_counter = 0.0;
        self.final_counter = 0.0;
        self.slash_base = 18.0;
        self.slash_offset = 6.0;
        self.speed_gain = 16.0;
        self.cooldown_time = 6.0;
        self.slash_timer = 8.0;
        self.aim_type = 0.0;
        self.anchor_x = me.x;
        self.anchor_y = me.y;
        self.aim_x = me.x;
        self.aim_y = me.y;
        self.cr = 0.0;
        self.cg = 0.0;
        self.cb = 0.0;
        self.line_width = 4.0;
        self.slash_list = Vec::new();
        self.movebox_x = 40.0;
        self.movebox_y = 60.0;
        self.line2 = -1.0;
        self.line3 = -1.0;
        self.do_final = true;
        self.my_surface = -4;
        self.turn_limit_4 = 270.0;
        self.debug = false;
        self.creatorid = NOONE;
    }

    /// Other_10
    fn user(&mut self, n: usize, _me: &mut Inst, _g: &mut Game) {
        if n != 0 {
            return;
        }
        if self.difficulty == 1.0 {
            self.slash_offset = 6.0;
            self.slash_number = 3.0;
            self.slash_array = [2.0, 3.0, 4.0, 4.0, 4.0, 4.0];
        }
        if self.difficulty == 2.0 {
            self.slash_offset = 0.0;
            self.slash_number = 3.0;
            self.slash_array = [3.0, 4.0, 4.0, 4.0, 4.0, 4.0];
        }
        match self.turn_type {
            "full" => self.local_turntimer = 400.0,
            "start" => self.local_turntimer = 320.0,
            "end" => {
                self.local_turntimer = 300.0;
                self.timer = 15.0;
            }
            "short start" => {
                self.local_turntimer = 270.0;
                self.timer = 12.0;
                self.turn_limit_4 = 250.0;
            }
            "short mid" => {
                self.local_turntimer = 260.0;
                self.timer = 15.0;
                self.turn_limit_4 = 250.0;
            }
            "short end" => {
                self.local_turntimer = 260.0;
                self.timer = 15.0;
            }
            _ => {}
        }
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        let mut kdepth = None;
        if let Some((ki, k)) = g.get_first::<KnightEnemy>("obj_knight_enemy") {
            k.siner2 = 0.0;
            self.anchor_x = ki.x;
            self.anchor_y = ki.y;
            kdepth = Some(ki.depth);
        }
        self.local_turntimer -= 1.0;

        // Follow-up attacks of obj_knight_combinations turns (type 105). Never reached from type 104
        // (next_up stays -1 there).
        if self.local_turntimer < 240.0 && self.next_up == 1.0 {
            // TODO(combinations): instance_create(obj_knight_enemy.x, obj_knight_enemy.y, obj_roaringknight_quickslash_attack),
            // scr_bullet_inherit, creatorid/creator, obj_knight_warp (x+50, y-44, event_user(0)), turn_type "end" /
            // "short mid" (turn_segment 1, next_up = next_next_up) / "short end" (turn_segment 2), anchor_x/y,
            // event_user(0), timer = spawn_speed.
            self.next_up = -999.0;
        }
        if self.local_turntimer < 220.0 && self.next_up == 3.0 {
            // TODO(combinations): obj_knight_tunnel_slasher_2_revised (warp x+25, y-44; "short mid" sets timer = -8;
            // segment 1 -> "short mid" segment 2; for short mid/end: timer = -12, local_turntimer += 12).
            self.next_up = -999.0;
        }
        if self.local_turntimer < self.turn_limit_4 && self.next_up == 4.0 {
            // TODO(combinations): obj_knight_swordfall (warp x-60, y-44), same turn_type chaining as above.
            self.next_up = -999.0;
        }
        if self.local_turntimer < self.turn_limit_4 && self.next_up == 5.0 {
            // TODO(combinations): obj_knight_weird_bottom_manager (no warp), turn_type chaining, event_user(0),
            // init_start = 4, init = 8.
            self.next_up = -999.0;
        }

        if gmod(g.glob.time, 4.0) == 0.0 && me.image_alpha != 0.0 {
            let fade = g.afterimage(me);
            let cdepth = g.inst(self.creatorid).map(|c| c.depth);
            if let Some(f) = g.inst_mut(fade) {
                if let Some(d) = cdepth {
                    f.depth = d + 1.0;
                }
                if let Some(d) = kdepth {
                    f.depth = d + 1.0;
                }
                f.image_alpha = 0.6;
                f.set_hspeed(4.0);
            }
            g.set_var(fade, "fadeSpeed", 0.04);
        }
        if self.line2 > -1.0 {
            self.line2 += 1.0;
            self.line2 %= 8.0;
        }
        if self.line3 > -1.0 {
            self.line3 += 1.0;
            self.line3 %= 8.0;
        }
        if me.image_index >= 5.0 && self.aim_type != 2.0 {
            me.image_index = 5.0;
            me.image_speed = 0.0;
        }

        if self.state == St::Intro {
            self.timer += 1.0;
            if self.timer > 16.0 {
                self.state = St::Aim;
                self.timer = 0.0;
            }
        }

        if self.state == St::Aim {
            self.timer += 1.0;
            if self.timer == 1.0 {
                g.snd_stop("snd_knight_rotatingslash_line");
                g.snd_loop("snd_knight_rotatingslash_line");
                self.rotation = self.rotation_base;
                self.spin = g.choose(&[-1.0, 1.0]);
                self.circle_size = 0.0;
                self.cr = 128.0;
                self.cg = 128.0;
                self.cb = 128.0;
                self.step_movebox(g);
                if self.movebox_x > 80.0 {
                    self.movebox_x -= 80.0;
                }
                if self.movebox_y > 120.0 {
                    self.movebox_y -= 120.0;
                }
                if self.aim_type != 2.0 {
                    me.image_index = 1.0;
                } else {
                    me.sprite_index = spr("spr_roaringknight_flurry_prepare");
                    me.image_index = 0.0;
                }
                self.lerp_to_movebox(me, g);
            }
            self.aim_direction += self.rotation * self.spin;
            self.rotation = scr_approach(self.rotation, self.rotation_goal, self.rotation_change);
            if self.timer == 1.0 {
                if self.aim_type == 0.0 {
                    if let Some(h) = g.first_inst("obj_heart") {
                        self.aim_x = h.x + 10.0;
                        self.aim_y = h.y + 10.0;
                    }
                }
                g.instance_create(self.aim_x, self.aim_y, Box::new(KnightCircle::default()));
            } else {
                self.cr = scr_approach(self.cr, 255.0, 9.142857142857142);
                self.cg = scr_approach(self.cg, 0.0, 9.142857142857142);
                self.cb = scr_approach(self.cb, 0.0, 9.142857142857142);
            }
            if self.timer == floor((self.slash_base + self.slash_offset) * 0.5) && self.aim_type != 2.0 {
                me.image_index += 1.0;
            }
            if self.timer == self.slash_base + self.slash_offset {
                if self.aim_type != 2.0 {
                    me.image_speed = 0.5;
                } else {
                    set_sprite_delayed(g, me.id, "spr_roaringknight_flurry", 4);
                    g.var_delayed(me.id, "image_speed", 1.0, 4);
                }
            }
            if self.timer == self.slash_base + 6.0 + self.slash_offset {
                self.state = St::Slash;
                self.timer = 0.0;
            }
        }

        if self.state == St::Slash {
            self.timer += 1.0;
            if self.timer == 1.0 {
                g.snd_stop("snd_knight_rotatingslash_line");
                self.slash_list.clear();
                let mut a = 0.0;
                while a < self.slash_number {
                    self.slash_list.push(((360.0 / (self.slash_number * 2.0)) * a) + self.random_offset + self.aim_direction);
                    a += 1.0;
                }
                let mut list = std::mem::take(&mut self.slash_list);
                ds_list_shuffle(g, &mut list);
                self.slash_list = list;
                g.snd_play("snd_knight_cut");
                g.snd_play("snd_explosion_firework");
            }
            if self.timer - 1.0 < self.slash_list.len() as f64 {
                let dir = self.slash_list[(self.timer - 1.0) as usize];
                self.make_slash(me, g, dir);
            }
            if self.timer == self.slash_timer {
                self.state = St::Cooldown;
                self.timer = 0.0;
            }
        }

        if self.state == St::Cooldown {
            self.timer += 1.0;
            if self.timer == self.cooldown_time || self.local_turntimer < 200.0 {
                self.slash_counter += 1.0;
                if self.slash_counter < self.slash_array.len() as f64 {
                    self.slash_number = self.slash_array[self.slash_counter as usize];
                    self.slash_offset = scr_approach(self.slash_offset, 0.0, 6.0);
                    self.slash_base = scr_approach(self.slash_base, 15.0, 1.0);
                }
                if self.local_turntimer < 200.0 && !self.slashes_done {
                    self.slashes_done = true;
                    self.local_turntimer = 99999.0;
                }
                if self.slashes_done {
                    if self.difficulty == 2.0 && self.turn_type == "full" {
                        if self.do_final {
                            g.snd_play("snd_knight_puff");
                            g.snd_play_x("snd_knight_teleport", 1.0, 0.5);
                            self.rotation_base = 18.0;
                            self.rotation_change = 0.5;
                            self.line_width = 4.0;
                            let cx = mean(bx(g, 2), bx(g, 0));
                            let cy = mean(bx(g, 1), bx(g, 3));
                            let s = g.instance_create(cx, cy, Box::new(AfterimageScreen::default()));
                            if let Some((_, o)) = g.get::<AfterimageScreen>(s) {
                                o.faderate = 0.05;
                                o.draw_end_flag = true;
                            }
                            self.slash_number = 1.0;
                            self.slash_base = 24.0;
                            self.cooldown_time = 2.0;
                            self.slash_timer = 2.0;
                            self.aim_type = scr_approach(self.aim_type, 2.0, 1.0);
                            self.do_final = false;
                            self.aim_x = mean(bx(g, 2), bx(g, 0));
                            self.aim_y = mean(bx(g, 1), bx(g, 3));
                        }
                    } else {
                        if self.turn_type == "start" || self.turn_type == "short start" || self.turn_type == "short mid" {
                            let w = g.instance_create_depth(me.x, me.y, me.depth, Box::new(KnightWarp::default()));
                            if let Some((_, wo)) = g.get::<KnightWarp>(w) {
                                wo.master = me.id;
                            }
                            g.user_event(w, 1);
                            // the warp's event_user(1) sets master.image_alpha = 0, but master (us) is
                            // executing and can't be reached through `g`; apply it here.
                            me.image_alpha = 0.0;
                            g.var_delayed(me.id, "image_index", 4.0, 2);
                            g.var_delayed(me.id, "image_index", 0.0, 4);
                            me.alarm[2] = 4;
                            return;
                        }
                        self.debug = true;
                        self.state = St::Return;
                        self.timer = 0.0;
                        g.var_delayed(me.id, "image_index", 0.0, 8);
                        let id = me.id;
                        let (x0, ax) = (me.x, self.anchor_x);
                        g.script_delayed(id, 8, move |g| {
                            g.lerpvar_instance(id, "x", x0, ax, 12.0, 1, "out");
                        });
                        let (y0, ay) = (me.y, self.anchor_y);
                        g.script_delayed(id, 8, move |g| {
                            g.lerpvar_instance(id, "y", y0, ay, 12.0, 1, "out");
                        });
                        me.alarm[3] = 22;
                        return;
                    }
                }
                if self.aim_type < 2.0 {
                    self.state = St::Aim;
                    self.timer = 0.0;
                    if self.aim_type == 1.0 {
                        self.line2 = 0.0;
                        me.alarm[1] = 4;
                        self.aim_type = scr_approach(self.aim_type, 2.0, 1.0);
                        return;
                    }
                }
                if self.aim_type == 2.0 {
                    self.state = St::Slash;
                    self.timer = 0.0;
                    self.aim_direction += self.speed_gain * self.spin;
                    self.speed_gain = scr_approach(self.speed_gain, 24.0, 1.0);
                    self.final_counter += 1.0;
                    if self.final_counter == 28.0 {
                        self.state = St::Return;
                        me.sprite_index = spr("spr_roaringknight_attack_ol");
                        me.image_index = 0.0;
                        me.image_speed = 0.0;
                        for lid in g.ids_of("obj_lerpvar") {
                            let mine = g.get::<LerpVar>(lid).map(|(_, l)| l.target == me.id).unwrap_or(false);
                            if mine {
                                g.destroy(lid);
                            }
                        }
                        g.lerpvar(me, "x", me.x, self.anchor_x, 12.0, 1, "out");
                        g.lerpvar(me, "y", me.y, self.anchor_y, 12.0, 1, "out");
                        me.alarm[3] = 22;
                        self.timer = 0.0;
                    } else {
                        self.step_movebox(g);
                        me.sprite_index = spr("spr_roaringknight_flurry");
                        me.image_speed = 1.0;
                        if self.movebox_x > 80.0 {
                            self.movebox_x -= 80.0;
                        }
                        if self.movebox_y > 120.0 {
                            self.movebox_y -= 120.0;
                        }
                        self.lerp_to_movebox(me, g);
                    }
                }
            }
        }
    }

    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            1 => self.line3 = 0.0,
            2 => {
                if self.next_up != -999.0 {
                    // TODO(combinations): create the next attack of an obj_knight_combinations turn at (x, y):
                    // next_up 1 = obj_roaringknight_quickslash_attack (timer = spawn_speed), 3 =
                    // obj_knight_tunnel_slasher_2_revised, 4 = obj_knight_swordfall, 5 = obj_knight_weird_bottom_manager;
                    // scr_bullet_inherit, creatorid/creator, turn_type "end" / "short mid" (turn_segment 1,
                    // next_up = next_next_up) / "short end" (turn_segment 2), anchor_x/y, event_user(0).
                    // Unreachable from type 104 (turn_type is always "full" there).
                }
                g.destroy_self(me);
            }
            3 => g.destroy_self(me),
            _ => {}
        }
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        self.slash_list.clear();
        if g.gfx.surface_exists(self.my_surface) {
            g.gfx.surface_free(self.my_surface);
        }
        if self.turn_type != "start" && self.turn_type != "short start" && self.turn_type != "short mid" && scr_bulletparent_count(g) < 2 {
            for k in g.ids_of("obj_knight_enemy") {
                if let Some(ki) = g.inst_mut(k) {
                    ki.image_alpha = 1.0;
                }
            }
            g.glob.turntimer = -1.0;
        }
        g.snd_stop("snd_knight_rotatingslash_line");
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let sw = bx(g, 0) - bx(g, 2) - 8.0;
        let sh = bx(g, 3) - bx(g, 1) - 8.0;
        if !g.gfx.surface_exists(self.my_surface) {
            self.my_surface = g.gfx.surface_create(sw as i32, sh as i32);
        }
        if g.gfx.surface_exists(self.my_surface) {
            // surface_resize every frame in GML; only reallocate when the size actually changes
            // (the surface is cleared right after, so this is equivalent).
            if g.gfx.surface_get_width(self.my_surface) != (sw as i32).max(1) as f64
                || g.gfx.surface_get_height(self.my_surface) != (sh as i32).max(1) as f64
            {
                g.gfx.surface_resize(self.my_surface, (sw as i32).max(1), (sh as i32).max(1));
            }
        }
        g.gfx.surface_set_target(self.my_surface);
        g.gfx.draw_clear_alpha(C_BLACK, 0.0);
        let ox = bx(g, 2) + 5.0;
        let oy = bx(g, 1) + 5.0;
        if self.state == St::Aim && truthy(self.timer) {
            let (ax, ay) = (self.aim_x - ox, self.aim_y - oy);
            let xs = self.timer * 0.2;
            let ys = 1.0 + 2.0 * (1.0 - self.timer / (self.slash_base + 6.0 + self.slash_offset));
            let color = make_color_rgb(self.cr, self.cg, self.cb);
            let mut a = 0.0;
            while a < self.slash_number {
                let dir = ((360.0 / (self.slash_number * 2.0)) * a) + self.random_offset + self.aim_direction;
                g.draw_sprite_ext(spr("spr_rk_quickslash_marker_gradient"), 0.0, ax, ay, xs, ys, dir, color, 1.0);
                a += 1.0;
            }
            let mut a = 0.0;
            while a < self.slash_number {
                let dir = ((360.0 / (self.slash_number * 2.0)) * a) + self.random_offset + self.aim_direction;
                g.draw_sprite_ext(spr("spr_rk_quickslash_marker"), 0.0, ax, ay, xs, ys, dir, C_BLACK, 1.0);
                a += 1.0;
            }
            for line in [self.line2, self.line3] {
                if truthy(line) {
                    g.gfx.draw_set_alpha(1.0 - line / 7.0);
                    let mut a = 0.0;
                    while a < self.slash_number {
                        let dir = ((360.0 / (self.slash_number * 2.0)) * a) + self.random_offset + self.aim_direction;
                        let dirx = lengthdir_x(320.0, dir);
                        let diry = lengthdir_y(320.0, dir);
                        let px = lengthdir_x(line * 6.0, dir + 90.0);
                        let py = lengthdir_y(line * 6.0, dir + 90.0);
                        g.gfx.draw_line_width_color(ax + dirx + px, ay + diry + py, ax - dirx + px, ay - diry + py, self.line_width, color, color);
                        g.gfx.draw_line_width_color(ax + dirx - px, ay + diry - py, ax - dirx - px, ay - diry - py, self.line_width, color, color);
                        a += 1.0;
                    }
                    g.gfx.draw_set_alpha(1.0);
                }
            }
        }
        // with (obj_roaringknight_slash): draw every slash into the box surface
        for sid in g.ids_of("obj_roaringknight_slash") {
            let Some((sx, sy, sdir, salpha)) = g.inst(sid).map(|s| (s.x, s.y, s.direction(), s.image_alpha)) else { continue };
            let width = g.get_var(sid, "width").unwrap_or(0.0);
            let slashdir = g.get_var(sid, "slashdir").unwrap_or(0.0);
            let hx = lengthdir_x(640.0, sdir);
            let hy = lengthdir_y(640.0, sdir);
            let hxoff = lengthdir_x(width, sdir + 90.0);
            let hyoff = lengthdir_y(width, sdir + 90.0);
            let color = make_color_rgb(255.0, (1.0 - salpha) * 255.0, (1.0 - salpha) * 255.0);
            let (x, y) = (sx - ox, sy - oy);
            if truthy(slashdir) {
                g.gfx.draw_triangle_color(x - hx * salpha, y - hy * salpha, x + hx + hxoff, y + hy + hyoff, (x + hx) - hxoff, (y + hy) - hyoff, color, color, color, false);
            } else {
                g.gfx.draw_triangle_color(x + hx * salpha, y + hy * salpha, (x - hx) + hxoff, (y - hy) + hyoff, x - hx - hxoff, y - hy - hyoff, color, color, color, false);
            }
        }
        g.gfx.surface_reset_target();
        g.gfx.draw_surface(self.my_surface, ox, oy);
        let t = g.glob.time;
        g.draw_sprite_ext(me.sprite_index, me.image_index, me.x, me.y + (t * 0.1).sin() * 2.0, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, me.image_alpha);
    }

    obj_vars!(
        difficulty, slash_number, rotation, rotation_base, rotation_change, rotation_goal, timer, turn_segment, next_up,
        next_next_up, local_turntimer, aim_direction, spin, random_offset, slash_counter, final_counter, slash_base,
        slash_offset, speed_gain, cooldown_time, slash_timer, aim_type, anchor_x, anchor_y, aim_x, aim_y, line_width,
        movebox_x, movebox_y, line2, line3, turn_limit_4, circle_size, creator
    );
}

// ============================================================================ obj_roaringknight_slash

/// obj_roaringknight_slash (parent obj_collidebullet)
#[derive(Default)]
pub struct RoaringknightSlash {
    pub width: f64,
    pub aoe: f64,
    pub slashdir: f64,
}

impl Object for RoaringknightSlash {
    fn name(&self) -> &'static str { "obj_roaringknight_slash" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::bullet_init(me);
        // event_inherited(): obj_collidebullet has no Create
        me.active = 1.0;
        me.element = 5.0;
        self.width = 24.0;
        me.grazepoints = 50.0;
        self.aoe = 1.0;
        me.alarm[0] = 1;
        me.alarm[1] = 3;
        me.image_index = 2.0;
        me.image_speed = 0.0;
        me.image_yscale = 0.1;
        self.slashdir = g.choose(&[-1.0, 1.0]);
        me.destroyonhit = 0.0;
    }

    fn alarm(&mut self, n: usize, me: &mut Inst, _g: &mut Game) {
        // Alarm_0: exit (it only exists so that `alarm[0]` counts down)
        if n == 1 {
            me.mask_index = spr("spr_nomask");
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let hx = lengthdir_x(640.0, me.direction());
        let hy = lengthdir_y(640.0, me.direction());
        let hxoff = lengthdir_x(self.width, me.direction() + 90.0);
        let hyoff = lengthdir_y(self.width, me.direction() + 90.0);
        let a = me.image_alpha;
        let color: Color = make_color_rgb(255.0, (1.0 - a) * 255.0, (1.0 - a) * 255.0);
        g.gfx.draw_set_alpha(a * 2.0);
        let (x, y) = (me.x, me.y);
        if truthy(self.slashdir) {
            g.gfx.draw_triangle_color(x - hx * a, y - hy * a, x + hx + hxoff, y + hy + hyoff, (x + hx) - hxoff, (y + hy) - hyoff, color, color, color, false);
        } else {
            g.gfx.draw_triangle_color(x + hx * a, y + hy * a, (x - hx) + hxoff, (y - hy) + hyoff, x - hx - hxoff, y - hy - hyoff, color, color, color, false);
        }
        g.gfx.draw_set_alpha(1.0);
    }

    /// Step_2
    fn end_step(&mut self, me: &mut Inst, g: &mut Game) {
        me.damage = 206.0;
        me.grazepoints = 50.0;
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
            for gt in g.ids_of("obj_growtangle") {
                let ox = g.choose(&[-2.0, -1.0, 0.0, 1.0, 2.0]);
                let oy = g.choose(&[-2.0, -1.0, 0.0, 1.0, 2.0]);
                if let Some(b) = g.inst_mut(gt) {
                    b.x = b.xstart + ox;
                    b.y = b.ystart + oy;
                }
            }
            scr_heartclamp(g);
        }
    }

    /// Other_15 (hit the SOUL)
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 5 {
            return;
        }
        me.damage = 206.0;
        if self.aoe == 1.0 {
            me.damage = 75.0;
            me.target = 3.0;
            for k in g.ids_of("obj_knight_enemy") {
                if let Some((_, ke)) = g.get::<KnightEnemy>(k) {
                    ke.aoedamage = true;
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
            if let Some((_, ke)) = g.get::<KnightEnemy>(k) {
                ke.aoedamage = false;
            }
        }
    }

    obj_vars!(width, aoe, slashdir);
}

// ============================================================================ obj_knight_circle

/// obj_knight_circle
pub struct KnightCircle {
    pub circle_size: f64,
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub r_goal: f64,
    pub g_goal: f64,
    pub b_goal: f64,
    pub fade_time: f64,
    pub size_goal: f64,
    pub growth: f64,
    my_surface: i32,
    pub color_1: Color,
    pub color_2: Color,
    pub draw_in_box: bool,
}

impl Default for KnightCircle {
    fn default() -> Self {
        KnightCircle {
            circle_size: 0.0,
            r: 128.0,
            g: 0.0,
            b: 0.0,
            r_goal: 0.0,
            g_goal: 0.0,
            b_goal: 0.0,
            fade_time: 28.0,
            size_goal: 960.0,
            growth: 40.0,
            my_surface: -4,
            color_1: 0,
            color_2: 255,
            draw_in_box: true,
        }
    }
}

impl Object for KnightCircle {
    fn name(&self) -> &'static str { "obj_knight_circle" }

    fn create(&mut self, _me: &mut Inst, _g: &mut Game) { *self = KnightCircle::default(); }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if !g.exists("obj_knight_roaring_fx") {
            me.image_alpha -= 0.1;
        }
        if me.image_alpha < 0.0 {
            g.destroy_self(me);
        }
        self.g = scr_approach(self.g, self.g_goal, 255.0 / self.fade_time);
        self.b = scr_approach(self.b, self.b_goal, 255.0 / self.fade_time);
        self.circle_size = scr_approach(self.circle_size, self.size_goal, self.growth);
        // (sic) `r == 0 && b == 0 && b == 0`
        if self.r == 0.0 && self.b == 0.0 && self.b == 0.0 {
            g.destroy_self(me);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.draw_in_box {
            if !g.gfx.surface_exists(self.my_surface) {
                let (xs, ys) = g.first_inst("obj_growtangle").map(|b| (b.image_xscale, b.image_yscale)).unwrap_or((1.0, 1.0));
                self.my_surface = g.gfx.surface_create((75.0 * xs) as i32, (75.0 * ys) as i32);
            }
            let (b2, b1) = (bx(g, 2), bx(g, 1));
            g.gfx.surface_set_target(self.my_surface);
            self.color_2 = make_color_rgb(self.r, self.g, self.b);
            g.gfx.draw_circle_color(me.x - b2, me.y - b1, self.circle_size, self.color_1, self.color_2, false);
            g.gfx.surface_reset_target();
            g.gfx.gpu_set_blendmode(bm::ADD);
            g.gfx.draw_surface(self.my_surface, b2, b1);
            g.gfx.gpu_set_blendmode(bm::NORMAL);
        } else {
            self.color_2 = make_color_rgb(self.r, self.g, self.b);
            g.gfx.gpu_set_blendmode(bm::ADD);
            g.gfx.draw_set_alpha(me.image_alpha);
            g.gfx.draw_circle_color(me.x, me.y, self.circle_size, self.color_1, self.color_2, false);
            g.gfx.gpu_set_blendmode(bm::NORMAL);
        }
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.gfx.surface_exists(self.my_surface) {
            g.gfx.surface_free(self.my_surface);
        }
    }

    obj_vars!(circle_size, r, g, b, r_goal, g_goal, b_goal, fade_time, size_goal, growth);
}

// ============================================================================ obj_particle_generic

/// obj_particle_generic
#[derive(Default)]
pub struct ParticleGeneric {
    pub fade_rate: f64,
    pub shrink_rate: f64,
    pub timer: f64,
}

impl Object for ParticleGeneric {
    fn name(&self) -> &'static str { "obj_particle_generic" }

    fn create(&mut self, _me: &mut Inst, _g: &mut Game) {
        self.fade_rate = 0.0;
        self.shrink_rate = 0.0;
        self.timer = -1.0;
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        me.image_alpha = scr_approach(me.image_alpha, 0.0, self.fade_rate);
        me.image_xscale = scr_approach(me.image_xscale, 0.0, self.shrink_rate);
        me.image_yscale = scr_approach(me.image_yscale, 0.0, self.shrink_rate);
        if me.image_xscale == 0.0 || me.image_yscale == 0.0 {
            g.destroy_self(me);
        }
        if me.image_alpha == 0.0 {
            g.destroy_self(me);
        }
        self.timer -= 1.0;
        if self.timer == 0.0 {
            g.destroy_self(me);
        }
    }

    /// Other_40 (Outside View 0) has no runtime event: emulated after motion, in End Step.
    fn end_step(&mut self, me: &mut Inst, g: &mut Game) {
        if let Some(bb) = g.bbox(me) {
            let (vx, vy) = (g.camerax(), g.cameray());
            let (vw, vh) = (g.camerawidth(), g.cameraheight());
            if bb[2] < vx || bb[0] > vx + vw || bb[3] < vy || bb[1] > vy + vh {
                g.destroy_self(me);
            }
        }
    }

    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 0 {
            g.destroy_self(me);
        }
    }

    obj_vars!(fade_rate, shrink_rate, timer);
}

// ============================================================================ obj_afterimage_screen

/// obj_afterimage_screen
pub struct AfterimageScreen {
    my_surface: i32,
    pub xscale: f64,
    pub yscale: f64,
    pub alpha: f64,
    pub anchor_x: f64,
    pub anchor_y: f64,
    pub xrate: f64,
    pub yrate: f64,
    pub faderate: f64,
    /// GML `draw_end`
    pub draw_end_flag: bool,
}

impl Default for AfterimageScreen {
    fn default() -> Self {
        AfterimageScreen {
            my_surface: -4,
            xscale: 1.0,
            yscale: 1.0,
            alpha: 0.5,
            anchor_x: 0.0,
            anchor_y: 0.0,
            xrate: 0.01,
            yrate: 0.01,
            faderate: 0.00625,
            draw_end_flag: false,
        }
    }
}

impl AfterimageScreen {
    fn snapshot_and_draw(&mut self, me: &Inst, g: &mut Game) {
        let app = g.gfx.app_surface;
        if !g.gfx.surface_exists(self.my_surface) {
            let (w, h) = (g.gfx.surface_get_width(app), g.gfx.surface_get_height(app));
            self.my_surface = g.gfx.surface_create(w as i32, h as i32);
        }
        g.gfx.surface_set_target(self.my_surface);
        g.gfx.draw_surface(app, 0.0, 0.0);
        g.gfx.surface_reset_target();
        g.gfx.draw_surface_ext(self.my_surface, me.x - self.anchor_x * self.xscale, me.y - self.anchor_y * self.yscale, self.xscale, self.yscale, 0.0, C_WHITE, self.alpha);
    }
}

impl Object for AfterimageScreen {
    fn name(&self) -> &'static str { "obj_afterimage_screen" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        *self = AfterimageScreen::default();
        self.anchor_x = me.x - g.camerax();
        self.anchor_y = me.y - g.cameray();
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.xscale += self.xrate;
        self.yscale += self.yrate;
        self.alpha = scr_approach(self.alpha, 0.0, self.faderate);
        if self.alpha == 0.0 {
            g.destroy_self(me);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.draw_end_flag {
            return;
        }
        self.snapshot_and_draw(me, g);
    }

    fn draw_end(&mut self, me: &mut Inst, g: &mut Game) {
        if !self.draw_end_flag {
            return;
        }
        self.snapshot_and_draw(me, g);
        // with (obj_heart) draw_self();
        for hid in g.ids_of("obj_heart") {
            let Some(h) = g.inst(hid) else { continue };
            let (s, i, x, y, xs, ys, a, c, al) = (h.sprite_index, h.image_index, h.x, h.y, h.image_xscale, h.image_yscale, h.image_angle, h.image_blend, h.image_alpha);
            g.draw_sprite_ext(s, i, x, y, xs, ys, a, c, al);
        }
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.gfx.surface_exists(self.my_surface) {
            g.gfx.surface_free(self.my_surface);
        }
    }

    obj_vars!(xscale, yscale, alpha, anchor_x, anchor_y, xrate, yrate, faderate);
}

// ============================================================================ obj_knight_warp

/// obj_knight_warp
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
        scr_darksize(me);
        me.image_speed = 0.0;
        g.snd_play("snd_knight_teleport");
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.master != NOONE && g.id_exists(self.master) {
            if let Some(m) = g.inst(self.master) {
                me.x = m.x + self.master_xoffset;
                me.y = m.y + self.master_yoffset;
            }
        }
    }

    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            0 => {
                if self.master != NOONE && g.id_exists(self.master) {
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

    /// Other_10 (warp in) / Other_11 (warp out)
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        let hide_master = |g: &mut Game, m: Id| {
            if m != NOONE && g.id_exists(m) {
                if let Some(mi) = g.inst_mut(m) {
                    mi.image_alpha = 0.0;
                }
            }
        };
        match n {
            0 => {
                hide_master(g, self.master);
                me.image_index = 6.0;
                g.lerpvar(me, "image_index", 5.0, 8.0, 4.0, 0, "out");
                me.alarm[0] = 4;
            }
            1 => {
                hide_master(g, self.master);
                me.image_index = 8.0;
                g.lerpvar(me, "image_index", 8.0, 5.0, 4.0, 0, "out");
                me.alarm[1] = 4;
            }
            _ => {}
        }
    }

    obj_vars!(master_xoffset, master_yoffset);
}
