//! Phase C — error-path differential tests that cannot run in-process.
//!
//! `ERRORS.md` rows 2-11 are fatal (SIGSEGV), non-terminating (empty needle)
//! or require injected allocation failures, so each one is executed in a child
//! process: once against the C `.so` and once against the Rust `.so`, both
//! loaded with `libloading`. The parent compares the full observable outcome —
//! exit code, terminating signal and stdout — and fails if they differ.
//!
//! This target runs without the libtest harness (`harness = false` in
//! Cargo.toml) so that it can re-exec itself as the child.
//!
//! Rows 1 and the in-process boundary cases live in `tests/differential.rs`.

mod common;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MEG: usize = 1 << 20;

/// Which library the child should load.
#[derive(Copy, Clone, PartialEq)]
enum Side {
    C,
    Rust,
}

impl Side {
    fn tag(self) -> &'static str {
        match self {
            Side::C => "c",
            Side::Rust => "rust",
        }
    }
    fn parse(s: &str) -> Side {
        match s {
            "c" => Side::C,
            "rust" => Side::Rust,
            other => panic!("unknown side {other:?}"),
        }
    }
    fn so(self) -> PathBuf {
        match self {
            Side::C => common::c_so_path(),
            Side::Rust => common::rust_so_path(),
        }
    }
}

// ---------------------------------------------------------------- probe table

/// `None` for an argument means a NULL pointer is passed.
struct Probe {
    name: &'static str,
    /// `ERRORS.md` row(s) covered.
    row: &'static str,
    /// exact allocation size the interposer must fail (0 = no interposer)
    fail_size: usize,
    /// true when the call is expected never to return
    hangs: bool,
}

const PROBES: &[Probe] = &[
    Probe { name: "null-orig",          row: "7",    fail_size: 0,       hangs: false },
    Probe { name: "null-search",        row: "8",    fail_size: 0,       hangs: false },
    Probe { name: "null-value",         row: "9",    fail_size: 0,       hangs: false },
    Probe { name: "null-all",           row: "7-9",  fail_size: 0,       hangs: false },
    Probe { name: "oom-strdup",         row: "2",    fail_size: MEG + 1, hangs: false },
    Probe { name: "oom-prefix-malloc",  row: "3",    fail_size: MEG + 1, hangs: false },
    Probe { name: "oom-value-realloc",  row: "4",    fail_size: MEG + 1, hangs: false },
    Probe { name: "oom-gap-realloc",    row: "5",    fail_size: MEG + 2, hangs: false },
    Probe { name: "oom-tail-realloc",   row: "6",    fail_size: MEG + 2, hangs: false },
    // sanity checks: with the interposer loaded but a size that is never
    // requested, the very same inputs must succeed on both sides. This proves
    // each NULL above really comes from the injected failure at the intended
    // allocation site, not from some earlier rejection.
    Probe { name: "oom-control-strdup", row: "2",    fail_size: 7,       hangs: false },
    Probe { name: "oom-control-prefix", row: "3",    fail_size: 7,       hangs: false },
    Probe { name: "oom-control-value",  row: "4",    fail_size: 7,       hangs: false },
    Probe { name: "oom-control-gap",    row: "5",    fail_size: 7,       hangs: false },
    Probe { name: "oom-control-tail",   row: "6",    fail_size: 7,       hangs: false },
    Probe { name: "hang-empty-search",  row: "10-11",fail_size: 0,       hangs: true  },
    Probe { name: "hang-empty-both",    row: "11",   fail_size: 0,       hangs: true  },
];

/// Build the three arguments of a probe. `None` == NULL pointer.
///
/// Buffers are over-allocated by 1000 bytes so that the child's own heap
/// requests can never coincide with the size the interposer fails.
fn inputs(name: &str) -> (Option<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>) {
    fn buf(parts: &[&[u8]]) -> Option<Vec<u8>> {
        let total: usize = parts.iter().map(|p| p.len()).sum();
        let mut v: Vec<u8> = Vec::with_capacity(total + 1001);
        for p in parts {
            v.extend_from_slice(p);
        }
        v.push(0);
        Some(v)
    }
    let big = vec![b'x'; MEG];
    match name {
        "null-orig" => (None, buf(&[b"ab"]), buf(&[b"Z"])),
        "null-search" => (buf(&[b"ab"]), None, buf(&[b"Z"])),
        "null-value" => (buf(&[b"ab"]), buf(&[b"a"]), None),
        "null-all" => (None, None, None),
        // no match, strdup(orig) must allocate MEG+1 -> fails
        "oom-strdup" | "oom-control-strdup" => (buf(&[&big]), buf(&[b"q"]), buf(&[b"Z"])),
        // first match at offset MEG -> malloc(MEG+1) -> fails
        "oom-prefix-malloc" | "oom-control-prefix" => (buf(&[&big, b"ab"]), buf(&[b"ab"]), buf(&[b"Z"])),
        // match at 0, value_len = MEG -> realloc(NULL, MEG+1) -> fails
        "oom-value-realloc" | "oom-control-value" => (buf(&[b"ab"]), buf(&[b"a"]), buf(&[&big])),
        // two matches separated by MEG bytes -> gap realloc(MEG+2) -> fails
        "oom-gap-realloc" | "oom-control-gap" => (buf(&[b"a", &big, b"a"]), buf(&[b"a"]), buf(&[b"Z"])),
        // one match at 0 with a MEG-byte tail -> tail realloc(MEG+2) -> fails
        "oom-tail-realloc" | "oom-control-tail" => (buf(&[b"a", &big]), buf(&[b"a"]), buf(&[b"Z"])),
        // empty needle: strstr always matches, the while loop never exits
        "hang-empty-search" => (buf(&[b"ab"]), buf(&[]), buf(&[])),
        "hang-empty-both" => (buf(&[]), buf(&[]), buf(&[])),
        other => panic!("unknown probe {other:?}"),
    }
}

// ---------------------------------------------------------------------- child

fn child(probe: &str, side: Side) -> ! {
    let (o, s, v) = inputs(probe);
    let lib = unsafe { libloading::Library::new(side.so()).expect("dlopen") };
    let f: libloading::Symbol<common::SearchAndReplaceFn> =
        unsafe { lib.get(b"searchAndReplace\0").expect("dlsym") };
    let ptr = |x: &Option<Vec<u8>>| match x {
        None => std::ptr::null(),
        Some(b) => b.as_ptr() as *const std::ffi::c_char,
    };
    // Flush the marker before the (possibly fatal) call so the parent can tell
    // "crashed inside the call" from "failed to start".
    println!("ENTER");
    let r = unsafe { f(ptr(&o), ptr(&s), ptr(&v)) };
    if r.is_null() {
        println!("OUT NULL");
    } else {
        let mut len = 0usize;
        let mut sum: u64 = 0;
        unsafe {
            while *r.add(len) != 0 {
                sum = sum
                    .wrapping_mul(1000003)
                    .wrapping_add(*(r.add(len) as *const u8) as u64);
                len += 1;
            }
        }
        println!("OUT len={len} hash={sum:016x}");
    }
    std::process::exit(0);
}

// --------------------------------------------------------------------- parent

struct Outcome {
    stdout: String,
    code: Option<i32>,
    signal: Option<i32>,
    still_running: bool,
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "code={:?} signal={:?} running={} stdout={:?}",
            self.code,
            self.signal,
            self.still_running,
            self.stdout.trim()
        )
    }
}

fn run_probe(exe: &Path, probe: &Probe, side: Side, preload: Option<&Path>) -> Outcome {
    let mut cmd = Command::new(exe);
    cmd.arg("child")
        .arg(probe.name)
        .arg(side.tag())
        .env("DRIVER_C_SO", common::c_so_path())
        .env("DRIVER_RUST_SO", common::rust_so_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if probe.fail_size != 0 {
        cmd.env("FAILMALLOC_SIZE", probe.fail_size.to_string());
        cmd.env("LD_PRELOAD", preload.expect("interposer built"));
    }
    let mut ch = cmd.spawn().expect("spawn child");

    let deadline = Instant::now() + Duration::from_secs(if probe.hangs { 3 } else { 60 });
    let mut status = None;
    while Instant::now() < deadline {
        match ch.try_wait().expect("try_wait") {
            Some(st) => {
                status = Some(st);
                break;
            }
            None => std::thread::sleep(Duration::from_millis(25)),
        }
    }
    let still_running = status.is_none();
    if still_running {
        let _ = ch.kill();
        let _ = ch.wait();
    }
    let mut stdout = String::new();
    if let Some(mut p) = ch.stdout.take() {
        let _ = p.read_to_string(&mut stdout);
    }
    use std::os::unix::process::ExitStatusExt;
    Outcome {
        stdout,
        code: status.as_ref().and_then(|s| s.code()),
        signal: status.as_ref().and_then(|s| s.signal()),
        still_running,
    }
}

fn build_interposer(exe: &Path) -> PathBuf {
    let dir = exe.parent().unwrap().parent().unwrap();
    let out = dir.join("failmalloc.so");
    let src = common::manifest_dir().join("tests/failmalloc.c");
    let st = Command::new("cc")
        .args(["-shared", "-fPIC", "-O1", "-o"])
        .arg(&out)
        .arg(&src)
        .status()
        .expect("run cc to build the allocation-failure interposer");
    assert!(st.success(), "compiling {src:?} failed");
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 4 && args[1] == "child" {
        child(&args[2], Side::parse(&args[3]));
    }
    // Ignore libtest-style flags cargo may pass (e.g. --nocapture, filters).
    let exe = std::env::current_exe().expect("current_exe");
    let interposer = build_interposer(&exe);
    // Fail fast if the interposer does not actually take effect.
    self_test_interposer(&interposer);

    let mut failed = Vec::new();
    println!("running {} error-path probes", PROBES.len());
    for probe in PROBES {
        let c = run_probe(&exe, probe, Side::C, Some(&interposer));
        let r = run_probe(&exe, probe, Side::Rust, Some(&interposer));

        let same = c.stdout == r.stdout
            && c.code == r.code
            && c.signal == r.signal
            && c.still_running == r.still_running;

        // Additionally pin down what the outcome must actually BE, so that a
        // probe cannot pass by both sides failing for an unrelated reason.
        let expectation = expected(probe, &c);

        if same && expectation.is_ok() {
            println!(
                "probe {:22} (ERRORS.md row {:5}) ... ok    [{}]",
                probe.name,
                probe.row,
                c.stdout.trim().replace('\n', " | ")
            );
        } else {
            println!(
                "probe {:22} (ERRORS.md row {:5}) ... FAILED\n    C    : {c}\n    Rust : {r}\n    check: {:?}",
                probe.name, probe.row, expectation
            );
            failed.push(probe.name);
        }
    }
    if failed.is_empty() {
        println!("\nresult: ok. {} probes passed", PROBES.len());
    } else {
        println!("\nresult: FAILED. {:?}", failed);
        std::process::exit(1);
    }
}

/// Check that the shared outcome is the one `ERRORS.md` predicts, not just
/// "both did the same thing".
fn expected(probe: &Probe, o: &Outcome) -> Result<(), String> {
    let out = o.stdout.trim();
    match probe.name {
        // rows 7-9: dereferencing a NULL argument must kill the process
        n if n.starts_with("null-") => {
            if o.signal == Some(11) && out == "ENTER" {
                Ok(())
            } else {
                Err(format!("expected SIGSEGV inside the call, got {o}"))
            }
        }
        // rows 10-11: must still be running when the timeout expires
        n if n.starts_with("hang-") => {
            if o.still_running && out == "ENTER" {
                Ok(())
            } else {
                Err(format!("expected non-termination, got {o}"))
            }
        }
        // controls: the interposer is loaded but never triggers
        n if n.starts_with("oom-control") => {
            if o.code == Some(0) && out.starts_with("ENTER\nOUT len=") {
                Ok(())
            } else {
                Err(format!("expected a successful call, got {o}"))
            }
        }
        // rows 2-6: the allocation fails and the function returns NULL
        _ => {
            if o.code == Some(0) && out == "ENTER\nOUT NULL" {
                Ok(())
            } else {
                Err(format!("expected a NULL return, got {o}"))
            }
        }
    }
}

/// Guard against a silently ineffective `LD_PRELOAD`: with the interposer
/// active, a 12345-byte malloc must fail.
fn self_test_interposer(interposer: &Path) {
    let st = Command::new("sh")
        .arg("-c")
        .arg(
            "printf '%s' '#include <stdlib.h>\n#include <stdio.h>\nint main(void){ \
             void*a=malloc(12345); void*b=malloc(999); \
             printf(\"%d %d\\n\", a==NULL, b==NULL); return 0; }' > \"$TMP/t.c\" && \
             cc -O0 -o \"$TMP/t\" \"$TMP/t.c\" && \
             FAILMALLOC_SIZE=12345 LD_PRELOAD=\"$PRELOAD\" \"$TMP/t\"",
        )
        .env("PRELOAD", interposer)
        .env("TMP", interposer.parent().unwrap())
        .output()
        .expect("run interposer self-test");
    let out = String::from_utf8_lossy(&st.stdout);
    assert_eq!(
        out.trim(),
        "1 0",
        "the allocation-failure interposer is not working (stdout {:?}, stderr {:?})",
        out,
        String::from_utf8_lossy(&st.stderr)
    );
    println!("interposer self-test ... ok");
}
