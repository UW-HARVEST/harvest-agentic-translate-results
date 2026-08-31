//! Shared harness: loads the reference C `libpng.so` and the translated Rust
//! `liblibpng.so` side by side through `libloading` and calls both purely
//! through their exported symbols.
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_long, c_uint, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::OnceLock;

/* ------------------------------------------------------------------ types */

pub type png_byte = u8;
pub type png_uint_16 = u16;
pub type png_int_32 = i32;
pub type png_uint_32 = u32;
pub type png_fixed_point = i32;
pub type png_alloc_size_t = usize;
pub type png_bytep = *mut u8;
pub type png_const_bytep = *const u8;
pub type png_voidp = *mut c_void;
pub type png_structp = *mut c_void;
pub type png_infop = *mut c_void;
pub type png_charp = *mut c_char;
pub type png_const_charp = *const c_char;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_color_16 {
    pub index: u8,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub gray: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_color_8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub gray: u8,
    pub alpha: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_sPLT_entry {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub alpha: u16,
    pub frequency: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct png_sPLT_t {
    pub name: *mut c_char,
    pub depth: u8,
    pub entries: *mut png_sPLT_entry,
    pub nentries: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct png_text {
    pub compression: c_int,
    pub key: *mut c_char,
    pub text: *mut c_char,
    pub text_length: usize,
    pub itxt_length: usize,
    pub lang: *mut c_char,
    pub lang_key: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_time {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct png_unknown_chunk {
    pub name: [u8; 5],
    pub data: *mut u8,
    pub size: usize,
    pub location: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_row_info {
    pub width: u32,
    pub rowbytes: usize,
    pub color_type: u8,
    pub bit_depth: u8,
    pub channels: u8,
    pub pixel_depth: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_control_opaque(pub *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_image {
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

pub const PNG_IMAGE_VERSION: u32 = 1;

/* ------------------------------------------------------------ constants */

pub const PNG_COLOR_MASK_PALETTE: u8 = 1;
pub const PNG_COLOR_MASK_COLOR: u8 = 2;
pub const PNG_COLOR_MASK_ALPHA: u8 = 4;

pub const PNG_COLOR_TYPE_GRAY: u8 = 0;
pub const PNG_COLOR_TYPE_PALETTE: u8 = 3;
pub const PNG_COLOR_TYPE_RGB: u8 = 2;
pub const PNG_COLOR_TYPE_RGB_ALPHA: u8 = 6;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: u8 = 4;

pub const PNG_INTERLACE_NONE: c_int = 0;
pub const PNG_INTERLACE_ADAM7: c_int = 1;
pub const PNG_COMPRESSION_TYPE_BASE: c_int = 0;
pub const PNG_FILTER_TYPE_BASE: c_int = 0;

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

pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;

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
pub const PNG_TRANSFORM_GRAY_TO_RGB: c_int = 0x2000;

pub const PNG_FILLER_BEFORE: c_int = 0;
pub const PNG_FILLER_AFTER: c_int = 1;

pub const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;
pub const PNG_BACKGROUND_GAMMA_FILE: c_int = 2;
pub const PNG_BACKGROUND_GAMMA_UNIQUE: c_int = 3;

pub const PNG_ALPHA_PNG: c_int = 0;
pub const PNG_ALPHA_STANDARD: c_int = 1;
pub const PNG_ALPHA_BROKEN: c_int = 2;
pub const PNG_ALPHA_OPTIMIZED: c_int = 3;

pub const PNG_FILTER_NONE: c_int = 0x08;
pub const PNG_FILTER_SUB: c_int = 0x10;
pub const PNG_FILTER_UP: c_int = 0x20;
pub const PNG_FILTER_AVG: c_int = 0x40;
pub const PNG_FILTER_PAETH: c_int = 0x80;
pub const PNG_ALL_FILTERS: c_int = 0xF8;
pub const PNG_NO_FILTERS: c_int = 0x00;

pub const PNG_FREE_ALL: c_int = 0x7fff;
pub const PNG_FREE_TEXT: c_int = 0x0200;

pub const PNG_HANDLE_CHUNK_AS_DEFAULT: c_int = 0;
pub const PNG_HANDLE_CHUNK_NEVER: c_int = 1;
pub const PNG_HANDLE_CHUNK_IF_SAFE: c_int = 2;
pub const PNG_HANDLE_CHUNK_ALWAYS: c_int = 3;

pub const PNG_CRC_DEFAULT: c_int = 0;
pub const PNG_CRC_ERROR_QUIT: c_int = 1;
pub const PNG_CRC_WARN_DISCARD: c_int = 2;
pub const PNG_CRC_WARN_USE: c_int = 3;
pub const PNG_CRC_QUIET_USE: c_int = 4;
pub const PNG_CRC_NO_CHANGE: c_int = 5;

pub const PNG_FORMAT_FLAG_ALPHA: u32 = 0x01;
pub const PNG_FORMAT_FLAG_COLOR: u32 = 0x02;
pub const PNG_FORMAT_FLAG_LINEAR: u32 = 0x04;
pub const PNG_FORMAT_FLAG_COLORMAP: u32 = 0x08;
pub const PNG_FORMAT_FLAG_BGR: u32 = 0x10;
pub const PNG_FORMAT_FLAG_AFIRST: u32 = 0x20;

pub const PNG_FORMAT_GRAY: u32 = 0;
pub const PNG_FORMAT_GA: u32 = PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_AG: u32 = PNG_FORMAT_GA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_RGB: u32 = PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_BGR: u32 = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_BGR;
pub const PNG_FORMAT_RGBA: u32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ARGB: u32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_BGRA: u32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ABGR: u32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_LINEAR_Y: u32 = PNG_FORMAT_FLAG_LINEAR;
pub const PNG_FORMAT_LINEAR_Y_ALPHA: u32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_LINEAR_RGB: u32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_LINEAR_RGB_ALPHA: u32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;

/// PNG_IMAGE_PIXEL_CHANNELS
pub fn image_pixel_channels(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
    }
}

/// PNG_IMAGE_PIXEL_COMPONENT_SIZE
pub fn image_component_size(fmt: u32) -> u32 {
    if fmt & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        ((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1
    }
}

/// PNG_IMAGE_SIZE
pub fn image_size(img: &png_image) -> usize {
    (image_component_size(img.format) * img.height * image_pixel_channels(img.format) * img.width)
        as usize
}

/// PNG_IMAGE_COLORMAP_SIZE
pub fn image_colormap_size(img: &png_image) -> usize {
    let sample_channels = (img.format & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1;
    let sample_component = ((img.format & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1;
    (sample_channels * sample_component * img.colormap_entries.max(256)) as usize
}

pub const PNG_LIBPNG_VER_STRING: &str = "1.6.59.git";

/* ------------------------------------------------------------- libraries */

pub struct Lib {
    pub lib: Library,
    pub name: &'static str,
}

impl Lib {
    pub fn sym<T>(&self, name: &str) -> Symbol<'_, T> {
        unsafe {
            self.lib
                .get(name.as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol {}: {}", self.name, name, e))
        }
    }
    pub fn has(&self, name: &str) -> bool {
        unsafe { self.lib.get::<*const c_void>(name.as_bytes()).is_ok() }
    }
}

pub struct Libs {
    pub c: Lib,
    pub r: Lib,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libpng.so")
}

pub fn rust_so_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join("liblibpng.so");
    if p.exists() {
        return p;
    }
    // fall back to the other profile dir
    for prof in ["release", "debug"] {
        let q = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(prof)
            .join("liblibpng.so");
        if q.exists() {
            return q;
        }
    }
    p
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        // The reference C build links libm implicitly through the compiler
        // driver; when the .so is dlopen'd from a Rust harness the math symbols
        // must already be globally visible.
        unsafe {
            use libloading::os::unix as u;
            for soname in ["libm.so.6", "libz.so.1"] {
                if let Ok(l) = u::Library::open(Some(soname), u::RTLD_NOW | u::RTLD_GLOBAL) {
                    std::mem::forget(l);
                }
            }
        }
        let c = c_so_path();
        let r = rust_so_path();
        let cl = unsafe { Library::new(&c) }
            .unwrap_or_else(|e| panic!("cannot load {}: {}", c.display(), e));
        let rl = unsafe { Library::new(&r) }
            .unwrap_or_else(|e| panic!("cannot load {}: {}", r.display(), e));
        Libs {
            c: Lib { lib: cl, name: "C" },
            r: Lib { lib: rl, name: "Rust" },
        }
    })
}

/* --------------------------------------------------------- error capture */

/// Per-thread capture of the messages libpng hands to the error/warning
/// callbacks, plus the payload the error callback unwinds with.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

thread_local! {
    static DIAG: std::cell::RefCell<Diag> = std::cell::RefCell::new(Diag::default());
}

pub fn diag_reset() {
    DIAG.with(|d| *d.borrow_mut() = Diag::default());
}

pub fn diag_take() -> Diag {
    DIAG.with(|d| std::mem::take(&mut *d.borrow_mut()))
}

pub fn diag_push_warning(s: String) {
    DIAG.with(|d| d.borrow_mut().warnings.push(s));
}

pub fn diag_set_error(s: String) {
    DIAG.with(|d| d.borrow_mut().error = Some(s));
}

struct PngUnwind;

/// Install, once, a panic hook that stays silent for the synthetic unwind used
/// to escape `png_error` but still reports genuine test failures.
fn install_hook() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if info.payload().downcast_ref::<PngUnwind>().is_none()
                || std::env::var_os("PNG_TEST_VERBOSE").is_some()
            {
                prev(info);
            }
        }));
    });
}

pub unsafe extern "C-unwind" fn error_cb(_p: png_structp, msg: png_const_charp) {
    let m = if msg.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(msg) }.to_string_lossy().into_owned()
    };
    diag_set_error(m.clone());
    if std::env::var_os("PNG_TEST_VERBOSE").is_some() {
        eprintln!("png_error: {m}");
    }
    std::panic::resume_unwind(Box::new(PngUnwind));
}

pub unsafe extern "C-unwind" fn warning_cb(_p: png_structp, msg: png_const_charp) {
    let m = if msg.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(msg) }.to_string_lossy().into_owned()
    };
    diag_push_warning(m);
}

/// Run `f`, catching a libpng error raised through `error_cb`.
pub fn guard<R>(f: impl FnOnce() -> R) -> Result<R, ()> {
    install_hook();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => Ok(v),
        Err(e) => {
            if e.downcast_ref::<PngUnwind>().is_some() {
                Err(())
            } else {
                std::panic::resume_unwind(e)
            }
        }
    }
}

/* ------------------------------------------------------------ signatures */

pub type FnCreateRead = unsafe extern "C-unwind" fn(
    png_const_charp,
    png_voidp,
    Option<unsafe extern "C-unwind" fn(png_structp, png_const_charp)>,
    Option<unsafe extern "C-unwind" fn(png_structp, png_const_charp)>,
) -> png_structp;
pub type FnCreateInfo = unsafe extern "C-unwind" fn(png_structp) -> png_infop;
pub type FnDestroyRead = unsafe extern "C-unwind" fn(*mut png_structp, *mut png_infop, *mut png_infop);
pub type FnDestroyWrite = unsafe extern "C-unwind" fn(*mut png_structp, *mut png_infop);
pub type FnSetWriteFn = unsafe extern "C-unwind" fn(
    png_structp,
    png_voidp,
    Option<unsafe extern "C-unwind" fn(png_structp, png_bytep, usize)>,
    Option<unsafe extern "C-unwind" fn(png_structp)>,
);
pub type FnSetReadFn = unsafe extern "C-unwind" fn(
    png_structp,
    png_voidp,
    Option<unsafe extern "C-unwind" fn(png_structp, png_bytep, usize)>,
);
pub type FnGetIoPtr = unsafe extern "C-unwind" fn(png_structp) -> png_voidp;

/* ------------------------------------------------- memory writer / reader */

/// Sink handed to libpng as the io_ptr for writing.
#[repr(C)]
pub struct MemWriter {
    pub buf: Vec<u8>,
    pub flushes: u32,
}

pub unsafe extern "C-unwind" fn mem_write(p: png_structp, data: png_bytep, len: usize) {
    let libs = libs();
    // io_ptr retrieval must go through whichever library owns `p`; both
    // implementations lay out png_struct differently in principle, so ask the
    // library that we recorded in the sink.  Instead we stash the sink pointer
    // in a thread-local keyed by png_ptr.
    let sink = SINKS.with(|s| {
        s.borrow()
            .iter()
            .find(|(k, _)| *k == p as usize)
            .map(|(_, v)| *v)
    });
    let sink = sink.expect("unknown png_ptr in mem_write");
    let w = unsafe { &mut *(sink as *mut MemWriter) };
    if !data.is_null() && len > 0 {
        w.buf.extend_from_slice(unsafe { std::slice::from_raw_parts(data, len) });
    }
    let _ = libs;
}

pub unsafe extern "C-unwind" fn mem_flush(p: png_structp) {
    let sink = SINKS.with(|s| {
        s.borrow()
            .iter()
            .find(|(k, _)| *k == p as usize)
            .map(|(_, v)| *v)
    });
    if let Some(sink) = sink {
        let w = unsafe { &mut *(sink as *mut MemWriter) };
        w.flushes += 1;
    }
}

pub struct MemReader {
    pub data: Vec<u8>,
    pub pos: usize,
}

pub unsafe extern "C-unwind" fn mem_read(p: png_structp, out: png_bytep, len: usize) {
    let sink = SINKS.with(|s| {
        s.borrow()
            .iter()
            .find(|(k, _)| *k == p as usize)
            .map(|(_, v)| *v)
    });
    let sink = sink.expect("unknown png_ptr in mem_read");
    let r = unsafe { &mut *(sink as *mut MemReader) };
    let avail = r.data.len().saturating_sub(r.pos);
    let n = len.min(avail);
    if n > 0 {
        unsafe { std::ptr::copy_nonoverlapping(r.data.as_ptr().add(r.pos), out, n) };
    }
    r.pos += n;
    if n < len {
        // emulate a short read: libpng treats this as an error via png_error
        diag_set_error("Read Error".to_string());
        std::panic::resume_unwind(Box::new(PngUnwind));
    }
}

thread_local! {
    pub static SINKS: std::cell::RefCell<Vec<(usize, *mut c_void)>> =
        std::cell::RefCell::new(Vec::new());
}

pub fn sink_register(p: png_structp, sink: *mut c_void) {
    SINKS.with(|s| s.borrow_mut().push((p as usize, sink)));
}

pub fn sink_clear() {
    SINKS.with(|s| s.borrow_mut().clear());
}

/* ---------------------------------------------------------------- helpers */

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

pub fn cstr_of(p: png_const_charp) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect::<Vec<_>>().join("")
}

/* ==================================================================== *
 *  High level drivers: run the same call sequence against one library
 *  and capture everything observable (byte stream, diagnostics).
 * ==================================================================== */

pub struct Ctx<'a> {
    pub lib: &'a Lib,
    pub png: png_structp,
    pub info: png_infop,
}

impl<'a> Ctx<'a> {
    pub fn sym<T>(&self, name: &str) -> Symbol<'a, T> {
        unsafe {
            self.lib
                .lib
                .get(name.as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol {}: {}", self.lib.name, name, e))
        }
    }
    /// void f(png_ptr)
    pub fn call1(&self, name: &str) {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp)> = self.sym(name);
        unsafe { f(self.png) }
    }
    /// void f(png_ptr, info_ptr)
    pub fn call2(&self, name: &str) {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop)> = self.sym(name);
        unsafe { f(self.png, self.info) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOutcome {
    pub bytes: Vec<u8>,
    pub diag: Diag,
    pub errored: bool,
    pub flushes: u32,
    pub notes: Vec<String>,
}

/// Create a write struct in `lib`, run `body`, then destroy it.
pub fn write_with(lib: &Lib, body: impl FnOnce(&Ctx, &mut Vec<String>)) -> WriteOutcome {
    diag_reset();
    let mut sink = Box::new(MemWriter { buf: Vec::new(), flushes: 0 });
    let create: Symbol<FnCreateRead> = lib.sym("png_create_write_struct");
    let ver = cs(PNG_LIBPNG_VER_STRING);
    let png = unsafe {
        create(
            ver.as_ptr(),
            std::ptr::null_mut(),
            Some(error_cb),
            Some(warning_cb),
        )
    };
    assert!(!png.is_null(), "{}: png_create_write_struct failed", lib.name);
    let create_info: Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
    let info = unsafe { create_info(png) };
    assert!(!info.is_null(), "{}: png_create_info_struct failed", lib.name);

    sink_register(png, (&mut *sink) as *mut MemWriter as *mut c_void);
    let set_write: Symbol<FnSetWriteFn> = lib.sym("png_set_write_fn");
    unsafe {
        set_write(
            png,
            (&mut *sink) as *mut MemWriter as *mut c_void,
            Some(mem_write),
            Some(mem_flush),
        )
    };

    let ctx = Ctx { lib, png, info };
    let mut notes = Vec::new();
    let res = guard(|| body(&ctx, &mut notes));

    let destroy: Symbol<FnDestroyWrite> = lib.sym("png_destroy_write_struct");
    let mut p = png;
    let mut i = info;
    let _ = guard(|| unsafe { destroy(&mut p, &mut i) });
    sink_clear();

    WriteOutcome {
        bytes: std::mem::take(&mut sink.buf),
        diag: diag_take(),
        errored: res.is_err(),
        flushes: sink.flushes,
        notes,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReadOutcome {
    pub rows: Vec<Vec<u8>>,
    pub notes: Vec<String>,
    pub diag: Diag,
    pub errored: bool,
}

/// Create a read struct in `lib` fed from `data`, run `body`, then destroy it.
pub fn read_with(
    lib: &Lib,
    data: &[u8],
    body: impl FnOnce(&Ctx, &mut ReadOutcome),
) -> ReadOutcome {
    diag_reset();
    let mut src = Box::new(MemReader { data: data.to_vec(), pos: 0 });
    let create: Symbol<FnCreateRead> = lib.sym("png_create_read_struct");
    let ver = cs(PNG_LIBPNG_VER_STRING);
    let png = unsafe {
        create(
            ver.as_ptr(),
            std::ptr::null_mut(),
            Some(error_cb),
            Some(warning_cb),
        )
    };
    assert!(!png.is_null(), "{}: png_create_read_struct failed", lib.name);
    let create_info: Symbol<FnCreateInfo> = lib.sym("png_create_info_struct");
    let info = unsafe { create_info(png) };
    let end_info = unsafe { create_info(png) };

    sink_register(png, (&mut *src) as *mut MemReader as *mut c_void);
    let set_read: Symbol<FnSetReadFn> = lib.sym("png_set_read_fn");
    unsafe {
        set_read(
            png,
            (&mut *src) as *mut MemReader as *mut c_void,
            Some(mem_read),
        )
    };

    let mut out = ReadOutcome::default();
    let ctx = Ctx { lib, png, info };
    let res = guard(|| body(&ctx, &mut out));

    let destroy: Symbol<FnDestroyRead> = lib.sym("png_destroy_read_struct");
    let mut p = png;
    let mut i = info;
    let mut e = end_info;
    let _ = guard(|| unsafe { destroy(&mut p, &mut i, &mut e) });
    sink_clear();

    out.diag = diag_take();
    out.errored = res.is_err();
    out
}

/// Collect the value of every `png_get_*` accessor into a comparable form.
pub fn snapshot_info(ctx: &Ctx) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let png = ctx.png;
    let info = ctx.info;

    macro_rules! g {
        ($t:ty, $n:expr) => {{
            let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> $t> = ctx.sym($n);
            v.push(format!("{}={:?}", $n, unsafe { f(png, info) }));
        }};
    }

    g!(u32, "png_get_image_width");
    g!(u32, "png_get_image_height");
    g!(u8, "png_get_bit_depth");
    g!(u8, "png_get_color_type");
    g!(u8, "png_get_filter_type");
    g!(u8, "png_get_interlace_type");
    g!(u8, "png_get_compression_type");
    g!(u8, "png_get_channels");
    g!(usize, "png_get_rowbytes");
    g!(u32, "png_get_x_pixels_per_meter");
    g!(u32, "png_get_y_pixels_per_meter");
    g!(u32, "png_get_pixels_per_meter");
    g!(u32, "png_get_x_pixels_per_inch");
    g!(u32, "png_get_y_pixels_per_inch");
    g!(u32, "png_get_pixels_per_inch");
    g!(i32, "png_get_x_offset_pixels");
    g!(i32, "png_get_y_offset_pixels");
    g!(i32, "png_get_x_offset_microns");
    g!(i32, "png_get_y_offset_microns");
    g!(f32, "png_get_x_offset_inches");
    g!(f32, "png_get_y_offset_inches");
    g!(f32, "png_get_pixel_aspect_ratio");
    g!(i32, "png_get_x_offset_inches_fixed");
    g!(i32, "png_get_y_offset_inches_fixed");
    g!(i32, "png_get_pixel_aspect_ratio_fixed");

    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, u32) -> u32> =
            ctx.sym("png_get_valid");
        for (name, flag) in [
            ("gAMA", PNG_INFO_gAMA),
            ("sBIT", PNG_INFO_sBIT),
            ("cHRM", PNG_INFO_cHRM),
            ("PLTE", PNG_INFO_PLTE),
            ("tRNS", PNG_INFO_tRNS),
            ("bKGD", PNG_INFO_bKGD),
            ("hIST", PNG_INFO_hIST),
            ("pHYs", PNG_INFO_pHYs),
            ("oFFs", PNG_INFO_oFFs),
            ("tIME", PNG_INFO_tIME),
            ("pCAL", PNG_INFO_pCAL),
            ("sRGB", PNG_INFO_sRGB),
            ("iCCP", PNG_INFO_iCCP),
            ("sPLT", PNG_INFO_sPLT),
            ("sCAL", PNG_INFO_sCAL),
            ("IDAT", PNG_INFO_IDAT),
            ("eXIf", PNG_INFO_eXIf),
            ("cICP", PNG_INFO_cICP),
            ("cLLI", PNG_INFO_cLLI),
            ("mDCV", PNG_INFO_mDCV),
        ] {
            v.push(format!("valid[{}]={}", name, unsafe { f(png, info, flag) }));
        }
    }
    {
        let f: Symbol<
            unsafe extern "C-unwind" fn(
                png_structp,
                png_infop,
                *mut u32,
                *mut u32,
                *mut c_int,
                *mut c_int,
                *mut c_int,
                *mut c_int,
                *mut c_int,
            ) -> u32,
        > = ctx.sym("png_get_IHDR");
        let (mut w, mut h) = (0u32, 0u32);
        let (mut bd, mut ct, mut il, mut cm, mut fm) = (-1, -1, -1, -1, -1);
        let r = unsafe {
            f(png, info, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm)
        };
        v.push(format!("IHDR r={r} {w}x{h} bd={bd} ct={ct} il={il} cm={cm} fm={fm}"));
    }
    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut i32) -> u32> =
            ctx.sym("png_get_gAMA_fixed");
        let mut g = -1i32;
        v.push(format!("gAMA_fixed r={} g={}", unsafe { f(png, info, &mut g) }, g));
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut f64) -> u32> =
            ctx.sym("png_get_gAMA");
        let mut g = -1f64;
        v.push(format!("gAMA r={} g={:?}", unsafe { f(png, info, &mut g) }, g));
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut c_int) -> u32> =
            ctx.sym("png_get_sRGB");
        let mut s = -12345;
        v.push(format!("sRGB r={} i={}", unsafe { f(png, info, &mut s) }, s));
    }
    {
        type F8i = unsafe extern "C-unwind" fn(
            png_structp,
            png_infop,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
        ) -> u32;
        let f: Symbol<F8i> = ctx.sym("png_get_cHRM_fixed");
        let mut a = [-1i32; 8];
        let r = unsafe {
            f(png, info, &mut a[0], &mut a[1], &mut a[2], &mut a[3], &mut a[4], &mut a[5],
              &mut a[6], &mut a[7])
        };
        v.push(format!("cHRM_fixed r={r} {a:?}"));

        type F8d = unsafe extern "C-unwind" fn(
            png_structp,
            png_infop,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
        ) -> u32;
        let f: Symbol<F8d> = ctx.sym("png_get_cHRM");
        let mut b = [-1f64; 8];
        let r = unsafe {
            f(png, info, &mut b[0], &mut b[1], &mut b[2], &mut b[3], &mut b[4], &mut b[5],
              &mut b[6], &mut b[7])
        };
        v.push(format!("cHRM r={r} {b:?}"));

        type F9i = unsafe extern "C-unwind" fn(
            png_structp,
            png_infop,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
            *mut i32,
        ) -> u32;
        let f: Symbol<F9i> = ctx.sym("png_get_cHRM_XYZ_fixed");
        let mut c = [-1i32; 9];
        let r = unsafe {
            f(png, info, &mut c[0], &mut c[1], &mut c[2], &mut c[3], &mut c[4], &mut c[5],
              &mut c[6], &mut c[7], &mut c[8])
        };
        v.push(format!("cHRM_XYZ_fixed r={r} {c:?}"));

        type F9d = unsafe extern "C-unwind" fn(
            png_structp,
            png_infop,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
        ) -> u32;
        let f: Symbol<F9d> = ctx.sym("png_get_cHRM_XYZ");
        let mut d = [-1f64; 9];
        let r = unsafe {
            f(png, info, &mut d[0], &mut d[1], &mut d[2], &mut d[3], &mut d[4], &mut d[5],
              &mut d[6], &mut d[7], &mut d[8])
        };
        v.push(format!("cHRM_XYZ r={r} {d:?}"));
    }
    {
        let f: Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut u8, *mut u8, *mut u8, *mut u8) -> u32,
        > = ctx.sym("png_get_cICP");
        let mut a = [0xffu8; 4];
        let r = unsafe { f(png, info, &mut a[0], &mut a[1], &mut a[2], &mut a[3]) };
        v.push(format!("cICP r={r} {a:?}"));
    }
    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut u32, *mut u32) -> u32> =
            ctx.sym("png_get_cLLI_fixed");
        let (mut a, mut b) = (0xffffu32, 0xffffu32);
        let r = unsafe { f(png, info, &mut a, &mut b) };
        v.push(format!("cLLI_fixed r={r} {a} {b}"));
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut f64, *mut f64) -> u32> =
            ctx.sym("png_get_cLLI");
        let (mut a, mut b) = (-1f64, -1f64);
        let r = unsafe { f(png, info, &mut a, &mut b) };
        v.push(format!("cLLI r={r} {a:?} {b:?}"));
    }
    {
        type F10 = unsafe extern "C-unwind" fn(
            png_structp,
            png_infop,
            *mut u32,
            *mut u32,
            *mut u32,
            *mut u32,
            *mut u32,
            *mut u32,
            *mut u32,
            *mut u32,
            *mut u32,
            *mut u32,
        ) -> u32;
        let f: Symbol<F10> = ctx.sym("png_get_mDCV_fixed");
        let mut a = [0xffffu32; 10];
        let r = unsafe {
            f(png, info, &mut a[0], &mut a[1], &mut a[2], &mut a[3], &mut a[4], &mut a[5],
              &mut a[6], &mut a[7], &mut a[8], &mut a[9])
        };
        v.push(format!("mDCV_fixed r={r} {a:?}"));

        type F10d = unsafe extern "C-unwind" fn(
            png_structp,
            png_infop,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
            *mut f64,
        ) -> u32;
        let f: Symbol<F10d> = ctx.sym("png_get_mDCV");
        let mut b = [-1f64; 10];
        let r = unsafe {
            f(png, info, &mut b[0], &mut b[1], &mut b[2], &mut b[3], &mut b[4], &mut b[5],
              &mut b[6], &mut b[7], &mut b[8], &mut b[9])
        };
        v.push(format!("mDCV r={r} {b:?}"));
    }
    {
        let f: Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut png_color, *mut c_int) -> u32,
        > = ctx.sym("png_get_PLTE");
        let mut p: *mut png_color = std::ptr::null_mut();
        let mut n: c_int = -1;
        let r = unsafe { f(png, info, &mut p, &mut n) };
        let entries = if r != 0 && !p.is_null() && n > 0 {
            unsafe { std::slice::from_raw_parts(p, n as usize) }.to_vec()
        } else {
            Vec::new()
        };
        v.push(format!("PLTE r={r} n={n} {entries:?}"));
    }
    {
        let f: Symbol<
            unsafe extern "C-unwind" fn(
                png_structp,
                png_infop,
                *mut *mut u8,
                *mut c_int,
                *mut *mut png_color_16,
            ) -> u32,
        > = ctx.sym("png_get_tRNS");
        let mut ta: *mut u8 = std::ptr::null_mut();
        let mut n: c_int = -1;
        let mut tc: *mut png_color_16 = std::ptr::null_mut();
        let r = unsafe { f(png, info, &mut ta, &mut n, &mut tc) };
        // png_handle_tRNS only fills its stack scratch buffer for palette
        // images; for the other colour types the C library copies
        // indeterminate bytes into info_ptr->trans_alpha, so the contents are
        // only meaningful when the file actually carries a palette.
        let has_plte = {
            let v: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, u32) -> u32> =
                ctx.sym("png_get_valid");
            (unsafe { v(png, info, PNG_INFO_PLTE) }) != 0
        };
        let alpha = if !ta.is_null() && n > 0 && has_plte {
            unsafe { std::slice::from_raw_parts(ta, n as usize) }.to_vec()
        } else {
            Vec::new()
        };
        let col = if tc.is_null() { None } else { Some(unsafe { *tc }) };
        v.push(format!("tRNS r={r} n={n} {alpha:?} {col:?}"));
    }
    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut png_color_16) -> u32> =
            ctx.sym("png_get_bKGD");
        let mut c: *mut png_color_16 = std::ptr::null_mut();
        let r = unsafe { f(png, info, &mut c) };
        v.push(format!(
            "bKGD r={r} {:?}",
            if c.is_null() { None } else { Some(unsafe { *c }) }
        ));
    }
    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut png_color_8) -> u32> =
            ctx.sym("png_get_sBIT");
        let mut c: *mut png_color_8 = std::ptr::null_mut();
        let r = unsafe { f(png, info, &mut c) };
        v.push(format!(
            "sBIT r={r} {:?}",
            if c.is_null() { None } else { Some(unsafe { *c }) }
        ));
    }
    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut u16) -> u32> =
            ctx.sym("png_get_hIST");
        let mut c: *mut u16 = std::ptr::null_mut();
        let r = unsafe { f(png, info, &mut c) };
        let h = if r != 0 && !c.is_null() {
            unsafe { std::slice::from_raw_parts(c, 256) }.to_vec()
        } else {
            Vec::new()
        };
        v.push(format!("hIST r={r} {h:?}"));
    }
    {
        let f: Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut u32, *mut u32, *mut c_int) -> u32,
        > = ctx.sym("png_get_pHYs");
        let (mut x, mut y, mut u) = (0u32, 0u32, -1);
        let r = unsafe { f(png, info, &mut x, &mut y, &mut u) };
        v.push(format!("pHYs r={r} {x} {y} {u}"));
        let f: Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut u32, *mut u32, *mut c_int) -> u32,
        > = ctx.sym("png_get_pHYs_dpi");
        let (mut x, mut y, mut u) = (0u32, 0u32, -1);
        let r = unsafe { f(png, info, &mut x, &mut y, &mut u) };
        v.push(format!("pHYs_dpi r={r} {x} {y} {u}"));
        let f: Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut i32, *mut i32, *mut c_int) -> u32,
        > = ctx.sym("png_get_oFFs");
        let (mut x, mut y, mut u) = (0i32, 0i32, -1);
        let r = unsafe { f(png, info, &mut x, &mut y, &mut u) };
        v.push(format!("oFFs r={r} {x} {y} {u}"));
    }
    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut png_time) -> u32> =
            ctx.sym("png_get_tIME");
        let mut t: *mut png_time = std::ptr::null_mut();
        let r = unsafe { f(png, info, &mut t) };
        v.push(format!(
            "tIME r={r} {:?}",
            if t.is_null() { None } else { Some(unsafe { *t }) }
        ));
    }
    v.push(snapshot_pcal(ctx));
    {
        let f: Symbol<
            unsafe extern "C-unwind" fn(
                png_structp,
                png_infop,
                *mut c_int,
                *mut *mut c_char,
                *mut *mut c_char,
            ) -> u32,
        > = ctx.sym("png_get_sCAL_s");
        let mut unit = -1;
        let mut w: *mut c_char = std::ptr::null_mut();
        let mut h: *mut c_char = std::ptr::null_mut();
        let r = unsafe { f(png, info, &mut unit, &mut w, &mut h) };
        v.push(format!(
            "sCAL_s r={r} unit={unit} w={:?} h={:?}",
            cstr_of(w),
            cstr_of(h)
        ));
        let f: Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut c_int, *mut f64, *mut f64) -> u32,
        > = ctx.sym("png_get_sCAL");
        let mut unit = -1;
        let (mut w, mut h) = (-1f64, -1f64);
        let r = unsafe { f(png, info, &mut unit, &mut w, &mut h) };
        v.push(format!("sCAL r={r} unit={unit} {w:?} {h:?}"));
        let f: Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut c_int, *mut i32, *mut i32) -> u32,
        > = ctx.sym("png_get_sCAL_fixed");
        let mut unit = -1;
        let (mut w, mut h) = (-1i32, -1i32);
        let r = unsafe { f(png, info, &mut unit, &mut w, &mut h) };
        v.push(format!("sCAL_fixed r={r} unit={unit} {w} {h}"));
    }
    {
        let f: Symbol<
            unsafe extern "C-unwind" fn(
                png_structp,
                png_infop,
                *mut *mut c_char,
                *mut c_int,
                *mut *mut u8,
                *mut u32,
            ) -> u32,
        > = ctx.sym("png_get_iCCP");
        let mut name: *mut c_char = std::ptr::null_mut();
        let mut comp = -1;
        let mut prof: *mut u8 = std::ptr::null_mut();
        let mut plen = 0u32;
        let r = unsafe { f(png, info, &mut name, &mut comp, &mut prof, &mut plen) };
        let bytes = if !prof.is_null() && plen > 0 {
            unsafe { std::slice::from_raw_parts(prof, plen as usize) }.to_vec()
        } else {
            Vec::new()
        };
        v.push(format!(
            "iCCP r={r} name={:?} comp={comp} len={plen} {}",
            cstr_of(name),
            hex(&bytes)
        ));
    }
    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut png_sPLT_t) -> c_int> =
            ctx.sym("png_get_sPLT");
        let mut p: *mut png_sPLT_t = std::ptr::null_mut();
        let n = unsafe { f(png, info, &mut p) };
        let mut s = format!("sPLT n={n}");
        if n > 0 && !p.is_null() {
            for i in 0..n as usize {
                let e = unsafe { *p.add(i) };
                let entries = if e.nentries > 0 && !e.entries.is_null() {
                    unsafe { std::slice::from_raw_parts(e.entries, e.nentries as usize) }.to_vec()
                } else {
                    Vec::new()
                };
                s += &format!(
                    " [{:?} depth={} n={} {:?}]",
                    cstr_of(e.name),
                    e.depth,
                    e.nentries,
                    entries
                );
            }
        }
        v.push(s);
    }
    {
        let f: Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut png_text, *mut c_int) -> c_int,
        > = ctx.sym("png_get_text");
        let mut p: *mut png_text = std::ptr::null_mut();
        let mut n: c_int = -1;
        let r = unsafe { f(png, info, &mut p, &mut n) };
        let mut s = format!("text r={r} n={n}");
        if r > 0 && !p.is_null() {
            for i in 0..r as usize {
                let t = unsafe { *p.add(i) };
                s += &format!(
                    " [comp={} key={:?} text={:?} tl={} il={} lang={:?} lk={:?}]",
                    t.compression,
                    cstr_of(t.key),
                    cstr_of(t.text),
                    t.text_length,
                    t.itxt_length,
                    cstr_of(t.lang),
                    cstr_of(t.lang_key)
                );
            }
        }
        v.push(s);
    }
    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop, *mut u32, *mut *mut u8) -> u32> =
            ctx.sym("png_get_eXIf_1");
        let mut n = 0u32;
        let mut p: *mut u8 = std::ptr::null_mut();
        let r = unsafe { f(png, info, &mut n, &mut p) };
        let bytes = if !p.is_null() && n > 0 {
            unsafe { std::slice::from_raw_parts(p, n as usize) }.to_vec()
        } else {
            Vec::new()
        };
        v.push(format!("eXIf r={r} n={n} {}", hex(&bytes)));
    }
    {
        let f: Symbol<
            unsafe extern "C-unwind" fn(png_structp, png_infop, *mut *mut png_unknown_chunk) -> c_int,
        > = ctx.sym("png_get_unknown_chunks");
        let mut p: *mut png_unknown_chunk = std::ptr::null_mut();
        let n = unsafe { f(png, info, &mut p) };
        let mut s = format!("unknown n={n}");
        if n > 0 && !p.is_null() {
            for i in 0..n as usize {
                let u = unsafe { *p.add(i) };
                let data = if !u.data.is_null() && u.size > 0 {
                    unsafe { std::slice::from_raw_parts(u.data, u.size) }.to_vec()
                } else {
                    Vec::new()
                };
                s += &format!(
                    " [{} loc={} size={} {}]",
                    String::from_utf8_lossy(&u.name[..4]),
                    u.location,
                    u.size,
                    hex(&data)
                );
            }
        }
        v.push(s);
    }
    {
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> *mut *mut u8> =
            ctx.sym("png_get_rows");
        v.push(format!("rows_set={}", !unsafe { f(png, info) }.is_null()));
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp) -> c_int> = ctx.sym("png_get_palette_max");
        v.push(format!("palette_max={}", unsafe { f(png) }));
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp) -> u8> =
            ctx.sym("png_get_rgb_to_gray_status");
        v.push(format!("rgb_to_gray_status={}", unsafe { f(png) }));
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp) -> u32> = ctx.sym("png_get_io_state");
        v.push(format!("io_state={}", unsafe { f(png) }));
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp) -> u32> = ctx.sym("png_get_io_chunk_type");
        v.push(format!("io_chunk_type={:#x}", unsafe { f(png) }));
        for n in [
            "png_get_user_width_max",
            "png_get_user_height_max",
            "png_get_chunk_cache_max",
        ] {
            let f: Symbol<unsafe extern "C-unwind" fn(png_structp) -> u32> = ctx.sym(n);
            v.push(format!("{n}={}", unsafe { f(png) }));
        }
        for n in ["png_get_chunk_malloc_max", "png_get_compression_buffer_size"] {
            let f: Symbol<unsafe extern "C-unwind" fn(png_structp) -> usize> = ctx.sym(n);
            v.push(format!("{n}={}", unsafe { f(png) }));
        }
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp) -> u32> =
            ctx.sym("png_get_current_row_number");
        v.push(format!("current_row={}", unsafe { f(png) }));
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp) -> u8> =
            ctx.sym("png_get_current_pass_number");
        v.push(format!("current_pass={}", unsafe { f(png) }));
        let f: Symbol<unsafe extern "C-unwind" fn(png_structp, png_infop) -> *const u8> =
            ctx.sym("png_get_signature");
        let sig = unsafe { f(png, info) };
        let sigv = if sig.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(sig, 8) }.to_vec()
        };
        v.push(format!("signature={}", hex(&sigv)));
    }
    v
}

/// pCAL has a nine-argument accessor; keep it separate for legibility.
pub fn snapshot_pcal(ctx: &Ctx) -> String {
    type F = unsafe extern "C-unwind" fn(
        png_structp,
        png_infop,
        *mut *mut c_char,
        *mut i32,
        *mut i32,
        *mut c_int,
        *mut c_int,
        *mut *mut c_char,
        *mut *mut *mut c_char,
    ) -> u32;
    let f: Symbol<F> = ctx.sym("png_get_pCAL");
    let mut purpose: *mut c_char = std::ptr::null_mut();
    let (mut x0, mut x1) = (0i32, 0i32);
    let (mut ty, mut nparams) = (-1, -1);
    let mut units: *mut c_char = std::ptr::null_mut();
    let mut params: *mut *mut c_char = std::ptr::null_mut();
    let r = unsafe {
        f(
            ctx.png,
            ctx.info,
            &mut purpose,
            &mut x0,
            &mut x1,
            &mut ty,
            &mut nparams,
            &mut units,
            &mut params,
        )
    };
    let mut s = format!(
        "pCAL r={r} purpose={:?} x0={x0} x1={x1} type={ty} nparams={nparams} units={:?}",
        cstr_of(purpose),
        cstr_of(units)
    );
    if r != 0 && !params.is_null() && nparams > 0 {
        for i in 0..nparams as usize {
            s += &format!(" p{}={:?}", i, cstr_of(unsafe { *params.add(i) }));
        }
    }
    s
}

/// Compare two snapshots line by line with a helpful diff message.
pub fn assert_snapshots_eq(label: &str, a: &[String], b: &[String]) {
    assert_eq!(a.len(), b.len(), "{label}: snapshot length differs");
    let mut diffs = Vec::new();
    for (x, y) in a.iter().zip(b.iter()) {
        if x != y {
            diffs.push(format!("  C: {x}\n  R: {y}"));
        }
    }
    assert!(diffs.is_empty(), "{label}: {} field(s) differ:\n{}", diffs.len(), diffs.join("\n"));
}
