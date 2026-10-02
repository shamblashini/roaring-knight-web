//! obj_knight_enemy (all events), its monster setup (scr_monstersetup type 104, encounter 115),
//! scr_enemy_hurt, obj_bgfountaintest, obj_block_vfx, obj_afterimage_fade_to_white.
//! OWNER: battle-core agent "knight". (stub)
#![allow(unused_variables)]

use crate::assets::Spr;
use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object};

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
impl Object for KnightEnemy {
    fn name(&self) -> &'static str { "obj_knight_enemy" }
    obj_vars!(siner2, aetimer, chargeupcon, chargeuptimer, whiteflash);
}

/// Create the knight monster in slot 0 at (x, y) and run its monster setup. Returns its id.
pub fn create_knight(g: &mut Game, x: f64, y: f64) -> Id {
    g.instance_create(x, y, Box::new(KnightEnemy::default()))
}

#[allow(dead_code)]
fn _unused(_: Spr, _: &mut Inst) {}
