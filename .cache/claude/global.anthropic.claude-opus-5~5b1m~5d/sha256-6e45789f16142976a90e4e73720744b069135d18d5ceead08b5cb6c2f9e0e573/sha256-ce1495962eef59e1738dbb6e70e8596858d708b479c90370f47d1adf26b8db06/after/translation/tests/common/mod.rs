//! Differential-test harness.
//!
//! Both the C `.so` (built by `c_src/CMakeLists.txt`) and the Rust `.so`
//! (`cdylib` produced by this crate) are loaded with `libloading` and driven
//! purely through their exported C symbols. No Rust function is ever called
//! directly, so the `#[no_mangle] extern "C"` wrappers are part of what is
//! under test.
//!
//! Isolation: every scenario gets a *private copy* of each `.so`, dlopen'd from
//! a unique path. glibc keys already-loaded objects on (dev, ino), so a copy is
//! a genuinely fresh mapping with fresh `static` state (`logger.c`'s
//! `log_file`). That makes the "logger was never initialised" paths testable
//! and stops scenarios from leaking state into each other.
//!
//! These tests mutate process-global state (environment, fds 1/2, cwd), so they
//! all take a global mutex. Run with `-- --test-threads=1` for the fork-based
//! error-path tests.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------- libc glue --

pub type FILE = c_void;

unsafe extern "C" {
    pub fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    pub fn unsetenv(name: *const c_char) -> c_int;
    pub fn fflush(stream: *mut FILE) -> c_int;
    pub fn dup(oldfd: c_int) -> c_int;
    pub fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    pub fn fork() -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    pub fn _exit(status: c_int) -> !;
}

const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ------------------------------------------------------------------ C types --

/// `typedef struct { char description[256]; int priority; } Task;`
#[repr(C)]
pub struct Task {
    pub description: [c_char; 256],
    pub priority: c_int,
}

/// `typedef struct { Task *tasks; int max_tasks; int task_count; } TaskManager;`
#[repr(C)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: c_int,
    pub task_count: c_int,
}

// ------------------------------------------------------------------ the API --

/// Every exported symbol of the library, resolved by name out of a `.so`.
pub struct Api {
    _lib: Library,
    pub which: &'static str,
    pub initialize_logger: unsafe extern "C" fn() -> c_int,
    pub log_info: unsafe extern "C" fn(*const c_char),
    pub log_warning: unsafe extern "C" fn(*const c_char),
    pub log_error: unsafe extern "C" fn(*const c_char),
    pub finalize_logger: unsafe extern "C" fn(),
    pub create_task_manager: unsafe extern "C" fn() -> *mut TaskManager,
    pub add_task: unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int),
    pub print_tasks: unsafe extern "C" fn(*const TaskManager),
    pub destroy_task_manager: unsafe extern "C" fn(*mut TaskManager),
}

impl Api {
    fn load(path: &Path, which: &'static str) -> Api {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

        macro_rules! sym {
            ($name:literal, $t:ty) => {{
                let s: Symbol<$t> = unsafe { lib.get(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|e| panic!("{} missing `{}`: {e}", which, $name));
                *s
            }};
        }

        let initialize_logger = sym!("initialize_logger", unsafe extern "C" fn() -> c_int);
        let log_info = sym!("log_info", unsafe extern "C" fn(*const c_char));
        let log_warning = sym!("log_warning", unsafe extern "C" fn(*const c_char));
        let log_error = sym!("log_error", unsafe extern "C" fn(*const c_char));
        let finalize_logger = sym!("finalize_logger", unsafe extern "C" fn());
        let create_task_manager =
            sym!("create_task_manager", unsafe extern "C" fn() -> *mut TaskManager);
        let add_task = sym!(
            "add_task",
            unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int)
        );
        let print_tasks = sym!("print_tasks", unsafe extern "C" fn(*const TaskManager));
        let destroy_task_manager =
            sym!("destroy_task_manager", unsafe extern "C" fn(*mut TaskManager));

        Api {
            _lib: lib,
            which,
            initialize_logger,
            log_info,
            log_warning,
            log_error,
            finalize_logger,
            create_task_manager,
            add_task,
            print_tasks,
            destroy_task_manager,
        }
    }

    /// `driver` is resolved lazily because a few scenarios don't need it.
    pub fn driver(&self) -> unsafe extern "C" fn(*const c_char) -> c_int {
        let s: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            unsafe { self._lib.get(b"driver\0") }
                .unwrap_or_else(|e| panic!("{} missing `driver`: {e}", self.which));
        *s
    }
}

// ------------------------------------------------------------------- layout --

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

pub fn c_so() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built. Run:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub fn rust_so() -> PathBuf {
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>/libdriver.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let p = profile_dir.join("libdriver.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {}.\n\
         `cargo test` does NOT build a `crate-type = [\"cdylib\"]` library (the test\n\
         harness cannot link it), so it must be built explicitly first:\n  \
         cargo build{}\n\
         Use ./run_verification.sh, which does this for every profile.",
        p.display(),
        if profile_dir.ends_with("release") {
            " --release"
        } else {
            ""
        }
    );

    // Guard against silently testing a stale artifact: every `src/*.rs` must be
    // older than the `.so`.
    let so_mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
    if let Some(so_mtime) = so_mtime {
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        if let Ok(rd) = std::fs::read_dir(&src) {
            for e in rd.flatten() {
                let path = e.path();
                if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                    continue;
                }
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    assert!(
                        m <= so_mtime,
                        "STALE cdylib: {} is newer than {}. Re-run `cargo build` \
                         (see ./run_verification.sh).",
                        path.display(),
                        p.display()
                    );
                }
            }
        }
    }
    p
}

fn scratch_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let base = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let d = base.join(format!("driver-difftest-{}", std::process::id()));
        std::fs::create_dir_all(&d).expect("create scratch root");
        d
    })
}

fn next_id() -> u64 {
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

// --------------------------------------------------------------- global lock --

fn global_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    match L.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ------------------------------------------------------------- fresh mapping --

/// A pristine, independently-mapped pair of libraries plus a private scratch
/// directory. Dropping it dlcloses both and deletes the scratch tree.
pub struct Pair {
    pub c: Api,
    pub rs: Api,
    dir: PathBuf,
    _guard: MutexGuard<'static, ()>,
}

impl Pair {
    pub fn new(label: &str) -> Pair {
        let guard = global_lock();
        let id = next_id();
        let dir = scratch_root().join(format!("{id:04}-{}", sanitize(label)));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("c")).expect("mkdir c");
        std::fs::create_dir_all(dir.join("rs")).expect("mkdir rs");
        std::fs::create_dir_all(dir.join("shared")).expect("mkdir shared");

        // Private copies => distinct inodes => distinct dlopen'd objects with
        // fresh `static` state on every scenario.
        let cp = dir.join("libdriver_c.so");
        let rp = dir.join("libdriver_rs.so");
        std::fs::copy(c_so(), &cp).expect("copy c so");
        std::fs::copy(rust_so(), &rp).expect("copy rust so");

        Pair {
            c: Api::load(&cp, "C"),
            rs: Api::load(&rp, "Rust"),
            dir,
            _guard: guard,
        }
    }

    pub fn side(&self, which: Which) -> Side {
        Side {
            dir: self.dir.join(match which {
                Which::C => "c",
                Which::Rust => "rs",
            }),
            which,
        }
    }

    pub fn api(&self, which: Which) -> &Api {
        match which {
            Which::C => &self.c,
            Which::Rust => &self.rs,
        }
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Which {
    C,
    Rust,
}

/// Per-side sandbox: a private directory that also doubles as the cwd for the
/// `LOG_FILE`-unset scenario.
pub struct Side {
    pub dir: PathBuf,
    pub which: Which,
}

impl Side {
    pub fn log_path(&self) -> PathBuf {
        self.dir.join("app.log")
    }
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
    /// A path that is *identical* for both sides. Required whenever the path
    /// itself becomes part of the compared output — `initialize_logger` prints
    /// it via `fprintf(stderr, "Failed to open log file: %s\n", ...)`, so a
    /// per-side path would produce a spurious mismatch.
    pub fn shared(&self, name: &str) -> PathBuf {
        self.dir
            .parent()
            .expect("side dir has a parent")
            .join("shared")
            .join(name)
    }
}

// ------------------------------------------------------------------ env glue --

pub fn set_env(key: &str, value: &[u8]) {
    let k = CString::new(key).unwrap();
    let v = CString::new(value).unwrap();
    let rc = unsafe { setenv(k.as_ptr(), v.as_ptr(), 1) };
    assert_eq!(rc, 0, "setenv({key}) failed");
}

pub fn set_env_path(key: &str, p: &Path) {
    set_env(key, p.as_os_str().as_encoded_bytes());
}

pub fn unset_env(key: &str) {
    let k = CString::new(key).unwrap();
    unsafe { unsetenv(k.as_ptr()) };
}

// ---------------------------------------------------------------- observation --

/// Ordered log of everything a scenario observed. Compared verbatim between the
/// two implementations.
#[derive(Default, PartialEq, Eq)]
pub struct Record(pub Vec<String>);

impl Record {
    pub fn note(&mut self, s: impl Into<String>) {
        self.0.push(s.into());
    }
    pub fn kv(&mut self, k: &str, v: impl std::fmt::Debug) {
        self.0.push(format!("{k}={v:?}"));
    }
    /// Snapshot every observable field of a `TaskManager`, including all 256
    /// description bytes of every live task (`strncpy` zero-pads, so they are
    /// fully determined). The 4 padding bytes at the end of `Task` are not
    /// written by the C, so they are excluded.
    pub fn manager(&mut self, tag: &str, m: *const TaskManager) {
        if m.is_null() {
            self.note(format!("{tag}: NULL"));
            return;
        }
        unsafe {
            let mt = (*m).max_tasks;
            let tc = (*m).task_count;
            self.note(format!(
                "{tag}: max_tasks={mt} task_count={tc} tasks_null={}",
                (*m).tasks.is_null()
            ));
            if (*m).tasks.is_null() {
                return;
            }
            for i in 0..tc.max(0) {
                let t = (*m).tasks.offset(i as isize);
                let bytes: &[u8] = std::slice::from_raw_parts(
                    (&raw const (*t).description) as *const u8,
                    256,
                );
                self.note(format!(
                    "{tag}[{i}]: prio={} desc={}",
                    (*t).priority,
                    hex(bytes)
                ));
            }
        }
    }
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &x in b {
        match x {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(x as char),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    s
}

/// Everything one side produced.
pub struct Outcome {
    pub record: Record,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub files: BTreeMap<String, Vec<u8>>,
    pub status: Option<String>,
}

fn open_trunc(p: &Path) -> c_int {
    let cp = CString::new(p.as_os_str().as_encoded_bytes()).unwrap();
    let fd = unsafe { open(cp.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o600) };
    assert!(fd >= 0, "open({}) failed", p.display());
    fd
}

/// Run `f` with fds 1 and 2 redirected into the side's scratch dir, then
/// harvest stdout, stderr and every file the run left behind.
fn observe<F>(side: &Side, f: F) -> Outcome
where
    F: FnOnce(&mut Record),
{
    let op = side.dir.join(".stdout.bin");
    let ep = side.dir.join(".stderr.bin");

    unsafe { fflush(std::ptr::null_mut()) };
    let ofd = open_trunc(&op);
    let efd = open_trunc(&ep);
    let sout = unsafe { dup(1) };
    let serr = unsafe { dup(2) };
    assert!(sout >= 0 && serr >= 0);
    unsafe {
        dup2(ofd, 1);
        dup2(efd, 2);
    }

    let mut rec = Record::default();
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut rec)));

    // Flush *all* libc streams: stdout/stderr plus each library's `log_file`.
    unsafe { fflush(std::ptr::null_mut()) };
    unsafe {
        dup2(sout, 1);
        dup2(serr, 2);
        close(sout);
        close(serr);
        close(ofd);
        close(efd);
    }
    if let Err(p) = res {
        std::panic::resume_unwind(p);
    }

    let stdout = std::fs::read(&op).unwrap_or_default();
    let stderr = std::fs::read(&ep).unwrap_or_default();
    let mut files = BTreeMap::new();
    if let Ok(rd) = std::fs::read_dir(&side.dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with(".stdout") || name.starts_with(".stderr") {
                continue;
            }
            if e.path().is_file() {
                files.insert(name, std::fs::read(e.path()).unwrap_or_default());
            }
        }
    }
    Outcome {
        record: rec,
        stdout,
        stderr,
        files,
        status: None,
    }
}

// ------------------------------------------------------------------- compare --

fn diff_bytes(what: &str, label: &str, a: &[u8], b: &[u8], msgs: &mut Vec<String>) {
    if a == b {
        return;
    }
    let at = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    msgs.push(format!(
        "[{label}] {what} differ (C {} bytes, Rust {} bytes, first diff at byte {at})\n  \
         C   : {}\n  Rust: {}",
        a.len(),
        b.len(),
        show(&a[at.saturating_sub(24)..(at + 96).min(a.len())]),
        show(&b[at.saturating_sub(24)..(at + 96).min(b.len())]),
    ));
}

fn compare(label: &str, c: &Outcome, r: &Outcome) {
    let mut msgs: Vec<String> = Vec::new();

    if c.record != r.record {
        let n = c.record.0.len().max(r.record.0.len());
        for i in 0..n {
            let a = c.record.0.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
            let b = r.record.0.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
            if a != b {
                msgs.push(format!(
                    "[{label}] observation #{i} differs\n  C   : {a}\n  Rust: {b}"
                ));
            }
        }
    }
    diff_bytes("stdout", label, &c.stdout, &r.stdout, &mut msgs);
    diff_bytes("stderr", label, &c.stderr, &r.stderr, &mut msgs);
    if c.status != r.status {
        msgs.push(format!(
            "[{label}] process status differs\n  C   : {:?}\n  Rust: {:?}",
            c.status, r.status
        ));
    }

    let names: std::collections::BTreeSet<&String> = c.files.keys().chain(r.files.keys()).collect();
    for n in names {
        match (c.files.get(n), r.files.get(n)) {
            (Some(a), Some(b)) => diff_bytes(&format!("file {n:?}"), label, a, b, &mut msgs),
            (Some(_), None) => msgs.push(format!("[{label}] file {n:?} only produced by C")),
            (None, Some(_)) => msgs.push(format!("[{label}] file {n:?} only produced by Rust")),
            (None, None) => unreachable!(),
        }
    }

    if !msgs.is_empty() {
        panic!("DIFFERENTIAL MISMATCH in `{label}`:\n{}", msgs.join("\n"));
    }
}

/// Run one scenario against both libraries and assert byte-identical results.
///
/// The closure receives the `Api` loaded from the `.so` under test, a private
/// scratch `Side`, and a `Record` to note return values / struct state into.
pub fn differential<F>(label: &str, mut scenario: F)
where
    F: FnMut(&Api, &Side, &mut Record),
{
    let pair = Pair::new(label);
    let cs = pair.side(Which::C);
    let rs = pair.side(Which::Rust);
    let c_out = observe(&cs, |rec| scenario(&pair.c, &cs, rec));
    let r_out = observe(&rs, |rec| scenario(&pair.rs, &rs, rec));
    compare(label, &c_out, &r_out);
}

// --------------------------------------------------------------- fork variant --

fn wait_status_text(st: c_int) -> String {
    if st & 0x7f == 0 {
        format!("exited({})", (st >> 8) & 0xff)
    } else if (st & 0x7f) == 0x7f {
        format!("stopped({})", (st >> 8) & 0xff)
    } else {
        format!("signal({}){}", st & 0x7f, if st & 0x80 != 0 { "+core" } else { "" })
    }
}

/// Same as [`differential`] but runs the scenario body in a forked child, so
/// scenarios that make the C library dereference a null pointer (there are no
/// null checks anywhere in `c_src`) can be compared by *termination status* as
/// well as by the output produced before the crash.
///
/// Requires `--test-threads=1` (the test binary must be effectively
/// single-threaded at fork time).
pub fn differential_forked<F>(label: &str, mut scenario: F)
where
    F: FnMut(&Api, &Side),
{
    let pair = Pair::new(label);
    let cs = pair.side(Which::C);
    let rs = pair.side(Which::Rust);

    let mut run = |api: &Api, side: &Side| -> Outcome {
        let mut out = observe(side, |_rec| {
            unsafe { fflush(std::ptr::null_mut()) };
            let pid = unsafe { fork() };
            if pid == 0 {
                scenario(api, side);
                unsafe { fflush(std::ptr::null_mut()) };
                unsafe { _exit(0) };
            }
            assert!(pid > 0, "fork failed");
            let mut st: c_int = 0;
            unsafe { waitpid(pid, &mut st, 0) };
            // Stash the status where the caller can pick it up.
            LAST_STATUS.store(st as i64, Ordering::SeqCst);
        });
        out.status = Some(wait_status_text(LAST_STATUS.load(Ordering::SeqCst) as c_int));
        out
    };

    let c_out = run(&pair.c, &cs);
    let r_out = run(&pair.rs, &rs);
    compare(label, &c_out, &r_out);
}

static LAST_STATUS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Run `scenario` in `runs` forked children **per side** and return the observed
/// termination statuses `(c, rust)`.
///
/// Needed for the two `ERRORS.md` rows whose C behaviour is *undefined and
/// allocator-dependent* (use of a `FILE*` after `finalize_logger` has
/// `fclose`d it without resetting `log_file`). Those cases are not
/// reproducible even when the identical library is run twice in a row, so the
/// only honest differential assertion is that both implementations' outcomes
/// are drawn from the same set.
pub fn forked_statuses<F>(label: &str, runs: usize, mut scenario: F) -> (Vec<String>, Vec<String>)
where
    F: FnMut(&Api, &Side),
{
    let pair = Pair::new(label);
    let mut both: Vec<Vec<String>> = Vec::new();
    for which in [Which::C, Which::Rust] {
        let side = pair.side(which);
        let api = pair.api(which);
        let mut sts: Vec<String> = Vec::new();
        {
            let sts = &mut sts;
            let _ = observe(&side, |_rec| {
                for _ in 0..runs {
                    unsafe { fflush(std::ptr::null_mut()) };
                    let pid = unsafe { fork() };
                    if pid == 0 {
                        scenario(api, &side);
                        unsafe { fflush(std::ptr::null_mut()) };
                        unsafe { _exit(0) };
                    }
                    assert!(pid > 0, "fork failed");
                    let mut st: c_int = 0;
                    unsafe { waitpid(pid, &mut st, 0) };
                    sts.push(wait_status_text(st));
                }
            });
        }
        both.push(sts);
    }
    let rust = both.pop().unwrap();
    let c = both.pop().unwrap();
    (c, rust)
}

// ---------------------------------------------------------------------- PRNG --

/// xorshift64* — deterministic, seeded, no external crates.
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
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Random NUL-free byte string with a random length in `0..max`.
    pub fn bytes_upto(&mut self, max: u64, alphabet: &[u8]) -> Vec<u8> {
        let n = self.below(max) as usize;
        self.bytes(n, alphabet)
    }
    /// Random NUL-free byte string of the given length drawn from `alphabet`.
    pub fn bytes(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len)
            .map(|_| alphabet[self.below(alphabet.len() as u64) as usize])
            .collect()
    }
}

/// Printable ASCII plus format-specifier bait and high bytes — never NUL.
pub fn alphabet_wide() -> Vec<u8> {
    let mut v: Vec<u8> = (0x20u8..=0x7e).collect();
    v.extend_from_slice(b"\t\r%%%sd\x80\xa0\xfe\xff");
    v
}

/// Same, plus newline so `driver`'s line splitter gets exercised.
pub fn alphabet_lines() -> Vec<u8> {
    let mut v = alphabet_wide();
    // Weight '\n' heavily enough to produce many short lines.
    v.extend_from_slice(b"\n\n\n\n\n\n\n\n");
    v
}

pub fn cstr(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("test input must not contain NUL")
}
