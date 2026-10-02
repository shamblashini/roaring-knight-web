//! Battle scene: party, menus, turn flow. (work in progress)

use crate::rt::{Game, Inst};

pub struct Scene {}

impl Scene {
    pub fn new(_g: &mut Game) -> Scene { Scene {} }
}

pub fn scene_update(_s: &mut Scene, _g: &mut Game) {}
pub fn scene_draw_under(_s: &mut Scene, g: &mut Game) {
    let s = crate::assets::spr("spr_roaringknight_idle");
    g.draw_sprite_ext(s, 0.0, 400.0, 100.0, 2.0, 2.0, 0.0, 0xFFFFFF, 1.0);
    g.draw_set_font(crate::assets::font("fnt_main"));
    g.draw_text(20.0, 20.0, "THE ROARING KNIGHT");
}
pub fn scene_draw_over(_s: &mut Scene, _g: &mut Game) {}
pub fn collision_pass(_g: &mut Game) {}
pub fn scr_damage_impl(_me: &mut Inst, _g: &mut Game) {}
pub fn mytarget(_g: &mut Game) -> f64 { 0.0 }
