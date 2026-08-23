//! Shared differential-test harness.
//!
//! Both the reference C `libpng.so` and the translated Rust `liblibpng.so` are
//! loaded with `libloading`; every call goes through the dynamic symbol table,
//! so the `#[no_mangle]` export wrappers are exercised exactly as an external
//! consumer would exercise them.
//!
//! Each driver produces a *trace*: a `Vec<String>` of every observable event
//! (return values, warning/error messages, produced bytes, decoded rows).  A
//! configuration passes when the C trace and the Rust trace are identical.
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub const VER_STRING: &[u8] = b"1.6.59.git\0";

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    lib: libloading::Library,
    pub tag: &'static str,
}

impl Lib {
    pub fn open(path: &Path, tag: &'static str) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
        Lib { lib, tag }
    }

    /// Resolve `name` and transmute it to the requested function-pointer type.
    pub fn f<T: Copy>(&self, name: &str) -> T {
        assert_eq!(
            std::mem::size_of::<T>(),
            std::mem::size_of::<*const c_void>(),
            "f::<T>() is only for pointer-sized types"
        );
        let p = self.raw(name);
        unsafe { std::mem::transmute_copy::<*const c_void, T>(&p) }
    }

    /// Resolve `name`, returning `None` when the symbol is absent.
    pub fn opt<T: Copy>(&self, name: &str) -> Option<T> {
        let mut owned = name.as_bytes().to_vec();
        owned.push(0);
        let sym: Result<libloading::Symbol<*const c_void>, _> = unsafe { self.lib.get(&owned) };
        sym.ok().map(|s| {
            let p = unsafe { *s };
            unsafe { std::mem::transmute_copy::<*const c_void, T>(&p) }
        })
    }

    pub fn raw(&self, name: &str) -> *const c_void {
        let mut owned = name.as_bytes().to_vec();
        owned.push(0);
        let sym: libloading::Symbol<*const c_void> = unsafe { self.lib.get(&owned) }
            .unwrap_or_else(|e| panic!("{}: missing symbol {name}: {e}", self.tag));
        unsafe { *sym }
    }

    /// Address of an exported data object.
    pub fn data(&self, name: &str) -> *const u8 {
        self.raw(name) as *const u8
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("PNG_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("target/cbuild/libpng.so");
    assert!(
        p.exists(),
        "C reference library not built: {} missing (cmake -S c_src -B target/cbuild && cmake --build target/cbuild)",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("PNG_RUST_SO") {
        return PathBuf::from(p);
    }
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe.parent().unwrap().parent().unwrap();
    for name in ["liblibpng.so", "libpng.so"] {
        let p = dir.join(name);
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib not found next to {}", exe.display());
}

pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// The reference C `.so` is linked without `-lm` (the upstream CMakeLists links
/// only zlib), so `floor`/`pow` are unresolved in it.  A real application links
/// libm; we emulate that by loading libm globally before either library is used,
/// identically for both.
fn ensure_libm() {
    static LIBM: OnceLock<Option<libloading::os::unix::Library>> = OnceLock::new();
    LIBM.get_or_init(|| unsafe {
        // RTLD_GLOBAL is essential: the C .so is dlopen'ed RTLD_LOCAL and can
        // only resolve floor/pow from the global scope.
        const RTLD_NOW: i32 = 2;
        const RTLD_GLOBAL: i32 = 0x100;
        libloading::os::unix::Library::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL).ok()
    });
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        ensure_libm();
        // Load the shim first so the landing pad exists before any png_error.
        shim();
        Pair {
            c: Lib::open(&c_so_path(), "C"),
            rust: Lib::open(&rust_so_path(), "RUST"),
        }
    })
}

// ---------------------------------------------------------------------------
// setjmp/longjmp shim
// ---------------------------------------------------------------------------

pub type ThFn = Option<unsafe extern "C" fn(*mut c_void)>;

pub struct Shim {
    _lib: libloading::Library,
    protect: unsafe extern "C" fn(ThFn, *mut c_void) -> c_int,
    pub longjmp_ptr: *const c_void,
    pub jmp_buf_size: usize,
}

unsafe impl Send for Shim {}
unsafe impl Sync for Shim {}

static SHIM: OnceLock<Shim> = OnceLock::new();

pub fn shim() -> &'static Shim {
    SHIM.get_or_init(|| {
        let src = manifest_dir().join("tests/support/testshim.c");
        let out_dir = manifest_dir().join("target/testsupport");
        std::fs::create_dir_all(&out_dir).unwrap();
        let so = out_dir.join("libtestshim.so");
        // Compile to a unique file then rename: several test binaries may race.
        let tmp = out_dir.join(format!("libtestshim-{}.so", std::process::id()));
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        let st = Command::new(&cc)
            .args(["-O0", "-fPIC", "-shared", "-o"])
            .arg(&tmp)
            .arg(&src)
            .status()
            .expect("failed to run C compiler for test shim");
        assert!(st.success(), "test shim compilation failed");
        std::fs::rename(&tmp, &so).unwrap();

        let lib = unsafe { libloading::Library::new(&so) }.expect("dlopen test shim");
        let protect: unsafe extern "C" fn(ThFn, *mut c_void) -> c_int = unsafe {
            let s: libloading::Symbol<*const c_void> = lib.get(b"th_protect\0").unwrap();
            std::mem::transmute(*s)
        };
        let longjmp_ptr = unsafe {
            let s: libloading::Symbol<*const c_void> = lib.get(b"th_longjmp\0").unwrap();
            *s
        };
        let size_fn: unsafe extern "C" fn() -> usize = unsafe {
            let s: libloading::Symbol<*const c_void> = lib.get(b"th_jmp_buf_size\0").unwrap();
            std::mem::transmute(*s)
        };
        let jmp_buf_size = unsafe { size_fn() };
        Shim {
            _lib: lib,
            protect,
            longjmp_ptr,
            jmp_buf_size,
        }
    })
}

/// Run `f` with a longjmp landing pad installed.  Returns 0 when `f` completed
/// normally, non-zero when libpng (or a harness callback) performed a longjmp.
///
/// `f` must not own anything that needs dropping: a longjmp skips destructors.
pub fn protected<F: FnMut()>(mut f: F) -> c_int {
    unsafe extern "C" fn tramp<F: FnMut()>(p: *mut c_void) {
        unsafe { (*(p as *mut F))() }
    }
    let s = shim();
    unsafe { (s.protect)(Some(tramp::<F>), &mut f as *mut F as *mut c_void) }
}

// ---------------------------------------------------------------------------
// Per-thread session state (trace log + simulated I/O)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Session {
    pub log: Vec<String>,
    /// Bytes handed to the write callback.
    pub out: Vec<u8>,
    /// Bytes served by the read callback.
    pub input: Vec<u8>,
    pub rpos: usize,
    /// Trace every png_malloc/png_free through the user-memory callbacks.
    pub trace_alloc: bool,
    /// Count of live allocations made through the user-memory callbacks.
    pub live_allocs: i64,
    /// Make the write callback fail (longjmp) once this many bytes were written.
    pub write_limit: Option<usize>,
    /// Fail malloc after this many successful allocations.
    pub malloc_limit: Option<usize>,
    pub malloc_count: usize,
}

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::default());
}

pub fn session_reset(input: Vec<u8>) {
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        *s = Session::default();
        s.input = input;
    });
}

pub fn with_session<R>(f: impl FnOnce(&mut Session) -> R) -> R {
    SESSION.with(|s| f(&mut s.borrow_mut()))
}

pub fn log(msg: impl AsRef<str>) {
    let m = msg.as_ref().to_string();
    SESSION.with(|s| s.borrow_mut().log.push(m));
}

pub fn take_log() -> Vec<String> {
    SESSION.with(|s| std::mem::take(&mut s.borrow_mut().log))
}

pub fn take_out() -> Vec<u8> {
    SESSION.with(|s| std::mem::take(&mut s.borrow_mut().out))
}

pub fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

// ---------------------------------------------------------------------------
// libpng callbacks (identical for both libraries)
// ---------------------------------------------------------------------------

pub unsafe extern "C" fn cb_error(_png: *mut c_void, msg: *const c_char) {
    log(format!("ERROR({})", cstr(msg)));
    // Returning lets libpng call png_longjmp -> th_longjmp, which is what a
    // real application relies on.
}

pub unsafe extern "C" fn cb_warning(_png: *mut c_void, msg: *const c_char) {
    log(format!("WARNING({})", cstr(msg)));
}

pub unsafe extern "C" fn cb_write(_png: *mut c_void, data: *mut u8, len: usize) {
    let mut over = false;
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        if len > 0 {
            let src = unsafe { std::slice::from_raw_parts(data, len) };
            s.out.extend_from_slice(src);
        }
        if let Some(lim) = s.write_limit {
            if s.out.len() > lim {
                over = true;
            }
        }
    });
    if over {
        log("WRITE_LIMIT".to_string());
        th_longjmp_now(7);
    }
}

pub unsafe extern "C" fn cb_flush(_png: *mut c_void) {
    log("FLUSH".to_string());
}

pub unsafe extern "C" fn cb_read(_png: *mut c_void, data: *mut u8, len: usize) {
    let mut short = false;
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        let avail = s.input.len().saturating_sub(s.rpos);
        if avail < len {
            short = true;
        } else {
            if len > 0 {
                let src = &s.input[s.rpos..s.rpos + len];
                unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), data, len) };
            }
            s.rpos += len;
        }
    });
    if short {
        log("READ_SHORT".to_string());
        th_longjmp_now(3);
    }
}

/// User-memory malloc callback: `png_malloc_ptr`.
pub unsafe extern "C" fn cb_malloc(_png: *mut c_void, size: usize) -> *mut c_void {
    let mut fail = false;
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        s.malloc_count += 1;
        if let Some(lim) = s.malloc_limit {
            if s.malloc_count > lim {
                fail = true;
            }
        }
        if s.trace_alloc {
            let n = s.malloc_count;
            s.log.push(format!("MALLOC(#{n},{size})"));
        }
        if !fail {
            s.live_allocs += 1;
        }
    });
    if fail {
        return std::ptr::null_mut();
    }
    // Use calloc so that any dependence on uninitialised memory shows up
    // identically in both libraries.
    unsafe { libc_calloc(1, if size == 0 { 1 } else { size }) }
}

pub unsafe extern "C" fn cb_free(_png: *mut c_void, p: *mut c_void) {
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        if s.trace_alloc {
            s.log.push(format!("FREE({})", if p.is_null() { 0 } else { 1 }));
        }
        if !p.is_null() {
            s.live_allocs -= 1;
        }
    });
    if !p.is_null() {
        unsafe { libc_free(p) };
    }
}

extern "C" {
    #[link_name = "calloc"]
    fn libc_calloc(n: usize, size: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

pub fn th_longjmp_now(val: c_int) -> ! {
    let f: unsafe extern "C" fn(*mut c_void, c_int) =
        unsafe { std::mem::transmute(shim().longjmp_ptr) };
    unsafe { f(std::ptr::null_mut(), val) };
    unreachable!("th_longjmp returned");
}

// ---------------------------------------------------------------------------
// Trace comparison
// ---------------------------------------------------------------------------

pub struct Trace {
    pub lines: Vec<String>,
    pub out: Vec<u8>,
    pub rc: c_int,
}

impl Trace {
    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("rc={}\n", self.rc));
        for l in &self.lines {
            s.push_str(l);
            s.push('\n');
        }
        s.push_str(&format!("out.len={}\n", self.out.len()));
        s.push_str(&format!("out={}\n", hex(&self.out)));
        s
    }
}

/// Every distinct `ERROR(...)`/`WARNING(...)` line observed from the C library,
/// appended to `target/testsupport/observed_msgs.txt` for mechanical
/// ERRORS.md coverage accounting.
static SEEN: OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> = OnceLock::new();

pub fn record_messages(lines: &[String]) {
    let set = SEEN.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    let mut fresh: Vec<String> = Vec::new();
    {
        let mut g = set.lock().unwrap();
        for l in lines {
            if l.starts_with("ERROR(") || l.starts_with("WARNING(") || l.starts_with("IMAGE_ERR(")
            {
                if g.insert(l.clone()) {
                    fresh.push(l.clone());
                }
            }
        }
    }
    if fresh.is_empty() {
        return;
    }
    use std::io::Write;
    let dir = manifest_dir().join("target/testsupport");
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("observed_msgs.txt"))
    {
        for l in fresh {
            let _ = writeln!(f, "{l}");
        }
    }
}

/// Run one driver closure against both libraries and require identical traces.
pub fn diff(label: &str, mut run: impl FnMut(&Lib) -> Trace) {
    let p = pair();
    let ct = run(&p.c);
    let rt = run(&p.rust);
    record_messages(&ct.lines);
    if ct.render() != rt.render() {
        let a = ct.render();
        let b = rt.render();
        let mut first = String::new();
        for (i, (la, lb)) in a.lines().zip(b.lines()).enumerate() {
            if la != lb {
                let sa = clip(la);
                let sb = clip(lb);
                first = format!("first divergence at trace line {i}:\n  C   : {sa}\n  RUST: {sb}");
                break;
            }
        }
        if first.is_empty() {
            let na = a.lines().count();
            let nb = b.lines().count();
            first = format!("trace lengths differ: C={na} lines, RUST={nb} lines");
        }
        panic!("[{label}] C and Rust diverge\n{first}\n");
    }
}

fn clip(s: &str) -> String {
    if s.len() <= 400 {
        s.to_string()
    } else {
        format!("{}...<{} bytes total>", &s[..400], s.len())
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every test is reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ---------------------------------------------------------------------------
// Sub-modules
// ---------------------------------------------------------------------------

pub mod core;
pub mod pngbuild;

use crate::support::core::{Cb, Core, Info, Png};

/// Create a write struct wired to the harness callbacks, run `body`, destroy.
pub fn with_write(lib: &Lib, body: &mut dyn FnMut(&Core, Png, Info)) -> Trace {
    session_reset(Vec::new());
    let core = Core::new(lib);
    let rc = protected(|| unsafe {
        let png = (core.create_write)(
            VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            cb_error as Cb,
            cb_warning as Cb,
        );
        log(format!("create_write={}", if png.is_null() { 0 } else { 1 }));
        if png.is_null() {
            return;
        }
        (core.set_longjmp)(png, shim().longjmp_ptr, shim().jmp_buf_size);
        (core.set_write_fn)(png, std::ptr::null_mut(), cb_write as Cb, cb_flush as Cb);
        let info = (core.create_info)(png);
        log(format!("create_info={}", if info.is_null() { 0 } else { 1 }));
        body(&core, png, info);
        let mut p = png;
        let mut i = info;
        (core.destroy_write)(&mut p, &mut i);
        log("destroyed".to_string());
    });
    Trace {
        lines: take_log(),
        out: take_out(),
        rc,
    }
}

/// Create a read struct fed from `input`, run `body`, destroy.
pub fn with_read(lib: &Lib, input: &[u8], body: &mut dyn FnMut(&Core, Png, Info)) -> Trace {
    session_reset(input.to_vec());
    let core = Core::new(lib);
    let rc = protected(|| unsafe {
        let png = (core.create_read)(
            VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            cb_error as Cb,
            cb_warning as Cb,
        );
        log(format!("create_read={}", if png.is_null() { 0 } else { 1 }));
        if png.is_null() {
            return;
        }
        (core.set_longjmp)(png, shim().longjmp_ptr, shim().jmp_buf_size);
        (core.set_read_fn)(png, std::ptr::null_mut(), cb_read as Cb);
        let info = (core.create_info)(png);
        log(format!("create_info={}", if info.is_null() { 0 } else { 1 }));
        body(&core, png, info);
        let mut p = png;
        let mut i = info;
        (core.destroy_read)(&mut p, &mut i, std::ptr::null_mut());
        log("destroyed".to_string());
    });
    Trace {
        lines: take_log(),
        out: take_out(),
        rc,
    }
}

/// Log every ancillary-information getter, so that any divergence in the
/// decoded info struct shows up in the trace.
pub unsafe fn log_all_info(c: &Core, png: Png, info: Info) {
    use crate::support::core::*;
    let mut w = 0u32;
    let mut h = 0u32;
    let (mut bd, mut ct, mut il, mut cm, mut fm) = (0, 0, 0, 0, 0);
    let r = (c.get_IHDR)(png, info, &mut w, &mut h, &mut bd, &mut ct, &mut il, &mut cm, &mut fm);
    log(format!(
        "IHDR rc={r} w={w} h={h} depth={bd} color={ct} interlace={il} comp={cm} filter={fm}"
    ));
    log(format!(
        "rowbytes={} channels={} palette_max={}",
        (c.get_rowbytes)(png, info),
        (c.get_channels)(png, info),
        (c.get_palette_max)(png, info)
    ));
    let mut valid = 0u32;
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
        let v = (c.get_valid)(png, info, flag);
        if v != 0 {
            valid |= flag;
        }
        log(format!("valid.{name}={v}"));
    }

    // PLTE
    let mut pal: *mut u8 = std::ptr::null_mut();
    let mut npal: c_int = -1;
    let r = (c.get_PLTE)(png, info, &mut pal, &mut npal);
    log(format!("PLTE rc={r} n={npal}"));
    if r != 0 && !pal.is_null() && npal > 0 {
        let s = std::slice::from_raw_parts(pal, npal as usize * 3);
        log(format!("PLTE data={}", hex(s)));
    }
    // tRNS
    let mut ta: *mut u8 = std::ptr::null_mut();
    let mut nt: c_int = -1;
    let mut tc: *mut u8 = std::ptr::null_mut();
    let r = (c.get_tRNS)(png, info, &mut ta, &mut nt, &mut tc);
    log(format!("tRNS rc={r} n={nt}"));
    if r != 0 && !ta.is_null() && nt > 0 {
        log(format!(
            "tRNS alpha={}",
            hex(std::slice::from_raw_parts(ta, nt as usize))
        ));
    }
    if r != 0 && !tc.is_null() {
        let v = *(tc as *const PngColor16);
        log(format!(
            "tRNS color idx={} r={} g={} b={} gray={}",
            v.index, v.red, v.green, v.blue, v.gray
        ));
    }
    // gAMA
    let mut g: i32 = -1;
    log(format!(
        "gAMA rc={} v={g}",
        (c.get_gAMA_fixed)(png, info, &mut g)
    ));
    let mut gd: f64 = -1.0;
    log(format!(
        "gAMA_fp rc={} v={:.10}",
        (c.get_gAMA)(png, info, &mut gd),
        gd
    ));
    // sRGB
    let mut intent: c_int = -1;
    log(format!(
        "sRGB rc={} intent={intent}",
        (c.get_sRGB)(png, info, &mut intent)
    ));
    // cHRM
    let mut v = [0i32; 8];
    let r = (c.get_cHRM_fixed)(
        png, info, &mut v[0], &mut v[1], &mut v[2], &mut v[3], &mut v[4], &mut v[5], &mut v[6],
        &mut v[7],
    );
    log(format!("cHRM rc={r} {v:?}"));
    let mut xyz = [0i32; 9];
    let r = (c.get_cHRM_XYZ_fixed)(
        png, info, &mut xyz[0], &mut xyz[1], &mut xyz[2], &mut xyz[3], &mut xyz[4], &mut xyz[5],
        &mut xyz[6], &mut xyz[7], &mut xyz[8],
    );
    log(format!("cHRM_XYZ rc={r} {xyz:?}"));
    // iCCP
    let mut name: *mut c_char = std::ptr::null_mut();
    let mut comp: c_int = -1;
    let mut prof: *mut u8 = std::ptr::null_mut();
    let mut plen: u32 = 0;
    let r = (c.get_iCCP)(png, info, &mut name, &mut comp, &mut prof, &mut plen);
    log(format!(
        "iCCP rc={r} name={} comp={comp} len={plen}",
        cstr(name)
    ));
    if r != 0 && !prof.is_null() && plen > 0 {
        log(format!(
            "iCCP data={}",
            hex(std::slice::from_raw_parts(prof, plen as usize))
        ));
    }
    // sBIT
    let mut sb: *mut u8 = std::ptr::null_mut();
    let r = (c.get_sBIT)(png, info, &mut sb);
    log(format!("sBIT rc={r}"));
    if r != 0 && !sb.is_null() {
        let v = *(sb as *const PngColor8);
        log(format!("sBIT v={v:?}"));
    }
    // bKGD
    let mut bk: *mut u8 = std::ptr::null_mut();
    let r = (c.get_bKGD)(png, info, &mut bk);
    log(format!("bKGD rc={r}"));
    if r != 0 && !bk.is_null() {
        let v = *(bk as *const PngColor16);
        log(format!("bKGD v={v:?}"));
    }
    // hIST
    let mut hi: *mut u16 = std::ptr::null_mut();
    let r = (c.get_hIST)(png, info, &mut hi);
    log(format!("hIST rc={r}"));
    if r != 0 && !hi.is_null() && npal > 0 {
        let s = std::slice::from_raw_parts(hi, npal as usize);
        log(format!("hIST v={s:?}"));
    }
    // pHYs
    let (mut px, mut py, mut unit) = (0u32, 0u32, 0);
    log(format!(
        "pHYs rc={} x={px} y={py} unit={unit}",
        (c.get_pHYs)(png, info, &mut px, &mut py, &mut unit)
    ));
    // oFFs
    let (mut ox, mut oy, mut ounit) = (0i32, 0i32, 0);
    log(format!(
        "oFFs rc={} x={ox} y={oy} unit={ounit}",
        (c.get_oFFs)(png, info, &mut ox, &mut oy, &mut ounit)
    ));
    // tIME
    let mut tp: *mut u8 = std::ptr::null_mut();
    let r = (c.get_tIME)(png, info, &mut tp);
    log(format!("tIME rc={r}"));
    if r != 0 && !tp.is_null() {
        let v = *(tp as *const PngTime);
        log(format!("tIME v={v:?}"));
    }
    // pCAL
    let mut purpose: *mut c_char = std::ptr::null_mut();
    let (mut x0, mut x1) = (0i32, 0i32);
    let (mut etype, mut nparams) = (0, 0);
    let mut units: *mut c_char = std::ptr::null_mut();
    let mut params: *mut *mut c_char = std::ptr::null_mut();
    let r = (c.get_pCAL)(
        png,
        info,
        &mut purpose,
        &mut x0,
        &mut x1,
        &mut etype,
        &mut nparams,
        &mut units,
        &mut params,
    );
    log(format!(
        "pCAL rc={r} purpose={} x0={x0} x1={x1} type={etype} nparams={nparams} units={}",
        cstr(purpose),
        cstr(units)
    ));
    if r != 0 && !params.is_null() {
        for i in 0..nparams as isize {
            log(format!("pCAL param[{i}]={}", cstr(*params.offset(i))));
        }
    }
    // sCAL
    let mut sunit: c_int = -1;
    let mut sw: *mut c_char = std::ptr::null_mut();
    let mut sh: *mut c_char = std::ptr::null_mut();
    let r = (c.get_sCAL_s)(png, info, &mut sunit, &mut sw, &mut sh);
    log(format!(
        "sCAL rc={r} unit={sunit} w={} h={}",
        cstr(sw),
        cstr(sh)
    ));
    // sPLT
    let mut splt: *mut c_void = std::ptr::null_mut();
    let n = (c.get_sPLT)(png, info, &mut splt);
    log(format!("sPLT n={n}"));
    if n > 0 && !splt.is_null() {
        let arr = std::slice::from_raw_parts(splt as *const PngSpltT, n as usize);
        for (i, e) in arr.iter().enumerate() {
            log(format!(
                "sPLT[{i}] name={} depth={} nentries={}",
                cstr(e.name),
                e.depth,
                e.nentries
            ));
            if !e.entries.is_null() && e.nentries > 0 {
                let ents = std::slice::from_raw_parts(e.entries, e.nentries as usize);
                log(format!("sPLT[{i}] entries={ents:?}"));
            }
        }
    }
    // eXIf
    let mut exif: *mut u8 = std::ptr::null_mut();
    let mut elen: u32 = 0;
    let r = (c.get_eXIf_1)(png, info, &mut elen, &mut exif);
    log(format!("eXIf rc={r} len={elen}"));
    if r != 0 && !exif.is_null() && elen > 0 {
        log(format!(
            "eXIf data={}",
            hex(std::slice::from_raw_parts(exif, elen as usize))
        ));
    }
    // cICP
    let (mut cp, mut tf, mut mc, mut vfr) = (0u8, 0u8, 0u8, 0u8);
    log(format!(
        "cICP rc={} p={cp} t={tf} m={mc} f={vfr}",
        (c.get_cICP)(png, info, &mut cp, &mut tf, &mut mc, &mut vfr)
    ));
    // cLLI
    let (mut maxcll, mut maxfall) = (0u32, 0u32);
    log(format!(
        "cLLI rc={} maxCLL={maxcll} maxFALL={maxfall}",
        (c.get_cLLI_fixed)(png, info, &mut maxcll, &mut maxfall)
    ));
    // mDCV
    let mut mx = [0i32; 8];
    let mut ml = [0u32; 2];
    let r = (c.get_mDCV_fixed)(
        png, info, &mut mx[0], &mut mx[1], &mut mx[2], &mut mx[3], &mut mx[4], &mut mx[5],
        &mut mx[6], &mut mx[7], &mut ml[0], &mut ml[1],
    );
    log(format!("mDCV rc={r} {mx:?} {ml:?}"));
    // text
    let mut tptr: *mut c_void = std::ptr::null_mut();
    let mut ntext: c_int = 0;
    let n = (c.get_text)(png, info, &mut tptr, &mut ntext);
    log(format!("text n={n} num={ntext}"));
    if n > 0 && !tptr.is_null() {
        let arr = std::slice::from_raw_parts(tptr as *const PngText, n as usize);
        for (i, t) in arr.iter().enumerate() {
            log(format!(
                "text[{i}] comp={} key={} text={} tlen={} ilen={} lang={} langkey={}",
                t.compression,
                cstr(t.key),
                cstr(t.text),
                t.text_length,
                t.itxt_length,
                cstr(t.lang),
                cstr(t.lang_key)
            ));
        }
    }
    // unknown chunks
    let mut uptr: *mut c_void = std::ptr::null_mut();
    let n = (c.get_unknown_chunks)(png, info, &mut uptr);
    log(format!("unknown n={n}"));
    if n > 0 && !uptr.is_null() {
        let arr = std::slice::from_raw_parts(uptr as *const PngUnknownChunk, n as usize);
        for (i, u) in arr.iter().enumerate() {
            let nm = String::from_utf8_lossy(&u.name[..4]).into_owned();
            log(format!(
                "unknown[{i}] name={nm} size={} loc={} data={}",
                u.size,
                u.location,
                if u.data.is_null() {
                    "<null>".to_string()
                } else {
                    hex(std::slice::from_raw_parts(u.data, u.size))
                }
            ));
        }
    }
    let _ = valid;
}
