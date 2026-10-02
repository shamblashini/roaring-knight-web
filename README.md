# The Roaring Knight — web port

A Rust/WebAssembly rewrite of the Roaring Knight battle from the end of DELTARUNE Chapter 3, ported from the
game's own (decompiled) GameMaker code so timings, patterns and numbers match the original.

The repository contains **no game assets**. They are extracted from your own installed copy of DELTARUNE and
stay on your machine (`extract/` and `assets/` are git-ignored). Don't publish a build that includes them.

## Requirements

* Rust with the `wasm32-unknown-unknown` target, [`trunk`](https://trunkrs.dev)
* .NET runtime + the UndertaleModTool CLI (`UTMT_CLI` env var, default `~/.cache/kf-tools/utmt/UndertaleModCli`)
* Python 3 with Pillow
* DELTARUNE installed (`DELTARUNE_DIR`, default `~/.steam/steam/steamapps/common/DELTARUNE`)

## Build

```bash
tools/extract.sh          # copies chapter 3 data from the install (read-only) and decompiles/exports it
python3 tools/pack_assets.py   # packs the sprites/sounds the code references into assets/
trunk serve --release     # http://127.0.0.1:8080
```

`trunk build --release` produces a static site in `dist/`.

## Controls

| key | action |
|---|---|
| arrows / WASD | move, navigate menus |
| Z / Enter | confirm |
| X / Shift | cancel, slow the SOUL while held |
| C / Ctrl | menu |
| F | fullscreen |
| ` (backquote) | toggle hitbox overlay |

## Layout

* `src/rt.rs` — small GameMaker-like runtime (instances, events, motion, collision masks, lerpvar, drawing)
* `src/gfx.rs` — WebGL2 renderer with GameMaker blend modes, fog, surfaces
* `src/audio.rs` — WebAudio playback with per-instance volume/pitch
* `src/battle/` — the Deltarune battle system (SOUL & board, heroes, menus, text writer, spells, the Knight)
* `src/attacks/` — the Knight's attack patterns, one module per bullet type
* `docs/PORTING.md`, `docs/BATTLE.md` — conventions used for the port
* `tools/` — extraction (UndertaleModTool scripts), atlas packer, object table generator
