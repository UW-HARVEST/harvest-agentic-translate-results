//! Shared infrastructure for the differential tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and are
//! only ever driven through their exported symbols:
//!
//!  * `c_build/libcdriver.so`   — built from the unmodified `c_src` (ground truth)
//!  * `target/debug/libdriver.so` — the Rust `cdylib`
//!
//! Every case is executed twice (once per shared object) and every observable
//! effect is turned into a text "transcript": return values, the fields of the
//! returned structs read back through the C struct layout, the bytes written to
//! `stdout` / `stderr`, and the files created on disk.  The two transcripts must
//! be byte identical.
#![allow(dead_code)]

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fs;
use std::io::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// The C types (transcribed from c_src/include/{shape,scene}.h).
//
// The layout is verified against the C compiler:
//   sizeof(shape_t) = 2444, type@0, name@4, art@36, width@2436, height@2440
//   sizeof(scene_t) =  472, name@0, shapes@64, shape_count@464
// ---------------------------------------------------------------------------

pub const MAX_SHAPE_WIDTH: usize = 80;
pub const MAX_SHAPE_HEIGHT: usize = 30;
pub const MAX_SHAPE_NAME: usize = 32;
pub const MAX_SHAPES_IN_SCENE: usize = 50;
pub const MAX_SCENE_NAME: usize = 64;
pub const SHAPE_COUNT: c_int = 10;
pub const MAX_SCENES: usize = 10;

#[repr(C)]
pub struct ShapeT {
    pub type_: c_int,
    pub name: [c_char; MAX_SHAPE_NAME],
    pub art: [[c_char; MAX_SHAPE_WIDTH]; MAX_SHAPE_HEIGHT],
    pub width: c_int,
    pub height: c_int,
}

#[repr(C)]
pub struct SceneT {
    pub name: [c_char; MAX_SCENE_NAME],
    pub shapes: [*mut ShapeT; MAX_SHAPES_IN_SCENE],
    pub shape_count: c_int,
}

// ---------------------------------------------------------------------------
// Paths / building
// ---------------------------------------------------------------------------

pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust `cdylib`.
///
/// `cargo test` does **not** rebuild a `cdylib` that no test target links
/// against, so `target/debug/libdriver.so` can silently be stale (or missing).
/// To make the differential tests independent of that, the library is (re)built here with a
/// nested `cargo build --lib` into a *separate* target directory - separate so
/// that it cannot deadlock against the `cargo test` that currently holds the
/// lock on `target/`.  Cargo itself decides whether anything has to be
/// recompiled, so no home grown staleness heuristic is involved.
pub fn rust_lib_path() -> PathBuf {
    let target = crate_dir().join("target").join("testlib");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = std::process::Command::new(cargo)
        .args(["build", "--lib"])
        .current_dir(crate_dir())
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .expect("failed to run `cargo build --lib`");
    assert!(
        out.status.success(),
        "`cargo build --lib` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let so = target.join("debug").join("libdriver.so");
    assert!(so.exists(), "{} was not produced", so.display());
    so
}

pub fn harness_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/debug")
        .to_path_buf();
    let h = dir.join("examples").join("diffharness");
    assert!(
        h.exists(),
        "{} does not exist - `cargo test` builds it as an example",
        h.display()
    );
    h
}

/// Builds `c_build/libcdriver.so` from the unmodified `c_src` sources if it is
/// missing or older than the sources.  The compile output is renamed into place
/// so that concurrently running test binaries cannot observe a partial file.
pub fn c_lib_path() -> PathBuf {
    let root = crate_dir();
    let out_dir = root.join("c_build");
    let so = out_dir.join("libcdriver.so");
    let sources = [
        root.join("c_src/src/main.c"),
        root.join("c_src/src/scene.c"),
        root.join("c_src/src/shape.c"),
        root.join("c_src/include/scene.h"),
        root.join("c_src/include/shape.h"),
    ];
    let newest = sources
        .iter()
        .map(|p| fs::metadata(p).and_then(|m| m.modified()).unwrap())
        .max()
        .unwrap();
    let up_to_date = fs::metadata(&so)
        .and_then(|m| m.modified())
        .map(|t| t >= newest)
        .unwrap_or(false);
    if up_to_date {
        return so;
    }

    fs::create_dir_all(&out_dir).unwrap();
    let tmp = out_dir.join(format!("libcdriver.{}.so", std::process::id()));
    let status = std::process::Command::new("gcc")
        .args(["-shared", "-fPIC", "-O0", "-g", "-I"])
        .arg(root.join("c_src/include"))
        .arg(root.join("c_src/src/main.c"))
        .arg(root.join("c_src/src/scene.c"))
        .arg(root.join("c_src/src/shape.c"))
        .arg("-o")
        .arg(&tmp)
        .status()
        .expect("failed to run gcc");
    assert!(status.success(), "gcc failed to build the C shared object");
    fs::rename(&tmp, &so).unwrap();
    so
}

/// Builds `tests/support/failmalloc.c` (test scaffolding, outside `c_src`) into a
/// shared object that can be `LD_PRELOAD`ed into the harness children to make
/// allocations of one exact size fail - the only way to reach the
/// allocation-failure branches of the C code from the outside.
pub fn failmalloc_path() -> PathBuf {
    let root = crate_dir();
    let out_dir = root.join("c_build");
    let so = out_dir.join("libfailmalloc.so");
    let src = root.join("tests/support/failmalloc.c");
    let newest = fs::metadata(&src).and_then(|m| m.modified()).unwrap();
    let fresh = fs::metadata(&so)
        .and_then(|m| m.modified())
        .map(|t| t >= newest)
        .unwrap_or(false);
    if fresh {
        return so;
    }
    fs::create_dir_all(&out_dir).unwrap();
    let tmp = out_dir.join(format!("libfailmalloc.{}.so", std::process::id()));
    let status = std::process::Command::new("gcc")
        .args(["-shared", "-fPIC", "-O0"])
        .arg(&src)
        .arg("-o")
        .arg(&tmp)
        .arg("-ldl")
        .status()
        .expect("gcc");
    assert!(status.success(), "gcc failed to build failmalloc.so");
    fs::rename(&tmp, &so).unwrap();
    so
}

static TMP_SEQ: AtomicUsize = AtomicUsize::new(0);

/// A fresh directory below `target/difftests/`.
pub fn fresh_dir(tag: &str) -> PathBuf {
    let n = TMP_SEQ.fetch_add(1, Ordering::SeqCst);
    let base = crate_dir()
        .join("target")
        .join("difftests")
        .join(format!("{}-{}-{}", tag, std::process::id(), n));
    fs::create_dir_all(&base).unwrap();
    base
}

// ---------------------------------------------------------------------------
// The loaded API
// ---------------------------------------------------------------------------

pub struct Api {
    pub which: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub shape_manager_init: unsafe extern "C" fn(),
    pub shape_manager_cleanup: unsafe extern "C" fn(),
    pub shape_get: unsafe extern "C" fn(c_int) -> *mut ShapeT,
    pub shape_print: unsafe extern "C" fn(*const ShapeT),
    pub shape_equals: unsafe extern "C" fn(*const ShapeT, *const ShapeT) -> c_int,
    pub shape_type_name: unsafe extern "C" fn(c_int) -> *const c_char,
    pub scene_create: unsafe extern "C" fn(*const c_char) -> *mut SceneT,
    pub scene_destroy: unsafe extern "C" fn(*mut SceneT),
    pub scene_add_shape: unsafe extern "C" fn(*mut SceneT, *mut ShapeT) -> c_int,
    pub scene_remove_shape: unsafe extern "C" fn(*mut SceneT, c_int) -> c_int,
    pub scene_print: unsafe extern "C" fn(*const SceneT),
    pub scene_equals: unsafe extern "C" fn(*const SceneT, *const SceneT) -> c_int,
    pub scene_save: unsafe extern "C" fn(*const SceneT, *const c_char) -> c_int,
    pub scene_load: unsafe extern "C" fn(*const c_char) -> *mut SceneT,
    pub scene_list_shapes: unsafe extern "C" fn(*const SceneT),
}

macro_rules! sym {
    ($lib:expr, $name:literal, $t:ty) => {{
        let s: Symbol<$t> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
        *s
    }};
}

impl Api {
    pub fn load(which: &'static str, path: PathBuf) -> Api {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {}", path.display(), e));
        let api = Api {
            which,
            path: path.clone(),
            shape_manager_init: sym!(lib, "shape_manager_init", unsafe extern "C" fn()),
            shape_manager_cleanup: sym!(lib, "shape_manager_cleanup", unsafe extern "C" fn()),
            shape_get: sym!(lib, "shape_get", unsafe extern "C" fn(c_int) -> *mut ShapeT),
            shape_print: sym!(lib, "shape_print", unsafe extern "C" fn(*const ShapeT)),
            shape_equals: sym!(
                lib,
                "shape_equals",
                unsafe extern "C" fn(*const ShapeT, *const ShapeT) -> c_int
            ),
            shape_type_name: sym!(
                lib,
                "shape_type_name",
                unsafe extern "C" fn(c_int) -> *const c_char
            ),
            scene_create: sym!(
                lib,
                "scene_create",
                unsafe extern "C" fn(*const c_char) -> *mut SceneT
            ),
            scene_destroy: sym!(lib, "scene_destroy", unsafe extern "C" fn(*mut SceneT)),
            scene_add_shape: sym!(
                lib,
                "scene_add_shape",
                unsafe extern "C" fn(*mut SceneT, *mut ShapeT) -> c_int
            ),
            scene_remove_shape: sym!(
                lib,
                "scene_remove_shape",
                unsafe extern "C" fn(*mut SceneT, c_int) -> c_int
            ),
            scene_print: sym!(lib, "scene_print", unsafe extern "C" fn(*const SceneT)),
            scene_equals: sym!(
                lib,
                "scene_equals",
                unsafe extern "C" fn(*const SceneT, *const SceneT) -> c_int
            ),
            scene_save: sym!(
                lib,
                "scene_save",
                unsafe extern "C" fn(*const SceneT, *const c_char) -> c_int
            ),
            scene_load: sym!(
                lib,
                "scene_load",
                unsafe extern "C" fn(*const c_char) -> *mut SceneT
            ),
            scene_list_shapes: sym!(lib, "scene_list_shapes", unsafe extern "C" fn(*const SceneT)),
            _lib: lib,
        };
        api
    }
}

pub struct Apis {
    pub c: Api,
    pub rust: Api,
}

pub fn load_apis() -> Apis {
    Apis {
        c: Api::load("C", c_lib_path()),
        rust: Api::load("RUST", rust_lib_path()),
    }
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

/// Runs `f` with `stdout` and `stderr` redirected into temporary files and
/// returns everything that was written.  `fflush(NULL)` is used on both sides of
/// the call so that the C library's own buffering cannot leak between cases.
pub fn capture<R>(dir: &Path, f: impl FnOnce() -> R) -> (R, Vec<u8>, Vec<u8>) {
    let out_path = dir.join(".capture.out");
    let err_path = dir.join(".capture.err");
    unsafe {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        libc::fflush(std::ptr::null_mut());

        let of = fs::File::create(&out_path).unwrap();
        let ef = fs::File::create(&err_path).unwrap();
        let saved_out = libc::dup(1);
        let saved_err = libc::dup(2);
        assert!(saved_out >= 0 && saved_err >= 0);
        assert!(libc::dup2(of.as_raw_fd(), 1) >= 0);
        assert!(libc::dup2(ef.as_raw_fd(), 2) >= 0);

        let r = f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved_out, 1);
        libc::dup2(saved_err, 2);
        libc::close(saved_out);
        libc::close(saved_err);
        drop(of);
        drop(ef);

        let out = fs::read(&out_path).unwrap();
        let err = fs::read(&err_path).unwrap();
        let _ = fs::remove_file(&out_path);
        let _ = fs::remove_file(&err_path);
        (r, out, err)
    }
}

// ---------------------------------------------------------------------------
// Normalisation
// ---------------------------------------------------------------------------

/// Replaces every `0x…` pointer with a stable id based on first use, so that
/// pointer *identity* relations are still compared while the (necessarily
/// different) addresses are not.
pub fn normalise_ptrs(data: &[u8]) -> Vec<u8> {
    let mut seen: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'0' && i + 1 < data.len() && data[i + 1] == b'x' {
            let mut j = i + 2;
            while j < data.len() && (data[j] as char).is_ascii_hexdigit() {
                j += 1;
            }
            if j > i + 2 {
                let key = data[i..j].to_vec();
                let n = seen.len();
                let id = *seen.entry(key).or_insert(n);
                out.extend_from_slice(format!("0xPTR{}", id).as_bytes());
                i = j;
                continue;
            }
        }
        out.push(data[i]);
        i += 1;
    }
    out
}

/// Replaces the (per run) working directory path with a placeholder.
pub fn normalise_dir(data: &[u8], dir: &Path) -> Vec<u8> {
    let needle = dir.as_os_str().as_bytes();
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i..].starts_with(needle) {
            out.extend_from_slice(b"<DIR>");
            i += needle.len();
        } else {
            out.push(data[i]);
            i += 1;
        }
    }
    out
}

pub fn escape(data: &[u8]) -> String {
    let mut s = String::new();
    for &b in data {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Deterministic RNG (no external crate, fixed seed => reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}

// ---------------------------------------------------------------------------
// Transcripts
// ---------------------------------------------------------------------------

/// Everything a case records about one run of one implementation.
pub struct Ctx {
    pub dir: PathBuf,
    pub text: String,
    ptrs: HashMap<usize, usize>,
    next_id: usize,
}

impl Ctx {
    fn new(dir: PathBuf) -> Ctx {
        Ctx {
            dir,
            text: String::new(),
            ptrs: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn line(&mut self, s: impl AsRef<str>) {
        self.text.push_str(s.as_ref());
        self.text.push('\n');
    }

    /// A stable id for a pointer, assigned on first use.  `NULL` is spelled out.
    pub fn tag(&mut self, p: *const c_void) -> String {
        let a = p as usize;
        if a == 0 {
            return "NULL".to_string();
        }
        let next = self.next_id;
        let id = *self.ptrs.entry(a).or_insert_with(|| next);
        if id == next {
            self.next_id += 1;
        }
        format!("#{}", id)
    }

    /// Forgets all pointer ids.  Used at points where the library frees memory:
    /// whether the allocator hands the *same* address out again afterwards is a
    /// property of glibc's `malloc`, not of the translation, and the two runs
    /// happen in the same process one after the other, so their allocator state
    /// is not comparable.
    pub fn forget_ptrs(&mut self) {
        self.ptrs.clear();
        self.next_id = 0;
    }

    /// An absolute path inside this run's working directory.
    pub fn path(&self, name: &str) -> CString {
        CString::new(self.dir.join(name).into_os_string().into_vec()).unwrap()
    }

    pub fn write_file(&self, name: &str, content: &[u8]) {
        fs::write(self.dir.join(name), content).unwrap();
    }

    pub fn dump_shape(&mut self, p: *mut ShapeT) {
        if p.is_null() {
            self.line("  shape = NULL");
            return;
        }
        let tag = self.tag(p as *const c_void);
        unsafe {
            let type_ = (*p).type_;
            let width = (*p).width;
            let height = (*p).height;
            let name = cstr_bytes(std::ptr::addr_of!((*p).name) as *const c_char, MAX_SHAPE_NAME);
            self.line(format!(
                "  shape = {} type={} name=\"{}\" width={} height={}",
                tag,
                type_,
                escape(&name),
                width,
                height
            ));
            // Only the rows `shape_print` reads are defined: the C code
            // `strcpy`s exactly `height` rows into `malloc`ed (uninitialised)
            // memory.
            let rows = if height < 0 {
                0
            } else if height as usize > MAX_SHAPE_HEIGHT {
                MAX_SHAPE_HEIGHT
            } else {
                height as usize
            };
            for r in 0..rows {
                let row = (std::ptr::addr_of!((*p).art) as *const c_char).add(r * MAX_SHAPE_WIDTH);
                let bytes = cstr_bytes(row, MAX_SHAPE_WIDTH);
                self.line(format!("    art[{}] = \"{}\"", r, escape(&bytes)));
            }
        }
    }

    pub fn dump_scene(&mut self, p: *mut SceneT) {
        if p.is_null() {
            self.line("  scene = NULL");
            return;
        }
        let tag = self.tag(p as *const c_void);
        unsafe {
            let count = (*p).shape_count;
            let name = cstr_bytes(std::ptr::addr_of!((*p).name) as *const c_char, MAX_SCENE_NAME);
            self.line(format!(
                "  scene = {} name=\"{}\" shape_count={}",
                tag,
                escape(&name),
                count
            ));
            let n = if count < 0 {
                0
            } else if count as usize > MAX_SHAPES_IN_SCENE {
                MAX_SHAPES_IN_SCENE
            } else {
                count as usize
            };
            for i in 0..n {
                let sp = *(std::ptr::addr_of!((*p).shapes) as *const *mut ShapeT).add(i);
                let t = self.tag(sp as *const c_void);
                self.line(format!("    shapes[{}] = {}", i, t));
            }
        }
    }

    /// All 64 bytes of `scene->name`.  Only defined when the scene was created
    /// with a non-NULL name: `strncpy` zero pads the whole buffer, and
    /// `scene_create` then sets `name[63] = 0` explicitly.
    pub fn dump_scene_name_raw(&mut self, p: *mut SceneT) {
        if p.is_null() {
            self.line("  raw name = NULL");
            return;
        }
        unsafe {
            let base = std::ptr::addr_of!((*p).name) as *const u8;
            let bytes = std::slice::from_raw_parts(base, MAX_SCENE_NAME);
            self.line(format!("  raw name = [{}]", escape(bytes)));
        }
    }

    /// Normalises the `0x…` pointers of a captured byte stream **through the same
    /// id map that `tag()` uses**.  A pointer the library printed is therefore
    /// compared against the object identities the test observed in the structs:
    /// printing the wrong (but consistently wrong) pointer cannot hide behind a
    /// per-stream renumbering.
    pub fn normalise_ptrs_shared(&mut self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len());
        let mut i = 0;
        while i < data.len() {
            if data[i] == b'0' && i + 1 < data.len() && data[i + 1] == b'x' {
                let mut j = i + 2;
                while j < data.len() && (data[j] as char).is_ascii_hexdigit() {
                    j += 1;
                }
                if j > i + 2 {
                    let text = std::str::from_utf8(&data[i + 2..j]).unwrap();
                    let addr = usize::from_str_radix(text, 16).unwrap_or(usize::MAX);
                    let tag = self.tag(addr as *const c_void);
                    out.extend_from_slice(format!("<ptr {}>", tag).as_bytes());
                    i = j;
                    continue;
                }
            }
            out.push(data[i]);
            i += 1;
        }
        out
    }

    pub fn c_str(&mut self, p: *const c_char) -> String {
        if p.is_null() {
            return "NULL".to_string();
        }
        unsafe { escape(CStr::from_ptr(p).to_bytes()) }
    }
}

/// The bytes of a C string, bounded by the size of its buffer.
pub unsafe fn cstr_bytes(p: *const c_char, max: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..max {
        let b = *p.add(i) as u8;
        if b == 0 {
            break;
        }
        v.push(b);
    }
    v
}

fn dir_listing(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue; // capture scratch files
        }
        if p.is_dir() {
            files.push((name + "/", Vec::new()));
        } else {
            files.push((name, fs::read(&p).unwrap_or_default()));
        }
    }
    files
}

/// Runs `body` against one implementation and returns the transcript.
pub fn transcript(api: &Api, case: &str, body: &dyn Fn(&Api, &mut Ctx)) -> String {
    let dir = fresh_dir(case);
    let mut ctx = Ctx::new(dir.clone());
    let (_, out, err) = capture(&dir, || body(api, &mut ctx));

    let mut text = String::new();
    text.push_str(&ctx.text);
    let out = ctx.normalise_ptrs_shared(&normalise_dir(&out, &dir));
    let err = ctx.normalise_ptrs_shared(&normalise_dir(&err, &dir));
    text.push_str("--- stdout ---\n");
    text.push_str(&numbered(&out));
    text.push_str("--- stderr ---\n");
    text.push_str(&numbered(&err));
    text.push_str("--- files ---\n");
    for (name, content) in dir_listing(&dir) {
        let content = ctx.normalise_ptrs_shared(&normalise_dir(&content, &dir));
        text.push_str(&format!("{}:\n", name));
        text.push_str(&numbered(&content));
    }
    let _ = fs::remove_dir_all(&dir);
    text
}

/// One transcript line per output line, so that a divergence points at the
/// offending line instead of at one huge blob.
pub fn numbered(data: &[u8]) -> String {
    let mut s = String::new();
    for (i, line) in data.split(|&b| b == b'\n').enumerate() {
        s.push_str(&format!("  {:>4} | {}\n", i + 1, escape(line)));
    }
    s
}

/// Runs one case against both shared objects and compares the transcripts.
pub fn diff_case(apis: &Apis, case: &str, body: &dyn Fn(&Api, &mut Ctx)) -> Result<(), String> {
    let c = transcript(&apis.c, case, body);
    let r = transcript(&apis.rust, case, body);
    if std::env::var_os("DIFF_DUMP").is_some() {
        eprintln!("===== case {} (C transcript) =====\n{}", case, c);
    }
    if c == r {
        return Ok(());
    }
    let mut msg = format!("case `{}` diverges:\n", case);
    let cl: Vec<&str> = c.lines().collect();
    let rl: Vec<&str> = r.lines().collect();
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or("<missing>");
        let b = rl.get(i).copied().unwrap_or("<missing>");
        if a != b {
            msg.push_str(&format!("  line {}:\n    C   : {}\n    RUST: {}\n", i + 1, a, b));
        }
    }
    Err(msg)
}

/// Collects the results of many cases and turns them into one assertion.
pub struct Report {
    pub failures: Vec<String>,
    pub passed: usize,
}

impl Report {
    pub fn new() -> Report {
        Report {
            failures: Vec::new(),
            passed: 0,
        }
    }
    pub fn check(&mut self, r: Result<(), String>) {
        match r {
            Ok(()) => self.passed += 1,
            Err(e) => self.failures.push(e),
        }
    }
    pub fn finish(self, what: &str) {
        if !self.failures.is_empty() {
            let shown: Vec<String> = self.failures.iter().take(10).cloned().collect();
            panic!(
                "{}: {} case(s) passed, {} FAILED\n\n{}",
                what,
                self.passed,
                self.failures.len(),
                shown.join("\n")
            );
        }
        eprintln!("{}: {} case(s) passed", what, self.passed);
    }
}

// ---------------------------------------------------------------------------
// Application level runs (child process, see examples/diffharness.rs)
// ---------------------------------------------------------------------------

pub const APP_TIMEOUT_MS: u64 = 3000;

/// Runs one scenario (a list of exported functions + the stdin they consume)
/// against one shared object in a fresh child process and a fresh working
/// directory, and returns the transcript.
#[allow(clippy::too_many_arguments)]
pub fn app_transcript(
    lib: &Path,
    case: &str,
    fns: &[&str],
    stdin: &[u8],
    seed_files: &[(&str, &[u8])],
    env: &[(&str, String)],
    timeout_ms: u64,
) -> String {
    let dir = fresh_dir(case);
    for (name, content) in seed_files {
        fs::write(dir.join(name), content).unwrap();
    }
    let stdin_path = dir.join(".stdin");
    fs::write(&stdin_path, stdin).unwrap();
    let out_path = dir.join(".stdout");
    let err_path = dir.join(".stderr");
    let res_path = dir.join(".result");

    let mut cmd = std::process::Command::new(harness_path());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .arg(lib)
        .arg(&res_path)
        .args(fns)
        .current_dir(&dir)
        .stdin(fs::File::open(&stdin_path).unwrap())
        .stdout(fs::File::create(&out_path).unwrap())
        .stderr(fs::File::create(&err_path).unwrap())
        .spawn()
        .expect("spawn diffharness");

    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait().unwrap() {
            Some(s) => break Some(s),
            None => {
                if start.elapsed().as_millis() as u64 > timeout_ms {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    };

    let out = fs::read(&out_path).unwrap_or_default();
    let err = fs::read(&err_path).unwrap_or_default();
    let res = fs::read(&res_path).unwrap_or_default();

    let status_text = match status {
        None => "TIMEOUT (killed)".to_string(),
        Some(s) => match s.code() {
            Some(c) => format!("exit {}", c),
            None => format!("signal {:?}", std::os::unix::process::ExitStatusExt::signal(&s)),
        },
    };

    let norm = |b: &[u8]| normalise_ptrs(&normalise_dir(b, &dir));
    let mut text = String::new();
    text.push_str(&format!("--- status ---\n{}\n", status_text));
    text.push_str("--- results ---\n");
    text.push_str(&numbered(&norm(&res)));
    text.push_str("--- stdout ---\n");
    text.push_str(&numbered(&norm(&out)));
    text.push_str("--- stderr ---\n");
    text.push_str(&numbered(&norm(&err)));
    text.push_str("--- files ---\n");
    for (name, content) in dir_listing(&dir) {
        text.push_str(&format!("{}:\n", name));
        text.push_str(&numbered(&norm(&content)));
    }
    let _ = fs::remove_dir_all(&dir);
    text
}

/// Runs one application level scenario against both shared objects.
#[allow(clippy::too_many_arguments)]
pub fn diff_app_env(
    apis: &Apis,
    case: &str,
    fns: &[&str],
    stdin: &[u8],
    seed_files: &[(&str, &[u8])],
    env: &[(&str, String)],
    timeout_ms: u64,
) -> Result<(), String> {
    diff_app_inner(apis, case, fns, stdin, seed_files, env, timeout_ms)
}

pub fn diff_app_full(
    apis: &Apis,
    case: &str,
    fns: &[&str],
    stdin: &[u8],
    seed_files: &[(&str, &[u8])],
    timeout_ms: u64,
) -> Result<(), String> {
    diff_app_inner(apis, case, fns, stdin, seed_files, &[], timeout_ms)
}

#[allow(clippy::too_many_arguments)]
fn diff_app_inner(
    apis: &Apis,
    case: &str,
    fns: &[&str],
    stdin: &[u8],
    seed_files: &[(&str, &[u8])],
    env: &[(&str, String)],
    timeout_ms: u64,
) -> Result<(), String> {
    let c = app_transcript(&apis.c.path, case, fns, stdin, seed_files, env, timeout_ms);
    let r = app_transcript(&apis.rust.path, case, fns, stdin, seed_files, env, timeout_ms);
    if std::env::var_os("DIFF_DUMP").is_some() {
        eprintln!("===== app case {} (C transcript) =====\n{}", case, c);
    }
    if c == r {
        return Ok(());
    }
    let mut msg = format!(
        "app case `{}` diverges (fns={:?}, stdin={:?}):\n",
        case,
        fns,
        escape(stdin)
    );
    let cl: Vec<&str> = c.lines().collect();
    let rl: Vec<&str> = r.lines().collect();
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or("<missing>");
        let b = rl.get(i).copied().unwrap_or("<missing>");
        if a != b {
            msg.push_str(&format!(
                "  line {}:\n    C   : {}\n    RUST: {}\n",
                i + 1,
                a,
                b
            ));
        }
    }
    Err(msg)
}

pub fn diff_app(apis: &Apis, case: &str, fns: &[&str], stdin: &[u8]) -> Result<(), String> {
    diff_app_full(apis, case, fns, stdin, &[], APP_TIMEOUT_MS)
}

pub fn diff_app_files(
    apis: &Apis,
    case: &str,
    fns: &[&str],
    stdin: &[u8],
    seed_files: &[(&str, &[u8])],
) -> Result<(), String> {
    diff_app_full(apis, case, fns, stdin, seed_files, APP_TIMEOUT_MS)
}
