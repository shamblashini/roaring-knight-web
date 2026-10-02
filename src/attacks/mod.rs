//! Ports of the Roaring Knight's attack patterns (bullet types reachable from obj_knight_enemy).
//!
//! | myattackchoice | dbulletcontroller type | module |
//! |---|---|---|
//! | 1  (Stars)            | 98  | `stars`          |
//! | 2  (Flurry)           | 99  | `flurry`         |
//! | 5  (rotating slash)   | 104 | `rotating_slash` |
//! | 9  (the Roaring)      | 107 | `roaring`        |
//! | 11/14/15/16/17 (tracking swords) | 151 | `swords` |
//! | 15 (sword vortex)     | 154 | `swords`         |
//! | 13 (sword tunnel)     | 153 | `sword_tunnel`   |

pub mod common;
pub mod flurry;
pub mod roaring;
pub mod rotating_slash;
pub mod stars;
pub mod sword_tunnel;
pub mod swords;

use crate::battle::DbCtrl;
use crate::rt::{Game, Inst};

/// obj_dbulletcontroller Step_0: the `if (type == N)` block for each attack.
pub fn controller_step(c: &mut DbCtrl, me: &mut Inst, g: &mut Game) {
    match c.typ {
        98 => stars::ctrl_step(c, me, g),
        99 => flurry::ctrl_step(c, me, g),
        104 => rotating_slash::ctrl_step(c, me, g),
        107 => roaring::ctrl_step(c, me, g),
        151 | 154 => swords::ctrl_step(c, me, g),
        153 => sword_tunnel::ctrl_step(c, me, g),
        _ => {}
    }
}
