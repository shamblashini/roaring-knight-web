//! TECH & ITEM logic: scr_spellmenu_setup, scr_spellinfo, scr_spell (Rude Buster, Heal Prayer, Pacify, ...),
//! obj_spellphase, spell effect objects, scr_iteminfo / scr_iteminfo_temp / scr_itemuse, scr_heal.
//! OWNER: battle-core agent "spells". (stub)
#![allow(unused_variables)]

use crate::obj_vars;
use crate::rt::{Game, Inst, Object};

/// scr_spellmenu_setup(): rebuild global.battlespell* for every party slot.
pub fn scr_spellmenu_setup(g: &mut Game) {}

/// scr_heal(slot, amount) → amount actually healed.
pub fn scr_heal(g: &mut Game, slot: usize, amt: f64) -> f64 { 0.0 }

/// (item name, item description) for an item id (scr_iteminfo).
pub fn item_info(id: f64) -> (String, String) { (String::new(), String::new()) }

/// obj_spellphase
#[derive(Default)]
pub struct SpellPhase {}
impl Object for SpellPhase {
    fn name(&self) -> &'static str { "obj_spellphase" }
    obj_vars!();
}

#[allow(dead_code)]
fn _unused(_: &mut Inst) {}
