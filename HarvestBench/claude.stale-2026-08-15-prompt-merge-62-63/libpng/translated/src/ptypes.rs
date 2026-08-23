//! Public libpng types (png_color, png_text, …) and the fundamental type
//! aliases from pngconf.h.  These are part of the public ABI and their layout
//! is fixed by png.h.
#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::cffi::{size_t, tm, time_t, z_stream, FILE};
pub use crate::pstruct::{png_info_def, png_struct_def};

// ---- Fundamental scalar aliases (pngconf.h) ----
pub type png_byte = u8;
pub type png_int_16 = i16;
pub type png_uint_16 = u16;
pub type png_int_32 = i32;
pub type png_uint_32 = u32;
pub type png_size_t = size_t;
pub type png_ptrdiff_t = isize;
pub type png_alloc_size_t = size_t;
pub type png_fixed_point = png_int_32;

pub type png_voidp = *mut c_void;
pub type png_const_voidp = *const c_void;
pub type png_bytep = *mut png_byte;
pub type png_const_bytep = *const png_byte;
pub type png_uint_32p = *mut png_uint_32;
pub type png_const_uint_32p = *const png_uint_32;
pub type png_int_32p = *mut png_int_32;
pub type png_uint_16p = *mut png_uint_16;
pub type png_const_uint_16p = *const png_uint_16;
pub type png_int_16p = *mut png_int_16;
pub type png_charp = *mut c_char;
pub type png_const_charp = *const c_char;
pub type png_fixed_point_p = *mut png_fixed_point;
pub type png_const_fixed_point_p = *const png_fixed_point;
pub type png_size_tp = *mut size_t;

pub type png_doublep = *mut f64;
pub type png_const_doublep = *const f64;

pub type png_bytepp = *mut png_bytep;
pub type png_uint_32pp = *mut png_uint_32p;
pub type png_uint_16pp = *mut png_uint_16p;
pub type png_const_uint_16pp = *const png_uint_16p;
pub type png_charpp = *mut png_charp;
pub type png_const_charpp = *const png_const_charp;
pub type png_doublepp = *mut png_doublep;
pub type png_charppp = *mut png_charpp;

pub type png_FILE_p = *mut FILE;

// ---- Opaque handle types ----
pub type png_structp = *mut png_struct_def;
pub type png_const_structp = *const png_struct_def;
pub type png_structpp = *mut png_structp;
pub type png_structrp = *mut png_struct_def;
pub type png_const_structrp = *const png_struct_def;

pub type png_infop = *mut png_info_def;
pub type png_const_infop = *const png_info_def;
pub type png_infopp = *mut png_infop;
pub type png_inforp = *mut png_info_def;
pub type png_const_inforp = *const png_info_def;

// ---- Public structs (fixed ABI) ----
#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_color {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
}
pub type png_colorp = *mut png_color;
pub type png_const_colorp = *const png_color;
pub type png_colorpp = *mut png_colorp;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_color_16 {
    pub index: png_byte,
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub gray: png_uint_16,
}
pub type png_color_16p = *mut png_color_16;
pub type png_const_color_16p = *const png_color_16;
pub type png_color_16pp = *mut png_color_16p;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_color_8 {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
    pub gray: png_byte,
    pub alpha: png_byte,
}
pub type png_color_8p = *mut png_color_8;
pub type png_const_color_8p = *const png_color_8;
pub type png_color_8pp = *mut png_color_8p;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_sPLT_entry {
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub alpha: png_uint_16,
    pub frequency: png_uint_16,
}
pub type png_sPLT_entryp = *mut png_sPLT_entry;
pub type png_const_sPLT_entryp = *const png_sPLT_entry;
pub type png_sPLT_entrypp = *mut png_sPLT_entryp;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_sPLT_t {
    pub name: png_charp,
    pub depth: png_byte,
    pub entries: png_sPLT_entryp,
    pub nentries: png_int_32,
}
pub type png_sPLT_tp = *mut png_sPLT_t;
pub type png_const_sPLT_tp = *const png_sPLT_t;
pub type png_sPLT_tpp = *mut png_sPLT_tp;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_text {
    pub compression: c_int,
    pub key: png_charp,
    pub text: png_charp,
    pub text_length: size_t,
    pub itxt_length: size_t,
    pub lang: png_charp,
    pub lang_key: png_charp,
}
pub type png_textp = *mut png_text;
pub type png_const_textp = *const png_text;
pub type png_textpp = *mut png_textp;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_time {
    pub year: png_uint_16,
    pub month: png_byte,
    pub day: png_byte,
    pub hour: png_byte,
    pub minute: png_byte,
    pub second: png_byte,
}
pub type png_timep = *mut png_time;
pub type png_const_timep = *const png_time;
pub type png_timepp = *mut png_timep;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_unknown_chunk {
    pub name: [png_byte; 5],
    pub data: *mut png_byte,
    pub size: size_t,
    pub location: png_byte,
}
pub type png_unknown_chunkp = *mut png_unknown_chunk;
pub type png_const_unknown_chunkp = *const png_unknown_chunk;
pub type png_unknown_chunkpp = *mut png_unknown_chunkp;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_row_info {
    pub width: png_uint_32,
    pub rowbytes: size_t,
    pub color_type: png_byte,
    pub bit_depth: png_byte,
    pub channels: png_byte,
    pub pixel_depth: png_byte,
}
pub type png_row_infop = *mut png_row_info;
pub type png_row_infopp = *mut png_row_infop;

// ---- Colorspace helper structs (pngstruct.h) ----
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_xy {
    pub redx: png_fixed_point,
    pub redy: png_fixed_point,
    pub greenx: png_fixed_point,
    pub greeny: png_fixed_point,
    pub bluex: png_fixed_point,
    pub bluey: png_fixed_point,
    pub whitex: png_fixed_point,
    pub whitey: png_fixed_point,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_XYZ {
    pub red_X: png_fixed_point,
    pub red_Y: png_fixed_point,
    pub red_Z: png_fixed_point,
    pub green_X: png_fixed_point,
    pub green_Y: png_fixed_point,
    pub green_Z: png_fixed_point,
    pub blue_X: png_fixed_point,
    pub blue_Y: png_fixed_point,
    pub blue_Z: png_fixed_point,
}

// ---- Callback function pointer types ----
pub type png_error_ptr = Option<unsafe extern "C" fn(png_structp, png_const_charp)>;
pub type png_rw_ptr = Option<unsafe extern "C" fn(png_structp, png_bytep, size_t)>;
pub type png_flush_ptr = Option<unsafe extern "C" fn(png_structp)>;
pub type png_read_status_ptr = Option<unsafe extern "C" fn(png_structp, png_uint_32, c_int)>;
pub type png_write_status_ptr = Option<unsafe extern "C" fn(png_structp, png_uint_32, c_int)>;
pub type png_progressive_info_ptr = Option<unsafe extern "C" fn(png_structp, png_infop)>;
pub type png_progressive_end_ptr = Option<unsafe extern "C" fn(png_structp, png_infop)>;
pub type png_progressive_row_ptr =
    Option<unsafe extern "C" fn(png_structp, png_bytep, png_uint_32, c_int)>;
pub type png_user_transform_ptr =
    Option<unsafe extern "C" fn(png_structp, png_row_infop, png_bytep)>;
pub type png_user_chunk_ptr = Option<unsafe extern "C" fn(png_structp, png_unknown_chunkp) -> c_int>;
pub type png_longjmp_ptr = Option<unsafe extern "C" fn(*mut c_void, c_int) -> !>;
pub type png_malloc_ptr = Option<unsafe extern "C" fn(png_structp, png_alloc_size_t) -> png_voidp>;
pub type png_free_ptr = Option<unsafe extern "C" fn(png_structp, png_voidp)>;

/// Read-filter callback stored in png_struct::read_filter.
pub type png_read_filter_fn =
    Option<unsafe extern "C" fn(png_row_infop, png_bytep, png_const_bytep)>;

// ---- Simplified API (png_image) ----
#[repr(C)]
pub struct png_image {
    pub opaque: *mut png_control,
    pub version: png_uint_32,
    pub width: png_uint_32,
    pub height: png_uint_32,
    pub format: png_uint_32,
    pub flags: png_uint_32,
    pub colormap_entries: png_uint_32,
    pub warning_or_error: png_uint_32,
    pub message: [c_char; 64],
}
pub type png_imagep = *mut png_image;

#[repr(C)]
pub struct png_control {
    pub png_ptr: png_structp,
    pub info_ptr: png_infop,
    pub error_buf: png_voidp,
    pub memory: png_const_bytep,
    pub size: size_t,
    /// bitfield: for_write:1, owned_file:1 packed into a byte
    pub bitfields: c_uint,
}
pub type png_controlp = *mut png_control;

impl png_control {
    #[inline]
    pub fn for_write(&self) -> bool {
        (self.bitfields & 1) != 0
    }
    #[inline]
    pub fn set_for_write(&mut self, v: bool) {
        if v {
            self.bitfields |= 1;
        } else {
            self.bitfields &= !1;
        }
    }
    #[inline]
    pub fn owned_file(&self) -> bool {
        (self.bitfields & 2) != 0
    }
    #[inline]
    pub fn set_owned_file(&mut self, v: bool) {
        if v {
            self.bitfields |= 2;
        } else {
            self.bitfields &= !2;
        }
    }
}

// ---- Write-side compression buffer list (pngstruct.h) ----
#[repr(C)]
pub struct png_compression_buffer {
    pub next: *mut png_compression_buffer,
    pub output: [png_byte; 1], // actually zbuffer_size
}
pub type png_compression_bufferp = *mut png_compression_buffer;

/// Internal count of filter values (used for the read_filter array length).
/// The public `PNG_FILTER_VALUE_LAST` constant lives in `consts`.
pub const PNG_FILTER_VALUE_COUNT: usize = 5;

// Re-export z_stream for pngstruct.
pub use crate::cffi::z_stream as png_zstream;

// Silence unused import warnings for aliases used only by other modules.
#[allow(unused_imports)]
use crate::cffi::{time_t as _time_t, tm as _tm};
const _: () = {
    let _ = core::mem::size_of::<z_stream>();
    let _ = core::mem::size_of::<tm>();
    let _: time_t = 0;
};
