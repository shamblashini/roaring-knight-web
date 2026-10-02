//! Text: obj_writer (typewriter with Deltarune control codes), obj_face, obj_battleblcon (enemy balloons),
//! msgset / msgnext / scr_speaker / scr_anyface_next / scr_battletext / scr_battletext_default /
//! scr_enemyblcon / scr_terminate_writer.
//! OWNER: battle-core agent "writer". (stub)
#![allow(unused_variables)]

use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object, NOONE};

/// obj_writer
#[derive(Default)]
pub struct Writer {
    pub reachedend: f64,
    pub skipme: f64,
    pub dialoguer: f64,
    pub facer: f64,
}
impl Object for Writer {
    fn name(&self) -> &'static str { "obj_writer" }
    obj_vars!(reachedend, skipme);
}

/// msgset(i, str) / msgsetloc
pub fn msgset(g: &mut Game, i: usize, s: &str) {
    g.glob.msgno = i as f64;
    if g.glob.msg.len() <= i {
        g.glob.msg.resize(i + 1, String::new());
    }
    g.glob.msg[i] = s.to_string();
}
/// msgnext(str) / msgnextloc
pub fn msgnext(g: &mut Game, s: &str) {
    let n = g.glob.msgno as usize + 1;
    msgset(g, n, s);
}
/// scr_speaker(name)
pub fn scr_speaker(g: &mut Game, who: &str) {}
/// scr_anyface_next(name, face)
pub fn scr_anyface_next(g: &mut Game, who: &str, face: &str) {}
/// scr_battletext(): writer in the battle box from global.msg. Returns the writer id.
pub fn scr_battletext(g: &mut Game) -> Id { NOONE }
/// scr_battletext_default()
pub fn scr_battletext_default(g: &mut Game) -> Id { NOONE }
/// scr_enemyblcon(x, y, kind): speech balloon (+ writer for kind 0). Returns the created instance.
pub fn scr_enemyblcon(g: &mut Game, x: f64, y: f64, kind: i32) -> Id { NOONE }
/// scr_terminate_writer()
pub fn scr_terminate_writer(g: &mut Game) -> bool { true }

#[allow(dead_code)]
fn _unused(_: &mut Inst) {}
