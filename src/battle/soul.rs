//! SOUL, bullet board and hero damage: obj_heart, obj_grazebox, obj_growtangle (obj_battlesolid),
//! obj_moveheart, obj_returnheart, obj_heartburst, obj_darkener, obj_shake; scr_damage, scr_dead, scr_moveheart,
//! and the collision events (obj_heart / obj_grazebox vs obj_collidebullet).
//! OWNER: battle-core agent "soul". (stub)
#![allow(unused_variables)]

use crate::assets::{Spr, NO_SPR};
use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object};

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
impl Object for Heart {
    fn name(&self) -> &'static str { "obj_heart" }
    obj_vars!(wspeed, canmove, color);
}
impl Object for Growtangle {
    fn name(&self) -> &'static str { "obj_growtangle" }
    obj_vars!(maxxscale, maxyscale, growcon, target_angle);
}

/// obj_grazebox
#[derive(Default)]
pub struct Grazebox {
    pub grazetimer: f64,
    pub grazetpfactor: f64,
    pub grazetimefactor: f64,
    pub grazesizefactor: f64,
    pub grazecount: f64,
}
impl Object for Grazebox {
    fn name(&self) -> &'static str { "obj_grazebox" }
    obj_vars!(grazetimer);
}

/// obj_darkener (dims the battle background during the enemy turn)
#[derive(Default)]
pub struct Darkener {
    pub darken: f64,
}
impl Object for Darkener {
    fn name(&self) -> &'static str { "obj_darkener" }
    obj_vars!(darken);
}

/// obj_shake (screen shake)
#[derive(Default)]
pub struct Shake {
    pub shakex: f64,
    pub shakey: f64,
    pub shakespeed: f64,
}
impl Object for Shake {
    fn name(&self) -> &'static str { "obj_shake" }
    obj_vars!(shakex, shakey, shakespeed);
}

/// The game-specific collision pass (obj_heart and obj_grazebox Collision events), run after motion.
pub fn collision_pass(g: &mut Game) {}

/// scr_damage(): see battle::scr_damage.
pub fn scr_damage_impl(me: &mut Inst, g: &mut Game) {}

/// scr_dead(slot)
pub fn scr_dead(g: &mut Game, slot: usize) {
    g.glob.charmove[slot] = 0.0;
    g.glob.charcantarget[slot] = 0.0;
    g.glob.chardead[slot] = 1.0;
    g.glob.charaction[slot] = 0.0;
    g.glob.charspecial[slot] = 0.0;
}

/// scr_moveheart(): the SOUL flies from Kris into the board. Returns the obj_moveheart id.
pub fn scr_moveheart(g: &mut Game) -> Id { crate::rt::NOONE }

#[allow(dead_code)]
fn _unused(_: Spr) -> Spr { NO_SPR }
