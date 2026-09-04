//! Differential-test harness: loads the C `libpng.so` and the Rust
//! `liblibpng.so` through `libloading` and drives both through their exported
//! C ABI only.  Nothing in the Rust crate is called directly.
//!
//! Every scenario runs in a *child process* (a re-exec of this very test
//! binary).  That is what makes error-path testing possible without
//! `setjmp`/`longjmp` in Rust: the installed `png_error` callback appends the
//! message to the record, writes the record out and `_exit()`s.  The parent
//! then compares the two records byte for byte.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_double, c_int, c_void};
use std::fmt::Write as _;
use std::path::PathBuf;

pub mod api;
pub mod configs;
pub mod errors_tbl;
pub mod errscen;
pub mod mkpng;
pub mod pngdefs;
pub mod rng;
pub mod scen;

pub use api::Api;
#[allow(unused_imports)]
pub use pngdefs::*;
#[allow(unused_imports)]
pub use rng::Rng;

/* ------------------------------------------------------------------ */
/* which library                                                       */
/* ------------------------------------------------------------------ */

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Which {
    C,
    Rust,
}

impl Which {
    pub fn tag(self) -> &'static str {
        match self {
            Which::C => "c",
            Which::Rust => "rust",
        }
    }
    pub fn parse(s: &str) -> Which {
        match s {
            "c" => Which::C,
            "rust" => Which::Rust,
            _ => panic!("bad lib tag {s}"),
        }
    }
    pub fn so_path(self) -> PathBuf {
        match self {
            Which::C => {
                let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                p.pop();
                p.push("c_src");
                p.push("build");
                p.push("libpng.so");
                p
            }
            Which::Rust => {
                // target/<profile>/deps/<test>-<hash>  ->  target/<profile>/liblibpng.so
                let mut p = std::env::current_exe().expect("current_exe");
                p.pop(); // deps
                p.pop(); // profile dir
                p.push("liblibpng.so");
                p
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* libc bits we need                                                   */
/* ------------------------------------------------------------------ */

extern "C" {
    fn _exit(code: c_int) -> !;
    fn floor(x: c_double) -> c_double;
    fn pow(x: c_double, y: c_double) -> c_double;
    fn atof(s: *const c_char) -> c_double;
    fn setrlimit(resource: c_int, rlim: *const Rlimit) -> c_int;
}

#[repr(C)]
struct Rlimit {
    cur: u64,
    max: u64,
}

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

/// `fopen`/`fclose` for the stdio-based libpng entry points.  Both libraries are
/// handed the *same* `FILE*` created by the test process.
pub unsafe fn api_fopen(path: *const c_char, mode: *const c_char) -> *mut c_void {
    fopen(path, mode)
}
pub unsafe fn api_fclose(f: *mut c_void) -> c_int {
    if f.is_null() { 0 } else { fclose(f) }
}

/// Several scenarios make libpng dereference a NULL pointer (which is exactly
/// what the C does, e.g. `png_write_image(png, NULL)`).  Writing a core dump for
/// each of those costs about a second, which under parallel test execution can
/// look like a hang.  Turn core dumps off for the whole test process tree.
pub fn disable_core_dumps() {
    static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        const RLIMIT_CORE: c_int = 4;
        let r = Rlimit { cur: 0, max: 0 };
        unsafe {
            setrlimit(RLIMIT_CORE, &r);
        }
    });
}

/// The C `libpng.so` is linked without `-lm` (see `c_src/CMakeLists.txt`), so it
/// has unversioned undefined references to `floor`/`pow`.  Referencing them here
/// puts `libm` in this binary's `NEEDED` list and therefore in the global symbol
/// scope that `dlopen` searches.
#[inline(never)]
pub fn keep_libm_alive() {
    unsafe {
        let x = floor(1.5) + pow(2.0, 3.0) + atof(b"1.0\0".as_ptr() as *const c_char);
        std::hint::black_box(x);
    }
}

/* ------------------------------------------------------------------ */
/* record                                                              */
/* ------------------------------------------------------------------ */

#[derive(Default)]
pub struct Rec {
    pub s: String,
}

impl Rec {
    pub fn new() -> Rec {
        Rec { s: String::new() }
    }
    pub fn line(&mut self, l: &str) {
        self.s.push_str(l);
        self.s.push('\n');
    }
    pub fn kv(&mut self, k: &str, v: impl std::fmt::Display) {
        let _ = writeln!(self.s, "{k}={v}");
    }
    pub fn bytes(&mut self, tag: &str, b: &[u8]) {
        let _ = write!(self.s, "{tag}[{}]=", b.len());
        for x in b {
            let _ = write!(self.s, "{x:02x}");
        }
        self.s.push('\n');
    }
    /// Compact digest for very large blobs (still exact: length + FNV-1a-64).
    pub fn digest(&mut self, tag: &str, b: &[u8]) {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &x in b {
            h ^= x as u64;
            h = h.wrapping_mul(0x1000_0000_01b3);
        }
        let _ = writeln!(self.s, "{tag}[{}]#{:016x}", b.len(), h);
    }
    pub fn cstr(&mut self, tag: &str, p: *const c_char) {
        if p.is_null() {
            self.kv(tag, "<null>");
        } else {
            let s = unsafe { std::ffi::CStr::from_ptr(p) };
            let _ = writeln!(self.s, "{tag}={:?}", s.to_string_lossy());
        }
    }
}

/* ------------------------------------------------------------------ */
/* per-child global state                                              */
/* ------------------------------------------------------------------ */

pub struct Globals {
    pub api: *const Api,
    pub rec: *mut Rec,
    pub out: String,
    /// bytes produced by the write callback
    pub wbuf: Vec<u8>,
    /// source for the read callback
    pub rbuf: Vec<u8>,
    pub rpos: usize,
    pub flushes: u32,
    /// short-read behaviour: 0 = png_error, 1 = zero-fill silently
    pub short_read_mode: u32,
    /// scratch used by progressive-read / user callbacks
    pub notes: Vec<String>,
}

impl Globals {
    fn new() -> Globals {
        Globals {
            api: std::ptr::null(),
            rec: std::ptr::null_mut(),
            out: String::new(),
            wbuf: Vec::new(),
            rbuf: Vec::new(),
            rpos: 0,
            flushes: 0,
            short_read_mode: 0,
            notes: Vec::new(),
        }
    }
}

static mut G_PTR: *mut Globals = std::ptr::null_mut();

#[inline]
pub fn g() -> &'static mut Globals {
    unsafe { &mut *G_PTR }
}

#[inline]
pub fn api() -> &'static Api {
    unsafe { &*g().api }
}

#[inline]
pub fn rec() -> &'static mut Rec {
    unsafe { &mut *g().rec }
}

/* ------------------------------------------------------------------ */
/* callbacks shared by both libraries                                  */
/* ------------------------------------------------------------------ */

pub unsafe extern "C" fn cb_error(_png: *mut c_void, msg: *const c_char) {
    let m = if msg.is_null() {
        "<null>".to_string()
    } else {
        std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned()
    };
    rec().line(&format!("ERROR {m}"));
    finish(70);
}

pub unsafe extern "C" fn cb_warn(_png: *mut c_void, msg: *const c_char) {
    let m = if msg.is_null() {
        "<null>".to_string()
    } else {
        std::ffi::CStr::from_ptr(msg).to_string_lossy().into_owned()
    };
    rec().line(&format!("WARN {m}"));
}

pub unsafe extern "C" fn cb_write(_png: *mut c_void, data: *mut u8, len: usize) {
    let gg = g();
    if len > 0 {
        gg.wbuf.extend_from_slice(std::slice::from_raw_parts(data, len));
    }
}

pub unsafe extern "C" fn cb_flush(_png: *mut c_void) {
    g().flushes += 1;
}

pub unsafe extern "C" fn cb_read(png: *mut c_void, data: *mut u8, len: usize) {
    let gg = g();
    let avail = gg.rbuf.len().saturating_sub(gg.rpos);
    if avail >= len {
        std::ptr::copy_nonoverlapping(gg.rbuf.as_ptr().add(gg.rpos), data, len);
        gg.rpos += len;
    } else {
        if gg.short_read_mode == 1 {
            std::ptr::copy_nonoverlapping(gg.rbuf.as_ptr().add(gg.rpos), data, avail);
            std::ptr::write_bytes(data.add(avail), 0, len - avail);
            gg.rpos = gg.rbuf.len();
        } else {
            (api().png_error)(png, b"Read Error\0".as_ptr() as *const c_char);
        }
    }
}

pub unsafe extern "C" fn cb_read_status(_png: *mut c_void, row: u32, pass: c_int) {
    g().notes.push(format!("rstat {row} {pass}"));
}

pub unsafe extern "C" fn cb_write_status(_png: *mut c_void, row: u32, pass: c_int) {
    g().notes.push(format!("wstat {row} {pass}"));
}

/* ------------------------------------------------------------------ */
/* record output + exit                                                */
/* ------------------------------------------------------------------ */

pub fn finish(code: c_int) -> ! {
    let gg = g();
    let out = gg.out.clone();
    let body = unsafe { (*gg.rec).s.clone() };
    let _ = std::fs::write(&out, body.as_bytes());
    unsafe { _exit(code) }
}

/* ------------------------------------------------------------------ */
/* child entry point                                                   */
/* ------------------------------------------------------------------ */

pub const ENV_SCEN: &str = "PNGDIFF_SCEN";
pub const ENV_LIB: &str = "PNGDIFF_LIB";
pub const ENV_OUT: &str = "PNGDIFF_OUT";

/// Called from the `worker` `#[test]`.  A no-op in the parent.
pub fn worker_main() {
    disable_core_dumps();
    let scen = match std::env::var(ENV_SCEN) {
        Ok(v) => v,
        Err(_) => return,
    };
    let which = Which::parse(&std::env::var(ENV_LIB).unwrap());
    let out = std::env::var(ENV_OUT).unwrap();

    unsafe {
        G_PTR = Box::leak(Box::new(Globals::new()));
    }
    let a: &'static Api = Box::leak(Box::new(unsafe { Api::load(&which.so_path()) }));
    let gg = g();
    gg.api = a;
    gg.rec = Box::leak(Box::new(Rec::new()));
    gg.out = out;

    let f = scen::lookup(&scen).unwrap_or_else(|| panic!("unknown scenario {scen}"));
    f();
    finish(0);
}

/* ------------------------------------------------------------------ */
/* parent side                                                         */
/* ------------------------------------------------------------------ */

pub struct RunResult {
    pub record: String,
    pub status: String,
}

pub fn run_child(scen: &str, which: Which) -> RunResult {
    disable_core_dumps();
    ensure_rust_so_built();
    let dir = std::env::temp_dir().join(format!(
        "pngdiff-{}-{}",
        std::process::id(),
        std::thread::current().id().as_u64_compat()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let out = dir.join(format!("{}-{}.rec", sanitize(scen), which.tag()));
    let _ = std::fs::remove_file(&out);

    let exe = std::env::current_exe().unwrap();
    let mut child = std::process::Command::new(exe)
        .args(["--exact", "worker", "--test-threads=1", "-q"])
        .env(ENV_SCEN, scen)
        .env(ENV_LIB, which.tag())
        .env(ENV_OUT, &out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn child");

    // Never let one scenario wedge the whole run.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let st = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break Some(s),
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    };

    let record = std::fs::read_to_string(&out).unwrap_or_else(|_| String::from("<no record>"));
    let status = match st {
        None => "TIMEOUT".to_string(),
        Some(s) => match s.code() {
            Some(c) => format!("exit {c}"),
            None => format!("signal {:?}", signal_of(&s)),
        },
    };
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_dir(&dir);
    RunResult { record, status }
}

fn signal_of(st: &std::process::ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        st.signal().unwrap_or(-1)
    }
    #[cfg(not(unix))]
    {
        -1
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

trait ThreadIdExt {
    fn as_u64_compat(&self) -> u64;
}
impl ThreadIdExt for std::thread::ThreadId {
    fn as_u64_compat(&self) -> u64 {
        // ThreadId has no stable numeric accessor; hash it.
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }
}

/// Run one scenario against both libraries and assert the records match.
pub fn assert_same(scen: &str) {
    let c = run_child(scen, Which::C);
    let r = run_child(scen, Which::Rust);
    if c.record == r.record && c.status == r.status {
        return;
    }
    let mut msg = format!(
        "scenario `{scen}` diverged\n  C   : {}\n  Rust: {}\n",
        c.status, r.status
    );
    let cl: Vec<&str> = c.record.lines().collect();
    let rl: Vec<&str> = r.record.lines().collect();
    let n = cl.len().max(rl.len());
    let mut shown = 0;
    for i in 0..n {
        let a = cl.get(i).copied().unwrap_or("<missing>");
        let b = rl.get(i).copied().unwrap_or("<missing>");
        if a != b {
            let _ = writeln!(msg, "  line {}:\n    C   : {}\n    Rust: {}", i + 1, trunc(a), trunc(b));
            shown += 1;
            if shown >= 8 {
                let _ = writeln!(msg, "  ... (further differences suppressed)");
                break;
            }
        }
    }
    if shown == 0 {
        let _ = writeln!(msg, "  (records equal; only exit status differs)");
    }
    panic!("{msg}");
}

fn trunc(s: &str) -> String {
    if s.len() <= 300 {
        s.to_string()
    } else {
        format!("{}... ({} bytes)", &s[..300], s.len())
    }
}

/// Run every `(description, scenario)` pair, record per-row pass/fail under
/// `.rowresults/<group>.tsv` (used to generate `CONFIGS.md` / `ERRORS.md`) and
/// panic if any row diverged.
pub fn check_rows(group: &str, rows: &[(String, String)]) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".rowresults");
    let _ = std::fs::create_dir_all(&dir);
    let mut out = String::new();
    let mut fails = Vec::new();
    for (desc, scen) in rows {
        let c = run_child(scen, Which::C);
        let r = run_child(scen, Which::Rust);
        let ok = c.record == r.record && c.status == r.status;
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}",
            if ok { "PASS" } else { "FAIL" },
            scen,
            desc,
            summarize(&c)
        );
        if !ok {
            let mut d = format!("{scen}\n      C {} / Rust {}", c.status, r.status);
            let cl: Vec<&str> = c.record.lines().collect();
            let rl: Vec<&str> = r.record.lines().collect();
            for i in 0..cl.len().max(rl.len()) {
                let a = cl.get(i).copied().unwrap_or("<missing>");
                let b = rl.get(i).copied().unwrap_or("<missing>");
                if a != b {
                    let _ = write!(
                        d,
                        "\n      line {}\n        C   : {}\n        Rust: {}",
                        i + 1,
                        trunc(a),
                        trunc(b)
                    );
                    break;
                }
            }
            fails.push(d);
        }
    }
    let _ = std::fs::write(dir.join(format!("{group}.tsv")), out.as_bytes());
    if !fails.is_empty() {
        panic!(
            "[{group}] {} of {} rows diverged:\n  {}",
            fails.len(),
            rows.len(),
            fails.join("\n  ")
        );
    }
}

/// A one-line summary of what the C library actually did, so the generated
/// documentation can report observed behaviour instead of a claim.
fn summarize(r: &RunResult) -> String {
    let mut parts = Vec::new();
    parts.push(r.status.clone());
    let warns: Vec<&str> = r
        .record
        .lines()
        .filter(|l| l.starts_with("WARN "))
        .map(|l| &l[5..])
        .collect();
    if let Some(e) = r.record.lines().find(|l| l.starts_with("ERROR ")) {
        parts.push(format!("png_error: {}", &e[6..]));
    }
    if !warns.is_empty() {
        let mut uniq: Vec<&str> = Vec::new();
        for w in &warns {
            if !uniq.contains(w) {
                uniq.push(w);
            }
        }
        parts.push(format!(
            "{} warning(s): {}",
            warns.len(),
            uniq.iter().take(3).cloned().collect::<Vec<_>>().join(" / ")
        ));
    }
    if r.record == "<no record>" {
        parts.push("no record written".to_string());
    }
    parts.join("; ").replace('\t', " ").replace('|', "/")
}

/// Convenience: turn a `configs::Row` slice into what `check_rows` wants.
pub fn rows_of(rows: &[configs::Row]) -> Vec<(String, String)> {
    rows.iter().map(|(_, d, s)| (d.clone(), s.clone())).collect()
}

/// `cargo test` compiles the library crate but does **not** refresh the
/// `cdylib` artifact that the tests `dlopen`.  Make sure it is current before
/// any scenario runs, otherwise a stale `.so` would be compared.
pub fn ensure_rust_so_built() {
    static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        if std::env::var(ENV_SCEN).is_ok() {
            return; // child process: never recurse
        }
        let exe = std::env::current_exe().unwrap();
        let release = exe.components().any(|c| c.as_os_str() == "release");
        let cargo = option_env!("CARGO").unwrap_or("cargo");
        let mut cmd = std::process::Command::new(cargo);
        cmd.arg("build").arg("--lib").arg("--offline");
        if release {
            cmd.arg("--release");
        }
        cmd.arg("--manifest-path")
            .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        // Clear cargo's own env so the nested invocation does not inherit the
        // outer build's jobserver / rustc wrapper settings.
        for k in [
            "CARGO", "RUSTC", "RUSTUP_TOOLCHAIN", "CARGO_MAKEFLAGS", "RUSTC_WORKSPACE_WRAPPER",
        ] {
            cmd.env_remove(k);
        }
        let _ = cmd.status();
    });
}

/// Assert every scenario in the list matches; report all failures at once.
pub fn assert_all(scens: &[&str]) {
    let mut fails = Vec::new();
    for s in scens {
        let c = run_child(s, Which::C);
        let r = run_child(s, Which::Rust);
        if c.record != r.record || c.status != r.status {
            let mut d = format!("{s}: C {} / Rust {}", c.status, r.status);
            let cl: Vec<&str> = c.record.lines().collect();
            let rl: Vec<&str> = r.record.lines().collect();
            for i in 0..cl.len().max(rl.len()) {
                let a = cl.get(i).copied().unwrap_or("<missing>");
                let b = rl.get(i).copied().unwrap_or("<missing>");
                if a != b {
                    let _ = write!(d, "\n    line {}\n      C   : {}\n      Rust: {}", i + 1, trunc(a), trunc(b));
                    break;
                }
            }
            fails.push(d);
        }
    }
    if !fails.is_empty() {
        panic!("{} of {} scenarios diverged:\n{}", fails.len(), scens.len(), fails.join("\n"));
    }
}
