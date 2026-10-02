//! TECH & ITEM logic: scr_spellmenu_setup, scr_spellinfo, scr_spellconsumeb, scr_spelltext, scr_spell
//! (Rude Buster, Heal Prayer, Pacify, UltraHeal, Spare, battle items), obj_spellphase, spell effect objects
//! (obj_rudebuster_anim, obj_rudebuster_bolt, obj_healanim, obj_oflash, obj_pacifyspell, obj_afterimage_grow),
//! scr_iteminfo / scr_iteminfo_temp / scr_itemconsumeb / scr_itemremove / scr_itemshift(_temp), scr_heal & co.
//! OWNER: battle-core agent "spells".

use crate::assets::{spr, Spr, NO_SPR};
use crate::battle::controller;
use crate::battle::heroes::{self, DmgWriter, Hero};
use crate::battle::writer::{msgnext, msgset, scr_battletext_default};
use crate::battle::KnightEnemy;
use crate::gfx::{merge_color, Color, C_BLACK, C_BLUE, C_FUCHSIA, C_LIME, C_ORANGE, C_RED, C_WHITE, C_YELLOW};
use crate::gm::*;
use crate::input::Key;
use crate::obj_vars;
use crate::rt::{Afterimage, Game, Id, Inst, Object, NOONE};

/// The fight is chapter 3.
const CHAPTER: f64 = 3.0;

fn sub1(s: &str, a: &str) -> String { s.replace("~1", a) }
fn sub2(s: &str, a: &str, b: &str) -> String { s.replace("~1", a).replace("~2", b) }

/// global.charname[global.char[slot]]
fn slot_name(g: &Game, slot: usize) -> String { g.glob.charname[g.glob.char[slot.min(2)]].clone() }

/// global.monster[i] with GML's out-of-range star (3) treated as "no monster".
fn monster(g: &Game, i: usize) -> f64 { if i < 3 { g.glob.monster[i] } else { 0.0 } }

/// scr_monsterpop()
pub fn scr_monsterpop(g: &Game) -> f64 { g.glob.monster[0] + g.glob.monster[1] + g.glob.monster[2] }

/// scr_havechar(c)
fn scr_havechar(g: &Game, c: usize) -> bool { g.glob.char.iter().any(|&x| x == c) }

// ============================================================================ spell info

/// Result of scr_spellinfo(id) (the instance variables it sets).
#[derive(Clone, Debug)]
pub struct SpellInfo {
    pub spellname: String,
    pub spellnameb: String,
    pub spelldescb: String,
    pub spelldesc: String,
    pub spelltarget: f64,
    pub cost: f64,
    pub spellanim: f64,
    pub spellusable: f64,
}

/// scr_spellinfo(id) — the spells reachable in this fight: 0 (none), 2 Heal Prayer, 3 Pacify, 4 Rude Buster,
/// 7 ACT, 11 UltraHeal. Other ids return the script's defaults.
pub fn scr_spellinfo(g: &Game, id: f64) -> SpellInfo {
    let mut s = SpellInfo {
        spellname: " ".into(),
        spellnameb: " ".into(),
        spelldescb: " ".into(),
        spelldesc: " ".into(),
        spelltarget: 1.0,
        cost: -1.0,
        spellanim: 0.0,
        spellusable: 0.0,
    };
    let set = |s: &mut SpellInfo, name: &str, nameb: &str, descb: &str, desc: &str, target: f64, cost: f64| {
        s.spellname = name.into();
        s.spellnameb = nameb.into();
        s.spelldescb = descb.into();
        s.spelldesc = desc.into();
        s.spelltarget = target;
        s.cost = cost;
        s.spellusable = 0.0;
    };
    match id as i32 {
        0 => {
            s.spellname = " ".into();
            s.spellnameb = " ".into();
            s.spelltarget = 0.0;
            s.cost = -1.0;
            s.spelldescb = "None".into();
        }
        2 => set(&mut s, "Heal Prayer", "Heal Prayer", "Heal#Ally", "Heavenly light restores a little HP to#one party member. Depends on Magic.", 1.0, 80.0),
        3 => set(&mut s, "Pacify", "Pacify", "Spare#TIRED foe", "SPARE a tired enemy by putting them to sleep.", 2.0, 40.0),
        4 => {
            let cost = if g.glob.charweapon[2] == 7.0 { 100.0 } else { 125.0 };
            set(&mut s, "Rude Buster", "Rude Buster", "Rude#Damage#", "Deals moderate Rude-elemental damage to#one foe. Depends on Attack & Magic.", 2.0, cost);
        }
        7 => {
            // chapter 3 description
            set(&mut s, "ACT", "ACT", "Use#action", "Many different skills.#It has nothing to do with magic.", 0.0, 0.0);
        }
        11 => {
            let cost = 225.0 - round(g.glob.flag(1045) * 2.5);
            set(&mut s, "UltraHeal", "UltraHeal", "Best#healing", "An awesome healing spell.#... right?", 1.0, cost);
        }
        _ => {}
    }
    s
}

/// scr_spellmenu_setup(): rebuild global.battlespell* for every party slot (ACTs first, then the spells).
pub fn scr_spellmenu_setup(g: &mut Game) {
    let mut actnamecheck = false;
    let monstertype = g.glob.monstertype[0];
    for i in 1..3 {
        if g.glob.monster[i] == 1.0 && g.glob.monstertype[i] != monstertype {
            actnamecheck = true;
        }
    }
    let gl = &mut g.glob;
    for i in 0..3 {
        for fj in 0..6 {
            gl.battlespell[i][fj] = 0.0;
            let c = gl.char[i];
            if c == 1 && gl.canact[fj] == 1.0 {
                gl.battlespell[i][fj] = -1.0;
                if gl.battleactcount[i] < (fj + 1) as f64 {
                    gl.battleactcount[i] = (fj + 1) as f64;
                }
                gl.battlespellcost[i][fj] = gl.actcost[fj];
                gl.battlespellname[i][fj] = gl.actname[fj].clone();
                gl.battlespelldesc[i][fj] = gl.actdesc[fj].clone();
                gl.battlespelltarget[i][fj] = 0.0;
                gl.battlespellspecial[i][fj] = 1.0;
            }
            if c == 2 && gl.canactsus[fj] == 1.0 {
                gl.battlespell[i][fj] = -1.0;
                if gl.battleactcount[i] < (fj + 1) as f64 {
                    gl.battleactcount[i] = (fj + 1) as f64;
                }
                gl.battlespellcost[i][fj] = gl.actcostsus[fj];
                gl.battlespellname[i][fj] = gl.actnamesus[fj].clone();
                if actnamecheck {
                    gl.battlespellname[i][fj] = "S-Action".into();
                }
                gl.battlespelldesc[i][fj] = gl.actdescsus[fj].clone();
                gl.battlespelltarget[i][fj] = 2.0;
                gl.battlespellspecial[i][fj] = 2.0;
            }
            if c == 3 && gl.canactral[fj] == 1.0 {
                gl.battlespell[i][fj] = -1.0;
                if gl.battleactcount[i] < (fj + 1) as f64 {
                    gl.battleactcount[i] = (fj + 1) as f64;
                }
                gl.battlespellcost[i][fj] = gl.actcostral[fj];
                gl.battlespellname[i][fj] = gl.actnameral[fj].clone();
                if actnamecheck {
                    gl.battlespellname[i][fj] = "R-Action".into();
                }
                gl.battlespelldesc[i][fj] = gl.actdescral[fj].clone();
                gl.battlespelltarget[i][fj] = 2.0;
                gl.battlespellspecial[i][fj] = 3.0;
            }
        }
    }
    // scr_spellinfo_all() + copy into the battle menu after the ACT entries
    for i in 0..3 {
        let c = g.glob.char[i];
        for fj in 0..12 {
            let ib = g.glob.battleactcount[i] as usize + fj;
            let info = scr_spellinfo(g, g.glob.spell[c][fj]);
            if ib >= 18 {
                continue;
            }
            let gl = &mut g.glob;
            gl.battlespell[i][ib] = gl.spell[c][fj];
            gl.battlespellcost[i][ib] = info.cost;
            gl.battlespellname[i][ib] = info.spellnameb;
            gl.battlespelldesc[i][ib] = info.spelldescb;
            gl.battlespelltarget[i][ib] = info.spelltarget;
        }
    }
}

/// scr_spellconsumeb(): `info` is the scr_spellinfo() of the chosen spell (GML reads `cost`/`spellanim`
/// left on the controller by its earlier scr_spellinfo call).
pub fn scr_spellconsumeb(g: &mut Game, info: &SpellInfo) {
    g.glob.tension -= floor(floor((info.cost / g.glob.maxtension) * 100.0) * 2.5);
    let ct = g.glob.charturn as usize;
    g.glob.faceaction[ct] = 2.0;
    g.glob.charaction[ct] = 2.0;
    let coord = g.glob.bmenucoord[2][ct] as usize;
    if g.glob.flag(34) == 1.0 {
        g.glob.charspecial[ct] = g.glob.spell[g.glob.char[ct]][coord.min(11)];
    }
    if g.glob.flag(34) == 0.0 {
        g.glob.charspecial[ct] = g.glob.battlespell[ct][coord.min(17)];
    }
    g.glob.tensionselect = 0.0;
    let hid = g.glob.charinstance[ct];
    if info.spellanim == 1.0 {
        g.set_var(hid, "spellframes", 0.0);
        if let Some((_, h)) = g.get::<Hero>(hid) {
            h.spellsprite = h.spellreadysprite;
        }
    } else if let Some(v) = g.get_var(hid, "remspellframes") {
        // spellframes = remspellframes; spellsprite = remspellsprite (only when the hero exposes them)
        g.set_var(hid, "spellframes", v);
    }
    controller::scr_nexthero(g);
}

// ============================================================================ spell text

/// scr_retarget_spell(): returns cancelattack.
fn scr_retarget_spell(g: &Game, star: &mut usize) -> bool {
    let mut cancelattack = false;
    if *star == 0 && g.glob.monster[0] == 0.0 {
        *star = 1;
    }
    if *star == 1 && g.glob.monster[1] == 0.0 {
        *star = 2;
    }
    if *star == 2 {
        if g.glob.monster[2] == 0.0 {
            *star = 3;
        }
        if *star == 3 && g.glob.monster[0] == 1.0 {
            *star = 0;
        }
        if *star == 3 && g.glob.monster[1] == 1.0 {
            *star = 1;
        }
        if *star == 3 {
            cancelattack = true;
        }
    }
    cancelattack
}

/// scr_spelltext(spell, caster): sets global.msg for the battle text of a spell/item/spare.
pub fn scr_spelltext(g: &mut Game, spell: f64, caster: usize) {
    let mut star = g.glob.chartarget[caster.min(2)].max(0.0) as usize;
    let name = slot_name(g, caster);
    let n = name.as_str();
    let item = |g: &mut Game, s: &str| msgset(g, 0, &sub1(s, n));
    match spell as i32 {
        1 => item(g, "* ~1 cast RUDE BUSTER!/%"),
        2 => item(g, "* ~1 cast HEAL PRAYER!/%"),
        3 => {
            item(g, "* ~1 cast PACIFY!/%");
            scr_retarget_spell(g, &mut star);
            if monster(g, star) == 1.0 && g.glob.monsterstatus[star] != 1.0 {
                item(g, "* ~1 cast PACIFY^1!&* But the enemy wasn't \\cBTIRED\\cW.../%");
                if g.glob.mercymod[star] >= 100.0 {
                    item(g, "* ~1 cast PACIFY^1!&* But the foe wasn't \\cBTIRED\\cW... try \\cYSPARING\\cW!/%");
                }
            }
        }
        4 => item(g, "* ~1 used RUDE BUSTER!/%"),
        5 => item(g, "* ~1 used RED BUSTER!/%"),
        6 => item(g, "* ~1 cast DUAL HEAL!/%"),
        8 => item(g, "* ~1 cast SLEEPMIST!/%"),
        9 => item(g, "* ~1 cast ICESHOCK!/%"),
        10 => item(g, "* ~1 cast SNOWGRAVE!/%"),
        11 => item(g, "* ~1 cast ULTRAHEAL!/%"),
        100 => {
            let cancelattack = false;
            let mname = g.glob.monstername[star.min(2)].clone();
            msgset(g, 0, &sub2("* ~1 spared ~2!/%", n, &mname));
            scr_retarget_spell(g, &mut star);
            let s = star.min(2);
            let mname = g.glob.monstername[s].clone();
            if g.glob.mercymod[s] >= 100.0 {
                msgset(g, 0, &sub2("* ~1 spared ~2!/%", n, &mname));
            } else {
                msgset(g, 0, &sub2("* ~1 spared ~2^2!&* But its name wasn't \\cYYELLOW\\cW.../%", n, &mname));
                if g.glob.monsterstatus[s] == 1.0 {
                    if scr_havechar(g, 3) {
                        msgset(g, 0, &sub2("* ~1 spared ~2^2!&* But its name wasn't \\cYYELLOW\\cW.../", n, &mname));
                        msgset(g, 1, "* (Try using Ralsei's \\cBPACIFY\\cW!)/%");
                    } else if scr_havechar(g, 4) {
                        msgset(g, 0, &sub2("* ~1 spared ~2^2!&* But its name wasn't \\cYYELLOW\\cW.../", n, &mname));
                        msgnext(g, "* (Try using Noelle's \\cBSLEEPMIST\\cW!)/%");
                    } else {
                        msgset(g, 0, &sub2("* ~1 spared ~2^2!&* But its name wasn't \\cYYELLOW\\cW.../", n, &mname));
                        msgnext(g, "* (Try using \\cBACTs\\cW!)/%");
                    }
                }
            }
            if cancelattack {
                item(g, "* ~1 spared!/%");
            }
        }
        201 => item(g, "* ~1 used the DARK CANDY!/%"),
        202 => item(g, "* ~1 used the REVIVEMINT!/%"),
        205 => item(g, "* ~1 used the BROKEN CAKE!/%"),
        206 => item(g, "* ~1 used the TOPCAKE!/%"),
        207 => item(g, "* ~1 used the SPINCAKE!/%"),
        208 => item(g, "* ~1 used the DARKBURGER!/%"),
        209 => item(g, "* ~1 used the LANCERCOOKIE!/%"),
        210 => item(g, "* ~1 used the GIGASALAD!/%"),
        211 => item(g, "* ~1 used the CLUBS SANDWICH!/%"),
        212 => item(g, "* ~1 used the HEARTS DONUT!/%"),
        213 => item(g, "* ~1 used the CHOCO DIAMOND!/%"),
        214 => item(g, "* ~1 used the FAV SANDWICH!/%"),
        215 => item(g, "* ~1 used the ROUXLS ROUX!/%"),
        216 => item(g, "* ~1 used the CD BAGEL!/%"),
        217 => {
            item(g, "* ~1 used the CLOTHESDOLL!/");
            msgnext(g, "* ... but nothing happened!/%");
        }
        218..=221 => item(g, "* ~1 used the ROTTEN TEA!/%"),
        222 => item(g, "* ~1 used the DD-BURGER!/%"),
        223 => item(g, "* ~1 used the LIGHTCANDY!/%"),
        224 => item(g, "* ~1 used the BUTJUICE!/%"),
        225 => item(g, "* ~1 used the SPAGHETTICODE!/%"),
        226 => item(g, "* ~1 used the JAVACOOKIE!/%"),
        227 => item(g, "* ~1 used the TENSIONBIT!&* Tension raised up earlier./%"),
        228 => item(g, "* ~1 used the TENSIONGEM!/%"),
        229 => item(g, "* ~1 used the TENSIONMAX!/%"),
        230 => item(g, "* ~1 used the REVIVEDUST!/%"),
        231 => item(g, "* ~1 used the REVIVEBRIGHT!/%"),
        232 => item(g, "* ~1 administered S.POISON!/%"),
        233 => item(g, "* ~1 admired DOGDOLLAR!/%"),
        234 => item(g, "* ~1 used the TVDINNER!/%"),
        235 => item(g, "* ~1 used the PIPIS!/%"),
        236 => item(g, "* ~1 used the FLATSODA!/%"),
        237 => item(g, "* ~1 used the TVSLOP!/%"),
        238 => item(g, "* ~1 used the EXECBUFFET!/%"),
        239 => item(g, "* ~1 used the DELUXEDINNER!/%"),
        // 203 GLOWSHARD / 204 MANUAL only matter for other encounters' monster types; plain text here.
        203 => {
            item(g, "* ~1 used the GLOWSHARD!/");
            msgset(g, 1, "* But nothing happened.../%");
        }
        204 => {
            item(g, "* ~1 read the MANUAL!/");
            msgset(g, 1, "* But nothing happened.../%");
        }
        _ => {}
    }
}

// ============================================================================ healing

/// scr_revive(slot)
pub fn scr_revive(g: &mut Game, slot: usize) {
    g.glob.charmove[slot] = 1.0;
    g.glob.charcantarget[slot] = 1.0;
    g.glob.chardead[slot] = 0.0;
}

/// scr_heal(slot, amount) → amount actually healed.
pub fn scr_heal(g: &mut Game, slot: usize, amt: f64) -> f64 {
    let mut abovemaxhp = false;
    let mut belowzero = false;
    let t = g.glob.char[slot];
    let curhp = g.glob.hp[t];
    if g.glob.hp[t] <= 0.0 {
        belowzero = true;
    }
    if g.glob.hp[t] > g.glob.maxhp[t] {
        abovemaxhp = true;
    }
    if !abovemaxhp {
        g.glob.hp[t] += amt;
        if g.glob.hp[t] > g.glob.maxhp[t] {
            g.glob.hp[t] = g.glob.maxhp[t];
        }
    }
    if belowzero && g.glob.hp[t] >= 0.0 {
        if g.glob.hp[t] < ceil(g.glob.maxhp[t] / 6.0) {
            g.glob.hp[t] = ceil(g.glob.maxhp[t] / 6.0);
        }
        scr_revive(g, slot);
    }
    g.snd_stop("snd_power");
    g.snd_play("snd_power");
    g.glob.hp[t] - curhp
}

/// scr_healall(amount)
pub fn scr_healall(g: &mut Game, amt: f64) {
    for i in 0..3 {
        if g.glob.char[i] != 0 {
            scr_heal(g, i, amt);
        }
    }
}

/// scr_heal_amount_modify_by_equipment(amount) for party slot `caster` (armor 26 adds 1/8).
pub fn scr_heal_amount_modify_by_equipment(g: &Game, caster: usize, amt: f64) -> f64 {
    let c = g.glob.char[caster.min(2)];
    let mut add = 0.0;
    if g.glob.chararmor1[c] == 26.0 {
        add += ceil(amt / 8.0);
    }
    if g.glob.chararmor2[c] == 26.0 {
        add += ceil(amt / 8.0);
    }
    amt + add
}

/// The hero instance that is currently executing scr_spell (its slot is checked out of the runtime).
type SelfHero<'a> = Option<(&'a mut Inst, &'a mut Hero)>;

/// (id, x, y, depth, myheight, tu) of the hero in party slot `slot`.
fn hero_info(g: &mut Game, slf: &mut SelfHero, slot: usize) -> Option<(Id, f64, f64, f64, f64, f64)> {
    let id = g.glob.charinstance[slot];
    if let Some((i, h)) = slf.as_mut() {
        if i.id == id {
            return Some((id, i.x, i.y, i.depth, h.myheight, h.tu));
        }
    }
    g.get::<Hero>(id).map(|(i, h)| (id, i.x, i.y, i.depth, h.myheight, h.tu))
}
fn hero_tu_add(g: &mut Game, slf: &mut SelfHero, slot: usize) {
    let id = g.glob.charinstance[slot];
    if let Some((i, h)) = slf.as_mut() {
        if i.id == id {
            h.tu += 1.0;
            return;
        }
    }
    if let Some((_, h)) = g.get::<Hero>(id) {
        h.tu += 1.0;
    }
}

/// `with (global.charinstance[slot]) { ha = obj_healanim; dmgwr = scr_dmgwriter_selfchar(); ...; tu += 1 }`
fn heal_fx(g: &mut Game, slf: &mut SelfHero, slot: usize, amount: f64) -> Id {
    let Some((hid, x, y, _depth, myheight, tu)) = hero_info(g, slf, slot) else { return NOONE };
    let ha = g.instance_create(x, y, Box::new(HealAnim::default()));
    if let Some((_, a)) = g.get::<HealAnim>(ha) {
        a.target = hid;
    }
    // scr_dmgwriter_selfchar()
    let dw = heroes::dmgwriter(g, x, (y + myheight) - 24.0 - (tu * 20.0));
    let c = g.glob.char[slot];
    let full = g.glob.hp[c] >= g.glob.maxhp[c];
    if let Some((_, d)) = g.get::<DmgWriter>(dw) {
        d.delay = 8.0;
        d.typ = 3.0;
        d.damage = amount;
        if full {
            d.specialmessage = 3.0;
        }
    }
    hero_tu_add(g, slf, slot);
    ha
}

/// scr_healitemspell(amount) on `star`. Returns the obj_healanim.
fn scr_healitemspell(g: &mut Game, slf: &mut SelfHero, caster: usize, star: usize, amt: f64) -> Id {
    let heal = scr_heal_amount_modify_by_equipment(g, caster, amt);
    scr_heal(g, star, heal);
    g.glob.spelldelay = 15.0;
    heal_fx(g, slf, star, heal)
}

/// scr_healallitemspell(amount)
fn scr_healallitemspell(g: &mut Game, slf: &mut SelfHero, caster: usize, amt: f64) {
    let heal = scr_heal_amount_modify_by_equipment(g, caster, amt);
    scr_healall(g, heal);
    for i in 0..3 {
        heal_fx(g, slf, i, heal);
    }
    g.glob.spelldelay = 20.0;
}

/// scr_mercyadd(monster, amount)
pub fn scr_mercyadd(g: &mut Game, m: usize, amt: f64) {
    g.glob.mercymod[m] += amt;
    if g.glob.mercymod[m] < 0.0 {
        g.glob.mercymod[m] = 0.0;
    }
    if g.glob.mercymod[m] >= 100.0 {
        g.glob.mercymod[m] = 100.0;
    }
    // (the "another mercy dmgwriter younger than 8 frames" check needs obj_dmgwriter.mercytimer — not exposed)
    if amt > 0.0 {
        let mut pitch = 0.8;
        if amt < 99.0 {
            pitch = 1.0;
        }
        if amt <= 50.0 {
            pitch = 1.2;
        }
        if amt <= 25.0 {
            pitch = 1.4;
        }
        g.snd_play_x("snd_mercyadd", 0.8, pitch);
    }
    let (mx, my, ht) = (g.glob.monsterx[m], g.glob.monstery[m], g.glob.hittarget[m]);
    let dw = heroes::dmgwriter(g, mx, (my + 20.0) - (ht * 20.0));
    if let Some((_, d)) = g.get::<DmgWriter>(dw) {
        d.damage = amt;
        d.typ = 5.0;
    }
    g.glob.hittarget[m] += 1.0;
}

// ============================================================================ scr_spell

/// scr_spell(spell, caster) as called from obj_heroparent's Step (`spelltimer` reaching 0):
/// `me`/`hero` are the calling hero (the caster, whose slot is checked out while its Step runs).
pub fn scr_spell(g: &mut Game, spell: f64, caster: usize, me: &mut Inst, hero: &mut Hero) {
    spell_impl(g, spell, caster, Some((me, hero)));
}

/// scr_spell(spell, caster) when no hero instance is currently executing.
pub fn scr_spell_detached(g: &mut Game, spell: f64, caster: usize) { spell_impl(g, spell, caster, None); }

fn spell_impl(g: &mut Game, spell: f64, caster: usize, mut slf: SelfHero) {
    let mut star = g.glob.chartarget[caster].max(0.0) as usize;
    g.glob.spelldelay = 10.0;
    let slf = &mut slf;
    match spell as i32 {
        2 => {
            // Heal Prayer
            let healnum = scr_heal_amount_modify_by_equipment(g, caster, g.glob.battlemag[caster] * 5.0);
            scr_heal(g, star, healnum);
            heal_fx(g, slf, star, healnum);
            g.glob.spelldelay = 15.0;
        }
        3 => {
            // Pacify
            if monster(g, star) == 0.0 {
                scr_retarget_spell(g, &mut star);
            }
            if monster(g, star) == 1.0 {
                let target = g.glob.monsterinstance[star];
                if g.glob.monsterstatus[star] == 1.0 {
                    let myself = g.get::<KnightEnemy>(target).map(|(_, k)| k.myself).unwrap_or(star);
                    let mt = g.glob.monstertype[myself];
                    if mt != 19.0 && mt != 3.0 && mt != 52.0 && mt != 43.0 {
                        let yoffy = if mt == 33.0 { -60.0 } else { 0.0 };
                        let (mx, my) = (g.glob.monsterx[myself], g.glob.monstery[myself] + yoffy);
                        let p = g.instance_create(mx, my, Box::new(PacifySpell::default()));
                        if let Some((_, ps)) = g.get::<PacifySpell>(p) {
                            ps.con = 20.0;
                            ps.target = target;
                        }
                        g.glob.set_flag(51 + myself as i32, 3.0);
                        g.user_event(target, 10);
                        // TODO: scr_monsterdefeat() — not exposed by battle::heroes / battle::knight.
                    } else {
                        // monster types 19/3/52/43 belong to other encounters (pacifycon on the monster).
                        g.glob.spelldelay = 999.0;
                    }
                } else {
                    let p = g.instance_create(0.0, 0.0, Box::new(PacifySpell::default()));
                    if let Some((_, ps)) = g.get::<PacifySpell>(p) {
                        ps.target = target;
                        ps.fail = 1.0;
                    }
                }
            }
            g.glob.spelldelay = 20.0;
        }
        4 => {
            // Rude Buster
            let mut cancelattack = false;
            g.glob.spelldelay = 30.0;
            if monster(g, star) == 0.0 {
                cancelattack = scr_retarget_spell(g, &mut star);
            }
            if !cancelattack {
                g.glob.spelldelay = 70.0;
                let mut damage = ceil(((g.glob.battlemag[caster] * 5.0) + (g.glob.battleat[caster] * 11.0)) - (g.glob.monsterdf[star] * 3.0));
                if let Some((_, k)) = g.get_first::<KnightEnemy>("obj_knight_enemy") {
                    damage = ceil(damage * (k.damagereduction + 0.65));
                }
                if g.glob.automiss[star] == 1.0 {
                    damage = 0.0;
                }
                // obj_herosusie.x / .y (Susie is normally the caster and thus checked out)
                let mut susie: Option<(f64, f64, f64)> = None;
                if let Some((i, _)) = slf.as_mut() {
                    if i.object == "obj_herosusie" {
                        susie = Some((i.x, i.y, i.depth));
                    }
                }
                let (sx, sy, sdepth) = match susie {
                    Some(s) => s,
                    None => g.first_inst("obj_herosusie").map(|i| (i.x, i.y, i.depth)).unwrap_or((0.0, 0.0, 0.0)),
                };
                let target = g.glob.monsterinstance[star];
                let a = g.instance_create(sx, sy, Box::new(RudebusterAnim::default()));
                if let Some((ai, an)) = g.get::<RudebusterAnim>(a) {
                    an.damage = damage;
                    an.star = star;
                    an.caster = caster as f64;
                    an.target = target;
                    // depth = obj_herosusie.depth (Create couldn't see the checked-out caster)
                    if susie.is_some() {
                        ai.depth = sdepth;
                        an.battlemode = 1.0;
                    }
                }
                if susie.is_some() {
                    if let Some((i, _)) = slf.as_mut() {
                        i.visible = false;
                    }
                }
            }
        }
        11 => {
            // UltraHeal
            g.glob.set_flag(1045, g.glob.flag(1045) + 1.0);
            if g.glob.flag(1045) > 5.0 {
                g.glob.set_flag(1045, 5.0);
            }
            let base = (g.glob.battlemag[caster] * 1.5) + 5.0 + g.glob.flag(1045);
            let healnum = ceil(scr_heal_amount_modify_by_equipment(g, caster, base));
            scr_heal(g, star, healnum);
            heal_fx(g, slf, star, healnum);
            g.glob.spelldelay = 15.0;
            scr_spellmenu_setup(g);
        }
        100 => {
            // SPARE
            if monster(g, star) == 0.0 {
                scr_retarget_spell(g, &mut star);
            }
            if monster(g, star) == 1.0 {
                let target = g.glob.monsterinstance[star];
                if g.glob.mercymod[star] >= 100.0 {
                    if g.glob.monstertype[star] != 3.0 && g.glob.monstertype[star] != 52.0 {
                        let myself = g.get::<KnightEnemy>(target).map(|(_, k)| k.myself).unwrap_or(star);
                        g.glob.set_flag(51 + myself as i32, 2.0);
                        g.user_event(target, 10);
                        // TODO: scr_monsterdefeat() — not exposed by battle::heroes / battle::knight.
                    }
                } else {
                    let sp = g.glob.sparepoint[star];
                    scr_mercyadd(g, star, sp);
                    let p = g.instance_create(0.0, 0.0, Box::new(PacifySpell::default()));
                    if let Some((_, ps)) = g.get::<PacifySpell>(p) {
                        ps.target = target;
                        ps.fail = 1.0;
                        ps.flashcolor = C_YELLOW;
                    }
                }
            }
            g.glob.spelldelay = 0.0;
        }
        201 => {
            scr_healitemspell(g, slf, caster, star, 40.0);
        }
        202 => {
            // ReviveMint
            let c = g.glob.char[star];
            let mut reviveamt = ceil(g.glob.maxhp[c] / 2.0);
            if g.glob.hp[c] <= 0.0 {
                reviveamt = ceil(g.glob.maxhp[c]) + abs(g.glob.hp[c]);
            }
            scr_healitemspell(g, slf, caster, star, reviveamt);
        }
        205 => {
            scr_healitemspell(g, slf, caster, star, 20.0);
        }
        206 => scr_healallitemspell(g, slf, caster, 160.0),
        207 => scr_healallitemspell(g, slf, caster, 150.0),
        208 => {
            scr_healitemspell(g, slf, caster, star, 70.0);
        }
        209 => {
            scr_healitemspell(g, slf, caster, star, 50.0);
        }
        210 => {
            scr_healitemspell(g, slf, caster, star, 4.0);
        }
        211 => scr_healallitemspell(g, slf, caster, 70.0),
        212 | 213 => {
            let amts: [f64; 4] = if spell as i32 == 212 { [20.0, 80.0, 50.0, 30.0] } else { [80.0, 20.0, 50.0, 70.0] };
            let c = g.glob.char[star];
            if (1..=4).contains(&c) {
                scr_healitemspell(g, slf, caster, star, amts[c - 1]);
            }
        }
        214 => {
            scr_healitemspell(g, slf, caster, star, 500.0);
        }
        215 => {
            scr_healitemspell(g, slf, caster, star, 50.0);
        }
        216 => {
            scr_healitemspell(g, slf, caster, star, 80.0);
        }
        218..=221 => {
            scr_healitemspell(g, slf, caster, star, 10.0);
        }
        222 => {
            scr_healitemspell(g, slf, caster, star, 60.0);
        }
        223 => {
            scr_healitemspell(g, slf, caster, star, 120.0);
        }
        224 => {
            scr_healitemspell(g, slf, caster, star, 100.0);
        }
        225 => scr_healallitemspell(g, slf, caster, 30.0),
        226 => {
            let amt = if g.glob.char[star] == 1 { 100.0 } else { 90.0 };
            scr_healitemspell(g, slf, caster, star, amt);
        }
        230 | 231 => {
            for j in 0..3 {
                if g.glob.char[j] > 0 {
                    let c = g.glob.char[j];
                    let mut amt = if spell as i32 == 230 { 10.0 } else { 50.0 };
                    if g.glob.hp[c] <= 0.0 {
                        amt = if spell as i32 == 230 { ceil(g.glob.maxhp[c] / 4.0) + abs(g.glob.hp[c]) } else { 999.0 };
                    }
                    scr_healitemspell(g, slf, caster, j, amt);
                }
            }
        }
        232 => {
            // S.POISON: (poisonamount on the hero is not modelled by battle::heroes)
            g.snd_play("snd_hurt1");
            let ha = scr_healitemspell(g, slf, caster, star, 40.0);
            if let Some((_, a)) = g.get::<HealAnim>(ha) {
                a.particlecolor = C_FUCHSIA;
            }
        }
        234 => {
            scr_healitemspell(g, slf, caster, star, 100.0);
        }
        236 => {
            scr_healitemspell(g, slf, caster, star, 20.0);
        }
        237 => {
            scr_healitemspell(g, slf, caster, star, 80.0);
        }
        238 => scr_healallitemspell(g, slf, caster, 100.0),
        239 => {
            scr_healitemspell(g, slf, caster, star, 140.0);
        }
        // 0, 200, 203, 204, 217, 227 (TensionBit acts at selection), 228, 229, 233, 235: nothing.
        _ => {}
    }
}

// ============================================================================ obj_spellphase

/// obj_spellphase: plays each party member's ITEM/SPELL in order (myfight 4), then scr_attackphase.
#[derive(Default)]
pub struct SpellPhase {
    pub spelltimer: f64,
    pub spellmax: f64,
    pub spelltotal: f64,
    pub char: f64,
    pub castyet: f64,
    pub re_castyet: f64,
    pub active: f64,
    pub using: [f64; 3],
    pub gotspell: [f64; 3],
    pub gotitem: [f64; 3],
    /// the battle writer showing the current spell text (aborted from outside by the knight/controller)
    pub spellwriter: Id,
}

fn set_hero_state(g: &mut Game, slot: usize, state: f64, reset_timer: bool) {
    let id = g.glob.charinstance[slot];
    if let Some((_, h)) = g.get::<Hero>(id) {
        h.state = state;
        if reset_timer {
            h.attacktimer = 0.0;
        }
    }
}

impl SpellPhase {
    fn finish(&mut self, me: &mut Inst, g: &mut Game) {
        controller::scr_attackphase(g);
        if self.spellwriter != NOONE {
            g.destroy(self.spellwriter);
        }
        g.destroy_self(me);
    }
}

impl Object for SpellPhase {
    fn name(&self) -> &'static str { "obj_spellphase" }
    fn create(&mut self, me: &mut Inst, _g: &mut Game) {
        self.spelltimer = 0.0;
        self.spellmax = 40.0;
        self.spelltotal = 0.0;
        self.char = 0.0;
        self.castyet = 0.0;
        self.re_castyet = 0.0;
        self.active = 0.0;
        self.spellwriter = NOONE;
        me.alarm[0] = 5;
    }
    fn alarm(&mut self, n: usize, _me: &mut Inst, g: &mut Game) {
        if n != 0 {
            return;
        }
        for xyz in 0..3 {
            self.using[xyz] = 0.0;
            self.gotspell[xyz] = 0.0;
            self.gotitem[xyz] = 0.0;
            for (action, state) in [(2.0, 2.0), (4.0, 4.0)] {
                if g.glob.charaction[xyz] == action {
                    self.spelltotal += 1.0;
                    self.using[xyz] = 1.0;
                    if action == 2.0 {
                        self.gotspell[xyz] = 1.0;
                    } else {
                        self.gotitem[xyz] = 1.0;
                    }
                    if self.castyet == 0.0 {
                        set_hero_state(g, xyz, state, true);
                        self.castyet = 1.0;
                        self.char = (xyz + 1) as f64;
                        scr_spelltext(g, g.glob.charspecial[xyz], xyz);
                        self.spellwriter = scr_battletext_default(g);
                    }
                }
            }
        }
        self.active = 1.0;
        g.glob.spelldelay = 90.0;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.active != 1.0 {
            return;
        }
        self.spelltimer += 1.0;
        if self.spelltimer >= g.glob.spelldelay && !g.id_exists(self.spellwriter) {
            if self.char >= 3.0 || self.spelltotal == 1.0 {
                self.finish(me, g);
            } else if scr_monsterpop(g) > 0.0 {
                let c = self.char as usize;
                if self.gotitem[c] == 1.0 {
                    self.re_castyet = 1.0;
                    set_hero_state(g, c, 4.0, false);
                    if self.spellwriter != NOONE {
                        g.destroy(self.spellwriter);
                    }
                    scr_spelltext(g, g.glob.charspecial[c], c);
                    self.spellwriter = scr_battletext_default(g);
                }
                if self.gotspell[c] == 1.0 {
                    self.re_castyet = 1.0;
                    set_hero_state(g, c, 2.0, false);
                    if self.spellwriter != NOONE {
                        g.destroy(self.spellwriter);
                    }
                    scr_spelltext(g, g.glob.charspecial[c], c);
                    self.spellwriter = scr_battletext_default(g);
                }
                g.glob.spelldelay = 90.0;
                if self.re_castyet == 0.0 {
                    g.glob.spelldelay = 1.0;
                }
                self.char += 1.0;
                for _ in 0..2 {
                    if self.char < 3.0 && self.using[self.char as usize] == 0.0 {
                        self.char += 1.0;
                    }
                }
                self.spelltimer = 0.0;
                self.re_castyet = 0.0;
            } else {
                self.finish(me, g);
            }
        }
    }
    fn draw(&mut self, _me: &mut Inst, _g: &mut Game) {}
    obj_vars!(spelltimer, spelltotal, char, active);
}

// ============================================================================ effect objects

/// scr_oflash() run by instance `target`: a white silhouette flash over it. Returns the obj_oflash id.
pub fn scr_oflash(g: &mut Game, target: Id, follow: bool) -> Id {
    let Some(t) = g.inst(target) else { return NOONE };
    let (x, y, depth, xs, ys, ii, si) = (t.x, t.y, t.depth, t.image_xscale, t.image_yscale, t.image_index, t.sprite_index);
    let id = g.instance_create_depth(x, y, depth - 1.0, Box::new(OFlash::default()));
    if let Some((i, o)) = g.get::<OFlash>(id) {
        i.image_xscale = xs;
        i.image_yscale = ys;
        i.image_speed = 0.0;
        i.image_index = ii;
        i.sprite_index = si;
        o.target = target;
        o.follow = follow;
    }
    id
}

/// obj_oflash
pub struct OFlash {
    pub flashspeed: f64,
    pub siner: f64,
    pub target: Id,
    pub flashcolor: Color,
    pub follow: bool,
}
impl Default for OFlash {
    fn default() -> Self { OFlash { flashspeed: 1.0, siner: 0.0, target: NOONE, flashcolor: C_WHITE, follow: false } }
}
impl Object for OFlash {
    fn name(&self) -> &'static str { "obj_oflash" }
    fn create(&mut self, me: &mut Inst, _g: &mut Game) {
        self.flashspeed = 1.0;
        self.siner = 0.0;
        me.image_speed = 0.0;
        self.flashcolor = C_WHITE;
        self.follow = false;
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if let Some(t) = g.inst(self.target) {
            me.image_index = t.image_index;
            me.sprite_index = t.sprite_index;
            if self.follow {
                me.x = t.x;
                me.y = t.y;
            }
        }
        self.siner += self.flashspeed;
        g.gfx.gpu_set_fog(true, self.flashcolor);
        g.draw_sprite_ext(me.sprite_index, me.image_index, me.x, me.y, me.image_xscale, me.image_yscale, 0.0, me.image_blend, (self.siner / 3.0).sin());
        g.gfx.gpu_set_fog(false, C_BLACK);
        if self.siner > 4.0 && (self.siner / 3.0).sin() < 0.0 {
            g.destroy_self(me);
        }
    }
    obj_vars!(siner, flashspeed);
}

/// obj_healanim: sparkle particles over a healed hero.
pub struct HealAnim {
    pub target: Id,
    pub starcount: f64,
    pub t: f64,
    pub particlesprite: Spr,
    pub particlecolor: Color,
    pub star: Vec<Id>,
    pub sw: f64,
    pub sh: f64,
}
impl Default for HealAnim {
    fn default() -> Self {
        HealAnim { target: NOONE, starcount: 0.0, t: 0.0, particlesprite: NO_SPR, particlecolor: C_LIME, star: Vec::new(), sw: 0.0, sh: 0.0 }
    }
}
impl Object for HealAnim {
    fn name(&self) -> &'static str { "obj_healanim" }
    fn create(&mut self, _me: &mut Inst, _g: &mut Game) {
        self.target = NOONE;
        self.starcount = 0.0;
        self.t = 0.0;
        self.particlesprite = spr("spr_sparestar_anim");
        self.particlecolor = C_LIME;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        self.t += 1.0;
        if g.id_exists(self.target) {
            if self.t == 1.0 {
                if let Some(t) = g.inst(self.target) {
                    me.x = t.x;
                    me.y = t.y;
                    me.image_xscale = t.image_xscale;
                    me.image_yscale = t.image_yscale;
                    me.sprite_index = t.sprite_index;
                }
                self.sw = g.sprite_width(me);
                self.sh = g.sprite_height(me);
                if let Some((_, h)) = g.get::<Hero>(self.target) {
                    // obj_herokris / obj_herosusie / obj_heroralsei
                    self.sw = h.mywidth;
                    self.sh = h.myheight;
                }
                scr_oflash(g, self.target, false);
            }
            if self.t >= 1.0 && self.t <= 5.0 {
                for _ in 0..2 {
                    let sx = me.x + g.random(self.sw);
                    let sy = me.y + g.random(self.sh);
                    let s = g.marker(sx, sy, self.particlesprite);
                    let ang = g.random(360.0);
                    let hs = 2.0 - g.random(2.0);
                    let vs = -3.0 - g.random(2.0);
                    if let Some(m) = g.inst_mut(s) {
                        m.image_angle = ang;
                        m.depth = -10.0;
                        m.image_xscale = 2.0;
                        m.image_yscale = 2.0;
                        m.image_alpha = 2.0;
                        m.image_speed = 0.25;
                        m.set_hspeed(hs);
                        m.set_vspeed(vs);
                        m.friction = 0.2;
                        m.sprite_index = self.particlesprite;
                        m.image_blend = self.particlecolor;
                    }
                    self.star.push(s);
                    self.starcount += 1.0;
                }
            }
            if self.t >= 5.0 && self.t <= 30.0 {
                for &s in &self.star {
                    let mut kill = false;
                    if let Some(m) = g.inst_mut(s) {
                        m.image_angle -= 10.0;
                        m.image_alpha -= 0.1;
                        kill = m.image_alpha <= 0.0;
                    }
                    if kill {
                        g.destroy(s);
                    }
                }
                if self.t >= 30.0 {
                    g.destroy_self(me);
                }
            }
        } else {
            for &s in &self.star {
                g.destroy(s);
            }
            g.destroy_self(me);
        }
    }
    obj_vars!(t, starcount);
}

/// obj_rudebuster_anim: Susie's Rude Buster swing; fires obj_rudebuster_bolt on frame 10.
pub struct RudebusterAnim {
    pub t: f64,
    pub target: Id,
    pub damage: f64,
    pub caster: f64,
    pub star: usize,
    pub battlemode: f64,
    pub red: f64,
}
impl Default for RudebusterAnim {
    fn default() -> Self { RudebusterAnim { t: 0.0, target: NOONE, damage: 1.0, caster: 0.0, star: 0, battlemode: 1.0, red: 0.0 } }
}
impl Object for RudebusterAnim {
    fn name(&self) -> &'static str { "obj_rudebuster_anim" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.t = 0.0;
        me.image_speed = 0.0;
        me.image_xscale = 2.0;
        me.image_yscale = 2.0;
        self.damage = 1.0;
        self.caster = 0.0;
        self.star = 0;
        self.battlemode = 1.0;
        if g.exists("obj_herosusie") {
            if let Some(s) = g.first_inst("obj_herosusie") {
                me.depth = s.depth;
                s.visible = false;
            }
        } else {
            self.battlemode = 0.0;
        }
        self.red = 0.0;
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        me.image_index = self.t / 2.0;
        if self.t >= 28.0 {
            for id in g.ids_of("obj_herosusie") {
                if let Some(s) = g.inst_mut(id) {
                    s.visible = true;
                }
            }
            g.destroy_self(me);
        }
        if g.id_exists(self.target) && self.t == 10.0 {
            g.snd_play("snd_rudebuster_swing");
            let b = g.instance_create(me.x + 40.0, me.y + 30.0, Box::new(RudebusterBolt::default()));
            if let Some((_, bo)) = g.get::<RudebusterBolt>(b) {
                bo.caster = self.caster;
                bo.target = self.target;
                bo.damage = self.damage;
                bo.star = self.star;
                if self.red == 1.0 {
                    bo.red = 1.0;
                }
            }
        }
        self.t += 1.0;
    }
    obj_vars!(t, damage);
}

/// obj_rudebuster_bolt: the homing beam; pressing Z at the right time adds bonus damage.
pub struct RudebusterBolt {
    pub target: Id,
    pub damage: f64,
    pub star: usize,
    pub caster: f64,
    pub a: f64,
    pub targetx: f64,
    pub targety: f64,
    pub cx: f64,
    pub cy: f64,
    pub t: f64,
    pub tmax: f64,
    pub siner: f64,
    pub explode: f64,
    pub bolt_timer: f64,
    pub chosen_bolt: f64,
    pub final_bolt: f64,
    pub lockdamage: bool,
    pub red: f64,
    pub battlemode: f64,
    pub bonus_anim: f64,
    pub burst: [Id; 8],
    pub aft: Vec<Id>,
}
impl Default for RudebusterBolt {
    fn default() -> Self {
        RudebusterBolt {
            target: NOONE,
            damage: 1.0,
            star: 0,
            caster: 0.0,
            a: 0.0,
            targetx: 0.0,
            targety: 0.0,
            cx: 0.0,
            cy: 0.0,
            t: 0.0,
            tmax: 4.0,
            siner: 0.0,
            explode: 0.0,
            bolt_timer: 0.0,
            chosen_bolt: 0.0,
            final_bolt: 0.0,
            lockdamage: false,
            red: 0.0,
            battlemode: 1.0,
            bonus_anim: 0.0,
            burst: [NOONE; 8],
            aft: Vec::new(),
        }
    }
}
impl Object for RudebusterBolt {
    fn name(&self) -> &'static str { "obj_rudebuster_bolt" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        *self = RudebusterBolt::default();
        me.image_alpha = 0.0;
        me.image_xscale = 2.0;
        me.image_yscale = 2.0;
        me.image_speed = 1.0;
        if g.glob.fighting == 0.0 {
            self.battlemode = 0.0;
        }
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if me.image_alpha < 1.0 {
            me.image_alpha += 0.25;
        } else {
            me.image_alpha = 1.0;
        }
        if self.t == 0.0 {
            if self.battlemode == 1.0 {
                let myself = g.get::<KnightEnemy>(self.target).map(|(_, k)| k.myself).unwrap_or(self.star).min(2);
                self.targetx = g.glob.monsterx[myself];
                self.targety = g.glob.monstery[myself];
                if g.exists("obj_knight_enemy") {
                    self.targety -= 50.0;
                }
            } else if let Some(t) = g.inst(self.target) {
                let (tx, ty) = (t.x, t.y);
                let (tw, th) = {
                    let t = g.inst(self.target).unwrap();
                    (g.sprite_width(t), g.sprite_height(t))
                };
                self.targetx = tx + tw / 2.0;
                self.targety = ty + th / 2.0;
            }
            self.cx = self.targetx;
            self.cy = self.targety;
            me.set_direction(point_direction(me.x, me.y, self.cx, self.cy) - 20.0);
            me.set_speed(24.0);
            me.friction = -1.5;
            me.image_angle = me.direction();
            if self.red == 1.0 {
                me.sprite_index = spr("spr_rudebuster_beam_red");
                me.image_xscale = 2.5;
                me.image_yscale = 2.5;
            }
        }
        if self.t >= 1.0 && self.explode == 0.0 {
            self.bolt_timer += 1.0;
            if g.input.pressed(Key::B1) && self.bolt_timer >= 4.0 && self.chosen_bolt == 0.0 && !self.lockdamage {
                self.chosen_bolt = self.bolt_timer;
                self.lockdamage = true;
            }
            let dir = point_direction(me.x, me.y, self.cx, self.cy);
            let d = me.direction() + angle_difference(dir, me.direction()) / 4.0;
            me.set_direction(d);
            me.image_angle = me.direction();
            if point_distance(me.x, me.y, self.cx, self.cy) <= 40.0 {
                self.final_bolt = self.bolt_timer;
                me.visible = false;
                self.explode = 1.0;
                self.t = 1.0;
            }
        }
        if self.explode == 1.0 {
            if self.t == 1.0 {
                self.bonus_anim = 0.0;
                if self.chosen_bolt > 0.0 {
                    let diff = self.final_bolt - self.chosen_bolt;
                    let bonus = match diff as i32 {
                        0 => 30.0,
                        1 => 28.0,
                        2 => 22.0,
                        3 => 20.0,
                        4 => 13.0,
                        5 => 11.0,
                        6 => 10.0,
                        _ => 0.0,
                    };
                    if (0.0..=6.0).contains(&diff) {
                        self.damage += bonus;
                    }
                    if abs(self.chosen_bolt - self.final_bolt) <= 2.0 {
                        self.bonus_anim = 1.0;
                        g.snd_play("snd_scytheburst");
                    }
                }
                if self.red == 1.0 {
                    self.damage += 90.0;
                }
                if self.battlemode == 1.0 {
                    g.glob.hittarget[self.star] = 0.0;
                    if g.exists("obj_knight_enemy") {
                        self.damage = round(self.damage / 2.0);
                    }
                    heroes::scr_damage_enemy(g, self.caster, self.star, self.damage);
                    if g.glob.monstertype[0] != 20.0 && g.glob.monstertype[0] != 103.0 {
                        let of = scr_oflash(g, self.target, false);
                        if self.red == 1.0 {
                            if let Some((_, o)) = g.get::<OFlash>(of) {
                                o.flashcolor = C_RED;
                            }
                        }
                    }
                } else {
                    let of = scr_oflash(g, self.target, false);
                    if self.red == 1.0 {
                        if let Some((_, o)) = g.get::<OFlash>(of) {
                            o.flashcolor = C_RED;
                        }
                    }
                }
                g.snd_play("snd_rudebuster_hit");
                for i in 0..8 {
                    let b = g.afterimage(me);
                    let spd = if self.bonus_anim == 1.0 { 40.0 } else { 25.0 };
                    if let Some(bi) = g.inst_mut(b) {
                        bi.image_speed = 0.5;
                        bi.x = self.cx;
                        bi.y = self.cy;
                        bi.image_angle = 45.0 + (i as f64 * 90.0);
                        bi.set_direction(bi.image_angle);
                        bi.set_speed(spd);
                        bi.depth = me.depth - 10.0;
                    }
                    self.burst[i] = b;
                }
            }
            if self.t >= 2.0 {
                for (i, &b) in self.burst.iter().enumerate() {
                    let f = if i < 4 { 0.75 } else { 0.8 };
                    if let Some(bi) = g.inst_mut(b) {
                        let s = bi.speed() * f;
                        bi.set_speed(s);
                        bi.image_xscale *= 0.8;
                    }
                }
            }
            if self.t >= 18.0 {
                g.destroy_self(me);
            }
        }
        if self.explode == 0.0 {
            let a = g.afterimage(me);
            if let Some(ai) = g.inst_mut(a) {
                ai.image_yscale = 1.8;
                ai.image_angle = me.image_angle;
                ai.image_index = 4.0;
                ai.image_speed = 0.5;
                ai.image_alpha = me.image_alpha - 0.2;
            }
            self.aft.push(a);
        }
        for &a in &self.aft {
            let mut kill = false;
            if let Some(ai) = g.inst_mut(a) {
                ai.image_yscale -= 0.1;
                kill = ai.image_yscale <= 0.1;
            }
            if kill {
                g.destroy(a);
            }
            if self.explode == 1.0 {
                let mut kill = false;
                if let Some(ai) = g.inst_mut(a) {
                    ai.image_alpha -= 0.07;
                    ai.image_yscale *= 0.9;
                    kill = ai.image_yscale <= 0.1;
                }
                if kill {
                    g.destroy(a);
                }
            }
        }
        self.a += 1.0;
        self.t += 1.0;
    }
    obj_vars!(t, damage, explode);
}

/// obj_pacifyspell: con 1 = sleep-away (tired), con 5.. = failed flash, con 20 = "Z" burst.
pub struct PacifySpell {
    pub target: Id,
    pub con: f64,
    pub siner: f64,
    pub xx: f64,
    pub yy: f64,
    pub fail: f64,
    pub flashcolor: Color,
    pub timer: f64,
    pub zcounter: f64,
}
impl Default for PacifySpell {
    fn default() -> Self {
        PacifySpell { target: NOONE, con: 1.0, siner: 0.0, xx: 0.0, yy: 0.0, fail: 0.0, flashcolor: C_BLUE, timer: 0.0, zcounter: 0.0 }
    }
}
impl Object for PacifySpell {
    fn name(&self) -> &'static str { "obj_pacifyspell" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.target = NOONE;
        self.con = 1.0;
        self.siner = 0.0;
        self.xx = g.camerax();
        self.yy = g.cameray();
        me.alarm[4] = 50;
        self.fail = 0.0;
        self.flashcolor = C_BLUE;
        self.timer = 0.0;
        self.zcounter = 0.0;
    }
    fn alarm(&mut self, n: usize, _me: &mut Inst, _g: &mut Game) {
        if n == 4 {
            self.con += 1.0;
        }
    }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if !g.id_exists(self.target) && self.con < 20.0 {
            self.con = 99.0;
            g.destroy_self(me);
            return;
        }
        let target = self.target;
        if self.con == 1.0 {
            if self.fail == 0.0 {
                self.siner += 1.0;
                let fc = self.flashcolor;
                if let Some(t) = g.inst_mut(target) {
                    t.image_blend = merge_color(t.image_blend, fc, 0.1);
                }
                if self.siner % 2.0 == 0.0 {
                    let (tx, ty, tw, th) = match g.inst(target) {
                        Some(t) => (t.x, t.y, g.sprite_width(t), g.sprite_height(t)),
                        None => (0.0, 0.0, 0.0, 0.0),
                    };
                    let ax = tx + g.random(tw);
                    let ai = g.instance_create(ax, (ty + th) - 20.0, Box::new(Afterimage::default()));
                    if let Some(a) = g.inst_mut(ai) {
                        a.gravity = 0.5;
                        a.sprite_index = spr("spr_savepoint");
                        a.image_speed = 0.2;
                    }
                }
                let s = self.siner;
                if let Some(t) = g.inst_mut(target) {
                    t.x += (s / 4.0) + ((s / 8.0).sin() * 10.0);
                    t.y -= s / 5.0;
                }
            } else {
                self.siner = 0.0;
                self.con = 5.0;
            }
        }
        if self.con == 2.0 {
            g.destroy(target);
            g.destroy_self(me);
        }
        if self.con == 5.0 {
            self.con = 6.0;
            me.alarm[4] = 8;
        }
        if self.con == 6.0 {
            let fc = self.flashcolor;
            if let Some(t) = g.inst_mut(target) {
                t.image_blend = merge_color(t.image_blend, fc, 0.12);
            }
        }
        if self.con == 7.0 {
            self.con = 8.0;
            me.alarm[4] = 8;
        }
        if self.con == 8.0 {
            if let Some(t) = g.inst_mut(target) {
                t.image_blend = merge_color(t.image_blend, C_WHITE, 0.16);
            }
        }
        if self.con == 9.0 {
            if let Some(t) = g.inst_mut(target) {
                t.image_blend = C_WHITE;
            }
            g.destroy_self(me);
        }
        if self.con == 20.0 {
            if self.timer == 0.0 && self.zcounter == 0.0 {
                g.snd_play("snd_pacify");
            }
            self.timer -= 1.0;
            if self.timer <= 0.0 {
                let z = g.instance_create(me.x, me.y, Box::new(AfterimageGrow::default()));
                let zc = self.zcounter;
                if let Some(zi) = g.inst_mut(z) {
                    zi.sprite_index = spr("spr_spare_z");
                    zi.set_speed(12.0);
                    zi.set_direction(zc * 40.0);
                    zi.friction = 1.0;
                }
                self.timer = 2.0;
                self.zcounter += 1.0;
            }
            if self.zcounter >= 9.0 {
                self.con += 1.0;
                g.destroy_self(me);
            }
        }
    }
    fn draw(&mut self, _me: &mut Inst, _g: &mut Game) {}
    obj_vars!(con, siner, timer);
}

/// obj_afterimage_grow (used by obj_pacifyspell's "Z" burst)
pub struct AfterimageGrow {
    pub xrate: f64,
    pub yrate: f64,
    pub fade: f64,
    pub target: Id,
    pub destroytime: f64,
}
impl Default for AfterimageGrow {
    fn default() -> Self { AfterimageGrow { xrate: 0.2, yrate: 0.2, fade: 0.1, target: NOONE, destroytime: -1.0 } }
}
impl Object for AfterimageGrow {
    fn name(&self) -> &'static str { "obj_afterimage_grow" }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.target != NOONE {
            if let Some(t) = g.inst(self.target) {
                me.x = t.x;
                me.y = t.y;
            }
        }
        me.image_alpha -= self.fade;
        me.image_xscale += self.xrate;
        me.image_yscale += self.yrate;
        if me.image_alpha < 0.0 {
            g.destroy_self(me);
        }
        if self.destroytime > -1.0 {
            self.destroytime -= 1.0;
        }
        if self.destroytime == 0.0 {
            g.destroy_self(me);
        }
    }
    obj_vars!(xrate, yrate, fade, destroytime);
}

// ============================================================================ items

/// Result of scr_iteminfo(id).
#[derive(Clone, Debug, Default)]
pub struct ItemInfo {
    pub itemnameb: String,
    pub itemdescb: String,
    /// 0 none, 1 one ally, 2 whole team
    pub itemtarget: f64,
    pub value: f64,
    pub usable: f64,
    /// item id the slot turns into after use (DD-Burger → Darkburger)
    pub replaceable: f64,
}

/// scr_iteminfo(id) (chapter 3 values).
pub fn scr_iteminfo(id: f64) -> ItemInfo {
    let mk = |n: &str, d: &str, target: f64, value: f64, usable: f64| ItemInfo {
        itemnameb: n.into(),
        itemdescb: d.into(),
        itemtarget: target,
        value,
        usable,
        replaceable: 0.0,
    };
    match id as i32 {
        0 => mk(" ", "---", 0.0, 0.0, 0.0),
        1 => mk("Dark Candy", "Heals#40HP", 1.0, 25.0, 1.0),
        2 => mk("ReviveMint", "Heal#Downed#Ally", 1.0, 400.0, 1.0),
        3 => mk("Glowshard", "Sell#at#shops", 0.0, 200.0 + CHAPTER * 100.0, 0.0),
        4 => mk("Manual", "Read#out of#battle", 2.0, 1.0, 0.0),
        5 => mk("BrokenCake", "Heals#20HP", 1.0, 5.0, 1.0),
        6 => mk("Top Cake", "Heals#team#160HP", 2.0, 150.0, 1.0),
        7 => mk("Spincake", "Heals#team#150HP", 2.0, 5.0, 1.0),
        8 => mk("Darkburger", "Heals#70HP", 1.0, 70.0, 1.0),
        9 => mk("LancerCookie", "Heals#50HP", 1.0, 10.0, 1.0),
        10 => mk("GigaSalad", "Heals#4HP", 1.0, 10.0, 1.0),
        11 => mk("ClubsSandwich", "Heals#team#70HP", 2.0, 70.0, 1.0),
        12 => mk("HeartsDonut", "Healing#varies", 1.0, 40.0, 1.0),
        13 => mk("ChocDiamond", "Healing#varies", 1.0, 40.0, 1.0),
        14 => mk("Favwich", "Heals#ALL HP", 1.0, 10.0, 1.0),
        15 => mk("RouxlsRoux", "Heals#50 HP", 1.0, 50.0, 1.0),
        16 => mk("CD Bagel", "Heals#80 HP", 1.0, 100.0, 1.0),
        17 => mk("Mannequin", "Useless", 0.0, 300.0, 0.0),
        18..=21 => mk("RottenTea", "Heals#10HP", 1.0, 2.0, 1.0),
        22 => {
            let mut i = mk("DD-Burger", "Heals#60HP 2x", 1.0, 110.0, 1.0);
            i.replaceable = 8.0;
            i
        }
        23 => mk("LightCandy", "Heals#120HP", 1.0, 200.0, 1.0),
        24 => mk("ButJuice", "Heals#100HP", 1.0, 200.0, 1.0),
        25 => mk("SpagettiCode", "Heals#team#30HP", 2.0, 180.0, 1.0),
        26 => mk("JavaCookie", "Healing#varies", 1.0, 160.0, 1.0),
        27 => mk("TensionBit", "Raises#TP#32%", 2.0, 100.0, 1.0),
        28 => mk("TensionGem", "Raises#TP#50%", 2.0, 300.0, 1.0),
        29 => mk("TensionMax", "Raises#TP#Max", 2.0, 1000.0, 1.0),
        30 => mk("ReviveDust", "Revives#team#25%", 2.0, 100.0, 1.0),
        31 => mk("ReviveBrite", "Revives#team#100%", 2.0, 4000.0, 1.0),
        32 => mk("S.POISON", "Hurts#party#member", 1.0, 110.0, 1.0),
        33 => mk("DogDollar", "Not#so#useful", 0.0, floor(200.0 / CHAPTER), 0.0),
        34 => mk("TVDinner", "Heals#100HP", 1.0, 200.0, 1.0),
        35 => mk("Pipis", "Does#nothing", 1.0, 0.0, 1.0),
        36 => mk("FlatSoda", "Heals#20HP", 1.0, 2.0, 1.0),
        37 => mk("TVSlop", "Heals#80HP", 1.0, 180.0, 1.0),
        38 => mk("ExecBuffet", "Heals#team#100HP", 2.0, 600.0, 1.0),
        39 => mk("DeluxeDinner", "Heals#140HP", 1.0, 600.0, 1.0),
        _ => mk(" ", " ", 0.0, 0.0, 0.0),
    }
}

/// (item name, item description) for an item id (scr_iteminfo's itemnameb / itemdescb).
pub fn item_info(id: f64) -> (String, String) {
    let i = scr_iteminfo(id);
    (i.itemnameb, i.itemdescb)
}

/// scr_iteminfo_temp(slot): info for the 12 entries of the controller's `tempitem[i][slot]`.
pub fn scr_iteminfo_temp(tempitem: &[[f64; 3]; 13], slot: usize) -> Vec<ItemInfo> {
    (0..12).map(|i| scr_iteminfo(tempitem[i][slot])).collect()
}

/// scr_itemshift_temp(loc, slot) on the controller's `tempitem[i][slot]`.
pub fn scr_itemshift_temp(tempitem: &mut [[f64; 3]; 13], loc: usize, slot: usize) {
    tempitem[12][slot] = 0.0;
    for i in loc..12 {
        tempitem[i][slot] = tempitem[i + 1][slot];
    }
}

/// scr_itemshift(loc, newitem) on global.item.
pub fn scr_itemshift(g: &mut Game, loc: usize, newitem: f64) {
    g.glob.item[12] = newitem;
    for i in loc..12 {
        g.glob.item[i] = g.glob.item[i + 1];
    }
}

/// scr_itemremove(id): remove the first `id` from global.item. Returns `removed`.
pub fn scr_itemremove(g: &mut Game, id: f64) -> bool {
    if let Some(loc) = (0..12).find(|&i| g.glob.item[i] == id) {
        scr_itemshift(g, loc, 0.0);
        return true;
    }
    false
}

/// scr_itemconsumeb(): queue the ITEM chosen in the menu for global.charturn (charspecial = item + 200).
/// `info` is scr_iteminfo of the chosen item (GML reads usable/replaceable left by that call).
pub fn scr_itemconsumeb(g: &mut Game, tempitem: &mut [[f64; 3]; 13], info: &ItemInfo) {
    let ct = g.glob.charturn as usize;
    let loc = g.glob.bmenucoord[4][ct] as usize;
    g.glob.faceaction[ct] = 3.0;
    g.glob.charaction[ct] = 4.0;
    g.glob.charspecial[ct] = tempitem[loc][ct] + 200.0;
    if info.usable == 1.0 && info.replaceable == 0.0 {
        scr_itemshift_temp(tempitem, loc, ct);
    } else if info.replaceable > 0.0 {
        tempitem[loc][ct] = info.replaceable;
    }
    controller::scr_nexthero(g);
}

/// obj_battlecontroller's ITEM-menu confirm for items with itemtarget 0 or 2 (no target selection):
/// TensionBit/Gem/Max heal TP immediately (snd_cardrive, orange obj_healanim on the hero, item removed,
/// scr_nexthero); anything else goes through scr_itemconsumeb. For itemtarget 1 the controller opens
/// the ally menu (bmenuno 7) and calls scr_itemconsumeb after choosing.
pub fn battle_item_select_untargeted(g: &mut Game, tempitem: &mut [[f64; 3]; 13]) {
    let ct = g.glob.charturn as usize;
    let loc = g.glob.bmenucoord[4][ct] as usize;
    let it = tempitem[loc][ct];
    let info = scr_iteminfo(it);
    if info.itemtarget != 0.0 && info.itemtarget != 2.0 {
        return;
    }
    let mut tensionhealed = false;
    if it == 27.0 {
        crate::battle::scr_tensionheal(g, 80.0);
        tensionhealed = true;
    }
    if it == 28.0 {
        let amt = ceil(g.glob.maxtension / 2.0);
        crate::battle::scr_tensionheal(g, amt);
        tensionhealed = true;
    }
    if it == 29.0 {
        let amt = ceil(g.glob.maxtension);
        crate::battle::scr_tensionheal(g, amt);
        tensionhealed = true;
    }
    if tensionhealed {
        let h = g.snd_play("snd_cardrive");
        g.audio.pitch(h, 1.4);
        g.audio.volume(h, 0.8, 0.0);
        let hid = g.glob.charinstance[ct];
        if let Some((x, y)) = g.inst(hid).map(|i| (i.x, i.y)) {
            let ha = g.instance_create(x, y, Box::new(HealAnim::default()));
            if let Some((_, a)) = g.get::<HealAnim>(ha) {
                a.target = hid;
                a.particlecolor = C_ORANGE;
            }
        }
        scr_itemshift_temp(tempitem, loc, ct);
        controller::scr_nexthero(g);
    } else {
        scr_itemconsumeb(g, tempitem, &info);
    }
}
