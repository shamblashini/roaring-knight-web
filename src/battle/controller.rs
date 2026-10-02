//! obj_battlecontroller (menus, HUD/char boxes, turn flow, bullet-phase timer) and the turn scripts:
//! scr_battle setup for encounter 115, scr_nexthero / scr_prevhero / scr_endturn / scr_attackphase /
//! scr_nextact / scr_act_simul / scr_mnendturn / scr_randomtarget / scr_charbox / scr_selectionmatrix /
//! scr_charcan / scr_battlecursor_memory_reset.
//! OWNER: battle-core agent "controller".
//!
//! The GML scripts run in the scope of obj_battlecontroller (they use its `tempitem`, `movenoise`, ...).
//! Here they are methods on [`BattleController`]; the `pub fn scr_*` wrappers find the controller instance
//! and run the method on it (for calls coming from other objects).

use crate::assets::{font, spr};
use crate::battle::heroes::{self, Hero};
use crate::battle::knight::{self, KnightEnemy};
use crate::battle::soul::Darkener;
use crate::battle::spells;
use crate::battle::writer::{self, Writer};
use crate::gfx::{merge_color, Color, C_AQUA, C_BLACK, C_BLUE, C_DKGRAY, C_FUCHSIA, C_GRAY, C_LIME, C_MAROON, C_ORANGE, C_PURPLE, C_RED, C_WHITE, C_YELLOW};
use crate::gm::{ceil, floor, PI};
use crate::input::Key;
use crate::obj_vars;
use crate::rt::{Game, Id, Inst, Object, NOONE};

/// GML array index from a (possibly negative) number.
#[inline]
fn ix(v: f64) -> usize { if v > 0.0 { v as usize } else { 0 } }

/// GML string(n)
fn gstr(v: f64) -> String {
    if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{:.2}", v) }
}

/// Item properties used by the battle menus (scr_iteminfo: itemtarget, usable, replaceable) for the
/// items that exist in this fight's inventory (plus the TP items).
fn item_props(id: f64) -> (f64, f64, f64) {
    // (itemtarget, usable, replaceable)
    match id as i32 {
        2 | 8 | 16 | 23 | 24 | 34 | 37 | 39 => (1.0, 1.0, 0.0),
        22 => (1.0, 1.0, 8.0),
        11 | 27 | 38 => (2.0, 1.0, 0.0),
        28 | 29 => (2.0, 1.0, 0.0),
        _ => (0.0, 0.0, 0.0),
    }
}

/// HP numbers in the char boxes (global.hpfont).
fn draw_hpfont_text(g: &mut Game, x: f64, y: f64, text: &str, halign_right: bool) {
    crate::battle::heroes::draw_hpfont_text(g, x, y, text, halign_right)
}

fn knight_end_cutscene(g: &mut Game) -> bool {
    g.get_first::<KnightEnemy>("obj_knight_enemy").map(|(_, k)| k.end_cutscene_version > 0.0).unwrap_or(false)
}

/// `with (inst) { if (flash == 0) fsiner = 0; flash = 1; becomeflash = 1; }` on a monster or hero.
fn flash_instance(g: &mut Game, id: Id) {
    if let Some((_, k)) = g.get::<KnightEnemy>(id) {
        // NOTE: KnightEnemy has no `fsiner` field; only flash/becomeflash are set.
        k.flash = 1.0;
        k.becomeflash = 1.0;
        return;
    }
    if let Some((_, h)) = g.get::<Hero>(id) {
        if h.flash == 0.0 {
            h.fsiner = 0.0;
        }
        h.flash = 1.0;
        h.becomeflash = 1.0;
    }
}

fn set_depth_all(g: &mut Game, obj: &str, d: f64) {
    for id in g.ids_of(obj) {
        if let Some(i) = g.inst_mut(id) {
            i.depth = d;
        }
    }
}

// ============================================================================ obj_returnheart (fallback)

/// obj_returnheart: the SOUL flies back to Kris after the bullet phase.
/// FALLBACK: battle::soul owns this object but its stub exposes no constructor, so it is ported here.
/// TODO: replace with the soul module's obj_returnheart once it exists (and create obj_heartburst in alarm 0).
#[derive(Default)]
pub struct ReturnHeartFallback {
    pub burst: f64,
    pub shift: f64,
    pub flytime: f64,
    pub distx: f64,
    pub disty: f64,
    pub dist: f64,
}
impl Object for ReturnHeartFallback {
    fn name(&self) -> &'static str { "obj_returnheart" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.burst = 0.0;
        self.shift = 1.0;
        me.image_alpha = 1.0;
        self.flytime = 8.0;
        let (kx, ky) = g.first_inst("obj_herokris").map(|k| (k.x, k.y)).unwrap_or((0.0, 0.0));
        self.distx = kx + 10.0;
        self.disty = ky + 40.0;
        self.dist = crate::gm::point_distance(me.x, me.y, self.distx, self.disty);
        me.move_towards_point(self.distx, self.disty, self.dist / self.flytime);
        me.alarm[0] = self.flytime as i32;
        me.image_speed = 0.0;
    }
    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 0 {
            me.x = self.distx;
            me.y = self.disty;
            // TODO: instance_create(x, y, obj_heartburst) (battle::soul)
            g.destroy_self(me);
        }
    }
    obj_vars!(burst, shift);
}

// ============================================================================ obj_battlecontroller

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
    // ---- added
    pub victoried: f64,
    pub skipvictory: f64,
    pub victortimer: f64,
    pub lastbattlewriter: Id,
    pub myface: Id,
    pub lbuffer: f64,
    pub rbuffer: f64,
    pub onebuffer: f64,
    pub twobuffer: f64,
    pub myfightreturntimer: f64,
    pub messagepriority: f64,
    pub attackpriority: f64,
    /// tempitem[item][party slot]
    pub tempitem: [[f64; 3]; 13],
    pub tempitemnameb: [[String; 3]; 12],
    pub tempitemdescb: [[String; 3]; 12],
    pub itempage: f64,
    pub spellpage: f64,
    pub movenoise: f64,
    pub selnoise: f64,
    pub laznoise: f64,
    pub damagenoise: f64,
    pub bcolor: Color,
    pub tp: f64,
    pub tpy: f64,
    pub chartotal: f64,
    /// charpos[c]: party slot of character c (0 Kris, 1 Susie, 2 Ralsei, 3 Noelle), -1 if absent
    pub charpos: [f64; 4],
    pub havechar: [f64; 4],
    pub mmy: [f64; 4],
    pub slmxx: f64,
    pub slmyy: f64,
    pub s_siner: f64,
    pub t_siner: f64,
    pub hpcolor: [Color; 4],
    pub hpcolorsoft: [Color; 4],
    pub charcolor: Color,
    pub idefendedthisturn: f64,
    pub oopsallacts: f64,
    pub rouxlsgridenabled: bool,
    // scr_actinfo_temp
    pub canact: [f64; 6],
    pub acttpcost: [f64; 6],
    pub actsimul: [f64; 6],
    pub thisenemy: f64,
    pub thischar: f64,
    // scr_iteminfo results
    pub itemtarget: f64,
    pub usable: f64,
    pub replaceable: f64,
    pub techwon: f64,
    pub fightphase: f64,
    pub moveswapped: f64,
    pub prevturn: f64,
}

impl BattleController {
    // ------------------------------------------------------------------ helpers
    fn ct(g: &Game) -> usize { ix(g.glob.charturn) }

    /// scr_iteminfo(id) — stores itemtarget/usable/replaceable.
    fn iteminfo(&mut self, id: f64) {
        let (t, u, r) = item_props(id);
        self.itemtarget = t;
        self.usable = u;
        self.replaceable = r;
    }

    /// scr_iteminfo_temp(slot)
    fn iteminfo_temp(&mut self, slot: usize) {
        for i in 0..12 {
            let id = self.tempitem[i][slot];
            let (n, d) = spells::item_info(id);
            self.tempitemnameb[i][slot] = if id == 0.0 { " ".to_string() } else { n };
            self.tempitemdescb[i][slot] = if id == 0.0 { "---".to_string() } else { d };
        }
    }

    /// scr_itemshift_temp(coord, slot)
    fn itemshift_temp(&mut self, coord: usize, slot: usize) {
        self.tempitem[12][slot] = 0.0;
        for i in coord..12 {
            self.tempitem[i][slot] = self.tempitem[i + 1][slot];
        }
    }

    /// scr_actinfo_temp(enemy)
    fn actinfo_temp(&mut self, g: &Game, _enemy: usize) {
        // only monster slot 0 exists in this fight (global.canact* are per-slot-0 arrays)
        let c = g.glob.char[Self::ct(g)];
        for a in 0..6 {
            self.canact[a] = 0.0;
            match c {
                1 => {
                    self.canact[a] = g.glob.canact[a];
                    self.acttpcost[a] = g.glob.actcost[a];
                    self.actsimul[a] = g.glob.actsimul[a];
                }
                2 => {
                    self.canact[a] = g.glob.canactsus[a];
                    self.acttpcost[a] = g.glob.actcostsus[a];
                    self.actsimul[a] = g.glob.actsimulsus[a];
                }
                3 => {
                    self.canact[a] = g.glob.canactral[a];
                    self.acttpcost[a] = g.glob.actcostral[a];
                    self.actsimul[a] = g.glob.actsimulral[a];
                }
                _ => {}
            }
        }
    }

    /// scr_actselect(enemy, act)
    fn actselect(&mut self, g: &mut Game, enemy: usize, act: usize) {
        let ct = Self::ct(g);
        let c = g.glob.char[ct];
        let mid = g.glob.monsterinstance[enemy.min(2)];
        if let Some((_, k)) = g.get::<KnightEnemy>(mid) {
            match c {
                1 => k.acting = act as f64 + 1.0,
                2 => k.actingsus = act as f64 + 1.0,
                3 => k.actingral = act as f64 + 1.0,
                _ => {}
            }
        }
        if c == 1 {
            g.glob.actingsimul[0] = self.actsimul[act];
            g.glob.acting[0] = 1.0;
            g.glob.actingsingle[0] = 1.0;
            g.glob.actingtarget[ct] = enemy as f64;
            let actor = g.glob.actactor[act];
            if actor == 2.0 && self.charpos[1] >= 0.0 {
                g.glob.acting[ix(self.charpos[1])] = 1.0;
            }
            if actor == 3.0 && self.charpos[2] >= 0.0 {
                g.glob.acting[ix(self.charpos[2])] = 1.0;
            }
            if actor == 4.0 {
                g.glob.acting[2] = 1.0;
                g.glob.acting[1] = 1.0;
            }
            if actor == 5.0 && self.charpos[3] >= 0.0 {
                g.glob.acting[ix(self.charpos[3])] = 1.0;
            }
            for i in 0..3 {
                if g.glob.acting[i] == 1.0 {
                    g.glob.faceaction[i] = 6.0;
                    g.glob.charaction[i] = 9.0;
                }
            }
        } else {
            g.glob.actingtarget[ct] = enemy as f64;
            g.glob.actingsingle[ct] = 1.0;
            g.glob.actingsimul[ct] = self.actsimul[act];
            g.glob.faceaction[ct] = 6.0;
            g.glob.charaction[ct] = 9.0;
        }
    }

    /// scr_itemconsumeb()
    fn itemconsumeb(&mut self, g: &mut Game) {
        let ct = Self::ct(g);
        let coord = ix(g.glob.bmenucoord[4][ct]);
        g.glob.faceaction[ct] = 3.0;
        g.glob.charaction[ct] = 4.0;
        g.glob.charspecial[ct] = self.tempitem[coord][ct] + 200.0;
        if self.usable == 1.0 && self.replaceable == 0.0 {
            self.itemshift_temp(coord, ct);
        } else if self.replaceable > 0.0 {
            self.tempitem[coord][ct] = self.replaceable;
        }
        self.nexthero(g);
    }

    /// scr_spellconsumeb() (global.flag[34] == 0 path)
    fn spellconsumeb(&mut self, g: &mut Game) {
        let ct = Self::ct(g);
        let coord = ix(g.glob.bmenucoord[2][ct]);
        // `cost` from the last scr_spellinfo call = this entry's cost
        let cost = g.glob.battlespellcost[ct][coord];
        g.glob.tension -= floor(floor((cost / g.glob.maxtension) * 100.0) * 2.5);
        g.glob.faceaction[ct] = 2.0;
        g.glob.charaction[ct] = 2.0;
        g.glob.charspecial[ct] = g.glob.battlespell[ct][coord];
        g.glob.tensionselect = 0.0;
        // spellanim is always 0 for these spells: `spellframes = remspellframes; spellsprite = remspellsprite`
        // on the hero — Hero has no rem* fields (they never change in this fight), so nothing to do.
        self.nexthero(g);
    }

    fn init_tempitem(&mut self, g: &Game) {
        for i in 0..12 {
            for j in 0..3 {
                self.tempitem[i][j] = g.glob.item[i];
            }
        }
    }

    // ------------------------------------------------------------------ turn scripts

    /// scr_charcan(slot)
    pub fn charcan(g: &Game, slot: usize) -> bool {
        let mut can = true;
        if g.glob.hp[g.glob.char[slot]] <= 0.0 {
            can = false;
        }
        if g.glob.acting[slot] == 1.0 {
            can = false;
        }
        if g.glob.char[slot] == 0 {
            can = false;
        }
        if g.glob.charmove[slot] == 0.0 {
            can = false;
        }
        // global.charauto is 0 for everyone here
        can
    }

    /// scr_nexthero()
    pub fn nexthero(&mut self, g: &mut Game) {
        self.moveswapped = 0.0;
        self.prevturn = g.glob.charturn;
        if g.glob.charturn == 0.0 {
            self.moveswapped = 1.0;
            if g.glob.charmove[1] == 1.0 && Self::charcan(g, 1) {
                g.glob.charturn = 1.0;
            } else if g.glob.charmove[2] == 1.0 && Self::charcan(g, 2) {
                g.glob.charturn = 2.0;
            } else {
                self.endturn(g);
            }
        }
        if g.glob.charturn == 1.0 && self.moveswapped == 0.0 {
            self.moveswapped = 1.0;
            if Self::charcan(g, 2) && g.glob.acting[1] == 0.0 {
                g.glob.charturn = 2.0;
            } else {
                self.endturn(g);
            }
        }
        // (Ralsei yarn sprite check: Rouxls fight only)
        if g.glob.charturn == 2.0 && self.moveswapped == 0.0 {
            self.endturn(g);
        }
        if self.moveswapped == 1.0 {
            g.glob.bmenuno = 0.0;
        }
        if g.glob.charturn > 0.0 && g.glob.charturn < 3.0 {
            let ct = Self::ct(g);
            let pt = ix(self.prevturn).min(2);
            g.glob.temptension[ct] = g.glob.tension;
            for i in 0..12 {
                self.tempitem[i][ct] = self.tempitem[i][pt];
            }
        }
    }

    /// scr_prevhero()
    pub fn prevhero(&mut self, g: &mut Game) {
        self.prevturn = g.glob.charturn;
        self.moveswapped = 0.0;
        if g.glob.charturn == 1.0 && g.glob.charmove[0] == 1.0 {
            g.glob.charturn = 0.0;
            self.moveswapped = 1.0;
        }
        if g.glob.charturn == 2.0 {
            self.moveswapped = 1.0;
            if g.glob.charmove[1] == 1.0 && g.glob.acting[1] == 0.0 {
                g.glob.charturn = 1.0;
            } else if g.glob.charmove[0] == 1.0 {
                g.glob.charturn = 0.0;
            }
        }
        if self.moveswapped == 1.0 {
            g.glob.bmenuno = 0.0;
            let ct = Self::ct(g);
            let c = g.glob.char[ct];
            if c == 3 || c == 2 {
                for id in g.ids_of("obj_monsterparent") {
                    if let Some((_, k)) = g.get::<KnightEnemy>(id) {
                        if c == 3 {
                            k.actingral = 0.0;
                        } else {
                            k.actingsus = 0.0;
                        }
                    }
                }
            }
            g.glob.actingsingle[ct] = 0.0;
            g.glob.actingsimul[ct] = 0.0;
            g.glob.faceaction[ct] = 0.0;
            g.glob.chartarget[ct] = 0.0;
            g.glob.charaction[ct] = 0.0;
            g.glob.charspecial[ct] = 0.0;
            self.movenoise = 1.0;
        }
        if self.idefendedthisturn > 0.0 {
            self.idefendedthisturn -= 1.0;
            self.mercytotal -= 40.0;
        }
        if g.glob.charturn == 0.0 {
            for id in g.ids_of("obj_monsterparent") {
                if let Some((_, k)) = g.get::<KnightEnemy>(id) {
                    k.acting = 0.0;
                }
            }
            g.glob.acting = [0.0; 3];
            g.glob.faceaction[1] = 0.0;
            g.glob.chartarget[1] = 0.0;
            g.glob.charaction[1] = 0.0;
            g.glob.charspecial[1] = 0.0;
            g.glob.faceaction[2] = 0.0;
            g.glob.tension = g.glob.temptension[0];
            for i in 0..12 {
                self.tempitem[i][0] = g.glob.item[i];
            }
        } else {
            let ct = Self::ct(g).min(2);
            g.glob.tension = g.glob.temptension[ct];
            for i in 0..12 {
                self.tempitem[i][ct] = self.tempitem[i][ct - 1];
            }
        }
    }

    /// scr_endturn()
    pub fn endturn(&mut self, g: &mut Game) {
        let ct = Self::ct(g).min(2);
        for i in 0..12 {
            g.glob.item[i] = self.tempitem[i][ct];
        }
        self.init_tempitem(g);
        self.moveswapped = 0.0;
        g.destroy_all("obj_writer");
        g.destroy_all("obj_face");
        g.destroy_all("obj_smallface");
        self.idefendedthisturn = 0.0;
        g.glob.attacking = 0.0;
        for i in 0..3 {
            g.glob.monsterattackname[i] = " ".to_string();
            // (global.charauto is 0 for everyone)
            if g.glob.charaction[i] == 1.0 {
                g.glob.attacking = 1.0;
            }
        }
        let mut noactors = true;
        if g.glob.acting[0] == 1.0 {
            noactors = false;
        }
        for i in 0..3 {
            if g.glob.actingsingle[i] == 1.0 {
                noactors = false;
            }
        }
        if noactors {
            self.attackphase(g);
        } else {
            g.glob.charturn = 3.0;
            g.glob.myfight = 3.0;
            g.glob.currentactingchar = 0.0;
            if g.glob.acting[0] == 0.0 {
                self.nextact(g);
            }
            if g.glob.acting[0] == 1.0 && g.glob.actingsimul[0] == 1.0 {
                self.act_simul(g);
            }
        }
        self.messagepriority = -1.0;
        self.attackpriority = -1.0;
    }

    /// scr_monsterpop()
    fn monsterpop(g: &Game) -> f64 { g.glob.monster[0] + g.glob.monster[1] + g.glob.monster[2] }

    /// scr_wincombat() (global.flag[60] == 0 path)
    fn wincombat(&mut self, g: &mut Game) {
        g.glob.myfight = 7.0;
        g.glob.mnfight = -1.0;
        self.victory = 1.0;
        // TODO: scr_monsterdefeat on remaining monsters (heroes module declares no fn for it).
    }

    /// scr_attackphase()
    pub fn attackphase(&mut self, g: &mut Game) {
        self.techwon = 0.0;
        if Self::monsterpop(g) == 0.0 {
            self.techwon = 1.0;
        }
        if self.techwon == 1.0 {
            self.wincombat(g);
        }
        if self.techwon == 0.0 {
            g.glob.hittarget = [0.0; 3];
            self.fightphase = 1.0;
            g.glob.charturn = 3.0;
            for i in 0..3 {
                if g.glob.charaction[i] == 4.0 || g.glob.charaction[i] == 2.0 {
                    self.fightphase = 0.0;
                }
            }
            if g.glob.myfight == 4.0 {
                self.fightphase = 1.0;
            }
            let (xx, yy) = (g.camerax(), g.cameray());
            if self.fightphase == 1.0 {
                g.glob.myfight = 1.0;
                g.instance_create(xx + 2.0, yy + 365.0, Box::new(heroes::AttackPress::default()));
            } else {
                g.glob.myfight = 4.0;
                g.instance_create(0.0, 0.0, Box::new(spells::SpellPhase::default()));
            }
        }
    }

    /// scr_nextact()
    pub fn nextact(&mut self, g: &mut Game) {
        g.glob.acting = [0.0; 3];
        let cac = ix(g.glob.currentactingchar);
        if cac < 3 {
            g.glob.actingsingle[cac] = 0.0;
        }
        let tgt = ix(g.glob.actingtarget[cac.min(2)]).min(2);
        let mi = g.glob.monsterinstance[tgt];
        if let Some((_, k)) = g.get::<KnightEnemy>(mi) {
            k.acting = 0.0;
            k.actcon = 0.0;
            k.actconsus = 0.0;
            k.actconral = 0.0;
        }
        let mut singleactcomplete = false;
        while g.glob.currentactingchar < 3.0 {
            g.glob.currentactingchar += 1.0;
            if g.glob.currentactingchar < 3.0 {
                let c = ix(g.glob.currentactingchar);
                if g.glob.actingsingle[c] == 1.0 {
                    let mi = g.glob.monsterinstance[ix(g.glob.actingtarget[c]).min(2)];
                    let ch = g.glob.char[c];
                    if ch == 2 || ch == 3 {
                        if let Some((_, k)) = g.get::<KnightEnemy>(mi) {
                            if ch == 2 {
                                k.actconsus = 1.0;
                            } else {
                                k.actconral = 1.0;
                            }
                        }
                        if g.glob.actingsimul[c] == 0.0 {
                            singleactcomplete = true;
                        }
                        break;
                    }
                }
            }
        }
        if g.glob.currentactingchar >= 3.0 {
            for id in g.ids_of("obj_monsterparent") {
                if let Some((_, k)) = g.get::<KnightEnemy>(id) {
                    k.acting = 0.0;
                    k.actingsus = 0.0;
                    k.actingral = 0.0;
                }
            }
            g.glob.currentactingchar = 0.0;
            self.attackphase(g);
        } else if !singleactcomplete {
            self.act_simul(g);
        }
    }

    /// scr_act_simul()
    pub fn act_simul(&mut self, g: &mut Game) {
        // NOTE: obj_monsterparent.simulorder{kri,sus,ral} / simultotal are not fields of KnightEnemy;
        // only the actcon* flags are set.
        let mut simulcount = 0.0;
        let mut ii = ix(g.glob.currentactingchar);
        while ii < 3 {
            let mut found = false;
            if g.glob.actingsingle[ii] == 1.0 && g.exists("obj_monsterparent") {
                let choice = ix(g.glob.actingchoice[ii]).min(5);
                let mi = g.glob.monsterinstance[ix(g.glob.actingtarget[ii]).min(2)];
                let ch = g.glob.char[ii];
                if ch == 1 && g.glob.actsimul[choice] == 1.0 {
                    if let Some((_, k)) = g.get::<KnightEnemy>(mi) {
                        k.actcon = 0.0;
                    }
                    found = true;
                }
                if ch == 2 && g.glob.actsimulsus[choice] == 1.0 {
                    if let Some((_, k)) = g.get::<KnightEnemy>(mi) {
                        k.actconsus = 1.0;
                    }
                    found = true;
                }
                if ch == 3 && g.glob.actsimulral[choice] == 1.0 {
                    if let Some((_, k)) = g.get::<KnightEnemy>(mi) {
                        k.actconral = 1.0;
                    }
                    found = true;
                }
                if found {
                    g.glob.actingsingle[ii] = 0.0;
                    simulcount += 1.0;
                }
            }
            ii += 1;
        }
        let _ = simulcount;
    }

    /// scr_mnendturn()
    pub fn mnendturn(&mut self, g: &mut Game) {
        self.techwon = 0.0;
        if Self::monsterpop(g) == 0.0 {
            self.techwon = 1.0;
        }
        if g.glob.flag(39) == 1.0 {
            self.techwon = 1.0;
        }
        if self.techwon == 1.0 {
            self.wincombat(g);
        }
        if self.techwon == 0.0 {
            self.messagepriority = -1.0;
            self.attackpriority = -1.0;
            scr_battlecursor_memory_reset(g);
            g.glob.mnfight = 0.0;
            g.glob.myfight = 0.0;
            g.glob.bmenuno = 0.0;
            g.glob.charturn = 0.0;
            let mut skip = false;
            for i in 0..3 {
                g.glob.hittarget[i] = 0.0;
                let ci = g.glob.charinstance[i];
                if let Some((_, h)) = g.get::<Hero>(ci) {
                    h.tu = 0.0;
                }
                let hptarget = g.glob.char[i];
                if hptarget != 0 && g.glob.hp[hptarget] <= 0.0 {
                    if g.exists("obj_knight_enemy") {
                        // the Knight fight: downed heroes don't auto-revive
                    } else {
                        let healamt = ceil(g.glob.maxhp[hptarget] / 8.0);
                        let (hx, hy, mh) = match g.get::<Hero>(ci) {
                            Some((hi, h)) => (hi.x, hi.y, h.myheight),
                            None => (0.0, 0.0, 0.0),
                        };
                        let dw = heroes::dmgwriter(g, hx, hy + mh - 24.0);
                        if let Some((_, d)) = g.get::<heroes::DmgWriter>(dw) {
                            d.delay = 1.0;
                            d.typ = 3.0;
                        }
                        let healed = spells::scr_heal(g, i, healamt);
                        let revived = g.glob.hp[hptarget] >= 1.0;
                        if let Some((_, d)) = g.get::<heroes::DmgWriter>(dw) {
                            d.damage = healed;
                            if revived {
                                d.specialmessage = 4.0;
                            }
                        }
                    }
                }
            }
            if g.glob.charmove[0] == 0.0 {
                g.glob.charturn = 1.0;
            }
            if g.glob.charturn == 1.0 && g.glob.charmove[1] == 0.0 {
                g.glob.charturn = 2.0;
            }
            if g.glob.charturn == 2.0 && g.glob.charmove[2] == 0.0 {
                skip = true;
            }
            for i in 0..3 {
                g.glob.acting[i] = 0.0;
                g.glob.actingsingle[i] = 0.0;
                g.glob.actingsimul[i] = 0.0;
                g.glob.actingtarget[i] = 0.0;
                g.glob.temptension[i] = g.glob.tension;
                g.glob.charspecial[i] = 0.0;
                g.glob.targeted[i] = 0.0;
                g.glob.charaction[i] = 0.0;
                g.glob.faceaction[i] = 0.0;
                g.glob.monsterattackname[i] = " ".to_string();
            }
            g.glob.currentactingchar = 0.0;
            for id in g.ids_of("obj_monsterparent") {
                if let Some((_, k)) = g.get::<KnightEnemy>(id) {
                    k.attacked = 0.0;
                    k.talked = 0.0;
                    k.acting = 0.0;
                    k.actingsus = 0.0;
                    k.actingral = 0.0;
                }
            }
            if skip {
                self.endturn(g);
            }
            self.init_tempitem(g);
        }
    }

    // ------------------------------------------------------------------ Create

    fn create_impl(&mut self, me: &mut Inst, g: &mut Game) {
        if g.glob.flag(9) == 1.0 {
            let battlemusicvolume = 0.7;
            g.glob.batmusic = g.audio.play_ext("mus_knight", battlemusicvolume, 1.0, true);
        }
        self.victory = 0.0;
        self.victoried = 0.0;
        self.skipvictory = 0.0;
        g.glob.battleend = 0.0;
        self.battlewriter = NOONE;
        self.myface = NOONE;
        self.lastbattlewriter = NOONE;
        self.lbuffer = 0.0;
        self.rbuffer = 0.0;
        self.onebuffer = 0.0;
        self.twobuffer = 0.0;
        self.myfightreturntimer = 1.0;
        self.messagepriority = -1.0;
        self.attackpriority = -1.0;
        self.hidemercy = 0.0;
        self.cantspare[2] = 0.0;
        g.glob.darkzone = 1.0;
        g.glob.fighting = 1.0;
        if g.glob.flag(62) == 0.0 {
            g.glob.fe = 0.0;
            g.glob.fc = 0.0;
            g.glob.typer = 4.0;
            g.glob.battletyper = 4.0;
        }
        g.glob.set_flag(62, 0.0);
        g.glob.myfight = 0.0;
        g.glob.mnfight = 0.0;
        g.glob.bmenuno = 0.0;
        g.glob.attacking = 0.0;
        g.glob.acting = [0.0; 3];
        g.glob.tension = 0.0;
        g.glob.spelldelay = 10.0;
        g.glob.turntimer = 120.0;
        // scr_spellinfo_all(): fills global.spell* tables (spells module, via scr_spellmenu_setup)
        g.glob.tensionselect = 0.0;
        for j in 0..3 {
            g.glob.temptension[j] = g.glob.tension;
        }
        self.init_tempitem(g);
        for i in 0..3 {
            g.glob.charcond[i] = 0.0;
            g.glob.automiss[i] = 0.0;
            if g.glob.char[i] != 0 {
                g.glob.charmove[i] = 1.0;
                g.glob.charcantarget[i] = 1.0;
                g.glob.chardead[i] = 0.0;
            } else {
                g.glob.charmove[i] = 0.0;
                g.glob.charcantarget[i] = 0.0;
            }
        }
        self.itempage = 0.0;
        self.spellpage = 0.0;
        for f in [50, 51, 52, 53, 63] {
            g.glob.set_flag(f, 0.0);
        }
        for i in 0..3 {
            monster_statreset(g, i);
        }
        for i in 0..3 {
            if g.glob.monstertype[i] > 0.0 {
                // scr_monster_makeinstance(i): only the Knight (slot 0) exists here
                g.glob.monster[i] = 1.0;
                let id = knight::create_knight(g, g.glob.monstermakex[i], g.glob.monstermakey[i]);
                g.glob.monsterinstance[i] = id;
                if let Some((_, k)) = g.get::<KnightEnemy>(id) {
                    k.myself = i;
                }
                // obj_knight_enemy Create_0 line 37: `with (obj_battlecontroller) cantspare[0] = 1;`
                // Mirrored here: the controller's slot is checked out during its own Create, so the
                // knight's write can't reach it in this runtime.
                if g.glob.monstertype[i] == 104.0 {
                    self.cantspare[0] = 1.0;
                }
            }
        }
        g.glob.charturn = 0.0;
        g.glob.currentactingchar = 0.0;
        for i in 0..3 {
            g.glob.acting[i] = 0.0;
            g.glob.actingsingle[i] = 0.0;
            g.glob.actingsimul[i] = 0.0;
            g.glob.actingtarget[i] = 0.0;
            g.glob.actingchoice[i] = 0.0;
            g.glob.charaction[i] = 0.0;
            g.glob.charspecial[i] = 0.0;
            g.glob.chartarget[i] = 0.0;
            g.glob.faceaction[i] = 0.0;
            g.glob.rembmenuno[i] = 0.0;
            g.glob.targeted[i] = 0.0;
            let c = g.glob.char[i];
            let (at, df, mag) = equip_bonus(&g.glob, c);
            g.glob.battleat[i] = g.glob.at[c] + at;
            g.glob.battledf[i] = g.glob.df[c] + df;
            g.glob.battlemag[i] = g.glob.mag[c] + mag;
            g.glob.battleactcount[i] = 0.0;
            g.glob.monsterattackname[i] = " ".to_string();
            for j in 0..18 {
                g.glob.battlespell[i][j] = 0.0;
                g.glob.battlespellname[i][j] = " ".to_string();
                g.glob.battlespelldesc[i][j] = " ".to_string();
                g.glob.battlespellcost[i][j] = 0.0;
                g.glob.battlespelltarget[i][j] = 2.0;
                g.glob.battlespellspecial[i][j] = 0.0;
            }
        }
        spells::scr_spellmenu_setup(g);
        g.glob.bmenucoord = [[0.0; 20]; 20];
        self.movenoise = 0.0;
        self.selnoise = 0.0;
        self.laznoise = 0.0;
        self.damagenoise = 0.0;
        self.grazenoise = 0.0;
        self.bcolor = merge_color(C_PURPLE, C_BLACK, 0.7);
        self.bcolor = merge_color(self.bcolor, C_DKGRAY, 0.5);
        self.tp = 0.0;
        self.tpy = 50.0;
        self.bp = 0.0;
        self.bpy = 152.0;
        self.intro = 1.0;
        self.chartotal = 0.0;
        self.charpos = [-1.0; 4];
        self.havechar = [0.0; 4];
        self.mmy = [0.0; 4];
        for i in 0..3 {
            let c = g.glob.char[i];
            if c != 0 {
                self.chartotal += 1.0;
            }
            if (1..=4).contains(&c) {
                self.havechar[c - 1] = 1.0;
                self.charpos[c - 1] = i as f64;
            }
        }
        // the hero instances (obj_herokris/susie/ralsei at heromakex/y, depth 200 - i*20, myself/char set)
        heroes::create_heroes(g);
        self.slmxx = 0.0;
        self.slmyy = 0.0;
        self.s_siner = 0.0;
        self.t_siner = 0.0;
        heroes::create_tensionbar(g);
        self.reset = 0.0;
        self.timeron = 1.0;
        self.noreturn = 0.0;
        self.hpcolor = [C_AQUA, C_FUCHSIA, C_LIME, C_YELLOW];
        for i in 0..4 {
            self.hpcolorsoft[i] = merge_color(self.hpcolor[i], C_WHITE, 0.5);
        }
        g.glob.set_flag(36, 0.0);
        g.glob.set_flag(39, 0.0);
        self.disablesusieact = 0.0;
        self.mercytotal = 0.0;
        self.idefendedthisturn = 0.0;
        self.oopsallacts = 0.0;
        self.rouxlsgridenabled = false;
        let _ = me;
    }

    // ------------------------------------------------------------------ Step

    fn step_impl(&mut self, me: &mut Inst, g: &mut Game) {
        if knight_end_cutscene(g) {
            return;
        }
        if self.victory == 1.0 && self.victoried == 0.0 {
            g.glob.faceaction = [0.0; 3];
            g.glob.battleend = 1.0;
            g.glob.mnfight = -1.0;
            g.glob.myfight = 7.0;
            g.destroy(self.battlewriter);
            g.destroy_all("obj_face");
            g.destroy_all("obj_smallface");
            for i in 0..5 {
                if g.glob.hp[i] < 1.0 {
                    g.glob.hp[i] = crate::gm::round(g.glob.maxhp[i] / 8.0);
                }
            }
            self.lastbattlewriter = NOONE;
            if self.skipvictory == 0.0 {
                // TODO: gold/EXP rewards (global.monstergold/monsterexp are not tracked in this port)
                g.glob.fc = 0.0;
                g.glob.fe = 0.0;
                g.glob.battlemsg[0] = "* You won^1!&* Got 0 EXP and 0 D$./%".to_string();
                g.glob.battletyper = 4.0;
                g.glob.msg[0] = g.glob.battlemsg[0].clone();
                g.glob.typer = g.glob.battletyper;
                self.lastbattlewriter = writer::scr_battletext(g);
                self.battlewriter = self.lastbattlewriter;
                for i in 0..3 {
                    let ci = g.glob.charinstance[i];
                    if let Some((_, h)) = g.get::<Hero>(ci) {
                        h.state = 7.0;
                        h.hurt = 0.0;
                        h.hurttimer = 0.0;
                    }
                }
            }
            self.victoried = 1.0;
            self.victortimer = 0.0;
            if self.skipvictory == 1.0 {
                self.victortimer = -20.0;
            }
            if let Some(tb) = g.first("obj_tensionbar") {
                if let Some(t) = g.inst_mut(tb) {
                    t.alarm[5] = 15;
                    t.set_hspeed(-10.0);
                    t.friction = -0.4;
                }
            }
        }
        if self.victoried == 1.0 {
            self.victortimer += 1.0;
            if !g.id_exists(self.lastbattlewriter) && self.victortimer >= 10.0 {
                self.intro = 2.0;
                if self.bp <= 0.0 {
                    // TODO: scr_endcombat() (leave the battle) — handled by the scene/lead.
                }
            }
        }
        if g.glob.myfight == 0.0 {
            if g.glob.bmenuno == 0.0 {
                self.step_menu_main(g);
            }
            if g.glob.bmenuno == 2.0 && g.glob.flag(34) == 0.0 {
                self.step_menu_tech(g);
            }
            if g.glob.bmenuno == 4.0 {
                self.step_menu_item(g);
            }
            if g.glob.bmenuno == 9.0 {
                self.step_menu_act(g);
            }
            let b = g.glob.bmenuno;
            if b == 7.0 || b == 1.0 || b == 8.0 || b == 3.0 || b == 11.0 || b == 12.0 || b == 13.0 {
                self.step_menu_target(g);
            }
        }
        if self.movenoise == 1.0 {
            g.snd_play("snd_menumove");
            self.movenoise = 0.0;
        }
        if self.grazenoise == 1.0 {
            g.snd_play("snd_graze");
            self.grazenoise = 0.0;
        }
        if self.selnoise == 1.0 {
            g.snd_play("snd_select");
            self.selnoise = 0.0;
        }
        if self.damagenoise == 1.0 {
            g.snd_play("snd_damage");
            self.damagenoise = 0.0;
        }
        if self.laznoise == 1.0 {
            g.snd_play("snd_laz_c");
            self.laznoise = 0.0;
        }
        self.onebuffer -= 1.0;
        self.twobuffer -= 1.0;
        self.lbuffer -= 1.0;
        self.rbuffer -= 1.0;
        if g.glob.mnfight == 2.0 && self.timeron == 1.0 {
            g.glob.turntimer -= 1.0;
            if g.glob.turntimer <= 0.0 && self.reset == 0.0 {
                g.destroy_all("obj_bulletparent");
                g.destroy_all("obj_bulletgenparent");
                for id in g.ids_of("obj_darkener") {
                    if let Some((_, d)) = g.get::<Darkener>(id) {
                        d.darken = 0.0;
                    }
                }
                for id in g.ids_of("obj_heart") {
                    let pos = g.inst(id).map(|h| (h.x, h.y));
                    if let Some((hx, hy)) = pos {
                        // TODO: use the soul module's obj_returnheart once it provides one.
                        g.instance_create(hx, hy, Box::new(ReturnHeartFallback::default()));
                        g.destroy(id);
                    }
                }
                self.reset = 1.0;
                if self.noreturn == 0.0 {
                    me.alarm[2] = 15;
                }
            }
        }
        if g.glob.myfight == 3.0 && Self::monsterpop(g) == 0.0 && !g.exists("obj_writer") {
            self.wincombat(g);
            if g.glob.myfight == 3.0 {
                self.endturn(g);
            }
        }
        self.t_siner += 1.0;
    }

    fn writer_to_back(&mut self, g: &mut Game, skip: bool, depth: f64) {
        let bw = self.battlewriter;
        if skip {
            if let Some((_, w)) = g.get::<Writer>(bw) {
                w.skipme = 1.0;
            }
        }
        if let Some(i) = g.inst_mut(bw) {
            i.depth = depth;
        }
        set_depth_all(g, "obj_face_parent", depth);
        set_depth_all(g, "obj_smallface", depth);
    }

    /// bmenuno 0: FIGHT / ACT(TECH) / ITEM / SPARE / DEFEND
    fn step_menu_main(&mut self, g: &mut Game) {
        let can_input = true;
        if !g.id_exists(self.battlewriter) {
            g.glob.msg[0] = g.glob.battlemsg[0].clone();
            g.glob.typer = g.glob.battletyper;
            self.battlewriter = writer::scr_battletext(g);
        }
        let ct = Self::ct(g).min(2);
        if g.input.pressed(Key::Left) && self.lbuffer < 0.0 && can_input {
            if g.glob.bmenucoord[0][ct] == 0.0 {
                g.glob.bmenucoord[0][ct] = 4.0;
            } else {
                g.glob.bmenucoord[0][ct] -= 1.0;
            }
            self.movenoise = 1.0;
            self.rbuffer = 1.0;
            if self.disablesusieact == 1.0 && ct == 1 && g.glob.bmenucoord[0][ct] == 1.0 {
                g.glob.bmenucoord[0][ct] = 0.0;
            }
        }
        if g.input.pressed(Key::Right) && self.rbuffer < 0.0 && can_input {
            if g.glob.bmenucoord[0][ct] == 4.0 {
                g.glob.bmenucoord[0][ct] = 0.0;
            } else {
                g.glob.bmenucoord[0][ct] += 1.0;
            }
            self.movenoise = 1.0;
            self.lbuffer = 1.0;
            if self.disablesusieact == 1.0 && ct == 1 && g.glob.bmenucoord[0][ct] == 1.0 {
                g.glob.bmenucoord[0][ct] = 2.0;
            }
        }
        if g.input.pressed(Key::B1) && self.twobuffer < 0.0 && can_input {
            self.onebuffer = 1.0;
            self.selnoise = 1.0;
            let coord = g.glob.bmenucoord[0][ct];
            if coord == 0.0 {
                g.glob.bmenuno = 1.0;
            }
            if coord == 1.0 && g.glob.char[ct] != 1 {
                self.onebuffer = 1.0;
                g.glob.bmenuno = 2.0;
            } else if coord == 1.0 {
                self.onebuffer = 1.0;
                g.glob.bmenuno = 11.0;
            }
            if coord == 2.0 && self.tempitem[0][ct] != 0.0 {
                self.onebuffer = 1.0;
                g.glob.bmenuno = 4.0;
                self.iteminfo_temp(ct);
                for _ in 0..12 {
                    let ic = g.glob.bmenucoord[4][ct];
                    if self.tempitem[ix(ic)][ct] == 0.0 && ic > 0.0 {
                        g.glob.bmenucoord[4][ct] -= 1.0;
                    }
                }
            }
            if coord == 3.0 {
                self.onebuffer = 1.0;
                g.glob.bmenuno = 12.0;
            }
            if coord == 4.0 {
                crate::battle::scr_tensionheal(g, 40.0);
                self.idefendedthisturn += 1.0;
                g.glob.faceaction[ct] = 4.0;
                g.glob.charaction[ct] = 10.0;
                self.nexthero(g);
            }
        }
        if g.input.pressed(Key::B2) && self.onebuffer < 0.0 && g.glob.charturn > 0.0 {
            self.twobuffer = 1.0;
            self.movenoise = 1.0;
            self.prevhero(g);
        }
        self.writer_to_back(g, false, 3.0);
    }

    /// bmenuno 2, global.flag[34] == 0: TECH list (ACT entries + spells)
    fn step_menu_tech(&mut self, g: &mut Game) {
        self.writer_to_back(g, true, 10.0);
        let ct = Self::ct(g).min(2);
        self.thischar = ct as f64;
        let tc = ct;
        if g.input.pressed(Key::Right) || g.input.pressed(Key::Left) {
            let mut cango = true;
            let spellcoord = g.glob.bmenucoord[2][ct];
            if spellcoord < 11.0 {
                if g.glob.battlespell[tc][ix(spellcoord + 1.0)] == 0.0 {
                    cango = false;
                    if spellcoord % 2.0 == 1.0 && spellcoord > 0.0 {
                        g.glob.bmenucoord[2][ct] -= 1.0;
                    }
                }
            } else {
                g.glob.bmenucoord[2][ct] -= 1.0;
                cango = false;
            }
            if cango {
                if spellcoord % 2.0 == 0.0 {
                    g.glob.bmenucoord[2][ct] += 1.0;
                } else {
                    g.glob.bmenucoord[2][ct] -= 1.0;
                }
            }
        }
        if g.input.pressed(Key::Down) {
            let spellcoord = g.glob.bmenucoord[2][ct];
            let mut cango = 1;
            if spellcoord >= 10.0 {
                cango = 0;
            } else {
                if g.glob.battlespell[tc][ix(spellcoord + 2.0)] == 0.0 {
                    cango = 0;
                }
                if spellcoord == 5.0 && g.glob.battlespell[tc][6] != 0.0 && g.glob.battlespell[tc][7] == 0.0 {
                    cango = 2;
                }
            }
            if cango == 1 {
                g.glob.bmenucoord[2][ct] += 2.0;
            }
            if cango == 2 {
                g.glob.bmenucoord[2][ct] = 6.0;
            }
        }
        if g.input.pressed(Key::Up) {
            let spellcoord = g.glob.bmenucoord[2][ct];
            if spellcoord > 1.0 {
                g.glob.bmenucoord[2][ct] -= 2.0;
            }
        }
        let coord = ix(g.glob.bmenucoord[2][ct]);
        g.glob.tensionselect = g.glob.battlespellcost[tc][coord];
        if g.input.pressed(Key::B1) && g.glob.battlespell[tc][coord] != 0.0 && self.onebuffer < 0.0 {
            if g.glob.battlespellcost[tc][coord] <= g.glob.tension {
                self.onebuffer = 2.0;
                g.glob.bmenuno = 0.0;
                self.selnoise = 1.0;
                if g.glob.battlespell[tc][coord] != -1.0 {
                    // scr_spellinfo(id).spelltarget == global.battlespelltarget for this entry
                    let spelltarget = g.glob.battlespelltarget[tc][coord];
                    if spelltarget == 0.0 {
                        self.spellconsumeb(g);
                    }
                    if spelltarget == 1.0 {
                        g.glob.bmenuno = 8.0;
                    }
                    if spelltarget == 2.0 {
                        g.glob.bmenuno = 3.0;
                    }
                    if spelltarget == 3.0 {
                        g.glob.bmenuno = 99.0;
                    }
                } else {
                    g.glob.bmenuno = 13.0;
                }
            }
        }
        if g.input.pressed(Key::B2) && self.onebuffer < 0.0 {
            g.glob.tensionselect = 0.0;
            self.twobuffer = 1.0;
            g.glob.bmenuno = 0.0;
            self.movenoise = 1.0;
        }
    }

    /// bmenuno 4: ITEM list
    fn step_menu_item(&mut self, g: &mut Game) {
        self.writer_to_back(g, true, 10.0);
        let ct = Self::ct(g).min(2);
        if self.tempitem[ix(g.glob.bmenucoord[4][ct])][ct] == 0.0 {
            g.glob.bmenucoord[4][ct] -= 1.0;
        }
        if g.input.pressed(Key::Right) {
            let mut cango = true;
            let itemcoord = g.glob.bmenucoord[4][ct];
            if itemcoord < 11.0 {
                if self.tempitem[ix(itemcoord + 1.0)][ct] == 0.0 {
                    cango = false;
                    if itemcoord % 2.0 == 1.0 && itemcoord > 0.0 {
                        g.glob.bmenucoord[4][ct] -= 1.0;
                    }
                }
            } else {
                g.glob.bmenucoord[4][ct] -= 1.0;
                cango = false;
            }
            if cango {
                if itemcoord % 2.0 == 0.0 {
                    g.glob.bmenucoord[4][ct] += 1.0;
                } else {
                    g.glob.bmenucoord[4][ct] -= 1.0;
                }
            }
        }
        if g.input.pressed(Key::Left) {
            let itemcoord = g.glob.bmenucoord[4][ct];
            if self.tempitem[1][ct] != 0.0 {
                if itemcoord % 2.0 == 0.0 {
                    g.glob.bmenucoord[4][ct] += 1.0;
                } else {
                    g.glob.bmenucoord[4][ct] -= 1.0;
                }
            }
        }
        if g.input.pressed(Key::Down) {
            let itemcoord = g.glob.bmenucoord[4][ct];
            let mut cango = 1;
            if itemcoord >= 10.0 {
                cango = 0;
            } else {
                if self.tempitem[ix(itemcoord + 2.0)][ct] == 0.0 {
                    cango = 0;
                }
                if itemcoord == 5.0 && self.tempitem[6][ct] != 0.0 && self.tempitem[7][ct] == 0.0 {
                    cango = 2;
                }
            }
            if cango == 1 {
                g.glob.bmenucoord[4][ct] += 2.0;
            }
            if cango == 2 {
                g.glob.bmenucoord[4][ct] = 6.0;
            }
        }
        if g.input.pressed(Key::Up) {
            let itemcoord = g.glob.bmenucoord[4][ct];
            if itemcoord > 1.0 {
                g.glob.bmenucoord[4][ct] -= 2.0;
            }
        }
        if self.tempitem[ix(g.glob.bmenucoord[4][ct])][ct] == 0.0 {
            g.glob.bmenucoord[4][ct] -= 1.0;
        }
        let coord = ix(g.glob.bmenucoord[4][ct]);
        if g.input.pressed(Key::B1) && self.tempitem[coord][ct] != 0.0 && self.onebuffer < 0.0 {
            self.onebuffer = 2.0;
            g.glob.bmenuno = 0.0;
            self.selnoise = 1.0;
            let item = self.tempitem[coord][ct];
            self.iteminfo(item);
            if self.itemtarget == 0.0 || self.itemtarget == 2.0 {
                let mut tensionhealed = false;
                if item == 27.0 {
                    crate::battle::scr_tensionheal(g, 80.0);
                    tensionhealed = true;
                }
                if item == 28.0 {
                    let a = ceil(g.glob.maxtension / 2.0);
                    crate::battle::scr_tensionheal(g, a);
                    tensionhealed = true;
                }
                if item == 29.0 {
                    let a = ceil(g.glob.maxtension);
                    crate::battle::scr_tensionheal(g, a);
                    tensionhealed = true;
                }
                if tensionhealed {
                    let h = g.snd_play("snd_cardrive");
                    g.audio.pitch(h, 1.4);
                    g.audio.volume(h, 0.8, 0.0);
                    // TODO: obj_healanim (target = hero, particlecolor = c_orange) — spells module
                    // declares no constructor for it.
                    self.itemshift_temp(coord, ct);
                    self.nexthero(g);
                }
                if !tensionhealed {
                    self.itemconsumeb(g);
                }
            }
            if self.itemtarget == 1.0 {
                g.glob.bmenuno = 7.0;
            }
        }
        if g.input.pressed(Key::B2) && self.onebuffer < 0.0 {
            self.twobuffer = 1.0;
            g.glob.bmenuno = 0.0;
            self.movenoise = 1.0;
        }
    }

    /// bmenuno 9: Kris's ACT list
    fn step_menu_act(&mut self, g: &mut Game) {
        let ct = Self::ct(g).min(2);
        self.thisenemy = g.glob.bmenucoord[11][ct];
        let thisenemy = ix(self.thisenemy);
        self.actinfo_temp(g, thisenemy);
        if g.input.pressed(Key::Right) {
            let mut cango = true;
            let actcoord = g.glob.bmenucoord[9][ct];
            if actcoord < 5.0 {
                if self.canact[ix(actcoord + 1.0).min(5)] == 0.0 {
                    cango = false;
                    if actcoord % 2.0 == 1.0 && actcoord > 0.0 {
                        g.glob.bmenucoord[9][ct] -= 1.0;
                    }
                }
            } else {
                g.glob.bmenucoord[9][ct] -= 1.0;
                cango = false;
            }
            if cango {
                if actcoord % 2.0 == 0.0 {
                    g.glob.bmenucoord[9][ct] += 1.0;
                } else {
                    g.glob.bmenucoord[9][ct] -= 1.0;
                }
            }
        }
        if g.input.pressed(Key::Left) {
            let actcoord = g.glob.bmenucoord[9][ct];
            if actcoord % 2.0 == 0.0 {
                if self.canact[ix(actcoord + 1.0).min(5)] != 0.0 {
                    g.glob.bmenucoord[9][ct] += 1.0;
                }
            } else {
                g.glob.bmenucoord[9][ct] -= 1.0;
            }
        }
        if g.input.pressed(Key::Down) {
            let actcoord = g.glob.bmenucoord[9][ct];
            let cango = !(actcoord >= 4.0 || self.canact[ix(actcoord + 2.0).min(5)] == 0.0);
            if cango {
                g.glob.bmenucoord[9][ct] += 2.0;
            }
        }
        if g.input.pressed(Key::Up) {
            let actcoord = g.glob.bmenucoord[9][ct];
            if actcoord > 1.0 {
                g.glob.bmenucoord[9][ct] -= 2.0;
            }
        }
        let coord = ix(g.glob.bmenucoord[9][ct]).min(5);
        g.glob.tensionselect = self.acttpcost[coord];
        let mut canpress = true;
        if g.glob.char[ct] == 1 {
            let actor = g.glob.actactor[coord];
            if (actor == 2.0 || actor == 4.0) && (self.havechar[1] == 0.0 || g.glob.hp[2] <= 0.0) {
                canpress = false;
            }
            if (actor == 3.0 || actor == 4.0) && (self.havechar[2] == 0.0 || g.glob.hp[3] <= 0.0) {
                canpress = false;
            }
            if actor == 5.0 && (self.havechar[3] == 0.0 || g.glob.hp[4] <= 0.0) {
                canpress = false;
            }
        }
        if canpress
            && g.input.pressed(Key::B1)
            && g.glob.canact[coord] == 1.0
            && g.glob.tension >= g.glob.tensionselect
            && self.onebuffer < 0.0
        {
            self.onebuffer = 2.0;
            g.glob.bmenuno = 0.0;
            self.selnoise = 1.0;
            g.glob.actingchoice[ct] = coord as f64;
            g.glob.tension -= self.acttpcost[coord];
            g.glob.tensionselect = 0.0;
            self.actselect(g, thisenemy, coord);
            g.glob.bmenucoord[9][ct] = 0.0;
            self.nexthero(g);
        }
        if g.input.pressed(Key::B2) && self.onebuffer < 0.0 {
            g.glob.bmenucoord[9][ct] = 0.0;
            g.glob.tensionselect = 0.0;
            self.twobuffer = 1.0;
            g.glob.bmenuno = 11.0;
            self.movenoise = 1.0;
        }
    }

    /// bmenuno 1/3/7/8/11/12/13: enemy / party target selection
    fn step_menu_target(&mut self, g: &mut Game) {
        self.writer_to_back(g, true, 10.0);
        let ct = Self::ct(g).min(2);
        if g.input.pressed(Key::B2) && self.onebuffer < 0.0 {
            self.twobuffer = 1.0;
            let b = g.glob.bmenuno;
            if b == 1.0 || b == 11.0 || b == 12.0 {
                g.glob.bmenuno = 0.0;
            }
            if b == 7.0 {
                g.glob.bmenuno = 4.0;
            }
            if b == 8.0 || b == 3.0 || b == 13.0 {
                g.glob.bmenuno = 2.0;
            }
            self.movenoise = 1.0;
        }
        let b = g.glob.bmenuno;
        if !(b == 7.0 || b == 1.0 || b == 8.0 || b == 3.0 || b == 11.0 || b == 12.0 || b == 13.0) {
            return;
        }
        let bm = ix(b);
        let mut ht = [0.0; 3];
        if b == 7.0 || b == 8.0 {
            for i in 0..3 {
                ht[i] = if g.glob.char[i] > 0 { 1.0 } else { 0.0 };
            }
        } else {
            for i in 0..3 {
                ht[i] = g.glob.monster[i];
            }
        }
        let bc = &mut g.glob.bmenucoord[bm][ct];
        if *bc == 2.0 && ht[2] == 0.0 {
            *bc = 0.0;
        }
        if *bc == 0.0 && ht[0] == 0.0 {
            *bc = 1.0;
        }
        if *bc == 1.0 && ht[1] == 0.0 {
            *bc = 0.0;
        }
        if *bc == 0.0 && ht[0] == 0.0 {
            *bc = 2.0;
        }
        if g.input.pressed(Key::Down) {
            let cur = g.glob.bmenucoord[bm][ct];
            let order: [usize; 2] = match cur as i32 {
                0 => [1, 2],
                1 => [2, 0],
                _ => [0, 1],
            };
            if cur == 0.0 || cur == 1.0 || cur == 2.0 {
                for o in order {
                    if ht[o] == 1.0 {
                        self.movenoise = 1.0;
                        g.glob.bmenucoord[bm][ct] = o as f64;
                        break;
                    }
                }
            }
        }
        if g.input.pressed(Key::Up) {
            let cur = g.glob.bmenucoord[bm][ct];
            let order: [usize; 2] = match cur as i32 {
                0 => [2, 1],
                1 => [0, 2],
                _ => [1, 0],
            };
            if cur == 0.0 || cur == 1.0 || cur == 2.0 {
                for o in order {
                    if ht[o] == 1.0 {
                        self.movenoise = 1.0;
                        g.glob.bmenucoord[bm][ct] = o as f64;
                        break;
                    }
                }
            }
        }
        if g.input.pressed(Key::B1) && self.onebuffer < 0.0 {
            self.onebuffer = 1.0;
            self.selnoise = 1.0;
            let sel = g.glob.bmenucoord[bm][ct];
            if g.glob.bmenuno == 1.0 {
                g.glob.chartarget[ct] = sel;
                g.glob.faceaction[ct] = 1.0;
                g.glob.charaction[ct] = 1.0;
                self.nexthero(g);
            }
            if g.glob.bmenuno == 7.0 {
                g.glob.chartarget[ct] = sel;
                self.itemconsumeb(g);
            }
            if g.glob.bmenuno == 8.0 || g.glob.bmenuno == 3.0 {
                g.glob.chartarget[ct] = sel;
                self.spellconsumeb(g);
            }
            if g.glob.bmenuno == 11.0 {
                g.glob.bmenuno = 9.0;
                let actcoord = ix(g.glob.bmenucoord[9][ct]).min(5);
                let c = g.glob.char[ct];
                for _ in 0..6 {
                    let can = match c {
                        1 => g.glob.canact[actcoord],
                        2 => g.glob.canactsus[actcoord],
                        3 => g.glob.canactral[actcoord],
                        _ => 1.0,
                    };
                    if can == 0.0 && actcoord > 0 {
                        g.glob.bmenucoord[9][ct] -= 1.0;
                    }
                }
                self.onebuffer = 1.0;
            }
            if g.glob.bmenuno == 12.0 {
                g.glob.faceaction[ct] = 10.0;
                g.glob.chartarget[ct] = g.glob.bmenucoord[12][ct];
                g.glob.charaction[ct] = 2.0;
                g.glob.charspecial[ct] = 100.0;
                self.nexthero(g);
            }
            if g.glob.bmenuno == 13.0 {
                self.onebuffer = 2.0;
                g.glob.bmenuno = 0.0;
                self.selnoise = 1.0;
                let coord2 = ix(g.glob.bmenucoord[2][ct]);
                g.glob.actingchoice[ct] = coord2 as f64;
                let tc = ix(self.thischar).min(2);
                g.glob.tension -= g.glob.battlespellcost[tc][coord2];
                g.glob.tensionselect = 0.0;
                let enemy = ix(g.glob.bmenucoord[13][ct]);
                self.actinfo_temp(g, enemy);
                self.actselect(g, enemy, coord2.min(5));
                self.nexthero(g);
            }
        }
    }

    // ------------------------------------------------------------------ Draw

    /// scr_selectionmatrix(x, y)
    fn selectionmatrix(&mut self, g: &mut Game, x: f64, y: f64) {
        self.slmxx = x;
        self.slmyy = y;
        self.s_siner += 2.0;
        let (sx, sy) = (self.slmxx, self.slmyy);
        g.gfx.draw_set_color(self.charcolor);
        g.gfx.draw_rectangle(sx, sy, sx + 210.0, sy + 3.0, false);
        for i in 0..12 {
            let myxx = self.s_siner + (i as f64 * (10.0 * PI));
            let s = (myxx / 60.0).sin();
            g.gfx.draw_set_alpha(s);
            g.gfx.draw_line_width(sx, sy - 3.0, sx, sy + 33.0, 2.0);
            g.gfx.draw_line_width(sx + 210.0 + 1.0, sy - 3.0, sx + 210.0 + 1.0, sy + 33.0, 2.0);
            if (myxx / 60.0).cos() < 0.0 {
                let a = (sx - s * 30.0) + 30.0;
                g.gfx.draw_line_width(a, sy, a, sy + 33.0, 2.0);
                let b = (sx + 210.0 + s * 30.0) - 30.0;
                g.gfx.draw_line_width(b, sy, b, sy + 33.0, 2.0);
            }
        }
        g.gfx.draw_set_alpha(1.0);
    }

    /// scr_charbox()
    fn charbox(&mut self, g: &mut Game, xx: f64, yy: f64, bpoff: f64) {
        let bp = self.bp;
        for c in 0..4 {
            if self.havechar[c] != 1.0 {
                continue;
            }
            self.charcolor = self.hpcolor[c];
            let gc = g.glob.charturn;
            let cp = self.charpos[c];
            let mut xchunk = 0.0;
            if self.chartotal == 3.0 {
                if cp == 1.0 {
                    xchunk = 213.0;
                }
                if cp == 2.0 {
                    xchunk = 426.0;
                }
            }
            if cp == 0.0 && self.chartotal == 2.0 {
                xchunk = 108.0;
            }
            if cp == 1.0 && self.chartotal == 2.0 {
                xchunk = 322.0;
            }
            if cp == 0.0 && self.chartotal == 1.0 {
                xchunk = 213.0;
            }
            if gc == cp {
                if self.mmy[c] > -32.0 {
                    self.mmy[c] -= 2.0;
                }
                if self.mmy[c] > -24.0 {
                    self.mmy[c] -= 4.0;
                }
                if self.mmy[c] > -16.0 {
                    self.mmy[c] -= 6.0;
                }
                if self.mmy[c] > -8.0 {
                    self.mmy[c] -= 8.0;
                }
                if self.mmy[c] < -32.0 {
                    self.mmy[c] = -64.0;
                }
            } else if self.mmy[c] < -14.0 {
                self.mmy[c] += 15.0;
            } else {
                self.mmy[c] = 0.0;
            }
            let mmy = self.mmy[c];
            if gc == cp && g.glob.myfight == 0.0 {
                self.selectionmatrix(g, xx + xchunk, (480.0 - bp) + yy);
            }
            let mut btc = [0.0; 5];
            if gc == cp {
                let b = ix(g.glob.bmenucoord[0][ix(g.glob.charturn).min(19)]).min(4);
                btc[b] = 1.0;
            }
            if g.glob.fighting == 1.0 {
                let mut spare_glow = false;
                for i in 0..3 {
                    if g.glob.monster[i] == 1.0 && g.glob.mercymod[i] >= 100.0 {
                        spare_glow = true;
                    }
                }
                let mut pacify_glow = false;
                if c == 2 || c == 3 {
                    let tensionamount = if c == 3 { 80.0 } else { 40.0 };
                    for i in 0..3 {
                        if g.glob.monster[i] == 1.0 && g.glob.monsterstatus[i] == 1.0 && g.glob.tension >= tensionamount {
                            pacify_glow = true;
                        }
                    }
                }
                let icon_offset = 5.0;
                let by = (485.0 - bp) + yy;
                g.draw_sprite(spr("spr_btfight"), btc[0], xx + xchunk + 15.0 + icon_offset, by);
                if c == 0 {
                    g.draw_sprite(spr("spr_btact"), btc[1], xx + xchunk + 50.0 + icon_offset, by);
                } else {
                    // (disablesusieact uses spr_ja_bttech_grey; never set in this fight)
                    g.draw_sprite(spr("spr_bttech"), btc[1], xx + xchunk + 50.0 + icon_offset, by);
                }
                g.draw_sprite(spr("spr_btitem"), btc[2], xx + xchunk + 85.0 + icon_offset, by);
                g.draw_sprite(spr("spr_btspare"), btc[3], xx + xchunk + 120.0 + icon_offset, by);
                g.draw_sprite(spr("spr_btdefend"), btc[4], xx + xchunk + 155.0 + icon_offset, by);
                let glow = 0.4 + ((g.glob.time / 6.0).sin() * 0.4);
                if spare_glow && gc == cp {
                    g.draw_sprite_ext(spr("spr_btspare"), 2.0, xx + xchunk + 120.0 + icon_offset, by, 1.0, 1.0, 0.0, C_WHITE, glow);
                }
                if pacify_glow && gc == cp {
                    g.draw_sprite_ext(spr("spr_bttech"), 2.0, xx + xchunk + 50.0 + icon_offset, by, 1.0, 1.0, 0.0, C_WHITE, glow);
                }
            }
            if gc == cp {
                g.gfx.draw_set_color(self.charcolor);
            } else {
                g.gfx.draw_set_color(self.bcolor);
            }
            if g.glob.charselect == cp || g.glob.charselect == 3.0 {
                g.gfx.draw_set_color(self.charcolor);
            }
            g.gfx.draw_rectangle(xx + xchunk, (480.0 - bp - 3.0) + yy + mmy, xx + xchunk + 212.0, ((480.0 - bp) + yy) - 2.0, false);
            g.gfx.draw_set_color(C_BLACK);
            g.gfx.draw_rectangle(xx + xchunk + 2.0, (480.0 - bp - 1.0) + yy + mmy, xx + xchunk + 210.0, (480.0 - bp) + yy + mmy + 33.0, false);
            let mut b_offset = 480.0;
            if g.glob.fighting == 0.0 {
                b_offset = 430.0;
            }
            if g.glob.fighting == 1.0 {
                b_offset = 336.0;
            }
            let base = bpoff + b_offset;
            let face = g.glob.faceaction[ix(cp).min(2)];
            let (head, name) = match c {
                0 => (spr("spr_headkris"), spr("spr_bnamekris")),
                1 => (spr("spr_headsusie"), spr("spr_bnamesusie")),
                2 => (spr("spr_headralsei"), spr("spr_bnameralsei")),
                _ => (spr("spr_headnoelle"), spr("spr_bnamenoelle")),
            };
            g.draw_sprite(head, face, xx + 13.0 + xchunk, base + mmy);
            g.draw_sprite(name, 0.0, xx + 51.0 + xchunk, base + 3.0 + mmy);
            g.draw_sprite(spr("spr_hpname"), 0.0, xx + 109.0 + xchunk, base + 11.0 + mmy);
            let (hp, maxhp) = (g.glob.hp[c + 1], g.glob.maxhp[c + 1]);
            g.gfx.draw_set_color(C_WHITE);
            if hp / maxhp <= 0.25 {
                g.gfx.draw_set_color(C_YELLOW);
            }
            if hp <= 0.0 {
                g.gfx.draw_set_color(C_RED);
            }
            draw_hpfont_text(g, xx + 160.0 + xchunk, (base - 2.0) + mmy, &gstr(hp), true);
            g.draw_sprite(spr("spr_hpslash"), 0.0, xx + 159.0 + xchunk, (base - 4.0) + mmy);
            draw_hpfont_text(g, xx + 205.0 + xchunk, (base - 2.0) + mmy, &gstr(maxhp), true);
            g.gfx.draw_set_color(C_MAROON);
            g.gfx.draw_rectangle(xx + 128.0 + xchunk, base + 11.0 + mmy, xx + 203.0 + xchunk, base + 19.0 + mmy, false);
            if hp > 0.0 && maxhp > 0.0 {
                g.gfx.draw_set_color(self.charcolor);
                g.gfx.draw_rectangle(xx + 128.0 + xchunk, base + 11.0 + mmy, xx + xchunk + 128.0 + ceil((hp / maxhp) * 75.0), base + 19.0 + mmy, false);
            }
        }
    }

    fn draw_impl(&mut self, g: &mut Game) {
        if knight_end_cutscene(g) {
            return;
        }
        let xx = g.camerax();
        let yy = g.cameray();
        let _tpoff = (self.tp - self.tpy) + yy;
        let bpoff = -self.bp + self.bpy + yy;
        let spell_offset = 500.0;
        if self.intro == 1.0 {
            if self.bp < self.bpy - 1.0 {
                if self.bpy - self.bp < 40.0 {
                    self.bp += crate::gm::round((self.bpy - self.bp) / 2.5);
                } else {
                    self.bp += 30.0;
                }
            } else {
                self.bp = self.bpy;
            }
            if self.bp == self.bpy {
                self.intro = 0.0;
            }
        }
        if self.intro == 2.0 {
            if self.bp > 0.0 {
                if crate::gm::round((self.bpy - self.bp) / 5.0) > 15.0 {
                    self.bp -= crate::gm::round((self.bpy - self.bp) / 2.5);
                } else {
                    self.bp -= 30.0;
                }
            } else {
                self.bp = 0.0;
            }
        }
        let bp = self.bp;
        g.gfx.draw_set_color(C_BLACK);
        g.gfx.draw_rectangle(xx - 10.0, 481.0 + yy, xx + 700.0, ((480.0 - bp) + yy) - 4.0, false);
        g.gfx.draw_set_color(self.bcolor);
        g.gfx.draw_rectangle(xx - 10.0, (480.0 - bp - 3.0) + yy, xx + 700.0, (480.0 - bp - 2.0) + yy, false);
        g.gfx.draw_set_color(self.bcolor);
        g.gfx.draw_rectangle(xx - 10.0, (480.0 - bp) + 34.0 + yy, xx + 700.0, (480.0 - bp) + 36.0 + yy, false);
        self.charbox(g, xx, yy, bpoff);

        let b = g.glob.bmenuno;
        let ct = Self::ct(g).min(2);
        if (b == 1.0 || b == 3.0 || b == 11.0 || b == 12.0 || b == 13.0) && g.glob.myfight == 0.0 {
            self.draw_enemy_targets(g, xx, yy);
        }
        if b == 2.0 && g.glob.myfight == 0.0 && g.glob.flag(34) == 0.0 {
            let tc = ct;
            let mut spellcoord = g.glob.bmenucoord[2][ct];
            let mut page = 0.0;
            if spellcoord > 5.0 {
                page = 1.0;
                spellcoord -= 6.0;
            }
            let (icx, icy) = menu_cursor(spellcoord, 230.0);
            g.draw_sprite(spr("spr_heart"), 0.0, xx + icx, yy + icy);
            g.draw_set_font(font("fnt_mainbig"));
            for i in 0..3 {
                for n in 0..2 {
                    let k = ix(page * 6.0 + (i as f64) * 2.0 + n as f64);
                    g.gfx.draw_set_color(C_WHITE);
                    if g.glob.battlespellspecial[tc][k] >= 1.0 {
                        let c = g.glob.char[tc];
                        g.gfx.draw_set_color(self.hpcolorsoft[c.saturating_sub(1).min(3)]);
                    }
                    if g.glob.tension < g.glob.battlespellcost[tc][k] {
                        g.gfx.draw_set_color(C_GRAY);
                    } else if g.glob.battlespell[tc][k] == 3.0 || g.glob.battlespell[tc][k] == 8.0 {
                        let mut pacify_glow = false;
                        for s in 0..3 {
                            if g.glob.monster[s] == 1.0 && g.glob.monsterstatus[s] == 1.0 {
                                pacify_glow = true;
                            }
                        }
                        if pacify_glow {
                            let aq = merge_color(C_AQUA, C_BLUE, 0.3);
                            let aq2 = merge_color(aq, C_WHITE, 0.5 + ((self.t_siner / 4.0).sin() * 0.5));
                            g.gfx.draw_set_color(aq2);
                        }
                    }
                    let name = g.glob.battlespellname[tc][k].replace('#', "\n");
                    g.draw_text(xx + 30.0 + (n as f64 * 230.0), yy + 375.0 + (i as f64 * 30.0), &name);
                }
            }
            let k = ix(page * 6.0 + spellcoord);
            g.gfx.draw_set_color(C_GRAY);
            let desc = g.glob.battlespelldesc[tc][k].replace('#', "\n");
            g.draw_text(xx + spell_offset, yy + 375.0, &desc);
            let thiscost = floor((g.glob.battlespellcost[tc][k] / g.glob.maxtension) * 100.0);
            g.gfx.draw_set_color(C_ORANGE);
            if thiscost > 0.0 {
                g.draw_text(xx + spell_offset, yy + 440.0, &format!("{}% TP", gstr(thiscost)));
            }
        }
        if b == 4.0 && g.glob.myfight == 0.0 {
            let mut itemcoord = g.glob.bmenucoord[4][ct];
            let mut page = 0.0;
            if itemcoord > 5.0 {
                page = 1.0;
                itemcoord -= 6.0;
            }
            let (icx, icy) = menu_cursor(itemcoord, 230.0);
            g.draw_sprite(spr("spr_heart"), 0.0, xx + icx, yy + icy);
            g.draw_set_font(font("fnt_mainbig"));
            for i in 0..3 {
                let k = ix(page * 6.0 + i as f64 * 2.0);
                let s1 = self.tempitemnameb[k][ct].replace('#', "\n");
                let s2 = self.tempitemnameb[k + 1][ct].replace('#', "\n");
                let w1 = g.string_width(&s1);
                let w2 = g.string_width(&s2);
                let xs1 = if w1 > 0.0 { (200.0 / w1).min(1.0) } else { 1.0 };
                let xs2 = if w2 > 0.0 { (200.0 / w2).min(1.0) } else { 1.0 };
                g.gfx.draw_set_color(C_WHITE);
                g.draw_text_transformed(xx + 30.0, yy + 375.0 + i as f64 * 30.0, &s1, xs1, 1.0, 0.0);
                g.draw_text_transformed(xx + 260.0, yy + 375.0 + i as f64 * 30.0, &s2, xs2, 1.0, 0.0);
            }
            if page == 0.0 && g.glob.item[6] != 0.0 {
                g.draw_sprite(spr("spr_morearrow"), 0.0, xx + 470.0, yy + 445.0 + ((self.s_siner / 10.0).sin() * 2.0));
            }
            if page == 1.0 {
                g.draw_sprite_ext(spr("spr_morearrow"), 0.0, xx + 470.0, (yy + 395.0) - ((self.s_siner / 10.0).sin() * 2.0), 1.0, -1.0, 0.0, C_WHITE, 1.0);
            }
            g.gfx.draw_set_color(C_GRAY);
            let desc = self.tempitemdescb[ix(page * 6.0 + itemcoord).min(11)][ct].replace('#', "\n");
            g.draw_text(xx + spell_offset, yy + 375.0, &desc);
        }
        if b == 9.0 && g.glob.myfight == 0.0 {
            self.draw_act_menu(g, xx, yy);
        }
        if (b == 7.0 || b == 8.0) && g.glob.myfight == 0.0 {
            let sel = g.glob.bmenucoord[ix(b)][ct];
            g.draw_sprite(spr("spr_heart"), 0.0, xx + 55.0, yy + 385.0 + sel * 30.0);
            g.draw_set_font(font("fnt_mainbig"));
            for i in 0..3 {
                if g.glob.char[i] != 0 {
                    let ci = g.glob.charinstance[ix(sel).min(2)];
                    flash_instance(g, ci);
                    let c = g.glob.char[i];
                    g.gfx.draw_set_color(C_WHITE);
                    let nm = g.glob.charname[c].clone();
                    g.draw_text(xx + 80.0, yy + 375.0 + i as f64 * 30.0, &nm);
                    let fy = yy + 380.0 + i as f64 * 30.0;
                    g.gfx.draw_set_color(C_MAROON);
                    g.gfx.draw_rectangle(xx + 400.0, fy, xx + 500.0, fy + 15.0, false);
                    g.gfx.draw_set_color(C_LIME);
                    let mut hp = (g.glob.hp[c] / g.glob.maxhp[c]) * 100.0;
                    if hp <= -100.0 {
                        hp = -100.0;
                    }
                    g.gfx.draw_rectangle(xx + 400.0, fy, xx + 400.0 + hp, fy + 15.0, false);
                }
            }
        }
    }

    fn draw_enemy_targets(&mut self, g: &mut Game, xx: f64, yy: f64) {
        let ct = Self::ct(g).min(2);
        let bm = ix(g.glob.bmenuno);
        let sel = g.glob.bmenucoord[bm][ct];
        g.draw_sprite(spr("spr_heart"), 0.0, xx + 55.0, yy + 385.0 + sel * 30.0);
        g.draw_set_font(font("fnt_mainbig"));
        let mut namewidthmax: f64 = 0.0;
        for i in 0..3 {
            let w = g.string_width(&g.glob.monstername[i].replace('#', "\n"));
            namewidthmax = namewidthmax.max(w);
        }
        for i in 0..3 {
            let mi = g.glob.monsterinstance[ix(sel).min(2)];
            flash_instance(g, mi);
            if g.glob.monster[i] != 1.0 {
                continue;
            }
            let fi = i as f64;
            g.gfx.draw_set_color(C_WHITE);
            let mut mnamecolor1 = C_WHITE;
            let mut mnamecolor2 = C_WHITE;
            let mut aqcolor = merge_color(C_AQUA, C_BLUE, 0.3);
            if g.glob.charturn == 2.0 {
                let a = merge_color(C_AQUA, C_BLUE, 0.3);
                aqcolor = merge_color(a, C_WHITE, 0.5 + ((self.t_siner / 4.0).sin() * 0.5));
            }
            let tireddraw = g.glob.monsterstatus[i] == 1.0;
            let mercydraw = g.glob.mercymod[i] >= g.glob.mercymax[i];
            let mname = g.glob.monstername[i].replace('#', "\n");
            let namewidth = g.string_width(&mname);
            if tireddraw {
                g.gfx.draw_set_color(aqcolor);
                mnamecolor1 = aqcolor;
                mnamecolor2 = aqcolor;
                g.draw_sprite(spr("spr_tiredmark"), 0.0, xx + 80.0 + namewidth + 40.0, yy + 385.0 + fi * 30.0);
            }
            if mercydraw {
                g.gfx.draw_set_color(C_YELLOW);
                mnamecolor1 = C_YELLOW;
                if !tireddraw {
                    mnamecolor2 = C_YELLOW;
                }
                if self.hidemercy == 0.0 {
                    g.draw_sprite(spr("spr_sparestar"), 0.0, xx + 80.0 + namewidth + 20.0, yy + 385.0 + fi * 30.0);
                }
            }
            g.draw_text_color(xx + 80.0, yy + 375.0 + fi * 30.0, &mname, mnamecolor1, mnamecolor2, mnamecolor2, mnamecolor1, 1.0);
            let row = yy + 380.0 + fi * 30.0;
            if g.glob.bmenuno != 13.0 {
                g.gfx.draw_set_color(C_GRAY);
                let cm = g.glob.monstercomment[i].replace('#', "\n");
                g.draw_text(xx + 80.0 + namewidth + 60.0, yy + 375.0 + fi * 30.0, &cm);
                g.gfx.draw_set_color(C_MAROON);
                g.gfx.draw_rectangle(xx + 420.0, row, xx + 500.0, row + 15.0, false);
                g.gfx.draw_set_color(C_LIME);
                let frac = if g.glob.monstermaxhp[i] != 0.0 { g.glob.monsterhp[i] / g.glob.monstermaxhp[i] } else { 0.0 };
                g.gfx.draw_rectangle(xx + 420.0, row, xx + 420.0 + frac * 80.0, row + 15.0, false);
                g.gfx.draw_set_color(C_WHITE);
                g.draw_text_transformed(xx + 424.0, yy + 364.0, "HP", 1.0, 0.5, 0.0);
                if g.exists("obj_knight_enemy") {
                    g.draw_text_transformed(xx + 424.0, row, "???", 1.0, 0.5, 0.0);
                } else {
                    g.draw_text_transformed(xx + 424.0, row, &format!("{}%", gstr(ceil(frac * 100.0))), 1.0, 0.5, 0.0);
                }
            } else {
                let plain = "Standard".to_string();
                let c2 = ix(g.glob.bmenucoord[2][ct]).min(5);
                let mut actname = plain.clone();
                // (per-monster act names: only monster slot 0 exists)
                match g.glob.char[ct] {
                    2 => actname = g.glob.actnamesus[c2].clone(),
                    3 => actname = g.glob.actnameral[c2].clone(),
                    _ => {}
                }
                if actname == "S-Action" || actname == "R-Action" || actname == "N-Action" {
                    actname = plain;
                }
                let c = g.glob.char[ct];
                g.gfx.draw_set_color(self.hpcolorsoft[c.saturating_sub(1).min(3)]);
                // draw_text_width(x, y, str, width): squash to fit
                let s = actname.replace('#', "\n");
                let maxw = 514.0 - (80.0 + namewidthmax + 60.0);
                let w = g.string_width(&s);
                let xs = if w > maxw && w > 0.0 { maxw / w } else { 1.0 };
                g.draw_text_transformed(xx + 80.0 + namewidthmax + 60.0, yy + 375.0 + fi * 30.0, &s, xs, 1.0, 0.0);
            }
            let mercyamt = g.glob.mercymod[i].min(100.0);
            if self.hidemercy == 0.0 {
                let mut mercypercent = ceil((g.glob.mercymod[i] / g.glob.mercymax[i]) * 100.0);
                if mercypercent > 100.0 {
                    mercypercent = 100.0;
                }
                g.gfx.draw_set_color(merge_color(C_ORANGE, C_RED, 0.5));
                g.gfx.draw_rectangle(xx + 520.0, row, xx + 600.0, row + 15.0, false);
                g.gfx.draw_set_color(C_YELLOW);
                if mercyamt > 0.0 && self.cantspare[i] == 0.0 {
                    g.gfx.draw_rectangle(xx + 520.0, row, xx + 520.0 + mercypercent * 0.8, row + 15.0, false);
                }
                g.gfx.draw_set_color(C_WHITE);
                g.draw_text_transformed(xx + 524.0, yy + 364.0, "MERCY", 1.0, 0.5, 0.0);
                g.gfx.draw_set_color(C_MAROON);
                if self.cantspare[i] == 0.0 {
                    g.draw_text_transformed(xx + 524.0, row, &format!("{}%", gstr(mercypercent)), 1.0, 0.5, 0.0);
                }
                if self.cantspare[i] == 1.0 {
                    g.gfx.draw_line_width_color((xx + 520.0) - 1.0, row, xx + 600.0, row + 15.0, 2.0, C_MAROON, C_MAROON);
                    g.gfx.draw_line_width_color((xx + 520.0) - 1.0, row + 15.0, xx + 600.0, row, 2.0, C_MAROON, C_MAROON);
                }
            }
        }
    }

    fn draw_act_menu(&mut self, g: &mut Game, xx: f64, yy: f64) {
        let ct = Self::ct(g).min(2);
        let actcoord = g.glob.bmenucoord[9][ct];
        let c = g.glob.char[ct];
        let mut actname: [String; 6] = Default::default();
        let mut actdesc: [String; 6] = Default::default();
        for a in 0..6 {
            self.canact[a] = 0.0;
            match c {
                1 => {
                    self.canact[a] = g.glob.canact[a];
                    self.acttpcost[a] = g.glob.actcost[a];
                    self.actsimul[a] = g.glob.actsimul[a];
                    actname[a] = g.glob.actname[a].clone();
                    actdesc[a] = g.glob.actdesc[a].clone();
                }
                2 => {
                    self.canact[a] = g.glob.canactsus[a];
                    self.acttpcost[a] = g.glob.actcostsus[a];
                    self.actsimul[a] = g.glob.actsimulsus[a];
                    actname[a] = g.glob.actnamesus[a].clone();
                    actdesc[a] = g.glob.actdescsus[a].clone();
                }
                3 => {
                    self.canact[a] = g.glob.canactral[a];
                    self.acttpcost[a] = g.glob.actcostral[a];
                    self.actsimul[a] = g.glob.actsimulral[a];
                    actname[a] = g.glob.actnameral[a].clone();
                    actdesc[a] = g.glob.actdescral[a].clone();
                }
                _ => {}
            }
        }
        let (icx, icy) = menu_cursor(actcoord, 230.0);
        g.draw_sprite(spr("spr_heart"), 0.0, xx + icx, yy + icy);
        g.draw_set_font(font("fnt_mainbig"));
        for i in 0..6 {
            let mut cant = false;
            let chartime = if c == 1 { g.glob.actactor[i] } else { 0.0 };
            let mut charoffset = 0.0;
            let xoffset = if i % 2 == 1 { 230.0 } else { 0.0 };
            let yoffset = match i {
                2 | 3 => 30.0,
                4 | 5 => 60.0,
                _ => 0.0,
            };
            let mut susblend = C_WHITE;
            let mut noeblend = C_WHITE;
            if chartime == 2.0 || chartime == 4.0 {
                if self.havechar[1] == 0.0 || g.glob.hp[2] <= 0.0 {
                    susblend = 0x808080;
                    cant = true;
                }
                charoffset = 30.0;
            }
            if chartime == 3.0 || chartime == 4.0 {
                if self.havechar[2] == 0.0 || g.glob.hp[3] <= 0.0 {
                    cant = true;
                }
                charoffset = 30.0;
            }
            if chartime == 5.0 {
                if self.havechar[3] == 0.0 || g.glob.hp[4] <= 0.0 {
                    noeblend = 0x808080;
                    cant = true;
                }
                charoffset = 30.0;
            }
            if g.glob.tension < self.acttpcost[i] {
                cant = true;
            }
            if chartime == 4.0 {
                charoffset *= 2.0;
            }
            g.gfx.draw_set_color(if cant { C_GRAY } else { C_WHITE });
            let (tx, ty) = (xx + 30.0 + xoffset, yy + 375.0 + yoffset);
            // (the GML uses susblend for Ralsei's head too)
            if chartime == 2.0 {
                g.draw_sprite_ext(spr("spr_headsusie"), 0.0, tx, ty, 1.0, 1.0, 0.0, susblend, 1.0);
            }
            if chartime == 3.0 {
                g.draw_sprite_ext(spr("spr_headralsei"), 0.0, tx, ty, 1.0, 1.0, 0.0, susblend, 1.0);
            }
            if chartime == 4.0 {
                g.draw_sprite_ext(spr("spr_headsusie"), 0.0, tx, ty, 1.0, 1.0, 0.0, susblend, 1.0);
                g.draw_sprite_ext(spr("spr_headralsei"), 0.0, xx + 60.0 + xoffset, ty, 1.0, 1.0, 0.0, susblend, 1.0);
            }
            if chartime == 5.0 {
                g.draw_sprite_ext(spr("spr_headnoelle"), 0.0, tx, ty, 1.0, 1.0, 0.0, noeblend, 1.0);
            }
            let s1 = actname[i].replace('#', "\n");
            let w = g.string_width(&s1).max(1.0);
            let xs = ((206.0 - charoffset) / w).clamp(0.5, 1.0);
            g.draw_text_transformed(xx + 30.0 + charoffset + xoffset, ty, &s1, xs, 1.0, 0.0);
        }
        g.gfx.draw_set_color(C_GRAY);
        let ac = ix(actcoord).min(5);
        let d = actdesc[ac].replace('#', "\n");
        g.draw_text(xx + 500.0, yy + 375.0, &d);
        if g.glob.tensionselect > 0.0 {
            let thiscost = crate::gm::round((self.acttpcost[ac] / g.glob.maxtension) * 100.0);
            g.gfx.draw_set_color(C_ORANGE);
            g.draw_text(xx + 500.0, yy + 440.0, &format!("{}% TP", gstr(thiscost)));
        }
        let mi = g.glob.monsterinstance[ix(g.glob.bmenucoord[11][ct]).min(2)];
        flash_instance(g, mi);
    }
}

/// Cursor position of the 2-column menus (TECH / ITEM / ACT).
fn menu_cursor(coord: f64, right_x: f64) -> (f64, f64) {
    let mut icx = 10.0;
    let mut icy = 385.0;
    if coord == 1.0 || coord == 3.0 || coord == 5.0 {
        icx = right_x;
    }
    if coord > 1.0 && coord < 4.0 {
        icy = 415.0;
    }
    if coord > 3.0 {
        icy = 445.0;
    }
    (icx, icy)
}

/// Equipment stat bonuses (global.itemat/itemdf/itemmag) for character `c` with this fight's fixed equipment.
fn equip_bonus(gl: &crate::rt::Glob, c: usize) -> (f64, f64, f64) {
    let (mut at, mut df, mut mag) = (0.0, 0.0, 0.0);
    match gl.charweapon[c] as i32 {
        16 | 17 => at += 4.0,
        18 => {
            at += 3.0;
            mag += 2.0;
        }
        _ => {}
    }
    for a in [gl.chararmor1[c], gl.chararmor2[c]] {
        match a as i32 {
            1 => df += 1.0,
            10 => df += 2.0,
            _ => {}
        }
    }
    (at, df, mag)
}

/// scr_monster_statreset(slot)
fn monster_statreset(g: &mut Game, i: usize) {
    let gl = &mut g.glob;
    gl.monster[i] = 0.0;
    gl.monsterx[i] = 0.0;
    gl.monstery[i] = 0.0;
    gl.monstername[i] = " ".to_string();
    gl.monsterat[i] = 0.0;
    gl.monsterdf[i] = 0.0;
    gl.monsterhp[i] = 0.0;
    gl.monstermaxhp[i] = 0.0;
    gl.monsterinstance[i] = NOONE;
    gl.sparepoint[i] = 0.0;
    gl.hittarget[i] = 0.0;
    gl.mercymod[i] = 0.0;
    gl.mercymax[i] = 0.0;
    gl.monstercomment[i] = " ".to_string();
    gl.monsterattackname[i] = " ".to_string();
    gl.monsterstatus[i] = 0.0;
    // scr_monster_actreset(i)
    if i == 0 {
        gl.canact = [0.0; 6];
        gl.canactsus = [0.0; 6];
        gl.canactral = [0.0; 6];
        gl.actcost = [0.0; 6];
        gl.actcostsus = [0.0; 6];
        gl.actcostral = [0.0; 6];
        gl.actsimul = [0.0; 6];
        gl.actsimulsus = [0.0; 6];
        gl.actsimulral = [0.0; 6];
        gl.actactor = [0.0; 6];
    }
}

impl Object for BattleController {
    fn name(&self) -> &'static str { "obj_battlecontroller" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) { self.create_impl(me, g) }
    fn step(&mut self, me: &mut Inst, g: &mut Game) { self.step_impl(me, g) }
    fn draw(&mut self, _me: &mut Inst, g: &mut Game) { self.draw_impl(g) }
    fn alarm(&mut self, n: usize, _me: &mut Inst, g: &mut Game) {
        if n == 2 {
            self.reset = 0.0;
            self.mnendturn(g);
        }
        // Alarm_11 = board-game death event (not in this fight)
    }
    obj_vars!(intro, bp, noreturn, mercytotal, grazenoise, timeron, reset, victory, hidemercy, disablesusieact);
}

// ============================================================================ public script entry points

/// Run `f` on the obj_battlecontroller instance (scripts executed `with (obj_battlecontroller)`).
/// If the controller doesn't exist (or is mid-event), run on a scratch controller so global effects still happen.
fn with_ctrl<R>(g: &mut Game, f: impl FnOnce(&mut BattleController, &mut Game) -> R) -> R {
    if let Some(id) = g.first("obj_battlecontroller") {
        if g.get::<BattleController>(id).is_some() {
            return g.with_t::<BattleController, R>(id, |_, c, g| f(c, g)).unwrap();
        }
    }
    crate::log("controller script called without an accessible obj_battlecontroller");
    let mut tmp = BattleController::default();
    tmp.init_tempitem(g);
    f(&mut tmp, g)
}

/// Set up the Roaring Knight battle (party stats, inventory, encounter 115, heroes, knight, TP bar, controller).
pub fn start_battle(g: &mut Game) {
    // ---- party (scr_gamestart / chapter 3 defaults)
    let gl = &mut g.glob;
    gl.char = [1, 2, 3];
    gl.charname[1] = "Kris".to_string();
    gl.charname[2] = "Susie".to_string();
    gl.charname[3] = "Ralsei".to_string();
    gl.maxhp[1] = 160.0;
    gl.maxhp[2] = 190.0;
    gl.maxhp[3] = 140.0;
    for c in 1..4 {
        gl.hp[c] = gl.maxhp[c];
    }
    gl.at[1] = 14.0;
    gl.df[1] = 2.0;
    gl.mag[1] = 0.0;
    gl.at[2] = 18.0;
    gl.df[2] = 2.0;
    gl.mag[2] = 2.0;
    gl.at[3] = 12.0;
    gl.df[3] = 2.0;
    gl.mag[3] = 11.0;
    gl.charweapon[1] = 16.0;
    gl.charweapon[2] = 17.0;
    gl.charweapon[3] = 18.0;
    for c in 1..4 {
        gl.chararmor1[c] = 1.0;
        gl.chararmor2[c] = 10.0;
    }
    gl.spell = [[0.0; 12]; 5];
    gl.spell[1][0] = 7.0;
    gl.spell[2][0] = 4.0;
    gl.spell[2][1] = 11.0;
    gl.spell[3][0] = 3.0;
    gl.spell[3][1] = 2.0;
    gl.item = [0.0; 13];
    let inv = [2.0, 8.0, 16.0, 11.0, 34.0, 38.0, 27.0, 39.0, 24.0, 23.0, 37.0, 22.0];
    gl.item[..12].copy_from_slice(&inv);
    gl.tension = 0.0;
    gl.maxtension = 250.0;
    gl.set_flag(34, 0.0);
    gl.set_flag(14, 0.0);
    // ---- scr_battle(115, ...)
    gl.encounterno = 115.0;
    if gl.flag(9) != 2.0 {
        gl.set_flag(9, 1.0);
    }
    // ---- scr_encountersetup(115)
    let (xx, yy) = (g.camerax(), g.cameray());
    let gl = &mut g.glob;
    gl.heromakex = [xx + 126.0, xx + 80.0, xx + 58.0];
    gl.heromakey = [yy + 104.0, yy + 142.0, yy + 190.0];
    gl.monstertype = [104.0, 0.0, 0.0];
    gl.monstermakex[0] = xx + 425.0;
    gl.monstermakey[0] = yy + 78.0;
    gl.battlemsg[0] = "* The Roaring Knight appeared.".to_string();
    // ---- obj_battlecontroller (its Create creates the knight, the heroes and the TP bar)
    g.instance_create(0.0, 0.0, Box::new(BattleController::default()));
}

/// scr_attackphase()
pub fn scr_attackphase(g: &mut Game) { with_ctrl(g, |c, g| c.attackphase(g)) }
/// scr_nextact()
pub fn scr_nextact(g: &mut Game) { with_ctrl(g, |c, g| c.nextact(g)) }
/// scr_act_simul()
pub fn scr_act_simul(g: &mut Game) { with_ctrl(g, |c, g| c.act_simul(g)) }
/// scr_endturn()
pub fn scr_endturn(g: &mut Game) { with_ctrl(g, |c, g| c.endturn(g)) }
/// scr_nexthero()
pub fn scr_nexthero(g: &mut Game) { with_ctrl(g, |c, g| c.nexthero(g)) }
/// scr_prevhero()
pub fn scr_prevhero(g: &mut Game) { with_ctrl(g, |c, g| c.prevhero(g)) }
/// scr_mnendturn()
pub fn scr_mnendturn(g: &mut Game) { with_ctrl(g, |c, g| c.mnendturn(g)) }
/// scr_charcan(slot)
pub fn scr_charcan(g: &Game, slot: usize) -> bool { BattleController::charcan(g, slot) }

/// scr_battlecursor_memory_reset()
pub fn scr_battlecursor_memory_reset(g: &mut Game) {
    if g.glob.flag(14) == 0.0 {
        g.glob.bmenucoord = [[0.0; 20]; 20];
    }
}

/// scr_randomtarget(): returns `mytarget` (0-2, 3 = all, 4 = random-living in ch2+), sets global.targeted.
pub fn scr_randomtarget(g: &mut Game) -> f64 {
    let abletotarget = !(g.glob.charcantarget[0] == 0.0 && g.glob.charcantarget[1] == 0.0 && g.glob.charcantarget[2] == 0.0);
    let mut mytarget: f64 = g.choose(&[0.0, 1.0, 2.0]);
    if abletotarget {
        while g.glob.charcantarget[mytarget as usize] == 0.0 {
            mytarget = g.choose(&[0.0, 1.0, 2.0]);
        }
    } else {
        mytarget = 3.0;
    }
    if mytarget < 3.0 {
        g.glob.targeted[mytarget as usize] = 1.0;
    }
    // global.chapter >= 2
    if mytarget != 3.0 {
        for i in 0..3 {
            if g.glob.charcantarget[i] != 0.0 {
                g.glob.targeted[i] = 1.0;
            }
        }
        mytarget = 4.0;
    }
    mytarget
}
