//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! driven only through their exported C symbols — the Rust code is never called
//! directly, so the `#[no_mangle]` wrappers and the C ABI are part of what gets
//! tested.
//!
//! `a.c` and `b.c` keep file-scope `static int` state that survives across
//! calls, so results depend on the whole call history.  Every test therefore
//! loads its **own private copy** of both shared objects (`fresh_pair`): the
//! copies live at unique paths, and since neither object carries a `SONAME`,
//! `dlopen` maps each copy separately with freshly zero-initialised statics.
//! `assert_fresh_state_is_independent` in `differential.rs` proves this.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

// ---------------------------------------------------------------------------
// C-compatible mirrors of the util.h types (checked against the C layout by
// `abi_layout` in differential.rs).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug)]
pub struct IntVec {
    pub data: *mut c_int,
    pub len: usize,
    pub cap: usize,
}

impl IntVec {
    pub const fn zeroed() -> IntVec {
        IntVec {
            data: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }
    /// Contents as a Rust slice copy (empty when `data` is NULL).
    pub fn items(&self) -> Vec<c_int> {
        if self.data.is_null() || self.len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(self.data, self.len) }.to_vec()
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Program {
    pub code: *const c_int,
    pub n: usize,
    pub ip: usize,
}

impl Program {
    pub const fn zeroed() -> Program {
        Program {
            code: std::ptr::null(),
            n: 0,
            ip: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: c_int,
}

impl VM {
    pub const fn zeroed() -> VM {
        VM {
            stack: IntVec::zeroed(),
            trace: IntVec::zeroed(),
            steps: 0,
        }
    }
}

/// Everything observable about a VM after a run.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VmSnapshot {
    pub stack: Vec<c_int>,
    pub trace: Vec<c_int>,
    pub steps: c_int,
    pub stack_len: usize,
    pub stack_cap: usize,
    pub trace_len: usize,
    pub trace_cap: usize,
}

/// Everything observable about an IntVec.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VecSnapshot {
    pub items: Vec<c_int>,
    pub len: usize,
    pub cap: usize,
    pub data_is_null: bool,
}

// ---------------------------------------------------------------------------
// libc bits used by the harness itself.
// ---------------------------------------------------------------------------
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn open_memstream(bufp: *mut *mut c_char, sizep: *mut usize) -> *mut c_void;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// Loaded API surface (all 18 non-`main` exports).
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub iv_init: unsafe extern "C" fn(*mut IntVec),
    pub iv_free: unsafe extern "C" fn(*mut IntVec),
    pub iv_reserve: unsafe extern "C" fn(*mut IntVec, usize) -> bool,
    pub iv_push: unsafe extern "C" fn(*mut IntVec, c_int) -> bool,
    pub iv_pop: unsafe extern "C" fn(*mut IntVec, *mut c_int) -> bool,
    pub iv_peek: unsafe extern "C" fn(*const IntVec, c_int) -> c_int,
    pub prog_init: unsafe extern "C" fn(*mut Program, *const c_int, usize),
    pub prog_fetch: unsafe extern "C" fn(*mut Program, *mut c_int) -> bool,
    pub vm_init: unsafe extern "C" fn(*mut VM),
    pub vm_free: unsafe extern "C" fn(*mut VM),
    pub vm_trace: unsafe extern "C" fn(*mut VM, c_int),
    pub vm_print: unsafe extern "C" fn(*mut c_void, *const c_char, *const VM),
    pub run_engine: unsafe extern "C" fn(c_int, *const c_int, usize, *mut VM) -> c_int,
    pub target: unsafe extern "C" fn(c_int) -> c_int,
    pub call_a_once: unsafe extern "C" fn(c_int) -> c_int,
    pub process_a_stream: unsafe extern "C" fn(*const c_int, usize) -> c_int,
    pub call_b_once: unsafe extern "C" fn(c_int) -> c_int,
    pub process_b_stream: unsafe extern "C" fn(*const c_int, usize) -> c_int,
}

macro_rules! sym {
    ($lib:expr, $name:literal) => {{
        let s = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name));
        *s
    }};
}

impl Api {
    pub fn load(name: &'static str, path: &Path) -> Api {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        let api = Api {
            name,
            path: path.to_path_buf(),
            iv_init: sym!(lib, "iv_init"),
            iv_free: sym!(lib, "iv_free"),
            iv_reserve: sym!(lib, "iv_reserve"),
            iv_push: sym!(lib, "iv_push"),
            iv_pop: sym!(lib, "iv_pop"),
            iv_peek: sym!(lib, "iv_peek"),
            prog_init: sym!(lib, "prog_init"),
            prog_fetch: sym!(lib, "prog_fetch"),
            vm_init: sym!(lib, "vm_init"),
            vm_free: sym!(lib, "vm_free"),
            vm_trace: sym!(lib, "vm_trace"),
            vm_print: sym!(lib, "vm_print"),
            run_engine: sym!(lib, "run_engine"),
            target: sym!(lib, "target"),
            call_a_once: sym!(lib, "call_a_once"),
            process_a_stream: sym!(lib, "process_a_stream"),
            call_b_once: sym!(lib, "call_b_once"),
            process_b_stream: sym!(lib, "process_b_stream"),
            _lib: lib,
        };
        api
    }

    // -- convenience wrappers -------------------------------------------------

    pub fn new_vec(&self) -> IntVec {
        let mut v = IntVec::zeroed();
        unsafe { (self.iv_init)(&mut v) };
        v
    }

    pub fn new_vm(&self) -> VM {
        let mut vm = VM::zeroed();
        unsafe { (self.vm_init)(&mut vm) };
        vm
    }

    pub fn snapshot_vec(&self, v: &IntVec) -> VecSnapshot {
        VecSnapshot {
            items: v.items(),
            len: v.len,
            cap: v.cap,
            data_is_null: v.data.is_null(),
        }
    }

    pub fn snapshot_vm(&self, vm: &VM) -> VmSnapshot {
        VmSnapshot {
            stack: vm.stack.items(),
            trace: vm.trace.items(),
            steps: vm.steps,
            stack_len: vm.stack.len,
            stack_cap: vm.stack.cap,
            trace_len: vm.trace.len,
            trace_cap: vm.trace.cap,
        }
    }

    /// `run_engine` on a freshly initialised VM; returns (rc, snapshot).
    pub fn run(&self, impl_id: c_int, code: &[c_int]) -> (c_int, VmSnapshot) {
        let mut vm = self.new_vm();
        let ptr = if code.is_empty() {
            std::ptr::null()
        } else {
            code.as_ptr()
        };
        let rc = unsafe { (self.run_engine)(impl_id, ptr, code.len(), &mut vm) };
        let snap = self.snapshot_vm(&vm);
        unsafe { (self.vm_free)(&mut vm) };
        (rc, snap)
    }

    /// `run_engine` with an explicit (possibly bogus) length / pointer.
    pub fn run_raw(&self, impl_id: c_int, code: *const c_int, n: usize) -> (c_int, VmSnapshot) {
        let mut vm = self.new_vm();
        let rc = unsafe { (self.run_engine)(impl_id, code, n, &mut vm) };
        let snap = self.snapshot_vm(&vm);
        unsafe { (self.vm_free)(&mut vm) };
        (rc, snap)
    }

    /// Capture what `vm_print` writes for a VM built from `stack`/`trace`/`steps`.
    pub fn print_vm(&self, label: &str, stack: &[c_int], trace: &[c_int], steps: c_int) -> Vec<u8> {
        let mut vm = self.new_vm();
        unsafe {
            for &x in stack {
                assert!((self.iv_push)(&mut vm.stack, x));
            }
            for &t in trace {
                (self.vm_trace)(&mut vm, t);
            }
            vm.steps = steps;
        }
        let out = self.print_vm_raw(label, &vm);
        unsafe { (self.vm_free)(&mut vm) };
        out
    }

    /// `vm_print` with a NULL label pointer (glibc's `%s` prints "(null)").
    pub fn print_vm_null_label(&self, vm: &VM) -> Vec<u8> {
        let mut buf: *mut c_char = std::ptr::null_mut();
        let mut size: usize = 0;
        unsafe {
            let f = open_memstream(&mut buf, &mut size);
            assert!(!f.is_null(), "open_memstream failed");
            (self.vm_print)(f, std::ptr::null(), vm);
            fflush(f);
            let bytes = std::slice::from_raw_parts(buf as *const u8, size).to_vec();
            fclose(f);
            free(buf as *mut c_void);
            bytes
        }
    }

    pub fn print_vm_raw(&self, label: &str, vm: &VM) -> Vec<u8> {
        let mut buf: *mut c_char = std::ptr::null_mut();
        let mut size: usize = 0;
        let mut lbl: Vec<u8> = label.as_bytes().to_vec();
        lbl.push(0);
        unsafe {
            let f = open_memstream(&mut buf, &mut size);
            assert!(!f.is_null(), "open_memstream failed");
            (self.vm_print)(f, lbl.as_ptr() as *const c_char, vm);
            fflush(f);
            let bytes = std::slice::from_raw_parts(buf as *const u8, size).to_vec();
            fclose(f);
            free(buf as *mut c_void);
            bytes
        }
    }

    pub fn stream_a(&self, xs: &[c_int]) -> c_int {
        let p = if xs.is_empty() {
            std::ptr::null()
        } else {
            xs.as_ptr()
        };
        unsafe { (self.process_a_stream)(p, xs.len()) }
    }

    pub fn stream_b(&self, xs: &[c_int]) -> c_int {
        let p = if xs.is_empty() {
            std::ptr::null()
        } else {
            xs.as_ptr()
        };
        unsafe { (self.process_b_stream)(p, xs.len()) }
    }
}

// ---------------------------------------------------------------------------
// Building / locating the two shared objects.
// ---------------------------------------------------------------------------

fn newest_mtime(paths: &[PathBuf]) -> std::time::SystemTime {
    paths
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .filter_map(|m| m.modified().ok())
        .max()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
}

fn c_sources() -> Vec<PathBuf> {
    let dir = Path::new(MANIFEST_DIR).join("c_src/src");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("c_src/src")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "c").unwrap_or(false))
        .collect();
    v.sort();
    v
}

/// Allow the runner (scripts/run_all.sh) to point the tests at a different
/// build of either side, e.g. the *debug* cdylib, where Rust's integer overflow
/// checks are enabled.
fn env_path(var: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(v) if !v.is_empty() => {
            let p = PathBuf::from(&v);
            let p = if p.is_absolute() {
                p
            } else {
                Path::new(MANIFEST_DIR).join(p)
            };
            assert!(p.exists(), "{var}={v} does not exist");
            Some(p)
        }
        _ => None,
    }
}

/// Path of the C shared object, built on demand from *all* C sources (so it
/// exports `main` too, matching what the CMake target compiles).
pub fn c_so() -> PathBuf {
    static CELL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    CELL.get_or_init(build_c_so).clone()
}

fn build_c_so() -> PathBuf {
    if let Some(p) = env_path("DRIVER_C_SO") {
        return p;
    }
    let build = Path::new(MANIFEST_DIR).join("c_src/build");
    std::fs::create_dir_all(&build).unwrap();
    let so = build.join("libdriver_c_full.so");
    let srcs = c_sources();
    let need = match std::fs::metadata(&so).and_then(|m| m.modified()) {
        Ok(t) => t < newest_mtime(&srcs),
        Err(_) => true,
    };
    if need {
        let tmp = build.join(format!(
            "libdriver_c_full.{}.{}.tmp.so",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let mut cmd = std::process::Command::new("gcc");
        // No -O flag: the CMake project sets no CMAKE_BUILD_TYPE, i.e. the
        // reference build is unoptimised.
        cmd.arg("-fPIC").arg("-shared").arg("-o").arg(&tmp);
        for s in &srcs {
            cmd.arg(s);
        }
        let out = cmd.output().expect("run gcc");
        assert!(
            out.status.success(),
            "building the C .so failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        // Another thread/process may have produced it first; either way the
        // final artifact must exist afterwards.
        let _ = std::fs::rename(&tmp, &so);
        let _ = std::fs::remove_file(&tmp);
        assert!(so.exists(), "{} was not produced", so.display());
    }
    so
}

/// Path of the Rust cdylib, built on demand (`cargo build --release`).
pub fn rust_so() -> PathBuf {
    static CELL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    CELL.get_or_init(build_rust_so).clone()
}

fn build_rust_so() -> PathBuf {
    if let Some(p) = env_path("DRIVER_RUST_SO") {
        return p;
    }
    let so = Path::new(MANIFEST_DIR).join("target/release/libdriver.so");
    let srcs = vec![
        Path::new(MANIFEST_DIR).join("src/lib.rs"),
        Path::new(MANIFEST_DIR).join("src/main.rs"),
        Path::new(MANIFEST_DIR).join("Cargo.toml"),
    ];
    let need = match std::fs::metadata(&so).and_then(|m| m.modified()) {
        Ok(t) => t < newest_mtime(&srcs),
        Err(_) => true,
    };
    if need {
        let out = std::process::Command::new(env!("CARGO"))
            .args(["build", "--release", "--quiet"])
            .current_dir(MANIFEST_DIR)
            .output()
            .expect("run cargo build --release");
        assert!(
            out.status.success(),
            "cargo build --release failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(so.exists(), "{} not found", so.display());
    so
}

/// Path of the C executable, built on demand with the same flags as the .so.
pub fn c_exe() -> PathBuf {
    static CELL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    CELL.get_or_init(build_c_exe).clone()
}

fn build_c_exe() -> PathBuf {
    if let Some(p) = env_path("DRIVER_C_EXE") {
        return p;
    }
    let build = Path::new(MANIFEST_DIR).join("c_src/build");
    std::fs::create_dir_all(&build).unwrap();
    let exe = build.join("driver");
    let srcs = c_sources();
    let need = match std::fs::metadata(&exe).and_then(|m| m.modified()) {
        Ok(t) => t < newest_mtime(&srcs),
        Err(_) => true,
    };
    if need {
        let tmp = build.join(format!(
            "driver.{}.{}.tmp",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let mut cmd = std::process::Command::new("gcc");
        cmd.arg("-o").arg(&tmp);
        for s in &srcs {
            cmd.arg(s);
        }
        let out = cmd.output().expect("run gcc");
        assert!(
            out.status.success(),
            "building the C exe failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let _ = std::fs::rename(&tmp, &exe);
        let _ = std::fs::remove_file(&tmp);
        assert!(exe.exists(), "{} was not produced", exe.display());
    }
    exe
}

/// Path of the Rust executable (built together with the cdylib).
pub fn rust_exe() -> PathBuf {
    static CELL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    CELL.get_or_init(build_rust_exe).clone()
}

fn build_rust_exe() -> PathBuf {
    if let Some(p) = env_path("DRIVER_RUST_EXE") {
        return p;
    }
    rust_so();
    let exe = Path::new(MANIFEST_DIR).join("target/release/driver");
    assert!(exe.exists(), "{} not found", exe.display());
    exe
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch_dir() -> PathBuf {
    let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let d = Path::new(&base).join(format!("driver_diff_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A private (C, Rust) pair with pristine `static` state.
pub fn fresh_pair(tag: &str) -> (Api, Api) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = scratch_dir();
    let cs = c_so();
    let rs = rust_so();
    let cdst = dir.join(format!("{tag}_{n}_c.so"));
    let rdst = dir.join(format!("{tag}_{n}_r.so"));
    std::fs::copy(&cs, &cdst).unwrap();
    std::fs::copy(&rs, &rdst).unwrap();
    (Api::load("C", &cdst), Api::load("Rust", &rdst))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), so every "randomised" run is reproducible.
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
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A "interesting" 32 bit value: small, boundary or fully random.
    pub fn value(&mut self) -> c_int {
        match self.below(10) {
            0 => 0,
            1 => (self.below(21) as i64 - 10) as c_int,
            2 => (self.below(200) as i64 - 100) as c_int,
            3 => c_int::MIN,
            4 => c_int::MAX,
            5 => (self.below(20) as c_int) * 10,
            6 => -(self.below(1000) as c_int),
            7 => self.below(1000) as c_int,
            _ => self.next_i32(),
        }
    }
    /// A random but mostly-valid bytecode program.
    pub fn program(&mut self, max_len: usize) -> Vec<c_int> {
        let len = 1 + self.below(max_len as u64) as usize;
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            match self.below(100) {
                // pushes (with immediate) dominate so the stack rarely empties
                0..=24 => {
                    out.push(0);
                    out.push(self.value());
                }
                25..=32 => out.push(1),
                33..=40 => out.push(2),
                41..=50 => out.push(3),
                51..=54 => out.push(4),
                55..=64 => out.push(5),
                65..=68 => {
                    out.push(6);
                    out.push((self.below(5)) as c_int);
                }
                69..=74 => {
                    out.push(7);
                    out.push((self.below(4)) as c_int);
                }
                75..=84 => out.push(8),
                85..=92 => {
                    out.push(9);
                    out.push((self.below(4)) as c_int);
                }
                93..=95 => out.push(10),
                _ => out.push(self.value()),
            }
        }
        out.truncate(len);
        out
    }
}

// ---------------------------------------------------------------------------
// Calling the exported `main` symbol through dlopen.
//
// `main` writes to stdout/stderr and returns an exit status, so it is invoked
// from a tiny C loader subprocess (`dlopen` + `dlsym("main")` + call).  That
// keeps the streams isolated from the test harness's own output and still
// exercises the *exported* symbol of each shared object rather than the
// program binaries.
// ---------------------------------------------------------------------------

const MAIN_CALLER_C: &str = r#"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "usage: main_caller <so> [argv...]\n"); return 127; }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 126; }
    int (*m)(int, char **) = (int (*)(int, char **))dlsym(h, "main");
    if (!m) { fprintf(stderr, "dlsym(main): %s\n", dlerror()); return 125; }
    int sub_argc = argc - 2;
    char **sub_argv = sub_argc ? argv + 2 : NULL;
    int rc = m(sub_argc, sub_argv);
    fflush(NULL);
    return rc;
}
"#;

/// Everything observable from a `main()` call.
#[derive(Debug, PartialEq, Eq)]
pub struct MainOutcome {
    pub rc: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Path to the (on demand compiled) C loader.
pub fn main_caller() -> PathBuf {
    static CELL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    CELL.get_or_init(build_main_caller).clone()
}

fn build_main_caller() -> PathBuf {
    let dir = scratch_dir();
    let src = dir.join("main_caller.c");
    let exe = dir.join("main_caller");
    if !exe.exists() {
        std::fs::write(&src, MAIN_CALLER_C).unwrap();
        let tmp = dir.join(format!(
            "main_caller.{}.{}.tmp",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let out = std::process::Command::new("gcc")
            .arg("-o")
            .arg(&tmp)
            .arg(&src)
            .arg("-ldl")
            .output()
            .expect("run gcc");
        assert!(
            out.status.success(),
            "compiling the main loader failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let _ = std::fs::rename(&tmp, &exe);
    }
    exe
}

impl Api {
    /// Call the `main` exported by *this* shared object.  `args` is the complete
    /// argv (including argv[0]); an empty slice means `argc == 0, argv == NULL`.
    pub fn call_main(&self, args: &[&str]) -> MainOutcome {
        let out = std::process::Command::new(main_caller())
            .arg(&self.path)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .expect("run main_caller");
        MainOutcome {
            rc: out.status.code(),
            stdout: out.stdout,
            stderr: out.stderr,
        }
    }
}

// ---------------------------------------------------------------------------
// Caller-supplied structs.
//
// The `IntVec` / `VM` are plain C structs owned by the *caller*, so a real
// consumer may hand over a vector whose `cap` was not produced by `iv_reserve`'s
// doubling (e.g. `cap == 10`).  `iv_reserve` then doubles starting from that
// value, which is only observable if the tests actually build such a vector.
// The buffer must come from libc `malloc` so that the library's `realloc`/`free`
// can take it over.
// ---------------------------------------------------------------------------

/// An `IntVec` with a `malloc`ed buffer of exactly `cap` ints and `len` items.
pub fn make_vec(items: &[c_int], cap: usize) -> IntVec {
    assert!(items.len() <= cap && cap > 0);
    let p = unsafe { malloc(cap * std::mem::size_of::<c_int>()) } as *mut c_int;
    assert!(!p.is_null());
    unsafe {
        for (i, &x) in items.iter().enumerate() {
            *p.add(i) = x;
        }
    }
    IntVec {
        data: p,
        len: items.len(),
        cap,
    }
}

/// A `VM` whose stack is a caller-provided vector (trace starts empty).
pub fn make_vm(stack: &[c_int], stack_cap: usize, steps: c_int) -> VM {
    VM {
        stack: make_vec(stack, stack_cap),
        trace: IntVec::zeroed(),
        steps,
    }
}
