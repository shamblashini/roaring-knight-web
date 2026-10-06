//! The Roaring Knight battle (DELTARUNE Chapter 3) rewritten in Rust for the web.

pub mod assets;
pub mod audio;
pub mod battle;
pub mod gfx;
pub mod gm;
pub mod input;
pub mod objdata;
pub mod rt;
pub mod attacks;

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub fn log(s: &str) { web_sys::console::log_1(&JsValue::from_str(s)); }

thread_local! {
    static APP: std::cell::RefCell<Option<Rc<RefCell<App>>>> = const { std::cell::RefCell::new(None) };
}

/// Debug hook for the browser console: `wasm_bindgen.debug_dump()` / window.rkDump().
#[wasm_bindgen]
pub fn debug_dump() -> String {
    APP.with(|a| a.borrow().as_ref().map(|app| app.borrow().game.debug_dump()).unwrap_or_default())
}

/// Top-level app state machine.
pub struct App {
    pub game: rt::Game,
    pub scene: battle::Scene,
    acc: f64,
    last: f64,
    /// debug: ?slow=N runs the game N times slower
    slow: f64,
}

impl App {
    fn tick(&mut self) {
        let g = &mut self.game;
        g.input.tick();
        if g.input.pressed(input::Key::Debug) {
            g.show_hitboxes = !g.show_hitboxes;
        }
        battle::scene_update(&mut self.scene, g);
        g.step();
        g.gfx.begin_frame();
        g.gfx.draw_clear_alpha(0, 1.0);
        battle::scene_draw_under(&mut self.scene, g);
        g.draw();
        battle::scene_draw_over(&mut self.scene, g);
        g.gfx.flush();
        g.purge();
    }

    fn frame(&mut self, now: f64) {
        if self.last == 0.0 {
            self.last = now;
        }
        let dt = (now - self.last).min(250.0);
        self.last = now;
        self.acc += dt;
        let step = 1000.0 / 30.0 * self.slow;
        let mut n = 0;
        while self.acc >= step && n < 4 {
            self.acc -= step;
            self.tick();
            n += 1;
        }
        if self.acc > step * 4.0 {
            self.acc = 0.0;
        }
        self.game.gfx.present();
    }
}

fn window() -> web_sys::Window { web_sys::window().unwrap() }

fn fit_canvas(canvas: &web_sys::HtmlCanvasElement) -> (u32, u32) {
    let w = window();
    let dpr = w.device_pixel_ratio();
    let cw = w.inner_width().unwrap().as_f64().unwrap();
    let ch = w.inner_height().unwrap().as_f64().unwrap();
    let pw = (cw * dpr) as u32;
    let ph = (ch * dpr) as u32;
    canvas.set_width(pw);
    canvas.set_height(ph);
    (pw, ph)
}

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(|info| {
        log(&format!("panic: {info}"));
    }));
    let doc = window().document().unwrap();
    let canvas: web_sys::HtmlCanvasElement = doc.get_element_by_id("screen").unwrap().dyn_into()?;
    fit_canvas(&canvas);
    let mut gfx = gfx::Gfx::new(&canvas);
    let assets = assets::Assets::load(&mut gfx, "assets").await?;
    let mut audio = audio::Audio::new();
    for (name, s) in &assets.sounds {
        audio.load(name, &format!("assets/{}", s.file), s.vol);
    }
    audio.load("mus_knight", "assets/mus/knight.ogg", 1.0);
    let seed = js_sys::Date::now() as u64;
    // ?mute=1: silence all audio (used for automated testing)
    let params = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|q| web_sys::UrlSearchParams::new_with_str(&q).ok());
    if params.as_ref().and_then(|p| p.get("mute")).is_some() {
        audio.master = 0.0;
        if let Some(c) = &audio.ctx {
            let _ = c.suspend();
        }
    }
    let mut game = rt::Game::new(gfx, audio, assets, seed);
    game.collision_hook = Some(battle::collision_pass);
    let scene = battle::Scene::new(&mut game);
    let slow = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|q| web_sys::UrlSearchParams::new_with_str(&q).ok())
        .and_then(|p| p.get("slow"))
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.0)
        .max(1.0);
    if let Some(el) = doc.get_element_by_id("loading") {
        el.remove();
    }
    let app = Rc::new(RefCell::new(App { game, scene, acc: 0.0, last: 0.0, slow }));
    APP.with(|a| *a.borrow_mut() = Some(app.clone()));

    // keyboard
    {
        let a = app.clone();
        let kd = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            let code = e.code();
            if let Some(k) = input::map_key(&code) {
                e.prevent_default();
                let mut app = a.borrow_mut();
                app.game.audio.resume();
                app.game.input.key_event(k, true);
            } else if code == "F4" || code == "KeyF" {
                let d = window().document().unwrap();
                if d.fullscreen_element().is_some() {
                    d.exit_fullscreen();
                } else if let Some(el) = d.document_element() {
                    let _ = el.request_fullscreen();
                }
            }
        });
        window().add_event_listener_with_callback("keydown", kd.as_ref().unchecked_ref())?;
        kd.forget();
        let a = app.clone();
        let ku = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            if let Some(k) = input::map_key(&e.code()) {
                e.prevent_default();
                a.borrow_mut().game.input.key_event(k, false);
            }
        });
        window().add_event_listener_with_callback("keyup", ku.as_ref().unchecked_ref())?;
        ku.forget();
        let a = app.clone();
        let blur = Closure::<dyn FnMut()>::new(move || a.borrow_mut().game.input.clear());
        window().add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref())?;
        blur.forget();
        let a = app.clone();
        let c2 = canvas.clone();
        let rs = Closure::<dyn FnMut()>::new(move || {
            let (w, h) = fit_canvas(&c2);
            a.borrow_mut().game.gfx.resize_canvas(w, h);
        });
        window().add_event_listener_with_callback("resize", rs.as_ref().unchecked_ref())?;
        rs.forget();
    }

    // animation loop
    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g2 = f.clone();
    let a = app.clone();
    *g2.borrow_mut() = Some(Closure::new(move |now: f64| {
        a.borrow_mut().frame(now);
        window().request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref()).unwrap();
    }));
    window().request_animation_frame(g2.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;

    // debug: ?bg=1 also drives the loop from a timer while the page is hidden (browsers pause
    // requestAnimationFrame for hidden pages), so automated tests keep running.
    let bg = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|q| web_sys::UrlSearchParams::new_with_str(&q).ok())
        .and_then(|p| p.get("bg"))
        .is_some();
    if bg {
        let a = app.clone();
        let tick = Closure::<dyn FnMut()>::new(move || {
            let hidden = window().document().map(|d| d.hidden()).unwrap_or(false);
            if hidden {
                let now = window().performance().map(|p| p.now()).unwrap_or(0.0);
                if let Ok(mut app) = a.try_borrow_mut() {
                    app.frame(now);
                }
            }
        });
        window().set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), 16)?;
        tick.forget();
    }
    Ok(())
}
