//! obj_battlecontroller (menus, HUD/char boxes, turn flow, bullet-phase timer) and the turn scripts:
//! scr_battle setup for encounter 115, scr_nexthero / scr_prevhero / scr_endturn / scr_attackphase /
//! scr_nextact / scr_mnendturn / scr_randomtarget / scr_charbox.
//! OWNER: battle-core agent "controller". (stub)
#![allow(unused_variables)]

use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object, NOONE};

/// obj_battlecontroller
#[derive(Default)]
pub struct BattleController {
    pub intro: f64,
    pub noreturn: f64,
    pub cantspare: [f64; 3],
    pub mercytotal: f64,
    pub grazenoise: f64,
    pub timeron: f64,
    pub reset: f64,
    pub bp: f64,
    pub bpy: f64,
    pub victory: f64,
    pub hidemercy: f64,
    pub disablesusieact: f64,
    pub battlewriter: Id,
}
impl Object for BattleController {
    fn name(&self) -> &'static str { "obj_battlecontroller" }
    obj_vars!(intro, bp);
}

/// Set up the Roaring Knight battle (party stats, inventory, encounter 115, heroes, knight, TP bar, controller).
pub fn start_battle(g: &mut Game) {}
pub fn scr_attackphase(g: &mut Game) {}
pub fn scr_nextact(g: &mut Game) {}
pub fn scr_endturn(g: &mut Game) {}
pub fn scr_nexthero(g: &mut Game) {}
pub fn scr_mnendturn(g: &mut Game) {}
/// scr_randomtarget(): returns `mytarget` (0-2, 3 = all, 4 = random-living in ch2+), sets global.targeted.
pub fn scr_randomtarget(g: &mut Game) -> f64 { 4.0 }

#[allow(dead_code)]
fn _unused(_: &mut Inst) -> Id { NOONE }
