//! Battle objects and helper scripts shared with the attack ports:
//! obj_heart, obj_grazebox, obj_growtangle, obj_dbulletcontroller, bullet parents,
//! scr_damage & co. Field names follow the GML variable names.

use crate::assets::{Spr, NO_SPR};
use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object, NOONE};
use std::collections::HashMap;

// ============================================================================ bullets

/// scr_bullet_init()
pub fn bullet_init(me: &mut Inst) {
    me.grazed = 0.0;
    me.grazetimer = 0.0;
    me.destroyonhit = 1.0;
    me.target = 0.0;
    me.inv = 60.0;
    me.damage = 10.0;
    me.element = 0.0;
    me.grazepoints = 1.0;
    me.timepoints = 1.0;
    me.active = 1.0;
    me.updateimageangle = 0.0;
}

/// Variables obj_regularbullet adds on top of scr_bullet_init.
#[derive(Clone, Copy, Debug)]
pub struct RegVars {
    pub spin: f64,
    pub spinspeed: f64,
    pub wall_destroy: f64,
    pub bottomfade: f64,
}
impl Default for RegVars {
    fn default() -> Self { RegVars { spin: 0.0, spinspeed: 0.0, wall_destroy: 1.0, bottomfade: 0.0 } }
}

/// obj_regularbullet Create (call from a child's create when GML does event_inherited() or has no Create).
pub fn regularbullet_create(me: &mut Inst, rb: &mut RegVars, g: &mut Game) {
    bullet_init(me);
    *rb = RegVars::default();
    me.image_alpha = 1.0;
    if !g.exists("obj_heart") {
        g.destroy_self(me);
    }
}

/// obj_regularbullet Step (call from a child's step when GML does event_inherited() or has no Step).
pub fn regularbullet_step(me: &mut Inst, rb: &mut RegVars, g: &mut Game) {
    if rb.wall_destroy == 1.0 {
        let (vx, vy) = (g.camerax(), g.cameray());
        if me.x < vx - 80.0 || me.x > vx + 760.0 || me.y < vy - 80.0 || me.y > vy + 580.0 {
            g.destroy_self(me);
        }
    }
    if me.updateimageangle == 1.0 {
        me.image_angle = me.direction();
    }
    if rb.spin == 1.0 {
        me.image_angle += rb.spinspeed;
    }
    if rb.bottomfade != 0.0 && me.y > g.cameray() + rb.bottomfade {
        me.image_alpha *= 0.8;
    }
}

/// obj_collidebullet Other_15 (event_user(5)): the bullet touched the SOUL.
pub fn collidebullet_hit(me: &mut Inst, g: &mut Game) {
    if me.active == 1.0 {
        if me.target != 3.0 {
            scr_damage(me, g);
        }
        if me.target == 3.0 {
            scr_damage_all(me, g);
        }
        if me.destroyonhit == 1.0 {
            g.destroy_self(me);
        }
    }
}

/// scr_bullet_inherit(dst): copy damage/graze/etc from `src` onto instance `dst`.
pub fn scr_bullet_inherit(src: &Inst, dst: Id, g: &mut Game) {
    let s = (src.damage, src.grazepoints, src.timepoints, src.inv, src.target, src.grazed, src.grazetimer, src.element);
    if let Some(d) = g.inst_mut(dst) {
        if s.0 != -1.0 { d.damage = s.0; }
        if s.1 != -1.0 { d.grazepoints = s.1; }
        if s.2 != -1.0 { d.timepoints = s.2; }
        if s.3 != -1.0 { d.inv = s.3; }
        if s.4 != -1.0 { d.target = s.4; }
        if s.5 != -1.0 { d.grazed = 0.0; }
        if s.6 != -1.0 { d.grazetimer = 0.0; }
        d.element = s.7;
    }
}

/// scr_childbullet(x, y, obj): create a bullet inheriting `parent`'s bullet variables.
pub fn scr_childbullet(g: &mut Game, parent: &Inst, x: f64, y: f64, obj: Box<dyn Object>) -> Id {
    let id = g.instance_create(x, y, obj);
    let p = (parent.damage, parent.grazepoints, parent.timepoints, parent.inv, parent.target, parent.grazed, parent.grazetimer, parent.element);
    if let Some(c) = g.inst_mut(id) {
        if p.0 != -1.0 { c.damage = p.0; }
        if p.1 != -1.0 { c.grazepoints = p.1; }
        if p.2 != -1.0 { c.timepoints = p.2; }
        if p.3 != -1.0 { c.inv = p.3; }
        if p.4 != -1.0 { c.target = p.4; }
        if p.5 != -1.0 { c.grazed = p.5; }
        if p.6 != -1.0 { c.grazetimer = p.6; }
        c.element = p.7;
    }
    id
}

/// scr_fire_bullet(x, y, obj, dir, speed, [sprite], [updateimageangle], [inherit_from], [depth])
pub fn scr_fire_bullet(
    g: &mut Game, x: f64, y: f64, obj: Box<dyn Object>, dir: f64, speed: f64, sprite: Option<Spr>, updateimageangle: bool,
    inherit_from: Option<&Inst>, depth: Option<f64>,
) -> Id {
    let id = match depth {
        Some(d) => g.instance_create_depth(x, y, d, obj),
        None => g.instance_create(x, y, obj),
    };
    if let Some(b) = g.inst_mut(id) {
        b.set_direction(dir);
        b.set_speed(speed);
        if let Some(s) = sprite {
            b.sprite_index = s;
        }
        b.updateimageangle = updateimageangle as i32 as f64;
        if updateimageangle {
            b.image_angle = dir;
        }
    }
    if let Some(src) = inherit_from {
        scr_bullet_inherit(src, id, g);
    }
    id
}

// ============================================================================ damage

/// scr_tensionheal(amount)
pub fn scr_tensionheal(g: &mut Game, amt: f64) {
    g.glob.tension = (g.glob.tension + amt).min(g.glob.maxtension);
}

/// scr_damage_calculation(damage, party_slot)
pub fn scr_damage_calculation(g: &Game, dmg: f64, slot: usize) -> f64 {
    let mut t = dmg;
    let def = g.glob.battledf[slot];
    let maxhp = g.glob.maxhp[g.glob.char[slot]];
    let (a, b) = (maxhp / 5.0, maxhp / 8.0);
    let mut i = 0.0;
    while i < def {
        if t > a {
            t -= 3.0;
        } else if t > b {
            t -= 2.0;
        } else {
            t -= 1.0;
        }
        i += 1.0;
    }
    t.max(1.0)
}

/// scr_damage(): the bullet `me` hits; damage `me.damage` to target `me.target` (0-2 slot, 3 all, 4 random).
pub fn scr_damage(me: &mut Inst, g: &mut Game) { crate::battle::scene::scr_damage_impl(me, g) }

/// scr_damage_all()
pub fn scr_damage_all(me: &mut Inst, g: &mut Game) {
    if g.glob.inv < 0.0 {
        let rem = me.damage;
        let tt = me.target;
        for ti in 0..3 {
            g.glob.inv = -1.0;
            me.damage = rem;
            me.target = ti as f64;
            let c = g.glob.char[ti];
            if c != 0 && g.glob.hp[c] > 0.0 {
                scr_damage(me, g);
            }
        }
        g.glob.inv = g.glob.invc * 30.0;
        me.target = tt;
    }
}

// ============================================================================ box & soul

/// scr_get_box(n): 0 right, 1 top, 2 left, 3 bottom, 4 x, 5 y of obj_growtangle.
pub fn scr_get_box(g: &mut Game, n: i32) -> f64 {
    let Some(id) = g.first("obj_growtangle") else { return 0.0 };
    let Some(b) = g.inst(id) else { return 0.0 };
    let (x, y) = (b.x, b.y);
    let w = g.sprite_get_width(b.sprite_index) * b.image_xscale;
    let h = g.sprite_get_height(b.sprite_index) * b.image_yscale;
    match n {
        0 => x + w * 0.5,
        1 => y - h * 0.5,
        2 => x - w * 0.5,
        3 => y + h * 0.5,
        4 => x,
        5 => y,
        _ => 0.0,
    }
}

/// obj_heart (the SOUL). `x,y` is its top-left (sprite origin 0,0); centre is x+10,y+10.
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
}
impl Default for Heart {
    fn default() -> Self {
        Heart { wspeed: 4.0, fly: 0.0, darken: 1.0, darkamt: 0.0, dmgnoise: 0.0, canmove: 1.0, boundaryup: 0.0, color: 0.0, disableslow: 0.0, siner: 0.0 }
    }
}

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

// ============================================================================ bullet controller

/// Parameters handed to an attack when the Knight starts it (obj_dbulletcontroller fields).
#[derive(Clone, Debug)]
pub struct DbCtrl {
    pub typ: i32,
    pub difficulty: f64,
    pub btimer: f64,
    pub made: f64,
    pub init: f64,
    pub side: f64,
    pub special: f64,
    pub size: f64,
    pub endtimer: f64,
    pub delay: f64,
    pub subdelay: f64,
    pub miny: f64,
    pub maxy: f64,
    pub minx: f64,
    pub maxx: f64,
    /// obj_knight_enemy instance id (GML creatorid)
    pub creatorid: Id,
    /// monster slot (GML creator)
    pub creator: f64,
    pub bulletmaker: Id,
    /// any other per-type variables
    pub vars: HashMap<&'static str, f64>,
}
impl DbCtrl {
    pub fn v(&self, k: &'static str) -> f64 { *self.vars.get(k).unwrap_or(&0.0) }
    pub fn set(&mut self, k: &'static str, v: f64) { self.vars.insert(k, v); }
}

/// obj_dbulletcontroller: dispatches to the attack's per-type logic every step.
pub struct DBulletController {
    pub c: DbCtrl,
}
impl Object for DBulletController {
    fn name(&self) -> &'static str { "obj_dbulletcontroller" }
    fn create(&mut self, me: &mut Inst, _g: &mut Game) {
        // obj_dbulletcontroller Create: bullet vars default to -1 (= "don't inherit")
        me.damage = -1.0;
        me.grazepoints = -1.0;
        me.timepoints = -1.0;
        me.inv = -1.0;
        me.grazed = -1.0;
        me.grazetimer = -1.0;
        me.target = -1.0;
        me.element = 0.0;
        me.visible = false;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.c.init == 0.0 {
            if let Some(id) = g.first("obj_growtangle") {
                if let Some(b) = g.inst(id) {
                    let (x, y, xs, ys, s) = (b.x, b.y, b.image_xscale, b.image_yscale, b.sprite_index);
                    let w = g.sprite_get_width(s) * xs;
                    let h = g.sprite_get_height(s) * ys;
                    self.c.miny = y - h / 2.0;
                    self.c.maxy = y + h / 2.0;
                    self.c.minx = x - w / 2.0;
                    self.c.maxx = x + w / 2.0;
                }
            }
            self.c.init = 1.0;
        }
        self.c.btimer += 1.0;
        crate::attacks::controller_step(&mut self.c, me, g);
    }
    fn draw(&mut self, _: &mut Inst, _: &mut Game) {}
    obj_vars!();
}

/// scr_bulletspawner + setup used by obj_knight_enemy for one attack.
pub fn spawn_dbulletcontroller(g: &mut Game, knight: &Inst, typ: i32, difficulty: f64, damage: Option<f64>) -> Id {
    let c = DbCtrl {
        typ,
        difficulty,
        btimer: 99.0,
        made: 0.0,
        init: 0.0,
        side: 1.0,
        special: 0.0,
        size: 0.0,
        endtimer: 0.0,
        delay: 0.0,
        subdelay: 0.0,
        miny: 999.0,
        maxy: 999.0,
        minx: 999.0,
        maxx: 999.0,
        creatorid: knight.id,
        creator: 0.0,
        bulletmaker: NOONE,
        vars: HashMap::new(),
    };
    let id = g.instance_create(knight.x, knight.y, Box::new(DBulletController { c }));
    let at = g.glob.monsterat[0];
    let mytarget = crate::battle::scene::mytarget(g);
    if let Some(i) = g.inst_mut(id) {
        i.target = mytarget;
        i.damage = damage.unwrap_or(at * 5.0);
    }
    id
}

// ============================================================================ misc helpers

/// The obj_knight_enemy instance (GML creatorid for every attack).
pub fn knight_id(g: &mut Game) -> Option<Id> { g.first("obj_knight_enemy") }

// ============================================================================ knight

/// obj_knight_enemy variables (Create_0 + scr_enemy_object_init). Behaviour lives in battle::knight.
/// Attack ports may read/write these, e.g. `g.get_first::<KnightEnemy>("obj_knight_enemy")`.
#[derive(Default)]
pub struct KnightEnemy {
    // scr_enemy_object_init
    pub myself: usize,
    pub becomeflash: f64,
    pub flash: f64,
    pub turns: f64,
    pub talktimer: f64,
    pub state: f64,
    pub siner: f64,
    pub talked: f64,
    pub attacked: f64,
    pub hurt: f64,
    pub hurttimer: f64,
    pub hurtshake: f64,
    pub shakex: f64,
    pub acttimer: f64,
    pub con: f64,
    pub mytarget: f64,
    pub acting: f64,
    pub actcon: f64,
    pub actingsus: f64,
    pub actingral: f64,
    pub actconsus: f64,
    pub actconral: f64,
    pub hurtspriteoffx: f64,
    pub hurtspriteoffy: f64,
    pub idlesprite: Spr,
    pub hurtsprite: Spr,
    pub sparedsprite: Spr,
    // obj_knight_enemy Create_0
    pub siner2: f64,
    pub aetimer: f64,
    pub phaseturn: f64,
    pub phase: f64,
    pub turn: f64,
    pub myattackchoice: f64,
    pub difficulty: f64,
    pub rotatingslash3used: bool,
    pub holdbreathcount: f64,
    pub sactcount: f64,
    pub ractcount: f64,
    pub chargeupcon: f64,
    pub chargeuptimer: f64,
    pub endcon: f64,
    pub endtimer: f64,
    pub end_cutscene_version: f64,
    pub balloonturn: f64,
    pub ballooncon: f64,
    pub balloonend: f64,
    pub blocking: f64,
    pub blockanim: f64,
    pub blocktimer: f64,
    pub damagereduction: f64,
    pub damagereductiontimer: f64,
    pub krisdamagereduction: f64,
    pub whiteflash: f64,
    pub haveusedroaring: bool,
    pub checkcount: f64,
    pub aoedamage: bool,
    pub stronghurtanim: bool,
    pub progamer: bool,
    pub krisdownmessage: bool,
    pub susiedownmessage: bool,
    pub ralseidownmessage: bool,
    pub setdownmessage: bool,
    pub damagecounter: f64,
    pub phase4turn: f64,
    pub rtimer: f64,
    pub attackchosen: bool,
}
