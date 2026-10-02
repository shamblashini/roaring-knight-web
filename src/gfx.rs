//! Minimal GameMaker-style 2D renderer on WebGL2.
//! Colours use GameMaker's BGR integer layout (c_red = 0x0000FF).

use wasm_bindgen::JsCast;
use web_sys::{WebGl2RenderingContext as GL, WebGlBuffer, WebGlFramebuffer, WebGlProgram, WebGlTexture, WebGlUniformLocation};

pub type Color = u32;

pub const C_WHITE: Color = 0xFFFFFF;
pub const C_BLACK: Color = 0x000000;
pub const C_RED: Color = 0x0000FF;
pub const C_LIME: Color = 0x00FF00;
pub const C_BLUE: Color = 0xFF0000;
pub const C_YELLOW: Color = 0x00FFFF;
pub const C_AQUA: Color = 0xFFFF00;
pub const C_FUCHSIA: Color = 0xFF00FF;
pub const C_GRAY: Color = 0x808080;
pub const C_DKGRAY: Color = 0x404040;
pub const C_LTGRAY: Color = 0xC0C0C0;
pub const C_ORANGE: Color = 0x40A0FF;
pub const C_PURPLE: Color = 0x800080;
pub const C_MAROON: Color = 0x000080;
pub const C_NAVY: Color = 0x800000;
pub const C_GREEN: Color = 0x008000;
pub const C_OLIVE: Color = 0x008080;
pub const C_TEAL: Color = 0x808000;
pub const C_SILVER: Color = 0xC0C0C0;

pub fn make_color_rgb(r: f64, g: f64, b: f64) -> Color {
    let c = |v: f64| (v.round().clamp(0.0, 255.0)) as u32;
    c(r) | (c(g) << 8) | (c(b) << 16)
}
pub fn color_get_red(c: Color) -> f64 { (c & 0xFF) as f64 }
pub fn color_get_green(c: Color) -> f64 { ((c >> 8) & 0xFF) as f64 }
pub fn color_get_blue(c: Color) -> f64 { ((c >> 16) & 0xFF) as f64 }
pub fn merge_color(a: Color, b: Color, t: f64) -> Color {
    let l = |x: f64, y: f64| x + (y - x) * t;
    make_color_rgb(
        l(color_get_red(a), color_get_red(b)),
        l(color_get_green(a), color_get_green(b)),
        l(color_get_blue(a), color_get_blue(b)),
    )
}
pub fn make_color_hsv(h: f64, s: f64, v: f64) -> Color {
    // GameMaker HSV components are 0..255
    let h = (h / 255.0 * 360.0).rem_euclid(360.0);
    let s = s / 255.0;
    let v = v / 255.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    make_color_rgb((r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0)
}

/// GameMaker blend factor constants.
pub mod bm {
    pub const ZERO: i32 = 1;
    pub const ONE: i32 = 2;
    pub const SRC_COLOUR: i32 = 3;
    pub const INV_SRC_COLOUR: i32 = 4;
    pub const SRC_ALPHA: i32 = 5;
    pub const INV_SRC_ALPHA: i32 = 6;
    pub const DEST_ALPHA: i32 = 7;
    pub const INV_DEST_ALPHA: i32 = 8;
    pub const DEST_COLOUR: i32 = 9;
    pub const INV_DEST_COLOUR: i32 = 10;
    pub const SRC_ALPHA_SAT: i32 = 11;
    // gpu_set_blendmode modes
    pub const NORMAL: i32 = 0;
    pub const ADD: i32 = 1;
    pub const MAX: i32 = 2;
    pub const SUBTRACT: i32 = 3;
}

fn gl_factor(f: i32) -> u32 {
    match f {
        bm::ZERO => GL::ZERO,
        bm::ONE => GL::ONE,
        bm::SRC_COLOUR => GL::SRC_COLOR,
        bm::INV_SRC_COLOUR => GL::ONE_MINUS_SRC_COLOR,
        bm::SRC_ALPHA => GL::SRC_ALPHA,
        bm::INV_SRC_ALPHA => GL::ONE_MINUS_SRC_ALPHA,
        bm::DEST_ALPHA => GL::DST_ALPHA,
        bm::INV_DEST_ALPHA => GL::ONE_MINUS_DST_ALPHA,
        bm::DEST_COLOUR => GL::DST_COLOR,
        bm::INV_DEST_COLOUR => GL::ONE_MINUS_DST_COLOR,
        bm::SRC_ALPHA_SAT => GL::SRC_ALPHA_SATURATE,
        _ => GL::ONE,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct State {
    tex: usize,
    blend: [i32; 4],
    blend_enable: bool,
    fog: bool,
    fog_color: Color,
    color_mask: [bool; 4],
    alpha_test: bool,
    alpha_ref: f32,
}

pub struct Texture {
    pub tex: WebGlTexture,
    pub w: u32,
    pub h: u32,
    /// framebuffer textures are stored upside down
    pub flipped: bool,
}

pub struct Surface {
    pub tex_index: usize,
    pub fb: WebGlFramebuffer,
    pub w: u32,
    pub h: u32,
    pub alive: bool,
}

pub type SurfaceId = i32;

pub struct Gfx {
    pub gl: GL,
    #[allow(dead_code)]
    prog: WebGlProgram,
    u_proj: WebGlUniformLocation,
    u_fog: WebGlUniformLocation,
    u_fogcol: WebGlUniformLocation,
    u_atest: WebGlUniformLocation,
    vbo: WebGlBuffer,
    verts: Vec<f32>,
    pub textures: Vec<Texture>,
    pub white_tex: usize,
    state: State,
    pending: State,
    pub surfaces: Vec<Surface>,
    target_stack: Vec<SurfaceId>,
    /// current render target size
    pub tw: f32,
    pub th: f32,
    pub draw_color: Color,
    pub draw_alpha: f64,
    pub app_surface: SurfaceId,
    pub canvas_w: u32,
    pub canvas_h: u32,
    /// camera offset (view x/y) applied when drawing to the application surface
    pub view_x: f64,
    pub view_y: f64,
    /// last mode passed to gpu_set_blendmode (for gpu_get_blendmode)
    pub blendmode: i32,
}

const VS: &str = r#"#version 300 es
layout(location=0) in vec2 a_pos;
layout(location=1) in vec2 a_uv;
layout(location=2) in vec4 a_col;
uniform vec4 u_proj; // sx, sy, tx, ty
out vec2 v_uv;
out vec4 v_col;
void main(){
  v_uv = a_uv; v_col = a_col;
  gl_Position = vec4(a_pos.x * u_proj.x + u_proj.z, a_pos.y * u_proj.y + u_proj.w, 0.0, 1.0);
}"#;

const FS: &str = r#"#version 300 es
precision mediump float;
in vec2 v_uv;
in vec4 v_col;
uniform sampler2D u_tex;
uniform int u_fog;
uniform vec3 u_fogcol;
uniform float u_atest;
out vec4 o;
void main(){
  vec4 c = texture(u_tex, v_uv) * v_col;
  if (u_fog == 1) c.rgb = u_fogcol;
  if (u_atest >= 0.0 && c.a <= u_atest) discard;
  o = c;
}"#;

fn compile(gl: &GL, ty: u32, src: &str) -> web_sys::WebGlShader {
    let s = gl.create_shader(ty).unwrap();
    gl.shader_source(&s, src);
    gl.compile_shader(&s);
    if !gl.get_shader_parameter(&s, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        panic!("shader: {:?}", gl.get_shader_info_log(&s));
    }
    s
}

impl Gfx {
    pub fn new(canvas: &web_sys::HtmlCanvasElement) -> Gfx {
        let attrs = web_sys::WebGlContextAttributes::new();
        attrs.set_alpha(false);
        attrs.set_antialias(false);
        attrs.set_premultiplied_alpha(false);
        let gl: GL = canvas
            .get_context_with_context_options("webgl2", &attrs)
            .unwrap()
            .expect("WebGL2 unsupported")
            .dyn_into()
            .unwrap();
        let prog = gl.create_program().unwrap();
        gl.attach_shader(&prog, &compile(&gl, GL::VERTEX_SHADER, VS));
        gl.attach_shader(&prog, &compile(&gl, GL::FRAGMENT_SHADER, FS));
        gl.link_program(&prog);
        gl.use_program(Some(&prog));
        let u_proj = gl.get_uniform_location(&prog, "u_proj").unwrap();
        let u_fog = gl.get_uniform_location(&prog, "u_fog").unwrap();
        let u_fogcol = gl.get_uniform_location(&prog, "u_fogcol").unwrap();
        let u_atest = gl.get_uniform_location(&prog, "u_atest").unwrap();
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(&vao));
        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        let stride = 8 * 4;
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_with_i32(1, 2, GL::FLOAT, false, stride, 8);
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_with_i32(2, 4, GL::FLOAT, false, stride, 16);
        gl.enable(GL::BLEND);
        gl.uniform1f(Some(&u_atest), -1.0);
        let st = State {
            tex: 0,
            blend: [bm::SRC_ALPHA, bm::INV_SRC_ALPHA, bm::SRC_ALPHA, bm::INV_SRC_ALPHA],
            blend_enable: true,
            fog: false,
            fog_color: 0,
            color_mask: [true; 4],
            alpha_test: false,
            alpha_ref: 0.0,
        };
        let mut g = Gfx {
            gl,
            prog,
            u_proj,
            u_fog,
            u_fogcol,
            u_atest,
            vbo,
            verts: Vec::with_capacity(1 << 16),
            textures: vec![],
            white_tex: 0,
            state: st,
            pending: st,
            surfaces: vec![],
            target_stack: vec![],
            tw: 640.0,
            th: 480.0,
            draw_color: C_WHITE,
            draw_alpha: 1.0,
            app_surface: -1,
            canvas_w: canvas.width(),
            canvas_h: canvas.height(),
            view_x: 0.0,
            view_y: 0.0,
            blendmode: 0,
        };
        g.white_tex = g.add_texture_rgba(1, 1, &[255, 255, 255, 255]);
        g.apply_state(true);
        g.app_surface = g.surface_create(640, 480);
        g
    }

    pub fn add_texture_rgba(&mut self, w: u32, h: u32, data: &[u8]) -> usize {
        let gl = &self.gl;
        let t = gl.create_texture().unwrap();
        gl.bind_texture(GL::TEXTURE_2D, Some(&t));
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            GL::TEXTURE_2D, 0, GL::RGBA as i32, w as i32, h as i32, 0, GL::RGBA, GL::UNSIGNED_BYTE, Some(data),
        )
        .unwrap();
        Self::tex_params(gl);
        self.textures.push(Texture { tex: t, w, h, flipped: false });
        self.state.tex = usize::MAX;
        self.textures.len() - 1
    }

    pub fn add_texture_image(&mut self, img: &web_sys::HtmlImageElement) -> usize {
        let gl = &self.gl;
        let t = gl.create_texture().unwrap();
        gl.bind_texture(GL::TEXTURE_2D, Some(&t));
        gl.tex_image_2d_with_u32_and_u32_and_html_image_element(GL::TEXTURE_2D, 0, GL::RGBA as i32, GL::RGBA, GL::UNSIGNED_BYTE, img)
            .unwrap();
        Self::tex_params(gl);
        self.textures.push(Texture { tex: t, w: img.natural_width(), h: img.natural_height(), flipped: false });
        self.state.tex = usize::MAX;
        self.textures.len() - 1
    }

    fn tex_params(gl: &GL) {
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::NEAREST as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::NEAREST as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
    }

    // ---------------------------------------------------------------- state
    fn apply_state(&mut self, force: bool) {
        let p = self.pending;
        let s = self.state;
        let gl = &self.gl;
        if force || p.tex != s.tex {
            if let Some(t) = self.textures.get(p.tex) {
                gl.bind_texture(GL::TEXTURE_2D, Some(&t.tex));
            }
        }
        if force || p.blend != s.blend {
            gl.blend_func_separate(gl_factor(p.blend[0]), gl_factor(p.blend[1]), gl_factor(p.blend[2]), gl_factor(p.blend[3]));
        }
        if force || p.blend_enable != s.blend_enable {
            if p.blend_enable { gl.enable(GL::BLEND) } else { gl.disable(GL::BLEND) }
        }
        if force || p.fog != s.fog || p.fog_color != s.fog_color {
            gl.uniform1i(Some(&self.u_fog), p.fog as i32);
            gl.uniform3f(
                Some(&self.u_fogcol),
                color_get_red(p.fog_color) as f32 / 255.0,
                color_get_green(p.fog_color) as f32 / 255.0,
                color_get_blue(p.fog_color) as f32 / 255.0,
            );
        }
        if force || p.color_mask != s.color_mask {
            gl.color_mask(p.color_mask[0], p.color_mask[1], p.color_mask[2], p.color_mask[3]);
        }
        if force || p.alpha_test != s.alpha_test || p.alpha_ref != s.alpha_ref {
            gl.uniform1f(Some(&self.u_atest), if p.alpha_test { p.alpha_ref } else { -1.0 });
        }
        self.state = p;
    }

    fn set_pending<F: FnOnce(&mut State)>(&mut self, f: F) {
        let mut p = self.pending;
        f(&mut p);
        if p != self.pending {
            self.flush();
            self.pending = p;
        }
    }

    pub fn flush(&mut self) {
        if self.verts.is_empty() {
            return;
        }
        if self.pending != self.state {
            self.apply_state(false);
        }
        let gl = &self.gl;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
        unsafe {
            let view = js_sys::Float32Array::view(&self.verts);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STREAM_DRAW);
        }
        gl.draw_arrays(GL::TRIANGLES, 0, (self.verts.len() / 8) as i32);
        self.verts.clear();
    }

    pub fn gpu_get_blendmode(&self) -> i32 { self.blendmode }
    pub fn gpu_set_blendmode(&mut self, mode: i32) {
        self.blendmode = mode;
        let b = match mode {
            bm::ADD => [bm::SRC_ALPHA, bm::ONE, bm::SRC_ALPHA, bm::ONE],
            bm::MAX => [bm::SRC_ALPHA, bm::INV_SRC_COLOUR, bm::SRC_ALPHA, bm::INV_SRC_COLOUR],
            bm::SUBTRACT => [bm::ZERO, bm::INV_SRC_COLOUR, bm::ZERO, bm::INV_SRC_COLOUR],
            _ => [bm::SRC_ALPHA, bm::INV_SRC_ALPHA, bm::SRC_ALPHA, bm::INV_SRC_ALPHA],
        };
        self.set_pending(|p| p.blend = b);
    }
    pub fn draw_set_blend_mode(&mut self, mode: i32) { self.gpu_set_blendmode(mode) }
    pub fn gpu_set_blendmode_ext(&mut self, src: i32, dst: i32) {
        self.set_pending(|p| p.blend = [src, dst, src, dst]);
    }
    pub fn gpu_set_blendmode_ext_sepalpha(&mut self, src: i32, dst: i32, srca: i32, dsta: i32) {
        self.set_pending(|p| p.blend = [src, dst, srca, dsta]);
    }
    pub fn gpu_set_blendenable(&mut self, e: bool) { self.set_pending(|p| p.blend_enable = e); }
    pub fn gpu_set_fog(&mut self, enable: bool, color: Color) {
        self.set_pending(|p| {
            p.fog = enable;
            p.fog_color = color;
        });
    }
    pub fn d3d_set_fog(&mut self, enable: bool, color: Color) { self.gpu_set_fog(enable, color) }
    pub fn gpu_set_colorwriteenable(&mut self, r: bool, g: bool, b: bool, a: bool) {
        self.set_pending(|p| p.color_mask = [r, g, b, a]);
    }
    pub fn gpu_set_alphatestenable(&mut self, e: bool) { self.set_pending(|p| p.alpha_test = e); }
    pub fn gpu_set_alphatestref(&mut self, r: f64) { self.set_pending(|p| p.alpha_ref = (r / 255.0) as f32); }
    pub fn reset_gpu_state(&mut self) {
        self.gpu_set_blendmode(bm::NORMAL);
        self.gpu_set_blendenable(true);
        self.gpu_set_fog(false, 0);
        self.gpu_set_colorwriteenable(true, true, true, true);
        self.gpu_set_alphatestenable(false);
    }

    // ---------------------------------------------------------------- surfaces
    pub fn surface_create(&mut self, w: i32, h: i32) -> SurfaceId {
        let w = w.max(1) as u32;
        let h = h.max(1) as u32;
        self.flush();
        let gl = &self.gl;
        let t = gl.create_texture().unwrap();
        gl.bind_texture(GL::TEXTURE_2D, Some(&t));
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            GL::TEXTURE_2D, 0, GL::RGBA as i32, w as i32, h as i32, 0, GL::RGBA, GL::UNSIGNED_BYTE, None,
        )
        .unwrap();
        Self::tex_params(gl);
        let fb = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&fb));
        gl.framebuffer_texture_2d(GL::FRAMEBUFFER, GL::COLOR_ATTACHMENT0, GL::TEXTURE_2D, Some(&t), 0);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(GL::COLOR_BUFFER_BIT);
        self.textures.push(Texture { tex: t, w, h, flipped: true });
        let ti = self.textures.len() - 1;
        self.state.tex = usize::MAX;
        // reuse a dead slot
        let surf = Surface { tex_index: ti, fb, w, h, alive: true };
        let id = if let Some(i) = self.surfaces.iter().position(|s| !s.alive) {
            self.surfaces[i] = surf;
            i as SurfaceId
        } else {
            self.surfaces.push(surf);
            (self.surfaces.len() - 1) as SurfaceId
        };
        self.bind_current_target();
        id
    }
    pub fn surface_exists(&self, s: SurfaceId) -> bool {
        s >= 0 && (s as usize) < self.surfaces.len() && self.surfaces[s as usize].alive
    }
    pub fn surface_free(&mut self, s: SurfaceId) {
        if self.surface_exists(s) && s != self.app_surface {
            self.flush();
            let surf = &mut self.surfaces[s as usize];
            surf.alive = false;
            self.gl.delete_framebuffer(Some(&surf.fb));
            let ti = surf.tex_index;
            self.gl.delete_texture(Some(&self.textures[ti].tex));
        }
    }
    pub fn surface_get_width(&self, s: SurfaceId) -> f64 {
        if self.surface_exists(s) { self.surfaces[s as usize].w as f64 } else { -1.0 }
    }
    pub fn surface_get_height(&self, s: SurfaceId) -> f64 {
        if self.surface_exists(s) { self.surfaces[s as usize].h as f64 } else { -1.0 }
    }
    pub fn surface_resize(&mut self, s: SurfaceId, w: i32, h: i32) {
        if !self.surface_exists(s) {
            return;
        }
        self.flush();
        let ti = self.surfaces[s as usize].tex_index;
        let gl = &self.gl;
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.textures[ti].tex));
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            GL::TEXTURE_2D, 0, GL::RGBA as i32, w, h, 0, GL::RGBA, GL::UNSIGNED_BYTE, None,
        )
        .unwrap();
        self.state.tex = usize::MAX;
        self.textures[ti].w = w as u32;
        self.textures[ti].h = h as u32;
        self.surfaces[s as usize].w = w as u32;
        self.surfaces[s as usize].h = h as u32;
    }
    pub fn surface_set_target(&mut self, s: SurfaceId) -> bool {
        if !self.surface_exists(s) {
            return false;
        }
        self.flush();
        self.target_stack.push(s);
        self.bind_current_target();
        true
    }
    pub fn surface_reset_target(&mut self) {
        self.flush();
        self.target_stack.pop();
        self.bind_current_target();
    }
    /// Begin a frame: target the application surface.
    pub fn begin_frame(&mut self) {
        self.target_stack.clear();
        let a = self.app_surface;
        self.target_stack.push(a);
        self.bind_current_target();
        self.reset_gpu_state();
        self.draw_color = C_WHITE;
        self.draw_alpha = 1.0;
    }
    pub fn current_target(&self) -> SurfaceId { *self.target_stack.last().unwrap_or(&-1) }
    fn bind_current_target(&mut self) {
        let gl = &self.gl;
        let top = self.target_stack.last().copied();
        let (w, h, offset) = match top {
            Some(s) if self.surface_exists(s) => {
                let surf = &self.surfaces[s as usize];
                gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&surf.fb));
                // the application surface is in room space, offset by the view
                (surf.w, surf.h, s == self.app_surface)
            }
            _ => {
                gl.bind_framebuffer(GL::FRAMEBUFFER, None);
                (self.canvas_w, self.canvas_h, false)
            }
        };
        gl.viewport(0, 0, w as i32, h as i32);
        self.tw = w as f32;
        self.th = h as f32;
        let (ox, oy) = if offset { (self.view_x as f32, self.view_y as f32) } else { (0.0, 0.0) };
        // framebuffer targets render with y up in texture space; we flag the texture as flipped instead
        let is_fb = top.map(|s| self.surface_exists(s)).unwrap_or(false);
        let sx = 2.0 / w as f32;
        let sy = if is_fb { 2.0 / h as f32 } else { -2.0 / h as f32 };
        let tx = -1.0 - ox * sx;
        let ty = if is_fb { -1.0 - oy * sy } else { 1.0 - oy * sy };
        gl.uniform4f(Some(&self.u_proj), sx, sy, tx, ty);
    }
    pub fn draw_clear_alpha(&mut self, color: Color, alpha: f64) {
        self.flush();
        let gl = &self.gl;
        gl.color_mask(true, true, true, true);
        gl.clear_color(
            color_get_red(color) as f32 / 255.0,
            color_get_green(color) as f32 / 255.0,
            color_get_blue(color) as f32 / 255.0,
            alpha as f32,
        );
        gl.clear(GL::COLOR_BUFFER_BIT);
        let m = self.state.color_mask;
        gl.color_mask(m[0], m[1], m[2], m[3]);
    }
    pub fn draw_clear(&mut self, color: Color) { self.draw_clear_alpha(color, 1.0) }
    pub fn surface_tex(&self, s: SurfaceId) -> Option<usize> {
        if self.surface_exists(s) { Some(self.surfaces[s as usize].tex_index) } else { None }
    }
    /// Copy src surface into dst at (x,y).
    pub fn surface_copy(&mut self, dst: SurfaceId, x: f64, y: f64, src: SurfaceId) {
        if !self.surface_exists(dst) || !self.surface_exists(src) {
            return;
        }
        self.surface_set_target(dst);
        self.gpu_set_blendenable(false);
        self.draw_surface_ext(src, x, y, 1.0, 1.0, 0.0, C_WHITE, 1.0);
        self.gpu_set_blendenable(true);
        self.surface_reset_target();
    }

    // ---------------------------------------------------------------- primitives
    #[inline]
    fn rgba(color: Color, alpha: f64) -> [f32; 4] {
        [
            color_get_red(color) as f32 / 255.0,
            color_get_green(color) as f32 / 255.0,
            color_get_blue(color) as f32 / 255.0,
            alpha as f32,
        ]
    }
    #[inline]
    fn push_v(&mut self, x: f32, y: f32, u: f32, v: f32, c: [f32; 4]) {
        self.verts.extend_from_slice(&[x, y, u, v, c[0], c[1], c[2], c[3]]);
    }
    fn use_tex(&mut self, t: usize) {
        if self.pending.tex != t {
            self.set_pending(|p| p.tex = t);
        }
        if self.verts.len() > 60000 {
            self.flush();
        }
    }
    /// Draw a textured quad with explicit corners (clockwise from top-left) and uvs.
    pub fn quad(&mut self, tex: usize, p: [[f32; 2]; 4], uv: [[f32; 2]; 4], c: [[f32; 4]; 4]) {
        self.use_tex(tex);
        for &i in &[0usize, 1, 2, 0, 2, 3] {
            self.push_v(p[i][0], p[i][1], uv[i][0], uv[i][1], c[i]);
        }
    }
    pub fn tri(&mut self, p: [[f32; 2]; 3], c: [[f32; 4]; 3]) {
        let t = self.white_tex;
        self.use_tex(t);
        for i in 0..3 {
            self.push_v(p[i][0], p[i][1], 0.5, 0.5, c[i]);
        }
    }

    pub fn draw_set_color(&mut self, c: Color) { self.draw_color = c; }
    pub fn draw_set_alpha(&mut self, a: f64) { self.draw_alpha = a; }
    pub fn draw_get_color(&self) -> Color { self.draw_color }
    pub fn draw_get_alpha(&self) -> f64 { self.draw_alpha }

    pub fn draw_rectangle_color(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, c1: Color, c2: Color, c3: Color, c4: Color, outline: bool) {
        let a = self.draw_alpha;
        let (x1, x2) = (x1.min(x2) as f32, x1.max(x2) as f32);
        let (y1, y2) = (y1.min(y2) as f32, y1.max(y2) as f32);
        if outline {
            self.draw_line_width_color(x1 as f64, y1 as f64 + 0.5, x2 as f64 + 1.0, y1 as f64 + 0.5, 1.0, c1, c2);
            self.draw_line_width_color(x2 as f64 + 0.5, y1 as f64, x2 as f64 + 0.5, y2 as f64 + 1.0, 1.0, c2, c3);
            self.draw_line_width_color(x2 as f64 + 1.0, y2 as f64 + 0.5, x1 as f64, y2 as f64 + 0.5, 1.0, c3, c4);
            self.draw_line_width_color(x1 as f64 + 0.5, y2 as f64 + 1.0, x1 as f64 + 0.5, y1 as f64, 1.0, c4, c1);
            return;
        }
        let t = self.white_tex;
        self.quad(
            t,
            [[x1, y1], [x2 + 1.0, y1], [x2 + 1.0, y2 + 1.0], [x1, y2 + 1.0]],
            [[0.5; 2]; 4],
            [Self::rgba(c1, a), Self::rgba(c2, a), Self::rgba(c3, a), Self::rgba(c4, a)],
        );
    }
    pub fn draw_rectangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, outline: bool) {
        let c = self.draw_color;
        self.draw_rectangle_color(x1, y1, x2, y2, c, c, c, c, outline);
    }
    pub fn draw_line_width_color(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, w: f64, c1: Color, c2: Color) {
        let a = self.draw_alpha;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            return;
        }
        let nx = (-dy / len * w / 2.0) as f32;
        let ny = (dx / len * w / 2.0) as f32;
        let (x1, y1, x2, y2) = (x1 as f32, y1 as f32, x2 as f32, y2 as f32);
        let t = self.white_tex;
        let ca = Self::rgba(c1, a);
        let cb = Self::rgba(c2, a);
        self.quad(t, [[x1 + nx, y1 + ny], [x2 + nx, y2 + ny], [x2 - nx, y2 - ny], [x1 - nx, y1 - ny]], [[0.5; 2]; 4], [ca, cb, cb, ca]);
    }
    pub fn draw_line_width(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, w: f64) {
        let c = self.draw_color;
        self.draw_line_width_color(x1, y1, x2, y2, w, c, c);
    }
    pub fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) { self.draw_line_width(x1, y1, x2, y2, 1.0) }
    pub fn draw_line_color(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, c1: Color, c2: Color) {
        self.draw_line_width_color(x1, y1, x2, y2, 1.0, c1, c2)
    }
    pub fn draw_triangle_color(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, c1: Color, c2: Color, c3: Color, outline: bool) {
        if outline {
            self.draw_line_color(x1, y1, x2, y2, c1, c2);
            self.draw_line_color(x2, y2, x3, y3, c2, c3);
            self.draw_line_color(x3, y3, x1, y1, c3, c1);
            return;
        }
        let a = self.draw_alpha;
        self.tri(
            [[x1 as f32, y1 as f32], [x2 as f32, y2 as f32], [x3 as f32, y3 as f32]],
            [Self::rgba(c1, a), Self::rgba(c2, a), Self::rgba(c3, a)],
        );
    }
    pub fn draw_triangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, outline: bool) {
        let c = self.draw_color;
        self.draw_triangle_color(x1, y1, x2, y2, x3, y3, c, c, c, outline);
    }
    pub fn draw_circle_color(&mut self, x: f64, y: f64, r: f64, c1: Color, c2: Color, outline: bool) {
        let n = 24;
        for i in 0..n {
            let a1 = i as f64 / n as f64 * std::f64::consts::TAU;
            let a2 = (i + 1) as f64 / n as f64 * std::f64::consts::TAU;
            let (px1, py1) = (x + a1.cos() * r, y + a1.sin() * r);
            let (px2, py2) = (x + a2.cos() * r, y + a2.sin() * r);
            if outline {
                self.draw_line_color(px1, py1, px2, py2, c2, c2);
            } else {
                self.draw_triangle_color(x, y, px1, py1, px2, py2, c1, c2, c2, false);
            }
        }
    }
    pub fn draw_circle(&mut self, x: f64, y: f64, r: f64, outline: bool) {
        let c = self.draw_color;
        self.draw_circle_color(x, y, r, c, c, outline);
    }
    pub fn draw_ellipse(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, outline: bool) {
        let c = self.draw_color;
        let (cx, cy, rx, ry) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0, (x2 - x1).abs() / 2.0, (y2 - y1).abs() / 2.0);
        let n = 24;
        for i in 0..n {
            let a1 = i as f64 / n as f64 * std::f64::consts::TAU;
            let a2 = (i + 1) as f64 / n as f64 * std::f64::consts::TAU;
            let (px1, py1) = (cx + a1.cos() * rx, cy + a1.sin() * ry);
            let (px2, py2) = (cx + a2.cos() * rx, cy + a2.sin() * ry);
            if outline {
                self.draw_line_color(px1, py1, px2, py2, c, c);
            } else {
                self.draw_triangle_color(cx, cy, px1, py1, px2, py2, c, c, c, false);
            }
        }
    }

    // ---------------------------------------------------------------- surfaces drawing
    pub fn draw_surface_general(
        &mut self, s: SurfaceId, left: f64, top: f64, w: f64, h: f64, x: f64, y: f64, xs: f64, ys: f64, rot: f64, cols: [Color; 4], alpha: f64,
    ) {
        let Some(ti) = self.surface_tex(s) else { return };
        if self.current_target() == s {
            return;
        }
        let tw = self.textures[ti].w as f64;
        let th = self.textures[ti].h as f64;
        let (u0, u1) = ((left / tw) as f32, ((left + w) / tw) as f32);
        // FBO targets are rendered with a y-up projection, so row 0 of the texture is the surface's top
        let (v0, v1) = ((top / th) as f32, ((top + h) / th) as f32);
        let (sn, cs) = (rot.to_radians().sin(), rot.to_radians().cos());
        let tf = |lx: f64, ly: f64| -> [f32; 2] {
            let lx = lx * xs;
            let ly = ly * ys;
            [(x + lx * cs + ly * sn) as f32, (y - lx * sn + ly * cs) as f32]
        };
        let p = [tf(0.0, 0.0), tf(w, 0.0), tf(w, h), tf(0.0, h)];
        let c = [Self::rgba(cols[0], alpha), Self::rgba(cols[1], alpha), Self::rgba(cols[2], alpha), Self::rgba(cols[3], alpha)];
        self.quad(ti, p, [[u0, v0], [u1, v0], [u1, v1], [u0, v1]], c);
    }
    pub fn draw_surface(&mut self, s: SurfaceId, x: f64, y: f64) {
        let a = self.draw_alpha;
        self.draw_surface_ext(s, x, y, 1.0, 1.0, 0.0, C_WHITE, a);
    }
    pub fn draw_surface_ext(&mut self, s: SurfaceId, x: f64, y: f64, xs: f64, ys: f64, rot: f64, col: Color, alpha: f64) {
        let (w, h) = (self.surface_get_width(s), self.surface_get_height(s));
        self.draw_surface_general(s, 0.0, 0.0, w, h, x, y, xs, ys, rot, [col; 4], alpha);
    }
    pub fn draw_surface_part_ext(&mut self, s: SurfaceId, l: f64, t: f64, w: f64, h: f64, x: f64, y: f64, xs: f64, ys: f64, col: Color, alpha: f64) {
        self.draw_surface_general(s, l, t, w, h, x, y, xs, ys, 0.0, [col; 4], alpha);
    }
    pub fn draw_surface_part(&mut self, s: SurfaceId, l: f64, t: f64, w: f64, h: f64, x: f64, y: f64) {
        let a = self.draw_alpha;
        self.draw_surface_general(s, l, t, w, h, x, y, 1.0, 1.0, 0.0, [C_WHITE; 4], a);
    }
    pub fn draw_surface_stretched(&mut self, s: SurfaceId, x: f64, y: f64, w: f64, h: f64) {
        let (sw, sh) = (self.surface_get_width(s), self.surface_get_height(s));
        let a = self.draw_alpha;
        self.draw_surface_general(s, 0.0, 0.0, sw, sh, x, y, w / sw, h / sh, 0.0, [C_WHITE; 4], a);
    }

    /// Blit the application surface to the canvas, scaled to fit with integer-ish letterboxing.
    pub fn present(&mut self) {
        self.flush();
        self.target_stack.clear();
        self.bind_current_target();
        self.reset_gpu_state();
        self.flush();
        let gl = &self.gl;
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(GL::COLOR_BUFFER_BIT);
        let (cw, ch) = (self.canvas_w as f64, self.canvas_h as f64);
        let scale = (cw / 640.0).min(ch / 480.0);
        let (w, h) = (640.0 * scale, 480.0 * scale);
        let x = ((cw - w) / 2.0).floor();
        let y = ((ch - h) / 2.0).floor();
        self.gpu_set_blendenable(false);
        let a = self.app_surface;
        self.draw_surface_general(a, 0.0, 0.0, 640.0, 480.0, x, y, scale, scale, 0.0, [C_WHITE; 4], 1.0);
        self.flush();
        self.gpu_set_blendenable(true);
    }

    pub fn resize_canvas(&mut self, w: u32, h: u32) {
        self.canvas_w = w;
        self.canvas_h = h;
    }
}
