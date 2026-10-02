//! obj_heroparent (obj_herokris / obj_herosusie / obj_heroralsei), obj_attackpress (FIGHT bars),
//! obj_dmgwriter (damage numbers), obj_tensionbar (TP bar), scr_damage_enemy.
//! OWNER: battle-core agent "heroes". (stub)
#![allow(unused_variables)]

use crate::assets::{Spr, NO_SPR};
use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object, NOONE};

/// obj_heroparent instance. `obj` is "obj_herokris", "obj_herosusie" or "obj_heroralsei".
pub struct Hero {
    pub obj: &'static str,
    pub char: f64,
    pub myself: f64,
    pub state: f64,
    pub hurt: f64,
    pub hurttimer: f64,
    pub attacktimer: f64,
    pub attacked: f64,
    pub index: f64,
    pub itemed: f64,
    pub tu: f64,
    pub myheight: f64,
    pub mywidth: f64,
    pub flash: f64,
    pub fsiner: f64,
    pub becomeflash: f64,
    pub darkify: f64,
    pub normalsprite: Spr,
    pub idlesprite: Spr,
    pub defendsprite: Spr,
    pub hurtsprite: Spr,
    pub attackreadysprite: Spr,
    pub attacksprite: Spr,
    pub itemsprite: Spr,
    pub itemreadysprite: Spr,
    pub actreadysprite: Spr,
    pub actsprite: Spr,
    pub spellreadysprite: Spr,
    pub spellsprite: Spr,
    pub defeatsprite: Spr,
    pub victorysprite: Spr,
}
impl Default for Hero {
    fn default() -> Self {
        Hero {
            obj: "obj_herokris",
            char: 1.0,
            myself: 0.0,
            state: 0.0,
            hurt: 0.0,
            hurttimer: 0.0,
            attacktimer: 0.0,
            attacked: 0.0,
            index: 0.0,
            itemed: 0.0,
            tu: 0.0,
            myheight: 37.0,
            mywidth: 34.0,
            flash: 0.0,
            fsiner: 0.0,
            becomeflash: 0.0,
            darkify: 0.0,
            normalsprite: NO_SPR,
            idlesprite: NO_SPR,
            defendsprite: NO_SPR,
            hurtsprite: NO_SPR,
            attackreadysprite: NO_SPR,
            attacksprite: NO_SPR,
            itemsprite: NO_SPR,
            itemreadysprite: NO_SPR,
            actreadysprite: NO_SPR,
            actsprite: NO_SPR,
            spellreadysprite: NO_SPR,
            spellsprite: NO_SPR,
            defeatsprite: NO_SPR,
            victorysprite: NO_SPR,
        }
    }
}
impl Object for Hero {
    fn name(&self) -> &'static str { self.obj }
    obj_vars!(state, hurt, hurttimer, flash, darkify);
}

/// Create Kris, Susie and Ralsei at global.heromakex/y, filling global.charinstance.
pub fn create_heroes(g: &mut Game) {}

/// obj_dmgwriter
#[derive(Default)]
pub struct DmgWriter {
    pub damage: f64,
    /// GML `type`
    pub typ: f64,
    pub delay: f64,
    pub delaytimer: f64,
    pub specialmessage: f64,
    pub killactive: f64,
    pub active: f64,
}
impl Object for DmgWriter {
    fn name(&self) -> &'static str { "obj_dmgwriter" }
    obj_vars!(damage, delay);
}
/// instance_create(x, y, obj_dmgwriter) (Create event runs; set damage/typ afterwards via g.get::<DmgWriter>).
pub fn dmgwriter(g: &mut Game, x: f64, y: f64) -> Id { NOONE }

/// obj_tensionbar
#[derive(Default)]
pub struct TensionBar {}
impl Object for TensionBar {
    fn name(&self) -> &'static str { "obj_tensionbar" }
    obj_vars!();
}
pub fn create_tensionbar(g: &mut Game) -> Id { NOONE }

/// obj_attackpress (FIGHT timing bars)
#[derive(Default)]
pub struct AttackPress {}
impl Object for AttackPress {
    fn name(&self) -> &'static str { "obj_attackpress" }
    obj_vars!();
}

/// scr_damage_enemy(target, damage) called by party member `caster` (party slot 0-2; 5 = special).
pub fn scr_damage_enemy(g: &mut Game, caster: f64, target: usize, damage: f64) {}

#[allow(dead_code)]
fn _unused(_: &mut Inst) {}
