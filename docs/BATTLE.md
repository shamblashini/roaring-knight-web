# Battle core architecture

Read `docs/PORTING.md` first — the same porting rules apply. The battle core is the generic Deltarune battle
system (as used by encounter 115 = the Roaring Knight, chapter 3), ported object-by-object like the attacks.
Chapter-specific branches for OTHER encounters (Tenna, Rouxls, minigames, board rooms, chapter 1/2/4, Japanese
text, debug keys) are dropped. `tools/prune_gml.py <file>` prints a GML file with many such branches stripped
(it is a reading aid only — it can be wrong; check the original when in doubt).

## Modules and owners

Each module is owned by exactly one agent. Its stub already declares the structs/functions other modules use.
**Keep every pre-declared item's name, signature and existing pub fields** (others code against them); add
whatever else you need *inside your own file*. Do not edit other files.

| module | GML |
|---|---|
| `battle/soul.rs` | obj_heart, obj_grazebox, obj_growtangle (+ obj_battlesolid behaviour), obj_moveheart, obj_returnheart, obj_heartburst, obj_darkener, obj_shake; scr_damage (hero damage incl. all knight branches), scr_dead, scr_moveheart; `collision_pass` = obj_heart Collision obj_collidebullet + obj_grazebox Collision obj_collidebullet + any other collision events of these objects |
| `battle/knight.rs` | obj_knight_enemy (Create, Step_0, Step_2, Draw_0, Alarm_4, Alarm_6, Other_10/12/13/14/22, CleanUp), scr_monstersetup type 104, scr_enemy_object_init, scr_enemy_hurt (+scr_enemy_drawidle_generic if used), obj_bgfountaintest, obj_block_vfx, obj_afterimage_fade_to_white, obj_afterimage_grow if not elsewhere |
| `battle/heroes.rs` | obj_heroparent + obj_herokris/obj_herosusie/obj_heroralsei (all events, incl. the hero attack that calls scr_damage_enemy with the knight's damage reductions), obj_attackpress, obj_dmgwriter, obj_tensionbar, scr_damage_enemy, scr_monsterdefeat (stub-level) |
| `battle/controller.rs` | obj_battlecontroller (Create/Step/Draw/Alarms), scr_charbox, scr_selectionmatrix, scr_nexthero, scr_prevhero, scr_endturn, scr_attackphase, scr_nextact, scr_act_simul, scr_mnendturn, scr_randomtarget, scr_battle/scr_encountersetup(115) setup, scr_battlecursor_memory_reset |
| `battle/spells.rs` | scr_spellmenu_setup, scr_spellinfo, scr_spellconsumeb, scr_spell (all spells Susie/Ralsei have: Rude Buster, Heal Prayer, Pacify, and whatever `global.spell` gives them), obj_spellphase, spell effect objects (e.g. obj_rudebuster_bolt, obj_healanim ...), scr_iteminfo/scr_iteminfo_temp, scr_itemuse, scr_heal, item effects |
| `battle/writer.rs` | obj_writer (typewriter, control codes `&` newline, `/` wait-for-input, `%` close, `^n` delays, `\E`/`\F` faces, `\c` colours, etc.), obj_face (+ obj_smallface if needed), obj_battleblcon, scr_speaker, scr_anyface / scr_anyface_next, msgset/msgnext, scr_battletext, scr_battletext_default, scr_enemyblcon, scr_terminate_writer, text sounds (typer → sound/font) |
| `battle/scene.rs` + `lib.rs` | (lead) title screen, music, calling `controller::start_battle`, ending/game-over screens |

## Shared state

All `global.*` battle variables live in `g.glob` (see `Glob` in `src/rt.rs`). Anything not there →
`g.glob.ex("name")` / `g.glob.set_ex("name", v)` (prefer a field in your own struct when it's object-local).
`global.charinstance[i]` → `g.glob.charinstance[i]` (ids of the Hero instances), `global.monsterinstance[0]` →
`g.glob.monsterinstance[0]` (the knight).

## Fixed setup (encounter 115, chapter 3 defaults — no save file is read)

* Party: Kris (char 1) HP 160, AT 14, DF 2, MAG 0, weapon 16 MechaSaber (AT+4), armors 1 Amber Card (DF+1) & 10
  GlowWrist (DF+2). Susie (char 2) HP 190, AT 18, DF 2, MAG 2, weapon 17 AutoAxe (AT+4), armors 1 & 10.
  Ralsei (char 3) HP 140, AT 12, DF 2, MAG 11, weapon 18 FiberScarf (AT+3, MAG+2), armors 1 & 10.
  So battleat = [18, 22, 15], battledf = [5, 5, 5], battlemag = [0, 2, 13].
  Spells (`global.spell`): Kris [7], Susie [4, 11], Ralsei [3, 2] (see scr_gamestart; resolve names via scr_spellinfo).
* Inventory (`global.item`, ids from scr_iteminfo): 2 ReviveMint, 8 Darkburger, 16 CD Bagel, 11 ClubsSandwich,
  34 TVDinner, 38 ExecBuffet, 27 TensionBit, 39 DeluxeDinner, 24 ButJuice, 23 LightCandy, 37 TVSlop, 22 DD-Burger.
* Encounter 115: heromakex/y = (126,104), (80,142), (58,190); knight at monstermakex/y (425, 78); battlemsg
  "* The Roaring Knight appeared."; music "mus_knight" (assets/mus/knight.ogg, looping, volume 0.7 per controller
  create; play with `g.audio.play_ext("mus_knight", 0.7, 1.0, true)` and store in `g.glob.batmusic`).
* Camera at (0, 0); 640x480.

## Turn flow (as in GML)

`myfight 0` menu → (`myfight 3` ACTs) → `scr_attackphase` → `myfight 4` obj_spellphase (items/spells) →
`myfight 1` obj_attackpress (FIGHT bars, hero attacks) → `mnfight 1` enemy talk (knight Step, Susie balloons) →
`mnfight 1.5` → knight creates obj_growtangle + SOUL + obj_dbulletcontroller (`battle::spawn_dbulletcontroller`),
`mnfight 2` bullets: controller decrements `global.turntimer`; at <= 0 it destroys obj_bulletparent &
obj_bulletgenparent, the SOUL returns (obj_returnheart), growtangle shrinks, then `scr_mnendturn` → next turn.
The knight's attack selection and its use of `obj_dbulletcontroller` are in `battle::api` (`spawn_dbulletcontroller`
creates the controller with `type`, `difficulty`, optional `damage`; attacks are in `src/attacks/`).

## Done

`cargo build --target wasm32-unknown-unknown` with no errors and no warnings from your file; commit in your
worktree (`git -c commit.gpgsign=false commit -am "..."`, message ending with
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`). Report: what you ported, what you skipped, cross-module
assumptions (calls you make into other modules' stubs and what you expect them to do), and TODOs.
