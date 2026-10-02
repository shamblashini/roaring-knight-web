//! obj_knight_enemy (all events), its monster setup (scr_monstersetup type 104, encounter 115),
//! scr_enemy_object_init, scr_enemy_hurt, scr_enemy_drawidle_generic, and the small objects it creates:
//! obj_bgfountaintest, obj_block_vfx, obj_afterimage_fade_to_white, obj_afterimage_grow,
//! obj_fadeout (scr_fadeout), obj_battle_marker (scr_battle_marker / scr_guardpeek).
//! OWNER: battle-core agent "knight".

use crate::assets::{spr, Spr, NO_SPR};
use crate::battle::controller::{self, BattleController};
use crate::battle::heroes::Hero;
use crate::battle::soul::{self, Darkener, Growtangle, Heart, Shake};
use crate::battle::writer::{msgnext, msgset, scr_anyface_next, scr_battletext_default, scr_enemyblcon, scr_speaker, scr_terminate_writer};
use crate::gfx::{make_color_hsv, make_color_rgb, merge_color, C_BLACK, C_PURPLE, C_WHITE};
use crate::gm::point_distance;
use crate::input::Key;
use crate::obj_vars;
use crate::rt::{Afterimage, Game, Id, Inst, Object, NOONE};

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
    // ---- added by the knight port
    pub fsiner: f64,
    pub fatal: f64,
    pub talkmax: f64,
    pub recruitcount: f64,
    pub susietalks: f64,
    pub ralseitalks: f64,
    /// balloon (obj_battleblcon) created by scr_enemyblcon
    pub myblcon: Id,
    /// last obj_dbulletcontroller spawned (GML `dc`)
    pub dc: Id,
}
impl Object for KnightEnemy {
    fn name(&self) -> &'static str { "obj_knight_enemy" }

    // ------------------------------------------------------------------ Create_0
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.scr_enemy_object_init(me, g);
        self.recruitcount = 5.0;
        self.talkmax = 90.0;
        me.image_speed = 1.0 / 6.0;
        self.idlesprite = spr("spr_roaringknight_idle");
        self.hurtsprite = spr("spr_roaringknight_idle");
        self.sparedsprite = spr("spr_roaringknight_hurt");
        self.siner = 0.0;
        self.siner2 = 0.0;
        // (obj_ch3_PTB02_roaringknight overworld handoff dropped: the battle starts standalone)
        self.aetimer = 0.0;
        self.rotatingslash3used = false;
        self.holdbreathcount = 0.0;
        self.sactcount = 0.0;
        self.ractcount = 0.0;
        self.chargeupcon = 0.0;
        self.chargeuptimer = 0.0;
        self.turn = 0.0;
        self.phaseturn = 0.0;
        self.phase = 1.0;
        self.myattackchoice = 0.0;
        self.endcon = 0.0;
        self.endtimer = 0.0;
        self.end_cutscene_version = 0.0;
        self.balloonturn = 0.0;
        if let Some((_, c)) = g.get_first::<BattleController>("obj_battlecontroller") {
            c.cantspare[0] = 1.0;
        }
        self.ralseitalks = 0.0;
        self.susietalks = 0.0;
        self.ballooncon = 0.0;
        self.balloonend = 1.0;
        self.blocking = 1.0;
        self.blockanim = 0.0;
        self.blocktimer = 0.0;
        self.damagereduction = 0.04;
        self.damagereductiontimer = 0.0;
        self.krisdamagereduction = 0.5;
        self.whiteflash = 0.0;
        self.haveusedroaring = false;
        self.checkcount = 0.0;
        self.aoedamage = false;
        self.stronghurtanim = false;
        self.progamer = true;
        self.krisdownmessage = false;
        self.susiedownmessage = false;
        self.ralseidownmessage = false;
        self.setdownmessage = false;
        self.damagecounter = 0.0;
        self.phase4turn = 0.0;
        self.myblcon = NOONE;
        self.dc = NOONE;
        // variable_global_exists("firstknightbattle") ? ++ : 1
        if g.glob.firstknightbattle == 0.0 {
            g.glob.firstknightbattle = 1.0;
        } else {
            g.glob.firstknightbattle += 1.0;
        }
        // with (obj_herosusie): serious Susie sprites (no-op if the heroes don't exist yet)
        let unarmed = g.glob.charweapon[2] == 0.0;
        for id in g.ids_of("obj_herosusie") {
            if let Some((_, h)) = g.get::<Hero>(id) {
                h.normalsprite = spr("spr_susier_dark_unhappy");
                h.idlesprite = spr("spr_susieb_idle_serious");
                h.defendsprite = spr("spr_susieb_defend_unhappy");
                h.actreadysprite = spr("spr_susieb_actready");
                h.attacksprite = spr("spr_susieb_attack_serious");
                if unarmed {
                    h.idlesprite = spr("spr_susieb_idle_unarmed_unhappy");
                }
                h.itemsprite = spr("spr_susieb_item_unhappy");
                h.itemreadysprite = spr("spr_susieb_itemready_unhappy");
                h.spellreadysprite = spr("spr_susieb_spellready_unhappy");
                h.spellsprite = spr("spr_susieb_spell_unhappy");
                h.defeatsprite = spr("spr_susie_dw_fell");
            }
        }
        g.destroy_all("obj_battleback");
        g.instance_create(me.x, me.y, Box::new(BgFountainTest::default()));
        g.glob.set_tempflag(96, 0.0);
    }

    // ------------------------------------------------------------------ Step_0
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if !g.exists("obj_herosusie") || !g.exists("obj_heroralsei") {
            return;
        }
        let m = self.myself;
        if g.glob.monsterhp[m] <= g.glob.monstermaxhp[m] * 0.8 {
            g.glob.set_tempflag(96, 1.0);
        }
        if self.holdbreathcount > 0.0 {
            if let Some((_, h)) = g.get_first::<Heart>("obj_heart") {
                h.wspeed = 5.0;
            }
        }
        if self.holdbreathcount > 0.0 && g.exists("obj_knight_roaring2") {
            if let Some((_, h)) = g.get_first::<Heart>("obj_heart") {
                h.wspeed = 6.0;
            }
        }
        self.damagereductiontimer += 1.0;
        if self.damagereductiontimer == 1.0 {
            if self.phaseturn == 1.0 && self.phase == 1.0 {
                self.phaseturn = 1.0;
                self.myattackchoice = 2.0;
                self.difficulty = 0.0;
            }
            self.damagereduction = 0.2;
            g.glob.monstername[m] = "Knight".to_string();
            self.blocking = 0.0;
            self.hurtsprite = spr("spr_roaringknight_hurt");
        }
        // ---- block animation (blockanim = 1 is set by the hero attack code)
        if self.blockanim == 1.0 {
            g.snd_stop("snd_bell");
            g.snd_play("snd_bell");
            self.idlesprite = spr("spr_roaringknight_block_ol");
            self.whiteflash = 2.0;
            self.blockanim = 2.0;
            self.blocktimer = 0.0;
            self.shakex = 5.0;
            self.state = 3.0;
            self.hurttimer = 30.0;
        }
        if self.blockanim == 2.0 {
            self.blocktimer += 1.0;
            if self.blocktimer == 1.0 {
                block_vfx(g, me.x + 14.0, me.y + 40.0, -8.0, 1.0);
                block_vfx(g, me.x + 14.0, me.y + 40.0, 8.0, 2.0);
            }
            if self.blocktimer == 3.0 || self.blocktimer == 6.0 {
                let vs = g.choose(&[-8.0, 8.0]);
                block_vfx(g, me.x + 14.0, me.y + 40.0, vs, 2.0);
            }
            if self.blocktimer == 15.0 {
                self.blocktimer = 0.0;
                self.blockanim = 0.0;
                self.idlesprite = spr("spr_roaringknight_idle");
            }
        }

        if g.glob.monster[m] == 1.0 {
            self.step_enemy_turn(me, g);
        }

        if g.glob.myfight == 3.0 {
            self.step_acts(g);
        }
        if self.state == 3.0 {
            self.scr_enemy_hurt(g);
        }
        // ---- ending after the Roaring (end_cutscene_version is set in Draw)
        if self.end_cutscene_version == 1.0 {
            self.endtimer += 1.0;
            if self.endtimer == 32.0 {
                let f = scr_fadeout(g, 15.0);
                if let Some((fi, fo)) = g.get::<FadeOut>(f) {
                    fi.image_blend = C_WHITE;
                    fi.x -= 40.0;
                    fo.length *= 2.0;
                }
            }
            if self.endcon == 1.0 && self.endtimer > 45.0 {
                g.glob.set_flag(50, 0.0);
                g.glob.set_flag(51, 1.0);
                g.destroy_all("obj_attackpress");
                g.destroy_all("obj_dmgwriter");
                // (obj_ch3_PTB02.teamdefeated = false: overworld cutscene not ported)
                self.endcon = 2.0;
                self.battle_exit(me, g);
                // Knight hurt ending (the story continues): the lead's scene handles it.
                g.glob.set_ex("battle_outcome", 2.0);
            }
        }
        // ---- chargeup before the Roaring
        if self.chargeupcon == 1.0 {
            self.chargeuptimer += 1.0;
            if self.chargeuptimer == 1.0 {
                g.snd_play("snd_knight_powerup_white");
            }
            if self.chargeuptimer % 4.0 == 0.0 && self.chargeuptimer > 10.0 {
                let fid = g.instance_create_depth(me.x, me.y, me.depth + 1.0, Box::new(AfterimageFadeToWhite::default()));
                let dir = g.random(360.0);
                if let Some(f) = g.inst_mut(fid) {
                    f.sprite_index = me.sprite_index;
                    f.image_index = me.image_index;
                    f.image_speed = 0.0;
                    f.image_xscale = me.image_xscale;
                    f.image_yscale = me.image_yscale;
                    f.image_alpha = 0.6;
                    f.set_speed(4.0);
                    f.set_direction(dir);
                }
            }
            if self.chargeuptimer == 60.0 {
                g.glob.turntimer = 1.0;
            }
        }
    }

    // ------------------------------------------------------------------ Step_2
    fn end_step(&mut self, _me: &mut Inst, g: &mut Game) {
        if g.glob.mnfight == 2.0 && self.myattackchoice == 0.0 {
            let cx = g.camerax();
            if let Some(h) = g.first_inst("obj_heart") {
                if h.x > cx + 165.0 {
                    h.x = cx + 165.0;
                }
            }
        }
        if self.end_cutscene_version == 1.0 {
            g.destroy_all("obj_dmgwriter");
        }
    }

    // ------------------------------------------------------------------ Draw_0
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        if !g.exists("obj_knight_roaring2") {
            self.siner2 += 1.0;
        }
        if g.exists("obj_knight_swordtunnelanim") {
            return;
        }
        if self.chargeupcon == 2.0 {
            self.chargeuptimer += 1.0;
            g.gfx.d3d_set_fog(true, C_WHITE);
            g.draw_sprite_ext(self.idlesprite, self.siner, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, (10.0 - self.chargeuptimer) / 10.0);
            g.gfx.d3d_set_fog(false, C_BLACK);
            if self.chargeuptimer == 10.0 {
                self.chargeupcon = 3.0;
                me.image_alpha = 0.0;
            }
            return;
        }
        if self.state == 0.0 || self.state == 3.0 {
            me.image_index = 0.0;
            me.y = me.ystart + (self.siner2 / 8.0).cos() * 8.0;
            self.aetimer += 1.0;
            if self.aetimer % 4.0 == 0.0 && me.image_alpha != 0.0 && self.chargeupcon == 0.0 {
                let mut ai = NOONE;
                if self.state == 0.0 && !g.exists("obj_knight_roaring2") {
                    ai = g.instance_create_depth(me.x, me.y, me.depth + 1.0, Box::new(Afterimage::default()));
                    if let Some(a) = g.inst_mut(ai) {
                        a.sprite_index = spr("spr_roaringknight_idle");
                        a.image_index = me.image_index;
                    }
                }
                if self.state == 3.0 {
                    ai = g.instance_create_depth(me.x + self.shakex + self.hurtspriteoffx, me.y + self.hurtspriteoffy, me.depth + 1.0, Box::new(Afterimage::default()));
                    let ball = !(self.hurttimer % 2.0 == 0.0 || !self.stronghurtanim);
                    let idle = self.idlesprite;
                    if let Some(a) = g.inst_mut(ai) {
                        a.image_index = me.image_index;
                        if !ball {
                            a.sprite_index = idle;
                        } else {
                            a.sprite_index = spr("spr_roaringknight_ball_transition");
                            a.image_index = 7.0;
                        }
                    }
                }
                // (GML would touch the previous `afterimage` if none was created this time; harmless to skip)
                if let Some((a, o)) = g.get::<Afterimage>(ai) {
                    a.image_alpha = 0.6;
                    o.fade_speed = 0.02;
                    a.set_hspeed(2.0);
                    a.image_speed = 0.0;
                    a.image_xscale = me.image_xscale;
                    a.image_yscale = me.image_yscale;
                }
            }
        }
        if self.end_cutscene_version == 1.0 {
            self.stronghurtanim = true;
            self.state = 3.0;
            self.shakex = 0.0;
        }
        let m = self.myself;
        if self.state == 3.0 && self.hurttimer >= 0.0 {
            if self.haveusedroaring
                && self.end_cutscene_version == 0.0
                && g.glob.monsterhp[m] <= g.glob.monstermaxhp[m] * 0.8
                && self.endcon != 1.0
            {
                // with (obj_spellphase) { scr_attackphase(); destroy spellwriter, obj_face; instance_destroy(); }
                for sp in g.ids_of("obj_spellphase") {
                    controller::scr_attackphase(g);
                    // spellwriter is obj_spellphase-private; approximated by destroying every obj_writer
                    g.destroy_all("obj_writer");
                    g.destroy_all("obj_face");
                    g.destroy(sp);
                }
                self.end_cutscene_version = 1.0;
                self.endcon = 1.0;
                // mus_fade(batmusic, 1): audio_sound_gain over (1*1000)/fps ms = one frame
                g.audio.volume(g.glob.batmusic, 0.0, 1.0);
                let sh = g.instance_create(me.x, me.y, Box::new(Shake::default()));
                if let Some((_, s)) = g.get::<Shake>(sh) {
                    s.shakex = 30.0;
                    s.shakey = 8.0;
                    s.shakespeed = 2.0;
                }
                self.stronghurtanim = true;
                self.hurttimer = 999.0;
                g.snd_play_x("snd_knight_hurt", 0.8, 1.0);
                g.snd_play_x("snd_knight_hurt", 0.8, 0.7);
                g.snd_play_x("snd_knight_hurt", 0.8, 1.3);
            }
            let (dx, dy) = (me.x + self.shakex + self.hurtspriteoffx, me.y + self.hurtspriteoffy);
            let modv = if self.end_cutscene_version == 1.0 { 3.0 } else { 2.0 };
            if self.hurttimer % modv == 0.0 || !self.stronghurtanim {
                g.draw_sprite_ext(self.idlesprite, self.siner, dx, dy, 2.0, 2.0, 0.0, me.image_blend, 1.0);
            } else {
                g.draw_sprite_ext(spr("spr_roaringknight_ball_transition"), 7.0, dx, dy, 2.0, 2.0, 0.0, me.image_blend, 1.0);
            }
            if self.hurttimer == 29.0 && self.stronghurtanim && self.end_cutscene_version == 0.0 {
                g.snd_play("snd_knight_hurtb");
            }
            if self.hurttimer == 15.0 {
                self.stronghurtanim = false;
            }
        }
        self.scr_enemy_drawidle_generic(me, g, 1.0 / 6.0);
        if self.whiteflash > 0.0 {
            self.whiteflash -= 1.0;
            g.gfx.d3d_set_fog(true, C_WHITE);
            if self.state == 3.0 && self.hurttimer >= 0.0 {
                g.draw_sprite_ext(self.hurtsprite, 0.0, me.x + self.shakex + self.hurtspriteoffx, me.y + self.hurtspriteoffy, 2.0, 2.0, 0.0, me.image_blend, 0.62);
            }
            if self.state == 0.0 {
                g.draw_sprite_ext(self.idlesprite, self.siner, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, 0.62);
            }
            g.gfx.d3d_set_fog(false, C_BLACK);
        }
        if self.chargeupcon == 1.0 {
            g.gfx.d3d_set_fog(true, C_WHITE);
            g.draw_sprite_ext(self.idlesprite, self.siner, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, self.chargeuptimer / 10.0);
            g.gfx.d3d_set_fog(false, C_BLACK);
        }
        if self.becomeflash == 0.0 {
            self.flash = 0.0;
        }
        self.becomeflash = 0.0;
    }

    // ------------------------------------------------------------------ Alarms
    fn alarm(&mut self, n: usize, _me: &mut Inst, _g: &mut Game) {
        match n {
            4 => self.con += 1.0,
            6 => {
                if self.balloonend == 1.0 {
                    self.talked = 1.0;
                } else {
                    self.talked = 0.6;
                    self.talktimer = 0.0;
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------ event_user
    fn user(&mut self, n: usize, me: &mut Inst, g: &mut Game) {
        match n {
            0 => {
                self.choose_attack();
                // debug override (like the game's scr_attack_override): ?attack=<myattackchoice>&diff=<difficulty>
                let ov = g.glob.ex("debug_attack");
                if ov != 0.0 {
                    self.myattackchoice = ov;
                    self.difficulty = g.glob.ex("debug_diff");
                }
            }
            2 => self.aoe_damage(me, g),
            3 => self.battle_end_flags(g),
            // Other_14: exit
            4 => {}
            12 => {
                // Other_22
                let m = self.myself;
                g.glob.monsterx[m] = me.x + g.sprite_width(me) / 2.0;
                g.glob.monstery[m] = me.y + g.sprite_height(me) / 2.0;
                scr_monstersetup(g, m);
            }
            // obj_monsterparent Other_20 (event_user(10): spare/recruit/scr_monsterdefeat) is unreachable in
            // this fight (mercy can't rise; the battle ends at 80% HP) and is not ported.
            // obj_monsterparent Other_25 (event_user(15): recruit info) is not needed.
            _ => {}
        }
    }

    // ------------------------------------------------------------------ CleanUp_0
    fn cleanup(&mut self, _me: &mut Inst, g: &mut Game) { g.glob.invc = 1.0; }

    obj_vars!(
        siner, siner2, aetimer, chargeupcon, chargeuptimer, whiteflash, state, hurttimer, shakex, con, talked,
        attacked, flash, becomeflash, hurtspriteoffx, hurtspriteoffy, damagereduction, blockanim, phase, phaseturn
    );
}

impl KnightEnemy {
    /// scr_enemy_object_init
    fn scr_enemy_object_init(&mut self, me: &mut Inst, g: &mut Game) {
        self.becomeflash = 0.0;
        self.flash = 0.0;
        self.turns = 0.0;
        self.talktimer = 0.0;
        self.state = 0.0;
        self.siner = 0.0;
        self.fsiner = 0.0;
        self.talked = 0.0;
        self.attacked = 0.0;
        self.hurt = 0.0;
        self.hurttimer = 0.0;
        self.hurtshake = 0.0;
        self.shakex = 0.0;
        self.acttimer = 0.0;
        self.con = 0.0;
        self.fatal = 0.0;
        self.mytarget = 0.0;
        me.damage = -1.0;
        me.grazepoints = -1.0;
        me.timepoints = -1.0;
        me.inv = -1.0;
        me.target = -1.0;
        me.grazed = -1.0;
        me.grazetimer = -1.0;
        me.element = 0.0;
        self.acting = 0.0;
        self.actcon = 0.0;
        self.actingsus = 0.0;
        self.actingral = 0.0;
        self.actconsus = 0.0;
        self.actconral = 0.0;
        self.talkmax = 90.0;
        self.recruitcount = 1.0;
        self.hurtspriteoffx = 0.0;
        self.hurtspriteoffy = 0.0;
        me.image_xscale = 2.0;
        me.image_yscale = 2.0;
        me.image_speed = 0.2;
        me.depth = 90.0 - (me.y - g.cameray()) / 50.0;
    }

    /// scr_enemy_hurt
    fn scr_enemy_hurt(&mut self, _g: &mut Game) {
        self.hurttimer -= 1.0;
        if self.hurttimer < 0.0 {
            self.state = 0.0;
        } else {
            // (global.monster[myself] == 0 → scr_defeatrun: unreachable here)
            self.hurtshake += 1.0;
            if self.hurtshake > 1.0 {
                if self.shakex > 0.0 {
                    self.shakex -= 1.0;
                }
                if self.shakex < 0.0 {
                    self.shakex += 1.0;
                }
                self.shakex = -self.shakex;
                self.hurtshake = 0.0;
            }
        }
    }

    /// scr_enemy_drawidle_generic(siner_add) + draw_monster_body_part
    fn scr_enemy_drawidle_generic(&mut self, me: &mut Inst, g: &mut Game, add: f64) {
        if self.state == 0.0 {
            self.fsiner += 1.0;
            self.siner += add;
            let m = self.myself;
            let mut thissprite = self.idlesprite;
            if g.glob.mercymod[m] >= g.glob.mercymax[m] {
                thissprite = self.sparedsprite;
            }
            g.draw_sprite_ext(thissprite, self.siner, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, me.image_alpha);
            if self.flash == 1.0 {
                // draw_sprite_ext_flash
                g.gfx.d3d_set_fog(true, C_WHITE);
                let a = -(self.fsiner / 5.0).cos() * 0.4 + 0.6;
                g.draw_sprite_ext(thissprite, self.siner, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, a);
                g.gfx.d3d_set_fog(false, C_BLACK);
            }
        }
    }

    /// scr_bulletspawner(x, y, obj_dbulletcontroller) + dc.type / dc.difficulty / optional dc.damage.
    fn bulletspawner(&mut self, me: &Inst, g: &mut Game, typ: i32, difficulty: f64, damage: Option<f64>) -> Id {
        let id = crate::battle::spawn_dbulletcontroller(g, me, typ, difficulty, damage);
        // our own slot is checked out during Step, so set __dc.target = mytarget explicitly
        let t = self.mytarget;
        if let Some(i) = g.inst_mut(id) {
            i.target = t;
        }
        self.dc = id;
        id
    }

    /// Step_0: everything inside `if (global.monster[myself] == 1)`.
    fn step_enemy_turn(&mut self, me: &mut Inst, g: &mut Game) {
        let m = self.myself;
        // ---- enemy talk: Susie's balloons
        if g.glob.mnfight == 1.0 && self.talked == 0.0 && self.end_cutscene_version == 0.0 {
            self.mytarget = controller::scr_randomtarget(g);
            if !g.exists("obj_darkener") {
                g.instance_create(0.0, 0.0, Box::new(Darkener::default()));
            }
            g.glob.fc = 22.0;
            g.glob.typer = 81.0;
            let mut createballoon = false;
            self.susietalks = 1.0;
            if g.glob.hp[2] > 0.0 {
                self.balloonturn += 1.0;
                let bt = self.balloonturn;
                let others_down = g.glob.hp[1] < 1.0 && g.glob.hp[3] < 1.0;
                let set = |g: &mut Game, s: &str, con: f64, end: f64, me_: &mut Self| {
                    msgset(g, 0, s);
                    me_.ballooncon = con;
                    me_.balloonend = end;
                };
                if bt == 6.0 {
                    set(g, "Heheh.../%", 1.0, 0.0, self);
                    createballoon = true;
                }
                if bt == 7.0 {
                    set(g, "Thing is,&you actually.../%", 2.0, 0.0, self);
                    createballoon = true;
                }
                if bt == 8.0 {
                    set(g, "You? You're all&damn alone.../%", 3.0, 0.0, self);
                    createballoon = true;
                }
                if bt == 9.0 {
                    set(g, "Even... even if&you knock me down.../%", 4.0, 0.0, self);
                    createballoon = true;
                }
                if bt == 9.0 && others_down {
                    set(g, "Even... even if&you knock them down.../%", 5.0, 0.0, self);
                    createballoon = true;
                }
                if bt == 10.0 {
                    set(g, "As long as Kris has got&a hand to lift me up with.../%", 6.0, 0.0, self);
                    createballoon = true;
                }
                if bt == 10.0 && others_down {
                    set(g, "As long as I'm here to&lift them back up.../%", 6.0, 0.0, self);
                    createballoon = true;
                }
                if bt == 11.0 {
                    set(g, "So... give up./%", 0.0, 1.0, self);
                    createballoon = true;
                }
                if bt == 12.0 {
                    set(g, "You know you can't&win... so... give up!/%", 0.0, 1.0, self);
                    createballoon = true;
                }
                if bt == 13.0 {
                    set(g, "... You won't even.../%", 7.0, 0.0, self);
                    createballoon = true;
                }
                if bt == 14.0 {
                    set(g, "... heh... heheheh.../%", 8.0, 0.0, self);
                    createballoon = true;
                }
            }
            if createballoon {
                g.glob.typer = 75.0;
                self.susie_balloon(me, g);
            }
            self.ralseitalks = 0.0;
            self.susietalks = 0.0;
            if !createballoon {
                g.glob.mnfight = 1.5;
            } else if self.ballooncon == 0.0 {
                self.talked = 0.5;
                self.talktimer = 0.0;
            } else {
                self.talked = 0.6;
                self.talktimer = 0.0;
            }
            self.rtimer = 0.0;
        }
        if self.talked == 0.5 {
            self.talktimer += 1.0;
            if (g.input.pressed(Key::B3) && self.talktimer > 15.0) || !g.exists("obj_writer") {
                g.destroy_all("obj_writer");
                me.alarm[6] = 1;
            }
        }
        if self.talked == 0.6 {
            self.talktimer += 1.0;
            if (g.input.pressed(Key::B3) && self.talktimer > 15.0) || !g.exists("obj_writer") {
                g.destroy_all("obj_writer");
                let (s, con, end) = match self.ballooncon as i32 {
                    1 => ("Didn't... think&we'd still be&standing, did you?/%", 0.0, 1.0),
                    2 => ("You actually messed up,&picking a fight with US!/%", 0.0, 1.0),
                    3 => ("Me? I got...&Kris and Ralsei&behind me./%", 0.0, 1.0),
                    4 => ("As long as Kris,&Ralsei, are here.../%", 0.0, 1.0),
                    5 => ("As long as&I'm here.../%", 0.0, 1.0),
                    6 => ("Heh... you're never gonna&win, you hear me?!/%", 0.0, 1.0),
                    7 => ("... say a thing, huh.../%", 0.0, 1.0),
                    8 => ("Man, I'm done talking./%", 9.0, 0.0),
                    9 => ("... people like you...&just piss me off./%", 0.0, 1.0),
                    _ => ("", self.ballooncon, self.balloonend),
                };
                if !s.is_empty() {
                    msgset(g, 0, s);
                    self.ballooncon = con;
                    self.balloonend = end;
                }
                self.talked = 0.7;
                self.susietalks = 1.0;
                g.glob.typer = 75.0;
                self.susie_balloon(me, g);
                self.ralseitalks = 0.0;
                self.susietalks = 0.0;
                me.alarm[6] = 1;
            }
        }
        if self.talked == 1.0 && g.glob.mnfight == 1.0 && !g.exists("obj_writer") {
            g.glob.mnfight = 1.5;
            self.attackchosen = false;
        }
        // ---- set up the bullet board, SOUL and attack choice
        if g.glob.mnfight == 1.5 && self.end_cutscene_version == 0.0 {
            if !g.exists("obj_growtangle") {
                self.user(0, me, g);
                self.setdownmessage = false;
                if self.damagereduction >= 0.2 && self.damagereduction < 0.35 {
                    self.damagereduction += 0.01;
                }
                let (vx, vy) = (g.camerax(), g.cameray());
                let ac = self.myattackchoice;
                let mut gt = NOONE;
                if ac == -1.0 {
                } else if ac == 0.0 {
                    gt = g.instance_create(vx + 320.0 - 152.0, vy + 170.0, Box::new(Growtangle::default()));
                } else if ac == 11.0 {
                    gt = g.instance_create(vx + 320.0, vy + 190.0, Box::new(Growtangle::default()));
                } else if ac == 13.0 {
                    gt = g.instance_create(vx + 300.0, vy + 190.0, Box::new(Growtangle::default()));
                } else {
                    gt = g.instance_create(vx + 320.0, vy + 170.0, Box::new(Growtangle::default()));
                }
                if let Some((_, b)) = g.get::<Growtangle>(gt) {
                    if ac == 0.0 {
                        b.maxxscale = 0.5;
                    }
                    if ac == 1.0 {
                        b.maxxscale = 2.25;
                        b.maxyscale = 1.75;
                    }
                    if ac == 4.0 {
                        b.maxxscale = 3.5;
                        b.maxyscale = 3.5;
                    }
                    if ac == 13.0 {
                        b.maxxscale = 3.0;
                    }
                }
            }
            if !g.exists("obj_moveheart") && !g.exists("obj_heart") && self.myattackchoice != -1.0 {
                let mh = soul::scr_moveheart(g);
                if self.myattackchoice == 13.0 {
                    let gpos = g.first_inst("obj_growtangle").map(|b| (b.x, b.y));
                    if let Some((bx, by)) = gpos {
                        let (distx, disty) = (bx - 40.0, by - 8.0);
                        let flytime = g.get_var(mh, "flytime").unwrap_or(8.0);
                        g.set_var(mh, "distx", distx);
                        g.set_var(mh, "disty", disty);
                        if let Some(h) = g.inst_mut(mh) {
                            let dist = point_distance(h.x, h.y, distx, disty);
                            h.move_towards_point(distx, disty, dist / flytime);
                        }
                    }
                }
            }
            g.glob.mnfight = 2.0;
            scr_turntimer(g, 90.0);
        }
        // ---- spawn the attack
        if g.glob.mnfight == 2.0 && self.attacked == 0.0 && self.end_cutscene_version == 0.0 {
            self.rtimer += 1.0;
            if self.rtimer == 12.0 {
                self.spawn_attack(me, g);
            }
        }
        // ---- end-of-turn messages
        if g.glob.mnfight == 2.0 && g.glob.turntimer <= 1.0 && !self.setdownmessage {
            self.setdownmessage = true;
            if g.glob.monsterhp[m] <= g.glob.monstermaxhp[m] * 0.8 && !self.haveusedroaring && self.phase != 4.0 {
                self.phase = 4.0;
            }
            if self.phase == 4.0 && self.phase4turn < 3.0 {
                if self.phase4turn == 0.0 {
                    g.glob.battlemsg[0] = "* Your heartbeat becomes twisted.".into();
                }
                if self.phase4turn == 1.0 && g.glob.hp[2] > 0.0 {
                    g.glob.battlemsg[0] = "* Susie grew pale.".into();
                }
                if self.phase4turn == 1.0 && g.glob.hp[2] < 1.0 {
                    g.glob.battlemsg[0] = "* Susie struggled to give some kind of warning.".into();
                }
            } else {
                let mut krisdown = String::new();
                let mut susiedown = String::new();
                let mut ralseidown = String::new();
                let mut downcount = 0;
                if !self.krisdownmessage && g.glob.hp[1] < 1.0 {
                    krisdown = "* Kris kneeled in silence.&".into();
                    downcount += 1;
                    self.krisdownmessage = true;
                    g.glob.battlemsg[0] = krisdown.clone();
                }
                if !self.susiedownmessage && g.glob.hp[2] < 1.0 {
                    susiedown = "* Susie was hurt and beaten.&".into();
                    downcount += 1;
                    self.susiedownmessage = true;
                    g.glob.battlemsg[0] = susiedown.clone();
                }
                if !self.ralseidownmessage && g.glob.hp[3] < 1.0 {
                    ralseidown = "* Ralsei became a pile of fluff.&".into();
                    downcount += 1;
                    self.ralseidownmessage = true;
                    g.glob.battlemsg[0] = ralseidown.clone();
                }
                if downcount == 2 {
                    g.glob.battlemsg[0] = krisdown + &susiedown + &ralseidown;
                }
            }
        }
        // ---- all heroes down
        let all_down = g.glob.hp[1] < 1.0 && g.glob.hp[2] < 1.0 && g.glob.hp[3] < 1.0;
        if all_down && g.glob.turntimer > 0.0 {
            g.glob.turntimer = 0.0;
            g.destroy_all("obj_lerpvar");
            g.destroy_all("obj_script_delayed");
            g.destroy_all("obj_afterimage_grow");
        }
        if g.glob.mnfight == 2.0 && g.glob.turntimer < 1.0 && all_down && self.endcon == 0.0 {
            self.endcon = 1.0;
            g.destroy_all("obj_particle_generic");
            // scr_get_knight_total_attempts(): counts past attempts from the save files in the real game.
            // Here: 0 on the first defeat (story continues), > 0 afterwards (game over).
            let attempts = g.glob.ex("knight_attempts");
            g.glob.set_ex("knight_attempts", attempts + 1.0);
            if attempts > 0.0 {
                // scr_gameover(): handled by the lead's scene
                g.glob.set_ex("battle_outcome", 3.0);
            } else {
                self.battle_exit(me, g);
                // team defeated, the story continues: handled by the lead's scene
                g.glob.set_ex("battle_outcome", 1.0);
            }
        }
    }

    /// obj_battlecontroller.noreturn/intro, tension bar exit, music fade, event_user(3), global.fighting = 0.
    fn battle_exit(&mut self, me: &mut Inst, g: &mut Game) {
        if let Some((_, c)) = g.get_first::<BattleController>("obj_battlecontroller") {
            c.noreturn = 1.0;
            c.intro = 2.0;
        }
        for id in g.ids_of("obj_tensionbar") {
            if let Some(t) = g.inst_mut(id) {
                t.alarm[5] = 15;
                t.set_hspeed(-10.0);
                t.friction = -0.4;
            }
        }
        g.audio.volume(g.glob.batmusic, 0.0, 90.0);
        self.user(3, me, g);
        g.glob.fighting = 0.0;
    }

    /// scr_enemyblcon(obj_herosusie.x + 92, obj_herosusie.y + 38, 14); scr_guardpeek(obj_herosusie);
    /// myblcon.depth = depth - 100
    fn susie_balloon(&mut self, me: &Inst, g: &mut Game) {
        let (sx, sy) = g.first_inst("obj_herosusie").map(|h| (h.x, h.y)).unwrap_or((0.0, 0.0));
        self.myblcon = scr_enemyblcon(g, sx + 92.0, sy + 38.0, 14);
        scr_guardpeek_susie(g);
        if let Some(b) = g.inst_mut(self.myblcon) {
            b.depth = me.depth - 100.0;
        }
    }

    /// The `rtimer == 12` block of Step_0: spawn the attack, set the turn timer and battle message.
    fn spawn_attack(&mut self, me: &mut Inst, g: &mut Game) {
        let m = self.myself;
        let ac = self.myattackchoice;
        let d = self.difficulty;
        self.aoedamage = false;
        let name = |g: &mut Game, s: &str| g.glob.monsterattackname[m] = s.to_string();
        // obj_dbulletcontroller's own default difficulty is 0
        if ac == 0.0 {
            name(g, "Swordslash");
            self.bulletspawner(me, g, 109, d, None);
            g.glob.invc = 0.4;
        } else if ac == 1.0 {
            name(g, "Stars");
            self.bulletspawner(me, g, 98, d, None);
            g.glob.invc = 1.0;
        } else if ac == 2.0 {
            name(g, "Flurry");
            self.bulletspawner(me, g, 99, d, None);
            g.glob.invc = 0.4;
        } else if ac == 3.0 {
            name(g, "swordtunnel");
            self.bulletspawner(me, g, 102, 0.0, None);
            g.glob.invc = 0.4;
        } else if ac == 4.0 {
            name(g, "xattacks");
            self.bulletspawner(me, g, 103, 0.0, None);
            g.glob.invc = 0.4;
        } else if ac == 5.0 {
            name(g, "rotatingslash");
            self.bulletspawner(me, g, 104, d, None);
            g.glob.invc = 1.0;
        } else if ac == 6.0 {
            name(g, "underboxattack");
            self.bulletspawner(me, g, 106, 0.0, None);
            g.glob.invc = 0.4;
        } else if ac == 7.0 {
            name(g, "combinationattack");
            self.bulletspawner(me, g, 105, 0.0, None);
            g.glob.invc = 0.4;
        } else if ac == 9.0 {
            name(g, "roaring");
            self.bulletspawner(me, g, 107, 0.0, None);
            g.glob.invc = 1.0;
        } else if ac == 10.0 {
            name(g, "swords falling");
            self.bulletspawner(me, g, 108, d, None);
            g.glob.invc = 0.4;
        } else if ac == 11.0 {
            name(g, "tracking swords");
            self.bulletspawner(me, g, 151, 0.0, Some(206.0));
            g.glob.invc = 0.4;
        } else if ac == 12.0 {
            name(g, "diagonal bullets");
            self.bulletspawner(me, g, 152, d, None);
            g.glob.invc = 0.4;
        } else if ac == 13.0 {
            name(g, "sword tunnel new");
            self.bulletspawner(me, g, 153, d, Some(62.0));
            g.glob.invc = 0.14;
        } else if ac == 14.0 {
            name(g, "tracking swords");
            self.bulletspawner(me, g, 151, 3.0, Some(206.0));
            g.glob.invc = 0.4;
        } else if ac == 15.0 {
            name(g, "sword vortex");
            self.bulletspawner(me, g, 154, 3.0, Some(206.0));
            name(g, "tracking swords");
            self.bulletspawner(me, g, 151, 0.0, Some(206.0));
            g.glob.invc = 0.4;
        } else if ac == 16.0 {
            name(g, "rotatingslash");
            self.bulletspawner(me, g, 104, 0.0, None);
            name(g, "tracking swords");
            self.bulletspawner(me, g, 151, 0.0, Some(206.0));
            g.glob.invc = 0.4;
        } else if ac == 17.0 {
            name(g, "tracking swords");
            self.bulletspawner(me, g, 151, 2.0, Some(206.0));
            g.glob.invc = 0.4;
        }
        if ac == -1.0 {
            self.chargeupcon = 1.0;
        } else if ac == 20.0 {
            name(g, "knightlines");
            self.bulletspawner(me, g, 101, 0.0, None);
        }
        let tt = if ac == 7.0 {
            270.0
        } else if ac == 2.0 {
            350.0
        } else if ac == 0.0 && d == 0.0 {
            300.0
        } else if ac == 0.0 && d == 1.0 {
            300.0
        } else if ac == 11.0 && d == 0.0 {
            292.0
        } else if ac == 11.0 {
            300.0
        } else if ac == 12.0 {
            300.0
        } else if ac == 13.0 && d == 3.0 {
            360.0
        } else if ac == 13.0 {
            330.0
        } else if ac == 14.0 || ac == 15.0 {
            300.0
        } else {
            240.0
        };
        scr_turntimer(g, tt);
        if self.damagereductiontimer >= 750.0 {
            g.glob.monstername[m] = "Knight".to_string();
        }
        self.turns += 1.0;
        g.glob.typer = 6.0;
        g.glob.fc = 0.0;
        let pt = self.phaseturn;
        let mut msg: Option<&str> = None;
        if self.phase == 1.0 {
            msg = match pt as i32 {
                0 => Some("* You felt lightheaded.&* You saw silver stars..."),
                1 => Some("* You felt something hovering close behind your head..."),
                2 => Some("* Suddenly, the north wind blew fiercely."),
                3 => Some("* Your vision narrows."),
                4 => Some("* Your chest feels tight."),
                _ => msg,
            };
        }
        if self.phase == 2.0 {
            msg = match pt as i32 {
                0 => Some("* You felt lightheaded.&* You saw golden stars..."),
                1 => Some("* Suddenly, the north and east winds blew fiercely."),
                2 => Some("* Your vision narrows.&* ... Your head is spinning."),
                3 => Some("* You feel surrounded."),
                4 => Some("* You felt your chest twisting."),
                _ => msg,
            };
        }
        if self.phase == 3.0 {
            msg = match pt as i32 {
                0 => Some("* You felt lightheaded.&* You felt a migraine coming on..."),
                1 => Some("* Suddenly, a tempest."),
                2 => Some("* Your vision narrows.&* ... The world revolves around you."),
                3 => Some("* You feel cornered."),
                4 => Some("* Your heartbeat becomes twisted."),
                _ => msg,
            };
        }
        if self.phase == 4.0 || self.haveusedroaring {
            if self.phase4turn == 2.0 {
                msg = Some("* The Knight's hands glow a strange color...");
            }
            if self.phase4turn > 2.0 {
                msg = Some("* The enemy suddenly let down its guard!");
            }
            if self.phase4turn == 3.0 && self.progamer {
                msg = Some("* Kris coughed.&* The enemy slowly tilted its head...");
            }
        }
        if let Some(s) = msg {
            g.glob.battlemsg[0] = s.to_string();
        }
        self.attacked = 1.0;
    }

    /// Step_0: `if (global.myfight == 3)` — ACT handling.
    fn step_acts(&mut self, g: &mut Game) {
        let m = self.myself;
        if self.acting == 1.0 && self.actcon == 0.0 {
            // Check
            self.actcon = 1.0;
            self.checkcount += 1.0;
            if self.checkcount == 1.0 {
                msgset(g, 0, "* Kris analyzed the enemy!/");
                msgnext(g, "* But Kris&couldn't learn anything./%");
            } else {
                msgset(g, 0, "* Kris points into the distance./");
                msgnext(g, "* Nothing happened./%");
            }
            scr_battletext_default(g);
        }
        if self.acting == 2.0 && self.actcon == 0.0 {
            // HoldBreath
            self.actcon = 1.0;
            self.holdbreathcount += 1.0;
            if self.holdbreathcount <= 1.0 {
                msgset(g, 0, "* Kris held their breath.&* Their heartbeat quickened.&* The SOUL now moves faster./%");
            }
            if self.holdbreathcount > 1.0 {
                msgset(g, 0, "* Kris held their breath...&* Kris smiled.&* Nothing happened./%");
            }
            scr_battletext_default(g);
            self.holdbreathcount = 1.0;
        }
        if self.actingsus == 1.0 && self.actconsus == 1.0 {
            // S-Action
            scr_speaker(g, "noone");
            msgset(g, 0, "* Susie talked to the Knight!/");
            scr_anyface_next(g, "susie", "J");
            msgnext(g, "\\EJ* I don't know what the hell you are, but.../");
            msgnext(g, "\\EJ* Leave Toriel alone! You hear me!?/");
            msgnext(g, "\\EV* .../");
            msgnext(g, "\\EW* ... Fine, you don't wanna listen?/");
            msgnext(g, "\\EX* Then we'll just. Have to do things the hard way./");
            scr_anyface_next(g, "none", "0");
            msgnext(g, "* (Susie will not ACT any more.)/%");
            scr_battletext_default(g);
            self.sactcount = 1.0;
            self.actcon = 1.0;
            self.actconsus = 0.0;
            g.glob.canactsus[m] = 0.0;
            g.glob.actnamesus[m] = String::new();
            g.glob.actsimulsus[m] = 0.0;
            // (global.actactorsus has no Glob field; unused by this fight)
            g.glob.actcostsus[m] = 0.0;
            g.glob.battlespell[1][0] = -1.0;
            g.glob.battleactcount[1] = 0.0;
            g.glob.battlespellname[1][0] = String::new();
            g.glob.battlespelldesc[1][0] = String::new();
            crate::battle::spells::scr_spellmenu_setup(g);
        }
        if self.actingral == 1.0 && self.actconral == 1.0 {
            // R-Action
            self.ractcount += 1.0;
            if self.ractcount == 1.0 {
                scr_speaker(g, "noone");
                msgset(g, 0, "* Ralsei tried talking.../");
                scr_anyface_next(g, "ralsei", "Q");
                msgnext(g, "\\EQ* Please... please, don't do this.../");
                msgnext(g, "\\Ee* If the Roaring happens, then... then.../");
                msgnext(g, "\\EZ* Please... stop...!/");
                scr_anyface_next(g, "none", "0");
                msgnext(g, "* (... but nothing happened.)/%");
                scr_speaker(g, "noone");
            } else {
                scr_speaker(g, "noone");
                msgset(g, 0, "* Ralsei tried talking.../");
                scr_anyface_next(g, "ralsei", "8");
                msgnext(g, "\\E8* Please, stop.../");
                scr_anyface_next(g, "none", "0");
                msgnext(g, "* (... but nothing happened.)/%");
            }
            scr_battletext_default(g);
            self.actcon = 1.0;
            self.actconral = 0.0;
        }
        if self.actcon == 20.0 || self.actconsus == 20.0 || self.actconral == 20.0 {
            if scr_terminate_writer(g) {
                self.actconsus = -1.0;
                self.actconral = -1.0;
                self.actcon = 1.0;
            }
        }
        if self.actcon == 1.0 && !g.exists("obj_writer") {
            controller::scr_nextact(g);
        }
    }

    /// Other_10 (event_user(0)): attack choice for this turn.
    fn choose_attack(&mut self) {
        if self.phase != 4.0 {
            self.turn += 1.0;
            self.phaseturn += 1.0;
        }
        let pt = self.phaseturn;
        if self.phase == 1.0 {
            match pt as i32 {
                1 => self.pick(1.0, 0.0),
                2 => self.pick(11.0, 0.0),
                3 => self.pick(2.0, 0.0),
                4 => self.pick(13.0, 0.0),
                5 => {
                    self.pick(5.0, 0.0);
                    self.phase = 2.0;
                    self.phaseturn = 0.0;
                }
                6 => self.pick(12.0, 0.0),
                7 => self.pick(16.0, 0.0),
                8 => self.pick(17.0, 0.0),
                9 => self.pick(7.0, 0.0),
                _ => {}
            }
        }
        // (phase/phaseturn are re-read: GML falls through into the next phase's checks)
        if self.phase == 2.0 {
            match self.phaseturn as i32 {
                1 => self.pick(1.0, 1.0),
                2 => self.pick(2.0, 1.0),
                3 => self.pick(13.0, 3.0),
                4 => self.pick(15.0, 0.0),
                5 => {
                    self.pick(5.0, 1.0);
                    self.phase = 3.0;
                    self.phaseturn = 0.0;
                }
                _ => {}
            }
        }
        if self.phase == 3.0 {
            match self.phaseturn as i32 {
                1 => self.pick(1.0, 2.0),
                2 => self.pick(2.0, 3.0),
                3 => self.pick(14.0, 0.0),
                4 => self.pick(13.0, 4.0),
                5 => {
                    self.pick(5.0, 2.0);
                    self.rotatingslash3used = true;
                    self.phaseturn = 0.0;
                }
                _ => {}
            }
        }
        if self.phase == 4.0 {
            self.phase4turn += 1.0;
            if self.phase4turn == 1.0 && self.rotatingslash3used {
                self.phase4turn = 2.0;
            }
            if self.phase4turn == 1.0 {
                self.pick(5.0, 2.0);
            }
            if self.phase4turn == 2.0 {
                self.pick(-1.0, 1.0);
            }
            if self.phase4turn == 3.0 {
                self.pick(9.0, 0.0);
                self.damagereduction = 0.4;
                self.haveusedroaring = true;
                self.phase = 3.0;
            }
        }
    }
    fn pick(&mut self, ac: f64, d: f64) {
        self.myattackchoice = ac;
        self.difficulty = d;
    }

    /// Other_12 (event_user(2)): the Roaring's hit on every living hero (40, or leaves them at 1 HP).
    fn aoe_damage(&mut self, me: &mut Inst, g: &mut Game) {
        if g.glob.inv < 0.0 {
            let temptarget = me.target;
            for ti in 0..3usize {
                g.glob.inv = -1.0;
                me.damage = 40.0;
                let h = g.glob.hp[ti + 1];
                if h > 1.0 && h < 41.0 {
                    me.damage = h - 1.0;
                }
                me.target = ti as f64;
                let c = g.glob.char[ti];
                if g.glob.hp[c] > 0.0 && c != 0 {
                    crate::battle::scr_damage(me, g);
                }
            }
            g.glob.inv = g.glob.invc * 30.0;
            me.target = temptarget;
        }
    }

    /// Other_13 (event_user(3)): end-of-battle bookkeeping (scr_monsterdefeat-like flags).
    fn battle_end_flags(&mut self, g: &mut Game) {
        let m = self.myself as i32;
        let gold = g.glob.ex("monstergold3") + g.glob.ex("monstergold0");
        g.glob.set_ex("monstergold3", gold);
        let exp = g.glob.ex("monsterexp3") + g.glob.ex("monsterexp0");
        g.glob.set_ex("monsterexp3", exp);
        let f = 51 + m;
        if g.glob.flag(f) == 0.0 {
            g.glob.set_flag(f, 2.0);
            if g.glob.monsterhp[self.myself] <= 0.0 {
                g.glob.set_flag(f, 1.0);
            }
        }
        let inc = |g: &mut Game, k: i32| {
            let v = g.glob.flag(k) + 1.0;
            g.glob.set_flag(k, v);
        };
        match g.glob.flag(f) as i32 {
            1 => {
                inc(g, 40);
                if self.fatal == 1.0 {
                    inc(g, 44);
                }
            }
            2 => inc(g, 41),
            3 => inc(g, 42),
            5 => inc(g, 43),
            6 => {
                inc(g, 45);
                let gold = g.glob.ex("monstergold3") + 24.0;
                g.glob.set_ex("monstergold3", gold);
            }
            _ => {}
        }
        let (mut frozened, mut violenced, mut spared, mut pacified) = (0, 0, 0, 0);
        for d in 0..3 {
            match g.glob.flag(51 + d) as i32 {
                1 => violenced += 1,
                2 => spared += 1,
                3 => pacified += 1,
                6 => frozened += 1,
                _ => {}
            }
        }
        if frozened > 0 {
            g.glob.set_flag(50, 6.0);
        }
        if pacified > 0 {
            g.glob.set_flag(50, 3.0);
        }
        if spared > 0 {
            g.glob.set_flag(50, 2.0);
        }
        if violenced > 0 {
            g.glob.set_flag(50, 1.0);
        }
        if g.glob.flag(50) == 6.0 {
            inc(g, 926);
        }
        let f54 = g.glob.flag(54);
        if f54 != 0.0 {
            let v = g.glob.flag(50);
            g.glob.set_flag(f54 as i32, v);
            g.glob.set_flag(54, 0.0);
        }
    }
}

/// scr_turntimer(n)
pub fn scr_turntimer(g: &mut Game, n: f64) {
    if g.glob.turntimer < n {
        g.glob.turntimer = n;
    }
}

/// scr_monster_actreset(slot) + scr_monstersetup() for monstertype 104 (the Roaring Knight).
pub fn scr_monstersetup(g: &mut Game, m: usize) {
    // scr_monster_actreset (Glob holds the ACT tables of monster slot 0 only)
    if m == 0 {
        for j in 0..6 {
            g.glob.canact[j] = 0.0;
            g.glob.actname[j] = " ".into();
            g.glob.actactor[j] = 1.0;
            g.glob.actdesc[j] = " ".into();
            g.glob.actcost[j] = 0.0;
            g.glob.actsimul[j] = 0.0;
            g.glob.canactsus[j] = 0.0;
            g.glob.actnamesus[j] = " ".into();
            g.glob.actdescsus[j] = " ".into();
            g.glob.actcostsus[j] = 0.0;
            g.glob.actsimulsus[j] = 0.0;
            g.glob.canactral[j] = 0.0;
            g.glob.actnameral[j] = " ".into();
            g.glob.actdescral[j] = " ".into();
            g.glob.actcostral[j] = 0.0;
            g.glob.actsimulral[j] = 0.0;
        }
    }
    if g.glob.monstertype[m] == 104.0 {
        g.glob.monstername[m] = String::new();
        g.glob.monstermaxhp[m] = 7300.0;
        g.glob.monsterhp[m] = 7300.0;
        g.glob.monsterat[m] = 40.0;
        g.glob.monsterdf[m] = 0.0;
        g.glob.set_ex("monsterexp0", 0.0);
        g.glob.set_ex("monstergold0", 0.0);
        g.glob.sparepoint[m] = 0.0;
        g.glob.mercymod[m] = 0.0;
        g.glob.mercymax[m] = 100.0;
        if m == 0 {
            g.glob.canact[0] = 1.0;
            g.glob.actname[0] = "Check".into();
            g.glob.actdesc[0] = "Useless#analysis".into();
            g.glob.canact[1] = 1.0;
            g.glob.actname[1] = "HoldBreath".into();
            g.glob.canactsus[0] = 1.0;
            g.glob.actnamesus[0] = "S-Action".into();
            g.glob.actsimulsus[0] = 0.0;
            g.glob.canactral[0] = 1.0;
            g.glob.actnameral[0] = "R-Action".into();
            g.glob.actsimulral[0] = 0.0;
        }
    }
}

/// Create the knight monster in slot 0 at (x, y) and run its monster setup. Returns its id.
/// (scr_monster_makeinstance(0) with monsterinstancetype = obj_knight_enemy, monstertype 104)
pub fn create_knight(g: &mut Game, x: f64, y: f64) -> Id {
    g.glob.monstertype[0] = 104.0;
    g.glob.monstermakex[0] = x;
    g.glob.monstermakey[0] = y;
    g.glob.monster[0] = 1.0;
    let old = g.glob.monsterinstance[0];
    if old != NOONE {
        g.destroy(old);
    }
    let id = g.instance_create(x, y, Box::new(KnightEnemy::default()));
    g.glob.monsterinstance[0] = id;
    if let Some((_, k)) = g.get::<KnightEnemy>(id) {
        k.myself = 0;
    }
    // event_user(12) = Other_22: monsterx/y + scr_monstersetup. (event_user(15), recruit info, not needed.)
    g.user_event(id, 12);
    id
}

// ============================================================================ helpers

/// One obj_block_vfx spark of the block animation.
fn block_vfx(g: &mut Game, x: f64, y: f64, vspeed: f64, gravity: f64) -> Id {
    let id = g.instance_create(x, y, Box::new(BlockVfx));
    if let Some(a) = g.inst_mut(id) {
        a.sprite_index = spr("spr_roaringknight_block_vfx");
        a.image_xscale = 1.3;
        a.image_yscale = 1.3;
        a.set_vspeed(vspeed);
        a.gravity_direction = 0.0;
        a.gravity = gravity;
        a.image_speed = 0.5;
    }
    id
}

/// scr_fadeout(frames): obj_fadeout fading the screen in `frames` frames.
pub fn scr_fadeout(g: &mut Game, frames: f64) -> Id {
    let (cx, cy) = (g.camerax(), g.cameray());
    let id = g.instance_create(cx - 200.0, cy - 200.0, Box::new(FadeOut::default()));
    if let Some((i, f)) = g.get::<FadeOut>(id) {
        f.fadespeed = 1.0 / frames;
        i.depth = 3.0;
    }
    id
}

/// scr_battle_marker(x, y, sprite)
pub fn scr_battle_marker(g: &mut Game, x: f64, y: f64, sprite: Spr) -> Id {
    let id = g.instance_create(x, y, Box::new(BattleMarker::default()));
    if let Some(m) = g.inst_mut(id) {
        m.sprite_index = sprite;
        m.image_speed = 1.0;
        m.image_xscale = 2.0;
        m.image_yscale = 2.0;
    }
    id
}

/// scr_guardpeek(obj_herosusie): a defending Susie peeks out from behind her guard while talking.
fn scr_guardpeek_susie(g: &mut Game) -> Id {
    let Some(sid) = g.first("obj_herosusie") else { return NOONE };
    let Some(slot) = g.get::<Hero>(sid).map(|(_, h)| h.myself) else { return NOONE };
    let slot = (slot.max(0.0) as usize).min(2);
    if g.glob.faceaction[slot] != 4.0 {
        return NOONE;
    }
    let Some((sx, sy, sd)) = g.inst_mut(sid).map(|s| {
        s.image_alpha = 0.0;
        (s.x, s.y, s.depth)
    }) else {
        return NOONE;
    };
    let p = scr_battle_marker(g, sx + 8.0, sy + 4.0, spr("spr_susie_defend_peek"));
    if let Some((i, m)) = g.get::<BattleMarker>(p) {
        i.depth = sd;
        m.sourceobject = sid;
    }
    p
}

// ============================================================================ obj_bgfountaintest

/// obj_bgfountaintest: the fountain background of the knight fight.
pub struct BgFountainTest {
    pub siner: f64,
    pub battleprog: f64,
    pub alphafactor: f64,
    pub oceanspeed: f64,
    pub death: f64,
}
impl Default for BgFountainTest {
    fn default() -> Self { BgFountainTest { siner: 0.0, battleprog: 0.0, alphafactor: 0.0, oceanspeed: 1.0, death: 0.0 } }
}
impl Object for BgFountainTest {
    fn name(&self) -> &'static str { "obj_bgfountaintest" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        self.siner = 0.0;
        me.image_blend = make_color_rgb(0x27 as f64, 0x29 as f64, 0x3F as f64);
        self.battleprog = 0.0;
        self.alphafactor = 0.0;
        me.depth = 50000.0;
        self.oceanspeed = 1.0;
        self.death = 0.0;
        g.lerpvar(me, "alphafactor", 0.0, 1.0, 120.0, 0, "out");
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        let (cx, cy) = (g.camerax(), g.cameray());
        me.depth = 150000.0;
        self.siner += 1.0;
        let (mut shakex, mut shakey) = (0.0, 0.0);
        if let Some(sid) = g.first("obj_shake") {
            // obj_shake.shakesign isn't a declared field of the soul module's Shake; default to +1
            let sign = g.get_var(sid, "shakesign").unwrap_or(1.0);
            if let Some((_, s)) = g.get::<Shake>(sid) {
                shakex = s.shakex * sign;
                shakey = s.shakey * sign;
            }
        }
        let knight = g.exists("obj_knight_enemy");
        if knight {
            let m = g.get_first::<KnightEnemy>("obj_knight_enemy").map(|(_, k)| k.myself).unwrap_or(0);
            let (hp, maxhp) = (g.glob.monsterhp[m], g.glob.monstermaxhp[m]);
            if maxhp != 0.0 {
                self.battleprog = 1.0 - ((hp - maxhp * 0.8) / maxhp) * 5.0;
            }
        }
        self.oceanspeed = 1.0;
        if self.battleprog > 0.65 {
            self.oceanspeed = 2.0;
        }
        let s = self.siner;
        let os = self.oceanspeed;
        let af = self.alphafactor;
        let desicolor = make_color_hsv(127.5 + ((s / 90.0).sin() * 255.0) / 2.0, 255.0, 255.0);
        let fountain1 = spr("spr_bg_fountain1");
        g.draw_sprite_tiled_ext(fountain1, 0.0, shakex + cx - s * os, shakey + cy + s * os, 2.0, 2.0, C_PURPLE, 0.5 * af * (self.battleprog + 0.3));
        g.draw_sprite_tiled_ext(fountain1, 0.0, shakex + cx - (s * os) / 2.0, shakey + cy - (s * os) / 2.0, 2.0, 2.0, C_PURPLE, 0.35 * af * (self.battleprog + 0.2));
        let grad = spr("spr_bg_knight_gradient");
        if knight {
            g.draw_sprite_ext(grad, 0.0, shakex + cx + g.camerawidth() + 640.0, shakey + cy + 90.0, -2.0, 2.0, 0.0, C_BLACK, af);
        }
        g.draw_sprite_ext(grad, 0.0, shakex + cx, shakey + cy + 90.0, 2.0, 2.05, 0.0, C_BLACK, af);
        let px = spr("spr_pxwhite");
        g.draw_sprite_ext(px, 0.0, shakex + cx - 40.0, shakey + cy, 720.0, 90.0, 0.0, C_BLACK, 1.0);
        me.image_blend = merge_color(make_color_rgb(0x27 as f64, 0x29 as f64, 0x3F as f64), desicolor, self.battleprog / 2.0);
        g.draw_sprite_ext(px, 0.0, shakex + cx + 138.0 + 50.0, shakey + cy, 240.0, 90.0, 0.0, merge_color(me.image_blend, C_BLACK, 0.8), af);
        let fb = spr("spr_cc_fountainbg_white");
        for i in 1..3 {
            let i = i as f64;
            g.draw_sprite_ext(fb, s / 10.0, shakex + cx + 138.0 - (s / 20.0).sin() * (i * 12.0), shakey + cy, 2.0, 2.0, 0.0, me.image_blend, (i / 12.0) * af);
            g.draw_sprite_ext(fb, s / 10.0, shakex + cx + 138.0 + (s / 13.0).sin() * (i * 6.0), shakey + cy, 2.0, 2.0, 0.0, me.image_blend, (i / 12.0) * af);
        }
        g.draw_sprite_ext(fb, s / 10.0, shakex + cx + 138.0, shakey + cy, 2.0, 2.0, 0.0, me.image_blend, 1.0);
        if knight {
            g.draw_sprite_ext(px, 0.0, shakex + cx - 40.0, shakey + cy, 40.0, 480.0, 0.0, C_BLACK, 1.0);
            g.draw_sprite_ext(px, 0.0, shakex + cx - 40.0, shakey + cy - 20.0, 720.0, 20.0, 0.0, C_BLACK, 1.0);
        }
        if self.death == 0.0 && !g.exists("obj_battlecontroller") {
            self.death = 1.0;
            g.lerpvar(me, "alphafactor", self.alphafactor, 0.0, 30.0, 0, "out");
            g.doom(me.id, 31);
        }
    }
    obj_vars!(alphafactor, battleprog, siner);
}

// ============================================================================ obj_block_vfx

/// obj_block_vfx: spark when the knight blocks a hit.
pub struct BlockVfx;
impl Object for BlockVfx {
    fn name(&self) -> &'static str { "obj_block_vfx" }
    fn create(&mut self, me: &mut Inst, g: &mut Game) {
        me.x += -6.0 + g.random(12.0);
        me.y += -6.0 + g.random(12.0);
        me.image_index = 0.0;
    }
    fn step(&mut self, me: &mut Inst, _g: &mut Game) {
        if me.vspeed() < 0.0 {
            let v = me.vspeed() + 0.4;
            me.set_vspeed(v);
        }
        if me.vspeed() > 0.0 {
            let v = me.vspeed() - 0.4;
            me.set_vspeed(v);
        }
        if me.image_index == 2.0 {
            me.depth += 1.0;
        }
        if me.image_index == 1.0 {
            me.image_xscale = 1.0;
            me.image_yscale = 1.0;
        }
    }
    fn animation_end(&mut self, me: &mut Inst, g: &mut Game) { g.destroy_self(me); }
    obj_vars!();
}

// ============================================================================ obj_afterimage_fade_to_white

/// obj_afterimage_fade_to_white: afterimage that flashes to white, then fades.
pub struct AfterimageFadeToWhite {
    pub fade_speed: f64,
    pub con: f64,
    pub whitealpha: f64,
}
impl Default for AfterimageFadeToWhite {
    fn default() -> Self { AfterimageFadeToWhite { fade_speed: 0.13, con: 0.0, whitealpha: 0.0 } }
}
impl Object for AfterimageFadeToWhite {
    fn name(&self) -> &'static str { "obj_afterimage_fade_to_white" }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        g.draw_self(me);
        if self.con == 0.0 {
            self.whitealpha += self.fade_speed;
        }
        if self.whitealpha >= 1.0 {
            self.con = 1.0;
            self.whitealpha = 1.0;
            me.image_alpha = 0.0;
        }
        if self.con == 1.0 {
            self.whitealpha -= self.fade_speed;
            if self.whitealpha < 0.0 {
                g.destroy_self(me);
            }
        }
        g.gfx.d3d_set_fog(true, C_WHITE);
        g.draw_sprite_ext(me.sprite_index, me.image_index, me.x, me.y, me.image_xscale, me.image_yscale, me.image_angle, me.image_blend, self.whitealpha);
        g.gfx.d3d_set_fog(false, C_BLACK);
    }
    fn var(&mut self, name: &str) -> Option<&mut f64> {
        match name {
            "fadeSpeed" => Some(&mut self.fade_speed),
            "con" => Some(&mut self.con),
            "whitealpha" => Some(&mut self.whitealpha),
            _ => None,
        }
    }
    fn as_any(&mut self) -> &mut dyn std::any::Any { self }
}

// ============================================================================ obj_afterimage_grow

/// obj_afterimage_grow: afterimage that grows while fading (optionally following `target`).
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
        if self.target != NOONE && g.id_exists(self.target) {
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

// ============================================================================ obj_fadeout

/// obj_fadeout (scr_fadeout): full-screen sprite fading in by `fadespeed` per frame.
pub struct FadeOut {
    pub fadespeed: f64,
    pub length: f64,
    pub height: f64,
    pub fadein: f64,
}
impl Default for FadeOut {
    fn default() -> Self { FadeOut { fadespeed: 0.08, length: 10.0 + 640.0 / 4.0, height: 10.0 + 480.0 / 4.0, fadein: 0.0 } }
}
impl Object for FadeOut {
    fn name(&self) -> &'static str { "obj_fadeout" }
    fn create(&mut self, me: &mut Inst, _g: &mut Game) {
        me.image_blend = C_BLACK;
        self.fadespeed = 0.08;
        me.image_alpha = 0.0;
        // room_width / room_height of the battle room = 640 x 480
        self.length = 10.0 + 640.0 / 4.0;
        self.height = 10.0 + 480.0 / 4.0;
        me.x = -20.0;
        me.y = -20.0;
        self.fadein = 0.0;
    }
    fn draw(&mut self, me: &mut Inst, g: &mut Game) {
        me.image_alpha += self.fadespeed;
        g.draw_sprite_ext(me.sprite_index, me.image_index, me.x, me.y, self.length, self.height, 0.0, me.image_blend, me.image_alpha);
        if self.fadein == 1.0 && me.image_alpha <= 0.0 {
            g.destroy_self(me);
        }
    }
    obj_vars!(fadespeed, length, height, fadein);
}

// ============================================================================ obj_battle_marker

/// obj_battle_marker (scr_battle_marker): a sprite that lives during the enemy-talk phase.
pub struct BattleMarker {
    pub endanimation: Spr,
    pub sourceobject: Id,
    pub destroyoncomplete: f64,
    pub autocancel: f64,
    pub loop_: bool,
}
impl Default for BattleMarker {
    fn default() -> Self { BattleMarker { endanimation: NO_SPR, sourceobject: NOONE, destroyoncomplete: 0.0, autocancel: 1.0, loop_: false } }
}
impl Object for BattleMarker {
    fn name(&self) -> &'static str { "obj_battle_marker" }
    fn step(&mut self, me: &mut Inst, g: &mut Game) {
        if self.destroyoncomplete == 0.0 && g.glob.mnfight != 1.0 {
            g.destroy_self(me);
        }
    }
    fn destroy(&mut self, me: &mut Inst, g: &mut Game) {
        if self.destroyoncomplete == 0.0 && self.endanimation != NO_SPR {
            let n = scr_battle_marker(g, me.x, me.y, self.endanimation);
            let src = self.sourceobject;
            if let Some((i, m)) = g.get::<BattleMarker>(n) {
                m.destroyoncomplete = 1.0;
                m.sourceobject = src;
                i.depth = me.depth;
                i.image_speed = 0.5;
            }
        } else if self.sourceobject != NOONE {
            if let Some(s) = g.inst_mut(self.sourceobject) {
                s.image_alpha = 1.0;
            }
        }
    }
    fn animation_end(&mut self, me: &mut Inst, g: &mut Game) {
        if self.destroyoncomplete != 0.0 {
            g.destroy_self(me);
        } else if !self.loop_ {
            me.image_speed = 0.0;
        }
    }
    obj_vars!(destroyoncomplete, autocancel);
}
