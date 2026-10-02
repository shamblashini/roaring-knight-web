//! A small GameMaker-like instance runtime: instances with builtin variables, the event loop
//! (begin step / alarms / step / motion+animation / collisions / end step / draw),
//! collision masks, sprite/text drawing and the lerpvar / delayed-script helpers.
//!
//! Porting convention: each GML object becomes a Rust struct implementing [`Object`].
//! Builtin variables live in [`Inst`] (passed as `me`); custom variables are struct fields.

use crate::assets::{spr, Assets, FontId, Frame, Spr, Sprite, NO_SPR};
use crate::audio::Audio;
use crate::gfx::{Color, Gfx, C_WHITE};
use crate::gm::{self, Rng};
use crate::input::Input;
use std::any::Any;
use std::collections::{HashMap, HashSet};

pub type Id = i64;
pub const NOONE: Id = -4;

// ============================================================================ Inst

/// GameMaker builtin instance variables (+ the bullet variables from scr_bullet_init,
/// which Deltarune treats as present on every bullet).
pub struct Inst {
    pub id: Id,
    pub object: &'static str,
    pub x: f64,
    pub y: f64,
    pub xstart: f64,
    pub ystart: f64,
    pub xprevious: f64,
    pub yprevious: f64,
    hspeed: f64,
    vspeed: f64,
    speed: f64,
    direction: f64,
    pub friction: f64,
    pub gravity: f64,
    pub gravity_direction: f64,
    pub sprite_index: Spr,
    pub image_index: f64,
    pub image_speed: f64,
    pub image_xscale: f64,
    pub image_yscale: f64,
    pub image_angle: f64,
    pub image_alpha: f64,
    pub image_blend: Color,
    pub mask_index: Spr,
    pub depth: f64,
    pub visible: bool,
    pub alarm: [i32; 12],
    pub destroyed: bool,
    // ---- bullet variables (scr_bullet_init)
    pub grazed: f64,
    pub grazetimer: f64,
    pub destroyonhit: f64,
    pub target: f64,
    pub inv: f64,
    pub damage: f64,
    pub element: f64,
    pub grazepoints: f64,
    pub timepoints: f64,
    pub active: f64,
    pub updateimageangle: f64,
}

impl Inst {
    fn new(id: Id, object: &'static str, x: f64, y: f64, depth: f64) -> Inst {
        Inst {
            id,
            object,
            x,
            y,
            xstart: x,
            ystart: y,
            xprevious: x,
            yprevious: y,
            hspeed: 0.0,
            vspeed: 0.0,
            speed: 0.0,
            direction: 0.0,
            friction: 0.0,
            gravity: 0.0,
            gravity_direction: 270.0,
            sprite_index: NO_SPR,
            image_index: 0.0,
            image_speed: 1.0,
            image_xscale: 1.0,
            image_yscale: 1.0,
            image_angle: 0.0,
            image_alpha: 1.0,
            image_blend: C_WHITE,
            mask_index: NO_SPR,
            depth,
            visible: true,
            alarm: [-1; 12],
            destroyed: false,
            grazed: 0.0,
            grazetimer: 0.0,
            destroyonhit: 1.0,
            target: 0.0,
            inv: 60.0,
            damage: 10.0,
            element: 0.0,
            grazepoints: 1.0,
            timepoints: 1.0,
            active: 1.0,
            updateimageangle: 0.0,
        }
    }

    // --- coupled motion variables (GML: speed/direction/hspeed/vspeed stay in sync)
    pub fn speed(&self) -> f64 { self.speed }
    pub fn direction(&self) -> f64 { self.direction }
    pub fn hspeed(&self) -> f64 { self.hspeed }
    pub fn vspeed(&self) -> f64 { self.vspeed }
    pub fn set_speed(&mut self, s: f64) {
        self.speed = s;
        self.hspeed = gm::lengthdir_x(s, self.direction);
        self.vspeed = gm::lengthdir_y(s, self.direction);
    }
    pub fn set_direction(&mut self, d: f64) {
        self.direction = d.rem_euclid(360.0);
        self.hspeed = gm::lengthdir_x(self.speed, self.direction);
        self.vspeed = gm::lengthdir_y(self.speed, self.direction);
    }
    fn from_components(&mut self) {
        self.speed = (self.hspeed * self.hspeed + self.vspeed * self.vspeed).sqrt();
        if self.hspeed != 0.0 || self.vspeed != 0.0 {
            self.direction = gm::point_direction(0.0, 0.0, self.hspeed, self.vspeed);
        }
    }
    pub fn set_hspeed(&mut self, h: f64) {
        self.hspeed = h;
        self.from_components();
    }
    pub fn set_vspeed(&mut self, v: f64) {
        self.vspeed = v;
        self.from_components();
    }
    /// motion_set(dir, speed)
    pub fn motion_set(&mut self, dir: f64, spd: f64) {
        self.direction = dir.rem_euclid(360.0);
        self.set_speed(spd);
    }
    /// motion_add(dir, speed)
    pub fn motion_add(&mut self, dir: f64, spd: f64) {
        self.hspeed += gm::lengthdir_x(spd, dir);
        self.vspeed += gm::lengthdir_y(spd, dir);
        self.from_components();
    }
    /// move_towards_point(x, y, sp)
    pub fn move_towards_point(&mut self, x: f64, y: f64, sp: f64) {
        self.direction = gm::point_direction(self.x, self.y, x, y);
        self.set_speed(sp);
    }

    /// Generic variable access by GML name, used by lerpvar & co.
    pub fn builtin_var(&mut self, name: &str) -> Option<&mut f64> {
        Some(match name {
            "x" => &mut self.x,
            "y" => &mut self.y,
            "image_index" => &mut self.image_index,
            "image_speed" => &mut self.image_speed,
            "image_xscale" => &mut self.image_xscale,
            "image_yscale" => &mut self.image_yscale,
            "image_angle" => &mut self.image_angle,
            "image_alpha" => &mut self.image_alpha,
            "depth" => &mut self.depth,
            "friction" => &mut self.friction,
            "gravity" => &mut self.gravity,
            "gravity_direction" => &mut self.gravity_direction,
            "damage" => &mut self.damage,
            "active" => &mut self.active,
            "grazed" => &mut self.grazed,
            "grazepoints" => &mut self.grazepoints,
            "target" => &mut self.target,
            _ => return None,
        })
    }
    pub fn get_motion_var(&self, name: &str) -> Option<f64> {
        Some(match name {
            "speed" => self.speed,
            "direction" => self.direction,
            "hspeed" => self.hspeed,
            "vspeed" => self.vspeed,
            "visible" => self.visible as i32 as f64,
            "image_blend" => self.image_blend as f64,
            "sprite_index" => self.sprite_index as f64,
            "mask_index" => self.mask_index as f64,
            _ => return None,
        })
    }
    pub fn set_motion_var(&mut self, name: &str, v: f64) -> bool {
        match name {
            "speed" => self.set_speed(v),
            "direction" => self.set_direction(v),
            "hspeed" => self.set_hspeed(v),
            "vspeed" => self.set_vspeed(v),
            "visible" => self.visible = v >= 0.5,
            "image_blend" => self.image_blend = v as Color,
            "sprite_index" => self.sprite_index = v as Spr,
            "mask_index" => self.mask_index = v as Spr,
            _ => return false,
        }
        true
    }
}

// ============================================================================ Object trait

#[allow(unused_variables)]
pub trait Object: Any {
    /// GML object name, e.g. "obj_heart". Used for parent lookups & instance queries.
    fn name(&self) -> &'static str;
    fn create(&mut self, me: &mut Inst, g: &mut Game) {}
    fn begin_step(&mut self, me: &mut Inst, g: &mut Game) {}
    fn step(&mut self, me: &mut Inst, g: &mut Game) {}
    fn end_step(&mut self, me: &mut Inst, g: &mut Game) {}
    /// Draw event. Default = draw_self (GameMaker's behaviour when no Draw event exists).
    fn draw(&mut self, me: &mut Inst, g: &mut Game) { g.draw_self(me) }
    fn draw_end(&mut self, me: &mut Inst, g: &mut Game) {}
    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {}
    /// event_user(n). Bullets' Other_15 is `user(5)`; the default implements obj_collidebullet's.
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 5 && g.is_a(me.object, "obj_collidebullet") {
            crate::battle::collidebullet_hit(me, g);
        }
    }
    fn destroy(&mut self, me: &mut Inst, g: &mut Game) {}
    fn cleanup(&mut self, me: &mut Inst, g: &mut Game) {}
    fn animation_end(&mut self, me: &mut Inst, g: &mut Game) {}
    /// Custom variables by name (for scr_lerpvar etc). Use the `vars!` macro.
    fn var(&mut self, name: &str) -> Option<&mut f64> { None }
    fn as_any(&mut self) -> &mut dyn Any;
}

/// Implements `as_any` and `var` for an object struct: `obj_vars!(field_a, field_b);`
#[macro_export]
macro_rules! obj_vars {
    ($($f:ident),* $(,)?) => {
        fn as_any(&mut self) -> &mut dyn std::any::Any { self }
        #[allow(unused_variables)]
        fn var(&mut self, name: &str) -> Option<&mut f64> {
            match name {
                $(stringify!($f) => Some(&mut self.$f),)*
                _ => None,
            }
        }
    };
}

struct Slot {
    inst: Inst,
    obj: Box<dyn Object>,
}

// ============================================================================ Globals

/// Deltarune `global.*` state used by the battle. Field names follow GML (`global.bmenuno` → `g.glob.bmenuno`).
/// Party-slot arrays are indexed 0..3 (global.char[slot] gives the character id); character arrays
/// (`hp`, `maxhp`, `charname`, ...) are indexed by character id (1 Kris, 2 Susie, 3 Ralsei).
/// Anything not declared here: `g.glob.extra` (`g.glob.ex("name")` / `g.glob.set_ex("name", v)`).
pub struct Glob {
    pub time: f64,
    pub inv: f64,
    pub invc: f64,
    pub turntimer: f64,
    pub mnfight: f64,
    pub myfight: f64,
    pub tension: f64,
    pub maxtension: f64,
    pub tensionselect: f64,
    pub temptension: [f64; 3],
    // ---- characters (index = character id)
    pub hp: [f64; 5],
    pub maxhp: [f64; 5],
    pub at: [f64; 5],
    pub df: [f64; 5],
    pub mag: [f64; 5],
    pub charname: [String; 5],
    pub charweapon: [f64; 5],
    pub chararmor1: [f64; 5],
    pub chararmor2: [f64; 5],
    /// spell ids known by each character id (global.spell[char][i])
    pub spell: [[f64; 12]; 5],
    // ---- party slots (index = 0..3)
    pub char: [usize; 3],
    pub charinstance: [Id; 3],
    pub battleat: [f64; 3],
    pub battledf: [f64; 3],
    pub battlemag: [f64; 3],
    pub charaction: [f64; 3],
    pub chartarget: [f64; 3],
    pub charspecial: [f64; 3],
    pub faceaction: [f64; 3],
    pub charmove: [f64; 3],
    pub charcantarget: [f64; 3],
    pub chardead: [f64; 3],
    pub charcond: [f64; 3],
    pub automiss: [f64; 3],
    pub acting: [f64; 3],
    pub actingsingle: [f64; 3],
    pub actingsimul: [f64; 3],
    pub actingtarget: [f64; 3],
    pub actingchoice: [f64; 3],
    pub hittarget: [f64; 3],
    pub targeted: [f64; 3],
    pub rembmenuno: [f64; 3],
    pub battleactcount: [f64; 3],
    pub currentactingchar: f64,
    pub charturn: f64,
    pub charselect: f64,
    pub attacking: f64,
    pub spelldelay: f64,
    /// TECH menu per party slot (global.battlespell[slot][i]; -1 = the character's ACT entry)
    pub battlespell: [[f64; 18]; 3],
    pub battlespellname: [[String; 18]; 3],
    pub battlespelldesc: [[String; 18]; 3],
    pub battlespellcost: [[f64; 18]; 3],
    pub battlespelltarget: [[f64; 18]; 3],
    pub battlespellspecial: [[f64; 18]; 3],
    // ---- menus
    pub bmenuno: f64,
    /// global.bmenucoord[menu][charturn]
    pub bmenucoord: [[f64; 20]; 20],
    // ---- inventory (item ids, 0 = empty)
    pub item: [f64; 13],
    // ---- monsters (index = monster slot; only slot 0 is used here)
    pub monster: [f64; 3],
    pub monstertype: [f64; 3],
    pub monsterinstance: [Id; 3],
    pub monstername: [String; 3],
    pub monstercomment: [String; 3],
    pub monsterstatus: [f64; 3],
    pub monsterhp: [f64; 3],
    pub monstermaxhp: [f64; 3],
    pub monsterat: [f64; 3],
    pub monsterdf: [f64; 3],
    pub monsterx: [f64; 3],
    pub monstery: [f64; 3],
    pub monstermakex: [f64; 3],
    pub monstermakey: [f64; 3],
    pub mercymod: [f64; 3],
    pub mercymax: [f64; 3],
    pub sparepoint: [f64; 3],
    pub monsterattackname: [String; 3],
    /// knight ACTs: global.canact / actname / actdesc / actcost / actsimul / actactor [0][i]
    pub canact: [f64; 6],
    pub actname: [String; 6],
    pub actdesc: [String; 6],
    pub actcost: [f64; 6],
    pub actsimul: [f64; 6],
    pub actactor: [f64; 6],
    pub canactsus: [f64; 6],
    pub actnamesus: [String; 6],
    pub actdescsus: [String; 6],
    pub actcostsus: [f64; 6],
    pub actsimulsus: [f64; 6],
    pub canactral: [f64; 6],
    pub actnameral: [String; 6],
    pub actdescral: [String; 6],
    pub actcostral: [f64; 6],
    pub actsimulral: [f64; 6],
    pub heromakex: [f64; 3],
    pub heromakey: [f64; 3],
    // ---- text
    pub msg: Vec<String>,
    pub msgno: f64,
    pub battlemsg: [String; 3],
    pub typer: f64,
    pub battletyper: f64,
    pub fc: f64,
    pub fe: f64,
    // ---- misc
    pub sp: f64,
    pub heartx: f64,
    pub hearty: f64,
    pub interact: f64,
    pub fighting: f64,
    pub darkzone: f64,
    pub battleend: f64,
    pub encounterno: f64,
    pub firstknightbattle: f64,
    /// handle of the playing battle music (global.batmusic[1])
    pub batmusic: i64,
    pub flag: HashMap<i32, f64>,
    pub tempflag: HashMap<i32, f64>,
    pub extra: HashMap<String, f64>,
}

fn strs<const N: usize>() -> [String; N] { std::array::from_fn(|_| String::new()) }

impl Default for Glob {
    fn default() -> Self {
        Glob {
            time: 0.0,
            inv: 0.0,
            invc: 1.0,
            turntimer: 0.0,
            mnfight: 0.0,
            myfight: 0.0,
            tension: 0.0,
            maxtension: 250.0,
            tensionselect: 0.0,
            temptension: [0.0; 3],
            hp: [0.0; 5],
            maxhp: [0.0; 5],
            at: [0.0; 5],
            df: [0.0; 5],
            mag: [0.0; 5],
            charname: strs(),
            charweapon: [0.0; 5],
            chararmor1: [0.0; 5],
            chararmor2: [0.0; 5],
            spell: [[0.0; 12]; 5],
            char: [1, 2, 3],
            charinstance: [NOONE; 3],
            battleat: [0.0; 3],
            battledf: [0.0; 3],
            battlemag: [0.0; 3],
            charaction: [0.0; 3],
            chartarget: [0.0; 3],
            charspecial: [0.0; 3],
            faceaction: [0.0; 3],
            charmove: [1.0; 3],
            charcantarget: [1.0; 3],
            chardead: [0.0; 3],
            charcond: [0.0; 3],
            automiss: [0.0; 3],
            acting: [0.0; 3],
            actingsingle: [0.0; 3],
            actingsimul: [0.0; 3],
            actingtarget: [0.0; 3],
            actingchoice: [0.0; 3],
            hittarget: [0.0; 3],
            targeted: [0.0; 3],
            rembmenuno: [0.0; 3],
            battleactcount: [0.0; 3],
            currentactingchar: 0.0,
            charturn: 0.0,
            charselect: -1.0,
            attacking: 0.0,
            spelldelay: 10.0,
            battlespell: [[0.0; 18]; 3],
            battlespellname: std::array::from_fn(|_| strs()),
            battlespelldesc: std::array::from_fn(|_| strs()),
            battlespellcost: [[0.0; 18]; 3],
            battlespelltarget: [[2.0; 18]; 3],
            battlespellspecial: [[0.0; 18]; 3],
            bmenuno: 0.0,
            bmenucoord: [[0.0; 20]; 20],
            item: [0.0; 13],
            monster: [0.0; 3],
            monstertype: [0.0; 3],
            monsterinstance: [NOONE; 3],
            monstername: strs(),
            monstercomment: strs(),
            monsterstatus: [0.0; 3],
            monsterhp: [0.0; 3],
            monstermaxhp: [0.0; 3],
            monsterat: [0.0; 3],
            monsterdf: [0.0; 3],
            monsterx: [0.0; 3],
            monstery: [0.0; 3],
            monstermakex: [0.0; 3],
            monstermakey: [0.0; 3],
            mercymod: [0.0; 3],
            mercymax: [100.0; 3],
            sparepoint: [0.0; 3],
            monsterattackname: strs(),
            canact: [0.0; 6],
            actname: strs(),
            actdesc: strs(),
            actcost: [0.0; 6],
            actsimul: [0.0; 6],
            actactor: [0.0; 6],
            canactsus: [0.0; 6],
            actnamesus: strs(),
            actdescsus: strs(),
            actcostsus: [0.0; 6],
            actsimulsus: [0.0; 6],
            canactral: [0.0; 6],
            actnameral: strs(),
            actdescral: strs(),
            actcostral: [0.0; 6],
            actsimulral: [0.0; 6],
            heromakex: [0.0; 3],
            heromakey: [0.0; 3],
            msg: vec![String::new(); 32],
            msgno: 0.0,
            battlemsg: strs(),
            typer: 4.0,
            battletyper: 4.0,
            fc: 0.0,
            fe: 0.0,
            sp: 4.0,
            heartx: 0.0,
            hearty: 0.0,
            interact: 0.0,
            fighting: 1.0,
            darkzone: 1.0,
            battleend: 0.0,
            encounterno: 115.0,
            firstknightbattle: 0.0,
            batmusic: -1,
            flag: HashMap::new(),
            tempflag: HashMap::new(),
            extra: HashMap::new(),
        }
    }
}
impl Glob {
    pub fn flag(&self, i: i32) -> f64 { *self.flag.get(&i).unwrap_or(&0.0) }
    pub fn set_flag(&mut self, i: i32, v: f64) { self.flag.insert(i, v); }
    pub fn tempflag(&self, i: i32) -> f64 { *self.tempflag.get(&i).unwrap_or(&0.0) }
    pub fn set_tempflag(&mut self, i: i32, v: f64) { self.tempflag.insert(i, v); }
    pub fn ex(&self, k: &str) -> f64 { *self.extra.get(k).unwrap_or(&0.0) }
    pub fn set_ex(&mut self, k: &str, v: f64) { self.extra.insert(k.to_string(), v); }
    /// hp of the character in party slot `slot`
    pub fn slot_hp(&self, slot: usize) -> f64 { self.hp[self.char[slot]] }
}

// ============================================================================ Game

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

pub struct Game {
    pub gfx: Gfx,
    pub audio: Audio,
    pub input: Input,
    pub assets: Assets,
    pub glob: Glob,
    pub rng: Rng,
    slots: Vec<Option<Slot>>,
    index: HashMap<Id, usize>,
    next_id: Id,
    /// ids of instances whose events are currently running (their slot is temporarily empty)
    exec: Vec<(Id, &'static str)>,
    /// executing instances that destroyed themselves (or were destroyed) mid-event
    dying: HashSet<Id>,
    parent_cache: HashMap<&'static str, &'static str>,
    pub font: FontId,
    pub halign: HAlign,
    pub valign: VAlign,
    pub frame: u64,
    /// set by battle code: the game-specific collision pass
    pub collision_hook: Option<fn(&mut Game)>,
    /// debug: show hitboxes
    pub show_hitboxes: bool,
    pub paused: bool,
}

impl Game {
    pub fn new(gfx: Gfx, audio: Audio, assets: Assets, seed: u64) -> Game {
        Game {
            gfx,
            audio,
            input: Input::default(),
            assets,
            glob: Glob::default(),
            rng: Rng::new(seed),
            slots: vec![],
            index: HashMap::new(),
            next_id: 100000,
            exec: vec![],
            dying: HashSet::new(),
            parent_cache: HashMap::new(),
            font: 0,
            halign: HAlign::Left,
            valign: VAlign::Top,
            frame: 0,
            collision_hook: None,
            show_hitboxes: false,
            paused: false,
        }
    }

    // ------------------------------------------------------------------ object hierarchy
    pub fn parent_of(&mut self, obj: &'static str) -> &'static str {
        if let Some(p) = self.parent_cache.get(obj) {
            return p;
        }
        let p = crate::objdata::obj_info(obj).map(|i| i.parent).unwrap_or("");
        self.parent_cache.insert(obj, p);
        p
    }
    /// object_is_ancestor-or-equal
    pub fn is_a(&mut self, obj: &'static str, ancestor: &str) -> bool {
        let mut o = obj;
        for _ in 0..16 {
            if o == ancestor {
                return true;
            }
            o = self.parent_of(o);
            if o.is_empty() {
                return false;
            }
        }
        false
    }

    // ------------------------------------------------------------------ creation / destruction
    /// instance_create(x, y, obj): uses the object's default depth.
    pub fn instance_create(&mut self, x: f64, y: f64, obj: Box<dyn Object>) -> Id {
        let name = obj.name();
        let depth = crate::objdata::obj_info(name).map(|i| i.depth).unwrap_or(0.0);
        self.instance_create_depth(x, y, depth, obj)
    }
    /// instance_create_depth(x, y, depth, obj). Runs the Create event immediately.
    pub fn instance_create_depth(&mut self, x: f64, y: f64, depth: f64, mut obj: Box<dyn Object>) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        let name = obj.name();
        let mut inst = Inst::new(id, name, x, y, depth);
        if let Some(info) = crate::objdata::obj_info(name) {
            if !info.sprite.is_empty() {
                inst.sprite_index = spr(info.sprite);
            }
            if !info.mask.is_empty() {
                inst.mask_index = spr(info.mask);
            }
            inst.visible = info.visible;
        }
        self.exec.push((id, name));
        obj.create(&mut inst, self);
        self.exec.pop();
        let dead = self.dying.remove(&id) || inst.destroyed;
        if dead {
            inst.destroyed = true;
            obj.destroy(&mut inst, self);
            obj.cleanup(&mut inst, self);
            return id;
        }
        self.slots.push(Some(Slot { inst, obj }));
        self.index.insert(id, self.slots.len() - 1);
        id
    }

    /// instance_destroy() on yourself, from inside your own event.
    pub fn destroy_self(&mut self, me: &mut Inst) {
        if !me.destroyed {
            me.destroyed = true;
            self.dying.insert(me.id);
        }
    }
    /// instance_destroy(id) on another instance (runs its Destroy + CleanUp events now).
    pub fn destroy(&mut self, id: Id) {
        if self.exec.iter().any(|e| e.0 == id) {
            self.dying.insert(id);
            return;
        }
        let Some(&i) = self.index.get(&id) else { return };
        let Some(mut slot) = self.slots[i].take() else { return };
        if slot.inst.destroyed {
            self.slots[i] = Some(slot);
            return;
        }
        slot.inst.destroyed = true;
        self.exec.push((id, slot.inst.object));
        slot.obj.destroy(&mut slot.inst, self);
        slot.obj.cleanup(&mut slot.inst, self);
        self.exec.pop();
        self.dying.remove(&id);
        self.slots[i] = Some(slot);
    }
    /// `with (obj) instance_destroy();`
    pub fn destroy_all(&mut self, obj: &str) {
        for id in self.ids_of(obj) {
            self.destroy(id);
        }
    }

    fn live(&self, s: &Slot) -> bool { !s.inst.destroyed && !self.dying.contains(&s.inst.id) }

    // ------------------------------------------------------------------ queries
    /// All live instances of `obj` (or its children), in creation order. Includes executing ones.
    pub fn ids_of(&mut self, obj: &str) -> Vec<Id> {
        let mut out = vec![];
        let mut cand: Vec<(Id, &'static str)> = vec![];
        for s in self.slots.iter().flatten() {
            if self.live(s) {
                cand.push((s.inst.id, s.inst.object));
            }
        }
        for &(id, name) in &self.exec {
            if !self.dying.contains(&id) && !cand.iter().any(|c| c.0 == id) {
                cand.push((id, name));
            }
        }
        cand.sort_by_key(|c| c.0);
        for (id, name) in cand {
            if self.is_a(name, obj) {
                out.push(id);
            }
        }
        out
    }
    pub fn exists(&mut self, obj: &str) -> bool { !self.ids_of(obj).is_empty() }
    /// i_ex / instance_exists for an id.
    pub fn id_exists(&self, id: Id) -> bool {
        if self.dying.contains(&id) {
            return false;
        }
        if self.exec.iter().any(|e| e.0 == id) {
            return true;
        }
        match self.index.get(&id) {
            Some(&i) => self.slots[i].as_ref().map(|s| !s.inst.destroyed).unwrap_or(false),
            None => false,
        }
    }
    pub fn instance_number(&mut self, obj: &str) -> usize { self.ids_of(obj).len() }
    /// First instance of `obj` (what `obj_name.x` refers to in GML).
    pub fn first(&mut self, obj: &str) -> Option<Id> { self.ids_of(obj).first().copied() }

    /// Builtin vars of an instance. None if destroyed or currently executing (use `me` instead).
    pub fn inst(&self, id: Id) -> Option<&Inst> {
        let &i = self.index.get(&id)?;
        let s = self.slots[i].as_ref()?;
        if self.live(s) { Some(&s.inst) } else { None }
    }
    pub fn inst_mut(&mut self, id: Id) -> Option<&mut Inst> {
        let &i = self.index.get(&id)?;
        let dying = self.dying.contains(&id);
        let s = self.slots[i].as_mut()?;
        if s.inst.destroyed || dying { None } else { Some(&mut s.inst) }
    }
    /// Builtin vars of the first instance of `obj`.
    pub fn first_inst(&mut self, obj: &str) -> Option<&mut Inst> {
        let id = self.first(obj)?;
        self.inst_mut(id)
    }
    /// Typed access to an instance's object struct and builtins.
    pub fn get<T: 'static>(&mut self, id: Id) -> Option<(&mut Inst, &mut T)> {
        let &i = self.index.get(&id)?;
        let dying = self.dying.contains(&id);
        let s = self.slots[i].as_mut()?;
        if s.inst.destroyed || dying {
            return None;
        }
        let o = s.obj.as_any().downcast_mut::<T>()?;
        Some((&mut s.inst, o))
    }
    /// Typed access to the first instance of `obj`.
    pub fn get_first<T: 'static>(&mut self, obj: &str) -> Option<(&mut Inst, &mut T)> {
        let id = self.first(obj)?;
        self.get::<T>(id)
    }

    /// `with (id) { ... }` — run code as another instance, with full game access.
    pub fn with<R>(&mut self, id: Id, f: impl FnOnce(&mut Inst, &mut dyn Object, &mut Game) -> R) -> Option<R> {
        let &i = self.index.get(&id)?;
        let mut slot = self.slots[i].take()?;
        if !self.live(&slot) {
            self.slots[i] = Some(slot);
            return None;
        }
        self.exec.push((id, slot.inst.object));
        let r = f(&mut slot.inst, slot.obj.as_mut(), self);
        self.exec.pop();
        self.finish_event(&mut slot);
        self.slots[i] = Some(slot);
        Some(r)
    }
    /// Typed `with`.
    pub fn with_t<T: 'static, R>(&mut self, id: Id, f: impl FnOnce(&mut Inst, &mut T, &mut Game) -> R) -> Option<R> {
        let &i = self.index.get(&id)?;
        let mut slot = self.slots[i].take()?;
        if !self.live(&slot) || slot.obj.as_any().downcast_mut::<T>().is_none() {
            self.slots[i] = Some(slot);
            return None;
        }
        self.exec.push((id, slot.inst.object));
        let r = {
            let Slot { inst, obj } = &mut slot;
            f(inst, obj.as_any().downcast_mut::<T>().unwrap(), self)
        };
        self.exec.pop();
        self.finish_event(&mut slot);
        self.slots[i] = Some(slot);
        Some(r)
    }
    /// `with (obj) { ... }` over every instance.
    pub fn with_all(&mut self, obj: &str, mut f: impl FnMut(&mut Inst, &mut dyn Object, &mut Game)) {
        for id in self.ids_of(obj) {
            self.with(id, |a, b, c| f(a, b, c));
        }
    }
    /// event_user(n) on another instance.
    pub fn user_event(&mut self, id: Id, n: usize) { self.with(id, |me, o, g| o.user(n, me, g)); }

    fn finish_event(&mut self, slot: &mut Slot) {
        let id = slot.inst.id;
        if self.dying.remove(&id) {
            slot.inst.destroyed = true;
            self.exec.push((id, slot.inst.object));
            slot.obj.destroy(&mut slot.inst, self);
            slot.obj.cleanup(&mut slot.inst, self);
            self.exec.pop();
        }
    }

    /// Generic variable read by name (builtin or custom via `var`).
    pub fn get_var(&mut self, id: Id, name: &str) -> Option<f64> {
        let &i = self.index.get(&id)?;
        let s = self.slots[i].as_mut()?;
        if let Some(v) = s.inst.get_motion_var(name) {
            return Some(v);
        }
        if let Some(v) = s.inst.builtin_var(name) {
            return Some(*v);
        }
        s.obj.var(name).map(|v| *v)
    }
    pub fn set_var(&mut self, id: Id, name: &str, v: f64) -> bool {
        let Some(&i) = self.index.get(&id) else { return false };
        let Some(s) = self.slots[i].as_mut() else { return false };
        if s.inst.set_motion_var(name, v) {
            return true;
        }
        if let Some(p) = s.inst.builtin_var(name) {
            *p = v;
            return true;
        }
        if let Some(p) = s.obj.var(name) {
            *p = v;
            return true;
        }
        crate::log(&format!("set_var: {} has no var {}", s.inst.object, name));
        false
    }

    // ------------------------------------------------------------------ helpers mirroring Deltarune scripts
    /// scr_lerpvar_instance(target, varname, a, b, time, [easetype, easeinout])
    pub fn lerpvar_instance(&mut self, target: Id, var: &str, a: f64, b: f64, time: f64, ease: i32, inout: &str) -> Id {
        let lv = LerpVar {
            target,
            varname: var.to_string(),
            pointa: Some(a),
            pointb: b,
            time: 0.0,
            maxtime: time,
            easetype: ease,
            easeinout: inout.to_string(),
            init: false,
        };
        self.instance_create(0.0, 0.0, Box::new(lv))
    }
    /// scr_lerpvar on `me`: `g.lerpvar(me, "image_alpha", 0., 1., 10., 0, "out")`.
    /// NOTE: while `me` is executing, the lerpvar only starts writing next frame (like GML).
    pub fn lerpvar(&mut self, me: &Inst, var: &str, a: f64, b: f64, time: f64, ease: i32, inout: &str) -> Id {
        self.lerpvar_instance(me.id, var, a, b, time, ease, inout)
    }
    /// scr_lerpvar starting from the target's current value (pointa = "" in GML).
    pub fn lerpvar_from_current(&mut self, target: Id, var: &str, b: f64, time: f64, ease: i32, inout: &str) -> Id {
        let lv = LerpVar {
            target,
            varname: var.to_string(),
            pointa: None,
            pointb: b,
            time: 0.0,
            maxtime: time,
            easetype: ease,
            easeinout: inout.to_string(),
            init: false,
        };
        self.instance_create(0.0, 0.0, Box::new(lv))
    }
    /// scr_script_delayed(script, delay, ...): run `f` after `delay` frames if `target` still exists.
    pub fn script_delayed(&mut self, target: Id, delay: i32, f: impl FnOnce(&mut Game) + 'static) -> Id {
        let id = self.instance_create(0.0, 0.0, Box::new(ScriptDelayed { target, f: Some(Box::new(f)) }));
        if let Some(i) = self.inst_mut(id) {
            i.alarm[0] = delay.max(1);
        }
        id
    }
    /// scr_var_delayed("var", value, delay) on target.
    pub fn var_delayed(&mut self, target: Id, var: &'static str, v: f64, delay: i32) -> Id {
        self.script_delayed(target, delay, move |g| {
            g.set_var(target, var, v);
        })
    }
    /// scr_doom(target, frames): destroy target after N frames.
    pub fn doom(&mut self, target: Id, frames: i32) {
        self.script_delayed(target, frames, move |g| g.destroy(target));
    }
    /// scr_afterimage(): copy of `me` that fades out (obj_afterimage).
    pub fn afterimage(&mut self, me: &Inst) -> Id {
        let id = self.instance_create(me.x, me.y, Box::new(Afterimage::default()));
        if let Some(a) = self.inst_mut(id) {
            a.sprite_index = me.sprite_index;
            a.image_index = me.image_index;
            a.image_blend = me.image_blend;
            a.image_speed = 0.0;
            a.depth = me.depth;
            a.image_xscale = me.image_xscale;
            a.image_yscale = me.image_yscale;
            a.image_angle = me.image_angle;
        }
        id
    }
    /// scr_marker(x, y, sprite)
    pub fn marker(&mut self, x: f64, y: f64, sprite: Spr) -> Id {
        let id = self.instance_create(x, y, Box::new(Marker));
        if let Some(m) = self.inst_mut(id) {
            m.sprite_index = sprite;
            m.image_speed = 0.0;
        }
        id
    }

    // ------------------------------------------------------------------ random
    pub fn random(&mut self, n: f64) -> f64 { self.rng.random(n) }
    pub fn random_range(&mut self, a: f64, b: f64) -> f64 { self.rng.random_range(a, b) }
    pub fn irandom(&mut self, n: f64) -> f64 { self.rng.irandom(n) }
    pub fn irandom_range(&mut self, a: f64, b: f64) -> f64 { self.rng.irandom_range(a, b) }
    pub fn choose<T: Copy>(&mut self, o: &[T]) -> T { self.rng.choose(o) }

    // ------------------------------------------------------------------ view
    pub fn camerax(&self) -> f64 { self.gfx.view_x }
    pub fn cameray(&self) -> f64 { self.gfx.view_y }
    pub fn camerawidth(&self) -> f64 { 640.0 }
    pub fn cameraheight(&self) -> f64 { 480.0 }

    // ------------------------------------------------------------------ sprites
    pub fn sprite(&self, s: Spr) -> Option<&Sprite> { self.assets.get(s) }
    pub fn sprite_get_width(&self, s: Spr) -> f64 { self.sprite(s).map(|s| s.w).unwrap_or(0.0) }
    pub fn sprite_get_height(&self, s: Spr) -> f64 { self.sprite(s).map(|s| s.h).unwrap_or(0.0) }
    pub fn sprite_get_xoffset(&self, s: Spr) -> f64 { self.sprite(s).map(|s| s.ox).unwrap_or(0.0) }
    pub fn sprite_get_yoffset(&self, s: Spr) -> f64 { self.sprite(s).map(|s| s.oy).unwrap_or(0.0) }
    pub fn sprite_get_number(&self, s: Spr) -> f64 { self.sprite(s).map(|s| s.frame_count() as f64).unwrap_or(0.0) }
    pub fn sprite_get_bbox_left(&self, s: Spr) -> f64 { self.sprite(s).map(|s| s.bl).unwrap_or(0.0) }
    pub fn sprite_get_bbox_right(&self, s: Spr) -> f64 { self.sprite(s).map(|s| s.br).unwrap_or(0.0) }
    pub fn sprite_get_bbox_top(&self, s: Spr) -> f64 { self.sprite(s).map(|s| s.bt).unwrap_or(0.0) }
    pub fn sprite_get_bbox_bottom(&self, s: Spr) -> f64 { self.sprite(s).map(|s| s.bb).unwrap_or(0.0) }
    /// sprite_width (scaled)
    pub fn sprite_width(&self, me: &Inst) -> f64 { self.sprite_get_width(me.sprite_index) * me.image_xscale }
    pub fn sprite_height(&self, me: &Inst) -> f64 { self.sprite_get_height(me.sprite_index) * me.image_yscale }

    /// sprite_create_from_surface(surf, x, y, w, h, removeback, smooth, xorig, yorig)
    pub fn sprite_create_from_surface(&mut self, surf: i32, x: f64, y: f64, w: f64, h: f64, xorig: f64, yorig: f64) -> Spr {
        self.sprite_create_from_surface_ext(surf, x, y, w, h, false, xorig, yorig)
    }
    /// sprite_create_from_surface with `removeback`: pixels matching the bottom-left pixel's colour become transparent.
    pub fn sprite_create_from_surface_ext(&mut self, surf: i32, x: f64, y: f64, w: f64, h: f64, removeback: bool, xorig: f64, yorig: f64) -> Spr {
        let ns = self.gfx.surface_create(w as i32, h as i32);
        self.gfx.surface_set_target(ns);
        self.gfx.draw_clear_alpha(0, 0.0);
        self.gfx.gpu_set_blendenable(false);
        self.gfx.draw_surface_part_ext(surf, x, y, w, h, 0.0, 0.0, 1.0, 1.0, C_WHITE, 1.0);
        self.gfx.gpu_set_blendenable(true);
        self.gfx.surface_reset_target();
        let tex = if removeback {
            let (iw, ih) = (w.max(1.0) as i32, h.max(1.0) as i32);
            let mut px = vec![0u8; (iw * ih * 4) as usize];
            self.gfx.flush();
            let gl = &self.gfx.gl;
            gl.bind_framebuffer(web_sys::WebGl2RenderingContext::FRAMEBUFFER, Some(&self.gfx.surfaces[ns as usize].fb));
            let _ = gl.read_pixels_with_opt_u8_array(0, 0, iw, ih, web_sys::WebGl2RenderingContext::RGBA, web_sys::WebGl2RenderingContext::UNSIGNED_BYTE, Some(&mut px));
            // FBO row 0 is the surface's top row (y-up projection), so the bottom-left pixel is the last row
            let bl = ((ih - 1) * iw * 4) as usize;
            let key = [px[bl], px[bl + 1], px[bl + 2]];
            for p in px.chunks_mut(4) {
                if p[0] == key[0] && p[1] == key[1] && p[2] == key[2] {
                    p[3] = 0;
                }
            }
            self.gfx.surface_free(ns);
            self.gfx.surface_set_target(self.gfx.app_surface);
            self.gfx.surface_reset_target();
            self.gfx.add_texture_rgba(iw as u32, ih as u32, &px)
        } else {
            let t = self.gfx.surface_tex(ns).unwrap();
            // keep the texture alive: detach from the surface table
            self.gfx.surfaces[ns as usize].alive = false;
            t
        };
        let id = self.assets.sprites.len() as Spr;
        self.assets.sprites.push(Sprite {
            name: format!("__dyn{id}"),
            w,
            h,
            ox: xorig,
            oy: yorig,
            bl: 0.0,
            br: w - 1.0,
            bt: 0.0,
            bb: h - 1.0,
            kind: 0,
            speed: 1.0,
            speedtype: 1,
            frames: vec![Some(Frame { tex, x: 0.0, y: 0.0, w: w as f32, h: h as f32, offx: 0.0, offy: 0.0 })],
            masks: vec![],
            mask_w: w as usize,
            mask_h: h as usize,
        });
        id
    }
    pub fn sprite_delete(&mut self, s: Spr) {
        if let Some(sp) = self.assets.sprites.get_mut(s as usize) {
            if sp.name.starts_with("__dyn") {
                if let Some(Some(f)) = sp.frames.first() {
                    let t = f.tex;
                    self.gfx.flush();
                    self.gfx.gl.delete_texture(Some(&self.gfx.textures[t].tex));
                }
                sp.frames.clear();
            }
        }
    }

    // ------------------------------------------------------------------ drawing
    fn frame_of(&self, s: Spr, sub: f64) -> Option<(Frame, f64, f64)> {
        let sp = self.sprite(s)?;
        let n = sp.frames.len();
        if n == 0 {
            return None;
        }
        let i = (sub.floor() as i64).rem_euclid(n as i64) as usize;
        let f = sp.frames[i]?;
        Some((f, sp.ox, sp.oy))
    }

    pub fn draw_sprite_ext(&mut self, s: Spr, sub: f64, x: f64, y: f64, xs: f64, ys: f64, rot: f64, col: Color, alpha: f64) {
        let Some((f, ox, oy)) = self.frame_of(s, sub) else { return };
        let (sn, cs) = (rot.to_radians().sin(), rot.to_radians().cos());
        let tf = |lx: f64, ly: f64| -> [f32; 2] {
            let lx = (lx - ox) * xs;
            let ly = (ly - oy) * ys;
            [(x + lx * cs + ly * sn) as f32, (y - lx * sn + ly * cs) as f32]
        };
        let (l, t) = (f.offx as f64, f.offy as f64);
        let (r, b) = (l + f.w as f64, t + f.h as f64);
        let tex = &self.gfx.textures[f.tex];
        let (tw, th) = (tex.w as f32, tex.h as f32);
        let (u0, v0, u1, v1) = (f.x / tw, f.y / th, (f.x + f.w) / tw, (f.y + f.h) / th);
        let c = rgba(col, alpha);
        self.gfx.quad(f.tex, [tf(l, t), tf(r, t), tf(r, b), tf(l, b)], [[u0, v0], [u1, v0], [u1, v1], [u0, v1]], [c; 4]);
    }
    pub fn draw_sprite(&mut self, s: Spr, sub: f64, x: f64, y: f64) {
        let a = self.gfx.draw_alpha;
        self.draw_sprite_ext(s, sub, x, y, 1.0, 1.0, 0.0, C_WHITE, a);
    }
    /// draw_self()
    pub fn draw_self(&mut self, me: &Inst) {
        self.draw_sprite_ext(me.sprite_index, me.image_index, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, me.image_alpha);
    }
    /// draw_sprite_general: part of a frame (in untrimmed frame coords), transformed. Origin ignored.
    pub fn draw_sprite_general(
        &mut self, s: Spr, sub: f64, left: f64, top: f64, w: f64, h: f64, x: f64, y: f64, xs: f64, ys: f64, rot: f64, cols: [Color; 4], alpha: f64,
    ) {
        let Some((f, _, _)) = self.frame_of(s, sub) else { return };
        // clip requested rect against the trimmed frame
        let (fl, ft) = (f.offx as f64, f.offy as f64);
        let (fr, fb) = (fl + f.w as f64, ft + f.h as f64);
        let l = left.max(fl);
        let t = top.max(ft);
        let r = (left + w).min(fr);
        let b = (top + h).min(fb);
        if r <= l || b <= t {
            return;
        }
        let (sn, cs) = (rot.to_radians().sin(), rot.to_radians().cos());
        let tf = |lx: f64, ly: f64| -> [f32; 2] {
            let lx = (lx - left) * xs;
            let ly = (ly - top) * ys;
            [(x + lx * cs + ly * sn) as f32, (y - lx * sn + ly * cs) as f32]
        };
        let tex = &self.gfx.textures[f.tex];
        let (tw, th) = (tex.w as f64, tex.h as f64);
        let u = |px: f64| ((f.x as f64 + px - fl) / tw) as f32;
        let v = |py: f64| ((f.y as f64 + py - ft) / th) as f32;
        let lerpc = |fx: f64, fy: f64| -> [f32; 4] {
            // bilinear colour across the requested rect
            let c0 = rgba(cols[0], alpha);
            let c1 = rgba(cols[1], alpha);
            let c2 = rgba(cols[2], alpha);
            let c3 = rgba(cols[3], alpha);
            let fx = fx as f32;
            let fy = fy as f32;
            let mut o = [0.0; 4];
            for k in 0..4 {
                let top = c0[k] + (c1[k] - c0[k]) * fx;
                let bot = c3[k] + (c2[k] - c3[k]) * fx;
                o[k] = top + (bot - top) * fy;
            }
            o
        };
        let fx = |px: f64| if w > 0.0 { (px - left) / w } else { 0.0 };
        let fy = |py: f64| if h > 0.0 { (py - top) / h } else { 0.0 };
        self.gfx.quad(
            f.tex,
            [tf(l, t), tf(r, t), tf(r, b), tf(l, b)],
            [[u(l), v(t)], [u(r), v(t)], [u(r), v(b)], [u(l), v(b)]],
            [lerpc(fx(l), fy(t)), lerpc(fx(r), fy(t)), lerpc(fx(r), fy(b)), lerpc(fx(l), fy(b))],
        );
    }
    pub fn draw_sprite_part_ext(&mut self, s: Spr, sub: f64, l: f64, t: f64, w: f64, h: f64, x: f64, y: f64, xs: f64, ys: f64, col: Color, alpha: f64) {
        self.draw_sprite_general(s, sub, l, t, w, h, x, y, xs, ys, 0.0, [col; 4], alpha);
    }
    pub fn draw_sprite_part(&mut self, s: Spr, sub: f64, l: f64, t: f64, w: f64, h: f64, x: f64, y: f64) {
        let a = self.gfx.draw_alpha;
        self.draw_sprite_general(s, sub, l, t, w, h, x, y, 1.0, 1.0, 0.0, [C_WHITE; 4], a);
    }
    pub fn draw_sprite_stretched_ext(&mut self, s: Spr, sub: f64, x: f64, y: f64, w: f64, h: f64, col: Color, alpha: f64) {
        let (sw, sh) = (self.sprite_get_width(s), self.sprite_get_height(s));
        if sw <= 0.0 || sh <= 0.0 {
            return;
        }
        self.draw_sprite_general(s, sub, 0.0, 0.0, sw, sh, x, y, w / sw, h / sh, 0.0, [col; 4], alpha);
    }
    pub fn draw_sprite_stretched(&mut self, s: Spr, sub: f64, x: f64, y: f64, w: f64, h: f64) {
        let a = self.gfx.draw_alpha;
        self.draw_sprite_stretched_ext(s, sub, x, y, w, h, C_WHITE, a);
    }
    /// draw_sprite_tiled_ext: tiles over the whole current view / render target.
    pub fn draw_sprite_tiled_ext(&mut self, s: Spr, sub: f64, x: f64, y: f64, xs: f64, ys: f64, col: Color, alpha: f64) {
        let (sw, sh) = (self.sprite_get_width(s) * xs, self.sprite_get_height(s) * ys);
        if sw.abs() < 1.0 || sh.abs() < 1.0 {
            return;
        }
        let (ox, oy) = (self.sprite_get_xoffset(s) * xs, self.sprite_get_yoffset(s) * ys);
        let (vx, vy, vw, vh) = (self.gfx.view_x, self.gfx.view_y, self.gfx.tw as f64, self.gfx.th as f64);
        let sx = x - ox + ((vx - (x - ox)) / sw).floor() * sw;
        let sy = y - oy + ((vy - (y - oy)) / sh).floor() * sh;
        let mut yy = sy;
        while yy < vy + vh {
            let mut xx = sx;
            while xx < vx + vw {
                self.draw_sprite_ext(s, sub, xx + ox, yy + oy, xs, ys, 0.0, col, alpha);
                xx += sw.abs();
            }
            yy += sh.abs();
        }
    }
    pub fn draw_sprite_tiled(&mut self, s: Spr, sub: f64, x: f64, y: f64) {
        let a = self.gfx.draw_alpha;
        self.draw_sprite_tiled_ext(s, sub, x, y, 1.0, 1.0, C_WHITE, a);
    }

    // ------------------------------------------------------------------ text
    pub fn draw_set_font(&mut self, f: FontId) { self.font = f; }
    pub fn draw_set_halign(&mut self, a: HAlign) { self.halign = a; }
    pub fn draw_set_valign(&mut self, a: VAlign) { self.valign = a; }
    pub fn string_width(&self, s: &str) -> f64 {
        let Some(f) = self.assets.fonts.get(self.font as usize) else { return 0.0 };
        s.split('\n')
            .map(|l| l.chars().map(|c| f.glyphs.get(&(c as u32)).map(|g| g.shift as f64).unwrap_or(0.0)).sum::<f64>())
            .fold(0.0, f64::max)
    }
    pub fn string_height(&self, s: &str) -> f64 {
        let Some(f) = self.assets.fonts.get(self.font as usize) else { return 0.0 };
        f.line_h as f64 * (s.split('\n').count() as f64)
    }
    pub fn draw_text_transformed_color(&mut self, x: f64, y: f64, s: &str, xs: f64, ys: f64, rot: f64, c: [Color; 4], alpha: f64) {
        let Some(font) = self.assets.fonts.get(self.font as usize) else { return };
        let tex = font.tex;
        let lh = font.line_h as f64;
        let (tw, th) = (self.gfx.textures[tex].w as f32, self.gfx.textures[tex].h as f32);
        let lines: Vec<&str> = s.split('\n').collect();
        let total_h = lh * lines.len() as f64;
        let y0 = match self.valign {
            VAlign::Top => 0.0,
            VAlign::Middle => -total_h / 2.0,
            VAlign::Bottom => -total_h,
        };
        let (sn, cs) = (rot.to_radians().sin(), rot.to_radians().cos());
        let mut quads = vec![];
        for (li, line) in lines.iter().enumerate() {
            let lw: f64 = line.chars().map(|ch| font.glyphs.get(&(ch as u32)).map(|g| g.shift as f64).unwrap_or(0.0)).sum();
            let mut cx = match self.halign {
                HAlign::Left => 0.0,
                HAlign::Center => -(lw / 2.0).floor(),
                HAlign::Right => -lw,
            };
            let cy = y0 + li as f64 * lh;
            for ch in line.chars() {
                if let Some(gl) = font.glyphs.get(&(ch as u32)) {
                    let l = cx + gl.offset as f64;
                    let t = cy;
                    let r = l + gl.w as f64;
                    let b = t + gl.h as f64;
                    let tf = |lx: f64, ly: f64| -> [f32; 2] {
                        let lx = lx * xs;
                        let ly = ly * ys;
                        [(x + lx * cs + ly * sn) as f32, (y - lx * sn + ly * cs) as f32]
                    };
                    let uv = [
                        [gl.x / tw, gl.y / th],
                        [(gl.x + gl.w) / tw, gl.y / th],
                        [(gl.x + gl.w) / tw, (gl.y + gl.h) / th],
                        [gl.x / tw, (gl.y + gl.h) / th],
                    ];
                    quads.push(([tf(l, t), tf(r, t), tf(r, b), tf(l, b)], uv));
                    cx += gl.shift as f64;
                }
            }
        }
        let cc = [rgba(c[0], alpha), rgba(c[1], alpha), rgba(c[2], alpha), rgba(c[3], alpha)];
        for (p, uv) in quads {
            self.gfx.quad(tex, p, uv, cc);
        }
    }
    pub fn draw_text(&mut self, x: f64, y: f64, s: &str) {
        let (c, a) = (self.gfx.draw_color, self.gfx.draw_alpha);
        self.draw_text_transformed_color(x, y, s, 1.0, 1.0, 0.0, [c; 4], a);
    }
    pub fn draw_text_transformed(&mut self, x: f64, y: f64, s: &str, xs: f64, ys: f64, rot: f64) {
        let (c, a) = (self.gfx.draw_color, self.gfx.draw_alpha);
        self.draw_text_transformed_color(x, y, s, xs, ys, rot, [c; 4], a);
    }
    pub fn draw_text_color(&mut self, x: f64, y: f64, s: &str, c1: Color, c2: Color, c3: Color, c4: Color, alpha: f64) {
        self.draw_text_transformed_color(x, y, s, 1.0, 1.0, 0.0, [c1, c2, c3, c4], alpha);
    }

    // ------------------------------------------------------------------ collision
    /// Collision mask sprite of an instance (mask_index, else sprite_index).
    pub fn mask_of(&self, i: &Inst) -> Spr { if i.mask_index >= 0 { i.mask_index } else { i.sprite_index } }

    /// bbox_left/top/right/bottom (inclusive) of an instance placed at (x, y).
    pub fn bbox_at(&self, i: &Inst, x: f64, y: f64) -> Option<[f64; 4]> {
        let sp = self.sprite(self.mask_of(i))?;
        if sp.frames.is_empty() && sp.masks.is_empty() {
            return None;
        }
        let corners = [(sp.bl, sp.bt), (sp.br + 1.0, sp.bt), (sp.br + 1.0, sp.bb + 1.0), (sp.bl, sp.bb + 1.0)];
        let (sn, cs) = (i.image_angle.to_radians().sin(), i.image_angle.to_radians().cos());
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (lx, ly) in corners {
            let lx = (lx - sp.ox) * i.image_xscale;
            let ly = (ly - sp.oy) * i.image_yscale;
            let wx = x + lx * cs + ly * sn;
            let wy = y - lx * sn + ly * cs;
            x0 = x0.min(wx);
            y0 = y0.min(wy);
            x1 = x1.max(wx);
            y1 = y1.max(wy);
        }
        Some([x0.round(), y0.round(), x1.round() - 1.0, y1.round() - 1.0])
    }
    pub fn bbox(&self, i: &Inst) -> Option<[f64; 4]> { self.bbox_at(i, i.x, i.y) }

    /// Does instance `i` placed at (x, y) cover world point (px, py)?
    fn covers(&self, i: &Inst, x: f64, y: f64, px: f64, py: f64, bb: &[f64; 4]) -> bool {
        let Some(sp) = self.sprite(self.mask_of(i)) else { return false };
        if sp.kind == 0 {
            return px >= bb[0] && px <= bb[2] + 1.0 && py >= bb[1] && py <= bb[3] + 1.0;
        }
        let (sn, cs) = (i.image_angle.to_radians().sin(), i.image_angle.to_radians().cos());
        let dx = px - x;
        let dy = py - y;
        // inverse of draw transform
        let lx = dx * cs - dy * sn;
        let ly = dx * sn + dy * cs;
        if i.image_xscale == 0.0 || i.image_yscale == 0.0 {
            return false;
        }
        let lx = lx / i.image_xscale + sp.ox;
        let ly = ly / i.image_yscale + sp.oy;
        let (mx, my) = (lx.floor() as i64, ly.floor() as i64);
        if sp.kind == 2 || sp.masks.is_empty() {
            return mx as f64 >= sp.bl && mx as f64 <= sp.br && my as f64 >= sp.bt && my as f64 <= sp.bb;
        }
        let n = sp.frame_count() as i64;
        let fi = (i.image_index.floor() as i64).rem_euclid(n.max(1)) as usize;
        sp.mask_at(fi, mx, my)
    }

    /// Does instance `i` (e.g. the running `me`) cover world point (px, py)? Uses its real collision mask.
    pub fn point_in_inst(&self, i: &Inst, px: f64, py: f64) -> bool {
        match self.bbox(i) {
            Some(bb) => px >= bb[0] && px <= bb[2] + 1.0 && py >= bb[1] && py <= bb[3] + 1.0 && self.covers(i, i.x, i.y, px, py, &bb),
            None => false,
        }
    }

    /// Collision between `a` placed at (ax, ay) and instance `b`.
    pub fn collide(&self, a: &Inst, ax: f64, ay: f64, b: &Inst) -> bool {
        let (Some(ba), Some(bb)) = (self.bbox_at(a, ax, ay), self.bbox(b)) else { return false };
        let l = ba[0].max(bb[0]);
        let t = ba[1].max(bb[1]);
        let r = ba[2].min(bb[2]);
        let btm = ba[3].min(bb[3]);
        if l > r || t > btm {
            return false;
        }
        let ka = self.sprite(self.mask_of(a)).map(|s| s.kind).unwrap_or(0);
        let kb = self.sprite(self.mask_of(b)).map(|s| s.kind).unwrap_or(0);
        if ka == 0 && kb == 0 {
            return true;
        }
        let mut yy = t;
        while yy <= btm {
            let mut xx = l;
            while xx <= r {
                let (px, py) = (xx + 0.5, yy + 0.5);
                if self.covers(a, ax, ay, px, py, &ba) && self.covers(b, b.x, b.y, px, py, &bb) {
                    return true;
                }
                xx += 1.0;
            }
            yy += 1.0;
        }
        false
    }

    /// place_meeting(x, y, obj) for `me`.
    pub fn place_meeting(&mut self, me: &Inst, x: f64, y: f64, obj: &str) -> bool { self.instance_place(me, x, y, obj).is_some() }
    /// instance_place(x, y, obj)
    pub fn instance_place(&mut self, me: &Inst, x: f64, y: f64, obj: &str) -> Option<Id> {
        for id in self.ids_of(obj) {
            if id == me.id {
                continue;
            }
            if let Some(o) = self.inst(id) {
                if self.collide(me, x, y, o) {
                    return Some(id);
                }
            }
        }
        None
    }
    /// All instances of `obj` colliding with `me` (instance_place_list).
    pub fn instance_place_list(&mut self, me: &Inst, x: f64, y: f64, obj: &str) -> Vec<Id> {
        let mut out = vec![];
        for id in self.ids_of(obj) {
            if id == me.id {
                continue;
            }
            if let Some(o) = self.inst(id) {
                if self.collide(me, x, y, o) {
                    out.push(id);
                }
            }
        }
        out
    }
    /// collision_point(x, y, obj, prec, notme)
    pub fn collision_point(&mut self, px: f64, py: f64, obj: &str, notme: Option<Id>) -> Option<Id> {
        for id in self.ids_of(obj) {
            if Some(id) == notme {
                continue;
            }
            if let Some(o) = self.inst(id) {
                if let Some(bb) = self.bbox(o) {
                    if px >= bb[0] && px <= bb[2] + 1.0 && py >= bb[1] && py <= bb[3] + 1.0 && self.covers(o, o.x, o.y, px, py, &bb) {
                        return Some(id);
                    }
                }
            }
        }
        None
    }
    /// collision_rectangle(x1, y1, x2, y2, obj, prec, notme)
    pub fn collision_rectangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, obj: &str, notme: Option<Id>) -> Option<Id> {
        let (l, r) = (x1.min(x2), x1.max(x2));
        let (t, b) = (y1.min(y2), y1.max(y2));
        for id in self.ids_of(obj) {
            if Some(id) == notme {
                continue;
            }
            if let Some(o) = self.inst(id) {
                if let Some(bb) = self.bbox(o) {
                    let il = l.max(bb[0]);
                    let ir = r.min(bb[2]);
                    let it = t.max(bb[1]);
                    let ib = b.min(bb[3]);
                    if il > ir || it > ib {
                        continue;
                    }
                    let mut yy = it.floor();
                    'outer: while yy <= ib {
                        let mut xx = il.floor();
                        while xx <= ir {
                            if self.covers(o, o.x, o.y, xx + 0.5, yy + 0.5, &bb) {
                                return Some(id);
                            }
                            xx += 1.0;
                        }
                        yy += 1.0;
                        if yy > ib + 1.0 {
                            break 'outer;
                        }
                    }
                }
            }
        }
        None
    }
    /// collision_line(x1, y1, x2, y2, obj, prec, notme)
    pub fn collision_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, obj: &str, notme: Option<Id>) -> Option<Id> {
        let len = gm::point_distance(x1, y1, x2, y2).max(1.0);
        let steps = len.ceil() as i64;
        for id in self.ids_of(obj) {
            if Some(id) == notme {
                continue;
            }
            if let Some(o) = self.inst(id) {
                if let Some(bb) = self.bbox(o) {
                    for k in 0..=steps {
                        let t = k as f64 / steps as f64;
                        let px = x1 + (x2 - x1) * t;
                        let py = y1 + (y2 - y1) * t;
                        if px >= bb[0] && px <= bb[2] + 1.0 && py >= bb[1] && py <= bb[3] + 1.0 && self.covers(o, o.x, o.y, px, py, &bb) {
                            return Some(id);
                        }
                    }
                }
            }
        }
        None
    }
    /// collision_circle(x, y, r, obj, prec, notme)
    pub fn collision_circle(&mut self, cx: f64, cy: f64, rad: f64, obj: &str, notme: Option<Id>) -> Option<Id> {
        for id in self.ids_of(obj) {
            if Some(id) == notme {
                continue;
            }
            if let Some(o) = self.inst(id) {
                if let Some(bb) = self.bbox(o) {
                    let il = (cx - rad).max(bb[0]).floor();
                    let ir = (cx + rad).min(bb[2]);
                    let it = (cy - rad).max(bb[1]).floor();
                    let ib = (cy + rad).min(bb[3]);
                    let mut yy = it;
                    while yy <= ib {
                        let mut xx = il;
                        while xx <= ir {
                            let (px, py) = (xx + 0.5, yy + 0.5);
                            if (px - cx).powi(2) + (py - cy).powi(2) <= rad * rad && self.covers(o, o.x, o.y, px, py, &bb) {
                                return Some(id);
                            }
                            xx += 1.0;
                        }
                        yy += 1.0;
                    }
                }
            }
        }
        None
    }

    // ------------------------------------------------------------------ main loop
    fn snapshot(&self) -> Vec<usize> { (0..self.slots.len()).collect() }

    fn run_on(&mut self, i: usize, f: &mut dyn FnMut(&mut Slot, &mut Game)) {
        let Some(mut slot) = self.slots[i].take() else { return };
        if slot.inst.destroyed {
            self.slots[i] = Some(slot);
            return;
        }
        self.exec.push((slot.inst.id, slot.inst.object));
        f(&mut slot, self);
        self.exec.pop();
        self.finish_event(&mut slot);
        self.slots[i] = Some(slot);
    }

    /// One 30fps GameMaker frame of logic (everything except drawing).
    pub fn step(&mut self) {
        self.frame += 1;
        self.glob.time += 1.0;
        let order = self.snapshot();
        for &i in &order {
            if let Some(s) = self.slots[i].as_mut() {
                s.inst.xprevious = s.inst.x;
                s.inst.yprevious = s.inst.y;
            }
        }
        // GameMaker iterates the live instance list: instances created earlier in a pass also get that event.
        let mut i = 0;
        while i < self.slots.len() {
            self.run_on(i, &mut |s, g| s.obj.begin_step(&mut s.inst, g));
            i += 1;
        }
        let order = self.snapshot();
        for &i in &order {
            for a in 0..12 {
                let fire = match self.slots[i].as_mut() {
                    Some(s) if !s.inst.destroyed && s.inst.alarm[a] > 0 => {
                        s.inst.alarm[a] -= 1;
                        s.inst.alarm[a] == 0
                    }
                    _ => false,
                };
                if fire {
                    self.run_on(i, &mut |s, g| {
                        s.obj.alarm(a, &mut s.inst, g);
                        if s.inst.alarm[a] == 0 {
                            s.inst.alarm[a] = -1;
                        }
                    });
                }
            }
        }
        let mut i = 0;
        while i < self.slots.len() {
            self.run_on(i, &mut |s, g| s.obj.step(&mut s.inst, g));
            i += 1;
        }
        // motion + animation
        let order = self.snapshot();
        for &i in &order {
            let mut anim_end = false;
            if let Some(s) = self.slots[i].as_mut() {
                if s.inst.destroyed {
                    continue;
                }
                let me = &mut s.inst;
                if me.friction != 0.0 {
                    let ns = if me.speed > 0.0 { me.speed - me.friction } else { me.speed + me.friction };
                    if (me.speed > 0.0 && ns < 0.0) || (me.speed < 0.0 && ns > 0.0) {
                        me.set_speed(0.0);
                    } else if me.speed != 0.0 {
                        me.set_speed(ns);
                    }
                }
                if me.gravity != 0.0 {
                    me.hspeed += gm::lengthdir_x(me.gravity, me.gravity_direction);
                    me.vspeed += gm::lengthdir_y(me.gravity, me.gravity_direction);
                    me.from_components();
                }
                me.x += me.hspeed;
                me.y += me.vspeed;
                if let Some(sp) = self.assets.get(me.sprite_index) {
                    let n = sp.frame_count() as f64;
                    let factor = if sp.speedtype == 0 { sp.speed / 30.0 } else { sp.speed };
                    me.image_index += me.image_speed * factor;
                    if me.image_index >= n {
                        me.image_index -= n;
                        anim_end = true;
                    } else if me.image_index < 0.0 {
                        me.image_index += n;
                        anim_end = true;
                    }
                }
            }
            if anim_end {
                self.run_on(i, &mut |s, g| s.obj.animation_end(&mut s.inst, g));
            }
        }
        if let Some(h) = self.collision_hook {
            h(self);
        }
        let mut i = 0;
        while i < self.slots.len() {
            self.run_on(i, &mut |s, g| s.obj.end_step(&mut s.inst, g));
            i += 1;
        }
    }

    /// Draw all visible instances in depth order (higher depth first).
    pub fn draw(&mut self) {
        let mut order: Vec<(f64, Id, usize)> = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().filter(|s| !s.inst.destroyed && s.inst.visible).map(|s| (s.inst.depth, s.inst.id, i)))
            .collect();
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)));
        for &(_, _, i) in &order {
            self.run_on(i, &mut |s, g| {
                if s.inst.visible {
                    s.obj.draw(&mut s.inst, g)
                }
            });
        }
        for &(_, _, i) in &order {
            self.run_on(i, &mut |s, g| {
                if s.inst.visible {
                    s.obj.draw_end(&mut s.inst, g)
                }
            });
        }
        if self.show_hitboxes {
            self.draw_hitboxes();
        }
    }

    fn draw_hitboxes(&mut self) {
        let ids: Vec<Id> = self.slots.iter().flatten().filter(|s| !s.inst.destroyed).map(|s| s.inst.id).collect();
        for id in ids {
            let collide = {
                let Some(o) = self.inst(id) else { continue };
                let o = o.object;
                self.is_a(o, "obj_collidebullet") || o == "obj_heart"
            };
            if !collide {
                continue;
            }
            if let Some(bb) = self.inst(id).and_then(|i| self.bbox(i)) {
                self.gfx.draw_set_alpha(1.0);
                self.gfx.draw_rectangle_color(bb[0], bb[1], bb[2], bb[3], 0x00FF00, 0x00FF00, 0x00FF00, 0x00FF00, true);
            }
        }
    }

    /// Remove destroyed instances from storage (end of frame).
    pub fn purge(&mut self) {
        if self.slots.iter().all(|s| s.as_ref().map(|s| !s.inst.destroyed).unwrap_or(false)) {
            return;
        }
        let old = std::mem::take(&mut self.slots);
        self.index.clear();
        for s in old.into_iter().flatten() {
            if !s.inst.destroyed {
                self.index.insert(s.inst.id, self.slots.len());
                self.slots.push(Some(s));
            }
        }
    }

    /// Destroy everything (room end).
    pub fn clear_all(&mut self) {
        let ids: Vec<Id> = self.slots.iter().flatten().map(|s| s.inst.id).collect();
        for id in ids {
            self.with(id, |me, o, g| {
                me.destroyed = true;
                o.cleanup(me, g);
            });
        }
        self.slots.clear();
        self.index.clear();
        self.dying.clear();
    }

    // ------------------------------------------------------------------ sound shorthands
    pub fn snd_play(&mut self, s: &str) -> crate::audio::SndHandle { self.audio.play(s) }
    pub fn snd_play_x(&mut self, s: &str, vol: f64, pitch: f64) -> crate::audio::SndHandle { self.audio.play_x(s, vol, pitch) }
    pub fn snd_play_pitch(&mut self, s: &str, pitch: f64) -> crate::audio::SndHandle { self.audio.play_pitch(s, pitch) }
    pub fn snd_stop(&mut self, s: &str) { self.audio.stop(s) }
    pub fn snd_loop(&mut self, s: &str) -> crate::audio::SndHandle { self.audio.play_loop(s) }
}

pub fn rgba(color: Color, alpha: f64) -> [f32; 4] {
    [
        (color & 0xFF) as f32 / 255.0,
        ((color >> 8) & 0xFF) as f32 / 255.0,
        ((color >> 16) & 0xFF) as f32 / 255.0,
        alpha as f32,
    ]
}

// ============================================================================ builtin helper objects

/// obj_lerpvar
pub struct LerpVar {
    pub target: Id,
    pub varname: String,
    /// None = start from the target's current value
    pub pointa: Option<f64>,
    pub pointb: f64,
    pub time: f64,
    pub maxtime: f64,
    pub easetype: i32,
    pub easeinout: String,
    init: bool,
}
impl Object for LerpVar {
    fn name(&self) -> &'static str { "obj_lerpvar" }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if !g.id_exists(self.target) {
            g.destroy_self(me);
            return;
        }
        if !self.init {
            if self.pointa.is_none() {
                self.pointa = g.get_var(self.target, &self.varname);
            }
            self.init = true;
        }
        self.time += 1.0;
        let a = self.pointa.unwrap_or(0.0);
        let t = if self.maxtime > 0.0 { self.time / self.maxtime } else { 1.0 };
        let v = if self.easetype == 0 {
            gm::lerp(a, self.pointb, t)
        } else {
            match self.easeinout.as_str() {
                "in" => gm::lerp_ease_in(a, self.pointb, t, self.easetype),
                "inout" => gm::lerp_ease_inout(a, self.pointb, t, self.easetype),
                _ => gm::lerp_ease_out(a, self.pointb, t, self.easetype),
            }
        };
        let tgt = self.target;
        let name = self.varname.clone();
        g.set_var(tgt, &name, v);
        if self.time >= self.maxtime {
            g.destroy_self(me);
        }
    }
    fn draw(&mut self, _: &mut Inst, _: &mut Game) {}
    fn as_any(&mut self) -> &mut dyn Any { self }
}

/// obj_script_delayed
pub struct ScriptDelayed {
    pub target: Id,
    pub f: Option<Box<dyn FnOnce(&mut Game)>>,
}
impl Object for ScriptDelayed {
    fn name(&self) -> &'static str { "obj_script_delayed" }
    fn alarm(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        if n == 0 {
            if g.id_exists(self.target) || self.target == NOONE {
                if let Some(f) = self.f.take() {
                    f(g);
                }
            }
            g.destroy_self(me);
        }
    }
    fn draw(&mut self, _: &mut Inst, _: &mut Game) {}
    fn as_any(&mut self) -> &mut dyn Any { self }
}

/// obj_afterimage: fades by fadeSpeed each step.
pub struct Afterimage {
    pub fade_speed: f64,
}
impl Default for Afterimage {
    fn default() -> Self { Afterimage { fade_speed: 0.04 } }
}
impl Object for Afterimage {
    fn name(&self) -> &'static str { "obj_afterimage" }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        me.image_alpha -= self.fade_speed;
        if me.image_alpha < 0.0 {
            g.destroy_self(me);
        }
    }
    fn var(&mut self, name: &str) -> Option<&mut f64> {
        match name {
            "fadeSpeed" => Some(&mut self.fade_speed),
            _ => None,
        }
    }
    fn as_any(&mut self) -> &mut dyn Any { self }
}

/// obj_marker: a plain sprite.
pub struct Marker;
impl Object for Marker {
    fn name(&self) -> &'static str { "obj_marker" }
    fn as_any(&mut self) -> &mut dyn Any { self }
}
