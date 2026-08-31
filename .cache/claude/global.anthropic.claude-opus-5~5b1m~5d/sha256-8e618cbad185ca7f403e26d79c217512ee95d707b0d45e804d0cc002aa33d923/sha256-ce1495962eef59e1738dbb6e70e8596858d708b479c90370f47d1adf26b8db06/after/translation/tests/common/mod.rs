//! Differential-test harness.
//!
//! Both the reference C `libpng.so` and the translated Rust `liblibpng.so` are
//! `dlopen`ed and driven through *identical* function-pointer types, so every
//! call in every test goes through the real exported ABI of both libraries.
//! Nothing is ever called directly on the Rust crate.
#![allow(non_camel_case_types, non_snake_case, dead_code, unused_imports)]

pub mod api;
pub mod types;

pub use api::Api;
pub use types::*;

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Library handles
// ---------------------------------------------------------------------------

fn root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR == <work>/translation
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

static C_LIB: OnceLock<Api> = OnceLock::new();
static RS_LIB: OnceLock<Api> = OnceLock::new();

/// The reference `CMakeLists.txt` links only zlib, so the C `libpng.so` has an
/// unresolved `floor`/`pow` from libm.  Make libm globally visible first.
fn preload(names: &[&str]) {
    use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_NOW};
    for n in names {
        if let Ok(l) = unsafe { UnixLib::open(Some(*n), RTLD_NOW | RTLD_GLOBAL) } {
            std::mem::forget(l);
            return;
        }
    }
    panic!("cannot preload any of {:?}", names);
}

pub fn c_api() -> &'static Api {
    C_LIB.get_or_init(|| {
        preload(&["libm.so.6", "libm.so"]);
        preload(&["libz.so.1", "libz.so"]);
        let p = root().join("c_src/build/libpng.so");
        assert!(p.exists(), "reference C library not built: {}", p.display());
        Api::load(p.to_str().unwrap(), "C")
    })
}

pub fn rs_api() -> &'static Api {
    RS_LIB.get_or_init(|| {
        let mut p = root().join("translation/target/release/liblibpng.so");
        if !p.exists() {
            p = root().join("translation/target/debug/liblibpng.so");
        }
        assert!(p.exists(), "rust cdylib not built: {}", p.display());
        Api::load(p.to_str().unwrap(), "RS")
    })
}

/// Both implementations, in a fixed order, for `for (name, api) in both()`.
pub fn both() -> [&'static Api; 2] {
    [c_api(), rs_api()]
}

// ---------------------------------------------------------------------------
// Diagnostics capture
//
// `png_error` never returns in C: the error callback is expected to
// `longjmp`.  We cannot `setjmp` from Rust, so instead the callback unwinds
// with a Rust panic.  All of libpng's exported entry points are declared
// `extern "C-unwind"` (in both libraries: the C one is compiled by gcc which
// emits unwind tables by default), so the panic propagates back to the
// `catch_unwind` in `Session::run`.
// ---------------------------------------------------------------------------

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

thread_local! {
    static DIAG: RefCell<Diag> = RefCell::new(Diag::default());
    static HOOK_SET: RefCell<bool> = const { RefCell::new(false) };
}

struct PngLongjmp;

fn install_quiet_hook() {
    HOOK_SET.with(|h| {
        if !*h.borrow() {
            *h.borrow_mut() = true;
        }
    });
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Suppress the noise from the expected png_error unwinds only.
            if info.payload().downcast_ref::<PngLongjmp>().is_some() {
                return;
            }
            prev(info);
        }));
    });
}

pub unsafe extern "C-unwind" fn cb_error(_png: png_structp, msg: png_const_charp) {
    let m = if msg.is_null() {
        String::from("<null>")
    } else {
        CStr::from_ptr(msg).to_string_lossy().into_owned()
    };
    DIAG.with(|d| d.borrow_mut().errors.push(m));
    std::panic::panic_any(PngLongjmp);
}

pub unsafe extern "C-unwind" fn cb_warning(_png: png_structp, msg: png_const_charp) {
    let m = if msg.is_null() {
        String::from("<null>")
    } else {
        CStr::from_ptr(msg).to_string_lossy().into_owned()
    };
    DIAG.with(|d| d.borrow_mut().warnings.push(m));
}

pub fn diag_reset() {
    DIAG.with(|d| *d.borrow_mut() = Diag::default());
}

pub fn diag_take() -> Diag {
    DIAG.with(|d| std::mem::take(&mut *d.borrow_mut()))
}

/// Run `f`, catching the panic raised by [`cb_error`].  Returns `None` when
/// libpng raised an error.
pub fn guard<T>(f: impl FnOnce() -> T) -> Option<T> {
    install_quiet_hook();
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => Some(v),
        Err(e) => {
            if e.downcast_ref::<PngLongjmp>().is_some() {
                None
            } else {
                std::panic::resume_unwind(e)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Memory IO
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct WriteSink {
    pub buf: Vec<u8>,
    pub flushes: u32,
}

pub unsafe extern "C-unwind" fn cb_write(png: png_structp, data: png_bytep, len: usize) {
    let api = current_api();
    let p = (api.png_get_io_ptr)(png) as *mut WriteSink;
    assert!(!p.is_null(), "no io ptr in write callback");
    if len > 0 {
        (*p).buf
            .extend_from_slice(std::slice::from_raw_parts(data, len));
    }
}

pub unsafe extern "C-unwind" fn cb_flush(png: png_structp) {
    let api = current_api();
    let p = (api.png_get_io_ptr)(png) as *mut WriteSink;
    if !p.is_null() {
        (*p).flushes += 1;
    }
}

#[repr(C)]
pub struct ReadSource {
    pub data: Vec<u8>,
    pub pos: usize,
}

pub unsafe extern "C-unwind" fn cb_read(png: png_structp, data: png_bytep, len: usize) {
    let api = current_api();
    let p = (api.png_get_io_ptr)(png) as *mut ReadSource;
    assert!(!p.is_null(), "no io ptr in read callback");
    let src = &mut *p;
    if src.pos + len > src.data.len() {
        // Mirror png_default_read_data's behaviour on a short read.
        let m = CString::new("Read Error").unwrap();
        (api.png_error)(png, m.as_ptr());
    }
    if len > 0 {
        std::ptr::copy_nonoverlapping(src.data.as_ptr().add(src.pos), data, len);
    }
    src.pos += len;
}

// The IO/error callbacks need to know *which* library they were called from in
// order to call back into it (png_get_io_ptr / png_error).  Tests only ever
// drive one library at a time, so a thread-local "current" pointer suffices.
thread_local! {
    static CUR: RefCell<*const Api> = const { RefCell::new(std::ptr::null()) };
}

pub fn set_current_api(a: &'static Api) {
    CUR.with(|c| *c.borrow_mut() = a as *const Api);
}

pub fn current_api() -> &'static Api {
    CUR.with(|c| {
        let p = *c.borrow();
        assert!(!p.is_null(), "current api not set");
        unsafe { &*p }
    })
}

// ---------------------------------------------------------------------------
// Session helpers
// ---------------------------------------------------------------------------

/// A read session: png_struct + info_struct + a memory source.
pub struct ReadSess {
    pub api: &'static Api,
    pub png: png_structp,
    pub info: png_infop,
    pub end: png_infop,
    pub src: Box<ReadSource>,
}

impl ReadSess {
    pub unsafe fn new(api: &'static Api, data: &[u8]) -> ReadSess {
        set_current_api(api);
        let v = ver();
        let png = (api.png_create_read_struct)(
            v.as_ptr(),
            std::ptr::null_mut(),
            Some(cb_error),
            Some(cb_warning),
        );
        assert!(!png.is_null(), "{}: create_read_struct", api.name);
        let info = (api.png_create_info_struct)(png);
        assert!(!info.is_null());
        let end = (api.png_create_info_struct)(png);
        assert!(!end.is_null());
        let mut src = Box::new(ReadSource {
            data: data.to_vec(),
            pos: 0,
        });
        (api.png_set_read_fn)(
            png,
            &mut *src as *mut ReadSource as png_voidp,
            Some(cb_read),
        );
        ReadSess {
            api,
            png,
            info,
            end,
            src,
        }
    }
}

impl Drop for ReadSess {
    fn drop(&mut self) {
        unsafe {
            set_current_api(self.api);
            let mut p = self.png;
            let mut i = self.info;
            let mut e = self.end;
            if !p.is_null() {
                (self.api.png_destroy_read_struct)(&mut p, &mut i, &mut e);
            }
        }
    }
}

/// A write session: png_struct + info_struct + a memory sink.
pub struct WriteSess {
    pub api: &'static Api,
    pub png: png_structp,
    pub info: png_infop,
    pub sink: Box<WriteSink>,
}

impl WriteSess {
    pub unsafe fn new(api: &'static Api) -> WriteSess {
        set_current_api(api);
        let v = ver();
        let png = (api.png_create_write_struct)(
            v.as_ptr(),
            std::ptr::null_mut(),
            Some(cb_error),
            Some(cb_warning),
        );
        assert!(!png.is_null(), "{}: create_write_struct", api.name);
        let info = (api.png_create_info_struct)(png);
        assert!(!info.is_null());
        let mut sink = Box::new(WriteSink {
            buf: Vec::new(),
            flushes: 0,
        });
        (api.png_set_write_fn)(
            png,
            &mut *sink as *mut WriteSink as png_voidp,
            Some(cb_write),
            Some(cb_flush),
        );
        WriteSess {
            api,
            png,
            info,
            sink,
        }
    }
}

impl Drop for WriteSess {
    fn drop(&mut self) {
        unsafe {
            set_current_api(self.api);
            let mut p = self.png;
            let mut i = self.info;
            if !p.is_null() {
                (self.api.png_destroy_write_struct)(&mut p, &mut i);
            }
        }
    }
}

/// PNG_ROWBYTES
pub fn rowbytes(pixel_depth: u32, width: u32) -> usize {
    if pixel_depth >= 8 {
        (width as usize) * ((pixel_depth as usize) >> 3)
    } else {
        (((width as usize) * (pixel_depth as usize)) + 7) >> 3
    }
}

pub const PNG_PASS_INC: [u32; 7] = [8, 8, 4, 4, 2, 2, 1];
pub const PNG_PASS_ROW_INC: [u32; 7] = [8, 8, 8, 4, 4, 2, 2];
pub const PNG_PASS_START_ROW: [u32; 7] = [0, 0, 4, 0, 2, 0, 1];
pub const PNG_PASS_START_COL: [u32; 7] = [0, 4, 0, 2, 0, 1, 0];

pub fn channels_of(color_type: c_int) -> u32 {
    match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => 1,
    }
}

/// The (color_type, bit_depth) pairs the PNG spec (and png_check_IHDR) allows.
pub fn legal_ihdr() -> Vec<(c_int, c_int)> {
    let mut v = Vec::new();
    for &d in &[1, 2, 4, 8, 16] {
        v.push((PNG_COLOR_TYPE_GRAY, d));
    }
    for &d in &[1, 2, 4, 8] {
        v.push((PNG_COLOR_TYPE_PALETTE, d));
    }
    for &d in &[8, 16] {
        v.push((PNG_COLOR_TYPE_RGB, d));
        v.push((PNG_COLOR_TYPE_GRAY_ALPHA, d));
        v.push((PNG_COLOR_TYPE_RGB_ALPHA, d));
    }
    v
}

// ---------------------------------------------------------------------------
// Small conveniences
// ---------------------------------------------------------------------------

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

pub unsafe fn rs_str(p: png_const_charp) -> Option<String> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_string_lossy().into_owned())
    }
}

pub fn ver() -> CString {
    cs(PNG_LIBPNG_VER_STRING)
}

/// Deterministic xorshift PRNG so every test is reproducible.
pub struct Rng(pub u64);

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
        (self.next_u64() >> 40) as u8
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.u32() % n
        }
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        // inclusive
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 0x10 != 0
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

/// Compare two byte buffers and report the first difference.
pub fn assert_bytes_eq(what: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let n = c.len().min(r.len());
    let mut first = n;
    for i in 0..n {
        if c[i] != r[i] {
            first = i;
            break;
        }
    }
    let lo = first.saturating_sub(8);
    let hi = (first + 24).min(n);
    panic!(
        "{}: byte mismatch (C len {}, RS len {}) at offset {}\n  C : {}\n  RS: {}",
        what,
        c.len(),
        r.len(),
        first,
        hex(&c[lo..hi.max(lo)]),
        hex(&r[lo..hi.max(lo)]),
    );
}
