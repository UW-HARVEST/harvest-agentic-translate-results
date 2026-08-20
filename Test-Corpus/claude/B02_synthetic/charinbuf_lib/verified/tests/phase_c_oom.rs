// Phase C, rows 9 / 17 / 18 of ERRORS.md — the `malloc`-failure branches.
//
// These three branches need `malloc` to fail for a 1..38-byte request, which
// cannot be provoked in the test process itself. They ARE reachable in a child
// process whose address space has been capped with `RLIMIT_AS` and whose
// remaining heap has then been drained, so each row gets a real differential
// test rather than a hand-wave:
//
//   parent: spawn this same test binary twice (once per library), capture the
//           child's stdout, compare the two byte-for-byte.
//   child:  dlopen ONE library, force the allocator to fail, make the call,
//           then report the return value on stdout and `_exit`.
//
// Everything that needs to allocate (dlopen, env lookup, stdout buffer) happens
// BEFORE the cap is applied, so the only failing allocation is the library's own.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::process::{Command, Stdio};

const ENV_LIB: &str = "CHARINBUF_OOM_LIB";
const ENV_CALL: &str = "CHARINBUF_OOM_CALL";
const BEGIN: &[u8] = b"#BEGIN\n";

// ---------------------------------------------------------------------------
// Child-side machinery
// ---------------------------------------------------------------------------

const RLIMIT_AS: c_int = 9;

#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

unsafe extern "C" {
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
    fn _exit(status: c_int) -> !;
}

/// Current virtual-address-space size in bytes, from `/proc/self/statm`.
fn vm_size_bytes() -> usize {
    let s = std::fs::read_to_string("/proc/self/statm").expect("read statm");
    let pages: usize = s
        .split_whitespace()
        .next()
        .expect("statm field")
        .parse()
        .expect("statm number");
    pages * 4096
}

/// Caps the address space just above what is already mapped, then drains every
/// remaining free chunk so that even a tiny `malloc` fails.
fn force_allocation_failure() {
    let limit = (vm_size_bytes() + 256 * 1024) as u64;
    let rl = RLimit {
        rlim_cur: limit,
        rlim_max: limit,
    };
    assert_eq!(
        unsafe { setrlimit(RLIMIT_AS, &rl) },
        0,
        "setrlimit(RLIMIT_AS) failed"
    );

    // Drain large-to-small so no usable fragment is left behind.
    //
    // `black_box` + a store into each block are load-bearing: LLVM knows
    // `malloc` is a removable allocation function, so a release build will
    // delete an allocation whose result is only null-checked. With the calls
    // optimized away nothing gets drained, the library's own `malloc` succeeds,
    // and the test silently stops testing the branch it exists for.
    let mut sink: usize = 0;
    for size in [1 << 20, 1 << 16, 4096, 512, 64, 40, 32, 24, 16] {
        let mut spins = 0u32;
        loop {
            let p = std::hint::black_box(unsafe { malloc(std::hint::black_box(size)) });
            if p.is_null() {
                break;
            }
            // Touch the block so it cannot be elided, then leak it: the child
            // exits immediately afterwards.
            unsafe {
                p.cast::<u8>().write_volatile(0xA5);
            }
            sink = sink.wrapping_add(p as usize);
            spins += 1;
            if spins > 5_000_000 {
                break;
            }
        }
    }
    std::hint::black_box(sink);
}

/// `write(1, ...)`-based reporting: no allocation, unlike `format!`.
fn report(value: c_int) {
    let mut buf = [0u8; 32];
    let mut n = 0;
    for &b in b"#RESULT=" {
        buf[n] = b;
        n += 1;
    }
    // Manual itoa over i32, including i32::MIN.
    let neg = value < 0;
    let mut mag = (value as i64).unsigned_abs();
    let mut digits = [0u8; 12];
    let mut d = 0;
    if mag == 0 {
        digits[d] = b'0';
        d += 1;
    }
    while mag > 0 {
        digits[d] = b'0' + (mag % 10) as u8;
        mag /= 10;
        d += 1;
    }
    if neg {
        buf[n] = b'-';
        n += 1;
    }
    while d > 0 {
        d -= 1;
        buf[n] = digits[d];
        n += 1;
    }
    buf[n] = b'\n';
    n += 1;
    unsafe {
        write(1, buf.as_ptr().cast(), n);
    }
}

/// The child. Runs only when the environment variables are set.
#[test]
fn oom_child_worker() {
    let which = match std::env::var(ENV_LIB) {
        Ok(v) => v,
        Err(_) => return, // not a child invocation; nothing to do
    };
    let call = std::env::var(ENV_CALL).expect("CHARINBUF_OOM_CALL");

    let (c, r) = apis();
    let api = if which == "c" { c } else { r };

    // Force the stdout buffer to be allocated now, while allocation still works,
    // and mark where the comparable output begins.
    let banner = c"#BEGIN\n";
    unsafe {
        // Use the same printf both libraries use.
        write(1, banner.as_ptr().cast(), 7);
        fflush(std::ptr::null_mut());
    }

    // The string create_buffer will be asked to duplicate.
    let probe = c"probe string for oom";

    force_allocation_failure();

    let result: c_int = match call.as_str() {
        "create_buffer" => {
            let p = unsafe { (api.create_buffer)(probe.as_ptr()) };
            if p.is_null() { 0 } else { 1 }
        }
        "mode2" => (api.charinbuf)(2, 0, 0, 0),
        "mode4" => (api.charinbuf)(4, 0, 0, 0),
        other => panic!("unknown call {other}"),
    };

    unsafe {
        fflush(std::ptr::null_mut());
    }
    report(result);
    unsafe {
        fflush(std::ptr::null_mut());
        _exit(0)
    }
}

// ---------------------------------------------------------------------------
// Parent-side driver
// ---------------------------------------------------------------------------

/// Runs the child for one library and returns the bytes it produced after
/// `#BEGIN`.
fn run_child(which: &str, call: &str) -> Vec<u8> {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(&exe)
        .args(["--exact", "oom_child_worker", "--nocapture", "--test-threads=1"])
        .env(ENV_LIB, which)
        .env(ENV_CALL, call)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn oom child");

    let pos = out
        .stdout
        .windows(BEGIN.len())
        .position(|w| w == BEGIN)
        .unwrap_or_else(|| {
            panic!(
                "child ({which}/{call}) never reached #BEGIN.\nstdout: {}\nstderr: {}",
                show(&out.stdout),
                show(&out.stderr)
            )
        });
    out.stdout[pos + BEGIN.len()..].to_vec()
}

fn result_of(bytes: &[u8]) -> i32 {
    let s = String::from_utf8_lossy(bytes);
    let line = s
        .lines()
        .find(|l| l.starts_with("#RESULT="))
        .unwrap_or_else(|| panic!("no #RESULT line in child output: \"{}\"", show(bytes)));
    line["#RESULT=".len()..].trim().parse().expect("parse result")
}

/// Everything except the `#RESULT=` line, i.e. what the library printed.
fn library_output(bytes: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(bytes);
    s.lines()
        .filter(|l| !l.starts_with("#RESULT="))
        .map(|l| format!("{l}\n"))
        .collect::<String>()
        .into_bytes()
}

#[track_caller]
fn diff_oom(call: &str) -> (i32, Vec<u8>) {
    let cb = run_child("c", call);
    let rb = run_child("rust", call);

    let (cr, rr) = (result_of(&cb), result_of(&rb));
    let (co, ro) = (library_output(&cb), library_output(&rb));

    assert_eq!(
        cr, rr,
        "{call} under allocation failure: C returned {cr}, Rust returned {rr}\n  \
         C stdout    = \"{}\"\n  Rust stdout = \"{}\"",
        show(&co),
        show(&ro)
    );
    assert_eq!(
        co,
        ro,
        "{call} under allocation failure: stdout differs\n  C    = \"{}\"\n  Rust = \"{}\"",
        show(&co),
        show(&ro)
    );
    (cr, co)
}

// ===========================================================================
// Row 9 — create_buffer with a failing malloc -> NULL (no strcpy)
// ===========================================================================

#[test]
fn err_09_create_buffer_malloc_failure() {
    let _g = gate();
    let (result, out) = diff_oom("create_buffer");
    assert_eq!(
        result, 0,
        "could not force malloc to fail — create_buffer still returned non-NULL"
    );
    assert!(
        out.is_empty(),
        "create_buffer must print nothing, got \"{}\"",
        show(&out)
    );
}

// ===========================================================================
// Row 17 — charinbuf mode 2 with a failing malloc -> "Failed to allocate
//          buffer" and -1
// ===========================================================================

#[test]
fn err_17_charinbuf_mode2_alloc_failure() {
    let _g = gate();
    let (result, out) = diff_oom("mode2");
    assert_eq!(
        result, -1,
        "mode 2 under allocation failure must return -1, got {result} (stdout \"{}\")",
        show(&out)
    );
    assert_eq!(
        out,
        b"Mode 2: Dynamic memory allocation and free\nFailed to allocate buffer\n".to_vec(),
        "mode 2 alloc-failure stdout was \"{}\"",
        show(&out)
    );
}

// ===========================================================================
// Row 18 — charinbuf mode 4 with a failing malloc: whole block skipped, so
//          NOTHING is printed after the banner and the result stays 0 (not -1)
// ===========================================================================

#[test]
fn err_18_charinbuf_mode4_alloc_failure() {
    let _g = gate();
    let (result, out) = diff_oom("mode4");
    assert_eq!(
        result, 0,
        "mode 4 under allocation failure must return 0 (NOT -1), got {result} (stdout \"{}\")",
        show(&out)
    );
    assert_eq!(
        out,
        b"Mode 4: Using memchr to find character\n".to_vec(),
        "mode 4 alloc-failure stdout was \"{}\"",
        show(&out)
    );
}

// Keep the unused-import warning away when the child path is not compiled in.
const _: Option<*const c_char> = None;
