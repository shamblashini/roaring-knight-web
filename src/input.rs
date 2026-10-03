//! Keyboard state, sampled once per 30fps game tick.
//! Deltarune bindings: Z/Enter = confirm (button1), X/Shift = cancel (button2), C/Ctrl = menu (button3).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    B1,
    B2,
    B3,
    Debug,
    /// Escape: hold to quit back to the title (obj_time quit_timer)
    Quit,
}
const N: usize = 9;

#[derive(Default)]
pub struct Input {
    raw: [bool; N],
    /// went down at least once since the last tick (catches taps shorter than a tick)
    tapped: [bool; N],
    released_raw: [bool; N],
    held: [bool; N],
    pressed: [bool; N],
    released: [bool; N],
}

pub fn map_key(code: &str) -> Option<Key> {
    Some(match code {
        "ArrowLeft" | "KeyA" => Key::Left,
        "ArrowRight" | "KeyD" => Key::Right,
        "ArrowUp" | "KeyW" => Key::Up,
        "ArrowDown" | "KeyS" => Key::Down,
        "KeyZ" | "Enter" | "NumpadEnter" => Key::B1,
        "KeyX" | "ShiftLeft" | "ShiftRight" => Key::B2,
        "KeyC" | "ControlLeft" | "ControlRight" => Key::B3,
        "Backquote" => Key::Debug,
        "Escape" => Key::Quit,
        _ => return None,
    })
}

impl Input {
    pub fn key_event(&mut self, k: Key, down: bool) {
        let i = k as usize;
        if down && !self.raw[i] {
            self.tapped[i] = true;
        }
        if !down && self.raw[i] {
            self.released_raw[i] = true;
        }
        self.raw[i] = down;
    }
    /// Call once at the start of every tick.
    pub fn tick(&mut self) {
        for i in 0..N {
            self.pressed[i] = self.tapped[i];
            self.released[i] = self.released_raw[i];
            self.held[i] = self.raw[i] || self.tapped[i];
            self.tapped[i] = false;
            self.released_raw[i] = false;
        }
    }
    pub fn held(&self, k: Key) -> bool { self.held[k as usize] }
    pub fn pressed(&self, k: Key) -> bool { self.pressed[k as usize] }
    pub fn released(&self, k: Key) -> bool { self.released[k as usize] }
    pub fn any_pressed(&self) -> bool { self.pressed.iter().any(|&p| p) }
    pub fn clear(&mut self) {
        *self = Input::default();
    }
}
