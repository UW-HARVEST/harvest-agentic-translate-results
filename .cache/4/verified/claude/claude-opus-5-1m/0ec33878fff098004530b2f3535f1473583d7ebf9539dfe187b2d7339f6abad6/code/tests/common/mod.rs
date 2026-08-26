//! Differential test harness: loads BOTH the C `libmujs.so` and the Rust
//! `libmujs.so` through `libloading` and calls every entry point purely
//! through the exported FFI symbols.
#![allow(dead_code, non_snake_case, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_int, c_short, c_uint, c_ushort, c_void, CStr, CString};
use std::path::PathBuf;

pub type JS = *mut c_void; // js_State *
pub type Rune = c_int;

pub type js_Alloc = Option<unsafe extern "C" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>;
pub type js_CFunction = Option<unsafe extern "C" fn(JS)>;
pub type js_Finalize = Option<unsafe extern "C" fn(JS, *mut c_void)>;
pub type js_Report = Option<unsafe extern "C" fn(JS, *const c_char)>;
pub type js_Panic = Option<unsafe extern "C" fn(JS)>;
pub type js_HasProperty = Option<unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> c_int>;
pub type js_Put = Option<unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> c_int>;
pub type js_Delete = Option<unsafe extern "C" fn(JS, *mut c_void, *const c_char) -> c_int>;

/* ------------------------------------------------------------ constants */

pub const JS_STRICT: c_int = 1;

pub const JS_REGEXP_G: c_int = 1;
pub const JS_REGEXP_I: c_int = 2;
pub const JS_REGEXP_M: c_int = 4;

pub const JS_READONLY: c_int = 1;
pub const JS_DONTENUM: c_int = 2;
pub const JS_DONTCONF: c_int = 4;

pub const JS_ISUNDEFINED: c_int = 0;
pub const JS_ISNULL: c_int = 1;
pub const JS_ISBOOLEAN: c_int = 2;
pub const JS_ISNUMBER: c_int = 3;
pub const JS_ISSTRING: c_int = 4;
pub const JS_ISFUNCTION: c_int = 5;
pub const JS_ISOBJECT: c_int = 6;

pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
pub const REG_NOTBOL: c_int = 4;

pub const REG_MAXSUB: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResubSub {
    pub sp: *const c_char,
    pub ep: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [ResubSub; REG_MAXSUB],
}

impl Default for Resub {
    fn default() -> Self {
        Resub {
            nsub: 0,
            sub: [ResubSub {
                sp: std::ptr::null(),
                ep: std::ptr::null(),
            }; REG_MAXSUB],
        }
    }
}

/* --------------------------------------------------------------- paths */

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    crate_root().join("c_src/build/libmujs.so")
}

/// The Rust cdylib. We deliberately load it through `libloading` rather than
/// linking it, so every call goes through the real `#[no_mangle]` exports.
pub fn rust_so_path() -> PathBuf {
    // OUT_DIR-free discovery: the test binary lives in target/<profile>/deps/
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().unwrap().to_path_buf();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let p = dir.join("libmujs.so");
    let p = if p.exists() {
        p
    } else {
        // fall back to the usual debug location
        crate_root().join("target/debug/libmujs.so")
    };
    assert_stale_free(&p);
    p
}

/// GUARD AGAINST FALSE PASSES.
///
/// `cargo test` does NOT rebuild the cdylib: the integration-test targets do not
/// depend on the `mujs` lib target (they reach it only through `dlopen`), so
/// cargo has no reason to rebuild `target/debug/libmujs.so`. Without this check,
/// editing `src/*.rs` and running `cargo test` silently tests the PREVIOUS
/// build, and every test passes no matter how broken the new code is.
///
/// Always run `cargo build` (or `cargo build && cargo test`) first. This
/// assertion turns a silent false pass into a loud failure.
fn assert_stale_free(so: &PathBuf) {
    let so_mtime = std::fs::metadata(so)
        .unwrap_or_else(|e| panic!("{}: {e}\nRun `cargo build` first.", so.display()))
        .modified()
        .expect("mtime");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let src = crate_root().join("src");
    for e in std::fs::read_dir(&src).expect("read src/") {
        let e = e.expect("dir entry");
        let path = e.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let m = e.metadata().expect("metadata").modified().expect("mtime");
        if newest.as_ref().is_none_or(|(_, t)| m > *t) {
            newest = Some((path, m));
        }
    }
    let (newest_path, newest_mtime) = newest.expect("no .rs files in src/");
    assert!(
        newest_mtime <= so_mtime,
        "STALE Rust cdylib: {} is newer than {}.\n\
         `cargo test` does not rebuild the cdylib, so this run would have tested \
         the previous build and passed vacuously.\n\
         Run ./run_tests.sh (or `cargo build --no-default-features`) first.",
        newest_path.display(),
        so.display()
    );

    // mtime alone is not enough: a tool that rewrites src/ with BACKDATED mtimes
    // (e.g. shutil.copytree, tar -p, git checkout) leaves the .so newer than the
    // sources while being built from different code. `run_tests.sh` stamps a
    // content hash right after `cargo build`; require it to match.
    let stamp = so.parent().unwrap().join(".src_hash");
    if let Ok(recorded) = std::fs::read_to_string(&stamp) {
        let actual = src_hash();
        assert_eq!(
            recorded.trim(),
            actual,
            "STALE Rust cdylib: the recorded build hash in {} does not match the \
             current contents of src/*.rs.\n\
             The .so was built from DIFFERENT source than what is on disk, so this \
             run would have passed vacuously.\n\
             Run ./run_tests.sh to rebuild and re-stamp.",
            stamp.display()
        );
    }
}

/// FNV-1a 64 over every `src/*.rs`, sorted by file name, hashing
/// `name \0 len \0 bytes`. `run_tests.sh` computes the identical value.
pub fn src_hash() -> String {
    let src = crate_root().join("src");
    let mut names: Vec<String> = std::fs::read_dir(&src)
        .expect("read src/")
        .filter_map(|e| {
            let p = e.ok()?.path();
            if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                Some(p.file_name()?.to_str()?.to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
    };
    for n in &names {
        let bytes = std::fs::read(src.join(n)).expect("read src file");
        feed(n.as_bytes());
        feed(b"\0");
        feed(bytes.len().to_string().as_bytes());
        feed(b"\0");
        feed(&bytes);
    }
    format!("{h:016x}")
}

/* --------------------------------------------------------------- Lib */

pub struct Lib {
    pub name: &'static str,
    lib: Library,
}

macro_rules! sym {
    ($self:expr, $name:literal, $t:ty) => {{
        let s: Symbol<$t> = $self
            .lib
            .get(concat!($name, "\0").as_bytes())
            .unwrap_or_else(|e| panic!("{}: missing symbol {}: {}", $self.name, $name, e));
        *s
    }};
}

/// The C `libmujs.so` produced by CMakeLists.txt is not linked against libm
/// (`ceil`, `floor`, `fmod`, `sqrt` are undefined in it), so we must make libm
/// globally visible before dlopen'ing it. `c_src/` is read-only, so this has to
/// happen here in the harness.
fn preload_libm() {
    use libloading::os::unix::Library as UnixLibrary;
    use std::sync::OnceLock;
    static M: OnceLock<()> = OnceLock::new();
    M.get_or_init(|| {
        const RTLD_NOW: i32 = 0x2;
        const RTLD_GLOBAL: i32 = 0x100;
        for cand in ["libm.so.6", "libm.so"] {
            if let Ok(l) = unsafe { UnixLibrary::open(Some(cand), RTLD_NOW | RTLD_GLOBAL) } {
                // must never be dlclose()d, or it leaves the global scope again
                std::mem::forget(l);
                return;
            }
        }
        panic!("cannot preload libm");
    });
}

impl Lib {
    pub fn open(name: &'static str, path: &PathBuf) -> Lib {
        preload_libm();
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e));
        Lib { name, lib }
    }

    pub fn has(&self, name: &str) -> bool {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        unsafe { self.lib.get::<*mut c_void>(&b) }.is_ok()
    }

    /// Generic symbol fetch for the handful of entry points whose signatures
    /// don't fit the helper families above (e.g. `js_pushvalue`, which takes a
    /// 16-byte `js_Value` by value).
    pub unsafe fn raw2<T: Copy>(&self, name: &str) -> T {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<T> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        *s
    }
}

/* Every wrapper below is `unsafe` and looks the symbol up on each call. */
impl Lib {
    /* ---------------- state ---------------- */
    pub unsafe fn js_newstate(&self, alloc: js_Alloc, actx: *mut c_void, flags: c_int) -> JS {
        sym!(self, "js_newstate", unsafe extern "C" fn(js_Alloc, *mut c_void, c_int) -> JS)(
            alloc, actx, flags,
        )
    }
    pub unsafe fn js_freestate(&self, j: JS) {
        sym!(self, "js_freestate", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_setcontext(&self, j: JS, u: *mut c_void) {
        sym!(self, "js_setcontext", unsafe extern "C" fn(JS, *mut c_void))(j, u)
    }
    pub unsafe fn js_getcontext(&self, j: JS) -> *mut c_void {
        sym!(self, "js_getcontext", unsafe extern "C" fn(JS) -> *mut c_void)(j)
    }
    pub unsafe fn js_setreport(&self, j: JS, r: js_Report) {
        sym!(self, "js_setreport", unsafe extern "C" fn(JS, js_Report))(j, r)
    }
    pub unsafe fn js_atpanic(&self, j: JS, p: js_Panic) -> js_Panic {
        sym!(self, "js_atpanic", unsafe extern "C" fn(JS, js_Panic) -> js_Panic)(j, p)
    }
    pub unsafe fn js_report(&self, j: JS, m: *const c_char) {
        sym!(self, "js_report", unsafe extern "C" fn(JS, *const c_char))(j, m)
    }
    pub unsafe fn js_gc(&self, j: JS, report: c_int) {
        sym!(self, "js_gc", unsafe extern "C" fn(JS, c_int))(j, report)
    }
    pub unsafe fn js_setlimit(&self, j: JS, runlimit: c_int, memlimit: c_int) {
        sym!(self, "js_setlimit", unsafe extern "C" fn(JS, c_int, c_int))(j, runlimit, memlimit)
    }
    pub unsafe fn js_dostring(&self, j: JS, src: *const c_char) -> c_int {
        sym!(self, "js_dostring", unsafe extern "C" fn(JS, *const c_char) -> c_int)(j, src)
    }
    pub unsafe fn js_ploadstring(&self, j: JS, f: *const c_char, s: *const c_char) -> c_int {
        sym!(
            self,
            "js_ploadstring",
            unsafe extern "C" fn(JS, *const c_char, *const c_char) -> c_int
        )(j, f, s)
    }
    pub unsafe fn js_pcall(&self, j: JS, n: c_int) -> c_int {
        sym!(self, "js_pcall", unsafe extern "C" fn(JS, c_int) -> c_int)(j, n)
    }
    pub unsafe fn js_pconstruct(&self, j: JS, n: c_int) -> c_int {
        sym!(self, "js_pconstruct", unsafe extern "C" fn(JS, c_int) -> c_int)(j, n)
    }
    pub unsafe fn js_call(&self, j: JS, n: c_int) {
        sym!(self, "js_call", unsafe extern "C" fn(JS, c_int))(j, n)
    }
    pub unsafe fn js_construct(&self, j: JS, n: c_int) {
        sym!(self, "js_construct", unsafe extern "C" fn(JS, c_int))(j, n)
    }
    pub unsafe fn js_eval(&self, j: JS) {
        sym!(self, "js_eval", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_loadstring(&self, j: JS, f: *const c_char, s: *const c_char) {
        sym!(
            self,
            "js_loadstring",
            unsafe extern "C" fn(JS, *const c_char, *const c_char)
        )(j, f, s)
    }
    pub unsafe fn js_loadeval(&self, j: JS, f: *const c_char, s: *const c_char) {
        sym!(
            self,
            "js_loadeval",
            unsafe extern "C" fn(JS, *const c_char, *const c_char)
        )(j, f, s)
    }
    pub unsafe fn js_endtry(&self, j: JS) {
        sym!(self, "js_endtry", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_savetry(&self, j: JS) -> *mut c_void {
        sym!(self, "js_savetry", unsafe extern "C" fn(JS) -> *mut c_void)(j)
    }
    pub unsafe fn js_throw(&self, j: JS) {
        sym!(self, "js_throw", unsafe extern "C" fn(JS))(j)
    }

    /* ---------------- refs / registry / globals ---------------- */
    pub unsafe fn js_ref(&self, j: JS) -> *const c_char {
        sym!(self, "js_ref", unsafe extern "C" fn(JS) -> *const c_char)(j)
    }
    pub unsafe fn js_unref(&self, j: JS, r: *const c_char) {
        sym!(self, "js_unref", unsafe extern "C" fn(JS, *const c_char))(j, r)
    }
    pub unsafe fn js_getregistry(&self, j: JS, n: *const c_char) {
        sym!(self, "js_getregistry", unsafe extern "C" fn(JS, *const c_char))(j, n)
    }
    pub unsafe fn js_setregistry(&self, j: JS, n: *const c_char) {
        sym!(self, "js_setregistry", unsafe extern "C" fn(JS, *const c_char))(j, n)
    }
    pub unsafe fn js_delregistry(&self, j: JS, n: *const c_char) {
        sym!(self, "js_delregistry", unsafe extern "C" fn(JS, *const c_char))(j, n)
    }
    pub unsafe fn js_getglobal(&self, j: JS, n: *const c_char) {
        sym!(self, "js_getglobal", unsafe extern "C" fn(JS, *const c_char))(j, n)
    }
    pub unsafe fn js_setglobal(&self, j: JS, n: *const c_char) {
        sym!(self, "js_setglobal", unsafe extern "C" fn(JS, *const c_char))(j, n)
    }
    pub unsafe fn js_defglobal(&self, j: JS, n: *const c_char, atts: c_int) {
        sym!(
            self,
            "js_defglobal",
            unsafe extern "C" fn(JS, *const c_char, c_int)
        )(j, n, atts)
    }
    pub unsafe fn js_delglobal(&self, j: JS, n: *const c_char) {
        sym!(self, "js_delglobal", unsafe extern "C" fn(JS, *const c_char))(j, n)
    }

    /* ---------------- properties ---------------- */
    pub unsafe fn js_hasproperty(&self, j: JS, idx: c_int, n: *const c_char) -> c_int {
        sym!(
            self,
            "js_hasproperty",
            unsafe extern "C" fn(JS, c_int, *const c_char) -> c_int
        )(j, idx, n)
    }
    pub unsafe fn js_getproperty(&self, j: JS, idx: c_int, n: *const c_char) {
        sym!(
            self,
            "js_getproperty",
            unsafe extern "C" fn(JS, c_int, *const c_char)
        )(j, idx, n)
    }
    pub unsafe fn js_setproperty(&self, j: JS, idx: c_int, n: *const c_char) {
        sym!(
            self,
            "js_setproperty",
            unsafe extern "C" fn(JS, c_int, *const c_char)
        )(j, idx, n)
    }
    pub unsafe fn js_defproperty(&self, j: JS, idx: c_int, n: *const c_char, atts: c_int) {
        sym!(
            self,
            "js_defproperty",
            unsafe extern "C" fn(JS, c_int, *const c_char, c_int)
        )(j, idx, n, atts)
    }
    pub unsafe fn js_delproperty(&self, j: JS, idx: c_int, n: *const c_char) {
        sym!(
            self,
            "js_delproperty",
            unsafe extern "C" fn(JS, c_int, *const c_char)
        )(j, idx, n)
    }
    pub unsafe fn js_defaccessor(&self, j: JS, idx: c_int, n: *const c_char, atts: c_int) {
        sym!(
            self,
            "js_defaccessor",
            unsafe extern "C" fn(JS, c_int, *const c_char, c_int)
        )(j, idx, n, atts)
    }
    pub unsafe fn js_getlength(&self, j: JS, idx: c_int) -> c_int {
        sym!(self, "js_getlength", unsafe extern "C" fn(JS, c_int) -> c_int)(j, idx)
    }
    pub unsafe fn js_setlength(&self, j: JS, idx: c_int, len: c_int) {
        sym!(self, "js_setlength", unsafe extern "C" fn(JS, c_int, c_int))(j, idx, len)
    }
    pub unsafe fn js_hasindex(&self, j: JS, idx: c_int, i: c_int) -> c_int {
        sym!(
            self,
            "js_hasindex",
            unsafe extern "C" fn(JS, c_int, c_int) -> c_int
        )(j, idx, i)
    }
    pub unsafe fn js_getindex(&self, j: JS, idx: c_int, i: c_int) {
        sym!(self, "js_getindex", unsafe extern "C" fn(JS, c_int, c_int))(j, idx, i)
    }
    pub unsafe fn js_setindex(&self, j: JS, idx: c_int, i: c_int) {
        sym!(self, "js_setindex", unsafe extern "C" fn(JS, c_int, c_int))(j, idx, i)
    }
    pub unsafe fn js_delindex(&self, j: JS, idx: c_int, i: c_int) {
        sym!(self, "js_delindex", unsafe extern "C" fn(JS, c_int, c_int))(j, idx, i)
    }

    /* ---------------- push / new ---------------- */
    pub unsafe fn js_currentfunction(&self, j: JS) {
        sym!(self, "js_currentfunction", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_currentfunctiondata(&self, j: JS) -> *mut c_void {
        sym!(
            self,
            "js_currentfunctiondata",
            unsafe extern "C" fn(JS) -> *mut c_void
        )(j)
    }
    pub unsafe fn js_pushglobal(&self, j: JS) {
        sym!(self, "js_pushglobal", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_pushundefined(&self, j: JS) {
        sym!(self, "js_pushundefined", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_pushnull(&self, j: JS) {
        sym!(self, "js_pushnull", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_pushboolean(&self, j: JS, v: c_int) {
        sym!(self, "js_pushboolean", unsafe extern "C" fn(JS, c_int))(j, v)
    }
    pub unsafe fn js_pushnumber(&self, j: JS, v: f64) {
        sym!(self, "js_pushnumber", unsafe extern "C" fn(JS, f64))(j, v)
    }
    pub unsafe fn js_pushstring(&self, j: JS, v: *const c_char) {
        sym!(self, "js_pushstring", unsafe extern "C" fn(JS, *const c_char))(j, v)
    }
    pub unsafe fn js_pushlstring(&self, j: JS, v: *const c_char, n: c_int) {
        sym!(
            self,
            "js_pushlstring",
            unsafe extern "C" fn(JS, *const c_char, c_int)
        )(j, v, n)
    }
    pub unsafe fn js_pushliteral(&self, j: JS, v: *const c_char) {
        sym!(self, "js_pushliteral", unsafe extern "C" fn(JS, *const c_char))(j, v)
    }
    pub unsafe fn js_newobjectx(&self, j: JS) {
        sym!(self, "js_newobjectx", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_newobject(&self, j: JS) {
        sym!(self, "js_newobject", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_newarray(&self, j: JS) {
        sym!(self, "js_newarray", unsafe extern "C" fn(JS))(j)
    }
    pub unsafe fn js_newboolean(&self, j: JS, v: c_int) {
        sym!(self, "js_newboolean", unsafe extern "C" fn(JS, c_int))(j, v)
    }
    pub unsafe fn js_newnumber(&self, j: JS, v: f64) {
        sym!(self, "js_newnumber", unsafe extern "C" fn(JS, f64))(j, v)
    }
    pub unsafe fn js_newstring(&self, j: JS, v: *const c_char) {
        sym!(self, "js_newstring", unsafe extern "C" fn(JS, *const c_char))(j, v)
    }
    pub unsafe fn js_newcfunction(
        &self,
        j: JS,
        f: js_CFunction,
        name: *const c_char,
        length: c_int,
    ) {
        sym!(
            self,
            "js_newcfunction",
            unsafe extern "C" fn(JS, js_CFunction, *const c_char, c_int)
        )(j, f, name, length)
    }
    pub unsafe fn js_newcfunctionx(
        &self,
        j: JS,
        f: js_CFunction,
        name: *const c_char,
        length: c_int,
        data: *mut c_void,
        fin: js_Finalize,
    ) {
        sym!(
            self,
            "js_newcfunctionx",
            unsafe extern "C" fn(JS, js_CFunction, *const c_char, c_int, *mut c_void, js_Finalize)
        )(j, f, name, length, data, fin)
    }
    pub unsafe fn js_newcconstructor(
        &self,
        j: JS,
        f: js_CFunction,
        c: js_CFunction,
        name: *const c_char,
        length: c_int,
    ) {
        sym!(
            self,
            "js_newcconstructor",
            unsafe extern "C" fn(JS, js_CFunction, js_CFunction, *const c_char, c_int)
        )(j, f, c, name, length)
    }
    pub unsafe fn js_newuserdata(
        &self,
        j: JS,
        tag: *const c_char,
        data: *mut c_void,
        fin: js_Finalize,
    ) {
        sym!(
            self,
            "js_newuserdata",
            unsafe extern "C" fn(JS, *const c_char, *mut c_void, js_Finalize)
        )(j, tag, data, fin)
    }
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn js_newuserdatax(
        &self,
        j: JS,
        tag: *const c_char,
        data: *mut c_void,
        has: js_HasProperty,
        put: js_Put,
        del: js_Delete,
        fin: js_Finalize,
    ) {
        sym!(
            self,
            "js_newuserdatax",
            unsafe extern "C" fn(
                JS,
                *const c_char,
                *mut c_void,
                js_HasProperty,
                js_Put,
                js_Delete,
                js_Finalize,
            )
        )(j, tag, data, has, put, del, fin)
    }
    pub unsafe fn js_newregexp(&self, j: JS, pattern: *const c_char, flags: c_int) {
        sym!(
            self,
            "js_newregexp",
            unsafe extern "C" fn(JS, *const c_char, c_int)
        )(j, pattern, flags)
    }

    /* ---------------- iterators ---------------- */
    pub unsafe fn js_pushiterator(&self, j: JS, idx: c_int, own: c_int) {
        sym!(self, "js_pushiterator", unsafe extern "C" fn(JS, c_int, c_int))(j, idx, own)
    }
    pub unsafe fn js_nextiterator(&self, j: JS, idx: c_int) -> *const c_char {
        sym!(
            self,
            "js_nextiterator",
            unsafe extern "C" fn(JS, c_int) -> *const c_char
        )(j, idx)
    }

    /* ---------------- predicates ---------------- */
    pub unsafe fn pred(&self, name: &'static str, j: JS, idx: c_int) -> c_int {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(JS, c_int) -> c_int> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(j, idx)
    }
    pub unsafe fn js_isuserdata(&self, j: JS, idx: c_int, tag: *const c_char) -> c_int {
        sym!(
            self,
            "js_isuserdata",
            unsafe extern "C" fn(JS, c_int, *const c_char) -> c_int
        )(j, idx, tag)
    }

    /* ---------------- conversions ---------------- */
    pub unsafe fn js_toboolean(&self, j: JS, idx: c_int) -> c_int {
        sym!(self, "js_toboolean", unsafe extern "C" fn(JS, c_int) -> c_int)(j, idx)
    }
    pub unsafe fn js_tonumber(&self, j: JS, idx: c_int) -> f64 {
        sym!(self, "js_tonumber", unsafe extern "C" fn(JS, c_int) -> f64)(j, idx)
    }
    pub unsafe fn js_tostring(&self, j: JS, idx: c_int) -> *const c_char {
        sym!(
            self,
            "js_tostring",
            unsafe extern "C" fn(JS, c_int) -> *const c_char
        )(j, idx)
    }
    pub unsafe fn js_touserdata(&self, j: JS, idx: c_int, tag: *const c_char) -> *mut c_void {
        sym!(
            self,
            "js_touserdata",
            unsafe extern "C" fn(JS, c_int, *const c_char) -> *mut c_void
        )(j, idx, tag)
    }
    pub unsafe fn js_trystring(
        &self,
        j: JS,
        idx: c_int,
        err: *const c_char,
    ) -> *const c_char {
        sym!(
            self,
            "js_trystring",
            unsafe extern "C" fn(JS, c_int, *const c_char) -> *const c_char
        )(j, idx, err)
    }
    pub unsafe fn js_trynumber(&self, j: JS, idx: c_int, err: f64) -> f64 {
        sym!(
            self,
            "js_trynumber",
            unsafe extern "C" fn(JS, c_int, f64) -> f64
        )(j, idx, err)
    }
    pub unsafe fn js_tryinteger(&self, j: JS, idx: c_int, err: c_int) -> c_int {
        sym!(
            self,
            "js_tryinteger",
            unsafe extern "C" fn(JS, c_int, c_int) -> c_int
        )(j, idx, err)
    }
    pub unsafe fn js_tryboolean(&self, j: JS, idx: c_int, err: c_int) -> c_int {
        sym!(
            self,
            "js_tryboolean",
            unsafe extern "C" fn(JS, c_int, c_int) -> c_int
        )(j, idx, err)
    }
    pub unsafe fn js_tointeger(&self, j: JS, idx: c_int) -> c_int {
        sym!(self, "js_tointeger", unsafe extern "C" fn(JS, c_int) -> c_int)(j, idx)
    }
    pub unsafe fn js_toint32(&self, j: JS, idx: c_int) -> c_int {
        sym!(self, "js_toint32", unsafe extern "C" fn(JS, c_int) -> c_int)(j, idx)
    }
    pub unsafe fn js_touint32(&self, j: JS, idx: c_int) -> c_uint {
        sym!(self, "js_touint32", unsafe extern "C" fn(JS, c_int) -> c_uint)(j, idx)
    }
    pub unsafe fn js_toint16(&self, j: JS, idx: c_int) -> c_short {
        sym!(self, "js_toint16", unsafe extern "C" fn(JS, c_int) -> c_short)(j, idx)
    }
    pub unsafe fn js_touint16(&self, j: JS, idx: c_int) -> c_ushort {
        sym!(self, "js_touint16", unsafe extern "C" fn(JS, c_int) -> c_ushort)(j, idx)
    }

    /* ---------------- stack ---------------- */
    pub unsafe fn js_gettop(&self, j: JS) -> c_int {
        sym!(self, "js_gettop", unsafe extern "C" fn(JS) -> c_int)(j)
    }
    pub unsafe fn js_pop(&self, j: JS, n: c_int) {
        sym!(self, "js_pop", unsafe extern "C" fn(JS, c_int))(j, n)
    }
    pub unsafe fn js_rot(&self, j: JS, n: c_int) {
        sym!(self, "js_rot", unsafe extern "C" fn(JS, c_int))(j, n)
    }
    pub unsafe fn js_copy(&self, j: JS, idx: c_int) {
        sym!(self, "js_copy", unsafe extern "C" fn(JS, c_int))(j, idx)
    }
    pub unsafe fn js_remove(&self, j: JS, idx: c_int) {
        sym!(self, "js_remove", unsafe extern "C" fn(JS, c_int))(j, idx)
    }
    pub unsafe fn js_insert(&self, j: JS, idx: c_int) {
        sym!(self, "js_insert", unsafe extern "C" fn(JS, c_int))(j, idx)
    }
    pub unsafe fn js_replace(&self, j: JS, idx: c_int) {
        sym!(self, "js_replace", unsafe extern "C" fn(JS, c_int))(j, idx)
    }
    pub unsafe fn nullary(&self, name: &'static str, j: JS) {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(JS)> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(j)
    }
    pub unsafe fn nullary_i(&self, name: &'static str, j: JS) -> c_int {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(JS) -> c_int> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(j)
    }

    /* ---------------- comparison / misc ---------------- */
    pub unsafe fn js_compare(&self, j: JS, okay: *mut c_int) -> c_int {
        sym!(
            self,
            "js_compare",
            unsafe extern "C" fn(JS, *mut c_int) -> c_int
        )(j, okay)
    }
    pub unsafe fn js_typeof(&self, j: JS, idx: c_int) -> *const c_char {
        sym!(
            self,
            "js_typeof",
            unsafe extern "C" fn(JS, c_int) -> *const c_char
        )(j, idx)
    }
    pub unsafe fn js_type(&self, j: JS, idx: c_int) -> c_int {
        sym!(self, "js_type", unsafe extern "C" fn(JS, c_int) -> c_int)(j, idx)
    }
    pub unsafe fn js_repr(&self, j: JS, idx: c_int) {
        sym!(self, "js_repr", unsafe extern "C" fn(JS, c_int))(j, idx)
    }
    pub unsafe fn js_torepr(&self, j: JS, idx: c_int) -> *const c_char {
        sym!(
            self,
            "js_torepr",
            unsafe extern "C" fn(JS, c_int) -> *const c_char
        )(j, idx)
    }
    pub unsafe fn js_tryrepr(&self, j: JS, idx: c_int, err: *const c_char) -> *const c_char {
        sym!(
            self,
            "js_tryrepr",
            unsafe extern "C" fn(JS, c_int, *const c_char) -> *const c_char
        )(j, idx, err)
    }

    /* ---------------- error constructors (non-throwing) ---------------- */
    pub unsafe fn newerror(&self, name: &'static str, j: JS, msg: *const c_char) {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(JS, *const c_char)> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(j, msg)
    }
    /// varargs error thrower - only safe inside a protected call
    pub unsafe fn throwerror(&self, name: &'static str, j: JS, fmt: *const c_char) {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(JS, *const c_char)> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(j, fmt)
    }
    pub unsafe fn throwerror_s(&self, name: &'static str, j: JS, fmt: *const c_char, a: *const c_char) {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(JS, *const c_char, *const c_char)> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(j, fmt, a)
    }
    pub unsafe fn throwerror_d(&self, name: &'static str, j: JS, fmt: *const c_char, a: c_int) {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(JS, *const c_char, c_int)> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(j, fmt, a)
    }

    /* ---------------- low level: numbers ---------------- */
    pub unsafe fn js_itoa(&self, buf: *mut c_char, a: c_int) -> *const c_char {
        sym!(
            self,
            "js_itoa",
            unsafe extern "C" fn(*mut c_char, c_int) -> *const c_char
        )(buf, a)
    }
    pub unsafe fn js_strtod(&self, s: *const c_char, ep: *mut *mut c_char) -> f64 {
        sym!(
            self,
            "js_strtod",
            unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64
        )(s, ep)
    }
    pub unsafe fn js_strtol(&self, s: *const c_char, ep: *mut *mut c_char, radix: c_int) -> f64 {
        sym!(
            self,
            "js_strtol",
            unsafe extern "C" fn(*const c_char, *mut *mut c_char, c_int) -> f64
        )(s, ep, radix)
    }
    pub unsafe fn js_stringtofloat(&self, s: *const c_char, ep: *mut *mut c_char) -> f64 {
        sym!(
            self,
            "js_stringtofloat",
            unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64
        )(s, ep)
    }
    pub unsafe fn js_grisu2(&self, v: f64, buf: *mut c_char, k: *mut c_int) -> c_int {
        sym!(
            self,
            "js_grisu2",
            unsafe extern "C" fn(f64, *mut c_char, *mut c_int) -> c_int
        )(v, buf, k)
    }
    pub unsafe fn js_fmtexp(&self, p: *mut c_char, e: c_int) {
        sym!(self, "js_fmtexp", unsafe extern "C" fn(*mut c_char, c_int))(p, e)
    }
    pub unsafe fn jsV_numbertointeger(&self, n: f64) -> c_int {
        sym!(self, "jsV_numbertointeger", unsafe extern "C" fn(f64) -> c_int)(n)
    }
    pub unsafe fn jsV_numbertoint32(&self, n: f64) -> c_int {
        sym!(self, "jsV_numbertoint32", unsafe extern "C" fn(f64) -> c_int)(n)
    }
    pub unsafe fn jsV_numbertouint32(&self, n: f64) -> c_uint {
        sym!(self, "jsV_numbertouint32", unsafe extern "C" fn(f64) -> c_uint)(n)
    }
    pub unsafe fn jsV_numbertoint16(&self, n: f64) -> c_short {
        sym!(self, "jsV_numbertoint16", unsafe extern "C" fn(f64) -> c_short)(n)
    }
    pub unsafe fn jsV_numbertouint16(&self, n: f64) -> c_ushort {
        sym!(
            self,
            "jsV_numbertouint16",
            unsafe extern "C" fn(f64) -> c_ushort
        )(n)
    }
    pub unsafe fn jsV_numbertostring(&self, j: JS, buf: *mut c_char, n: f64) -> *const c_char {
        sym!(
            self,
            "jsV_numbertostring",
            unsafe extern "C" fn(JS, *mut c_char, f64) -> *const c_char
        )(j, buf, n)
    }
    pub unsafe fn jsV_stringtonumber(&self, j: JS, s: *const c_char) -> f64 {
        sym!(
            self,
            "jsV_stringtonumber",
            unsafe extern "C" fn(JS, *const c_char) -> f64
        )(j, s)
    }

    /* ---------------- low level: utf ---------------- */
    pub unsafe fn jsU_chartorune(&self, rune: *mut Rune, s: *const c_char) -> c_int {
        sym!(
            self,
            "jsU_chartorune",
            unsafe extern "C" fn(*mut Rune, *const c_char) -> c_int
        )(rune, s)
    }
    pub unsafe fn jsU_runetochar(&self, s: *mut c_char, rune: *const Rune) -> c_int {
        sym!(
            self,
            "jsU_runetochar",
            unsafe extern "C" fn(*mut c_char, *const Rune) -> c_int
        )(s, rune)
    }
    pub unsafe fn jsU_runelen(&self, c: c_int) -> c_int {
        sym!(self, "jsU_runelen", unsafe extern "C" fn(c_int) -> c_int)(c)
    }
    pub unsafe fn rune_pred(&self, name: &'static str, c: Rune) -> c_int {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(Rune) -> c_int> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(c)
    }
    pub unsafe fn rune_full(&self, name: &'static str, c: Rune) -> *const Rune {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(Rune) -> *const Rune> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(c)
    }
    pub unsafe fn js_utflen(&self, s: *const c_char) -> c_int {
        sym!(self, "js_utflen", unsafe extern "C" fn(*const c_char) -> c_int)(s)
    }
    pub unsafe fn js_utfptrtoidx(&self, s: *const c_char, p: *const c_char) -> c_int {
        sym!(
            self,
            "js_utfptrtoidx",
            unsafe extern "C" fn(*const c_char, *const c_char) -> c_int
        )(s, p)
    }
    pub unsafe fn js_runeat(&self, j: JS, s: *const c_char, i: c_int) -> c_int {
        sym!(
            self,
            "js_runeat",
            unsafe extern "C" fn(JS, *const c_char, c_int) -> c_int
        )(j, s, i)
    }
    pub unsafe fn js_isarrayindex(&self, j: JS, s: *const c_char, idx: *mut c_int) -> c_int {
        sym!(
            self,
            "js_isarrayindex",
            unsafe extern "C" fn(JS, *const c_char, *mut c_int) -> c_int
        )(j, s, idx)
    }

    /* ---------------- low level: lexer helpers ---------------- */
    pub unsafe fn int_pred(&self, name: &'static str, c: c_int) -> c_int {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: Symbol<unsafe extern "C" fn(c_int) -> c_int> = self
            .lib
            .get(&b)
            .unwrap_or_else(|e| panic!("{}: missing {}: {}", self.name, name, e));
        (*s)(c)
    }
    pub unsafe fn jsY_tokenstring(&self, t: c_int) -> *const c_char {
        sym!(
            self,
            "jsY_tokenstring",
            unsafe extern "C" fn(c_int) -> *const c_char
        )(t)
    }
    pub unsafe fn jsY_findword(
        &self,
        s: *const c_char,
        list: *const *const c_char,
        num: c_int,
    ) -> c_int {
        sym!(
            self,
            "jsY_findword",
            unsafe extern "C" fn(*const c_char, *const *const c_char, c_int) -> c_int
        )(s, list, num)
    }

    /* ---------------- low level: regexp ---------------- */
    pub unsafe fn js_regcomp(
        &self,
        pattern: *const c_char,
        cflags: c_int,
        errorp: *mut *const c_char,
    ) -> *mut c_void {
        sym!(
            self,
            "js_regcomp",
            unsafe extern "C" fn(*const c_char, c_int, *mut *const c_char) -> *mut c_void
        )(pattern, cflags, errorp)
    }
    pub unsafe fn js_regexec(
        &self,
        prog: *mut c_void,
        s: *const c_char,
        sub: *mut Resub,
        eflags: c_int,
    ) -> c_int {
        sym!(
            self,
            "js_regexec",
            unsafe extern "C" fn(*mut c_void, *const c_char, *mut Resub, c_int) -> c_int
        )(prog, s, sub, eflags)
    }
    pub unsafe fn js_regfree(&self, prog: *mut c_void) {
        sym!(self, "js_regfree", unsafe extern "C" fn(*mut c_void))(prog)
    }
    pub unsafe fn js_regcompx(
        &self,
        alloc: js_Alloc,
        ctx: *mut c_void,
        pattern: *const c_char,
        cflags: c_int,
        errorp: *mut *const c_char,
    ) -> *mut c_void {
        sym!(
            self,
            "js_regcompx",
            unsafe extern "C" fn(
                js_Alloc,
                *mut c_void,
                *const c_char,
                c_int,
                *mut *const c_char,
            ) -> *mut c_void
        )(alloc, ctx, pattern, cflags, errorp)
    }
    pub unsafe fn js_regfreex(&self, alloc: js_Alloc, ctx: *mut c_void, prog: *mut c_void) {
        sym!(
            self,
            "js_regfreex",
            unsafe extern "C" fn(js_Alloc, *mut c_void, *mut c_void)
        )(alloc, ctx, prog)
    }

    /* ---------------- low level: intern / memory ---------------- */
    pub unsafe fn js_intern(&self, j: JS, s: *const c_char) -> *const c_char {
        sym!(
            self,
            "js_intern",
            unsafe extern "C" fn(JS, *const c_char) -> *const c_char
        )(j, s)
    }
    pub unsafe fn js_strdup(&self, j: JS, s: *const c_char) -> *mut c_char {
        sym!(
            self,
            "js_strdup",
            unsafe extern "C" fn(JS, *const c_char) -> *mut c_char
        )(j, s)
    }
    pub unsafe fn js_malloc(&self, j: JS, n: c_int) -> *mut c_void {
        sym!(self, "js_malloc", unsafe extern "C" fn(JS, c_int) -> *mut c_void)(j, n)
    }
    pub unsafe fn js_realloc(&self, j: JS, p: *mut c_void, n: c_int) -> *mut c_void {
        sym!(
            self,
            "js_realloc",
            unsafe extern "C" fn(JS, *mut c_void, c_int) -> *mut c_void
        )(j, p, n)
    }
    pub unsafe fn js_free(&self, j: JS, p: *mut c_void) {
        sym!(self, "js_free", unsafe extern "C" fn(JS, *mut c_void))(j, p)
    }
    pub unsafe fn jsS_dumpstrings(&self, j: JS) {
        sym!(self, "jsS_dumpstrings", unsafe extern "C" fn(JS))(j)
    }
}

/* ------------------------------------------------------- the two libs */

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Lib::open("C", &c_so_path()),
        rs: Lib::open("RUST", &rust_so_path()),
    })
}

/* --------------------------------------------- current-library context */

thread_local! {
    static CUR: Cell<*const Lib> = const { Cell::new(std::ptr::null()) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

pub fn cur() -> &'static Lib {
    let p = CUR.with(|c| c.get());
    assert!(!p.is_null(), "no current library set");
    unsafe { &*p }
}

pub fn set_cur(l: &Lib) {
    CUR.with(|c| c.set(l as *const Lib));
}

pub fn out_clear() {
    OUT.with(|o| o.borrow_mut().clear());
}

pub fn out_push(s: &[u8]) {
    OUT.with(|o| o.borrow_mut().extend_from_slice(s));
}

pub fn out_take() -> String {
    OUT.with(|o| {
        let v = std::mem::take(&mut *o.borrow_mut());
        String::from_utf8_lossy(&v).into_owned()
    })
}

pub fn cstr(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap()
}

pub unsafe fn from_c(p: *const c_char) -> String {
    if p.is_null() {
        "<NULL>".to_string()
    } else {
        String::from_utf8_lossy(CStr::from_ptr(p).to_bytes()).into_owned()
    }
}

/// Format a double the way a test comparison wants it: exact bits, so that
/// -0.0 / NaN payloads / every last mantissa bit is compared.
pub fn fbits(x: f64) -> u64 {
    x.to_bits()
}

/* ------------------------------------------------------- script drivers */

pub unsafe extern "C" fn report_cb(_j: JS, msg: *const c_char) {
    out_push(b"[report] ");
    if !msg.is_null() {
        out_push(CStr::from_ptr(msg).to_bytes());
    } else {
        out_push(b"<NULL>");
    }
    out_push(b"\n");
}

/// `print(...)` - joins arguments with a space using js_tostring.
pub unsafe extern "C" fn print_cb(j: JS) {
    let l = cur();
    let top = l.js_gettop(j);
    for i in 1..top {
        if i > 1 {
            out_push(b" ");
        }
        let s = l.js_tostring(j, i);
        if s.is_null() {
            out_push(b"<NULL>");
        } else {
            out_push(CStr::from_ptr(s).to_bytes());
        }
    }
    out_push(b"\n");
    l.js_pushundefined(j);
}

/// `repr(x)` - pushes the js_torepr of argument 1 (used to compare values).
pub unsafe extern "C" fn repr_cb(j: JS) {
    let l = cur();
    let s = l.js_torepr(j, 1);
    l.js_pushstring(j, s);
}

/// `dump(...)` - like print but uses js_torepr for exact value shape.
pub unsafe extern "C" fn dump_cb(j: JS) {
    let l = cur();
    let top = l.js_gettop(j);
    for i in 1..top {
        if i > 1 {
            out_push(b" ");
        }
        let s = l.js_torepr(j, i);
        if s.is_null() {
            out_push(b"<NULL>");
        } else {
            out_push(CStr::from_ptr(s).to_bytes());
        }
    }
    out_push(b"\n");
    l.js_pushundefined(j);
}

pub const PRINT: *const c_char = b"print\0".as_ptr() as *const c_char;
pub const DUMP: *const c_char = b"dump\0".as_ptr() as *const c_char;
pub const ERRSTR: *const c_char = b"<throw>\0".as_ptr() as *const c_char;
pub const FILENAME: *const c_char = b"test.js\0".as_ptr() as *const c_char;

/// Create a fresh state with `print` / `dump` installed and the report hook
/// pointing into the per-thread output buffer.
pub unsafe fn new_state(l: &Lib, flags: c_int) -> JS {
    set_cur(l);
    let j = l.js_newstate(None, std::ptr::null_mut(), flags);
    assert!(!j.is_null(), "{}: js_newstate returned NULL", l.name);
    l.js_setreport(j, Some(report_cb));
    l.js_newcfunction(j, Some(print_cb), PRINT, 1);
    l.js_setglobal(j, PRINT);
    l.js_newcfunction(j, Some(dump_cb), DUMP, 1);
    l.js_setglobal(j, DUMP);
    j
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EvalResult {
    pub load_rc: c_int,
    pub call_rc: c_int,
    pub result: String,
    pub out: String,
    pub top: c_int,
}

/// Full pipeline through the low level entry points:
/// js_ploadstring -> js_pushundefined -> js_pcall -> js_tryrepr
pub fn eval(l: &Lib, flags: c_int, src: &str) -> EvalResult {
    unsafe {
        out_clear();
        let j = new_state(l, flags);
        let cs = cstr(src);
        let load_rc = l.js_ploadstring(j, FILENAME, cs.as_ptr());
        let mut call_rc = -999;
        let result;
        if load_rc == 0 {
            l.js_pushundefined(j);
            call_rc = l.js_pcall(j, 0);
            result = from_c(l.js_tryrepr(j, -1, ERRSTR));
            l.js_pop(j, 1);
        } else {
            result = from_c(l.js_tryrepr(j, -1, ERRSTR));
            l.js_pop(j, 1);
        }
        let top = l.js_gettop(j);
        l.js_freestate(j);
        EvalResult {
            load_rc,
            call_rc,
            result,
            out: out_take(),
            top,
        }
    }
}

/// The convenience one-shot wrapper.
pub fn dostring(l: &Lib, flags: c_int, src: &str) -> (c_int, String) {
    unsafe {
        out_clear();
        let j = new_state(l, flags);
        let cs = cstr(src);
        let rc = l.js_dostring(j, cs.as_ptr());
        l.js_freestate(j);
        (rc, out_take())
    }
}

/// Differential assertion for a JS snippet through the low level pipeline.
pub fn diff_eval(flags: c_int, src: &str) {
    let p = libs();
    let a = eval(&p.c, flags, src);
    let b = eval(&p.rs, flags, src);
    assert_eq!(a, b, "eval divergence (flags={flags})\nsrc: {src}");
}

/// Differential assertion for js_dostring.
pub fn diff_dostring(flags: c_int, src: &str) {
    let p = libs();
    let a = dostring(&p.c, flags, src);
    let b = dostring(&p.rs, flags, src);
    assert_eq!(a, b, "dostring divergence (flags={flags})\nsrc: {src}");
}

/* ---------------------------------------------------- tiny prng (xorshift) */

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
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            lo
        } else {
            lo + (self.next_u64() % ((hi - lo) as u64)) as i64
        }
    }
    /// arbitrary f64 including subnormals / inf / nan
    pub fn f64_any(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// "reasonable" f64 in a range
    pub fn f64_sane(&mut self) -> f64 {
        let k = self.below(12);
        match k {
            0 => 0.0,
            1 => -0.0,
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => f64::NAN,
            5 => (self.next_u32() as i32) as f64,
            6 => (self.next_u64() as i64) as f64,
            7 => self.next_u32() as f64 / 4096.0,
            8 => self.f64_any(),
            9 => (self.range(-1000, 1000)) as f64,
            10 => (self.range(-1000, 1000)) as f64 + 0.5,
            _ => {
                let m = self.next_u64() & ((1u64 << 52) - 1);
                let e = self.range(-40, 40) as i32;
                (m as f64) * 2f64.powi(e)
            }
        }
    }
    pub fn ascii_string(&mut self, maxlen: usize) -> String {
        let n = self.below(maxlen as u32 + 1) as usize;
        (0..n)
            .map(|_| {
                let c = 0x20u8 + (self.below(0x5f) as u8);
                c as char
            })
            .collect()
    }
    /// bytes that may or may not be valid utf-8 (never NUL)
    pub fn raw_bytes(&mut self, maxlen: usize) -> Vec<u8> {
        let n = self.below(maxlen as u32 + 1) as usize;
        (0..n)
            .map(|_| {
                let b = (self.next_u32() & 0xff) as u8;
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect()
    }
    pub fn unicode_string(&mut self, maxlen: usize) -> String {
        let n = self.below(maxlen as u32 + 1) as usize;
        let mut s = String::new();
        for _ in 0..n {
            loop {
                let r = match self.below(5) {
                    0 => self.below(0x80),
                    1 => 0x80 + self.below(0x780),
                    2 => 0x800 + self.below(0xf800),
                    3 => 0x10000 + self.below(0x100000),
                    _ => 0x20 + self.below(0x5f),
                };
                if r == 0 {
                    continue;
                }
                if let Some(c) = char::from_u32(r) {
                    s.push(c);
                    break;
                }
            }
        }
        s
    }
}

/* --------------------------------------------------- big-stack test runner */

/// Both libraries use deep native recursion (`regexp.c` `match()` up to
/// `REG_MAXREC` = 4096 frames, `jsparse.c` up to `JS_ASTLIMIT` = 400, and
/// `jsrun.c` for nested calls). An unoptimised Rust build has considerably
/// larger frames than the C build, so tests that drive those paths must run on
/// a thread with a generous stack; otherwise the harness itself overflows
/// before the library's own limit is reached.
pub fn with_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    let h = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(f)
        .expect("spawn big-stack thread");
    h.join().expect("big-stack thread panicked");
}
