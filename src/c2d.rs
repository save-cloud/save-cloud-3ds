use std::{
    error::Error,
    ffi::{c_char, c_float, c_uint, c_void},
    marker::PhantomData,
    ops::Deref,
    ptr::{null, null_mut},
};

use ctru::services::gfx::Gfx;

use crate::{platform::setup_romfs, utils::str_to_c_null_term_bytes};

extern "C" {
    fn c2d_raw_init();
    fn c2d_raw_fini();
    fn c2d_raw_load_sprite_sheet(path: *const c_char) -> *mut c_void;
    fn c2d_raw_free_sprite_sheet(sheet: *mut c_void);
    fn c2d_raw_image_from_sheet(sheet: *mut c_void, index: usize) -> *mut c_void;
    fn c2d_raw_free_image_from_sheet(sprite: *mut c_void);
    fn c2d_raw_start_drawing();
    fn c2d_raw_end_drawing();
    fn c2d_raw_start_scene(target: *mut c_void);
    fn c2d_raw_clear_scene(target: *mut c_void, color: c_uint);
    fn c2d_raw_clear_text_buf();
    fn c2d_raw_create_text(text: *const c_char) -> *mut c_void;
    fn c2d_raw_free_text(text: *mut c_void);
    fn c2d_raw_draw_text(
        text: *mut c_void,
        x: c_float,
        y: c_float,
        z: c_float,
        scaleX: c_float,
        scaleY: c_float,
        color: c_uint,
        max_width: *const c_float,
    );
    fn c2d_raw_free_image(image: *mut c_void);
    fn c2d_raw_load_icon_from_buffer(buffer: *const c_void) -> *mut c_void;
    fn c2d_raw_load_qrcode_from_buffer(buffer: *const c_void) -> *mut c_void;

    // raw
    fn C2D_CreateScreenTarget(screen: c_uint, side: c_uint) -> *mut c_void;
    fn C3D_RenderTargetDelete(target: *mut c_void);
    fn c2d_drawrectsolid(x: f32, y: f32, z: f32, w: f32, h: f32, color: u32);
    fn C2D_DrawLine(
        x0: f32,
        y0: f32,
        color0: u32,
        x1: f32,
        y1: f32,
        color1: u32,
        width: f32,
        z: f32,
    );
    fn c2d_raw_draw_image_at(
        image: *const c_void,
        x: c_float,
        y: c_float,
        z: c_float,
        scaleX: c_float,
        scaleY: c_float,
    );
    fn C2D_TextGetDimensions(
        text: *const c_void,
        scaleX: c_float,
        scaleY: c_float,
        outWidth: *mut c_float,
        outHeight: *mut c_float,
    );
    fn C2D_SpriteSheetCount(sheet: *mut c_void) -> usize;
}

#[repr(C)]
pub struct C2dPos {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[repr(C)]
pub struct C2dPoint {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
pub struct C2dDrawParams {
    pub pos: C2dPos,
    pub center: C2dPoint,
    pub depth: f32,
    pub angle: f32,
}

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

pub fn c2d_draw_rect(x: f32, y: f32, z: f32, w: f32, h: f32, color: u32) {
    unsafe {
        c2d_drawrectsolid(x, y, z, w, h, color);
    }
}

pub fn c2d_draw_line(
    x0: f32,
    y0: f32,
    color0: u32,
    x1: f32,
    y1: f32,
    color1: u32,
    width: f32,
    z: f32,
) {
    unsafe {
        C2D_DrawLine(x0, y0, color0, x1, y1, color1, width, z);
    }
}

pub fn c2d_draw_image(image: &C2dImage, x: f32, y: f32, z: f32, scale_x: f32, scale_y: f32) {
    unsafe {
        c2d_raw_draw_image_at(image.ptr, x, y, z, scale_x, scale_y);
    }
}

pub fn c2d_draw_text(text: &C2dText, x: f32, y: f32, z: f32, scale: f32, color: u32) {
    unsafe {
        c2d_raw_draw_text(text.ptr, x, y, z, scale, scale, color, null());
    }
}

pub fn c2d_draw_text_wrap(
    text: &C2dText,
    x: f32,
    y: f32,
    z: f32,
    scale: f32,
    color: u32,
    max_width: f32,
) {
    unsafe {
        let ptr = &max_width as *const f32;
        c2d_raw_draw_text(text.ptr, x, y, z, scale, scale, color, ptr);
    }
}

pub struct C2dText {
    pub ptr: *mut c_void,
}

impl C2dText {
    pub fn new(text: &str) -> Self {
        unsafe {
            let text = str_to_c_null_term_bytes(text);
            let raw = c2d_raw_create_text(text.as_ptr());
            Self { ptr: raw }
        }
    }

    pub fn dimension(&self, scale_x: f32, scale_y: f32) -> (f32, f32) {
        unsafe {
            let mut width = 0.0;
            let mut height = 0.0;
            C2D_TextGetDimensions(self.ptr, scale_x, scale_y, &mut width, &mut height);
            (width, height)
        }
    }
}

impl Drop for C2dText {
    fn drop(&mut self) {
        unsafe {
            c2d_raw_free_text(self.ptr);
        }
    }
}

pub trait C2dImageTrait {
    fn get_image(&self) -> &C2dImage;
}

// 确保 C2dImage 的生命周期不超过 C2dSpriteSheet
pub struct C2dImage {
    from_sheet: bool,
    pub ptr: *mut c_void,
}

impl C2dImageTrait for C2dImage {
    fn get_image(&self) -> &C2dImage {
        self
    }
}

impl Drop for C2dImage {
    fn drop(&mut self) {
        unsafe {
            if !self.from_sheet {
                c2d_raw_free_image(self.ptr);
            } else {
                c2d_raw_free_image_from_sheet(self.ptr);
            }
        }
    }
}

pub struct C2dImageFromSheet<'a> {
    pub image: C2dImage,
    _phantom: PhantomData<&'a ()>,
}

impl Deref for C2dImageFromSheet<'_> {
    type Target = C2dImage;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl C2dImageTrait for C2dImageFromSheet<'_> {
    fn get_image(&self) -> &C2dImage {
        &self.image
    }
}

struct C2dSpriteSheet {
    ptr: *mut c_void,
}

impl C2dSpriteSheet {
    fn new() -> Self {
        unsafe {
            if let Ok(_) = setup_romfs() {
                let path = str_to_c_null_term_bytes("romfs:/assets.t3x");
                let raw = c2d_raw_load_sprite_sheet(path.as_ptr());
                Self { ptr: raw }
            } else {
                Self { ptr: null_mut() }
            }
        }
    }

    pub fn count(&self) -> usize {
        if self.ptr.is_null() {
            return 0;
        }
        unsafe { C2D_SpriteSheetCount(self.ptr) }
    }

    fn get_image(&self, idx: usize) -> C2dImageFromSheet {
        unsafe {
            C2dImageFromSheet {
                image: C2dImage {
                    from_sheet: true,
                    ptr: c2d_raw_image_from_sheet(self.ptr, idx),
                },
                _phantom: PhantomData,
            }
        }
    }
}

impl Drop for C2dSpriteSheet {
    fn drop(&mut self) {
        unsafe {
            c2d_raw_free_sprite_sheet(self.ptr);
        }
    }
}

pub fn c2d_load_icon_from_buffer(buffer: &[u16]) -> C2dImage {
    unsafe {
        C2dImage {
            from_sheet: false,
            ptr: c2d_raw_load_icon_from_buffer(buffer.as_ptr() as *const c_void),
        }
    }
}

pub fn c2d_load_qrcode_from_buffer(buffer: &[u8]) -> C2dImage {
    unsafe {
        C2dImage {
            from_sheet: false,
            ptr: c2d_raw_load_qrcode_from_buffer(buffer.as_ptr() as *const c_void),
        }
    }
}

pub struct C2D {
    pub gfx: Gfx,
    top_render_target_left: *mut c_void,
    top_render_target_right: *mut c_void,
    bottom_render_target: *mut c_void,
    sheet: C2dSpriteSheet,
}

impl C2D {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let gfx = Gfx::new()?;
        unsafe {
            c2d_raw_init();
        }
        let top_render_target_left =
            Self::create_render_target(ctru_sys::GFX_TOP, ctru_sys::GFX_LEFT);
        let top_render_target_right =
            Self::create_render_target(ctru_sys::GFX_TOP, ctru_sys::GFX_RIGHT);
        let bottom_render_target =
            Self::create_render_target(ctru_sys::GFX_BOTTOM, ctru_sys::GFX_LEFT);
        Ok(Self {
            gfx,
            top_render_target_left,
            top_render_target_right,
            bottom_render_target,
            sheet: C2dSpriteSheet::new(),
        })
    }

    pub fn enable_3d(&self) {
        unsafe {
            ctru_sys::gfxSet3D(true);
        }
    }

    pub fn disable_3d(&self) {
        unsafe {
            ctru_sys::gfxSet3D(false);
        }
    }

    pub fn sheet_image_count(&self) -> usize {
        self.sheet.count()
    }

    pub fn get_image_from_sheet(&self, idx: usize) -> Option<C2dImageFromSheet> {
        if idx >= self.sheet_image_count() {
            return None;
        }
        Some(self.sheet.get_image(idx))
    }

    fn create_render_target(
        screen: ctru_sys::gfxScreen_t,
        side: ctru_sys::gfx3dSide_t,
    ) -> *mut c_void {
        unsafe { C2D_CreateScreenTarget(screen, side) }
    }

    pub fn start_drawing(&self) {
        unsafe {
            c2d_raw_clear_text_buf();
            c2d_raw_start_drawing();
        }
    }

    pub fn end_drawing(&self) {
        unsafe {
            c2d_raw_end_drawing();
        }
    }

    pub fn start_top_scene(&self) {
        self.start_top_scene_left();
    }

    pub fn start_top_scene_left(&self) {
        unsafe {
            c2d_raw_start_scene(self.top_render_target_left);
        }
    }

    pub fn start_top_scene_right(&self) {
        unsafe {
            c2d_raw_start_scene(self.top_render_target_right);
        }
    }

    pub fn start_bottom_scene(&self) {
        unsafe {
            c2d_raw_start_scene(self.bottom_render_target);
        }
    }

    pub fn clear_top_scene(&self, color: u32) {
        self.clear_top_scene_left(color);
    }

    pub fn clear_top_scene_left(&self, color: u32) {
        unsafe {
            c2d_raw_clear_scene(self.top_render_target_left, color);
        }
    }

    pub fn clear_top_scene_right(&self, color: u32) {
        unsafe {
            c2d_raw_clear_scene(self.top_render_target_right, color);
        }
    }

    pub fn clear_bottom_scene(&self, color: u32) {
        unsafe {
            c2d_raw_clear_scene(self.bottom_render_target, color);
        }
    }
}

impl Drop for C2D {
    fn drop(&mut self) {
        unsafe {
            C3D_RenderTargetDelete(self.top_render_target_left);
            C3D_RenderTargetDelete(self.bottom_render_target);
            c2d_raw_fini();
        }
    }
}
