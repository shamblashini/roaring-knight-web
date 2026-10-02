# Porting GML to this runtime

The decompiled chapter 3 code is in `extract/dump/CodeEntries/` (`gml_Object_<obj>_<Event>_<n>.gml`,
`gml_GlobalScript_<script>.gml`). Object metadata (default sprite, mask, parent) is in `extract/raw/objects.json`
and `src/objdata.rs`; sprite metadata in `extract/raw/sprites.json`. **Never touch the game install or saves.**

The goal is a faithful, line-by-line port: same frame counts, same numbers, same order of operations.
The game runs at **30 fps**; every timer is in frames. Room/view is 640x480 with the camera at (0,0).

## Objects

Each GML object becomes a Rust struct implementing `crate::rt::Object`:

```rust
use crate::rt::{Game, Id, Inst, Object, NOONE};
use crate::assets::{spr, Spr, NO_SPR};
use crate::gm::*;              // lengthdir_x, point_direction, lerp, scr_approach, easing, ...
use crate::gfx::{self, bm, Color, C_WHITE, C_BLACK, C_RED, merge_color, make_color_rgb};
use crate::battle::{self, RegVars, KnightEnemy, Heart, Growtangle, DbCtrl};
use crate::obj_vars;

#[derive(Default)]
pub struct KnightFoo { pub timer: f64, pub rb: RegVars, pub master: Id /* ... */ }

impl Object for KnightFoo {
    fn name(&self) -> &'static str { "obj_knight_foo" }          // exact GML object name
    fn create(&mut self, me: &mut Inst, g: &mut Game) { ... }     // Create_0
    fn step(&mut self, me: &mut Inst, g: &mut Game) { ... }       // Step_0
    fn begin_step / end_step                                       // Step_1 / Step_2
    fn draw(&mut self, me: &mut Inst, g: &mut Game) { ... }       // Draw_0 (default = draw_self)
    fn draw_end(...)                                               // Draw_73
    fn alarm(&mut self, n: usize, me, g)                           // Alarm_n
    fn user(&mut self, n: usize, me, g)                            // Other_(10+n) i.e. event_user(n)
    fn destroy / cleanup / animation_end                           // Destroy_0 / CleanUp_0 / Other_7
    obj_vars!(timer, foo);   // REQUIRED: implements as_any + `var(name)` for lerpvar'd f64 fields
}
```

* `me: &mut Inst` holds builtins: `x y xstart ystart xprevious yprevious friction gravity gravity_direction
  sprite_index image_index image_speed image_xscale image_yscale image_angle image_alpha image_blend
  mask_index depth visible alarm[12]` and bullet vars `damage grazed grazetimer target inv active
  destroyonhit grazepoints timepoints element updateimageangle` (all `f64` except sprites/colours/bool visible).
* `speed/direction/hspeed/vspeed` are coupled: read with `me.speed()`, `me.direction()`, `me.hspeed()`,
  `me.vspeed()`; write with `me.set_speed(v)`, `me.set_direction(v)`, `me.set_hspeed(v)`, `me.set_vspeed(v)`,
  `me.motion_add(dir, spd)`, `me.move_towards_point(x, y, sp)`.
* Default sprite/mask/visible/depth come from the object table automatically on create.
* GML `sprite_width` → `g.sprite_width(me)`; `sprite_get_width(s)` → `g.sprite_get_width(s)`, etc.
* Object custom variables → struct fields. Prefer `f64` for numbers, `bool` only when the GML treats it as a
  bool and nothing lerps it, `Id` (i64) for instance references (`NOONE` = -4), `Spr` (i32) for sprites.
  Any field that GML code animates with `scr_lerpvar`/`scr_lerp_var_instance`/`scr_var_delayed` **must** be an
  `f64` listed in `obj_vars!`.
* Parent events: if a child lacks an event, the parent's runs. `event_inherited()` = call the parent's code
  explicitly. For obj_regularbullet children embed `rb: RegVars` and call `battle::regularbullet_create(me, &mut self.rb, g)`
  / `battle::regularbullet_step(me, &mut self.rb, g)`. obj_collidebullet's Other_15 (bullet hits the SOUL) is the
  default `user(5)`; only override `user` when the GML object defines its own `Other_15` (and fall back to
  `battle::collidebullet_hit(me, g)` for n == 5 when it calls event_inherited). If your `user()` override handles
  other numbers, keep `5 => battle::collidebullet_hit(me, g)` for objects that inherit Other_15.
  For any other parent (obj_bulletparent, obj_collidebullet) there is no Create/Step code to inherit, but call
  `battle::bullet_init(me)` wherever GML calls `scr_bullet_init()`.

## Instances

| GML | Rust |
|---|---|
| `instance_create(x, y, obj_foo)` | `let id = g.instance_create(x, y, Box::new(Foo::default()));` (Create runs immediately) |
| `instance_create_depth(x, y, d, obj)` | `g.instance_create_depth(x, y, d, Box::new(...))` |
| `inst.var = v` after create | `if let Some((i, o)) = g.get::<Foo>(id) { i.x = ..; o.timer = ..; }` |
| builtins only | `g.inst_mut(id)` / `g.inst(id)` |
| `with (inst) { ... }` needing `g` | `g.with_t::<Foo, _>(id, \|me, o, g\| { ... })` or untyped `g.with(id, \|me, o, g\| ...)` |
| `with (obj_foo) {...}` | `for id in g.ids_of("obj_foo") { ... }` or `g.with_all("obj_foo", \|me, o, g\| ...)` |
| `obj_heart.x` | `g.first_inst("obj_heart").map(\|h\| h.x).unwrap_or(0.)` |
| `instance_exists(obj)` / `i_ex(obj)` | `g.exists("obj_foo")` |
| `i_ex(id)` | `g.id_exists(id)` |
| `instance_number(obj)` | `g.instance_number("obj_foo")` |
| `instance_destroy()` (self) | `g.destroy_self(me); return;` (GML continues after destroy; only `return` if the GML `exit`s) |
| `instance_destroy(id)` / `with (x) instance_destroy()` | `g.destroy(id)` / `g.destroy_all("obj_foo")` |
| `event_user(n)` on self | `self.user(n, me, g)` ; on other: `g.user_event(id, n)` |
| `other` in collision/with | pass the needed values explicitly |

Notes: while an instance's event runs, its own slot is checked out — `g.get/inst/with(own id)` return `None`.
Always use `me`/`self` for your own fields. `g.ids_of/exists/instance_number` do include the running instance.

## Helpers (on `g: &mut Game`)

* random: `g.random(n)`, `g.random_range(a,b)`, `g.irandom(n)`, `g.irandom_range(a,b)`, `g.choose(&[..])`.
  (choose with mixed types → pick an index.)
* view: `g.camerax()`, `g.cameray()`, `g.camerawidth()`, `g.cameraheight()`; `__view_get(XView)` = `g.camerax()`.
* `scr_lerpvar("var", a, b, t, [ease, "out"])` → `g.lerpvar(me, "var", a, b, t, ease, "out")` (ease 0 = linear).
  `scr_lerpvar_instance(id, ...)` / `scr_lerp_var_instance` → `g.lerpvar_instance(id, "var", a, b, t, ease, "in")`.
  The target var name must be a builtin (`x`,`y`,`image_alpha`,`image_xscale`,`image_angle`,`speed`,`direction`,
  `hspeed`,`vspeed`,`image_blend`,`visible`, ...) or a field listed in `obj_vars!`.
* `scr_script_delayed(fn, delay, args...)` → `g.script_delayed(me.id, delay, move |g| { ... })` (closure runs only
  if the target instance still exists). `scr_var_delayed("v", val, t)` → `g.var_delayed(me.id, "v", val, t)`.
* `scr_doom(id, t)` → `g.doom(id, t)`; `scr_marker(x,y,spr)` → `g.marker(x, y, spr)`; `scr_afterimage()` →
  `g.afterimage(me)` (returns id; obj_afterimage fades by its `fadeSpeed` var — `g.set_var(id,"fadeSpeed",v)`).
* sound: `g.snd_play("snd_x")`, `g.snd_play_x("snd_x", vol, pitch)`, `g.snd_play_pitch`, `g.snd_stop("snd_x")`,
  `g.snd_loop`, `g.audio.volume(handle, vol, frames)`, `g.audio.pitch(handle, p)`, `g.audio.stop_handle(h)`.
  Sound names must be string literals in the source (the asset packer greps for `"snd_..."`).
* sprites: `spr("spr_name")` — **always a string literal** (the asset packer greps `"spr_..."`). Cache in fields
  if used every frame is not necessary (lookup is a hash).
* collisions: `g.place_meeting(me, x, y, "obj")`, `g.instance_place`, `g.collision_rectangle(x1,y1,x2,y2,"obj",Some(me.id))`,
  `g.collision_line`, `g.collision_circle`, `g.collision_point`; `g.bbox(me)` → `[left, top, right, bottom]`.
* globals: `g.glob.turntimer`, `g.glob.inv`, `g.glob.invc`, `g.glob.time`, `g.glob.tension`, `g.glob.hp[charid]`, ...
* battle: `battle::scr_get_box(g, n)`, `battle::scr_damage(me, g)`, `battle::scr_damage_all(me, g)`,
  `battle::scr_tensionheal(g, amt)`, `battle::scr_bullet_inherit(&src_inst, dst_id, g)`,
  `battle::scr_childbullet(g, me, x, y, Box::new(..))`, `battle::scr_fire_bullet(g, x, y, Box::new(..), dir, spd, Some(spr), upd_angle, inherit_from, depth)`.
  The knight: `g.get_first::<KnightEnemy>("obj_knight_enemy")` (fields named as in GML). The SOUL: `obj_heart` /
  `Heart` (`x,y` = top-left; GML often uses `obj_heart.x + 10`). The box: `obj_growtangle` / `Growtangle`.

## Drawing (in `draw`)

`g.draw_self(me)`, `g.draw_sprite_ext(spr, sub, x, y, xs, ys, rot, col, alpha)`, `g.draw_sprite(spr, sub, x, y)`,
`g.draw_sprite_part_ext(spr, sub, l, t, w, h, x, y, xs, ys, col, a)`, `g.draw_sprite_general(spr, sub, l, t, w, h, x, y, xs, ys, rot, [c1,c2,c3,c4], a)`,
`g.draw_sprite_tiled_ext`, `g.draw_sprite_stretched_ext`, text: `g.draw_text`, `g.draw_set_font(font("fnt_main"))`.
Primitives / state are on `g.gfx`: `draw_set_color`, `draw_set_alpha`, `draw_rectangle(x1,y1,x2,y2,outline)`,
`draw_rectangle_color`, `draw_line_width_color`, `draw_triangle_color`, `draw_circle_color`, `gpu_set_blendmode(bm::ADD)`,
`draw_set_blend_mode`, `gpu_set_blendmode_ext(bm::ONE, bm::ZERO)`, `gpu_set_blendmode_ext_sepalpha`, `gpu_set_blendenable`,
`d3d_set_fog(true, col)` / `gpu_set_fog`, `gpu_set_colorwriteenable`, `gpu_set_alphatestenable`, surfaces:
`surface_create(w,h) -> i32`, `surface_exists`, `surface_set_target`, `surface_reset_target`, `surface_free`,
`draw_clear_alpha(col, a)`, `draw_surface`, `draw_surface_ext`, `draw_surface_part_ext`, `surface_copy`, `surface_resize`.
`sprite_create_from_surface` → `g.sprite_create_from_surface(surf, x, y, w, h, xorig, yorig)`; `sprite_delete` → `g.sprite_delete`.
Primitive lists (`draw_primitive_begin/draw_vertex/end`) → build triangles with `g.gfx.draw_triangle_color` or `g.gfx.tri`.
`application_surface` → `g.gfx.app_surface`. Colours are GameMaker BGR ints (`c_red == 0x0000FF`).
Remember to restore GPU state exactly where the GML does.

## Numbers

GML numbers are f64. Truthiness: nonzero = true. `round()` is banker's rounding → `gm::round`. `a % b` keeps the
sign of `a` (Rust `%` on f64 matches). `div` → `(a / b).trunc()`. `irandom(n)` is inclusive. Angles in degrees.
`choose(a, b, c)` → `g.choose(&[a, b, c])`.

## Build

`cargo build --target wasm32-unknown-unknown` must compile without errors. Keep warnings low. Do not edit files
outside the ones you were assigned, except to add your `pub mod` line / match arm where instructed.
