//! Differential-test harness.
//!
//! Loads **both** shared libraries with `libloading` and drives them only
//! through their exported symbols, so the `#[no_mangle]` wrappers are part of
//! what is being tested.
//!
//! Three libraries are used:
//!   * `c_lib()`        - the reference `.so` built exactly as the task
//!                        describes (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`,
//!                        no `CMAKE_BUILD_TYPE`, so `assert()` is **live**).
//!   * `c_lib_ndebug()` - the same unmodified `c_src/src/lib.c` compiled with
//!                        `gcc -O0 -fPIC -DNDEBUG`, i.e. the identical build
//!                        with `assert()` compiled out.  Selected automatically
//!                        when the `c-asserts` feature is off.
//!   * `c_lib_variant()`- the same source again with a different stack frame
//!                        layout; the undefined-behaviour oracle.
//!   * `rust_lib()`     - `translation/target/release/libunfilter_lib.so`,
//!                        rebuilt with this test binary's feature set.
//!
//! Every case runs in a `fork()`ed child on a shared-memory scratch region, so
//! that (a) a crash or a hang in either library cannot take the test runner
//! down, (b) the two runs see byte-identical memory *around* the nominal
//! buffers (the region is fully zeroed before each run and the input is placed
//! at the same offset and 4-byte alignment), and (c) writes the child performs
//! on the exported tables cannot leak into another case.
//!
//! `run()` is serialised on the mutex that guards that one region: forking from
//! several threads at once, with a fresh mapping per case, lets the kernel hand
//! the same address to a second thread while the first thread's child is still
//! writing to the old mapping.

#![allow(dead_code)]

pub mod deflate;

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// paths / building
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn repo_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

fn newer(a: &Path, b: &Path) -> bool {
    let ma = match std::fs::metadata(a) {
        Ok(m) => m.modified().unwrap(),
        Err(_) => return false,
    };
    let mb = match std::fs::metadata(b) {
        Ok(m) => m.modified().unwrap(),
        Err(_) => return true,
    };
    ma >= mb
}

fn glob_one(dir: &Path, prefix: &str, suffix: &str) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with(prefix) && n.ends_with(suffix)
        })
        .collect();
    hits.sort();
    hits.pop()
}

/// The C reference library, built exactly as the task describes.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let c_src = repo_root().join("c_src");
    // 1. the in-tree build the task documents
    if let Some(p) = glob_one(&c_src.join("build"), "lib", ".so") {
        if newer(&p, &c_src.join("src/lib.c")) {
            return p;
        }
    }
    // 2. otherwise configure + build out-of-tree (nothing under c_src/ is touched)
    let bdir = manifest_dir().join("target/cbuild");
    std::fs::create_dir_all(&bdir).unwrap();
    let st = Command::new("cmake")
        .arg("-S")
        .arg(&c_src)
        .arg("-B")
        .arg(&bdir)
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output()
        .expect("cmake not found");
    assert!(st.status.success(), "cmake configure failed: {}", String::from_utf8_lossy(&st.stderr));
    let st = Command::new("cmake").arg("--build").arg(&bdir).output().unwrap();
    assert!(st.status.success(), "cmake build failed: {}", String::from_utf8_lossy(&st.stderr));
    glob_one(&bdir, "lib", ".so").expect("no C .so produced")
}

/// The very same C source compiled with `-DNDEBUG` (asserts compiled out).
pub fn c_ndebug_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_NDEBUG") {
        return PathBuf::from(p);
    }
    let c_src = repo_root().join("c_src");
    let out = manifest_dir().join("target/libc_ndebug.so");
    if !newer(&out, &c_src.join("src/lib.c")) {
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        let st = Command::new("gcc")
            .args(["-O0", "-fPIC", "-DNDEBUG", "-shared"])
            .arg("-I")
            .arg(c_src.join("include"))
            .arg("-I")
            .arg(c_src.join("src"))
            .arg(c_src.join("src/lib.c"))
            .arg("-o")
            .arg(&out)
            .arg("-lm")
            .output()
            .expect("gcc not found");
        assert!(st.status.success(), "gcc failed: {}", String::from_utf8_lossy(&st.stderr));
    }
    out
}

/// A **stack-layout variant** of the reference C library: the same unmodified
/// `c_src/src/lib.c`, the same `NDEBUG` setting, but compiled with
/// `-fstack-protector-all -fno-omit-frame-pointer --param=ssp-buffer-size=1`,
/// which changes where gcc places the function-local variables.
///
/// This is the harness's *undefined-behaviour oracle*: for an input on which the
/// C code is well defined, the two C builds must behave identically, because
/// nothing observable depends on the frame layout.  If they differ, the input
/// reaches one of `c_src`'s out-of-bounds stack accesses (`cp_dynamic`'s
/// `lens[n]` run overshooting `uint8_t lens[288+32]`, its `lens[-1]` read, or
/// `cp_build`'s `counts[lens[n]]++`), and no translation can be expected to
/// reproduce a particular compiler's frame.
pub fn c_variant_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_VARIANT") {
        return PathBuf::from(p);
    }
    let c_src = repo_root().join("c_src");
    let out = manifest_dir().join(if C_ASSERTS {
        "target/libc_variant.so"
    } else {
        "target/libc_variant_ndebug.so"
    });
    if !newer(&out, &c_src.join("src/lib.c")) {
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        let mut cmd = Command::new("gcc");
        cmd.args([
            "-O0",
            "-fPIC",
            "-shared",
            "-fstack-protector-all",
            "-fno-omit-frame-pointer",
            "--param=ssp-buffer-size=1",
        ]);
        if !C_ASSERTS {
            cmd.arg("-DNDEBUG");
        }
        let st = cmd
            .arg("-I")
            .arg(c_src.join("include"))
            .arg("-I")
            .arg(c_src.join("src"))
            .arg(c_src.join("src/lib.c"))
            .arg("-o")
            .arg(&out)
            .arg("-lm")
            .output()
            .expect("gcc not found");
        assert!(st.status.success(), "gcc failed: {}", String::from_utf8_lossy(&st.stderr));
    }
    out
}

/// `true` when this test binary was compiled with the `c-asserts` feature, i.e.
/// when the Rust `.so` under test reproduces the *assert-enabled* C build.
pub const C_ASSERTS: bool = cfg!(feature = "c-asserts");

static RUST_BUILD: OnceLock<PathBuf> = OnceLock::new();

pub fn rust_so_path() -> PathBuf {
    // `cargo test` does not build the `cdylib` artifact by itself, so build it
    // here (once per process).
    RUST_BUILD
        .get_or_init(|| {
            if let Ok(p) = std::env::var("TRANSLATION_SO") {
                return PathBuf::from(p);
            }
            let out = manifest_dir().join("target/release/libunfilter_lib.so");
            // Always re-run cargo: the feature selection of *this* test binary
            // has to be mirrored onto the cdylib, and a stale `.so` from the
            // other feature set would silently test the wrong library.
            let mut cmd = Command::new(env!("CARGO"));
            cmd.args(["build", "--release", "--offline"]);
            if !C_ASSERTS {
                cmd.arg("--no-default-features");
            }
            let st = cmd
                .current_dir(manifest_dir())
                .output()
                .expect("cargo not found");
            assert!(
                st.status.success(),
                "cargo build --release failed: {}",
                String::from_utf8_lossy(&st.stderr)
            );
            assert!(out.exists(), "{} was not produced", out.display());
            out
        })
        .clone()
}

// ---------------------------------------------------------------------------
// loaded libraries
// ---------------------------------------------------------------------------

pub type UnfilterFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int;
pub type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Table {
    FixedTable,
    PermutationOrder,
    LenExtraBits,
    LenBase,
    DistExtraBits,
    DistBase,
}

impl Table {
    pub const ALL: [Table; 6] = [
        Table::FixedTable,
        Table::PermutationOrder,
        Table::LenExtraBits,
        Table::LenBase,
        Table::DistExtraBits,
        Table::DistBase,
    ];
    pub fn sym(self) -> &'static [u8] {
        match self {
            Table::FixedTable => b"cp_fixed_table\0",
            Table::PermutationOrder => b"cp_permutation_order\0",
            Table::LenExtraBits => b"cp_len_extra_bits\0",
            Table::LenBase => b"cp_len_base\0",
            Table::DistExtraBits => b"cp_dist_extra_bits\0",
            Table::DistBase => b"cp_dist_base\0",
        }
    }
    /// Size in bytes, straight out of the C declarations.
    pub fn bytes(self) -> usize {
        match self {
            Table::FixedTable => 288 + 32,
            Table::PermutationOrder => 19,
            Table::LenExtraBits => 29 + 2,
            Table::LenBase => (29 + 2) * 4,
            Table::DistExtraBits => 30 + 2,
            Table::DistBase => (30 + 2) * 4,
        }
    }
}

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub unfilter: UnfilterFn,
    pub cp_inflate: InflateFn,
    pub cp_error_reason: *mut *const c_char,
    tables: [*mut u8; 6],
}

unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

impl Lib {
    pub fn table(&self, t: Table) -> *mut u8 {
        self.tables[Table::ALL.iter().position(|x| *x == t).unwrap()]
    }
    pub fn read_table(&self, t: Table) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.table(t), t.bytes()).to_vec() }
    }
}

fn load(name: &'static str, path: PathBuf) -> Lib {
    unsafe {
        let lib = libloading::Library::new(&path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let lib: &'static libloading::Library = Box::leak(Box::new(lib));
        let unfilter: libloading::Symbol<UnfilterFn> =
            lib.get(b"unfilter\0").expect("no `unfilter` symbol");
        let cp_inflate: libloading::Symbol<InflateFn> =
            lib.get(b"cp_inflate\0").expect("no `cp_inflate` symbol");
        let err: libloading::Symbol<*mut *const c_char> =
            lib.get(b"cp_error_reason\0").expect("no `cp_error_reason` symbol");
        let mut tables = [std::ptr::null_mut(); 6];
        for (i, t) in Table::ALL.iter().enumerate() {
            let s: libloading::Symbol<*mut u8> = lib
                .get(t.sym())
                .unwrap_or_else(|e| panic!("no `{}` symbol: {e}", String::from_utf8_lossy(t.sym())));
            tables[i] = *s;
        }
        Lib {
            name,
            path,
            unfilter: *unfilter,
            cp_inflate: *cp_inflate,
            cp_error_reason: *err,
            tables,
        }
    }
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static C_LIB_ND: OnceLock<Lib> = OnceLock::new();
static C_LIB_VAR: OnceLock<Lib> = OnceLock::new();
static R_LIB: OnceLock<Lib> = OnceLock::new();

/// See `c_variant_so_path`.
pub fn c_lib_variant() -> &'static Lib {
    C_LIB_VAR.get_or_init(|| load("C(layout-variant)", c_variant_so_path()))
}

/// The UB oracle: `true` when the C source's behaviour on this input depends on
/// the compiler's stack frame layout, i.e. when the input reaches one of
/// `c_src`'s out-of-bounds stack accesses.
pub fn is_layout_dependent(case: &Case) -> bool {
    let a = run(c_ref(), case);
    is_layout_dependent_given(case, &a)
}

/// Same, reusing an already-computed reference outcome.
pub fn is_layout_dependent_given(case: &Case, reference: &Outcome) -> bool {
    &run(c_lib_variant(), case) != reference
}

pub fn c_lib() -> &'static Lib {
    C_LIB.get_or_init(|| load("C(assert)", c_so_path()))
}
pub fn c_lib_ndebug() -> &'static Lib {
    C_LIB_ND.get_or_init(|| load("C(NDEBUG)", c_ndebug_so_path()))
}
pub fn rust_lib() -> &'static Lib {
    R_LIB.get_or_init(|| load("Rust", rust_so_path()))
}

/// The C library that corresponds to the Rust `.so` currently under test:
/// the assert-enabled reference build for the default feature set, the
/// `-DNDEBUG` build for `--no-default-features`.
pub fn c_ref() -> &'static Lib {
    if C_ASSERTS {
        c_lib()
    } else {
        c_lib_ndebug()
    }
}

// ---------------------------------------------------------------------------
// shared-memory scratch + forked runner
// ---------------------------------------------------------------------------

const HDR_SIZE: usize = 4096;
const ERR_CAP: usize = 1024;

#[repr(C)]
struct Hdr {
    done: i32,
    ret: i32,
    err_len: i32, // -1 == cp_error_reason was NULL
    err: [u8; ERR_CAP],
}

/// One process-wide shared-memory scratch region, mapped once and reused by
/// every case.
///
/// It is deliberately *never* unmapped while the process lives: mapping and
/// unmapping a fresh region per case lets the kernel hand the same address out
/// again, and a `fork()`ed child then keeps writing into the *old* mapping while
/// its parent already sees a fresh, zero-filled one.  Reusing one region also
/// removes two syscalls per case.
struct Shm {
    base: *mut u8,
    total: usize,
    scratch_len: usize,
}

unsafe impl Send for Shm {}

impl Shm {
    /// Grow (never shrink) so that `scratch_len` bytes of scratch fit, and
    /// zero the part of the region this case will use.
    fn prepare(&mut self, scratch_len: usize) {
        let pages = scratch_len / 4096 + 2;
        let want = HDR_SIZE + pages * 4096;
        if want > self.total {
            unsafe {
                if !self.base.is_null() {
                    libc::munmap(self.base as *mut c_void, self.total);
                }
                let p = libc::mmap(
                    std::ptr::null_mut(),
                    want,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                );
                assert!(p != libc::MAP_FAILED, "mmap({want}) failed");
                self.base = p as *mut u8;
                self.total = want;
            }
        }
        self.scratch_len = scratch_len;
        // The *whole* region is zeroed, not just the part this case declares:
        // the C code over-reads and over-writes past the nominal buffers, and
        // both libraries must see identical bytes there.  (The region only ever
        // grows, so a later small case would otherwise inherit a big case's
        // leftovers - and the second of the two runs would inherit the first
        // run's.)
        unsafe { std::ptr::write_bytes(self.base, 0, self.total) };
    }
    fn hdr(&self) -> *mut Hdr {
        self.base as *mut Hdr
    }
    /// Page-aligned start of the scratch region.
    fn scratch(&self) -> *mut u8 {
        unsafe { self.base.add(HDR_SIZE) }
    }
}

/// Serialises every `run()`: only one `fork()` at a time, and the scratch
/// region is single-instance.
static SHM: std::sync::Mutex<Shm> =
    std::sync::Mutex::new(Shm { base: std::ptr::null_mut(), total: 0, scratch_len: 0 });

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Status {
    Exited(i32),
    Signaled(i32),
}

#[derive(Clone, Eq)]
pub struct Outcome {
    pub status: Status,
    pub ret: i32,
    pub err: Option<Vec<u8>>,
    pub scratch: Vec<u8>,
    /// Everything the call wrote to stderr (the `assert()` diagnostic, if any).
    /// Not compared directly - the program name and the source directory
    /// differ between the two libraries - see `assert_msg`.
    pub stderr: Vec<u8>,
    /// The `assert()` diagnostic with the program name and the source
    /// directory stripped, e.g.
    /// ``lib.c:217: cp_decode: Assertion `(search >> len) == (key >> len)' failed.``
    /// This makes "the *same* assert fired" a checkable property.
    pub assert_msg: Option<String>,
}

impl PartialEq for Outcome {
    fn eq(&self, o: &Outcome) -> bool {
        self.status == o.status
            && self.ret == o.ret
            && self.err == o.err
            && self.scratch == o.scratch
            && self.assert_msg == o.assert_msg
    }
}

/// `"{prog}: {dir}/lib.c:{line}: {func}: Assertion `{expr}' failed."`
/// -> `"lib.c:{line}: {func}: Assertion `{expr}' failed."`
pub fn normalize_assert(raw: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(raw).to_string();
    let line = text.lines().find(|l| l.contains("Assertion `"))?;
    let rest = line.splitn(2, ": ").nth(1)?;
    let (path, tail) = rest.split_once(':')?;
    let base = path.rsplit('/').next().unwrap_or(path);
    Some(format!("{base}:{tail}"))
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Outcome {{ status: {:?}, ret: {}, err: {:?}, assert: {:?}, scratch: {} }}",
            self.status,
            self.ret,
            self.err.as_ref().map(|e| String::from_utf8_lossy(e).to_string()),
            self.assert_msg,
            hex(&self.scratch)
        )
    }
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::new();
    for (i, x) in b.iter().enumerate() {
        if i == 96 {
            s.push_str(&format!("... (+{} bytes)", b.len() - 96));
            break;
        }
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// A table override applied (in the forked child, i.e. in isolation) before
/// the call.
#[derive(Clone, Debug)]
pub struct TableWrite {
    pub table: Table,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub enum Call {
    Unfilter { w: i32, h: i32, bpp: i32, raw_off: isize, null_raw: bool },
    Inflate { in_off: isize, in_bytes: i32, out_off: isize, out_bytes: i32, null_in: bool, null_out: bool },
}

#[derive(Clone, Debug)]
pub struct Case {
    /// Full prefill of the shared scratch region; its length *is* the scratch
    /// length, so both libraries see the same bytes before *and* after the
    /// nominal buffers.
    pub scratch: Vec<u8>,
    pub call: Call,
    pub tables: Vec<TableWrite>,
    /// Seconds before SIGALRM kills the child (infinite loops are possible in
    /// both libraries for corrupt input).
    pub timeout: u32,
}

impl Case {
    pub fn unfilter(scratch: Vec<u8>, w: i32, h: i32, bpp: i32, raw_off: isize) -> Case {
        Case {
            scratch,
            call: Call::Unfilter { w, h, bpp, raw_off, null_raw: false },
            tables: vec![],
            timeout: 4,
        }
    }
    pub fn unfilter_null(w: i32, h: i32, bpp: i32) -> Case {
        Case {
            scratch: vec![0u8; 64],
            call: Call::Unfilter { w, h, bpp, raw_off: 0, null_raw: true },
            tables: vec![],
            timeout: 4,
        }
    }
    pub fn inflate(scratch: Vec<u8>, in_off: isize, in_bytes: i32, out_off: isize, out_bytes: i32) -> Case {
        Case {
            scratch,
            call: Call::Inflate { in_off, in_bytes, out_off, out_bytes, null_in: false, null_out: false },
            tables: vec![],
            timeout: 4,
        }
    }
    pub fn with_table(mut self, table: Table, bytes: Vec<u8>) -> Case {
        self.tables.push(TableWrite { table, bytes });
        self
    }
    pub fn with_null_in(mut self) -> Case {
        if let Call::Inflate { null_in, .. } = &mut self.call {
            *null_in = true;
        }
        self
    }
    pub fn with_null_out(mut self) -> Case {
        if let Call::Inflate { null_out, .. } = &mut self.call {
            *null_out = true;
        }
        self
    }
    pub fn with_timeout(mut self, t: u32) -> Case {
        self.timeout = t;
        self
    }
}

/// Run one case against one library inside a forked child.
pub fn run(lib: &Lib, case: &Case) -> Outcome {
    // Held for the whole call: exactly one fork() at a time, and one scratch
    // region for the whole process (see `Shm`).
    let mut guard = SHM.lock().unwrap_or_else(|e| e.into_inner());
    let shm: &mut Shm = &mut guard;
    shm.prepare(case.scratch.len().max(1));
    unsafe {
        std::ptr::copy_nonoverlapping(case.scratch.as_ptr(), shm.scratch(), case.scratch.len());
        let h = shm.hdr();
        (*h).done = 0;
        (*h).ret = i32::MIN;
        (*h).err_len = -2;
    }

    // capture the child's stderr (the assert diagnostic) through a pipe
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    let (rd, wr) = (fds[0], fds[1]);

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // ---- child ----
        unsafe {
            libc::close(rd);
            libc::dup2(wr, 2);
            libc::close(wr);
            // Crashing (SIGSEGV / SIGABRT from a live `assert()`) is an
            // *expected* outcome for several rows; make sure it stays cheap by
            // disabling core dumps.
            let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
            libc::alarm(case.timeout);
            for tw in &case.tables {
                assert!(tw.bytes.len() <= tw.table.bytes());
                std::ptr::copy_nonoverlapping(tw.bytes.as_ptr(), lib.table(tw.table), tw.bytes.len());
            }
            *lib.cp_error_reason = std::ptr::null();
            let base = shm.scratch();
            let ret = match case.call {
                Call::Unfilter { w, h, bpp, raw_off, null_raw } => {
                    let p = if null_raw { std::ptr::null_mut() } else { base.offset(raw_off) };
                    (lib.unfilter)(w, h, bpp, p)
                }
                Call::Inflate { in_off, in_bytes, out_off, out_bytes, null_in, null_out } => {
                    let i = if null_in { std::ptr::null_mut() } else { base.offset(in_off) as *mut c_void };
                    let o = if null_out { std::ptr::null_mut() } else { base.offset(out_off) as *mut c_void };
                    (lib.cp_inflate)(i, in_bytes, o, out_bytes)
                }
            };
            let h = shm.hdr();
            (*h).ret = ret;
            let e = *lib.cp_error_reason;
            if e.is_null() {
                (*h).err_len = -1;
            } else {
                let mut n = 0usize;
                while n < ERR_CAP - 1 && *e.add(n) != 0 {
                    n += 1;
                }
                std::ptr::copy_nonoverlapping(e as *const u8, (*h).err.as_mut_ptr(), n);
                (*h).err_len = n as i32;
            }
            (*h).done = 1;
            libc::_exit(0);
        }
    }

    // ---- parent ----
    // Drain the pipe *before* waiting, so a chatty child cannot deadlock.
    let mut stderr_buf: Vec<u8> = Vec::new();
    unsafe {
        libc::close(wr);
        let mut buf = [0u8; 4096];
        loop {
            let n = libc::read(rd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            if stderr_buf.len() < 64 * 1024 {
                stderr_buf.extend_from_slice(&buf[..n as usize]);
            }
        }
        libc::close(rd);
    }

    let mut wstatus: c_int = 0;
    let r = unsafe { libc::waitpid(pid, &mut wstatus, 0) };
    assert_eq!(r, pid, "waitpid failed");
    let status = if libc::WIFEXITED(wstatus) {
        Status::Exited(libc::WEXITSTATUS(wstatus))
    } else if libc::WIFSIGNALED(wstatus) {
        Status::Signaled(libc::WTERMSIG(wstatus))
    } else {
        Status::Exited(-999)
    };
    unsafe {
        let h = shm.hdr();
        let done = (*h).done == 1;
        // A child that exited *normally* without filling the header means the
        // harness itself misbehaved (e.g. it panicked inside the child).  Never
        // silently report that as a library outcome.
        if !done && matches!(status, Status::Exited(_)) {
            panic!(
                "HARNESS ERROR: the forked child for {:?} exited with {:?} without \
                 completing the call.\n  child stderr: {}",
                case.call,
                status,
                String::from_utf8_lossy(&stderr_buf)
            );
        }
        let ret = if done { (*h).ret } else { 0 };
        let err = if !done {
            None
        } else if (*h).err_len < 0 {
            None
        } else {
            Some((&(*h).err)[..(*h).err_len as usize].to_vec())
        };
        let scratch = std::slice::from_raw_parts(shm.scratch(), shm.scratch_len)[..case.scratch.len()].to_vec();
        let assert_msg = normalize_assert(&stderr_buf);
        Outcome { status, ret, err, scratch, stderr: stderr_buf, assert_msg }
    }
}

/// Run the same case against both libraries and require bit-identical results.
#[track_caller]
pub fn diff(case: &Case, ctx: &str) -> Outcome {
    diff_against(c_ref(), case, ctx)
}

/// Like `diff`, but a divergence is tolerated when the UB oracle proves that
/// the C code's behaviour on this input depends on the compiler's stack frame
/// layout.  Returns `(outcome, was_ub)`.
#[track_caller]
pub fn diff_or_ub(case: &Case, ctx: &str) -> (Outcome, bool) {
    let a = run(c_ref(), case);
    let b = run(rust_lib(), case);
    if a == b {
        return (a, false);
    }
    assert!(
        is_layout_dependent_given(case, &a),
        "DIVERGENCE [{ctx}] that the UB oracle does *not* explain\n  case: {:?}\n  \
         {:>9}: {a:?}\n  {:>9}: {b:?}",
        case.call,
        c_ref().name,
        "Rust"
    );
    (a, true)
}

#[track_caller]
pub fn diff_against(cl: &Lib, case: &Case, ctx: &str) -> Outcome {
    let a = run(cl, case);
    let b = run(rust_lib(), case);
    if a != b {
        let mut msg = format!(
            "DIVERGENCE [{ctx}]\n  case: {:?}\n  {:>9}: {:?}\n  {:>9}: {:?}\n",
            case.call, cl.name, a, "Rust", b
        );
        if a.assert_msg != b.assert_msg {
            msg += &format!(
                "  {} stderr: {}\n  Rust stderr: {}\n",
                cl.name,
                String::from_utf8_lossy(&a.stderr).trim(),
                String::from_utf8_lossy(&b.stderr).trim()
            );
        }
        if a.scratch.len() == b.scratch.len() {
            let mut shown = 0;
            for i in 0..a.scratch.len() {
                if a.scratch[i] != b.scratch[i] {
                    msg += &format!(
                        "  first scratch mismatch at +{i}: {} = {:02x}, Rust = {:02x}\n",
                        cl.name, a.scratch[i], b.scratch[i]
                    );
                    shown += 1;
                    if shown == 8 {
                        break;
                    }
                }
            }
        }
        panic!("{msg}");
    }
    a
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64)
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as u32) as i32
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u32) as usize]
    }
}
