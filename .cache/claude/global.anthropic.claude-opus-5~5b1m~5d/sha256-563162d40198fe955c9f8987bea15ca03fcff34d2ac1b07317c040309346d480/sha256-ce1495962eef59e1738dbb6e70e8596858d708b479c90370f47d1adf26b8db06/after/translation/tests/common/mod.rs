//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading`:
//!   * the reference C build  `c_src/build/libpng.so`
//!   * the Rust translation   `translation/target/<profile>/liblibpng.so`
//!
//! Nothing in the Rust crate is ever called directly — every call goes through
//! the `.so` export table, exactly as an external C consumer would do, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub mod api;
pub mod harness;
pub mod pngbuild;

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libpng public types (repr(C) mirrors of png.h)
// ---------------------------------------------------------------------------

pub type png_byte = u8;
pub type png_uint_32 = u32;
pub type png_int_32 = i32;
pub type png_uint_16 = u16;
pub type png_int_16 = i16;
pub type png_fixed_point = i32;
pub type png_size_t = usize;
pub type png_alloc_size_t = usize;
pub type png_structp = *mut c_void;
pub type png_infop = *mut c_void;
pub type png_bytep = *mut png_byte;
pub type png_const_bytep = *const png_byte;
pub type png_charp = *mut c_char;
pub type png_const_charp = *const c_char;
pub type png_voidp = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct png_color {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct png_color_8 {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
    pub gray: png_byte,
    pub alpha: png_byte,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct png_color_16 {
    pub index: png_byte,
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub gray: png_uint_16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct png_sPLT_entry {
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub alpha: png_uint_16,
    pub frequency: png_uint_16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct png_sPLT_t {
    pub name: png_charp,
    pub depth: png_byte,
    pub entries: *mut png_sPLT_entry,
    pub nentries: png_int_32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct png_text {
    pub compression: c_int,
    pub key: png_charp,
    pub text: png_charp,
    pub text_length: usize,
    pub itxt_length: usize,
    pub lang: png_charp,
    pub lang_key: png_charp,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct png_time {
    pub year: png_uint_16,
    pub month: png_byte,
    pub day: png_byte,
    pub hour: png_byte,
    pub minute: png_byte,
    pub second: png_byte,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct png_unknown_chunk {
    pub name: [png_byte; 5],
    pub data: *mut png_byte,
    pub size: usize,
    pub location: png_byte,
}

/// `png_image` from png.h — the simplified API entry structure.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_image {
    pub opaque: *mut c_void,
    pub version: png_uint_32,
    pub width: png_uint_32,
    pub height: png_uint_32,
    pub format: png_uint_32,
    pub flags: png_uint_32,
    pub colormap_entries: png_uint_32,
    pub warning_or_error: png_uint_32,
    pub message: [c_char; 64],
}

impl Default for png_image {
    fn default() -> Self {
        png_image {
            opaque: std::ptr::null_mut(),
            version: PNG_IMAGE_VERSION,
            width: 0,
            height: 0,
            format: 0,
            flags: 0,
            colormap_entries: 0,
            warning_or_error: 0,
            message: [0; 64],
        }
    }
}

impl png_image {
    pub fn msg(&self) -> String {
        let b: Vec<u8> = self
            .message
            .iter()
            .take_while(|c| **c != 0)
            .map(|c| *c as u8)
            .collect();
        String::from_utf8_lossy(&b).into_owned()
    }
    /// Everything except `opaque` (which is an allocation address and therefore
    /// legitimately differs between the two libraries).
    pub fn cmp_tuple(&self) -> (u32, u32, u32, u32, u32, u32, u32, String) {
        (
            self.version,
            self.width,
            self.height,
            self.format,
            self.flags,
            self.colormap_entries,
            self.warning_or_error,
            self.msg(),
        )
    }
}

impl std::fmt::Debug for png_image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("png_image")
            .field("version", &self.version)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("format", &format_args!("0x{:x}", self.format))
            .field("flags", &self.flags)
            .field("colormap_entries", &self.colormap_entries)
            .field("warning_or_error", &self.warning_or_error)
            .field("message", &self.msg())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Constants from png.h / pngconf.h
// ---------------------------------------------------------------------------

pub const PNG_LIBPNG_VER_STRING: &[u8] = b"1.6.59.git\0";
pub const PNG_IMAGE_VERSION: png_uint_32 = 1;
pub const PNG_IMAGE_WARNING: png_uint_32 = 1;
pub const PNG_IMAGE_ERROR: png_uint_32 = 2;

pub const PNG_COLOR_MASK_PALETTE: c_int = 1;
pub const PNG_COLOR_MASK_COLOR: c_int = 2;
pub const PNG_COLOR_MASK_ALPHA: c_int = 4;
pub const PNG_COLOR_TYPE_GRAY: c_int = 0;
pub const PNG_COLOR_TYPE_PALETTE: c_int = 3;
pub const PNG_COLOR_TYPE_RGB: c_int = 2;
pub const PNG_COLOR_TYPE_RGB_ALPHA: c_int = 6;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: c_int = 4;

pub const PNG_COMPRESSION_TYPE_BASE: c_int = 0;
pub const PNG_FILTER_TYPE_BASE: c_int = 0;
pub const PNG_INTRAPIXEL_DIFFERENCING: c_int = 64;
pub const PNG_INTERLACE_NONE: c_int = 0;
pub const PNG_INTERLACE_ADAM7: c_int = 1;

pub const PNG_NO_FILTERS: c_int = 0x00;
pub const PNG_FILTER_NONE: c_int = 0x08;
pub const PNG_FILTER_SUB: c_int = 0x10;
pub const PNG_FILTER_UP: c_int = 0x20;
pub const PNG_FILTER_AVG: c_int = 0x40;
pub const PNG_FILTER_PAETH: c_int = 0x80;
pub const PNG_ALL_FILTERS: c_int = 0xf8;

pub const PNG_FILTER_VALUE_NONE: c_int = 0;
pub const PNG_FILTER_VALUE_SUB: c_int = 1;
pub const PNG_FILTER_VALUE_UP: c_int = 2;
pub const PNG_FILTER_VALUE_AVG: c_int = 3;
pub const PNG_FILTER_VALUE_PAETH: c_int = 4;
pub const PNG_FILTER_VALUE_LAST: c_int = 5;

pub const PNG_INFO_gAMA: png_uint_32 = 0x0001;
pub const PNG_INFO_sBIT: png_uint_32 = 0x0002;
pub const PNG_INFO_cHRM: png_uint_32 = 0x0004;
pub const PNG_INFO_PLTE: png_uint_32 = 0x0008;
pub const PNG_INFO_tRNS: png_uint_32 = 0x0010;
pub const PNG_INFO_bKGD: png_uint_32 = 0x0020;
pub const PNG_INFO_hIST: png_uint_32 = 0x0040;
pub const PNG_INFO_pHYs: png_uint_32 = 0x0080;
pub const PNG_INFO_oFFs: png_uint_32 = 0x0100;
pub const PNG_INFO_tIME: png_uint_32 = 0x0200;
pub const PNG_INFO_pCAL: png_uint_32 = 0x0400;
pub const PNG_INFO_sRGB: png_uint_32 = 0x0800;
pub const PNG_INFO_iCCP: png_uint_32 = 0x1000;
pub const PNG_INFO_sPLT: png_uint_32 = 0x2000;
pub const PNG_INFO_sCAL: png_uint_32 = 0x4000;
pub const PNG_INFO_IDAT: png_uint_32 = 0x8000;
pub const PNG_INFO_eXIf: png_uint_32 = 0x10000;
pub const PNG_INFO_cICP: png_uint_32 = 0x20000;
pub const PNG_INFO_cLLI: png_uint_32 = 0x40000;
pub const PNG_INFO_mDCV: png_uint_32 = 0x80000;

pub const PNG_TRANSFORM_IDENTITY: c_int = 0x0000;
pub const PNG_TRANSFORM_STRIP_16: c_int = 0x0001;
pub const PNG_TRANSFORM_STRIP_ALPHA: c_int = 0x0002;
pub const PNG_TRANSFORM_PACKING: c_int = 0x0004;
pub const PNG_TRANSFORM_PACKSWAP: c_int = 0x0008;
pub const PNG_TRANSFORM_EXPAND: c_int = 0x0010;
pub const PNG_TRANSFORM_INVERT_MONO: c_int = 0x0020;
pub const PNG_TRANSFORM_SHIFT: c_int = 0x0040;
pub const PNG_TRANSFORM_BGR: c_int = 0x0080;
pub const PNG_TRANSFORM_SWAP_ALPHA: c_int = 0x0100;
pub const PNG_TRANSFORM_SWAP_ENDIAN: c_int = 0x0200;
pub const PNG_TRANSFORM_INVERT_ALPHA: c_int = 0x0400;
pub const PNG_TRANSFORM_STRIP_FILLER: c_int = 0x0800;
pub const PNG_TRANSFORM_STRIP_FILLER_BEFORE: c_int = 0x0800;
pub const PNG_TRANSFORM_STRIP_FILLER_AFTER: c_int = 0x1000;
pub const PNG_TRANSFORM_GRAY_TO_RGB: c_int = 0x2000;
pub const PNG_TRANSFORM_EXPAND_16: c_int = 0x4000;
pub const PNG_TRANSFORM_SCALE_16: c_int = 0x8000;

pub const PNG_FILLER_BEFORE: c_int = 0;
pub const PNG_FILLER_AFTER: c_int = 1;

pub const PNG_BACKGROUND_GAMMA_UNKNOWN: c_int = 0;
pub const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;
pub const PNG_BACKGROUND_GAMMA_FILE: c_int = 2;
pub const PNG_BACKGROUND_GAMMA_UNIQUE: c_int = 3;

pub const PNG_ALPHA_PNG: c_int = 0;
pub const PNG_ALPHA_STANDARD: c_int = 1;
pub const PNG_ALPHA_ASSOCIATED: c_int = 1;
pub const PNG_ALPHA_OPTIMIZED: c_int = 2;
pub const PNG_ALPHA_BROKEN: c_int = 3;

pub const PNG_DEFAULT_sRGB: png_fixed_point = -1;
pub const PNG_GAMMA_MAC_18: png_fixed_point = -2;

pub const PNG_CRC_DEFAULT: c_int = 0;
pub const PNG_CRC_ERROR_QUIT: c_int = 1;
pub const PNG_CRC_WARN_DISCARD: c_int = 2;
pub const PNG_CRC_WARN_USE: c_int = 3;
pub const PNG_CRC_QUIET_USE: c_int = 4;
pub const PNG_CRC_NO_CHANGE: c_int = 5;

pub const PNG_HANDLE_CHUNK_AS_DEFAULT: c_int = 0;
pub const PNG_HANDLE_CHUNK_NEVER: c_int = 1;
pub const PNG_HANDLE_CHUNK_IF_SAFE: c_int = 2;
pub const PNG_HANDLE_CHUNK_ALWAYS: c_int = 3;
pub const PNG_HANDLE_CHUNK_LAST: c_int = 4;

pub const PNG_OPTION_INVALID: c_int = 1;
pub const PNG_OPTION_OFF: c_int = 2;
pub const PNG_OPTION_ON: c_int = 3;
pub const PNG_MAXIMUM_INFLATE_WINDOW: c_int = 2;
pub const PNG_SKIP_sRGB_CHECK_PROFILE: c_int = 4;
pub const PNG_IGNORE_ADLER32: c_int = 8;
pub const PNG_OPTION_NEXT: c_int = 16; // png.h:3493

pub const PNG_IO_NONE: png_uint_32 = 0x0000;
pub const PNG_IO_READING: png_uint_32 = 0x0001;
pub const PNG_IO_WRITING: png_uint_32 = 0x0002;
pub const PNG_IO_SIGNATURE: png_uint_32 = 0x0010;
pub const PNG_IO_CHUNK_HDR: png_uint_32 = 0x0020;
pub const PNG_IO_CHUNK_DATA: png_uint_32 = 0x0040;
pub const PNG_IO_CHUNK_CRC: png_uint_32 = 0x0080;

pub const PNG_TEXT_COMPRESSION_NONE_WR: c_int = -3;
pub const PNG_TEXT_COMPRESSION_zTXt_WR: c_int = -2;
pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;

pub const PNG_RESOLUTION_UNKNOWN: c_int = 0;
pub const PNG_RESOLUTION_METER: c_int = 1;
pub const PNG_OFFSET_PIXEL: c_int = 0;
pub const PNG_OFFSET_MICROMETER: c_int = 1;
pub const PNG_SCALE_UNKNOWN: c_int = 0;
pub const PNG_SCALE_METER: c_int = 1;
pub const PNG_SCALE_RADIAN: c_int = 2;
pub const PNG_EQUATION_LINEAR: c_int = 0;
pub const PNG_EQUATION_BASE_E: c_int = 1;
pub const PNG_EQUATION_ARBITRARY: c_int = 2;
pub const PNG_EQUATION_HYPERBOLIC: c_int = 3;
pub const PNG_EQUATION_LAST: c_int = 4;

pub const PNG_sRGB_INTENT_PERCEPTUAL: c_int = 0;
pub const PNG_sRGB_INTENT_RELATIVE: c_int = 1;
pub const PNG_sRGB_INTENT_SATURATION: c_int = 2;
pub const PNG_sRGB_INTENT_ABSOLUTE: c_int = 3;
pub const PNG_sRGB_INTENT_LAST: c_int = 4;

pub const PNG_FLAG_MNG_EMPTY_PLTE: c_int = 0x01;
pub const PNG_FLAG_MNG_FILTER_64: c_int = 0x04;
pub const PNG_ALL_MNG_FEATURES: c_int = 0x05;

pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fff_ffff;
pub const PNG_UINT_32_MAX: png_uint_32 = u32::MAX;

// simplified-API formats
pub const PNG_FORMAT_FLAG_ALPHA: png_uint_32 = 0x01;
pub const PNG_FORMAT_FLAG_COLOR: png_uint_32 = 0x02;
pub const PNG_FORMAT_FLAG_LINEAR: png_uint_32 = 0x04;
pub const PNG_FORMAT_FLAG_COLORMAP: png_uint_32 = 0x08;
pub const PNG_FORMAT_FLAG_BGR: png_uint_32 = 0x10;
pub const PNG_FORMAT_FLAG_AFIRST: png_uint_32 = 0x20;
pub const PNG_FORMAT_FLAG_ASSOCIATED_ALPHA: png_uint_32 = 0x40;

pub const PNG_FORMAT_GRAY: png_uint_32 = 0;
pub const PNG_FORMAT_GA: png_uint_32 = PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_AG: png_uint_32 = PNG_FORMAT_GA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_RGB: png_uint_32 = PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_BGR: png_uint_32 = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_BGR;
pub const PNG_FORMAT_RGBA: png_uint_32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ARGB: png_uint_32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_BGRA: png_uint_32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ABGR: png_uint_32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_LINEAR_Y: png_uint_32 = PNG_FORMAT_FLAG_LINEAR;
pub const PNG_FORMAT_LINEAR_Y_ALPHA: png_uint_32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_LINEAR_RGB: png_uint_32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_LINEAR_RGB_ALPHA: png_uint_32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_RGB_COLORMAP: png_uint_32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGR_COLORMAP: png_uint_32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_RGBA_COLORMAP: png_uint_32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ARGB_COLORMAP: png_uint_32 = PNG_FORMAT_ARGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGRA_COLORMAP: png_uint_32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ABGR_COLORMAP: png_uint_32 = PNG_FORMAT_ABGR | PNG_FORMAT_FLAG_COLORMAP;

pub const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: png_uint_32 = 0x01;
pub const PNG_IMAGE_FLAG_FAST: png_uint_32 = 0x02;
pub const PNG_IMAGE_FLAG_16BIT_sRGB: png_uint_32 = 0x04;

// ---------------------------------------------------------------------------
// The two libraries
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("PNG_C_SO") {
        return PathBuf::from(p);
    }
    repo_root().join("c_src/build/libpng.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("PNG_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = repo_root().join("translation/target");
    // Prefer the profile the test binary itself was built with.
    let (first, second) = if cfg!(debug_assertions) {
        ("debug", "release")
    } else {
        ("release", "debug")
    };
    let a = base.join(first).join("liblibpng.so");
    if a.exists() {
        return a;
    }
    base.join(second).join("liblibpng.so")
}

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        // The reference C build is linked against zlib but relies on the
        // executable pulling in libm (`floor`, `pow`, ...).  A Rust test binary
        // does not, so load libm with RTLD_GLOBAL first.
        {
            use libloading::os::unix as u;
            let flags = u::RTLD_NOW | u::RTLD_GLOBAL;
            for name in ["libm.so.6", "libm.so"] {
                if let Ok(l) = unsafe { u::Library::open(Some(name), flags) } {
                    std::mem::forget(l);
                    break;
                }
            }
        }
        let cp = c_so_path();
        let rp = rust_so_path();
        // RTLD_LOCAL (libloading default) keeps the two symbol tables apart.
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("cannot load C .so {}: {e}", cp.display()));
        let rs = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("cannot load Rust .so {}: {e}", rp.display()));
        Libs { c, rs }
    })
}

/// Resolve a symbol of the given type from one of the loaded libraries.
pub fn sym<T>(lib: &'static Library, name: &str) -> T
where
    T: Copy,
{
    let cname = std::ffi::CString::new(name).unwrap();
    unsafe {
        let s: Symbol<T> = lib
            .get(cname.as_bytes_with_nul())
            .unwrap_or_else(|e| panic!("symbol {name} not found: {e}"));
        *s
    }
}

/// Fetch the same symbol from both libraries.
pub fn both<T: Copy>(name: &str) -> (T, T) {
    let l = libs();
    (sym::<T>(&l.c, name), sym::<T>(&l.rs, name))
}

/// True when the symbol exists in both libraries.
pub fn has_both(name: &str) -> bool {
    let l = libs();
    let cname = std::ffi::CString::new(name).unwrap();
    unsafe {
        l.c.get::<*const c_void>(cname.as_bytes_with_nul()).is_ok()
            && l.rs.get::<*const c_void>(cname.as_bytes_with_nul()).is_ok()
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — reproducible property-style inputs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 40) as u16
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 48) as u8
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    pub fn range(&mut self, lo: u32, hi_inclusive: u32) -> u32 {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 != 0
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }
    /// Values biased towards interesting boundaries.
    pub fn interesting_u32(&mut self) -> u32 {
        const SPECIAL: [u32; 16] = [
            0,
            1,
            2,
            0x7f,
            0x80,
            0xff,
            0x100,
            0x7fff,
            0x8000,
            0xffff,
            0x1_0000,
            0x7fff_ffff,
            0x8000_0000,
            0xffff_ffff,
            0x7fff_fffe,
            0x8000_0001,
        ];
        if self.next_u64() & 3 == 0 {
            SPECIAL[self.below(16) as usize]
        } else {
            self.next_u32()
        }
    }
}

// ---------------------------------------------------------------------------
// Function-pointer typedefs used by the tests
// ---------------------------------------------------------------------------

pub type png_error_ptr = Option<unsafe extern "C" fn(png_structp, png_const_charp)>;
pub type png_rw_ptr = Option<unsafe extern "C" fn(png_structp, png_bytep, usize)>;
pub type png_flush_ptr = Option<unsafe extern "C" fn(png_structp)>;
pub type png_read_status_ptr =
    Option<unsafe extern "C" fn(png_structp, png_uint_32, c_int)>;
pub type png_write_status_ptr =
    Option<unsafe extern "C" fn(png_structp, png_uint_32, c_int)>;
pub type png_progressive_info_ptr = Option<unsafe extern "C" fn(png_structp, png_infop)>;
pub type png_progressive_row_ptr =
    Option<unsafe extern "C" fn(png_structp, png_bytep, png_uint_32, c_int)>;
pub type png_progressive_end_ptr = Option<unsafe extern "C" fn(png_structp, png_infop)>;
pub type png_user_transform_ptr =
    Option<unsafe extern "C" fn(png_structp, *mut c_void, png_bytep)>;
pub type png_user_chunk_ptr = Option<unsafe extern "C" fn(png_structp, *mut c_void) -> c_int>;
pub type png_longjmp_ptr = Option<unsafe extern "C" fn(*mut c_void, c_int)>;
pub type png_malloc_ptr = Option<unsafe extern "C" fn(png_structp, usize) -> png_voidp>;
pub type png_free_ptr = Option<unsafe extern "C" fn(png_structp, png_voidp)>;

// ---------------------------------------------------------------------------
// Signature aliases for every entry point the tests use
// ---------------------------------------------------------------------------

pub type Fn_create_read = unsafe extern "C" fn(
    png_const_charp,
    png_voidp,
    png_error_ptr,
    png_error_ptr,
) -> png_structp;
pub type Fn_create_write = Fn_create_read;
pub type Fn_create_info = unsafe extern "C" fn(png_structp) -> png_infop;
pub type Fn_destroy_read = unsafe extern "C" fn(*mut png_structp, *mut png_infop, *mut png_infop);
pub type Fn_destroy_write = unsafe extern "C" fn(*mut png_structp, *mut png_infop);
pub type Fn_set_error_fn =
    unsafe extern "C" fn(png_structp, png_voidp, png_error_ptr, png_error_ptr);
pub type Fn_set_read_fn = unsafe extern "C" fn(png_structp, png_voidp, png_rw_ptr);
pub type Fn_set_write_fn =
    unsafe extern "C" fn(png_structp, png_voidp, png_rw_ptr, png_flush_ptr);
pub type Fn_set_longjmp = unsafe extern "C" fn(png_structp, png_longjmp_ptr, usize) -> *mut c_void;

pub type Fn_v_p = unsafe extern "C" fn(png_structp);
pub type Fn_v_pi = unsafe extern "C" fn(png_structp, png_infop);
pub type Fn_v_pii = unsafe extern "C" fn(png_structp, png_infop, c_int);
pub type Fn_i_p = unsafe extern "C" fn(png_structp) -> c_int;
pub type Fn_u_p = unsafe extern "C" fn(png_structp) -> png_uint_32;
pub type Fn_u32_pi = unsafe extern "C" fn(png_structp, png_infop) -> png_uint_32;
pub type Fn_b_pi = unsafe extern "C" fn(png_structp, png_infop) -> png_byte;
pub type Fn_sz_pi = unsafe extern "C" fn(png_structp, png_infop) -> usize;

// ---------------------------------------------------------------------------
// Error/warning capture: an `extern "C"` handler that records the message in a
// thread-local slot and then unwinds out of the library via a Rust panic ...
// which is NOT allowed for the C library.  Instead we use `png_set_longjmp_fn`
// with a real `setjmp` replacement is impossible from Rust, so the strategy is:
//
//   * install an error handler that records the message and then calls
//     `png_longjmp`-equivalent by *panicking*.  For the C library the panic
//     unwinds through C frames compiled with `-fexceptions`?  Not guaranteed.
//
// The portable approach used throughout these tests: run the library call in a
// child *process* is too slow.  Instead we exploit the documented libpng
// contract: the error handler must not return.  We therefore record the message
// and `longjmp` using the `jmp_buf` libpng itself owns, by calling the
// exported `png_longjmp` only in the Rust case ... also not portable.
//
// => The tests below never let an error escape when the C library is involved
//    unless the call is made through `png_safe_execute`-based entry points
//    (the simplified API, `png_image_*`), which return an error code instead of
//    longjmp'ing.  For direct low-level calls the tests use *valid* input, and
//    error paths are exercised through:
//      - the simplified API (returns 0 + message in png_image.message)
//      - `png_set_*` app errors, which are warnings in this build for the read
//        side and thus return normally
//      - getters/utility functions, which return sentinels
//    plus `abort_on_error` handlers that make a divergence loud.
// ---------------------------------------------------------------------------

/// A do-nothing warning handler (libpng calls it and continues).
pub unsafe extern "C" fn ignore_warning(_p: png_structp, _m: png_const_charp) {}

pub fn cstr_to_string(p: png_const_charp) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_dbg<T: PartialEq + std::fmt::Debug>(what: &str, c: T, r: T) {
    assert_eq!(c, r, "MISMATCH in {what}: C={c:?} RUST={r:?}");
}

#[track_caller]
pub fn eq_bytes(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let n = c.len().min(r.len());
        let mut first = n;
        for i in 0..n {
            if c[i] != r[i] {
                first = i;
                break;
            }
        }
        panic!(
            "MISMATCH in {what}: lengths C={} RUST={}, first differing byte at {}\n  C   [{}..]={:02x?}\n  RUST[{}..]={:02x?}",
            c.len(),
            r.len(),
            first,
            first,
            &c[first..(first + 24).min(c.len())],
            first,
            &r[first..(first + 24).min(r.len())],
        );
    }
}

// re-export the ffi scalar types so test files need only `use common::*`
pub use std::ffi::{c_char as CChar, c_int as CInt};
pub type CDouble = c_double;
pub type CLong = c_long;
pub type CUint = c_uint;
pub type CUlong = c_ulong;
pub type CVoid = c_void;
