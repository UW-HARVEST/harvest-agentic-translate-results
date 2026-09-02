//! Differential-test harness: loads BOTH the reference C `libpng.so` and the
//! translated Rust `liblibpng.so` through `libloading` and drives them through
//! their exported C symbols only.  Nothing in the Rust crate is called directly.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr};
use std::ptr;

// ---------------------------------------------------------------------------
// C structs shared across the FFI boundary (layouts copied from png.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PngColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PngColor8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub gray: u8,
    pub alpha: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PngColor16 {
    pub index: u8,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub gray: u16,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PngText {
    pub compression: c_int,
    pub key: *mut c_char,
    pub text: *mut c_char,
    pub text_length: usize,
    pub itxt_length: usize,
    pub lang: *mut c_char,
    pub lang_key: *mut c_char,
}

impl Default for PngText {
    fn default() -> Self {
        PngText {
            compression: -1,
            key: ptr::null_mut(),
            text: ptr::null_mut(),
            text_length: 0,
            itxt_length: 0,
            lang: ptr::null_mut(),
            lang_key: ptr::null_mut(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PngTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PngUnknownChunk {
    pub name: [u8; 5],
    pub data: *mut u8,
    pub size: usize,
    pub location: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PngSpltEntry {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub alpha: u16,
    pub frequency: u16,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PngSpltT {
    pub name: *mut c_char,
    pub depth: u8,
    pub entries: *mut PngSpltEntry,
    pub nentries: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PngImage {
    pub opaque: *mut c_void,
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub flags: u32,
    pub colormap_entries: u32,
    pub warning_or_error: u32,
    pub message: [c_char; 64],
}

impl Default for PngImage {
    fn default() -> Self {
        PngImage {
            opaque: ptr::null_mut(),
            version: 1,
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

impl PngImage {
    pub fn msg(&self) -> String {
        let b: Vec<u8> = self.message.iter().map(|&c| c as u8).collect();
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        String::from_utf8_lossy(&b[..end]).into_owned()
    }
}

// ---------------------------------------------------------------------------
// The `api!` macro: one field per exported C function, resolved with dlsym.
// ---------------------------------------------------------------------------

macro_rules! api {
    ( $( fn $name:ident ( $($arg:ty),* $(,)? ) $(-> $ret:ty)? ; )* ) => {
        pub struct Api {
            pub which: &'static str,
            _lib: libloading::Library,
            $( pub $name: unsafe extern "C" fn($($arg),*) $(-> $ret)? , )*
        }

        impl Api {
            pub fn open(path: &str, which: &'static str) -> Api {
                unsafe {
                    let lib = libloading::Library::new(path)
                        .unwrap_or_else(|e| panic!("dlopen {path}: {e}"));
                    $(
                        let $name = {
                            let s: libloading::Symbol<
                                unsafe extern "C" fn($($arg),*) $(-> $ret)?
                            > = lib
                                .get(concat!(stringify!($name), "\0").as_bytes())
                                .unwrap_or_else(|e| {
                                    panic!("{}: missing symbol {}: {}",
                                        path, stringify!($name), e)
                                });
                            *s
                        };
                    )*
                    Api { which, _lib: lib, $( $name, )* }
                }
            }
        }
    };
}

macro_rules! api_priv {
    ( $( fn $name:ident ( $($arg:ty),* $(,)? ) $(-> $ret:ty)? ; )* ) => {
        /// pngpriv.h entry points, resolved from the same shared object.
        pub struct Priv {
            pub which: &'static str,
            _lib: libloading::Library,
            pub sRGB_table: *const u16,
            pub sRGB_base: *const u16,
            pub sRGB_delta: *const u8,
            $( pub $name: unsafe extern "C" fn($($arg),*) $(-> $ret)? , )*
        }

        impl Priv {
            pub fn open(path: &str, which: &'static str) -> Priv {
                unsafe {
                    let lib = libloading::Library::new(path)
                        .unwrap_or_else(|e| panic!("dlopen {path}: {e}"));
                    let sRGB_table = *lib
                        .get::<*const u16>(b"png_sRGB_table\0")
                        .expect("png_sRGB_table");
                    let sRGB_base = *lib
                        .get::<*const u16>(b"png_sRGB_base\0")
                        .expect("png_sRGB_base");
                    let sRGB_delta = *lib
                        .get::<*const u8>(b"png_sRGB_delta\0")
                        .expect("png_sRGB_delta");
                    $(
                        let $name = {
                            let s: libloading::Symbol<
                                unsafe extern "C" fn($($arg),*) $(-> $ret)?
                            > = lib
                                .get(concat!(stringify!($name), "\0").as_bytes())
                                .unwrap_or_else(|e| {
                                    panic!("{}: missing internal symbol {}: {}",
                                        path, stringify!($name), e)
                                });
                            *s
                        };
                    )*
                    Priv { which, _lib: lib, sRGB_table, sRGB_base, sRGB_delta, $( $name, )* }
                }
            }
        }
    };
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PngXy {
    pub redx: i32,
    pub redy: i32,
    pub greenx: i32,
    pub greeny: i32,
    pub bluex: i32,
    pub bluey: i32,
    pub whitex: i32,
    pub whitey: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PngXYZ {
    pub red_X: i32,
    pub red_Y: i32,
    pub red_Z: i32,
    pub green_X: i32,
    pub green_Y: i32,
    pub green_Z: i32,
    pub blue_X: i32,
    pub blue_Y: i32,
    pub blue_Z: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PngRowInfo {
    pub width: u32,
    pub rowbytes: usize,
    pub color_type: u8,
    pub bit_depth: u8,
    pub channels: u8,
    pub pixel_depth: u8,
}

pub const PNG_NUMBER_FORMAT_u: c_int = 1;
pub const PNG_NUMBER_FORMAT_02u: c_int = 2;
pub const PNG_NUMBER_FORMAT_x: c_int = 3;
pub const PNG_NUMBER_FORMAT_02x: c_int = 4;
pub const PNG_NUMBER_FORMAT_fixed: c_int = 5;
pub const PNG_PACKSWAP: u32 = 0x10000;

include!("api_gen.rs");

pub fn c_priv() -> Priv {
    Priv::open(C_SO, "C")
}
pub fn rust_priv() -> Priv {
    Priv::open(RUST_SO, "RUST")
}
pub fn both_priv() -> (Priv, Priv) {
    (c_priv(), rust_priv())
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

pub const C_SO: &str = "../c_src/build/libpng.so";
pub const RUST_SO: &str = "target/release/liblibpng.so";

pub fn c_api() -> Api {
    Api::open(C_SO, "C")
}
pub fn rust_api() -> Api {
    Api::open(RUST_SO, "RUST")
}

/// Both implementations, to be driven with the same closure.
pub fn both() -> (Api, Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// setjmp / longjmp plumbing.  `png_jmpbuf(p)` in C expands to
// `*png_set_longjmp_fn(p, longjmp, sizeof(jmp_buf))`; we do exactly that and
// then setjmp() on the returned buffer ourselves.
// ---------------------------------------------------------------------------

pub const JMP_BUF_SIZE: usize = 200; // glibc x86-64 sizeof(jmp_buf)

extern "C" {
    fn setjmp(env: *mut c_void) -> c_int;
    fn longjmp(env: *mut c_void, val: c_int) -> !;
}

pub fn longjmp_addr() -> *mut c_void {
    longjmp as *const () as *mut c_void
}

// ---------------------------------------------------------------------------
// Per-run recording context.  Held in a thread-local so the extern "C"
// callbacks can reach it without needing png_get_io_ptr(), and so that a
// longjmp out of libpng cannot lose the data collected so far.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Ctx {
    /// Bytes libpng wrote through our write callback.
    pub out: Vec<u8>,
    /// Bytes we feed libpng through our read callback.
    pub input: Vec<u8>,
    pub pos: usize,
    /// Set when the read callback runs out of data.
    pub read_underrun: bool,
    pub flushes: u32,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    /// Free-form log of return values / observations, compared verbatim.
    pub log: Vec<String>,
    /// row callbacks: (row, pass)
    pub rows: Vec<(u32, c_int)>,
    /// progressive-read row payloads
    pub prows: Vec<(u32, c_int, Vec<u8>)>,
    /// user-chunk callback: chunk names seen
    pub uchunks: Vec<String>,
    /// value returned by the user chunk callback
    pub uchunk_ret: c_int,
}

impl Ctx {
    pub fn digest(&self) -> Report {
        Report {
            out: self.out.clone(),
            flushes: self.flushes,
            warnings: self.warnings.clone(),
            error: self.error.clone(),
            log: self.log.clone(),
            rows: self.rows.clone(),
            prows: self.prows.clone(),
            uchunks: self.uchunks.clone(),
            read_underrun: self.read_underrun,
        }
    }
}

/// Everything a differential test compares.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub out: Vec<u8>,
    pub flushes: u32,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub log: Vec<String>,
    pub rows: Vec<(u32, c_int)>,
    pub prows: Vec<(u32, c_int, Vec<u8>)>,
    pub uchunks: Vec<String>,
    pub read_underrun: bool,
}

impl Report {
    /// Compact, readable difference summary for assertion messages.
    pub fn brief(&self) -> String {
        format!(
            "out={}B out_head={:02x?} flushes={} err={:?} warns={:?} log={:?} rows={} prows={} uchunks={:?} underrun={}",
            self.out.len(),
            &self.out[..self.out.len().min(32)],
            self.flushes,
            self.error,
            self.warnings,
            self.log,
            self.rows.len(),
            self.prows.len(),
            self.uchunks,
            self.read_underrun
        )
    }
}

thread_local! {
    static CUR: RefCell<*mut Ctx> = const { RefCell::new(ptr::null_mut()) };
}

fn ctx() -> &'static mut Ctx {
    CUR.with(|c| {
        let p = *c.borrow();
        assert!(!p.is_null(), "no Ctx installed");
        unsafe { &mut *p }
    })
}

pub fn set_ctx(p: *mut Ctx) {
    CUR.with(|c| *c.borrow_mut() = p);
}

fn cstr_to_string(s: *const c_char) -> String {
    if s.is_null() {
        "<null>".to_string()
    } else {
        unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned()
    }
}

// ---------------------------------------------------------------------------
// Callbacks handed to libpng
// ---------------------------------------------------------------------------

pub unsafe extern "C" fn cb_write(_png: *mut c_void, data: *mut u8, len: usize) {
    let c = ctx();
    if len > 0 && !data.is_null() {
        c.out.extend_from_slice(std::slice::from_raw_parts(data, len));
    }
}

pub unsafe extern "C" fn cb_flush(_png: *mut c_void) {
    ctx().flushes += 1;
}

pub unsafe extern "C" fn cb_read(_png: *mut c_void, data: *mut u8, len: usize) {
    let c = ctx();
    let avail = c.input.len().saturating_sub(c.pos);
    let n = len.min(avail);
    if n < len {
        c.read_underrun = true;
    }
    if n > 0 && !data.is_null() {
        ptr::copy_nonoverlapping(c.input.as_ptr().add(c.pos), data, n);
    }
    // Zero-fill any shortfall so behaviour is deterministic in both libs.
    if n < len && !data.is_null() {
        ptr::write_bytes(data.add(n), 0, len - n);
    }
    c.pos += n;
}

// ---------------------------------------------------------------------------
// ERRORS.md coverage instrumentation: every distinct error/warning message text
// either implementation emits during the whole test run is appended to
// `target/observed_messages.txt`, so Phase C coverage can be checked against
// the message literals in the C source.
// ---------------------------------------------------------------------------

fn record_message(kind: &str, msg: &str) {
    use std::collections::HashSet;
    use std::io::Write;
    use std::sync::Mutex;
    use std::sync::OnceLock;
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = format!("{kind}\t{msg}");
    let mut g = seen.lock().unwrap();
    if g.insert(key.clone()) {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("target/observed_messages.txt")
        {
            let _ = writeln!(f, "{key}");
        }
    }
}

pub unsafe extern "C" fn cb_error(_png: *mut c_void, msg: *const c_char) {
    let c = ctx();
    let m = cstr_to_string(msg);
    record_message("error", &m);
    if c.error.is_none() {
        c.error = Some(m);
    }
    // Returning lets libpng's own png_default_error()/png_longjmp() run, so the
    // default error path is exercised too.
}

pub unsafe extern "C" fn cb_warn(_png: *mut c_void, msg: *const c_char) {
    let m = cstr_to_string(msg);
    record_message("warning", &m);
    ctx().warnings.push(m);
}

pub unsafe extern "C" fn cb_row(_png: *mut c_void, row: u32, pass: c_int) {
    ctx().rows.push((row, pass));
}

// ---------------------------------------------------------------------------
// `protect`: run a closure with libpng's longjmp-on-error active.
//
// Everything the closure records lives in the heap-allocated Ctx (reached via
// the thread-local), never in this frame, so a longjmp back here cannot lose
// or corrupt it.  The closure is a &mut dyn FnMut so there is no drop flag in
// this frame either.
// ---------------------------------------------------------------------------

pub fn protect(api: &Api, png_ptr: *mut c_void, f: &mut dyn FnMut()) -> bool {
    unsafe {
        let jb = (api.png_set_longjmp_fn)(png_ptr, longjmp_addr(), JMP_BUF_SIZE);
        if jb.is_null() {
            ctx().log.push("png_set_longjmp_fn=NULL".into());
            return false;
        }
        if setjmp(jb as *mut c_void) == 0 {
            f();
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Small deterministic PRNG (xorshift64*) so every property-style test is
// reproducible from a fixed seed.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.u32() % n
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
}

// ---------------------------------------------------------------------------
// png.h constants used by the tests
// ---------------------------------------------------------------------------

pub const PNG_COLOR_TYPE_GRAY: c_int = 0;
pub const PNG_COLOR_TYPE_RGB: c_int = 2;
pub const PNG_COLOR_TYPE_PALETTE: c_int = 3;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: c_int = 4;
pub const PNG_COLOR_TYPE_RGB_ALPHA: c_int = 6;

pub const PNG_INTERLACE_NONE: c_int = 0;
pub const PNG_INTERLACE_ADAM7: c_int = 1;
pub const PNG_COMPRESSION_TYPE_BASE: c_int = 0;
pub const PNG_FILTER_TYPE_BASE: c_int = 0;
pub const PNG_INTRAPIXEL_DIFFERENCING: c_int = 64;

pub const PNG_NO_FILTERS: c_int = 0x00;
pub const PNG_FILTER_NONE: c_int = 0x08;
pub const PNG_FILTER_SUB: c_int = 0x10;
pub const PNG_FILTER_UP: c_int = 0x20;
pub const PNG_FILTER_AVG: c_int = 0x40;
pub const PNG_FILTER_PAETH: c_int = 0x80;
pub const PNG_ALL_FILTERS: c_int = 0xf8;

pub const PNG_INFO_gAMA: u32 = 0x0001;
pub const PNG_INFO_sBIT: u32 = 0x0002;
pub const PNG_INFO_cHRM: u32 = 0x0004;
pub const PNG_INFO_PLTE: u32 = 0x0008;
pub const PNG_INFO_tRNS: u32 = 0x0010;
pub const PNG_INFO_bKGD: u32 = 0x0020;
pub const PNG_INFO_hIST: u32 = 0x0040;
pub const PNG_INFO_pHYs: u32 = 0x0080;
pub const PNG_INFO_oFFs: u32 = 0x0100;
pub const PNG_INFO_tIME: u32 = 0x0200;
pub const PNG_INFO_pCAL: u32 = 0x0400;
pub const PNG_INFO_sRGB: u32 = 0x0800;
pub const PNG_INFO_iCCP: u32 = 0x1000;
pub const PNG_INFO_sPLT: u32 = 0x2000;
pub const PNG_INFO_sCAL: u32 = 0x4000;
pub const PNG_INFO_IDAT: u32 = 0x8000;
pub const PNG_INFO_eXIf: u32 = 0x10000;
pub const PNG_INFO_cICP: u32 = 0x20000;
pub const PNG_INFO_cLLI: u32 = 0x40000;
pub const PNG_INFO_mDCV: u32 = 0x80000;

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
pub const PNG_ALPHA_OPTIMIZED: c_int = 2;
pub const PNG_ALPHA_BROKEN: c_int = 3;

pub const PNG_ERROR_ACTION_NONE: c_int = 1;
pub const PNG_ERROR_ACTION_WARN: c_int = 2;
pub const PNG_ERROR_ACTION_ERROR: c_int = 3;
pub const PNG_RGB_TO_GRAY_DEFAULT: c_int = -1;

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

pub const PNG_FREE_ALL: u32 = 0xffff;
pub const PNG_FREE_TEXT: u32 = 0x4000;

pub const PNG_HAVE_IHDR: c_int = 0x01;
pub const PNG_HAVE_PLTE: c_int = 0x02;
pub const PNG_AFTER_IDAT: c_int = 0x08;

pub const PNG_FORMAT_FLAG_ALPHA: u32 = 0x01;
pub const PNG_FORMAT_FLAG_COLOR: u32 = 0x02;
pub const PNG_FORMAT_FLAG_LINEAR: u32 = 0x04;
pub const PNG_FORMAT_FLAG_COLORMAP: u32 = 0x08;
pub const PNG_FORMAT_FLAG_BGR: u32 = 0x10;
pub const PNG_FORMAT_FLAG_AFIRST: u32 = 0x20;

pub const PNG_IMAGE_VERSION: u32 = 1;

pub const PNG_MAXIMUM_INFLATE_WINDOW: c_int = 2;
pub const PNG_SKIP_sRGB_CHECK_PROFILE: c_int = 4;
pub const PNG_IGNORE_ADLER32: c_int = 8;
pub const PNG_OPTION_NEXT: c_int = 16;
pub const PNG_OPTION_UNSET: c_int = 0;
pub const PNG_OPTION_INVALID: c_int = 1;
pub const PNG_OPTION_OFF: c_int = 2;
pub const PNG_OPTION_ON: c_int = 3;

pub const PNG_SCALE_UNKNOWN: c_int = 0;
pub const PNG_SCALE_METER: c_int = 1;
pub const PNG_SCALE_RADIAN: c_int = 2;
pub const PNG_SCALE_LAST: c_int = 3;

pub const PNG_OFFSET_PIXEL: c_int = 0;
pub const PNG_OFFSET_MICROMETER: c_int = 1;
pub const PNG_OFFSET_LAST: c_int = 2;

pub const PNG_RESOLUTION_UNKNOWN: c_int = 0;
pub const PNG_RESOLUTION_METER: c_int = 1;
pub const PNG_RESOLUTION_LAST: c_int = 2;

pub const PNG_EQUATION_LINEAR: c_int = 0;
pub const PNG_EQUATION_BASE_E: c_int = 1;
pub const PNG_EQUATION_ARBITRARY: c_int = 2;
pub const PNG_EQUATION_HYPERBOLIC: c_int = 3;
pub const PNG_EQUATION_LAST: c_int = 4;

pub const PNG_sRGB_INTENT_PERCEPTUAL: c_int = 0;
pub const PNG_sRGB_INTENT_LAST: c_int = 4;

pub const VER: &[u8] = b"1.6.59\0";

pub fn ver() -> *const c_char {
    VER.as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// Common scaffolding for write / read sessions
// ---------------------------------------------------------------------------

/// Number of channels for a colour type, per the PNG spec (as libpng computes).
pub fn channels(color_type: c_int) -> usize {
    match color_type {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => 1,
    }
}

pub fn rowbytes(width: u32, bit_depth: c_int, color_type: c_int) -> usize {
    let pixel_bits = channels(color_type) * bit_depth as usize;
    ((width as usize * pixel_bits) + 7) / 8
}

/// The reference C `libpng.so` is linked against zlib only (see
/// `c_src/CMakeLists.txt`), yet it calls `floor`/`pow`/`modf`/`frexp` from libm.
/// Load libm (and libz) into the global namespace once so the lazy bindings in
/// the C `.so` resolve.  This affects only how the .so is loaded, never what it
/// computes.
fn preload_deps() {
    use std::sync::OnceLock;
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        for name in ["libm.so.6", "libz.so.1", "libc.so.6"] {
            unsafe {
                match libloading::os::unix::Library::open(
                    Some(name),
                    libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
                ) {
                    // Leak the handle: dropping it would dlclose and remove the
                    // library from the global namespace again.
                    Ok(h) => std::mem::forget(h),
                    Err(e) => panic!("preload {name}: {e}"),
                }
            }
        }
    });
}

/// A loaded implementation: public API + internal entry points from the same
/// shared object.
pub struct Lib {
    pub api: Api,
    pub pv: Priv,
    pub which: &'static str,
}

pub fn libs() -> (Lib, Lib) {
    preload_deps();
    (
        Lib {
            api: Api::open(C_SO, "C"),
            pv: Priv::open(C_SO, "C"),
            which: "C",
        },
        Lib {
            api: Api::open(RUST_SO, "RUST"),
            pv: Priv::open(RUST_SO, "RUST"),
            which: "RUST",
        },
    )
}

/// Log a `png_image` state AND record its `message` for ERRORS.md coverage.
///
/// The simplified API installs its own error handler (`png_safe_error`), so
/// messages raised inside `png_safe_execute` land in `png_image::message` and
/// never reach the application error callback.  Recording them here means those
/// rejection rows are counted as observed.
pub fn log_img(tag: &str, im: &PngImage) {
    let m = im.msg();
    if !m.is_empty() {
        record_message("image", &m);
    }
    log(format!(
        "{tag}: {}x{} fmt={:#x} flags={:#x} cmap={} woe={} msg={:?}",
        im.width, im.height, im.format, im.flags, im.colormap_entries, im.warning_or_error, m
    ));
}

/// Append bytes to the current run's recorded output (used when a test produces
/// a datastream through an API that does not go through our write callback).
pub fn out_extend(b: &[u8]) {
    ctx().out.extend_from_slice(b);
}

/// Append a line to the comparison log of the current run.
pub fn log(s: impl Into<String>) {
    ctx().log.push(s.into());
}

#[macro_export]
macro_rules! lg {
    ($($t:tt)*) => { $crate::common::log(format!($($t)*)) };
}

/// A write session: creates the struct, installs our IO + error callbacks and
/// arranges longjmp handling.  Returns the recorded Report.
pub fn write_session(l: &Lib, body: &mut dyn FnMut(&Lib, *mut c_void, *mut c_void)) -> Report {
    let api = &l.api;
    let mut ctxb = Box::new(Ctx::default());
    set_ctx(&mut *ctxb as *mut Ctx);
    unsafe {
        let png = (api.png_create_write_struct)(
            ver(),
            ptr::null_mut(),
            cb_error as *mut c_void,
            cb_warn as *mut c_void,
        );
        assert!(!png.is_null(), "{}: create_write_struct failed", l.which);
        let info = (api.png_create_info_struct)(png);
        assert!(!info.is_null(), "{}: create_info_struct failed", l.which);
        (api.png_set_write_fn)(
            png,
            ptr::null_mut(),
            cb_write as *mut c_void,
            cb_flush as *mut c_void,
        );
        protect(api, png, &mut || body(l, png, info));
        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_write_struct)(&mut pp, &mut ip);
    }
    let r = ctxb.digest();
    set_ctx(ptr::null_mut());
    r
}

/// A read session over an in-memory PNG datastream.
pub fn read_session(
    l: &Lib,
    input: Vec<u8>,
    body: &mut dyn FnMut(&Lib, *mut c_void, *mut c_void),
) -> Report {
    let api = &l.api;
    let mut ctxb = Box::new(Ctx::default());
    ctxb.input = input;
    set_ctx(&mut *ctxb as *mut Ctx);
    unsafe {
        let png = (api.png_create_read_struct)(
            ver(),
            ptr::null_mut(),
            cb_error as *mut c_void,
            cb_warn as *mut c_void,
        );
        assert!(!png.is_null(), "{}: create_read_struct failed", l.which);
        let info = (api.png_create_info_struct)(png);
        assert!(!info.is_null(), "{}: create_info_struct failed", l.which);
        (api.png_set_read_fn)(png, ptr::null_mut(), cb_read as *mut c_void);
        protect(api, png, &mut || body(l, png, info));
        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_read_struct)(&mut pp, &mut ip, ptr::null_mut());
    }
    let r = ctxb.digest();
    set_ctx(ptr::null_mut());
    r
}

/// A session with no PNG datastream at all: just a live png_struct so that
/// helpers which call png_error()/png_malloc() can be exercised.
pub fn bare_session(l: &Lib, body: &mut dyn FnMut(&Lib, *mut c_void, *mut c_void)) -> Report {
    write_session(l, body)
}

/// Deterministic palette of `n` entries.
pub fn make_palette(n: usize, seed: u64) -> Vec<PngColor> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| PngColor {
            red: rng.u8(),
            green: rng.u8(),
            blue: rng.u8(),
        })
        .collect()
}

/// Deterministic image rows.  `in_rowbytes` is the size of the buffer the
/// application hands to libpng, which is NOT always `png_get_rowbytes` (e.g.
/// with `png_set_packing` or `png_set_filler` the input row is wider).
pub fn make_rows(nrows: usize, in_rowbytes: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed);
    (0..nrows).map(|_| rng.bytes(in_rowbytes)).collect()
}

/// Drive a complete write: IHDR (+PLTE) -> `setup` -> png_write_info -> rows ->
/// png_write_end.  `setup` applies the configuration under test.
#[allow(clippy::too_many_arguments)]
pub fn write_full(
    l: &Lib,
    w: u32,
    h: u32,
    ct: c_int,
    bd: c_int,
    interlace: c_int,
    filter_method: c_int,
    palette: &[PngColor],
    in_rowbytes: usize,
    seed: u64,
    setup: &mut dyn FnMut(&Lib, *mut c_void, *mut c_void),
) -> Report {
    let rows = make_rows(h as usize, in_rowbytes, seed);
    write_session(l, &mut |l, png, info| unsafe {
        (l.api.png_set_IHDR)(
            png,
            info,
            w,
            h,
            bd,
            ct,
            interlace,
            PNG_COMPRESSION_TYPE_BASE,
            filter_method,
        );
        if !palette.is_empty() {
            (l.api.png_set_PLTE)(png, info, palette.as_ptr(), palette.len() as c_int);
        }
        setup(l, png, info);
        (l.api.png_write_info)(png, info);
        let passes = if interlace == PNG_INTERLACE_ADAM7 {
            (l.api.png_set_interlace_handling)(png)
        } else {
            1
        };
        log(format!("passes={passes} rowbytes={}", (l.api.png_get_rowbytes)(png, info)));
        for _ in 0..passes {
            for row in &rows {
                (l.api.png_write_row)(png, row.as_ptr());
            }
        }
        (l.api.png_write_end)(png, info);
    })
}

/// No-op setup for `write_full`.
pub fn no_setup(_l: &Lib, _png: *mut c_void, _info: *mut c_void) {}

/// Run the same closure against both libraries and assert byte-identical
/// results.  `label` identifies the CONFIGS.md / ERRORS.md row.
pub fn diff(label: &str, c: &Lib, r: &Lib, run: &mut dyn FnMut(&Lib) -> Report) {
    let rc = run(c);
    let rr = run(r);
    if rc != rr {
        let mut detail = String::new();
        if rc.out != rr.out {
            let n = rc
                .out
                .iter()
                .zip(rr.out.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(rc.out.len().min(rr.out.len()));
            let lo = n.saturating_sub(8);
            detail.push_str(&format!(
                "\n  first differing out byte at {n} (C len {} vs Rust len {})\n  C   : {:02x?}\n  RUST: {:02x?}",
                rc.out.len(),
                rr.out.len(),
                &rc.out[lo..(n + 24).min(rc.out.len())],
                &rr.out[lo..(n + 24).min(rr.out.len())],
            ));
        }
        if rc.log != rr.log {
            let n = rc
                .log
                .iter()
                .zip(rr.log.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(rc.log.len().min(rr.log.len()));
            detail.push_str(&format!(
                "\n  first differing log entry at {n}:\n  C   : {:?}\n  RUST: {:?}",
                rc.log.get(n),
                rr.log.get(n)
            ));
        }
        panic!(
            "DIVERGENCE [{label}]\n  C   : {}\n  RUST: {}{}",
            rc.brief(),
            rr.brief(),
            detail
        );
    }
}

/// Like `diff` but the closure needs no png_struct: it records into a Ctx we
/// install here.
pub fn diff_bare(label: &str, c: &Lib, r: &Lib, run: &mut dyn FnMut(&Lib)) {
    let mut go = |l: &Lib| -> Report {
        let mut ctxb = Box::new(Ctx::default());
        set_ctx(&mut *ctxb as *mut Ctx);
        run(l);
        let rep = ctxb.digest();
        set_ctx(ptr::null_mut());
        rep
    };
    let rc = go(c);
    let rr = go(r);
    if rc != rr {
        let n = rc
            .log
            .iter()
            .zip(rr.log.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(rc.log.len().min(rr.log.len()));
        panic!(
            "DIVERGENCE [{label}] at log entry {n}\n  C   : {:?}\n  RUST: {:?}\n  (C log len {}, Rust log len {})",
            rc.log.get(n),
            rr.log.get(n),
            rc.log.len(),
            rr.log.len()
        );
    }
}
