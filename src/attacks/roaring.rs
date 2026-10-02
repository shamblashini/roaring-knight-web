//! "The Roaring" (obj_knight_enemy myattackchoice 9, obj_dbulletcontroller type 107).
//!
//! Ported objects:
//! * obj_knight_roaring2 (`Roaring2`, parent obj_bulletparent) — Create / Step / Other_10 / Draw / CleanUp
//! * obj_knight_roaring_star (`RoaringStar`, parent obj_regularbullet) — Create / Step / Other_10 / Other_11 / Other_15
//! * obj_knight_pointing_starchild (`Starchild`, parent obj_regularbullet) — created by exploding stars
//! * obj_roaringknight_slash (`RoaringSlash`, parent obj_collidebullet) — the final screen-splitting cut
//! * obj_knight_roaring_fx (`RoaringFx`) + obj_knight_crush (`KnightCrush`) + plain obj_regularbullet
//!   (`RoaringFireBullet`) — the cutscene version of the roar (not created by the battle attack)
//! * obj_knight_circle, obj_particle_generic, obj_afterimage_screen, obj_afterimage_grow, obj_afterimage_blend
//! * scripts: scr_precise_hit, scr_draw_beam_color, scr_draw_outline(_ext), scr_pingpong/scr_loop, remap,
//!   clamp01, scr_rotatetowards, scr_angle_lerp, scr_onscreen_tolerance, scr_heartclamp, scr_afterimagefast,
//!   scr_afterimage_grow, scr_custom_afterimage, scr_script_repeat, ossafe_fill_rectangle_color.

use crate::assets::{spr, Spr, NO_SPR};
use crate::audio::SndHandle;
use crate::battle::{self, DBulletController, DbCtrl, Growtangle, Heart, KnightEnemy, RegVars};
use crate::gfx::{bm, make_color_hsv, make_color_rgb, merge_color, Color, C_BLACK, C_DKGRAY, C_GRAY, C_GREEN, C_RED, C_WHITE};
use crate::gm::{angle_difference, lengthdir_x, lengthdir_y, lerp, point_direction, point_distance, scr_approach, scr_ease_out, scr_movetowards, sign, truthy, PI};
use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object, NOONE};

const SND_NONE: SndHandle = -4;

// ============================================================================ controller

/// obj_dbulletcontroller Step_0, `if (type == 107)`.
pub fn ctrl_step(c: &mut DbCtrl, me: &mut Inst, g: &mut Game) {
    if c.made == 0.0 {
        g.glob.turntimer = 999999.0;
        c.made = 1.0;
        if let Some(k) = g.inst_mut(c.creatorid) {
            k.image_alpha = 0.0;
        }
        let (kx, ky) = g.inst(c.creatorid).map(|k| (k.x, k.y)).unwrap_or((0.0, 0.0));
        let id = g.instance_create(kx, ky, Box::new(Roaring2::default()));
        if let Some(i) = g.inst_mut(id) {
            i.target = 3.0;
        }
        // scr_bullet_inherit(knight_roarin2), called by the controller: also copies creatorid/creator
        battle::scr_bullet_inherit(me, id, g);
        if let Some((_, o)) = g.get::<Roaring2>(id) {
            o.creatorid = c.creatorid;
            o.creator = c.creator;
        }
    }
}

// ============================================================================ helpers

/// GML `==` on reals compares with math_get_epsilon() (default 0.00001).
fn feq(a: f64, b: f64) -> bool { (a - b).abs() <= 0.00001 }

fn clamp01(v: f64) -> f64 { v.max(0.0).min(1.0) }

/// remap(a, b, c, d, v) = lerp(c, d, scr_inverselerp(a, b, v))
fn remap(a: f64, b: f64, c: f64, d: f64, v: f64) -> f64 {
    let t = if b == a { 0.0 } else { (v - a) / (b - a) };
    lerp(c, d, t)
}

fn scr_loop(v: f64, m: f64) -> f64 {
    if m == 0.0 {
        return v;
    }
    let mut r = v % m;
    if v < 0.0 {
        r += m;
    }
    r
}

fn scr_pingpong(v: f64, m: f64) -> f64 {
    if m == 0.0 {
        return v;
    }
    let mut r = scr_loop(v, m * 2.0);
    if r > m {
        r = m * 2.0 - r;
    }
    r
}

fn scr_rotatetowards(a: f64, b: f64, amt: f64) -> f64 {
    let d = angle_difference(b, a);
    if d.abs() > amt {
        a + sign(d) * amt
    } else {
        b
    }
}

fn scr_angle_lerp(a: f64, b: f64, t: f64) -> f64 { a + lerp(0.0, angle_difference(b, a), t) }

/// scr_darksize()
fn scr_darksize(me: &mut Inst) {
    me.image_xscale = 2.0;
    me.image_yscale = 2.0;
}

/// ossafe_fill_rectangle_color (non-PlayStation path).
fn ossafe_fill_rect(g: &mut Game, mut x1: f64, mut y1: f64, mut x2: f64, mut y2: f64, c: Color) {
    if x1 > x2 {
        std::mem::swap(&mut x1, &mut x2);
    }
    if y1 > y2 {
        std::mem::swap(&mut y1, &mut y2);
    }
    g.gfx.draw_rectangle_color(x1, y1, x2, y2, c, c, c, c, false);
}

/// scr_draw_beam_color(x, y, length, width, angle, col1, col2, alpha, circle)
#[allow(clippy::too_many_arguments)]
fn scr_draw_beam_color(g: &mut Game, x: f64, y: f64, len: f64, w: f64, ang: f64, c1: Color, c2: Color, alpha: f64, circle: bool) {
    let e0 = (x + lengthdir_x(len, ang), y + lengthdir_y(len, ang));
    let e1 = (x + lengthdir_x(len, ang + w / 2.0), y + lengthdir_y(len, ang + w / 2.0));
    let e2 = (x + lengthdir_x(len, ang - w / 2.0), y + lengthdir_y(len, ang - w / 2.0));
    let prev = g.gfx.draw_get_alpha();
    g.gfx.draw_set_alpha(alpha);
    if circle {
        g.gfx.draw_circle(e0.0, e0.1, w / 2.0, false);
    }
    g.gfx.draw_triangle_color(x, y, e1.0, e1.1, e2.0, e2.1, c1, c2, c2, false);
    g.gfx.draw_set_alpha(prev);
}

/// scr_draw_outline_ext(sprite, subimg, x, y, xscale, yscale, angle, colour, alpha, thickness)
#[allow(clippy::too_many_arguments)]
fn scr_draw_outline_ext(g: &mut Game, s: Spr, sub: f64, x: f64, y: f64, xs: f64, ys: f64, rot: f64, col: Color, alpha: f64, th: f64) {
    g.gfx.gpu_set_fog(true, col);
    let (mut xa, mut xb, mut ya, mut yb) = (th, 0.0, 0.0, th);
    if rot % 90.0 != 0.0 {
        xa = lengthdir_x(th, rot);
        xb = lengthdir_x(th, rot + 90.0);
        ya = lengthdir_y(th, rot + 90.0);
        yb = lengthdir_y(th, rot);
    }
    g.draw_sprite_ext(s, sub, x + xa, y + ya, xs, ys, rot, C_WHITE, alpha);
    g.draw_sprite_ext(s, sub, x - xa, y - ya, xs, ys, rot, C_WHITE, alpha);
    g.draw_sprite_ext(s, sub, x + xb, y + yb, xs, ys, rot, C_WHITE, alpha);
    g.draw_sprite_ext(s, sub, x - xb, y - yb, xs, ys, rot, C_WHITE, alpha);
    g.gfx.gpu_set_fog(false, C_WHITE);
}

/// scr_draw_outline(thickness, colour, alpha) on `me`.
fn scr_draw_outline(g: &mut Game, me: &Inst, th: f64, col: Color, alpha: f64) {
    let a = me.image_alpha * alpha;
    scr_draw_outline_ext(g, me.sprite_index, me.image_index, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, col, a, th);
}

/// Builtin draw arguments of another instance (for `with (obj) draw_sprite_ext(...)`).
fn inst_draw_args(g: &Game, id: Id) -> Option<(Spr, f64, f64, f64, f64, f64, f64, Color, f64)> {
    let i = g.inst(id)?;
    Some((i.sprite_index, i.image_index, i.x, i.y, i.image_xscale, i.image_yscale, i.image_angle, i.image_blend, i.image_alpha))
}

/// `with (obj_heart) draw_self();` (dx/dy = -camera for screen space).
fn draw_hearts(g: &mut Game, dx: f64, dy: f64) {
    for id in g.ids_of("obj_heart") {
        if let Some((s, sub, x, y, xs, ys, rot, col, a)) = inst_draw_args(g, id) {
            g.draw_sprite_ext(s, sub, x + dx, y + dy, xs, ys, rot, col, a);
        }
    }
}

/// scr_afterimagefast()
fn scr_afterimagefast(me: &Inst, g: &mut Game) -> Id {
    let id = g.afterimage(me);
    g.set_var(id, "fadeSpeed", 0.08);
    id
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

/// scr_custom_afterimage(obj_afterimage_blend)
fn scr_custom_afterimage_blend(me: &Inst, g: &mut Game) -> Id {
    let id = g.instance_create(me.x, me.y, Box::new(AfterimageBlend::default()));
    if let Some(a) = g.inst_mut(id) {
        a.sprite_index = me.sprite_index;
        a.image_index = me.image_index;
        a.image_blend = me.image_blend;
        a.image_speed = 0.0;
        a.depth = me.depth + 1.0;
        a.image_xscale = me.image_xscale;
        a.image_yscale = me.image_yscale;
        a.image_angle = me.image_angle;
    }
    id
}

/// scr_script_repeat(instance_create, 8, 2, x, y, obj_afterimage_screen): obj_script_delayed with
/// constant = 1 runs on its 1st, 3rd, 5th and 7th step (totaltimer 0, 2, 4, 6 < max_time 8), as long
/// as `target` exists.
fn script_repeat_afterimage_screen(g: &mut Game, target: Id, x: f64, y: f64) {
    for d in [1, 3, 5, 7] {
        g.script_delayed(target, d, move |g| {
            g.instance_create(x, y, Box::new(AfterimageScreen::default()));
        });
    }
}

/// scr_onscreen_tolerance(self, spacer)
fn scr_onscreen_tolerance(g: &Game, me: &Inst, spacer: f64) -> bool {
    let (cx, cy) = (g.camerax(), g.cameray());
    let (sw, sh) = (g.sprite_width(me), g.sprite_height(me));
    !(me.x + sw + spacer < cx || me.x - spacer > cx + 640.0 || me.y + sh + spacer < cy || me.y - spacer > cy + 480.0)
}

/// scr_heartclamp()
fn scr_heartclamp(g: &mut Game) {
    let Some(gt) = g.first("obj_growtangle") else { return };
    let Some((gxs, gys)) = g.inst(gt).map(|b| (b.image_xscale, b.image_yscale)) else { return };
    let xthick = gxs * 2.0 + 1.0;
    let ythick = gys * 2.0 + 1.0;
    let (l, t, r, b) = (battle::scr_get_box(g, 2), battle::scr_get_box(g, 1), battle::scr_get_box(g, 0), battle::scr_get_box(g, 3));
    for id in g.ids_of("obj_heart") {
        if let Some(h) = g.inst_mut(id) {
            h.x = h.x.max(l + xthick).min(r - (20.0 + xthick));
            h.y = h.y.max(t + ythick).min(b - (20.0 + ythick));
        }
    }
}

/// `with (obj_knight_enemy) event_user(2);` — the Roaring's "true damage to everyone" hit.
fn knight_roaring_hit(g: &mut Game) {
    // TODO(battle): obj_knight_enemy Other_12 (40 damage to each party member, never killing one from
    // 2..40 HP) must be implemented as KnightEnemy's `user(2)`; until then this is a no-op.
    for id in g.ids_of("obj_knight_enemy") {
        g.user_event(id, 2);
    }
}

fn set_knight_aoedamage(g: &mut Game, v: bool) {
    for id in g.ids_of("obj_knight_enemy") {
        if let Some((_, k)) = g.get::<KnightEnemy>(id) {
            k.aoedamage = v;
        }
    }
}

/// Does the (precise) mask of `i` cover world point (px, py)? Mirrors rt's private `covers`.
fn covers(g: &Game, i: &Inst, px: f64, py: f64, bb: &[f64; 4]) -> bool {
    let Some(sp) = g.sprite(g.mask_of(i)) else { return false };
    if sp.kind == 0 {
        return px >= bb[0] && px <= bb[2] + 1.0 && py >= bb[1] && py <= bb[3] + 1.0;
    }
    if i.image_xscale == 0.0 || i.image_yscale == 0.0 {
        return false;
    }
    let (sn, cs) = (i.image_angle.to_radians().sin(), i.image_angle.to_radians().cos());
    let (dx, dy) = (px - i.x, py - i.y);
    let lx = (dx * cs - dy * sn) / i.image_xscale + sp.ox;
    let ly = (dx * sn + dy * cs) / i.image_yscale + sp.oy;
    let (mx, my) = (lx.floor() as i64, ly.floor() as i64);
    if sp.kind == 2 || sp.masks.is_empty() {
        return mx as f64 >= sp.bl && mx as f64 <= sp.br && my as f64 >= sp.bt && my as f64 <= sp.bb;
    }
    let n = sp.frame_count() as i64;
    let fi = (i.image_index.floor() as i64).rem_euclid(n.max(1)) as usize;
    sp.mask_at(fi, mx, my)
}

/// scr_precise_hit(hitbox): precise collision of the SOUL's centre (point or square) with `me`.
fn scr_precise_hit(me: &Inst, g: &mut Game, hitbox: f64) -> bool {
    let a = hitbox / 2.0;
    let Some((hx, hy)) = g.first_inst("obj_heart").map(|h| (h.x + 10.0, h.y + 10.0)) else { return false };
    let Some(bb) = g.bbox(me) else { return false };
    if a <= 0.0 {
        return hx >= bb[0] && hx <= bb[2] + 1.0 && hy >= bb[1] && hy <= bb[3] + 1.0 && covers(g, me, hx, hy, &bb);
    }
    // collision_rectangle(hx - a, hy - a, hx + a, hy + a, id, true, false)
    let il = (hx - a).max(bb[0]);
    let ir = (hx + a).min(bb[2]);
    let it = (hy - a).max(bb[1]);
    let ib = (hy + a).min(bb[3]);
    if il > ir || it > ib {
        return false;
    }
    let mut yy = it.floor();
    while yy <= ib {
        let mut xx = il.floor();
        while xx <= ir {
            if covers(g, me, xx + 0.5, yy + 0.5, &bb) {
                return true;
            }
            xx += 1.0;
        }
        yy += 1.0;
    }
    false
}

fn heart_hitbox(g: &mut Game, small: f64, normal: f64) -> f64 {
    let s = g.first_inst("obj_heart").map(|h| h.sprite_index).unwrap_or(NO_SPR);
    if s == spr("spr_dodgeheart_smaller_2px") {
        small
    } else {
        normal
    }
}

// ============================================================================ obj_knight_roaring2

/// obj_knight_roaring2 (parent obj_bulletparent).
#[derive(Default)]
pub struct Roaring2 {
    pub jumpimages: bool,
    pub my_surface: i32,
    pub ball_surface: i32,
    pub star_surface: i32,
    pub terrible_surface: i32,
    pub darkness: f64,
    pub ball_darkness: f64,
    pub timer: f64,
    pub fake_x: f64,
    pub fake_y: f64,
    pub fake_alpha: f64,
    pub rand_angle: f64,
    pub rand_dist: f64,
    pub star_flicker: f64,
    pub intensity: f64,
    pub intensify: f64,
    pub attack_timer: f64,
    pub attack_timer_goal: f64,
    pub attack_token: f64,
    pub roaring_timer: f64,
    pub line_timer: f64,
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub knight_sprite: Spr,
    pub knight_sprite_image: f64,
    pub knight_sprite_speed: f64,
    pub bobble_count: f64,
    pub bobble_freq: f64,
    pub bobble_amp: f64,
    pub ball_counter: f64,
    pub ball_speed: f64,
    pub player_suck: f64,
    pub hsv: f64,
    pub hsv_switch: bool,
    pub sound: SndHandle,
    /// audio_sound_get_pitch(sound) (the audio module has no pitch getter)
    pub sound_pitch: f64,
    pub bullet_list: Vec<Id>,
    pub do_fake_screen: bool,
    pub fakey_screen: Spr,
    pub fakey_screen_2: Spr,
    pub stop: bool,
    pub starcount_p1: f64,
    pub starcount_p2: f64,
    pub starvar: f64,
    pub colorize: f64,
    /// set by scr_bullet_inherit from obj_dbulletcontroller
    pub creatorid: Id,
    pub creator: f64,
}

impl Roaring2 {
    /// Centre of the fake knight (where everything is sucked towards).
    fn centre(&self, g: &Game) -> (f64, f64) { (g.camerax() + self.fake_x, g.cameray() + self.fake_y + 55.0) }

    /// A star fired from the edge of the screen towards the knight (attack_timer == 4 blocks).
    fn fire_incoming_star(&self, g: &mut Game, x: f64, y: f64) -> Id {
        battle::scr_fire_bullet(g, x, y, Box::new(RoaringStar::default()), 0.0, 0.0, Some(spr("spr_knight_bullet_star")), false, None, None)
    }

    /// A star fired out of the knight during the roar (scales from 0.1 to `scale_to` over 32 frames).
    fn fire_roar_star(&self, g: &mut Game, dir: f64, spd: f64, scale_to: f64) -> Id {
        let (cx, cy) = self.centre(g);
        let id = battle::scr_fire_bullet(g, cx, cy, Box::new(RoaringStar::default()), dir, spd, Some(spr("spr_knight_bullet_star")), false, None, None);
        if let Some((i, s)) = g.get::<RoaringStar>(id) {
            i.visible = false;
            s.wall_destroy = 0.0;
            s.bottomfade = 0.0;
            i.destroyonhit = 0.0;
            i.image_xscale = 0.1;
            i.image_yscale = 0.1;
        }
        g.lerpvar_instance(id, "image_xscale", 0.1, scale_to, 32.0, 0, "out");
        g.lerpvar_instance(id, "image_yscale", 0.1, scale_to, 32.0, 0, "out");
        id
    }

    /// The obj_afterimage_screen created every 3 frames while charging.
    fn charge_afterimage_screen(&self, g: &mut Game) {
        let (cx, cy) = self.centre(g);
        let ox = g.irandom_range(-30.0, 30.0);
        let oy = g.irandom_range(-30.0, 30.0);
        let id = g.instance_create(cx + ox, cy + oy, Box::new(AfterimageScreen::default()));
        let intensity = self.intensity;
        if let Some((_, a)) = g.get::<AfterimageScreen>(id) {
            a.faderate = 0.1 / intensity;
            a.draw_end = true;
            a.xrate = -0.01;
            a.yrate = -0.01;
        }
    }

    /// The light streaks sucked into the knight.
    fn charge_particle(&self, g: &mut Game) {
        let (cx, cy) = self.centre(g);
        let randangle = g.irandom(360.0);
        let randdistance = 480.0 + g.irandom(80.0);
        let id = g.instance_create(cx + lengthdir_x(randdistance, randangle), cy + lengthdir_y(randdistance, randangle), Box::new(ParticleGeneric::default()));
        let (px, py) = g.inst(id).map(|i| (i.x, i.y)).unwrap_or((0.0, 0.0));
        if let Some((i, p)) = g.get::<ParticleGeneric>(id) {
            p.not_outbound = false;
            i.sprite_index = spr("spr_pixel_white_front");
            i.set_direction(point_direction(px, py, cx, cy));
            i.image_angle = i.direction();
            i.image_xscale = 16.0;
            i.image_yscale = 0.5;
        }
        g.lerpvar_instance(id, "image_xscale", 320.0, 2.0, 16.0, 0, "out");
        g.lerpvar_instance(id, "image_yscale", 2.0, 0.1, 16.0, 0, "out");
        g.lerpvar_instance(id, "image_alpha", 1.0, 0.5, 16.0, 0, "out");
        g.lerpvar_instance(id, "x", px, cx, 8.0, 1, "out");
        g.lerpvar_instance(id, "y", py, cy, 8.0, 1, "out");
        if let Some((_, p)) = g.get::<ParticleGeneric>(id) {
            p.timer = 18.0;
        }
    }

    /// `with (scr_afterimage_grow()) {...}` (timer >= 120). `delayed_destroy` = Other_10's variant.
    fn grow_afterimage(&self, me: &Inst, g: &mut Game, delayed_destroy: bool) {
        let aid = scr_afterimage_grow(me, g);
        let s = 2.2 + (self.timer - 116.0).min(18.0) * 0.15;
        if let Some((a, o)) = g.get::<AfterimageGrow>(aid) {
            a.sprite_index = spr("spr_roaringknight_front_filled");
            a.image_alpha = 0.01;
            a.image_xscale = s;
            a.image_yscale = s;
            a.visible = false;
            o.xrate = 0.0;
            o.yrate = 0.0;
            o.fade = 0.0;
        }
        g.lerpvar_instance(aid, "image_xscale", s, s * 1.2, 2.0, 1, "out");
        g.lerpvar_instance(aid, "image_yscale", s, s * 1.2, 2.0, 1, "out");
        g.script_delayed(aid, 2, move |g| {
            g.lerpvar_instance(aid, "image_xscale", s * 1.2, 2.0, 14.0, 1, "out");
        });
        g.script_delayed(aid, 2, move |g| {
            g.lerpvar_instance(aid, "image_yscale", s * 1.2, 2.0, 14.0, 1, "out");
        });
        g.lerpvar_instance(aid, "image_alpha", 0.01, 0.35, 16.0, 1, "in");
        if delayed_destroy {
            g.doom(aid, 24);
        } else if let Some((_, o)) = g.get::<AfterimageGrow>(aid) {
            o.destroytime = 24.0;
        }
    }

    /// Shared opening of Step_0 / Other_10 (everything up to `if (timer > 128)`).
    fn step_intro(&mut self, me: &mut Inst, g: &mut Game) {
        if self.jumpimages {
            scr_afterimagefast(me, g);
        }
        self.timer += 1.0;
        if self.line_timer > -1.0 {
            self.line_timer += 1.0;
        }
        self.bobble_count += self.bobble_freq;
        if self.timer == 30.0 {
            for gt in g.ids_of("obj_growtangle") {
                let Some(b) = g.inst(gt) else { continue };
                let (xs, ys) = (b.image_xscale, b.image_yscale);
                let (sw, sh) = (g.sprite_width(b), g.sprite_height(b));
                g.lerpvar_instance(gt, "image_xscale", xs, 2560.0 / sw, 160.0, 1, "out");
                g.lerpvar_instance(gt, "image_yscale", ys, 1920.0 / sh, 160.0, 1, "out");
            }
        }
        if self.timer == 80.0 {
            g.lerpvar(me, "fake_alpha", 0.0, 1.0, 48.0, 1, "out");
            g.lerpvar(me, "fake_y", 24.0, 88.0, 48.0, 2, "out");
        }
    }

    /// `with (obj_afterimage_grow)` follow + the timer 118 / 132 / >132 bits (same in both versions).
    fn step_follow_and_sound(&mut self, me: &mut Inst, g: &mut Game) {
        let (cx, cy) = (g.camerax(), g.cameray());
        let ax = cx + self.fake_x;
        let ay = cy + self.fake_y + (self.bobble_count * 0.1).sin() * self.bobble_amp - 10.0 + 55.0;
        for id in g.ids_of("obj_afterimage_grow") {
            if let Some(a) = g.inst_mut(id) {
                a.x = ax;
                a.y = ay;
            }
        }
        if self.timer == 118.0 {
            let id = me.id;
            g.script_delayed(id, 16, move |g| {
                g.lerpvar_instance(id, "ball_darkness", 0.0, 1.0, 32.0, 1, "out");
            });
        }
        if self.timer == 132.0 {
            self.sound = g.snd_play_pitch("snd_knight_stretch", 0.1);
            self.sound_pitch = 0.1;
        }
        if self.timer > 132.0 && self.sound != SND_NONE {
            self.sound_pitch += 0.000535;
            g.audio.pitch(self.sound, self.sound_pitch);
        }
    }

    /// Shared part of `if (timer > 128)` before the attack_timer block. `suck_goal` differs (1 vs 3).
    fn step_charge(&mut self, me: &mut Inst, g: &mut Game, suck_goal: f64) {
        self.intensity = scr_approach(self.intensity, 4.0, 0.008);
        if self.roaring_timer < 1.0 && self.intensity < 4.0 {
            self.ball_speed = self.intensity * 3.0;
            if self.intensity < 3.75 {
                self.player_suck = scr_approach(self.player_suck, suck_goal, 0.1625);
            }
        }
        self.player_suck = scr_approach(self.player_suck, 0.0, 0.15);
        let (cx, cy) = self.centre(g);
        if let Some(h) = g.first_inst("obj_heart") {
            let tempdir = point_direction(h.x + 10.0, h.y + 10.0, cx, cy);
            h.x += lengthdir_x(self.player_suck, tempdir);
            h.y += lengthdir_y(self.player_suck, tempdir);
        }
        self.attack_timer += 1.0;
        if self.timer % 3.0 == 0.0 && self.intensity < 3.9 {
            self.charge_afterimage_screen(g);
        }
        if feq(self.intensity, 3.66) {
            let id = me.id;
            g.script_delayed(id, 16, move |g| {
                g.lerpvar_instance(id, "ball_darkness", 1.0, 0.0, 32.0, 1, "out");
            });
            let c = g.instance_create_depth(cx, cy, me.depth, Box::new(KnightCircle::default()));
            if let Some((i, k)) = g.get::<KnightCircle>(c) {
                k.r = 0.0;
                k.g = 0.0;
                k.b = 0.0;
                k.r_goal = 255.0;
                k.g_goal = 255.0;
                k.b_goal = 255.0;
                k.fade_time = 48.0;
                k.circle_size = 480.0;
                k.size_goal = 0.0;
                k.growth = 10.0;
                // scr_lerpvar(r_goal, 0, 255, ...) x3: the *value* 255 is passed as the variable name,
                // so these lerpvars never write anything (faithfully omitted).
                k.draw_in_box = false;
                i.visible = false;
            }
            g.doom(c, 48);
        }
        if feq(self.intensity, 3.74) && self.knight_sprite == spr("spr_roaringknight_front") {
            self.knight_sprite = spr("spr_roaringknight_front_flourish");
            self.knight_sprite_image = 0.0;
            self.knight_sprite_speed = 0.0;
            let id = me.id;
            g.script_delayed(id, 8, move |g| {
                g.lerpvar_instance(id, "knight_sprite_image", 0.0, 4.0, 16.0, 0, "out");
            });
            g.script_delayed(id, 8, move |g| {
                g.lerpvar_instance(id, "fake_alpha", 1.0, 0.0, 32.0, 0, "out");
            });
        }
        if self.timer >= 136.0 && self.intensity < 3.75 && self.timer % 1.0 == 0.0 {
            self.charge_particle(g);
        }
    }

    /// The roar itself (`if (intensity == 4)`). `alt` = Other_10's variant.
    fn step_roar(&mut self, me: &mut Inst, g: &mut Game, alt: bool) {
        self.roaring_timer += 1.0;
        let rt = self.roaring_timer;
        let (cx, cy) = self.centre(g);
        if rt < if alt { 181.0 } else { 169.0 } {
            if rt == 9.0 {
                g.lerpvar(me, "knight_sprite_image", 4.0, 6.0, 4.0, 0, "out");
                self.fake_alpha = 1.0;
                self.player_suck = self.player_suck.min(-6.0);
                self.ball_speed = -32.0;
                self.ball_darkness = 1.0;
                g.lerpvar(me, "bobble_freq", 1.0, 3.0, 8.0, 0, "out");
                g.snd_play("snd_knight_roar");
                script_repeat_afterimage_screen(g, me.id, cx, cy);
                let c = g.instance_create_depth(cx, cy, me.depth, Box::new(KnightCircle::default()));
                if let Some((i, k)) = g.get::<KnightCircle>(c) {
                    k.r = 255.0;
                    k.g = 255.0;
                    k.b = 255.0;
                    k.draw_in_box = false;
                    i.visible = false;
                }
                for a in 0..8 {
                    let spd = 8.5 + g.random(2.0);
                    self.fire_roar_star(g, a as f64 * 45.0, spd, 1.2);
                }
            }
            if rt == 15.0 {
                me.sprite_index = spr("spr_roaringknight_front_roar");
                me.image_speed = 0.5;
                self.knight_sprite = spr("spr_roaringknight_front_roar");
                self.knight_sprite_image = 0.0;
                self.knight_sprite_speed = 0.5;
            }
            if rt >= 9.0 {
                self.player_suck = self.player_suck.min(-3.0);
            }
            if rt % 3.0 == 0.0 {
                let ox = g.irandom_range(-30.0, 30.0);
                let oy = g.irandom_range(-30.0, 30.0);
                let id = g.instance_create(cx + ox, cy + oy, Box::new(AfterimageScreen::default()));
                if let Some((_, a)) = g.get::<AfterimageScreen>(id) {
                    a.xrate = 0.015;
                    a.yrate = 0.015;
                    a.faderate = 0.025;
                    a.draw_end = true;
                }
            }
            if rt > 15.0 && rt % 5.0 == 0.0 {
                let starsound = g.snd_play_pitch("snd_stardrop", 0.5);
                g.audio.volume(starsound, 0.5, 0.0);
                if !alt {
                    self.rand_angle += 60.0 + g.irandom(10.0);
                    self.starcount_p2 += 1.0;
                    // star_angle1 = point_direction(centre, obj_heart) is immediately overwritten.
                    let star_angle1 = self.rand_angle;
                    let star_angle2 = self.rand_angle + 20.0;
                    let star_angle3 = self.rand_angle - 20.0;
                    let s1 = 6.5 + g.random(2.0);
                    self.fire_roar_star(g, star_angle1, s1, 1.6);
                    let s2 = 8.5 + g.random(2.0);
                    self.fire_roar_star(g, star_angle2, s2, 1.6);
                    let s3 = 8.5 + g.random(2.0);
                    self.fire_roar_star(g, star_angle3, s3, 1.6);
                } else {
                    self.rand_angle += 40.0 + g.irandom(40.0);
                    let s1 = 6.5 + g.random(2.0);
                    let ra = self.rand_angle;
                    self.fire_roar_star(g, ra, s1, 1.6);
                    let d2 = ra + 80.0 + g.random(80.0);
                    let s2 = 8.5 + g.random(2.0);
                    self.fire_roar_star(g, d2, s2, 1.6);
                    let d3 = ra - (80.0 + g.random(80.0));
                    let s3 = 8.5 + g.random(2.0);
                    self.fire_roar_star(g, d3, s3, 1.6);
                }
            }
        }
        if rt == 181.0 {
            self.colorize = 6.0;
            g.lerpvar(me, "player_suck", self.player_suck, 0.0, 24.0, 0, "out");
            g.lerpvar(me, "ball_speed", self.ball_speed, 1.0, 24.0, 0, "out");
            g.lerpvar(me, "bobble_freq", 3.0, 1.0, 16.0, 0, "out");
            me.sprite_index = spr("spr_roaringknight_front_flourish");
            me.image_index = 0.0;
            me.image_speed = 0.0;
            self.knight_sprite = spr("spr_roaringknight_front_flourish");
            self.knight_sprite_speed = 0.0;
            g.lerpvar(me, "knight_sprite_image", 5.99, 0.0, 12.0, 0, "out");
            for id in g.ids_of("obj_knight_roaring_star") {
                if let Some(s) = g.inst_mut(id) {
                    s.friction = 0.5;
                }
                self.bullet_list.push(id);
            }
        }
        if rt >= 182.0 && !self.bullet_list.is_empty() {
            let bul = self.bullet_list[0];
            if let Some((_, s)) = g.get::<RoaringStar>(bul) {
                s.con = 1.0;
            }
            self.bullet_list.remove(0);
        }
        if rt == 275.0 {
            me.sprite_index = spr("spr_roaringknight_front_slash");
            self.knight_sprite = spr("spr_roaringknight_front_slash");
            g.lerpvar(me, "knight_sprite_image", 0.0, 2.0, 8.0, 0, "out");
            g.lerpvar(me, "image_index", 0.0, 2.0, 8.0, 0, "out");
            g.lerpvar(me, "bobble_amp", 4.0, 0.0, 24.0, 0, "out");
            self.line_timer = 0.0;
            g.lerpvar(me, "r", 128.0, 255.0, 16.0, 0, "out");
            g.lerpvar(me, "g", 128.0, 0.0, 16.0, 0, "out");
            g.lerpvar(me, "b", 128.0, 0.0, 16.0, 0, "out");
        }
        if rt == 299.0 {
            me.x = g.camerax() + self.fake_x;
            me.y = g.cameray() + self.fake_y + 20.0;
            for gt in g.ids_of("obj_growtangle") {
                if let Some(b) = g.inst_mut(gt) {
                    b.image_xscale = 0.0;
                    b.image_yscale = 0.0;
                }
            }
            g.lerpvar(me, "knight_sprite_image", 2.0, 5.0, 6.0, 0, "out");
            g.lerpvar(me, "image_index", 2.0, 5.0, 6.0, 0, "out");
            self.do_fake_screen = true;
            g.snd_play("snd_knight_cut");
            let sx = g.camerax() + g.camerawidth() * 0.5 - lengthdir_x(-160.0, 117.0);
            let sy = g.cameray() + g.cameraheight() * 0.5 - lengthdir_y(-160.0, 117.0);
            let slashid = g.instance_create(sx, sy, Box::new(RoaringSlash::default()));
            if let Some((i, s)) = g.get::<RoaringSlash>(slashid) {
                i.set_direction(117.0);
                i.image_xscale = 4.0;
                i.image_angle = i.direction();
                s.width *= 4.0;
                s.slashdir = -1.0;
            }
            battle::scr_bullet_inherit(me, slashid, g);
            // with (slashid) event_user(0): obj_roaringknight_slash has no Other_10.
            self.jumpimages = true;
            let y = me.y;
            g.lerpvar(me, "y", y, y + 40.0, 16.0, 1, "out");
            let id = me.id;
            g.script_delayed(id, 16, move |g| {
                g.lerpvar_instance(id, "y", y + 40.0, y - 320.0, 24.0, 1, "in");
            });
        }
        if rt == 363.0 {
            self.jumpimages = false;
            if let Some(c) = g.inst(self.creatorid) {
                me.x = c.x;
            }
            if let Some((k, ko)) = g.get_first::<KnightEnemy>("obj_knight_enemy") {
                ko.siner2 = 0.0;
                k.y = k.ystart + (ko.siner2 / 8.0).cos() * 8.0;
                me.y = k.y;
            }
            me.sprite_index = spr("spr_knight_warp");
            me.image_index = 5.0;
            me.image_speed = 0.0;
            g.lerpvar(me, "image_index", 5.0, 8.0, 8.0, 0, "out");
        }
        if rt == 375.0 {
            for k in g.ids_of("obj_knight_enemy") {
                if let Some(i) = g.inst_mut(k) {
                    i.image_alpha = 1.0;
                }
            }
            for gt in g.ids_of("obj_growtangle") {
                if let Some((_, b)) = g.get::<Growtangle>(gt) {
                    b.growcon = 3.0;
                    b.timer = 0.0;
                }
            }
            // ends the bullet phase: the battle controller now destroys every bullet (incl. this one)
            g.glob.turntimer = -1.0;
        }
    }

    /// Final `if (roaring_timer < 1) with (obj_knight_roaring_star) {...}`.
    fn step_spiral_stars(&mut self, g: &mut Game, alt: bool) {
        if self.roaring_timer >= 1.0 {
            return;
        }
        let (cx, cy) = self.centre(g);
        let (rt, intensity) = (self.roaring_timer, self.intensity);
        for id in g.ids_of("obj_knight_roaring_star") {
            g.with_t::<RoaringStar, _>(id, |me, s, g| {
                if rt < 180.0 {
                    let dist = point_distance(me.x, me.y, cx, cy);
                    if alt {
                        if dist < 400.0 + 120.0 * intensity {
                            me.set_speed(2.5 + 4.5 * intensity * (1.0 - 0.000925925925925926 * dist));
                        } else {
                            me.set_speed(2.5 + 1.5 * intensity * (1.0 - 0.000925925925925926 * dist));
                        }
                    }
                    me.image_xscale = 0.0058823529411764705 * dist;
                    me.image_yscale = 0.0058823529411764705 * dist;
                    if !alt {
                        if me.image_xscale < 0.2 {
                            me.image_xscale = 0.2;
                        }
                        if me.image_yscale < 0.2 {
                            me.image_yscale = 0.2;
                        }
                    }
                    me.set_direction(point_direction(me.x, me.y, cx, cy));
                    let d = me.speed() * 0.625 * (1.0 / intensity);
                    let dir = me.direction() + 90.0 * s.spinspeed;
                    me.x += lengthdir_x(d, dir);
                    me.y += lengthdir_y(d, dir);
                }
                if point_distance(me.x, me.y, cx, cy) < 12.0 {
                    g.destroy_self(me);
                }
            });
        }
    }

    /// Step_0's `if (attack_timer == 4)` block.
    fn step_attack(&mut self, g: &mut Game) {
        let (cx, cy) = (g.camerax() + self.fake_x, g.cameray() + self.fake_y);
        let target = self.centre(g);
        self.rand_dist = 600.0;
        self.starcount_p1 += 1.0;
        let _spinspeed: f64 = g.choose(&[-1.0, 1.0]);
        if self.starcount_p1 == 1.0 && self.intensity < 3.7 {
            if self.intensity >= 2.7 {
                self.rand_angle += 9.0;
                let star_angle1 = self.rand_angle;
                let mut o = 0.0;
                for _ in 0..2 {
                    let (x, y) = (cx + lengthdir_x(self.rand_dist, star_angle1 + o), cy + lengthdir_y(self.rand_dist, star_angle1 + o));
                    let id = self.fire_incoming_star(g, x, y);
                    if let Some((i, s)) = g.get::<RoaringStar>(id) {
                        s.wall_destroy = 0.0;
                        i.destroyonhit = 0.0;
                        s.bottomfade = 0.0;
                        s.spinspeed = 1.0;
                        i.visible = false;
                        i.image_index = 0.0;
                        i.image_speed = 0.0;
                        i.image_xscale = 2.0;
                        i.image_yscale = 2.0;
                        i.set_direction(point_direction(i.x, i.y, target.0, target.1));
                        i.set_speed(16.0);
                        i.friction = -0.1;
                    }
                    o += 180.0;
                }
            } else {
                self.rand_angle += 32.0;
                for _ in 0..6 {
                    self.rand_angle += 60.0;
                    let (x, y) = (cx + lengthdir_x(self.rand_dist, self.rand_angle), cy + lengthdir_y(self.rand_dist, self.rand_angle));
                    let id = self.fire_incoming_star(g, x, y);
                    let spd = 8.0 + self.intensity;
                    if let Some((i, s)) = g.get::<RoaringStar>(id) {
                        s.wall_destroy = 0.0;
                        i.destroyonhit = 0.0;
                        s.bottomfade = 0.0;
                        s.spinspeed = 1.0;
                        i.visible = false;
                        i.image_index = 0.0;
                        i.image_speed = 0.0;
                        i.image_xscale = 2.0;
                        i.image_yscale = 2.0;
                        i.set_direction(point_direction(i.x, i.y, target.0, target.1));
                        i.set_speed(spd);
                        i.friction = -0.1;
                    }
                }
            }
        }
        if self.starcount_p1 == 3.0 || self.intensity >= 2.7 {
            self.starcount_p1 = 0.0;
        }
        // GML then sets star_angle1/2/3 = -1, so the four following `if (star_angle1 != -1 ...)`
        // fire blocks (and their random(120)) can never run.
        if self.intensity >= 3.0 && self.intensity < 4.0 {
            let (vx, vy, vw, vh) = (g.camerax(), g.cameray(), g.camerawidth(), g.cameraheight());
            for id in g.ids_of("obj_knight_roaring_star") {
                if let Some(s) = g.inst_mut(id) {
                    if s.x < vx - 60.0 {
                        s.x = vx - 60.0;
                    }
                    if s.x > vx + vw + 60.0 {
                        s.x = vx + vw + 60.0;
                    }
                    if s.y < vy - 60.0 {
                        s.y = vy - 60.0;
                    }
                    if s.y > vy + vh + 60.0 {
                        s.y = vy + vh + 60.0;
                    }
                }
            }
        }
        self.attack_timer = (-1.0 + self.intensity).floor();
    }

    /// Other_10's `if (attack_timer == 4)` block.
    fn step_attack_alt(&mut self, g: &mut Game) {
        let (cx, cy) = (g.camerax() + self.fake_x, g.cameray() + self.fake_y);
        let target = self.centre(g);
        self.rand_angle += 80.0 + g.irandom(80.0);
        self.rand_dist = 600.0 + g.irandom(80.0);
        for k in 0..2 {
            let ang = self.rand_angle + if k == 0 { 0.0 } else { 180.0 };
            let (x, y) = (cx + lengthdir_x(self.rand_dist, ang), cy + lengthdir_y(self.rand_dist, ang));
            let id = self.fire_incoming_star(g, x, y);
            let spin: f64 = g.choose(&[-1.0, 1.0]);
            let spd = 7.0 + g.random(2.0);
            if let Some((i, s)) = g.get::<RoaringStar>(id) {
                s.wall_destroy = 0.0;
                i.destroyonhit = 0.0;
                s.bottomfade = 0.0;
                s.spinspeed = spin;
                if k == 1 {
                    i.active = 0.0;
                    i.image_blend = C_DKGRAY;
                }
                i.visible = false;
                i.image_index = 0.0;
                i.image_speed = 0.0;
                let sc = if k == 0 { 2.0 } else { 1.0 };
                i.image_xscale = sc;
                i.image_yscale = sc;
                i.set_direction(point_direction(i.x, i.y, target.0, target.1));
                i.set_speed(spd);
            }
        }
        if self.intensity < 3.5 {
            self.attack_timer = (self.attack_timer_goal + self.attack_token).floor();
        }
        self.attack_token = 1.0 - self.attack_token;
        self.attack_timer_goal = scr_approach(self.attack_timer_goal, 2.0, 0.1);
    }

    /// The fake knight drawn row by row (wobbling), at (ox, oy) offset.
    fn draw_fake_knight(&self, g: &mut Game, me: &Inst, ox: f64, oy: f64) {
        let ks = self.knight_sprite;
        let bl = g.sprite_get_bbox_left(ks);
        let bt = g.sprite_get_bbox_top(ks);
        let h = g.sprite_get_height(ks);
        let t = g.glob.time;
        let bob = (self.bobble_count * 0.1).sin() * self.bobble_amp;
        if self.intensify > 1.5 {
            let mut a = 0.0;
            while a < h {
                let w = ((a + t * 4.0) * 0.15).sin() * (self.intensify - 1.5) * 8.0;
                let xx = if a % 2.0 == 0.0 { ox + self.fake_x - 70.0 + w } else { ox + self.fake_x - 70.0 - w };
                let yy = oy + self.fake_y + a * 2.0 + bob - 10.0 - bt * 2.0;
                g.draw_sprite_part_ext(ks, self.knight_sprite_image, bl, a, 70.0, 1.0, xx, yy, 2.0, 2.0, me.image_blend, self.fake_alpha * 0.75);
                a += 1.0;
            }
        }
        let mut a = 0.0;
        while a < h {
            let xx = ox + self.fake_x - 70.0 + ((a + t * 4.0) * 0.2).sin() * self.intensify * 0.3;
            let yy = oy + self.fake_y + a * 2.0 + bob - 10.0 - bt * 2.0;
            g.draw_sprite_part_ext(ks, self.knight_sprite_image, bl, a, 70.0, 1.0, xx, yy, 2.0, 2.0, me.image_blend, self.fake_alpha);
            a += 1.0;
        }
    }

    /// One `with (obj_knight_roaring_star) { if (skip) continue; event_user(con == 0 ? 0 : 1) }` pass.
    fn draw_star_pass(g: &mut Game, pass: u8) {
        for id in g.ids_of("obj_knight_roaring_star") {
            g.with_t::<RoaringStar, _>(id, |me, s, g| {
                let skip = match pass {
                    1 => me.image_blend == C_WHITE,
                    2 => me.image_blend == C_DKGRAY,
                    _ => (me.image_blend == C_DKGRAY || me.image_xscale > 1.0) && s.con < 1.0,
                };
                if skip {
                    return;
                }
                if s.con == 0.0 {
                    s.user(0, me, g);
                } else {
                    s.user(1, me, g);
                }
            });
        }
    }
}

impl Object for Roaring2 {
    fn name(&self) -> &'static str { "obj_knight_roaring2" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        scr_darksize(me);
        battle::bullet_init(me);
        me.image_speed = 0.0;
        self.jumpimages = false;
        self.my_surface = -4;
        self.ball_surface = -4;
        self.star_surface = -4;
        self.terrible_surface = -4;
        self.darkness = 0.0;
        self.ball_darkness = 0.0;
        self.timer = 0.0;
        self.fake_x = g.camerawidth() * 0.5;
        self.fake_y = 24.0;
        self.fake_alpha = 0.0;
        self.rand_angle = g.irandom(360.0);
        self.rand_dist = 320.0;
        self.star_flicker = 2.0;
        self.intensity = 1.5;
        self.intensify = 1.5;
        self.attack_timer = 0.0;
        self.attack_timer_goal = -2.0;
        self.attack_token = 1.0;
        self.roaring_timer = 0.0;
        self.line_timer = -1.0;
        self.r = 128.0;
        self.g = 128.0;
        self.b = 128.0;
        self.knight_sprite = spr("spr_roaringknight_front");
        self.knight_sprite_image = 0.0;
        self.knight_sprite_speed = 0.5;
        self.bobble_count = 0.0;
        self.bobble_freq = 1.0;
        self.bobble_amp = 4.0;
        self.ball_counter = 0.0;
        self.ball_speed = 2.0;
        self.player_suck = 0.5;
        self.hsv = 128.0;
        self.hsv_switch = false;
        self.sound = SND_NONE;
        self.sound_pitch = 1.0;
        self.bullet_list = vec![];
        self.do_fake_screen = false;
        self.fakey_screen = -4;
        self.fakey_screen_2 = -4;
        self.stop = false;
        self.creatorid = NOONE;
        if let Some((_, h)) = g.get_first::<Heart>("obj_heart") {
            h.boundaryup = 160.0;
        }
        me.y -= 320.0;
        if let Some((_, k)) = g.get_first::<KnightEnemy>("obj_knight_enemy") {
            k.chargeupcon = 2.0;
            k.chargeuptimer = 0.0;
        }
        let id = me.id;
        g.script_delayed(id, 20, move |g| {
            g.lerpvar_instance(id, "darkness", 0.0, 1.0, 32.0, 0, "out");
        });
        self.starcount_p1 = 0.0;
        self.starcount_p2 = 0.0;
        self.starvar = 0.0;
    }

    /// Step_0
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        let (vx, vy, vw, vh) = (g.camerax(), g.cameray(), g.camerawidth(), g.cameraheight());
        for id in g.ids_of("obj_heart") {
            if let Some(h) = g.inst_mut(id) {
                if h.x < vx {
                    h.x = vx;
                }
                if h.x > vx + vw - 20.0 {
                    h.x = vx + vw - 20.0;
                }
                if h.y < vy {
                    h.y = vy;
                }
                if h.y > vy + vh {
                    h.y = vy + vh - 20.0;
                }
            }
        }
        self.step_intro(me, g);
        if self.timer >= 120.0 && self.intensity < 3.75 && self.timer % 3.0 == 0.0 {
            self.grow_afterimage(me, g, false);
        }
        self.step_follow_and_sound(me, g);
        if self.timer > 128.0 {
            self.step_charge(me, g, 1.0);
            if self.attack_timer == 4.0 {
                self.step_attack(g);
            }
        }
        if feq(self.intensity, 4.0) {
            self.step_roar(me, g, false);
        }
        self.step_spiral_stars(g, false);
    }

    /// Other_10 (event_user(0)): an alternate, unused version of the Step event.
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 0 {
            return;
        }
        self.step_intro(me, g);
        if self.timer >= 120.0 && self.intensity < 3.75 && self.timer % 3.0 == 0.0 {
            self.grow_afterimage(me, g, true);
        }
        self.step_follow_and_sound(me, g);
        if self.timer > 128.0 {
            self.step_charge(me, g, 3.0);
            if self.attack_timer == 4.0 {
                self.step_attack_alt(g);
            }
        }
        if feq(self.intensity, 4.0) {
            self.step_roar(me, g, true);
        }
        self.step_spiral_stars(g, true);
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let (cw, ch) = (g.camerawidth(), g.cameraheight());
        if !g.gfx.surface_exists(self.my_surface) {
            self.my_surface = g.gfx.surface_create(cw as i32, ch as i32);
        }
        if !g.gfx.surface_exists(self.ball_surface) {
            self.ball_surface = g.gfx.surface_create(cw as i32, ch as i32);
        }
        if !g.gfx.surface_exists(self.star_surface) {
            self.star_surface = g.gfx.surface_create(cw as i32, ch as i32);
        }
        if !g.gfx.surface_exists(self.terrible_surface) {
            self.terrible_surface = g.gfx.surface_create(cw as i32, ch as i32);
        }
        g.draw_self(me);
        if self.stop {
            return;
        }
        let t = g.glob.time;
        let (vx, vy) = (g.camerax(), g.cameray());

        // ---- the swirling ball
        g.gfx.surface_set_target(self.ball_surface);
        g.gfx.draw_clear_alpha(C_BLACK, 0.0);
        let flow = spr("spr_knight_bullet_flow");
        g.draw_sprite_tiled(flow, 0.0, self.fake_x + t * 2.0, self.fake_y);
        g.gfx.gpu_set_blendmode(bm::ADD);
        for _ in 0..4 {
            g.draw_sprite_tiled(flow, 0.0, self.fake_x + t * 2.0, self.fake_y);
        }
        g.gfx.gpu_set_blendmode(bm::NORMAL);
        self.ball_counter += self.ball_speed;
        if self.ball_counter < 0.0 {
            self.ball_counter += 1800.0;
        }
        if self.ball_counter > 1800.0 {
            self.ball_counter -= 1800.0;
        }
        g.gfx.gpu_set_blendmode_ext(bm::ZERO, bm::SRC_COLOUR);
        for a in 0..6 {
            let rad = 1800.0 - (self.ball_counter + 300.0 * a as f64) % 1800.0;
            g.gfx.draw_circle_color(self.fake_x, self.fake_y + 57.0, rad, C_WHITE, 0x595959, false);
        }
        g.gfx.draw_circle_color(self.fake_x, self.fake_y + 57.0, 640.0, C_WHITE, C_BLACK, false);
        g.gfx.gpu_set_blendmode(bm::NORMAL);
        g.gfx.surface_reset_target();

        // ---- stars, particles & afterimages
        g.gfx.surface_set_target(self.star_surface);
        g.gfx.draw_clear_alpha(C_BLACK, 0.0);
        for id in g.ids_of("obj_knight_circle") {
            // obj_knight_circle has no Other_11: this does nothing
            g.user_event(id, 1);
        }
        for id in g.ids_of("obj_particle_generic") {
            if let Some((s, sub, x, y, xs, ys, rot, col, a)) = inst_draw_args(g, id) {
                g.draw_sprite_ext(s, sub, x - vx, y - vy, xs, ys, rot, col, a);
            }
        }
        Self::draw_star_pass(g, 1);
        Self::draw_star_pass(g, 2);
        g.gfx.gpu_set_colorwriteenable(true, true, true, false);
        g.draw_sprite_ext(spr("spr_knight_line_grate"), 0.0, 0.0, self.star_flicker, 2.0, 2.0, 0.0, C_BLACK, 1.0);
        self.star_flicker = 2.0 - self.star_flicker;
        g.gfx.gpu_set_colorwriteenable(true, true, true, true);
        Self::draw_star_pass(g, 3);
        for id in g.ids_of("obj_knight_pointing_starchild") {
            g.with(id, |sme, so, g| {
                g.gfx.draw_set_blend_mode(bm::ADD);
                let timer = so.var("timer").map(|v| *v).unwrap_or(0.0);
                let glow = scr_pingpong(timer, 2.0) / 4.0;
                let (sx, sy) = (sme.x - g.camerax(), sme.y - g.cameray());
                scr_draw_outline_ext(g, sme.sprite_index, sme.image_index, sx, sy, sme.image_xscale, sme.image_yscale, sme.image_angle, C_WHITE, glow * sme.image_alpha, sme.image_xscale);
                g.draw_sprite_ext(sme.sprite_index, sme.image_index, sx, sy, sme.image_xscale, sme.image_yscale, sme.image_angle, C_WHITE, sme.image_alpha);
                g.gfx.draw_set_blend_mode(bm::NORMAL);
                sme.image_alpha = clamp01(remap(45.0, 60.0, 1.0, 0.0, timer));
                if sme.image_alpha < 1.0 {
                    sme.active = 0.0;
                }
                if sme.image_alpha == 0.0 {
                    g.destroy_self(sme);
                }
            });
        }
        for id in g.ids_of("obj_afterimage") {
            if let Some((s, sub, x, y, xs, ys, rot, _, a)) = inst_draw_args(g, id) {
                g.draw_sprite_ext(s, sub, x - vx, y - vy, xs, ys, rot, C_WHITE, a);
            }
        }
        g.gfx.surface_reset_target();

        // ---- composite
        g.gfx.surface_set_target(self.my_surface);
        g.gfx.draw_clear_alpha(C_BLACK, 0.0);
        ossafe_fill_rect(g, 0.0, 0.0, cw, ch, C_BLACK);
        if !self.hsv_switch {
            self.hsv += 1.0;
        } else {
            self.hsv -= 1.0;
        }
        if self.hsv >= 288.0 {
            self.hsv_switch = true;
        }
        if self.hsv <= 128.0 {
            self.hsv_switch = false;
        }
        g.gfx.gpu_set_blendmode(bm::ADD);
        let bh = g.gfx.surface_get_height(self.ball_surface);
        let bw = g.gfx.surface_get_width(self.ball_surface);
        let mut a = 0.0;
        while a < bh {
            let color = make_color_hsv(self.hsv % 255.0, 255.0, 255.0);
            let xo = ((a + t) * 0.1).sin() * 4.0 * self.intensity + ((a + t) * 0.35).sin() * 0.5 * self.intensity;
            g.gfx.draw_surface_part_ext(self.ball_surface, 0.0, a, bw, 1.0, xo, a, 1.0, 1.0, color, self.ball_darkness);
            a += 1.0;
        }
        g.gfx.draw_surface(self.star_surface, 0.0, 0.0);
        g.gfx.gpu_set_blendmode(bm::NORMAL);
        for id in g.ids_of("obj_afterimage_grow") {
            if let Some((s, sub, x, y, xs, ys, rot, _, al)) = inst_draw_args(g, id) {
                g.draw_sprite_ext(s, sub, x - vx, y - vy, xs, ys, rot, C_WHITE, al);
            }
        }
        if self.line_timer > -1.0 {
            g.gfx.gpu_set_colorwriteenable(true, true, true, false);
            let dir = -63.0;
            let color = make_color_rgb(self.r, self.g, self.b);
            let lx = cw * 0.5 - lengthdir_x(280.0, -63.0);
            let ly = ch * 0.5 - lengthdir_y(280.0, -63.0);
            let ys = 4.0 + 8.0 * (1.0 - self.line_timer.min(16.0) / 16.0);
            g.draw_sprite_ext(spr("spr_rk_quickslash_marker_gradient"), 0.0, lx, ly, self.line_timer, ys, dir, color, 1.0);
            g.draw_sprite_ext(spr("spr_rk_quickslash_marker"), 0.0, lx, ly, self.line_timer, ys, dir, C_BLACK, 1.0);
            g.gfx.gpu_set_colorwriteenable(true, true, true, true);
        }
        self.knight_sprite_image += self.knight_sprite_speed;
        if self.intensity < 3.75 {
            self.intensify = self.intensity;
        } else {
            self.intensify = scr_approach(self.intensify, 0.0, 0.1);
        }
        if !self.do_fake_screen {
            self.draw_fake_knight(g, me, 0.0, 0.0);
        }
        g.gfx.surface_reset_target();
        g.gfx.draw_surface_ext(self.my_surface, vx, vy, 1.0, 1.0, 0.0, C_WHITE, self.darkness);
        draw_hearts(g, 0.0, 0.0);

        // ---- the final cut: split the screen in two
        if self.do_fake_screen {
            let midway = cw * 0.5;
            g.destroy_all("obj_knight_pointing_starchild");
            self.draw_fake_knight(g, me, vx, vy);
            g.gfx.surface_set_target(self.terrible_surface);
            g.gfx.draw_clear_alpha(C_BLACK, 0.0);
            g.gfx.draw_surface_ext(self.my_surface, 0.0, 0.0, 1.0, 1.0, 0.0, C_WHITE, self.darkness);
            draw_hearts(g, -vx, -vy);
            g.gfx.gpu_set_blendenable(false);
            g.gfx.draw_set_alpha(0.0);
            ossafe_fill_rect(g, midway + 120.0, -1.0, cw, ch, C_BLACK);
            g.gfx.draw_triangle_color(midway - 120.0, -1.0, midway + 120.0, ch, midway + 120.0, -1.0, C_BLACK, 0, 0, false);
            g.gfx.draw_set_alpha(1.0);
            g.gfx.gpu_set_blendenable(true);
            self.fakey_screen = g.sprite_create_from_surface(self.terrible_surface, 0.0, 0.0, cw, ch, cw * 0.25, ch * 0.5);
            g.gfx.draw_clear_alpha(C_BLACK, 0.0);
            g.gfx.draw_surface_ext(self.my_surface, 0.0, 0.0, 1.0, 1.0, 0.0, C_WHITE, self.darkness);
            draw_hearts(g, -vx, -vy);
            g.gfx.gpu_set_blendenable(false);
            g.gfx.draw_set_alpha(0.0);
            ossafe_fill_rect(g, midway - 119.0, 0.0, -1.0, ch, C_BLACK);
            g.gfx.draw_triangle_color(midway - 120.0, 0.0, midway - 120.0, ch, midway + 120.0, ch, C_BLACK, 0, 0, false);
            g.gfx.draw_set_alpha(1.0);
            g.gfx.gpu_set_blendenable(true);
            g.gfx.surface_reset_target();
            self.fakey_screen_2 = g.sprite_create_from_surface(self.terrible_surface, 0.0, 0.0, cw, ch, cw * 0.75, ch * 0.5);
            self.stop = true;
            // TODO(battle): the SOUL is destroyed here; the battle core must recreate obj_heart for the next turn.
            g.destroy_all("obj_heart");
            let m1 = g.marker(vx + cw * 0.25, vy + ch * 0.5, self.fakey_screen);
            if let Some(i) = g.inst_mut(m1) {
                i.set_direction(180.0);
            }
            g.lerpvar_instance(m1, "speed", 15.0, 0.5, 12.0, 1, "out");
            g.var_delayed(m1, "gravity", 1.0, 12);
            if let Some(i) = g.inst_mut(m1) {
                i.gravity_direction = 180.0;
            }
            let m2 = g.marker(vx + cw * 0.75, vy + ch * 0.5, self.fakey_screen_2);
            if let Some(i) = g.inst_mut(m2) {
                i.set_direction(0.0);
            }
            g.lerpvar_instance(m2, "speed", 14.0, 0.5, 12.0, 1, "out");
            g.var_delayed(m2, "gravity", 1.0, 12);
            if let Some(i) = g.inst_mut(m2) {
                i.gravity_direction = 0.0;
            }
        }
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        for s in [self.my_surface, self.ball_surface, self.star_surface, self.terrible_surface] {
            if g.gfx.surface_exists(s) {
                g.gfx.surface_free(s);
            }
        }
        self.bullet_list.clear();
        if self.fakey_screen >= 0 {
            g.sprite_delete(self.fakey_screen);
        }
        if self.fakey_screen_2 >= 0 {
            g.sprite_delete(self.fakey_screen_2);
        }
        for id in g.ids_of("obj_knight_enemy") {
            if let Some(i) = g.inst_mut(id) {
                i.image_alpha = 1.0;
            }
            if let Some((_, k)) = g.get::<KnightEnemy>(id) {
                k.siner2 = 0.0;
                k.chargeupcon = 0.0;
            }
        }
        g.snd_stop("snd_knight_stretch");
        g.snd_stop("snd_knight_roar");
        g.snd_stop("snd_stardrop");
        g.snd_stop("snd_knight_cut");
        // TODO(battle): the bullet board is destroyed here; the battle core must recreate obj_growtangle.
        g.destroy_all("obj_growtangle");
    }

    obj_vars!(darkness, ball_darkness, fake_x, fake_y, fake_alpha, knight_sprite_image, bobble_freq, bobble_amp, player_suck, ball_speed, r, g, b, timer, intensity);
}

// ============================================================================ obj_knight_roaring_star

/// obj_knight_roaring_star (parent obj_regularbullet; its Create/Step don't call event_inherited).
#[derive(Default)]
pub struct RoaringStar {
    // set from the outside (obj_regularbullet variables, never acted on: no inherited Step)
    pub wall_destroy: f64,
    pub bottomfade: f64,
    pub spinspeed: f64,
    pub spin: f64,
    pub even: bool,
    pub timer: f64,
    pub con: f64,
    pub growstart: f64,
    pub play_sound: bool,
    pub beamflicker: f64,
    pub split: f64,
    pub outbound: bool,
    pub splitmax: f64,
    pub splitease: f64,
    pub finalx: f64,
}

impl RoaringStar {
    /// Other_10: draw (con == 0), in screen space.
    fn draw_idle(&self, me: &Inst, g: &mut Game) {
        let xs = (g.sprite_width(me) + 16.0) / g.sprite_get_width(me.sprite_index);
        let ys = (g.sprite_height(me) + 16.0) / g.sprite_get_height(me.sprite_index);
        let (sx, sy) = (me.x - g.camerax(), me.y - g.cameray());
        if self.split < 2.0 {
            g.draw_sprite_ext(me.sprite_index, 0.0, sx, sy, xs, ys, me.image_angle, me.image_blend, me.image_alpha);
        } else {
            g.draw_sprite_ext(spr("spr_knight_bullet_star_top"), 0.0, sx + self.splitease / 2.0, sy + self.splitease, xs, ys, me.image_angle, me.image_blend, me.image_alpha);
            g.draw_sprite_ext(spr("spr_knight_bullet_star_bottom"), 0.0, sx - self.splitease / 2.0, sy - self.splitease, xs, ys, me.image_angle, me.image_blend, me.image_alpha);
        }
    }

    /// Other_11: draw (con >= 1: charging up and exploding), in screen space.
    fn draw_burst(&self, me: &Inst, g: &mut Game) {
        let xs = (g.sprite_width(me) + 16.0) / g.sprite_get_width(me.sprite_index);
        let ys = (g.sprite_height(me) + 16.0) / g.sprite_get_height(me.sprite_index);
        let alpha = ((self.timer * 3.0).sin() + 1.0) * 0.25;
        let (sx, sy) = (me.x - g.camerax(), me.y - g.cameray());
        let (topx, topy) = (sx + self.splitease / 2.0, sy + self.splitease);
        let (botx, boty) = (sx - self.splitease / 2.0, sy - self.splitease);
        let con = self.con;
        if con == 2.0 || feq(con, 2.5) || con == 3.0 {
            let mut a = 1.0;
            let mut length = 120.0;
            if con == 2.0 {
                a = clamp01(self.timer / 30.0 - alpha);
                length = 50.0 * clamp01(self.timer / 30.0 - (self.timer % 2.0) * 0.75) + 50.0;
            }
            g.gfx.draw_set_blend_mode(bm::ADD);
            scr_draw_beam_color(g, topx, topy, length, 10.0, 90.0, C_WHITE, 0, a, false);
            scr_draw_beam_color(g, botx, boty, length, 10.0, 156.0, C_WHITE, 0, a, false);
            scr_draw_beam_color(g, topx, topy, length, 10.0, 24.0, C_WHITE, 0, a, false);
            scr_draw_beam_color(g, botx, boty, length, 10.0, 270.0, C_WHITE, 0, a, false);
            scr_draw_beam_color(g, topx, topy, length, 10.0, 336.0, C_WHITE, 0, a, false);
            scr_draw_beam_color(g, botx, boty, length, 10.0, 204.0, C_WHITE, 0, a, false);
            g.gfx.draw_set_blend_mode(bm::NORMAL);
        }
        let bottom = spr("spr_knight_bullet_star_bottom");
        if con == 1.0 || con == 2.0 || feq(con, 2.5) {
            let color = merge_color(C_GRAY, C_RED, clamp01(self.timer / 30.0));
            g.draw_sprite_ext(me.sprite_index, 1.0, topx, topy, xs + 0.1, ys + 0.1, 0.0, C_WHITE, alpha);
            g.draw_sprite_ext(me.sprite_index, 0.0, topx, topy, xs, ys, 0.0, color, 1.0);
            if self.split >= 2.0 {
                g.draw_sprite_ext(bottom, 1.0, botx, boty, xs + 0.1, ys + 0.1, 0.0, C_WHITE, alpha);
                g.draw_sprite_ext(bottom, 0.0, botx, boty, xs, ys, 0.0, color, 1.0);
            }
        }
        if con == 3.0 || con == 4.0 {
            let fa = ((self.timer * 6.0).sin() + 1.0) * 0.25;
            g.draw_sprite_ext(me.sprite_index, 2.0, topx, topy, xs + 0.1, ys + 0.1, 0.0, C_WHITE, fa);
            g.draw_sprite_ext(me.sprite_index, 2.0, topx, topy, xs, ys, 0.0, C_WHITE, 1.0);
            if self.split >= 2.0 {
                g.draw_sprite_ext(bottom, 2.0, botx, boty, xs + 0.1, ys + 0.1, 0.0, C_WHITE, fa);
                g.draw_sprite_ext(bottom, 2.0, botx, boty, xs, ys, 0.0, C_WHITE, 1.0);
            }
        }
    }

    /// Other_15: touched the SOUL.
    fn hit(&mut self, me: &mut Inst, g: &mut Game) {
        if me.active == 1.0 {
            let hb = heart_hitbox(g, 0.0, 2.0);
            if !scr_precise_hit(me, g, hb) {
                return;
            }
            if g.exists("obj_knight_roaring2") {
                knight_roaring_hit(g);
            } else {
                if me.target != 3.0 {
                    battle::scr_damage(me, g);
                }
                if me.target == 3.0 {
                    battle::scr_damage_all(me, g);
                }
            }
            if me.destroyonhit == 1.0 {
                g.destroy_self(me);
            }
        }
    }
}

impl Object for RoaringStar {
    fn name(&self) -> &'static str { "obj_knight_roaring_star" }

    fn create(&mut self, me: &mut Inst, _g: &mut Game) {
        battle::bullet_init(me);
        me.image_xscale = 0.0;
        me.image_yscale = 0.0;
        self.even = false;
        me.destroyonhit = 0.0;
        self.timer = 0.0;
        self.con = 0.0;
        self.growstart = 0.0;
        self.play_sound = true;
        self.beamflicker = 0.0;
        self.split = 0.0;
        self.outbound = false;
        self.splitmax = 14.0;
        self.splitease = 0.0;
        self.finalx = 0.0;
        me.damage = 206.0;
        me.element = 5.0;
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        let (cx, cy, cw) = (g.camerax(), g.cameray(), g.camerawidth());
        let (sw, sh) = (g.sprite_width(me), g.sprite_height(me));
        if me.x < cx - sw || me.x > cx + cw + sw || me.y < cy - sh || me.y > cy + 480.0 + sh {
            if self.outbound {
                g.destroy_self(me);
                return;
            }
        } else {
            self.outbound = true;
        }
        if self.con == 1.0 {
            me.friction = 0.5;
            self.con += 1.0;
        } else if self.con == 2.0 {
            if me.speed() == 0.0 && me.gravity == 0.0 {
                me.gravity = 0.1;
                me.gravity_direction = me.direction() - 180.0;
                me.friction = 0.0;
                if truthy(self.split) {
                    let finaltime = 54.0 - self.timer;
                    let len = 0.5 * me.gravity * finaltime * finaltime;
                    let subfinalx = (me.x - cx) + lengthdir_x(len, me.gravity_direction);
                    let subfinaly = (me.y - cy) + lengthdir_y(len, me.gravity_direction);
                    self.finalx = subfinalx - subfinaly / 2.0;
                }
            }
            self.timer += 1.0;
            if self.timer >= 40.0 && !truthy(self.split) {
                self.timer = 0.0;
                self.con += 1.0;
                if self.play_sound {
                    g.snd_play("snd_explosion_firework");
                }
            }
            self.growstart = me.image_xscale;
        } else if feq(self.con, 2.5) {
            if self.split == 1.0 {
                me.set_speed(0.0);
                me.gravity = 0.0;
                me.sprite_index = spr("spr_knight_bullet_star_top");
                self.timer = -10.0;
                self.split = 2.0;
            }
            self.timer += 1.0;
            self.splitease = scr_ease_out(clamp01(self.timer / 20.0), 4) * self.splitmax * me.image_xscale;
            if self.timer == 20.0 {
                self.con = 3.0;
                self.timer = 0.0;
            }
        } else if self.con == 3.0 {
            self.timer += 1.0;
            me.image_xscale = self.growstart + clamp01(self.timer / 2.0);
            me.image_yscale = self.growstart + clamp01(self.timer / 2.0);
            if self.timer == 3.0 {
                let mut angle = 90.0;
                for i in 0..6 {
                    // GML computes split-adjusted _xx/_yy here but spawns at (x, y) anyway.
                    let d = battle::scr_childbullet(g, me, me.x, me.y, Box::new(Starchild::default()));
                    if let Some((di, dc)) = g.get::<Starchild>(d) {
                        di.image_angle = angle;
                        di.set_direction(angle);
                        di.set_speed(1.0);
                        di.friction = -0.1;
                        di.image_xscale = me.image_xscale * 0.5;
                        di.image_yscale = me.image_yscale * 0.5;
                        dc.deceleration = 0.15;
                    }
                    if i == 1 || i == 4 {
                        angle += 57.0;
                    } else {
                        angle += 66.0;
                    }
                }
            }
            if self.timer >= 4.0 {
                me.image_xscale = (g.sprite_width(me) + 16.0) / g.sprite_get_width(me.sprite_index);
                me.image_yscale = (g.sprite_height(me) + 16.0) / g.sprite_get_height(me.sprite_index);
                let aimage = g.afterimage(me);
                let fs = g.get_var(aimage, "fadeSpeed").unwrap_or(0.04);
                g.set_var(aimage, "fadeSpeed", fs * 3.0);
                g.destroy_self(me);
            }
        }
    }

    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            0 => self.draw_idle(me, g),
            1 => self.draw_burst(me, g),
            5 => self.hit(me, g),
            _ => {}
        }
    }

    obj_vars!(timer, con, split, splitease, spinspeed);
}

// ============================================================================ obj_knight_pointing_starchild

/// obj_knight_pointing_starchild (parent obj_regularbullet).
pub struct Starchild {
    pub rb: RegVars,
    pub deceleration: f64,
    pub minspeed: f64,
    pub timer: f64,
    pub drawtimer: f64,
    pub lifetime: f64,
    pub difficulty: f64,
    pub con: f64,
    pub tracking: bool,
    pub start_angle: f64,
    pub target_angle: f64,
    pub rotation: f64,
    pub delay: f64,
    pub init: bool,
    pub rotatespeed: f64,
    pub ease: f64,
    pub xscale_start: f64,
    pub yscale_start: f64,
    pub xprev: f64,
    pub yprev: f64,
    pub outline: Color,
    pub accel: f64,
}

impl Default for Starchild {
    fn default() -> Self {
        Starchild {
            rb: RegVars::default(),
            deceleration: 0.1,
            minspeed: 1.0,
            timer: 0.0,
            drawtimer: 0.0,
            lifetime: 60.0,
            difficulty: 0.0,
            con: 0.0,
            tracking: true,
            start_angle: 0.0,
            target_angle: 0.0,
            rotation: 0.0,
            delay: 0.0,
            init: false,
            rotatespeed: 10.0,
            ease: 0.0,
            xscale_start: 0.0,
            yscale_start: 0.0,
            xprev: 0.0,
            yprev: 0.0,
            outline: 0,
            accel: 0.5,
        }
    }
}

impl Starchild {
    fn hit_once(&mut self, me: &mut Inst, g: &mut Game, small: f64, normal: f64) {
        if me.active == 1.0 {
            let hb = heart_hitbox(g, small, normal);
            if !scr_precise_hit(me, g, hb) {
                return;
            }
            if g.exists("obj_knight_roaring2") {
                knight_roaring_hit(g);
            } else {
                if me.target != 3.0 {
                    battle::scr_damage(me, g);
                }
                if me.target == 3.0 {
                    battle::scr_damage_all(me, g);
                }
            }
            if me.destroyonhit == 1.0 {
                g.destroy_self(me);
            }
        }
    }
}

impl Object for Starchild {
    fn name(&self) -> &'static str { "obj_knight_pointing_starchild" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::regularbullet_create(me, &mut self.rb, g);
        battle::bullet_init(me);
        *self = Starchild { rb: self.rb, ..Starchild::default() };
        me.damage = 1.0;
        me.element = 5.0;
        self.xprev = me.x;
        self.yprev = me.y;
        me.sprite_index = spr("spr_knight_starchild_parts");
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if !self.init {
            self.init = true;
            if self.difficulty >= 2.0 {
                self.delay = 25.0;
                let mut my_delay = self.delay;
                for cid in g.ids_of("obj_dbulletcontroller") {
                    g.with_t::<DBulletController, _>(cid, |_, ctl, _| {
                        my_delay += ctl.c.delay;
                        if ctl.c.subdelay == 4.0 {
                            ctl.c.subdelay = 0.0;
                            ctl.c.delay += 5.0;
                        } else {
                            ctl.c.subdelay += 1.0;
                            ctl.c.delay += 1.0;
                        }
                    });
                }
                self.delay = my_delay;
            }
        }
        battle::regularbullet_step(me, &mut self.rb, g);
        if g.exists("obj_knight_roaring2") {
            return;
        }
        // ---- outside the Roaring (unreachable during it: these die with obj_knight_roaring2)
        if self.con <= 2.0 && self.con <= 3.0 {
            if me.speed() > self.minspeed {
                me.set_speed(scr_movetowards(me.speed(), self.minspeed, self.deceleration));
            }
            if self.con == 0.0 && self.delay > 0.0 {
                self.timer += 1.0;
                if self.timer >= self.delay {
                    if !scr_onscreen_tolerance(g, me, 10.0) {
                        g.destroy_self(me);
                    }
                    self.timer = 0.0;
                    self.con = 1.0;
                }
            }
        }
        if self.con >= 1.0 && self.con <= 3.0 {
            // TODO(battle): obj_heart_follower doesn't exist yet; falls back to obj_heart.
            let f = g.first_inst("obj_heart_follower").map(|h| (h.x, h.y));
            let f = f.or_else(|| g.first_inst("obj_heart").map(|h| (h.x, h.y)));
            if let Some((hx, hy)) = f {
                self.target_angle = point_direction(me.x, me.y, hx + 10.0, hy + 10.0);
            }
            if self.con >= 2.0 && self.tracking {
                let diff = angle_difference(self.target_angle, me.direction());
                if diff.abs() < 90.0 {
                    if self.con < 3.0 {
                        me.set_direction(scr_rotatetowards(me.direction(), self.target_angle, 2.0));
                        me.image_angle = me.direction();
                    } else if diff.abs() <= 4.0 {
                        self.rotation = 0.0;
                    } else if diff.abs() > 30.0 {
                        self.rotation = sign(diff) * 2.0;
                    } else {
                        self.rotation = sign(diff);
                    }
                } else if self.con >= 3.0 {
                    self.tracking = false;
                    self.rotation = sign(self.rotation);
                }
            } else {
                me.set_direction(me.direction() + self.rotation);
                me.image_angle += self.rotation;
            }
        }
        if self.con == 1.0 {
            me.image_angle = scr_angle_lerp(me.direction(), self.target_angle, self.timer / 10.0);
            self.timer += 1.0;
            if self.timer >= 10.0 {
                self.timer = 0.0;
                self.con = 2.0;
                me.set_direction(me.image_angle);
                self.tracking = true;
            }
            if self.xscale_start == 0.0 {
                self.xscale_start = me.image_xscale;
            }
            if self.yscale_start == 0.0 {
                self.yscale_start = me.image_yscale;
            }
            let c = (self.timer / 5.0 * PI).cos();
            me.image_yscale = self.yscale_start * c;
            me.image_blend = merge_color(C_WHITE, C_BLACK, c);
            self.outline = merge_color(C_BLACK, C_RED, c);
        }
        if self.con == 2.0 {
            self.timer += 1.0;
            if self.timer >= 10.0 {
                self.timer = 0.0;
                self.con = 3.0;
            }
        }
        if self.con >= 1.0 && self.ease < 40.0 {
            let sp = (1.0 - self.ease / 40.0) * 2.0;
            me.x -= lengthdir_x(sp, self.target_angle);
            me.y -= lengthdir_y(sp, self.target_angle);
            self.ease += 1.0;
        }
        if self.con == 3.0 {
            me.set_speed(scr_movetowards(me.speed(), 25.0, 0.5));
            me.image_xscale = self.xscale_start + me.speed() / 60.0;
            me.image_yscale = self.yscale_start - me.speed() / 90.0;
            scr_custom_afterimage_blend(me, g);
            let afbm = scr_custom_afterimage_blend(me, g);
            let dir = me.image_angle;
            if let Some((a, o)) = g.get::<AfterimageBlend>(afbm) {
                a.image_blend = C_RED;
                o.fade_speed = 0.25;
                a.image_alpha = 0.5;
                a.set_direction(dir);
                a.set_speed(1.0);
            }
        }
        if self.con == 4.0 {
            me.set_speed(0.0);
            self.timer += 1.0;
            if self.timer >= 4.0 {
                g.destroy_self(me);
            }
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.con == 4.0 {
            let scale = (me.image_yscale + me.image_xscale) / 2.0;
            g.draw_sprite_ext(spr("spr_thrash_missile_explosion"), self.timer, me.x, me.y, scale, scale, me.image_angle - 90.0, C_RED, 1.0);
            return;
        }
        g.gfx.draw_set_blend_mode(bm::ADD);
        self.drawtimer += 1.0;
        let glow = scr_pingpong(self.drawtimer, 2.0) / 4.0;
        let mut glowcol: Color = 16777215;
        if self.con >= 1.0 {
            if self.con > 1.0 {
                glowcol = 255;
            } else {
                glowcol = merge_color(C_WHITE, C_RED, self.timer / 10.0);
            }
        }
        scr_draw_outline(g, me, me.image_xscale, glowcol, glow * me.image_alpha);
        if self.con > 0.0 {
            g.draw_sprite_ext(me.sprite_index, 1.0, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, self.outline, me.image_alpha);
        }
        g.gfx.draw_set_blend_mode(bm::NORMAL);
        g.draw_sprite_ext(me.sprite_index, 0.0, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, me.image_alpha);
        if self.difficulty < 2.0 {
            me.image_alpha = clamp01(remap(self.lifetime - 15.0, self.lifetime, 1.0, 0.0, self.drawtimer));
            if me.image_alpha < 1.0 {
                me.active = 0.0;
            }
            if me.image_alpha == 0.0 {
                g.destroy_self(me);
            }
        }
    }

    /// Other_15 (overrides obj_collidebullet's).
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 5 {
            return;
        }
        if g.exists("obj_knight_roaring2") {
            self.hit_once(me, g, 0.0, 2.0);
        } else {
            me.target = 3.0;
            me.damage = 75.0;
            set_knight_aoedamage(g, true);
            self.hit_once(me, g, 1.0, 5.0);
            set_knight_aoedamage(g, false);
        }
    }

    obj_vars!(timer, drawtimer, con, deceleration, delay);
}

// ============================================================================ obj_roaringknight_slash

/// obj_roaringknight_slash (parent obj_collidebullet).
pub struct RoaringSlash {
    pub width: f64,
    pub aoe: bool,
    pub slashdir: f64,
}

impl Default for RoaringSlash {
    fn default() -> Self { RoaringSlash { width: 24.0, aoe: true, slashdir: 1.0 } }
}

impl Object for RoaringSlash {
    fn name(&self) -> &'static str { "obj_roaringknight_slash" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::bullet_init(me);
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
        if n == 1 {
            me.mask_index = spr("spr_nomask");
        }
    }

    /// Step_2
    fn end_step(&mut self, me: &mut Inst, g: &mut Game) {
        me.damage = 206.0;
        me.grazepoints = 50.0;
        // `!alarm[0]`: true once alarm 0 has fired (it is then -1)
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
                let jx: f64 = g.choose(&[-2.0, -1.0, 0.0, 1.0, 2.0]);
                let jy: f64 = g.choose(&[-2.0, -1.0, 0.0, 1.0, 2.0]);
                if let Some(b) = g.inst_mut(gt) {
                    b.x = b.xstart + jx;
                    b.y = b.ystart + jy;
                }
            }
            scr_heartclamp(g);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let dir = me.direction();
        let hx = lengthdir_x(640.0, dir);
        let hy = lengthdir_y(640.0, dir);
        let hxoff = lengthdir_x(self.width, dir + 90.0);
        let hyoff = lengthdir_y(self.width, dir + 90.0);
        let color = make_color_rgb(255.0, (1.0 - me.image_alpha) * 255.0, (1.0 - me.image_alpha) * 255.0);
        g.gfx.draw_set_alpha(me.image_alpha * 2.0);
        let a = me.image_alpha;
        if truthy(self.slashdir) {
            g.gfx.draw_triangle_color(me.x - hx * a, me.y - hy * a, me.x + hx + hxoff, me.y + hy + hyoff, me.x + hx - hxoff, me.y + hy - hyoff, color, color, color, false);
        } else {
            g.gfx.draw_triangle_color(me.x + hx * a, me.y + hy * a, me.x - hx + hxoff, me.y - hy + hyoff, me.x - hx - hxoff, me.y - hy - hyoff, color, color, color, false);
        }
        g.gfx.draw_set_alpha(1.0);
    }

    /// Other_15 (overrides obj_collidebullet's).
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 5 {
            return;
        }
        me.damage = 206.0;
        if self.aoe {
            me.damage = 75.0;
            me.target = 3.0;
            set_knight_aoedamage(g, true);
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
        set_knight_aoedamage(g, false);
    }

    obj_vars!(width, slashdir);
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
    pub my_surface: i32,
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
        if self.r == 0.0 && self.b == 0.0 && self.b == 0.0 {
            g.destroy_self(me);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if self.draw_in_box {
            let Some((gxs, gys)) = g.first_inst("obj_growtangle").map(|b| (b.image_xscale, b.image_yscale)) else { return };
            if !g.gfx.surface_exists(self.my_surface) {
                self.my_surface = g.gfx.surface_create((75.0 * gxs) as i32, (75.0 * gys) as i32);
            }
            let (l, t) = (battle::scr_get_box(g, 2), battle::scr_get_box(g, 1));
            g.gfx.surface_set_target(self.my_surface);
            self.color_2 = make_color_rgb(self.r, self.g, self.b);
            g.gfx.draw_circle_color(me.x - l, me.y - t, self.circle_size, self.color_1, self.color_2, false);
            g.gfx.surface_reset_target();
            g.gfx.gpu_set_blendmode(bm::ADD);
            g.gfx.draw_surface(self.my_surface, l, t);
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

    obj_vars!(circle_size, r, g, b, r_goal, g_goal, b_goal);
}

// ============================================================================ obj_particle_generic

/// obj_particle_generic. NOTE: its Other_40 (Outside Room → instance_destroy) has no runtime equivalent.
pub struct ParticleGeneric {
    pub fade_rate: f64,
    pub shrink_rate: f64,
    pub timer: f64,
    pub not_outbound: bool,
}

impl Default for ParticleGeneric {
    fn default() -> Self { ParticleGeneric { fade_rate: 0.0, shrink_rate: 0.0, timer: -1.0, not_outbound: true } }
}

impl Object for ParticleGeneric {
    fn name(&self) -> &'static str { "obj_particle_generic" }

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

    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 0 {
            g.destroy_self(me);
        }
    }

    obj_vars!(fade_rate, shrink_rate, timer);
}

// ============================================================================ obj_afterimage_screen

/// obj_afterimage_screen: a growing, fading copy of the whole screen.
pub struct AfterimageScreen {
    pub my_surface: i32,
    pub xscale: f64,
    pub yscale: f64,
    pub alpha: f64,
    pub anchor_x: f64,
    pub anchor_y: f64,
    pub xrate: f64,
    pub yrate: f64,
    pub faderate: f64,
    pub draw_end: bool,
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
            draw_end: false,
        }
    }
}

impl AfterimageScreen {
    fn draw_copy(&mut self, me: &Inst, g: &mut Game) {
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
        if self.draw_end {
            return;
        }
        self.draw_copy(me, g);
    }

    fn draw_end(&mut self, me: &mut Inst, g: &mut Game) {
        if !self.draw_end {
            return;
        }
        self.draw_copy(me, g);
        draw_hearts(g, 0.0, 0.0);
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.gfx.surface_exists(self.my_surface) {
            g.gfx.surface_free(self.my_surface);
        }
    }

    obj_vars!(xscale, yscale, alpha, xrate, yrate, faderate);
}

// ============================================================================ obj_afterimage_grow

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
        if self.target != NOONE {
            if let Some((tx, ty)) = g.inst(self.target).map(|t| (t.x, t.y)) {
                me.x = tx;
                me.y = ty;
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

// ============================================================================ obj_afterimage_blend

/// obj_afterimage_blend (parent obj_afterimage: inherits its fadeSpeed Create and fading Step).
pub struct AfterimageBlend {
    pub blendmode: i32,
    pub dest_blend: i32,
    pub fade_speed: f64,
}

impl Default for AfterimageBlend {
    fn default() -> Self { AfterimageBlend { blendmode: bm::ADD, dest_blend: -4, fade_speed: 0.04 } }
}

impl Object for AfterimageBlend {
    fn name(&self) -> &'static str { "obj_afterimage_blend" }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        me.image_alpha -= self.fade_speed;
        if me.image_alpha < 0.0 {
            g.destroy_self(me);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        // gpu_get_blendmode() isn't available: callers always draw with bm_normal here.
        if self.dest_blend != -4 {
            g.gfx.gpu_set_blendmode_ext(self.blendmode, self.dest_blend);
        } else {
            g.gfx.gpu_set_blendmode(self.blendmode);
        }
        g.draw_self(me);
        g.gfx.gpu_set_blendmode(bm::NORMAL);
    }

    fn var(&mut self, name: &str) -> Option<&mut f64> {
        match name {
            "fadeSpeed" => Some(&mut self.fade_speed),
            _ => None,
        }
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any { self }
}

// ============================================================================ obj_knight_roaring_fx (cutscene roar)

/// Plain obj_regularbullet as fired by obj_knight_roaring_fx (with its lerped, unused `anglechange`).
#[derive(Default)]
pub struct RoaringFireBullet {
    pub rb: RegVars,
    pub anglechange: f64,
}

impl Object for RoaringFireBullet {
    fn name(&self) -> &'static str { "obj_regularbullet" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) { battle::regularbullet_create(me, &mut self.rb, g); }
    fn step(&mut self, me: &mut Inst, g: &mut Game) { battle::regularbullet_step(me, &mut self.rb, g); }
    obj_vars!(anglechange);
}

/// obj_knight_crush
#[derive(Default)]
pub struct KnightCrush {
    pub my_surface: i32,
    pub radius: f64,
    pub alpha: f64,
    pub hsv: f64,
}

impl Object for KnightCrush {
    fn name(&self) -> &'static str { "obj_knight_crush" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.my_surface = -4;
        self.radius = 960.0;
        self.alpha = 0.0;
        self.hsv = 256.0;
        g.lerpvar(me, "radius", 960.0, 160.0, 24.0, 0, "out");
        g.lerpvar(me, "alpha", 0.0, 0.1, 24.0, 0, "out");
        g.lerpvar(me, "hsv", 256.0, 64.0, 64.0, 1, "out");
        me.alarm[0] = 24;
    }

    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 0 {
            g.lerpvar(me, "radius", 160.0, 0.0, 64.0, 1, "out");
            g.lerpvar(me, "alpha", 0.1, 1.0, 64.0, 1, "out");
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let app = g.gfx.app_surface;
        if !g.gfx.surface_exists(self.my_surface) {
            let (w, h) = (g.gfx.surface_get_width(app), g.gfx.surface_get_height(app));
            self.my_surface = g.gfx.surface_create(w as i32, h as i32);
        }
        let modifier = 1.0;
        let (vx, vy) = (g.camerax(), g.cameray());
        g.gfx.surface_set_target(self.my_surface);
        g.gfx.draw_clear_alpha(C_BLACK, 0.0);
        g.gfx.draw_circle_color(me.x - vx, me.y - vy, self.radius * modifier, C_BLACK, C_BLACK, false);
        g.gfx.gpu_set_colorwriteenable(true, true, true, false);
        let t = g.glob.time;
        g.draw_sprite_tiled(spr("spr_knight_bullet_flow"), 0.0, t * 8.0, 0.0);
        g.gfx.gpu_set_colorwriteenable(true, true, true, true);
        g.gfx.surface_reset_target();
        let color = make_color_hsv(self.hsv, 255.0, 255.0);
        g.gfx.gpu_set_blendmode(bm::ADD);
        for _ in 0..4 {
            g.gfx.draw_surface_ext(self.my_surface, vx, vy, 1.0, 1.0, 0.0, color, self.alpha);
        }
        g.gfx.draw_set_alpha(self.alpha);
        g.gfx.draw_circle_color(me.x, me.y, self.radius * modifier, C_WHITE, C_WHITE, false);
        g.gfx.draw_set_alpha(1.0);
        g.gfx.gpu_set_blendmode(bm::NORMAL);
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.gfx.surface_exists(self.my_surface) {
            g.gfx.surface_free(self.my_surface);
        }
    }

    obj_vars!(radius, alpha, hsv);
}

/// obj_knight_roaring_fx: the cutscene roar (obj_ch3_PTB02). Not created by the battle attack.
pub struct RoaringFx {
    pub timer: f64,
    pub gigatimer: f64,
    pub spin: f64,
    pub random_aim: f64,
    pub attack_speed: f64,
    pub counter: f64,
    pub state: &'static str,
    pub whiteout: bool,
    pub whiteout_counter: f64,
    pub shudder: f64,
    pub bar: f64,
    pub density: f64,
    pub createbullets: bool,
    pub roarendtimer: f64,
    pub roarendtimermax: f64,
}

impl Default for RoaringFx {
    fn default() -> Self {
        RoaringFx {
            timer: 0.0,
            gigatimer: 0.0,
            spin: 1.0,
            random_aim: 0.0,
            attack_speed: 0.0,
            counter: 0.0,
            state: "intro",
            whiteout: false,
            whiteout_counter: 0.0,
            shudder: 0.0,
            bar: 0.0,
            density: 12.0,
            createbullets: false,
            roarendtimer: 0.0,
            roarendtimermax: 190.0,
        }
    }
}

impl RoaringFx {
    fn pxy(me: &Inst, g: &Game) -> (f64, f64) { (me.x + g.sprite_width(me) * 0.42, me.y + g.sprite_height(me) * 0.5) }

    /// One spr_roaring_fire2 bullet of the ring.
    #[allow(clippy::too_many_arguments)]
    fn fire(&self, g: &mut Game, px: f64, py: f64, dir: f64, spd: f64, spd_to: f64, t: f64, xs: f64, ys: f64) {
        let id = battle::scr_fire_bullet(g, px, py, Box::new(RoaringFireBullet::default()), dir, spd, Some(spr("spr_roaring_fire2")), false, None, None);
        g.lerpvar_instance(id, "speed", spd, spd_to, t, 1, "out");
        if let Some(i) = g.inst_mut(id) {
            i.image_xscale = 0.0;
            i.image_yscale = ys;
        }
        g.lerpvar_instance(id, "image_xscale", 0.0, xs, 8.0, 1, "out");
        g.lerpvar_instance(id, "image_yscale", ys, xs, 8.0, 1, "out");
        let ac = 6.0 * self.spin;
        if let Some((_, b)) = g.get::<RoaringFireBullet>(id) {
            b.anglechange = ac;
        }
        g.lerpvar_instance(id, "anglechange", ac, 0.0, 40.0, 1, "out");
    }
}

impl Object for RoaringFx {
    fn name(&self) -> &'static str { "obj_knight_roaring_fx" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        scr_darksize(me);
        me.image_speed = 0.0;
        self.timer = 0.0;
        self.gigatimer = 0.0;
        self.spin = g.choose(&[1.0, -1.0]);
        self.random_aim = 0.0;
        self.attack_speed = 0.0;
        self.counter = 0.0;
        me.sprite_index = spr("spr_roaringknight_shift_ol");
        me.image_index = 1.0;
        self.state = "intro";
        for gt in g.ids_of("obj_growtangle") {
            if let Some((_, b)) = g.get::<Growtangle>(gt) {
                b.keep = 1.0;
            }
        }
        self.whiteout = false;
        self.whiteout_counter = 0.0;
        self.shudder = 0.0;
        self.bar = 0.0;
        self.density = 12.0;
        // TODO(battle): obj_battlecontroller doesn't exist as an instance yet.
        self.createbullets = g.exists("obj_battlecontroller");
        self.roarendtimer = 0.0;
        self.roarendtimermax = 190.0;
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.gigatimer += 1.0;
        if truthy(self.shudder) {
            self.shudder -= 1.0;
        }
        let (px, py) = Self::pxy(me, g);
        if self.whiteout {
            self.whiteout_counter = scr_approach(self.whiteout_counter, 1.0, 0.020833333333333332);
            if self.state != "roaring" {
                let n = 2.0 + g.irandom(2.0);
                for _ in 0..n as i32 {
                    let randdir = g.irandom(360.0);
                    let dist = 40.0 + g.irandom(240.0);
                    let randsize = 0.25 + g.random(0.75);
                    let id = g.instance_create(px + lengthdir_x(dist, randdir), py + lengthdir_y(dist, randdir), Box::new(ParticleGeneric::default()));
                    let mut xs = randsize;
                    if let Some(i) = g.inst_mut(id) {
                        i.image_xscale = randsize;
                        i.image_yscale = randsize;
                        let d = point_direction(i.x, i.y, px, py);
                        i.set_direction(d);
                        i.image_angle = d;
                        xs = i.image_xscale;
                    }
                    let to = 16.0 + g.irandom(8.0);
                    g.lerpvar_instance(id, "speed", 4.0, to, 32.0, 1, "in");
                    g.lerpvar_instance(id, "image_xscale", xs, xs * 16.0, 32.0, 1, "in");
                    g.lerpvar_instance(id, "image_yscale", xs, xs * 0.5, 32.0, 1, "in");
                }
            }
            for id in g.ids_of("obj_particle_generic") {
                let near = g.inst(id).map(|i| point_distance(i.x, i.y, px, py) <= 32.0).unwrap_or(false);
                if near {
                    g.destroy(id);
                }
            }
        }
        if self.state == "intro" {
            self.timer += 1.0;
            if self.timer == 16.0 {
                g.instance_create(px, py, Box::new(KnightCrush::default()));
            }
            if self.timer == 24.0 {
                self.whiteout = true;
                let stretch = g.snd_play("snd_knight_stretch");
                g.audio.pitch(stretch, 0.75);
            }
            if self.timer == 8.0 {
                self.shudder = 999.0;
            }
            if self.timer == 32.0 {
                self.shudder = 999.0;
            }
            if self.timer == 64.0 {
                self.state = "roaring";
                self.timer = -20.0;
            }
        }
        if self.state == "roaring" {
            self.timer += 1.0;
            if self.timer == 16.0 && !truthy(self.attack_speed) {
                self.bar = 24.0;
            }
            if self.timer % 3.0 == 0.0 && truthy(self.attack_speed) {
                scr_afterimage_grow(me, g);
                let ox = g.irandom_range(-30.0, 30.0);
                let oy = g.irandom_range(-30.0, 30.0);
                let id = g.instance_create(px + ox, py + oy, Box::new(AfterimageScreen::default()));
                if let Some((_, a)) = g.get::<AfterimageScreen>(id) {
                    a.faderate = 0.05;
                    a.draw_end = true;
                }
                if g.exists("obj_growtangle") && g.irandom(4.0) == 0.0 {
                    let bx = battle::scr_get_box(g, 2);
                    let by = battle::scr_get_box(g, 1) + {
                        let b5 = battle::scr_get_box(g, 5);
                        g.irandom(b5)
                    };
                    let id = g.instance_create_depth(bx, by, 0.0, Box::new(ParticleGeneric::default()));
                    let spd = 8.0 + g.random(4.0);
                    let shrink = 0.01 + g.random(0.02);
                    let rand = 1.0 + g.random(1.0);
                    if let Some((i, p)) = g.get::<ParticleGeneric>(id) {
                        i.set_direction(180.0);
                        i.set_speed(spd);
                        i.alarm[0] = 32;
                        p.shrink_rate = shrink;
                        i.image_blend = C_GREEN;
                        i.image_xscale = rand;
                        i.image_yscale = rand;
                    }
                }
            }
            let density = self.density;
            if self.timer == 24.0 - self.attack_speed {
                if self.attack_speed == 0.0 {
                    me.sprite_index = spr("spr_roaringknight_pose_ol");
                    me.image_index = 0.0;
                    me.image_speed = 0.5;
                    g.snd_play("snd_knight_roar");
                    self.whiteout = false;
                    g.destroy_all("obj_particle_generic");
                    for gt in g.ids_of("obj_growtangle") {
                        let Some(x) = g.inst(gt).map(|b| b.x) else { continue };
                        g.lerpvar_instance(gt, "x", x, x - 60.0, 32.0, 1, "out");
                        g.script_delayed(gt, 32, move |g| {
                            g.lerpvar_instance(gt, "x", x - 60.0, x - 100.0, 240.0, 0, "out");
                        });
                    }
                    script_repeat_afterimage_screen(g, me.id, px, py);
                    let c = g.instance_create_depth(px, py, me.depth, Box::new(KnightCircle::default()));
                    if let Some((_, k)) = g.get::<KnightCircle>(c) {
                        k.r = 255.0;
                        k.g = 255.0;
                        k.b = 255.0;
                        k.draw_in_box = false;
                    }
                } else {
                    let raar = g.snd_play("snd_knight_puff");
                    g.audio.pitch(raar, 0.15);
                }
                if self.createbullets {
                    self.random_aim = g.irandom(360.0);
                    let mut a = 0.0;
                    while a < density {
                        let dir = (360.0 / density) * a + self.random_aim * 7.0 + 180.0 / density - 5.0 * self.spin;
                        let speedmod = 0.85 + (dir.to_radians().sin()).abs() * 0.15;
                        self.fire(g, px, py, dir, 18.0 * speedmod, 8.0, 20.0, 1.0, 3.0);
                        a += 1.0;
                    }
                }
            }
            if self.timer == 28.0 - self.attack_speed {
                if self.createbullets {
                    let mut a = 0.0;
                    while a < density {
                        let base = (360.0 / density) * a + self.random_aim * 7.0;
                        let speedmod = 0.85 + (base.to_radians().sin()).abs() * 0.15;
                        self.fire(g, px, py, base, 18.0 * speedmod, 8.0, 28.0, 0.5, 2.0);
                        let speedmod = 0.85 + ((base + 10.0).to_radians().sin()).abs() * 0.15;
                        self.fire(g, px, py, base + 10.0, 16.0 * speedmod, 6.0, 28.0, 0.5, 2.0);
                        let speedmod = 0.85 + ((base - 10.0).to_radians().sin()).abs() * 0.15;
                        self.fire(g, px, py, base - 10.0, 16.0 * speedmod, 6.0, 28.0, 0.5, 2.0);
                        a += 1.0;
                    }
                }
                self.spin *= -1.0;
                self.counter += 1.0;
                self.attack_speed = scr_approach(self.attack_speed, 14.0, 1.0);
                if self.counter % 3.0 == 0.0 {
                    self.density = scr_approach(self.density, 6.0, 1.0);
                }
                if self.counter < 30.0 {
                    self.timer = 0.0;
                }
            }
            self.roarendtimer += 1.0;
            if self.roarendtimer >= self.roarendtimermax {
                g.destroy_self(me);
            }
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let (px, py) = Self::pxy(me, g);
        if truthy(self.bar) {
            g.gfx.draw_line_width_color(px, py - self.bar * 40.0, px, py + self.bar * 40.0, self.bar, C_WHITE, C_WHITE);
            self.bar *= 0.65;
            if self.bar < 0.5 {
                self.bar = 0.0;
            }
        }
        if me.sprite_index == spr("spr_roaringknight_shift_ol") {
            let (mut xoff, mut yoff) = (0.0, 0.0);
            if truthy(self.shudder) {
                xoff = g.irandom_range(-1.0, 1.0);
                yoff = g.irandom_range(-1.0, 1.0);
            }
            g.draw_sprite_ext(me.sprite_index, me.image_index, me.x - 20.0 + xoff, me.y + 20.0 + yoff, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, me.image_alpha);
            if self.whiteout {
                g.gfx.gpu_set_fog(true, C_WHITE);
                g.draw_sprite_ext(me.sprite_index, me.image_index, me.x - 20.0 + xoff, me.y + 20.0 + yoff, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, self.whiteout_counter);
                g.gfx.gpu_set_fog(false, C_BLACK);
            }
        } else {
            let t = g.glob.time;
            g.draw_sprite_ext(me.sprite_index, me.image_index, me.x, me.y + (t * 0.2).sin() * 2.0, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, me.image_alpha);
        }
    }

    obj_vars!(timer, whiteout_counter, shudder, bar, density, attack_speed);
}
