//! GameMaker math helpers (degrees, y-down screen space).

pub use std::f64::consts::PI;

#[inline]
pub fn degtorad(d: f64) -> f64 { d.to_radians() }
#[inline]
pub fn radtodeg(r: f64) -> f64 { r.to_degrees() }
#[inline]
pub fn dsin(d: f64) -> f64 { d.to_radians().sin() }
#[inline]
pub fn dcos(d: f64) -> f64 { d.to_radians().cos() }
#[inline]
pub fn lengthdir_x(len: f64, dir: f64) -> f64 { len * dcos(dir) }
#[inline]
pub fn lengthdir_y(len: f64, dir: f64) -> f64 { -len * dsin(dir) }
pub fn point_direction(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let d = (-(y2 - y1)).atan2(x2 - x1).to_degrees();
    if d < 0.0 { d + 360.0 } else { d }
}
#[inline]
pub fn point_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 { ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt() }
/// GameMaker angle_difference(a, b): signed shortest difference a - b in (-180, 180].
pub fn angle_difference(a: f64, b: f64) -> f64 { (a - b + 540.0).rem_euclid(360.0) - 180.0 }
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }
#[inline]
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 { v.max(lo).min(hi) }
#[inline]
pub fn sign(v: f64) -> f64 {
    if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 }
}
#[inline]
pub fn sqr(v: f64) -> f64 { v * v }
#[inline]
pub fn frac(v: f64) -> f64 { v - v.trunc() }
/// GameMaker round(): round half to even.
#[inline]
pub fn round(v: f64) -> f64 { v.round_ties_even() }
#[inline]
pub fn floor(v: f64) -> f64 { v.floor() }
#[inline]
pub fn ceil(v: f64) -> f64 { v.ceil() }
#[inline]
pub fn abs(v: f64) -> f64 { v.abs() }
#[inline]
pub fn power(a: f64, b: f64) -> f64 { a.powf(b) }
/// GML `%` / `mod`: sign follows the dividend (like Rust's `%`).
#[inline]
pub fn gmod(a: f64, b: f64) -> f64 { if b == 0.0 { 0.0 } else { a % b } }
#[inline]
pub fn min(a: f64, b: f64) -> f64 { a.min(b) }
#[inline]
pub fn max(a: f64, b: f64) -> f64 { a.max(b) }
#[inline]
pub fn truthy(v: f64) -> bool { v >= 0.5 }

pub fn scr_approach(a: f64, b: f64, amt: f64) -> f64 {
    if a < b {
        let r = a + amt;
        if r > b { b } else { r }
    } else {
        let r = a - amt;
        if r < b { b } else { r }
    }
}
pub fn scr_movetowards(a: f64, b: f64, amt: f64) -> f64 {
    if a == b { a } else if a > b { (a - amt).max(b) } else { (a + amt).min(b) }
}

// ---- easing (scr_ease_in / scr_ease_out / scr_ease_inout and the ease_* helpers they call)
fn ease_out_bounce(t: f64) -> f64 {
    let (n1, d1) = (7.5625, 2.75);
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}
fn ease_in_bounce(t: f64) -> f64 { 1.0 - ease_out_bounce(1.0 - t) }
fn ease_out_elastic(t: f64) -> f64 {
    if t <= 0.0 { return 0.0; }
    if t >= 1.0 { return 1.0; }
    let p = 0.3;
    let s = p / 4.0;
    2f64.powf(-10.0 * t) * ((t - s) * (2.0 * PI) / p).sin() + 1.0
}
fn ease_in_elastic(t: f64) -> f64 {
    if t <= 0.0 { return 0.0; }
    if t >= 1.0 { return 1.0; }
    let p = 0.3;
    let s = p / 4.0;
    let t = t - 1.0;
    -(2f64.powf(10.0 * t) * ((t - s) * (2.0 * PI) / p).sin())
}
fn ease_out_back(t: f64) -> f64 {
    let s = 1.70158;
    let t = t - 1.0;
    t * t * ((s + 1.0) * t + s) + 1.0
}
fn ease_inout_back(t: f64) -> f64 {
    let s = 1.70158 * 1.525;
    let t = t * 2.0;
    if t < 1.0 {
        0.5 * (t * t * ((s + 1.0) * t - s))
    } else {
        let t = t - 2.0;
        0.5 * (t * t * ((s + 1.0) * t + s) + 2.0)
    }
}

pub fn scr_ease_out(t: f64, e: i32) -> f64 {
    if !(-3..=7).contains(&e) {
        return t;
    }
    match e {
        -3 => ease_out_bounce(t),
        -2 => ease_out_elastic(t),
        -1 => ease_out_back(t),
        0 => t,
        1 => (t * PI / 2.0).sin(),
        2 => -t * (t - 2.0),
        6 => -(2f64.powf(-10.0 * t)) + 1.0,
        7 => {
            let t = t - 1.0;
            (1.0 - t * t).sqrt()
        }
        _ => {
            let t = t - 1.0;
            if e == 4 { -1.0 * (t.powi(e) - 1.0) } else { t.powi(e) + 1.0 }
        }
    }
}
pub fn scr_ease_in(t: f64, e: i32) -> f64 {
    if !(-3..=7).contains(&e) {
        return t;
    }
    match e {
        -3 => ease_in_bounce(t),
        -2 => ease_in_elastic(t),
        -1 => {
            let s = 1.70158;
            t * t * ((s + 1.0) * t - s)
        }
        0 => t,
        1 => -(t * PI / 2.0).cos() + 1.0,
        6 => 2f64.powf(10.0 * (t - 1.0)),
        7 => -((1.0 - t * t).sqrt() - 1.0),
        _ => t.powi(e),
    }
}
pub fn scr_ease_inout(t: f64, e: i32) -> f64 {
    if !(-3..=7).contains(&e) {
        return t;
    }
    match e {
        -3 => {
            if t < 0.5 { ease_in_bounce(t * 2.0) * 0.5 } else { ease_out_bounce(t * 2.0 - 1.0) * 0.5 + 0.5 }
        }
        -2 => {
            if t < 0.5 { ease_in_elastic(t * 2.0) * 0.5 } else { ease_out_elastic(t * 2.0 - 1.0) * 0.5 + 0.5 }
        }
        -1 => ease_inout_back(t),
        // the game's own formula (sic)
        1 => -0.5 * ((PI * t) - 1.0).cos(),
        0 => t,
        _ => {
            let t = t * 2.0;
            if t < 1.0 { 0.5 * scr_ease_in(t, e) } else { 0.5 * (scr_ease_out(t - 1.0, e) + 1.0) }
        }
    }
}
pub fn lerp_ease_out(a: f64, b: f64, t: f64, e: i32) -> f64 { lerp(a, b, scr_ease_out(t, e)) }
pub fn lerp_ease_in(a: f64, b: f64, t: f64, e: i32) -> f64 { lerp(a, b, scr_ease_in(t, e)) }
pub fn lerp_ease_inout(a: f64, b: f64, t: f64, e: i32) -> f64 { lerp(a, b, scr_ease_inout(t, e)) }

/// Deterministic xorshift RNG standing in for GameMaker's random functions.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Rng { Rng(seed | 1) }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// random(n): [0, n)
    pub fn random(&mut self, n: f64) -> f64 { (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 * n }
    pub fn random_range(&mut self, a: f64, b: f64) -> f64 { a + self.random(b - a) }
    /// irandom(n): integer in [0, n]
    pub fn irandom(&mut self, n: f64) -> f64 {
        let n = n.floor();
        if n <= 0.0 { 0.0 } else { (self.random(n + 1.0)).floor().min(n) }
    }
    pub fn irandom_range(&mut self, a: f64, b: f64) -> f64 {
        let (lo, hi) = (a.min(b).round(), a.max(b).round());
        lo + self.irandom(hi - lo)
    }
    pub fn choose<T: Copy>(&mut self, opts: &[T]) -> T { opts[(self.random(opts.len() as f64) as usize).min(opts.len() - 1)] }
}
