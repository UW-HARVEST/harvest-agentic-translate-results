//! Shared differential-test harness.
//!
//! Loads BOTH the reference C `libpng.so` and the translated Rust
//! `liblibpng.so` with `libloading` and calls every function through the
//! dynamic-symbol table, exactly like an external C consumer.  Nothing in the
//! Rust crate is ever called directly.
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libloading::{Library, Symbol};
use std::cell::RefCell;
use std::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void, CStr, CString};
use std::sync::OnceLock;

// ---------------------------------------------------------------- libpng types

pub type png_byte = u8;
pub type png_uint_16 = u16;
pub type png_int_32 = i32;
pub type png_uint_32 = u32;
pub type png_fixed_point = i32;
pub type png_structp = *mut c_void;
pub type png_infop = *mut c_void;
pub type png_voidp = *mut c_void;
pub type png_bytep = *mut png_byte;
pub type png_const_bytep = *const png_byte;
pub type png_charp = *mut c_char;
pub type png_const_charp = *const c_char;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct png_color {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct png_color_16 {
    pub index: png_byte,
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub gray: png_uint_16,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct png_color_8 {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
    pub gray: png_byte,
    pub alpha: png_byte,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
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
#[derive(Clone, Copy, Default, Debug, PartialEq)]
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

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct png_row_info {
    pub width: png_uint_32,
    pub rowbytes: usize,
    pub color_type: png_byte,
    pub bit_depth: png_byte,
    pub channels: png_byte,
    pub pixel_depth: png_byte,
}

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

impl png_image {
    pub fn msg(&self) -> String {
        let b: Vec<u8> = self
            .message
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        String::from_utf8_lossy(&b).into_owned()
    }
    /// everything except `opaque` (which is a heap pointer and always differs)
    pub fn cmp_key(&self) -> (u32, u32, u32, u32, u32, u32, u32, String) {
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

// -------------------------------------------------------------- png.h constants

pub const PNG_LIBPNG_VER_STRING: &[u8] = b"1.6.59\0";

pub const PNG_COLOR_TYPE_GRAY: c_int = 0;
pub const PNG_COLOR_TYPE_PALETTE: c_int = 3;
pub const PNG_COLOR_TYPE_RGB: c_int = 2;
pub const PNG_COLOR_TYPE_RGB_ALPHA: c_int = 6;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: c_int = 4;
pub const PNG_COLOR_MASK_PALETTE: c_int = 1;
pub const PNG_COLOR_MASK_COLOR: c_int = 2;
pub const PNG_COLOR_MASK_ALPHA: c_int = 4;

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
pub const PNG_ALL_FILTERS: c_int = 0xF8;

pub const PNG_FILTER_VALUE_NONE: c_int = 0;
pub const PNG_FILTER_VALUE_SUB: c_int = 1;
pub const PNG_FILTER_VALUE_UP: c_int = 2;
pub const PNG_FILTER_VALUE_AVG: c_int = 3;
pub const PNG_FILTER_VALUE_PAETH: c_int = 4;

pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fff_ffff;
pub const PNG_FP_1: png_fixed_point = 100000;

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

pub const PNG_ERROR_ACTION_NONE: c_int = 1;
pub const PNG_ERROR_ACTION_WARN: c_int = 2;
pub const PNG_ERROR_ACTION_ERROR: c_int = 3;

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

pub const PNG_HAVE_IHDR: c_int = 0x01;
pub const PNG_HAVE_PLTE: c_int = 0x02;
pub const PNG_AFTER_IDAT: c_int = 0x08;

// Values verified against c_src/include/png.h lines 1838-1853.
pub const PNG_FREE_HIST: c_int = 0x0008;
pub const PNG_FREE_ICCP: c_int = 0x0010;
pub const PNG_FREE_SPLT: c_int = 0x0020;
pub const PNG_FREE_ROWS: c_int = 0x0040;
pub const PNG_FREE_PCAL: c_int = 0x0080;
pub const PNG_FREE_SCAL: c_int = 0x0100;
pub const PNG_FREE_UNKN: c_int = 0x0200;
pub const PNG_FREE_PLTE: c_int = 0x1000;
pub const PNG_FREE_TRNS: c_int = 0x2000;
pub const PNG_FREE_TEXT: c_int = 0x4000;
pub const PNG_FREE_EXIF: c_int = 0x8000;
pub const PNG_FREE_ALL: c_int = 0xffff;
pub const PNG_FREE_MUL: c_int = 0x4220;

pub const PNG_DESTROY_WILL_FREE_DATA: c_int = 1;
pub const PNG_SET_WILL_FREE_DATA: c_int = 1;
pub const PNG_USER_WILL_FREE_DATA: c_int = 2;

pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;

pub const PNG_MAXIMUM_INFLATE_WINDOW: c_int = 2;
pub const PNG_SKIP_sRGB_CHECK_PROFILE: c_int = 4;
pub const PNG_IGNORE_ADLER32: c_int = 8;
pub const PNG_OPTION_UNSET: c_int = 0;
pub const PNG_OPTION_INVALID: c_int = 1;
pub const PNG_OPTION_OFF: c_int = 2;
pub const PNG_OPTION_ON: c_int = 3;

pub const PNG_FLAG_MNG_EMPTY_PLTE: c_int = 0x01;
pub const PNG_FLAG_MNG_FILTER_64: c_int = 0x04;
pub const PNG_ALL_MNG_FEATURES: c_int = 0x05;

pub const PNG_FORMAT_FLAG_ALPHA: png_uint_32 = 0x01;
pub const PNG_FORMAT_FLAG_COLOR: png_uint_32 = 0x02;
pub const PNG_FORMAT_FLAG_LINEAR: png_uint_32 = 0x04;
pub const PNG_FORMAT_FLAG_COLORMAP: png_uint_32 = 0x08;
pub const PNG_FORMAT_FLAG_BGR: png_uint_32 = 0x10;
pub const PNG_FORMAT_FLAG_AFIRST: png_uint_32 = 0x20;
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
pub const PNG_FORMAT_GRAY_COLORMAP: png_uint_32 = PNG_FORMAT_GRAY | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_IMAGE_VERSION: png_uint_32 = 1;

pub const PNG_NUMBER_FORMAT_u: c_int = 1;
pub const PNG_NUMBER_FORMAT_02u: c_int = 2;
pub const PNG_NUMBER_FORMAT_d: c_int = 1;
pub const PNG_NUMBER_FORMAT_02d: c_int = 2;
pub const PNG_NUMBER_FORMAT_x: c_int = 3;
pub const PNG_NUMBER_FORMAT_02x: c_int = 4;
pub const PNG_NUMBER_FORMAT_fixed: c_int = 5;
pub const PNG_NUMBER_BUFFER_SIZE: usize = 24;
pub const PNG_WARNING_PARAMETER_SIZE: usize = 32;
pub const PNG_WARNING_PARAMETER_COUNT: usize = 8;
pub const PNG_fp_MAX: usize = 32;

// ------------------------------------------------------------ library loading

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        // The reference C build (c_src/CMakeLists.txt) links zlib but NOT libm,
        // so `libpng.so` has undefined `floor`/`pow`/`frexp`/`modf`.  Pull libm
        // into the global scope first so those resolve; the Rust cdylib links
        // libm itself and is unaffected.
        unsafe {
            let _ = libloading::os::unix::Library::open(
                Some("libm.so.6"),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
            );
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let cpath = root.join("../c_src/build/libpng.so");
        let rpath = root.join("target/release/liblibpng.so");
        let rpath = if rpath.exists() {
            rpath
        } else {
            root.join("target/debug/liblibpng.so")
        };
        unsafe {
            Libs {
                c: Library::new(&cpath)
                    .unwrap_or_else(|e| panic!("load {}: {e}", cpath.display())),
                rs: Library::new(&rpath)
                    .unwrap_or_else(|e| panic!("load {}: {e}", rpath.display())),
            }
        }
    })
}

/// Fetch a symbol of type `T` from a library, panicking with a clear message.
pub fn sym<'a, T>(lib: &'a Library, name: &str) -> Symbol<'a, T> {
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe { lib.get::<T>(&n).unwrap_or_else(|e| panic!("symbol {name}: {e}")) }
}

/// Run `f` once against the C library and once against the Rust library.
pub fn both<R, F: Fn(&'static Library) -> R>(f: F) -> (R, R) {
    let l = libs();
    (f(&l.c), f(&l.rs))
}

// ------------------------------------------------------- error/warning capture
//
// libpng reports errors through the app's error callback which is expected NOT
// to return (the app longjmps).  We install a callback that records the exact
// message bytes and then unwinds via `panic!`, which is the translated crate's
// own stand-in for longjmp and which propagates correctly out of the C frames
// too (they are compiled with -fasynchronous-unwind-tables and have no
// cleanups).

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    Warn(Vec<u8>),
    Err(Vec<u8>),
}

impl std::fmt::Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Msg::Warn(m) => write!(f, "WARN({})", String::from_utf8_lossy(m)),
            Msg::Err(m) => write!(f, "ERR({})", String::from_utf8_lossy(m)),
        }
    }
}

thread_local! {
    static LOG: RefCell<Vec<Msg>> = const { RefCell::new(Vec::new()) };
}

pub fn log_clear() {
    LOG.with(|l| l.borrow_mut().clear());
}
pub fn log_take() -> Vec<Msg> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}
pub fn log_push(m: Msg) {
    LOG.with(|l| l.borrow_mut().push(m));
}

unsafe fn cstr_bytes(s: png_const_charp) -> Vec<u8> {
    if s.is_null() {
        b"<null>".to_vec()
    } else {
        CStr::from_ptr(s).to_bytes().to_vec()
    }
}

struct PngLongjmp;

pub unsafe extern "C-unwind" fn rec_error(_pp: png_structp, msg: png_const_charp) {
    log_push(Msg::Err(cstr_bytes(msg)));
    std::panic::panic_any(PngLongjmp);
}

pub unsafe extern "C-unwind" fn rec_warning(_pp: png_structp, msg: png_const_charp) {
    log_push(Msg::Warn(cstr_bytes(msg)));
}

/// Result of one differential operation.
pub struct Run<T> {
    pub out: Option<T>, // None == the library errored out (longjmp/panic)
    pub log: Vec<Msg>,
}

impl<T: PartialEq + std::fmt::Debug> Run<T> {
    pub fn assert_eq(&self, other: &Run<T>, what: &str) {
        assert_eq!(
            self.log.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
            other.log.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
            "{what}: message log differs (C first)"
        );
        assert_eq!(
            self.out.is_some(),
            other.out.is_some(),
            "{what}: one library errored and the other did not (C errored={})",
            self.out.is_none()
        );
        assert_eq!(self.out, other.out, "{what}: output differs");
    }
}

/// Execute `f`, capturing an error-callback unwind and the message log.
pub fn capture<T, F: FnOnce() -> T>(f: F) -> Run<T> {
    log_clear();
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok();
    std::panic::set_hook(prev);
    Run { out, log: log_take() }
}

/// Same as [`capture`] but re-raises a panic that is *not* a libpng error
/// (i.e. a genuine bug such as an arithmetic overflow or index panic).
pub fn capture_strict<T, F: FnOnce() -> T>(f: F) -> Run<T> {
    log_clear();
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::panic::set_hook(prev);
    let out = match r {
        Ok(v) => Some(v),
        Err(e) => {
            if e.downcast_ref::<PngLongjmp>().is_some() {
                None
            } else {
                std::panic::resume_unwind(e);
            }
        }
    };
    Run { out, log: log_take() }
}

// --------------------------------------------------------------- I/O plumbing

/// Sink for `png_set_write_fn`: collects all bytes libpng writes.
pub struct WriteSink {
    pub data: Vec<u8>,
    pub flushes: usize,
}

// The sink/source is reached through a thread-local rather than through
// `png_get_io_ptr` so that the harness never has to interpret the internals of
// either library's `png_struct`.  The io_ptr we hand to `png_set_write_fn` /
// `png_set_read_fn` is still non-NULL and is checked separately by the
// `png_get_io_ptr` differential test.
thread_local! {
    static SINK: RefCell<WriteSink> = const { RefCell::new(WriteSink { data: Vec::new(), flushes: 0 }) };
    static SRC: RefCell<ReadSrc> = const { RefCell::new(ReadSrc { data: Vec::new(), pos: 0 }) };
}

pub fn sink_reset() {
    SINK.with(|s| {
        let mut s = s.borrow_mut();
        s.data.clear();
        s.flushes = 0;
    });
}
pub fn sink_take() -> (Vec<u8>, usize) {
    SINK.with(|s| {
        let mut s = s.borrow_mut();
        (std::mem::take(&mut s.data), std::mem::replace(&mut s.flushes, 0))
    })
}
pub fn sink_len() -> usize {
    SINK.with(|s| s.borrow().data.len())
}
pub fn src_set(data: &[u8]) {
    SRC.with(|s| {
        let mut s = s.borrow_mut();
        s.data = data.to_vec();
        s.pos = 0;
    });
}
pub fn src_pos() -> usize {
    SRC.with(|s| s.borrow().pos)
}

pub unsafe extern "C-unwind" fn write_cb(_pp: png_structp, data: png_bytep, len: usize) {
    let slice = std::slice::from_raw_parts(data, len);
    SINK.with(|s| s.borrow_mut().data.extend_from_slice(slice));
}

pub unsafe extern "C-unwind" fn flush_cb(_pp: png_structp) {
    SINK.with(|s| s.borrow_mut().flushes += 1);
}

/// Source for `png_set_read_fn`.
pub struct ReadSrc {
    pub data: Vec<u8>,
    pub pos: usize,
}

pub unsafe extern "C-unwind" fn read_cb(_pp: png_structp, out: png_bytep, len: usize) {
    let n = SRC.with(|s| {
        let mut s = s.borrow_mut();
        let avail = s.data.len().saturating_sub(s.pos);
        let n = core::cmp::min(len, avail);
        if n > 0 {
            std::ptr::copy_nonoverlapping(s.data.as_ptr().add(s.pos), out, n);
            s.pos += n;
        }
        n
    });
    if n < len {
        // A real app's read callback must not return short; it calls png_error,
        // which longjmps.  Do the equivalent, identically for both libraries.
        log_push(Msg::Err(b"harness: short read".to_vec()));
        std::panic::panic_any(PngLongjmp);
    }
}

// ------------------------------------------------------------ tiny RNG (xorshift)

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
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
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.below((hi - lo + 1) as u32) as i32)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    pub fn f64_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub type c_charp = *mut c_char;
pub type PngErrorFn = unsafe extern "C-unwind" fn(png_structp, png_const_charp);
pub type PngRwFn = unsafe extern "C-unwind" fn(png_structp, png_bytep, usize);
pub type PngFlushFn = unsafe extern "C-unwind" fn(png_structp);
pub type c_size = usize;
pub type C_int = c_int;
pub type C_uint = c_uint;
pub type C_long = c_long;
pub type C_ulong = c_ulong;
pub type C_double = c_double;

// ------------------------------------------------------------- API bindings

#[macro_export]
macro_rules! decl_api {
    ($( fn $name:ident ( $($p:ident : $t:ty),* $(,)? ) $(-> $r:ty)? ; )*) => {
        $(
            #[allow(clippy::too_many_arguments)]
            pub unsafe fn $name(l: &::libloading::Library, $($p: $t),*) $(-> $r)? {
                let f: ::libloading::Symbol<unsafe extern "C-unwind" fn($($t),*) $(-> $r)?> =
                    $crate::common::sym(l, stringify!($name));
                f($($p),*)
            }
        )*
    };
}

pub mod api;
