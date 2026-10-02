//! The Roaring Knight's "Stars" attack (knight myattackchoice 1, obj_dbulletcontroller type 98).
//!
//! Objects: obj_knight_pointing_cone (the pointing Knight + the light cone), obj_knight_pointing_star,
//! obj_knight_pointing_starchild, obj_heart_follower, obj_fake_gt, obj_afterimage_blend.

use crate::assets::{spr, Spr};
use crate::battle::{self, DBulletController, DbCtrl, Growtangle, KnightEnemy, RegVars};
use crate::gfx::{bm, merge_color, Color, C_BLACK, C_GRAY, C_RED, C_WHITE};
use crate::gm::*;
use crate::obj_vars;
use crate::rt::{Afterimage, Game, Id, Inst, Object, NOONE};

// ============================================================================ helper scripts

/// clamp01(v)
fn clamp01(v: f64) -> f64 { clamp(v, 0.0, 1.0) }

/// scr_loop(v, len)
fn scr_loop(v: f64, len: f64) -> f64 {
    if len == 0.0 {
        return v;
    }
    let mut r = v % len;
    if v < 0.0 {
        r += len;
    }
    r
}

/// scr_loop_ext(v, lo, hi)
fn scr_loop_ext(v: f64, lo: f64, hi: f64) -> f64 {
    let range = hi - lo;
    if range == 0.0 {
        return v;
    }
    let amount = v - lo;
    let mut r = amount % range;
    if amount < 0.0 {
        r += range;
    }
    r + lo
}

/// scr_pingpong(v, len)
fn scr_pingpong(v: f64, len: f64) -> f64 {
    if len == 0.0 {
        return v;
    }
    let mut r = scr_loop(v, len * 2.0);
    if r > len {
        r = len * 2.0 - r;
    }
    r
}

/// scr_inverselerp(a, b, v)
fn scr_inverselerp(a: f64, b: f64, v: f64) -> f64 {
    if b == a {
        return 0.0;
    }
    (v - a) / (b - a)
}

/// remap(a, b, c, d, v)
fn remap(a: f64, b: f64, c: f64, d: f64, v: f64) -> f64 { lerp(c, d, scr_inverselerp(a, b, v)) }

/// scr_rotatetowards(angle, target, amount)
fn scr_rotatetowards(a: f64, b: f64, amt: f64) -> f64 {
    let diff = angle_difference(b, a);
    if diff.abs() > amt { a + sign(diff) * amt } else { b }
}

/// scr_angle_lerp(a, b, t)
fn scr_angle_lerp(a: f64, b: f64, t: f64) -> f64 { a + lerp(0.0, angle_difference(b, a), t) }

/// scr_rotatevector(new Vector2(len), angle) + origin
fn beam_point(x: f64, y: f64, len: f64, ang: f64) -> (f64, f64) {
    if ang == 0.0 {
        return (x + len, y);
    }
    let dir = point_direction(0.0, 0.0, len, 0.0);
    let l = point_distance(0.0, 0.0, len, 0.0);
    (x + lengthdir_x(l, dir + ang), y + lengthdir_y(l, dir + ang))
}

/// scr_draw_beam_color(x, y, length, width, angle, col1, col2, alpha, circle)
#[allow(clippy::too_many_arguments)]
fn scr_draw_beam_color(g: &mut Game, x: f64, y: f64, len: f64, w: f64, ang: f64, c1: Color, c2: Color, alpha: f64, circle: bool) {
    let e0 = beam_point(x, y, len, ang);
    let e1 = beam_point(x, y, len, ang + w / 2.0);
    let e2 = beam_point(x, y, len, ang - w / 2.0);
    let a = g.gfx.draw_get_alpha();
    g.gfx.draw_set_alpha(alpha);
    if circle {
        let c = g.gfx.draw_get_color();
        g.gfx.draw_circle_color(e0.0, e0.1, w / 2.0, c, c, false);
    }
    g.gfx.draw_triangle_color(x, y, e1.0, e1.1, e2.0, e2.1, c1, c2, c2, false);
    g.gfx.draw_set_alpha(a);
}

/// scr_draw_outline(width, colour, alpha) for `me`
fn scr_draw_outline(me: &Inst, g: &mut Game, w: f64, col: Color, alpha: f64) {
    g.gfx.gpu_set_fog(true, col);
    let (mut xa, mut xb, mut ya, mut yb) = (w, 0.0, 0.0, w);
    if me.image_angle % 90.0 != 0.0 {
        xa = lengthdir_x(w, me.image_angle);
        xb = lengthdir_x(w, me.image_angle + 90.0);
        ya = lengthdir_y(w, me.image_angle + 90.0);
        yb = lengthdir_y(w, me.image_angle);
    }
    let a = me.image_alpha * alpha;
    for (dx, dy) in [(xa, ya), (-xa, -ya), (xb, yb), (-xb, -yb)] {
        g.draw_sprite_ext(me.sprite_index, me.image_index, me.x + dx, me.y + dy, me.image_xscale, me.image_yscale, me.image_angle, C_WHITE, a);
    }
    g.gfx.gpu_set_fog(false, C_WHITE);
}

/// scr_onscreen_tolerance(self, spacer)
fn scr_onscreen_tolerance(me: &Inst, g: &Game, spacer: f64) -> bool {
    let (cx, cy) = (g.camerax(), g.cameray());
    let (sw, sh) = (g.sprite_width(me), g.sprite_height(me));
    !(me.x + sw + spacer < cx || me.x - spacer > cx + 640.0 || me.y + sh + spacer < cy || me.y - spacer > cy + 480.0)
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

/// gt_maxx()
fn gt_maxx(g: &mut Game) -> f64 { battle::scr_get_box(g, 0) }

/// obj_heart.x / obj_heart.y
fn heart_xy(g: &mut Game) -> (f64, f64) {
    // NOTE: if the battle core's collision pass runs a bullet's user(5) while the heart's own event is
    // executing, the heart is checked out and this falls back to the cached global heart position.
    match g.first_inst("obj_heart") {
        Some(h) => (h.x, h.y),
        None => (g.glob.heartx, g.glob.hearty),
    }
}

/// Does `me`'s collision mask cover world point (px, py)? (mirror of the runtime's private test)
fn covers(me: &Inst, g: &Game, px: f64, py: f64, bb: &[f64; 4]) -> bool {
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

/// scr_precise_hit(hitbox): precise collision of `me` against a small box around the SOUL's centre.
fn scr_precise_hit(me: &Inst, g: &mut Game, hitbox: f64) -> bool {
    let r = hitbox / 2.0;
    let (hx, hy) = heart_xy(g);
    let (hx, hy) = (hx + 10.0, hy + 10.0);
    let Some(bb) = g.bbox(me) else { return false };
    if r <= 0.0 {
        // collision_point(_hx, _hy, id, true, false)
        return hx >= bb[0] && hx <= bb[2] + 1.0 && hy >= bb[1] && hy <= bb[3] + 1.0 && covers(me, g, hx, hy, &bb);
    }
    // collision_rectangle(_hx - r, _hy - r, _hx + r, _hy + r, id, true, false)
    let (l, rr, t, b) = (hx - r, hx + r, hy - r, hy + r);
    let il = l.max(bb[0]);
    let ir = rr.min(bb[2]);
    let it = t.max(bb[1]);
    let ib = b.min(bb[3]);
    if il > ir || it > ib {
        return false;
    }
    let mut yy = it.floor();
    while yy <= ib {
        let mut xx = il.floor();
        while xx <= ir {
            if covers(me, g, xx + 0.5, yy + 0.5, &bb) {
                return true;
            }
            xx += 1.0;
        }
        yy += 1.0;
    }
    false
}

/// `with (obj_knight_enemy) aoedamage = v;`
fn set_aoedamage(g: &mut Game, v: bool) {
    for id in g.ids_of("obj_knight_enemy") {
        if let Some((_, k)) = g.get::<KnightEnemy>(id) {
            k.aoedamage = v;
        }
    }
}

// ============================================================================ controller (type 98)

/// obj_dbulletcontroller Step_0, `if (type == 98)`.
pub fn ctrl_step(c: &mut DbCtrl, me: &mut Inst, g: &mut Game) {
    if c.init == 1.0 {
        c.init = 2.0;
        let (hx, hy) = heart_xy(g);
        g.instance_create(hx, hy, Box::new(HeartFollower::default()));
        c.bulletmaker = g.instance_create(me.x, me.y, Box::new(KnightPointingCone::default()));
        c.btimer = 0.0;
        c.endtimer = 120.0;
        if c.difficulty >= 2.0 {
            c.endtimer += 30.0;
            g.glob.turntimer += 60.0;
            c.endtimer += 60.0;
        }
        if let Some((_, cone)) = g.get::<KnightPointingCone>(c.bulletmaker) {
            cone.difficulty = c.difficulty;
            cone.endtimer = c.endtimer;
        }
        c.delay = 0.0;
        c.subdelay = 0.0;
        g.glob.turntimer += 30.0;
        if c.difficulty == 0.0 {
            c.side = g.choose(&[-1.0, 1.0]);
        }
    } else if c.init >= 3.0 {
        return;
    }
    if g.glob.turntimer <= c.endtimer + 1.0 {
        c.init = 3.0;
    } else if (c.made != 0.0 && c.btimer >= 4.0) || c.btimer >= 45.0 {
        if c.difficulty == 2.0 {
            c.side = g.choose(&[0.0, 66.0, -66.0]);
        }
        let (bx, by, bangle) = match g.get::<KnightPointingCone>(c.bulletmaker) {
            Some((i, o)) => (i.x, i.y, o.angle),
            None => (me.x, me.y, 0.0),
        };
        let d = battle::scr_childbullet(g, me, bx + 22.0, by + 56.0, Box::new(KnightPointingStar::default()));
        if let Some((_, s)) = g.get::<KnightPointingStar>(d) {
            s.difficulty = c.difficulty;
            s.side = c.side;
        }
        let starsound = g.snd_play_pitch("snd_stardrop", 0.5);
        g.audio.volume(starsound, 0.5, 0.0);
        if c.made == 0.0 {
            c.size = g.random_range(0.5, 1.0);
            c.special = g.random_range(-0.5, 0.5);
        } else {
            c.special += 0.5;
            c.special += 0.5 + g.random(1.0).sin() * 0.3;
            c.special %= 1.0;
            c.special -= 0.5;
            c.size += 0.5 + c.made.sin() * 0.5;
            c.size %= 1.0;
        }
        let mut dir = 180.0 + c.special * bangle;
        if c.size <= 0.1 {
            let (hx, hy) = heart_xy(g);
            let heartdir = point_direction(bx + 22.0, by + 56.0, hx + 10.0, hy + 10.0);
            if angle_difference(heartdir, dir).abs() < 20.0 {
                if dir < heartdir {
                    dir = (heartdir - 20.0) + angle_difference(heartdir, dir);
                } else {
                    dir = heartdir + 20.0 + angle_difference(heartdir, dir);
                }
                dir = scr_loop_ext(dir, 180.0 - bangle, 180.0 + bangle);
            }
        }
        let size = c.size;
        if let Some((i, s)) = g.get::<KnightPointingStar>(d) {
            i.set_direction(dir);
            i.set_speed(lerp(10.0, 5.0, size));
            // GML writes `d.grow_Speed` (capital S) while the star reads `growspeed`: the value is never used.
            s.grow_speed_unused = lerp(0.1, 0.25, size);
        }
        c.made += 1.0;
        c.btimer = 0.0;
    }
}

// ============================================================================ obj_heart_follower

/// obj_heart_follower (parent obj_bulletparent): a smoothed copy of the SOUL's position that the
/// star children aim at.
pub struct HeartFollower {
    pub smoothing: f64,
    pub max_speed: f64,
    pub xdiff: f64,
    pub ydiff: f64,
}
impl Default for HeartFollower {
    fn default() -> Self { HeartFollower { smoothing: 0.125, max_speed: 4.0, xdiff: 0.0, ydiff: 0.0 } }
}
impl Object for HeartFollower {
    fn name(&self) -> &'static str { "obj_heart_follower" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        // target = obj_heart
        let hd = g.first_inst("obj_heart").map(|h| h.depth).unwrap_or(0.0);
        me.depth = hd - 5.0;
        self.smoothing = 0.125;
        self.max_speed = 4.0;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        let (tx, ty) = heart_xy(g);
        self.xdiff = tx - me.x;
        self.ydiff = ty - me.y;
        me.x = scr_movetowards(me.x, tx, clamp(self.xdiff.abs() * self.smoothing, 1.0, self.max_speed));
        me.y = scr_movetowards(me.y, ty, clamp(self.ydiff.abs() * self.smoothing, 1.0, self.max_speed));
    }
    obj_vars!(smoothing, max_speed);
}

// ============================================================================ obj_fake_gt

/// obj_fake_gt (parent obj_bulletparent): draws the bullet board shaken by an offset while the real one
/// is hidden.
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
        if let Some(gt) = g.first_inst("obj_growtangle") {
            gt.visible = false;
            me.depth = gt.depth;
        }
    }
    fn end_step(&mut self, me: &mut Inst, g: &mut Game) {
        if let Some(gt) = g.first_inst("obj_growtangle") {
            me.x = gt.x;
            me.y = gt.y;
            me.image_xscale = gt.image_xscale;
            me.image_yscale = gt.image_yscale;
        }
    }
    fn draw(&mut self, _me: &mut Inst, g: &mut Game) {
        let (xoff, yoff) = (self.xoffset, self.yoffset);
        for id in g.ids_of("obj_growtangle") {
            let Some(i) = g.inst(id) else { continue };
            let (s, sub, x, y, xs, ys, ang, col, a) =
                (i.sprite_index, i.image_index, i.x, i.y, i.image_xscale, i.image_yscale, i.image_angle, i.image_blend, i.image_alpha);
            let custom = g
                .get::<Growtangle>(id)
                .map(|(_, gt)| (gt.custom_box && gt.growth != 0.0 && gt.growcon != 2.0, gt.spr_custom_box, gt.maxxscale, gt.maxyscale));
            g.draw_sprite_ext(s, 1.0, x + xoff, y + yoff, xs, ys, ang, col, a);
            match custom {
                Some((true, cs, mx, my)) => g.draw_sprite_ext(cs, 0.0, x + xoff, y + yoff, xs / (mx / 2.0), ys / (my / 2.0), ang, col, a),
                _ => g.draw_sprite_ext(s, sub, x + xoff, y + yoff, xs, ys, ang, col, a),
            }
        }
    }
    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if let Some(gt) = g.first_inst("obj_growtangle") {
            gt.visible = true;
        }
    }
    obj_vars!(xoffset, yoffset);
}

// ============================================================================ obj_afterimage_blend

/// obj_afterimage_blend (parent obj_afterimage): an afterimage drawn with a blend mode.
pub struct AfterimageBlend {
    pub blendmode: i32,
    pub dest_blend: i32,
    pub fade_speed: f64,
}
impl Default for AfterimageBlend {
    fn default() -> Self { AfterimageBlend { blendmode: 1, dest_blend: -4, fade_speed: 0.04 } }
}
impl Object for AfterimageBlend {
    fn name(&self) -> &'static str { "obj_afterimage_blend" }
    fn create(&mut self, _me: &mut Inst, _g: &mut Game) {
        self.blendmode = 1;
        self.dest_blend = -4;
        // event_inherited(): obj_afterimage Create
        self.fade_speed = 0.04;
    }
    // obj_afterimage Step (inherited)
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        me.image_alpha -= self.fade_speed;
        if me.image_alpha < 0.0 {
            g.destroy_self(me);
        }
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        // `var _bm = gpu_get_blendmode()`: the runtime has no getter; the mode is normal during object draws.
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

// ============================================================================ obj_knight_pointing_cone

/// obj_knight_pointing_cone (parent obj_bulletparent): the Knight pointing left, the light cone and the
/// star-drawing surfaces.
pub struct KnightPointingCone {
    pub surf: i32,
    pub starsurf: i32,
    pub angle: f64,
    pub target_angle: f64,
    pub bg_x: f64,
    pub lines_x: f64,
    pub tween: f64,
    pub ease: f64,
    pub angle_lerp: f64,
    pub aetimer: f64,
    pub con: f64,
    pub difficulty: f64,
    pub yoff: f64,
    pub timer: f64,
    pub fake_gt: Id,
    pub gt_x: f64,
    pub knockback: f64,
    pub star_flicker: f64,
    pub draw_angle: f64,
    pub starlist: Vec<Id>,
    pub childlist: Vec<Id>,
    pub point_frame: f64,
    pub afterimage_spread: f64,
    pub image_timer: f64,
    pub timerb: f64,
    pub endtimer: f64,
    pub child_attack: bool,
    pub count: f64,
    pub i: f64,
}
impl Default for KnightPointingCone {
    fn default() -> Self {
        KnightPointingCone {
            surf: -4,
            starsurf: -4,
            angle: 0.0,
            target_angle: 60.0,
            bg_x: 0.0,
            lines_x: 0.0,
            tween: 0.0,
            ease: 0.0,
            angle_lerp: 0.0,
            aetimer: 0.0,
            con: 0.0,
            difficulty: 0.0,
            yoff: 0.0,
            timer: 0.0,
            fake_gt: NOONE,
            gt_x: 0.0,
            knockback: 0.0,
            star_flicker: 0.0,
            draw_angle: 0.0,
            starlist: vec![NOONE],
            childlist: vec![NOONE],
            point_frame: 0.0,
            afterimage_spread: 0.0,
            image_timer: 0.0,
            timerb: 0.0,
            endtimer: 120.0,
            child_attack: false,
            count: 0.0,
            i: 0.0,
        }
    }
}

impl KnightPointingCone {
    fn knight(g: &mut Game) -> Option<Id> { g.first("obj_knight_enemy") }
}

impl Object for KnightPointingCone {
    fn name(&self) -> &'static str { "obj_knight_pointing_cone" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        *self = KnightPointingCone::default();
        me.image_xscale = 2.0;
        me.image_yscale = 2.0;
        me.image_speed = 0.0;
        match Self::knight(g) {
            Some(k) => {
                if let Some(ki) = g.inst_mut(k) {
                    ki.visible = false;
                }
                self.aetimer = g.get::<KnightEnemy>(k).map(|(_, ke)| ke.aetimer).unwrap_or(0.0);
            }
            None => self.aetimer = 0.0,
        }
        self.con = 0.0;
        self.difficulty = 0.0;
        self.yoff = g.irandom(60.0) + 2.0;
        self.timer = 0.0;
        self.fake_gt = g.instance_create(me.x, me.y, Box::new(FakeGt::default()));
        self.gt_x = g.first_inst("obj_growtangle").map(|b| b.x).unwrap_or(0.0);
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.timerb += 1.0;
        if self.timerb == 3.0 {
            g.snd_play_x("snd_knight_drawpower", 1.0, 1.3);
            g.snd_play_x("snd_knight_drawpower", 1.0, 1.3);
            g.snd_play_x("snd_knight_drawpower", 1.0, 1.3);
        }
        if self.timerb == 120.0 {
            let (a, b) = (2.0, 0.7);
            g.snd_play_x("snd_knight_star_explosion_close", a, b);
            g.snd_play_x("snd_knight_star_explosion_close", a, b);
            g.snd_play_x("snd_knight_star_explosion_close", a, b);
        }
        if self.con == 4.0 {
            let (kx, ky) = g.first_inst("obj_knight_enemy").map(|k| (k.x, k.y)).unwrap_or((me.x, me.y));
            me.x = lerp(me.x, kx, 0.15);
            me.y = lerp(me.y, ky, 0.15);
            // (tween is never reset to 0 in the GML, so this never fires: the cone stays until the turn ends.)
            if self.tween == 0.0 {
                self.con = 5.0;
                if let Some(k) = Self::knight(g) {
                    if let Some(ki) = g.inst_mut(k) {
                        ki.visible = true;
                    }
                    let ae = self.aetimer;
                    if let Some((_, ke)) = g.get::<KnightEnemy>(k) {
                        ke.aetimer = ae;
                    }
                }
            }
        } else if self.tween < 1.0 {
            self.tween = scr_movetowards(self.tween, 1.0, 0.05);
            let e = scr_ease_out(self.tween, 4);
            let (gx, gy) = g.first_inst("obj_growtangle").map(|b| (b.x, b.y)).unwrap_or((0.0, 0.0));
            me.x = lerp(me.xstart, gx + 115.0, e);
            me.y = lerp(me.ystart, gy - 56.0, e);
        }
        if self.con < 2.0 {
            return;
        } else if g.glob.turntimer <= self.endtimer {
            if self.angle_lerp == 1.0 {
                let mut count = 0usize;
                for id in g.ids_of("obj_knight_pointing_star") {
                    if count < self.starlist.len() {
                        self.starlist[count] = id;
                    } else {
                        self.starlist.push(id);
                    }
                    if let Some((_, s)) = g.get::<KnightPointingStar>(id) {
                        s.con = 1.0;
                    }
                    count += 1;
                }
                self.i = 0.0;
                while (self.i as usize) < count {
                    let i = self.i;
                    if let Some((_, s)) = g.get::<KnightPointingStar>(self.starlist[i as usize]) {
                        s.timer = -i;
                        if i % 2.0 == 1.0 {
                            s.play_sound = false;
                        }
                    }
                    self.i += 1.0;
                }
                self.knockback = 10.0;
            }
            if self.angle_lerp == 0.0 && self.con < 3.0 {
                self.timer = 10.0;
                self.con = 3.0;
                self.yoff = 120.0 + g.irandom_range(-60.0, 60.0);
            }
            self.angle_lerp = scr_movetowards(self.angle_lerp, 0.0, 0.1);
            self.angle = lerp(0.0, self.target_angle, scr_ease_in(self.angle_lerp, 6));
        } else if self.angle < self.target_angle {
            self.angle_lerp = scr_movetowards(self.angle_lerp, 1.0, 0.025);
            self.angle = lerp(0.0, self.target_angle, scr_ease_out(self.angle_lerp, 6));
        } else {
            me.x += 0.25;
        }
        let (xo, yo);
        if self.knockback != 0.0 {
            let kb = scr_ease_in(self.knockback / 10.0, 5) * 10.0;
            self.gt_x -= kb;
            self.knockback = scr_movetowards(self.knockback, 0.0, 0.5);
            xo = g.random_range(-1.0, 1.0) * (kb / 10.0);
            yo = g.random_range(-1.0, 1.0) * (kb / 10.0);
        } else {
            self.gt_x -= self.angle / self.target_angle / 2.0;
            xo = g.random_range(-1.0, 1.0) * (self.angle / self.target_angle);
            yo = g.random_range(-1.0, 1.0) * (self.angle / self.target_angle);
        }
        if let Some((_, f)) = g.get::<FakeGt>(self.fake_gt) {
            f.xoffset = xo;
            f.yoffset = yo;
        }
        let gx = round(self.gt_x);
        if let Some(gt) = g.first_inst("obj_growtangle") {
            gt.x = gx;
        }
        let maxx = gt_maxx(g);
        if let Some(h) = g.first_inst("obj_heart") {
            h.x = h.x.min(maxx - 22.0);
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let image_number = g.sprite_get_number(me.sprite_index);
        if self.con >= 3.0 {
            if me.image_index == image_number - 1.0 {
                me.image_index -= 1.0;
            } else if me.image_index > 0.0 {
                me.image_index -= 0.25;
            }
        } else if me.image_index < image_number - 1.0 {
            me.image_index += 0.5;
        }
        if self.con < 5.0 {
            g.draw_self(me);
        }
        self.aetimer += 1.0;
        if self.con <= 4.0 && self.aetimer % 4.0 == 0.0 {
            let ai = g.afterimage(me);
            let kd = g.first_inst("obj_knight_enemy").map(|k| k.depth).unwrap_or(0.0);
            let (spd, dir) = (2.0 + self.afterimage_spread / 30.0, (self.aetimer.sin() * self.angle) / 2.0);
            if let Some((ai_i, ai_o)) = g.get::<Afterimage>(ai) {
                ai_i.image_alpha = 0.6;
                ai_o.fade_speed = 0.02;
                ai_i.set_speed(spd);
                ai_i.set_direction(dir);
                ai_i.depth = kd + 1.0;
            }
            if self.con == 4.0 {
                self.afterimage_spread = scr_movetowards(self.afterimage_spread, 0.0, 20.0);
            }
            if self.con >= 1.0 {
                self.afterimage_spread += 1.0;
            }
        }
        if self.con >= 4.0 {
            return;
        }
        if !g.gfx.surface_exists(self.surf) {
            self.surf = g.gfx.surface_create(640, 480);
        }
        if !g.gfx.surface_exists(self.starsurf) {
            self.starsurf = g.gfx.surface_create(640, 480);
        }
        let (camx, camy) = (g.camerax(), g.cameray());
        let flow = spr("spr_knight_bullet_flow");
        let sx = me.x + 22.0 - camx; // screenx(x + 22)
        if self.con <= 1.0 {
            if self.con == 0.0 {
                self.con = 1.0;
            }
            if self.timer < 28.0 {
                g.gfx.draw_set_blend_mode(bm::ADD);
                g.draw_sprite_part_ext(flow, 2.0, self.timer, self.timer * 4.0 + self.yoff - 2.0, sx / 2.0, 1.0, camx, me.y + 54.0, 2.0, 2.0, C_GRAY, 1.0);
                if self.timer % 2.0 == 0.0 {
                    g.draw_sprite_part_ext(flow, 2.0, self.timer * 2.0, self.timer * 4.0 + self.yoff, sx / 2.0, 1.0, camx, me.y + 54.0, 2.0, 2.0, C_GRAY, 1.0);
                }
                g.gfx.draw_set_blend_mode(bm::NORMAL);
            } else {
                g.gfx.draw_set_color(C_WHITE);
                // ossafe_fill_rectangle
                g.gfx.draw_rectangle(camx, me.y + 54.0, me.x + 22.0, me.y + 56.0, false);
            }
            self.timer += 1.0;
            if self.timer >= 30.0 {
                g.snd_play_pitch("snd_rocket_long", 0.6);
                self.con = 2.0;
            }
            return;
        }
        if self.con == 3.0 && self.timer > 0.0 {
            g.gfx.draw_set_blend_mode(bm::ADD);
            let t = 10.0 - self.timer;
            g.draw_sprite_part_ext(flow, 2.0, t * 2.0, self.yoff - t * 4.0, sx / 2.0, 1.0, camx, me.y + 54.0, 2.0, 2.0, C_GRAY, self.timer / 10.0);
            g.draw_sprite_part_ext(flow, 2.0, t * 2.0, self.yoff + t * 4.0, sx / 2.0, 1.0, camx, me.y + 54.0, 2.0, 2.0, C_GRAY, self.timer / 10.0);
            self.timer -= 1.0;
            g.gfx.draw_set_blend_mode(bm::NORMAL);
            if self.timer == 0.0 {
                self.con = 4.0;
            }
            return;
        }
        g.gfx.surface_set_target(self.surf);
        g.gfx.draw_clear_alpha(C_BLACK, 0.0);
        g.gfx.draw_set_blend_mode(bm::NORMAL);
        g.gfx.draw_set_alpha(1.0);
        g.gfx.draw_set_color(merge_color(C_WHITE, C_BLACK, self.angle / self.target_angle));
        // draw_primitive_begin(pr_trianglelist) with 4 vertices: only the first 3 form a triangle.
        self.draw_angle = 1.0 - self.draw_angle;
        let a = if self.angle > 0.0 { self.angle + self.draw_angle } else { 0.0 };
        let xleft = lengthdir_x(600.0, 180.0 + a / 2.0);
        let ytop = lengthdir_y(600.0, 180.0 - a / 2.0);
        let _ybottom = lengthdir_y(600.0, 180.0 + a / 2.0);
        let sy56 = me.y + 56.0 - camy;
        g.gfx.draw_triangle(sx + xleft, sy56 + ytop, sx, sy56, sx + xleft, me.y + 58.0 - camy + _ybottom, false);
        g.gfx.gpu_set_alphatestenable(true);
        g.gfx.gpu_set_blendmode_ext_sepalpha(bm::SRC_ALPHA, bm::ONE, bm::DEST_ALPHA, bm::ZERO);
        let blend = 1.0;
        g.draw_sprite_ext(flow, 0.0, self.bg_x, 0.0, 2.0, 2.0, 0.0, C_WHITE, blend);
        g.draw_sprite_ext(flow, 0.0, self.bg_x + 640.0, 0.0, 2.0, 2.0, 0.0, C_WHITE, blend);
        g.draw_sprite_ext(flow, 1.0, self.lines_x, 0.0, 2.0, 2.0, 0.0, C_WHITE, blend);
        g.draw_sprite_ext(flow, 1.0, self.lines_x + 640.0, 0.0, 2.0, 2.0, 0.0, C_WHITE, blend);
        self.lines_x -= 80.0;
        self.bg_x -= 20.0;
        if self.lines_x < -640.0 {
            self.lines_x += 640.0;
        }
        if self.bg_x < -640.0 {
            self.bg_x += 640.0;
        }
        g.gfx.draw_set_blend_mode(bm::SUBTRACT);
        for id in g.ids_of("obj_heart") {
            if let Some(h) = g.inst(id) {
                let (s, sub, hx, hy) = (h.sprite_index, h.image_index, h.x - camx, h.y - camy);
                g.draw_sprite(s, sub, hx, hy);
            }
        }
        if g.exists("obj_knight_pointing_star") {
            g.gfx.surface_set_target(self.starsurf);
            g.gfx.draw_set_blend_mode(bm::NORMAL);
            g.gfx.draw_clear_alpha(C_BLACK, 0.0);
            for id in g.ids_of("obj_knight_pointing_star") {
                if g.inst(id).map(|s| s.image_xscale > 0.5).unwrap_or(false) {
                    g.user_event(id, 0);
                }
            }
            g.gfx.draw_set_blend_mode(bm::SUBTRACT);
            g.draw_sprite_ext(spr("spr_knight_line_grate"), 0.0, 0.0, self.star_flicker, 2.0, 2.0, 0.0, C_BLACK, 1.0);
            self.star_flicker = 2.0 - self.star_flicker;
            g.gfx.draw_set_blend_mode(bm::NORMAL);
            for id in g.ids_of("obj_knight_pointing_star") {
                if g.inst(id).map(|s| s.image_xscale <= 0.5).unwrap_or(false) {
                    g.user_event(id, 0);
                }
            }
            g.gfx.surface_reset_target();
            g.gfx.gpu_set_alphatestenable(true);
            g.gfx.gpu_set_blendmode_ext_sepalpha(bm::SRC_ALPHA, bm::INV_SRC_ALPHA, bm::DEST_ALPHA, bm::ZERO);
            g.gfx.draw_surface(self.starsurf, 0.0, 0.0);
        }
        g.gfx.gpu_set_alphatestenable(false);
        g.gfx.draw_set_blend_mode(bm::ADD);
        g.gfx.surface_reset_target();
        g.gfx.draw_surface(self.surf, camx, camy);
        g.gfx.draw_set_blend_mode(bm::NORMAL);
    }

    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.gfx.surface_exists(self.surf) {
            g.gfx.surface_free(self.surf);
        }
        if g.gfx.surface_exists(self.starsurf) {
            g.gfx.surface_free(self.starsurf);
        }
        if let Some(k) = Self::knight(g) {
            let hidden = g.inst(k).map(|ki| !ki.visible).unwrap_or(false);
            if hidden {
                if let Some(ki) = g.inst_mut(k) {
                    ki.visible = true;
                }
                let ae = self.aetimer;
                if let Some((_, ke)) = g.get::<KnightEnemy>(k) {
                    ke.aetimer = ae;
                }
            }
        }
    }

    obj_vars!(angle, target_angle, tween, angle_lerp, aetimer, con, timer, knockback, gt_x, endtimer);
}

// ============================================================================ obj_knight_pointing_star

/// obj_knight_pointing_star (parent obj_collidebullet): a star thrown along the cone; it grows, stops,
/// flashes beams and bursts into obj_knight_pointing_starchild.
pub struct KnightPointingStar {
    pub growspeed: f64,
    /// GML `grow_Speed`, written by the controller but never read.
    pub grow_speed_unused: f64,
    pub even: bool,
    pub timer: f64,
    pub con: f64,
    pub growstart: f64,
    pub play_sound: bool,
    pub beamflicker: f64,
    pub difficulty: f64,
    pub side: f64,
    pub init: bool,
    pub rotation: f64,
    pub dir: f64,
    pub i: f64,
}
impl Default for KnightPointingStar {
    fn default() -> Self {
        KnightPointingStar {
            growspeed: 0.02,
            grow_speed_unused: 0.0,
            even: false,
            timer: 0.0,
            con: 0.0,
            growstart: 0.0,
            play_sound: true,
            beamflicker: 0.0,
            difficulty: 0.0,
            side: 0.0,
            init: false,
            rotation: 0.0,
            dir: 0.0,
            i: 0.0,
        }
    }
}

impl KnightPointingStar {
    fn scales(me: &Inst, g: &Game) -> (f64, f64) {
        let sw = g.sprite_get_width(me.sprite_index);
        let sh = g.sprite_get_height(me.sprite_index);
        ((g.sprite_width(me) + 16.0) / sw, (g.sprite_height(me) + 16.0) / sh)
    }
}

impl Object for KnightPointingStar {
    fn name(&self) -> &'static str { "obj_knight_pointing_star" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::bullet_init(me);
        self.growspeed = 0.02;
        me.image_xscale = 0.0;
        me.image_yscale = 0.0;
        self.even = false;
        me.destroyonhit = 0.0;
        self.timer = 0.0;
        self.con = 0.0;
        self.growstart = 0.0;
        self.play_sound = true;
        self.beamflicker = 0.0;
        me.damage = 1.0;
        me.grazepoints = 2.0;
        me.element = 5.0;
        self.difficulty = 0.0;
        me.grazetimer = 0.0;
        self.side = 0.0;
        self.init = false;
        me.mask_index = spr("spr_knight_bullet_star_mask");
        self.rotation = 0.0;
        self.dir = g.choose(&[-1.0, 1.0]);
    }

    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        let (sw, sh) = (g.sprite_width(me), g.sprite_height(me));
        let (cx, cy) = (g.camerax(), g.cameray());
        if me.x < cx - sw / 2.0 || me.y < cy - sh / 2.0 || me.y > cy + 480.0 + sh / 2.0 {
            g.destroy_self(me);
            return;
        }
        if !self.init {
            if self.difficulty == 0.0 {
                me.sprite_index = spr("spr_knight_bullet_star_easy");
            }
            self.init = true;
        }
        me.grazetimer += 1.0;
        if me.grazetimer % 4.0 == 0.0 {
            me.grazed = 0.0;
        }
        if self.con == 0.0 {
            me.image_xscale += self.growspeed;
            me.image_yscale += self.growspeed;
        } else if self.con == 1.0 {
            me.friction = 0.5;
            self.con += 1.0;
        } else if self.con == 2.0 {
            me.mask_index = spr("spr_knight_bullet_star_mask");
            if me.speed() == 0.0 {
                me.gravity = 0.1;
                me.gravity_direction = me.direction() - 180.0;
                me.friction = 0.0;
            }
            self.timer += 1.0;
            if self.timer >= 40.0 {
                self.timer = 0.0;
                self.con += 1.0;
                if self.play_sound {
                    g.snd_play("snd_explosion_firework");
                }
            }
            self.growstart = me.image_xscale;
        } else if self.con == 3.0 {
            self.timer += 1.0;
            me.image_xscale = self.growstart + clamp01(self.timer / 2.0);
            me.image_yscale = self.growstart + clamp01(self.timer / 2.0);
            if self.timer == 3.0 {
                let mut angle = 90.0;
                if self.difficulty == 2.0 {
                    angle += self.side;
                }
                self.i = 0.0;
                while self.i < 6.0 {
                    let i = self.i;
                    let d = battle::scr_childbullet(g, me, me.x, me.y, Box::new(KnightPointingStarchild::default()));
                    let (xs, ys, diff) = (me.image_xscale, me.image_yscale, self.difficulty);
                    if let Some((di, dc)) = g.get::<KnightPointingStarchild>(d) {
                        di.image_angle = angle;
                        di.set_direction(angle);
                        if diff == 0.0 && i % 2.0 == 1.0 {
                            di.set_speed(1.0);
                            dc.lifetime = 30.0;
                        } else {
                            di.set_speed(4.0);
                        }
                        di.image_xscale = xs * 0.5;
                        di.image_yscale = ys * 0.5;
                        dc.deceleration = 0.15;
                        if diff == 2.0 && i % 3.0 > 0.0 {
                            dc.difficulty = -1.0;
                            dc.lifetime = 30.0;
                            di.set_speed(2.0);
                            if i == 1.0 || i == 4.0 {
                                di.set_speed(di.speed() / 3.0);
                                dc.minspeed /= 3.0;
                                dc.deceleration /= 3.0;
                            } else {
                                di.set_speed(di.speed() * (2.0 / 3.0));
                                dc.minspeed *= 2.0 / 3.0;
                                dc.deceleration *= 2.0 / 3.0;
                            }
                            di.sprite_index = spr("spr_knight_starchild_trail");
                        } else {
                            dc.difficulty = diff;
                        }
                    }
                    if i == 1.0 || i == 4.0 {
                        angle += if self.difficulty == 2.0 { 180.0 } else { 48.0 };
                    } else {
                        angle += if self.difficulty == 2.0 { 0.0 } else { 66.0 };
                    }
                    self.i += 1.0;
                }
                me.active = 0.0;
            }
            if self.timer >= 4.0 {
                let (xs, ys) = Self::scales(me, g);
                me.image_xscale = xs;
                me.image_yscale = ys;
                let ai = g.afterimage(me);
                if let Some((_, a)) = g.get::<Afterimage>(ai) {
                    a.fade_speed *= 3.0;
                }
                g.destroy_self(me);
            }
        }
    }

    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        // GML: `if (instance_exists(548 && con == 0)) exit;` — decompiled as instance_exists(<bool>), i.e.
        // object index 0/1 (never present in battle), so it never exits. At con == 0 nothing below draws anyway.
        let (xscale, yscale) = Self::scales(me, g);
        let color = merge_color(C_GRAY, C_RED, clamp01(self.timer / 30.0));
        let alpha = ((self.timer * 3.0).sin() + 1.0) * 0.25;
        let (x, y) = (me.x, me.y);
        if self.con == 2.0 || self.con == 3.0 {
            let mut a = 1.0;
            let mut length = 120.0;
            let mut prog = clamp01(self.timer / 30.0);
            if self.con == 2.0 {
                a = clamp01(prog - alpha);
                length = 50.0 * clamp01(prog - (self.timer % 2.0) * 0.75) + 50.0;
            }
            let offset: f64; // `_offset = 66` in GML; only read in the difficulty-2 branch, which overwrites it
            let sublength = if self.difficulty >= 1.0 { length } else { length / 2.0 };
            g.gfx.draw_set_blend_mode(bm::ADD);
            let mut beamcolor = if self.difficulty >= 2.0 { color } else { 16777215 };
            if self.difficulty == 2.0 {
                prog = scr_ease_in(clamp01(self.timer / 20.0), 4);
                if self.con == 3.0 {
                    offset = 5.0;
                } else {
                    offset = lerp(66.0, 5.0, prog);
                }
                beamcolor = color;
                if self.timer >= 30.0 {
                    prog = scr_ease_in(clamp01((self.timer - 30.0) / 10.0), 4);
                    length += 50.0 - prog * 50.0;
                }
                let side = self.side;
                scr_draw_beam_color(g, x, y, length, 10.0, 90.0 + side, beamcolor, 0, a, false);
                scr_draw_beam_color(g, x, y, length, 10.0, -90.0 + side, beamcolor, 0, a, false);
                scr_draw_beam_color(g, x, y, sublength, 10.0, 90.0 + side + offset, beamcolor, 0, a, false);
                scr_draw_beam_color(g, x, y, sublength, 10.0, (90.0 + side) - offset, beamcolor, 0, a, false);
                scr_draw_beam_color(g, x, y, sublength, 10.0, -90.0 + side + offset, beamcolor, 0, a, false);
                scr_draw_beam_color(g, x, y, sublength, 10.0, (-90.0 + side) - offset, beamcolor, 0, a, false);
            } else {
                scr_draw_beam_color(g, x, y, length, 10.0, 90.0, beamcolor, 0, a, false);
                if self.difficulty != 2.0 {
                    scr_draw_beam_color(g, x, y, sublength, 10.0, 156.0, C_WHITE, 0, a, false);
                    scr_draw_beam_color(g, x, y, sublength, 10.0, 24.0, C_WHITE, 0, a, false);
                    scr_draw_beam_color(g, x, y, sublength, 10.0, 270.0, C_WHITE, 0, a, false);
                }
                scr_draw_beam_color(g, x, y, length, 10.0, 336.0, beamcolor, 0, a, false);
                scr_draw_beam_color(g, x, y, length, 10.0, 204.0, beamcolor, 0, a, false);
            }
            g.gfx.draw_set_blend_mode(bm::NORMAL);
        }
        let s: Spr = me.sprite_index;
        if self.con == 1.0 || self.con == 2.0 {
            g.draw_sprite_ext(s, 1.0, x, y, xscale + 0.1, yscale + 0.1, me.image_angle, C_WHITE, alpha);
            g.draw_sprite_ext(s, 0.0, x, y, xscale, yscale, me.image_angle, color, 1.0);
        }
        if self.con == 3.0 || self.con == 4.0 {
            g.draw_sprite_ext(s, 2.0, x, y, xscale + 0.1, yscale + 0.1, me.image_angle, C_WHITE, ((self.timer * 6.0).sin() + 1.0) * 0.25);
            g.draw_sprite_ext(s, 2.0, x, y, xscale, yscale, me.image_angle, C_WHITE, 1.0);
        }
    }

    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            // Other_10: drawn into the cone's star surface (screen space)
            0 => {
                let (xscale, yscale) = Self::scales(me, g);
                let (sx, sy) = (me.x - g.camerax(), me.y - g.cameray());
                g.draw_sprite_ext(me.sprite_index, 0.0, sx, sy, xscale, yscale, 0.0, C_WHITE, 1.0);
            }
            // Other_15: hit the SOUL
            5 => {
                me.target = 3.0;
                me.damage = 75.0;
                set_aoedamage(g, true);
                if me.active == 1.0 {
                    let hitbox = 3.0;
                    if !scr_precise_hit(me, g, hitbox) {
                        return; // (GML exits here, leaving aoedamage = true)
                    }
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
                set_aoedamage(g, false);
            }
            _ => {}
        }
    }

    obj_vars!(growspeed, timer, con, growstart, difficulty, side);
}

// ============================================================================ obj_knight_pointing_starchild

/// obj_knight_pointing_starchild (parent obj_regularbullet): the shards a star bursts into.
pub struct KnightPointingStarchild {
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
impl Default for KnightPointingStarchild {
    fn default() -> Self {
        KnightPointingStarchild {
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

impl Object for KnightPointingStarchild {
    fn name(&self) -> &'static str { "obj_knight_pointing_starchild" }

    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        battle::regularbullet_create(me, &mut self.rb, g);
        battle::bullet_init(me);
        *self = KnightPointingStarchild { rb: self.rb, ..Default::default() };
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
                for id in g.ids_of("obj_dbulletcontroller") {
                    if let Some((_, ctrl)) = g.get::<DBulletController>(id) {
                        let c = &mut ctrl.c;
                        self.delay += c.delay;
                        if c.subdelay == 4.0 {
                            c.subdelay = 0.0;
                            c.delay += 5.0;
                        } else {
                            c.subdelay += 1.0;
                            c.delay += 1.0;
                        }
                    }
                }
            }
        }
        battle::regularbullet_step(me, &mut self.rb, g);
        if g.exists("obj_knight_roaring2") {
            return;
        }
        if self.con <= 2.0 {
            if me.speed() > self.minspeed {
                me.set_speed(scr_movetowards(me.speed(), self.minspeed, self.deceleration));
            }
            if self.con == 0.0 && self.delay > 0.0 {
                self.timer += 1.0;
                if self.timer >= self.delay {
                    if !scr_onscreen_tolerance(me, g, 10.0) {
                        g.destroy_self(me);
                    }
                    self.timer = 0.0;
                    self.con = 1.0;
                }
            }
        }
        if self.con >= 1.0 && self.con <= 3.0 {
            let (fx, fy) = g.first_inst("obj_heart_follower").map(|f| (f.x, f.y)).unwrap_or_else(|| (me.x - 10.0, me.y - 10.0));
            self.target_angle = point_direction(me.x, me.y, fx + 10.0, fy + 10.0);
            if self.con >= 2.0 && self.tracking {
                let difference = angle_difference(self.target_angle, me.direction());
                if difference.abs() < 90.0 {
                    if self.con < 3.0 {
                        me.set_direction(scr_rotatetowards(me.direction(), self.target_angle, 2.0));
                        me.image_angle = me.direction();
                    } else if difference.abs() <= 4.0 {
                        self.rotation = 0.0;
                    } else if difference.abs() > 30.0 {
                        self.rotation = sign(difference) * 2.0;
                    } else {
                        self.rotation = sign(difference);
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
            let k = ((self.timer / 5.0) * PI).cos();
            me.image_yscale = self.yscale_start * k;
            me.image_blend = merge_color(C_WHITE, C_BLACK, k);
            self.outline = merge_color(C_BLACK, C_RED, k);
        }
        if self.con == 2.0 {
            self.timer += 1.0;
            if self.timer >= 10.0 {
                self.timer = 0.0;
                self.con = 3.0;
            }
        }
        if self.con >= 1.0 && self.ease < 40.0 {
            let spd = (1.0 - self.ease / 40.0) * 2.0;
            me.x -= lengthdir_x(spd, self.target_angle);
            me.y -= lengthdir_y(spd, self.target_angle);
            self.ease += 1.0;
        }
        if self.con == 3.0 {
            me.set_speed(scr_movetowards(me.speed(), 25.0, 0.5));
            me.image_xscale = self.xscale_start + me.speed() / 60.0;
            me.image_yscale = self.yscale_start - me.speed() / 90.0;
            // scr_custom_afterimage is called twice; only the second one is customised.
            scr_custom_afterimage_blend(me, g);
            let afbm = scr_custom_afterimage_blend(me, g);
            let ang = me.image_angle;
            if let Some((ai, ao)) = g.get::<AfterimageBlend>(afbm) {
                ai.image_blend = C_RED;
                ao.fade_speed = 0.25;
                ai.image_alpha = 0.5;
                ai.set_direction(ang);
                ai.set_speed(1.0);
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
        scr_draw_outline(me, g, me.image_xscale, glowcol, glow * me.image_alpha);
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

    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n != 5 {
            return;
        }
        let small = g.first_inst("obj_heart").map(|h| h.sprite_index) == Some(spr("spr_dodgeheart_smaller_2px"));
        if g.exists("obj_knight_roaring2") {
            if me.active == 1.0 {
                let hitbox = if small { 0.0 } else { 2.0 };
                if !scr_precise_hit(me, g, hitbox) {
                    return;
                }
                if g.exists("obj_knight_roaring2") {
                    for k in g.ids_of("obj_knight_enemy") {
                        g.user_event(k, 2);
                    }
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
        } else {
            me.target = 3.0;
            me.damage = 75.0;
            set_aoedamage(g, true);
            if me.active == 1.0 {
                let hitbox = if small { 1.0 } else { 5.0 };
                if !scr_precise_hit(me, g, hitbox) {
                    return; // (GML exits here, leaving aoedamage = true)
                }
                if g.exists("obj_knight_roaring2") {
                    for k in g.ids_of("obj_knight_enemy") {
                        g.user_event(k, 2);
                    }
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
            set_aoedamage(g, false);
        }
    }

    obj_vars!(deceleration, minspeed, timer, drawtimer, lifetime, difficulty, con, rotation, delay, ease, xscale_start, yscale_start);
}
