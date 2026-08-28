//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Rows that need a hostile environment (allocation failure, fatal signal,
//! non-termination) are run in a **child process**: the test binary re-executes
//! itself under `sh -c 'ulimit -v N; exec <self> --exact child_dispatch …'` with
//! `SR_CHILD_LIB` / `SR_CHILD_SCENARIO` set. The C and the Rust `.so` each get
//! their own child with identical limits, and the two outcomes (fatal signal,
//! exit status, and the reported `NULL`/length/hash of the result) must match.

mod common;

use common::{assert_same, c_impl, call, call_raw, rust_impl, SearchAndReplaceFn};
use std::ffi::c_char;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const ENV_LIB: &str = "SR_CHILD_LIB";
const ENV_SCENARIO: &str = "SR_CHILD_SCENARIO";
const MARKER: &str = "SR_RESULT:";

/// Size of the "one fits, two do not" buffer used by the allocation-failure
/// scenarios, and the address-space cap of the children. Both are tunable with
/// `SR_BIG_MB` / `SR_ULIMIT_MB` (the parent forwards them to its children) so a
/// host with unusual per-process overhead can be accommodated without editing
/// the test.
fn big_mb() -> usize {
    env_usize("SR_BIG_MB", 200)
}

fn ulimit_mb() -> usize {
    env_usize("SR_ULIMIT_MB", 400)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn big() -> usize {
    big_mb() * 1024 * 1024
}

// ===========================================================================
// child side
// ===========================================================================

fn fnv1a(b: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

fn report(r: Option<Vec<u8>>) {
    match r {
        None => println!("{MARKER} NULL"),
        Some(v) => println!("{MARKER} OK len={} hash={:016x}", v.len(), fnv1a(&v)),
    }
}

/// A NUL-terminated buffer of `n` copies of `c`, built without an extra copy.
fn big_buf(n: usize, c: u8) -> Vec<u8> {
    let mut v = vec![c; n + 1];
    v[n] = 0;
    v
}

fn nul_term(mut v: Vec<u8>) -> Vec<u8> {
    v.push(0);
    v
}

/// The entry point used by every child process.
#[test]
fn child_dispatch() {
    let (lib, scenario) = match (std::env::var(ENV_LIB), std::env::var(ENV_SCENARIO)) {
        (Ok(l), Ok(s)) => (l, s),
        // Not a child: nothing to do.
        _ => return,
    };
    let f: SearchAndReplaceFn = match lib.as_str() {
        "c" => c_impl(),
        "rust" => rust_impl(),
        other => panic!("unknown {ENV_LIB}={other}"),
    };
    let nul: *const c_char = b"\0".as_ptr() as *const c_char;
    let big = big();
    let p = |v: &Vec<u8>| v.as_ptr() as *const c_char;

    match scenario.as_str() {
        // ---- fatal-signal scenarios (E6..E9) -------------------------------
        "null_orig" => {
            let s = nul_term(b"a".to_vec());
            let v = nul_term(b"b".to_vec());
            report(unsafe { call_raw(f, std::ptr::null(), p(&s), p(&v)) });
        }
        "null_search" => {
            let o = nul_term(b"aaa".to_vec());
            let v = nul_term(b"b".to_vec());
            report(unsafe { call_raw(f, p(&o), std::ptr::null(), p(&v)) });
        }
        "null_value" => {
            let o = nul_term(b"aaa".to_vec());
            let s = nul_term(b"a".to_vec());
            report(unsafe { call_raw(f, p(&o), p(&s), std::ptr::null()) });
        }
        // `value` is measured before the match test, so this faults even though
        // `search` does not occur in `orig`.
        "null_value_no_match" => {
            let o = nul_term(b"aaa".to_vec());
            let s = nul_term(b"zzz".to_vec());
            report(unsafe { call_raw(f, p(&o), p(&s), std::ptr::null()) });
        }
        "all_null" => {
            report(unsafe { call_raw(f, std::ptr::null(), std::ptr::null(), std::ptr::null()) });
        }
        "null_orig_empty_others" => {
            report(unsafe { call_raw(f, std::ptr::null(), nul, nul) });
        }

        // ---- non-termination scenarios (E10, E11) --------------------------
        "hang_empty_search" => {
            let o = nul_term(b"hello world".to_vec());
            report(unsafe { call_raw(f, p(&o), nul, nul) });
        }
        "hang_empty_orig_and_search" => {
            report(unsafe { call_raw(f, nul, nul, nul) });
        }

        // ---- allocation-failure scenarios (E1..E5, E12, E13) ---------------
        // E1: malloc(inx_start + 1) for the prefix copy fails.
        "oom_prefix_malloc" => {
            // one single allocation of `big + 8` (a later re-allocation would
            // double the capacity and blow the address-space cap in the child
            // itself instead of inside the library)
            let mut o = Vec::with_capacity(big + 8);
            o.extend(std::iter::repeat(b'a').take(big));
            o.extend_from_slice(b"NEEDLE\0");
            let s = nul_term(b"NEEDLE".to_vec());
            let v = nul_term(b"x".to_vec());
            report(unsafe { call_raw(f, p(&o), p(&s), p(&v)) });
        }
        // E2: realloc for the replacement copy fails on a later loop iteration.
        "oom_value_realloc" => {
            let o = nul_term(b"xxxx".to_vec()); // 4 adjacent matches
            let s = nul_term(b"x".to_vec());
            let v = big_buf(big / 3, b'v');
            report(unsafe { call_raw(f, p(&o), p(&s), p(&v)) });
        }
        // E13: a single oversized replacement (one match, one realloc).
        "oom_oversized_value" => {
            let o = nul_term(b"x".to_vec());
            let s = nul_term(b"x".to_vec());
            let v = big_buf(big, b'v');
            report(unsafe { call_raw(f, p(&o), p(&s), p(&v)) });
        }
        // E3: realloc for the inter-match gap copy fails (value_len == 0, so the
        // only growth comes from the gap).
        "oom_gap_realloc" => {
            let mut o = Vec::with_capacity(big + 8);
            o.push(b'x');
            o.extend(std::iter::repeat(b'y').take(big));
            o.push(b'x');
            o.push(0);
            let s = nul_term(b"x".to_vec());
            report(unsafe { call_raw(f, p(&o), p(&s), nul) });
        }
        // E4: realloc for the trailing tail copy fails.
        "oom_tail_realloc" => {
            let mut o = Vec::with_capacity(big + 8);
            o.push(b'x');
            o.extend(std::iter::repeat(b'y').take(big));
            o.push(0);
            let s = nul_term(b"x".to_vec());
            report(unsafe { call_raw(f, p(&o), p(&s), nul) });
        }
        // E5: no match at all -> strdup(orig) fails.
        "oom_strdup" => {
            let o = big_buf(big, b'a');
            let s = nul_term(b"zz".to_vec());
            let v = nul_term(b"q".to_vec());
            report(unsafe { call_raw(f, p(&o), p(&s), p(&v)) });
        }
        // E12: empty needle + non-empty value -> unbounded growth -> NULL.
        "oom_empty_search" => {
            let o = nul_term(b"abc".to_vec());
            let v = big_buf((big / 24).max(1024 * 1024), b'v');
            report(unsafe { call_raw(f, p(&o), nul, p(&v)) });
        }
        other => panic!("unknown {ENV_SCENARIO}={other}"),
    }
}

// ===========================================================================
// parent side
// ===========================================================================

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    /// `Some(sig)` if the child was killed by a signal.
    signal: Option<i32>,
    code: Option<i32>,
    /// The `SR_RESULT: …` line, if the call returned at all.
    result: Option<String>,
    still_running: bool,
}

fn run_child(lib: &str, scenario: &str, ulimit_kb: Option<u64>, wait: Duration) -> Outcome {
    run_child_sized(lib, scenario, ulimit_kb, wait, big_mb())
}

fn run_child_sized(
    lib: &str,
    scenario: &str,
    ulimit_kb: Option<u64>,
    wait: Duration,
    big_mb_override: usize,
) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().expect("current_exe");
    let prefix = match ulimit_kb {
        Some(kb) => format!("ulimit -v {kb}; "),
        None => String::new(),
    };
    let cmd = format!(
        "{prefix}exec '{}' --exact child_dispatch --nocapture --test-threads=1",
        exe.display()
    );
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(&cmd)
        .env(ENV_LIB, lib)
        .env(ENV_SCENARIO, scenario)
        .env("RUST_BACKTRACE", "0")
        .env("SR_BIG_MB", big_mb_override.to_string())
        .env("SR_ULIMIT_MB", ulimit_mb().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn child");

    let start = Instant::now();
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break Some(s),
            None => {
                if start.elapsed() >= wait {
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    };

    let still_running = status.is_none();
    if still_running {
        let _ = child.kill();
        let _ = child.wait();
        return Outcome {
            signal: None,
            code: None,
            result: None,
            still_running: true,
        };
    }
    let status = status.unwrap();

    let mut out = String::new();
    if let Some(mut s) = child.stdout.take() {
        let _ = s.read_to_string(&mut out);
    }
    let mut err = String::new();
    if let Some(mut s) = child.stderr.take() {
        let _ = s.read_to_string(&mut err);
    }
    // libtest with --nocapture prefixes the first captured line with
    // "test child_dispatch ... ", so look for the marker anywhere in the line.
    let result = out
        .lines()
        .filter_map(|l| l.find(MARKER).map(|i| l[i..].trim().to_string()))
        .next();
    let _ = &err;

    Outcome {
        signal: status.signal(),
        code: status.code(),
        result,
        still_running: false,
    }
}

/// Run the same scenario against both `.so`s in separate children (so a leak in
/// one cannot perturb the other) and require identical outcomes.
fn assert_children_agree(row: &str, scenario: &str, ulimit_kb: Option<u64>, wait: Duration) -> Outcome {
    let c = run_child("c", scenario, ulimit_kb, wait);
    let r = run_child("rust", scenario, ulimit_kb, wait);
    assert_eq!(
        c, r,
        "[{row}] scenario `{scenario}`: C and Rust children disagree\n  C    = {c:?}\n  Rust = {r:?}"
    );
    c
}

fn assert_segv(row: &str, scenario: &str) {
    let o = assert_children_agree(row, scenario, None, Duration::from_secs(30));
    assert_eq!(
        o.signal,
        Some(11),
        "[{row}] scenario `{scenario}`: expected both to die with SIGSEGV, got {o:?}"
    );
    assert!(
        o.result.is_none(),
        "[{row}] scenario `{scenario}`: the call was expected to fault, not return: {o:?}"
    );
}

fn assert_returns_null(row: &str, scenario: &str) {
    let kb = (ulimit_mb() * 1024) as u64;
    let o = assert_children_agree(row, scenario, Some(kb), Duration::from_secs(180));
    assert_eq!(o.code, Some(0), "[{row}] `{scenario}`: child failed: {o:?}");
    assert_eq!(
        o.result.as_deref(),
        Some("SR_RESULT: NULL"),
        "[{row}] `{scenario}`: expected a NULL return under ulimit -v {kb}, got {o:?}"
    );
}

/// Positive control for the allocation-failure rows: the very same scenario,
/// with a small `SR_BIG_MB` and a generous address-space cap, must *succeed*
/// identically in both implementations. That proves the inputs are otherwise
/// valid and that the code really reaches the allocation site under test, i.e.
/// the `NULL` observed by `assert_returns_null` is caused by the capped
/// allocation and not by some earlier rejection.
fn assert_positive_control(row: &str, scenario: &str) {
    let kb = Some(400u64 * 1024);
    let wait = Duration::from_secs(120);
    let c = run_child_sized("c", scenario, kb, wait, 8);
    let r = run_child_sized("rust", scenario, kb, wait, 8);
    assert_eq!(
        c, r,
        "[{row}] positive control `{scenario}`: C and Rust disagree\n  C    = {c:?}\n  Rust = {r:?}"
    );
    assert_eq!(c.code, Some(0), "[{row}] positive control `{scenario}` failed: {c:?}");
    let line = c.result.clone().unwrap_or_default();
    assert!(
        line.starts_with("SR_RESULT: OK"),
        "[{row}] positive control `{scenario}` should return a buffer, got {c:?}"
    );
}

fn assert_hangs(row: &str, scenario: &str) {
    let c = run_child("c", scenario, None, Duration::from_secs(3));
    let r = run_child("rust", scenario, None, Duration::from_secs(3));
    assert!(
        c.still_running,
        "[{row}] `{scenario}`: the C implementation was expected to loop forever, got {c:?}"
    );
    assert!(
        r.still_running,
        "[{row}] `{scenario}`: the Rust implementation must loop forever like the C one, got {r:?}"
    );
    assert_eq!(c, r, "[{row}] `{scenario}`: outcomes differ");
}

// ---------------------------------------------------------------------------
// E1..E5, E12, E13 — allocation-failure rows
// ---------------------------------------------------------------------------

#[test]
fn e1_malloc_prefix_fails_returns_null() {
    assert_positive_control("E1", "oom_prefix_malloc");
    assert_returns_null("E1", "oom_prefix_malloc");
}

#[test]
fn e2_realloc_value_fails_returns_null() {
    assert_positive_control("E2", "oom_value_realloc");
    assert_returns_null("E2", "oom_value_realloc");
}

#[test]
fn e3_realloc_gap_fails_returns_null() {
    assert_positive_control("E3", "oom_gap_realloc");
    assert_returns_null("E3", "oom_gap_realloc");
}

#[test]
fn e4_realloc_tail_fails_returns_null() {
    assert_positive_control("E4", "oom_tail_realloc");
    assert_returns_null("E4", "oom_tail_realloc");
}

#[test]
fn e5_strdup_fails_returns_null() {
    assert_positive_control("E5", "oom_strdup");
    assert_returns_null("E5", "oom_strdup");
}

#[test]
fn e12_empty_search_nonempty_value_oom_null() {
    assert_returns_null("E12", "oom_empty_search");
}

#[test]
fn e13_oversized_value_returns_null() {
    assert_positive_control("E13", "oom_oversized_value");
    assert_returns_null("E13", "oom_oversized_value");
}

// ---------------------------------------------------------------------------
// E6..E9 — NULL pointer rows
// ---------------------------------------------------------------------------

#[test]
fn e6_null_orig_segv() {
    assert_segv("E6", "null_orig");
    assert_segv("E6", "null_orig_empty_others");
}

#[test]
fn e7_null_search_segv() {
    assert_segv("E7", "null_search");
}

#[test]
fn e8_null_value_segv() {
    assert_segv("E8", "null_value");
}

#[test]
fn e8b_null_value_no_match_segv() {
    assert_segv("E8", "null_value_no_match");
}

#[test]
fn e9_all_null_segv() {
    assert_segv("E9", "all_null");
}

// ---------------------------------------------------------------------------
// E10, E11 — non-termination rows
// ---------------------------------------------------------------------------

#[test]
fn e10_empty_search_empty_value_hangs() {
    assert_hangs("E10", "hang_empty_search");
}

#[test]
fn e11_empty_orig_empty_search_hangs() {
    assert_hangs("E11", "hang_empty_orig_and_search");
}

// ---------------------------------------------------------------------------
// E14..E16 — in-process boundary rows (must NOT error)
// ---------------------------------------------------------------------------

#[test]
fn e14_zero_length_inputs_are_not_errors() {
    // empty haystack, non-empty needle -> strdup("")
    for search in [&b"a"[..], b"zz", b"\xff", b"abcdefgh"] {
        for value in [&b""[..], b"x", b"\x01\xfe"] {
            let out = assert_same("E14", b"", search, value);
            assert_eq!(out.as_deref(), Some(&b""[..]), "expected non-NULL empty string");
        }
    }
    // empty replacement (pure deletion) is valid, never NULL
    for orig in [&b"a"[..], b"aa", b"xax", b"aaa aaa", b"\xff\x01\xff"] {
        for search in [&b"a"[..], b"aa", b"\xff", b" "] {
            let out = assert_same("E14", orig, search, b"");
            assert!(out.is_some(), "deletion must not return NULL");
        }
    }
    // all three empty-but-valid combinations that terminate
    assert!(assert_same("E14", b"", b"z", b"").is_some());
    assert!(assert_same("E14", b"z", b"z", b"").is_some());
}

#[test]
fn e15_needle_one_past_haystack_len() {
    let mut rng = common::Rng::new(0xE015);
    for _ in 0..2000 {
        let orig = rng.bytes_range(1, 24, common::ABC);
        // needle exactly one byte longer than the haystack
        let mut longer = orig.clone();
        longer.push(b'a');
        let out = assert_same("E15", &orig, &longer, b"REPL");
        assert_eq!(out.as_deref(), Some(&orig[..]));

        // needle of exactly the haystack length, differing in the last byte
        let mut same_len = orig.clone();
        let last = same_len.len() - 1;
        same_len[last] = if same_len[last] == b'z' { b'y' } else { b'z' };
        let out = assert_same("E15", &orig, &same_len, b"REPL");
        assert_eq!(out.as_deref(), Some(&orig[..]));

        // ... and the exact-length needle that *does* match
        let out = assert_same("E15", &orig, &orig, b"REPL");
        assert_eq!(out.as_deref(), Some(&b"REPL"[..]));
    }
}

#[test]
fn e16_match_at_last_byte_no_tail() {
    let mut rng = common::Rng::new(0xE016);
    for _ in 0..2000 {
        let prefix = rng.bytes_range(0, 24, b"ab");
        let mut orig = prefix.clone();
        orig.push(b'Z'); // needle is the final byte -> from == orig_len
        let value = rng.bytes_range(0, 8, b"ab");
        let out = assert_same("E16", &orig, b"Z", &value);
        let mut want = prefix.clone();
        want.extend_from_slice(&value);
        assert_eq!(out.as_deref(), Some(&want[..]));
    }
}

// ---------------------------------------------------------------------------
// Extra generic FFI-boundary checks
// ---------------------------------------------------------------------------

/// The public header has no enum/integer parameter, so there is no
/// "out-of-range enum value" to smuggle across the FFI boundary; the analogous
/// invalid values for a `const char *` API are NULL (E6-E9) and the empty needle
/// (E10-E12). This test pins that fact mechanically so it cannot silently change.
#[test]
fn e17_no_enum_or_integer_parameters_in_public_api() {
    let header = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/include/lib.h"
    ))
    .expect("read c_src/include/lib.h");
    assert!(
        !header.contains("enum"),
        "the public header grew an enum; ERRORS.md must gain out-of-range-variant rows:\n{header}"
    );
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| l.contains("searchAndReplace"))
        .collect();
    assert_eq!(decls.len(), 1, "unexpected public API surface: {decls:?}");
    assert_eq!(
        decls[0].matches("const char *").count(),
        3,
        "unexpected parameter types: {}",
        decls[0]
    );
}

/// Aliased arguments (the same pointer passed for several parameters) must be
/// handled identically; nothing in the C forbids it.
#[test]
fn e18_aliased_arguments() {
    for s in [&b"a"[..], b"abc", b"\xff\xfe", b"aa"] {
        let buf = common::cstr(s);
        let p = buf.as_ptr() as *const c_char;
        let c = unsafe { call_raw(c_impl(), p, p, p) };
        let r = unsafe { call_raw(rust_impl(), p, p, p) };
        assert_eq!(c, r, "aliased orig==search==value diverged for {s:?}");
        assert_eq!(c.as_deref(), Some(s), "identity replacement expected");

        // orig aliases search, value distinct
        let v = common::cstr(b"Q");
        let c = unsafe { call_raw(c_impl(), p, p, v.as_ptr() as *const c_char) };
        let r = unsafe { call_raw(rust_impl(), p, p, v.as_ptr() as *const c_char) };
        assert_eq!(c, r, "aliased orig==search diverged for {s:?}");
        assert_eq!(c.as_deref(), Some(&b"Q"[..]));
    }
}

/// A needle whose only occurrence is the last byte, a haystack of length 1, and
/// the 1-byte-everything shape: the smallest possible non-empty inputs.
#[test]
fn e19_minimal_shapes() {
    for (o, s, v) in [
        (&b"a"[..], &b"a"[..], &b""[..]),
        (b"a", b"a", b"a"),
        (b"a", b"a", b"bb"),
        (b"a", b"b", b""),
        (b"a", b"b", b"c"),
        (b"ab", b"b", b""),
        (b"ab", b"a", b""),
        (b"\xff", b"\xff", b"\x01"),
    ] {
        assert_same("E19", o, s, v);
    }
}

/// Sanity: the harness really loads two *different* shared objects and would
/// notice a divergence (guards against a vacuous test suite).
#[test]
fn e20_harness_is_not_vacuous() {
    let c = c_impl();
    let r = rust_impl();
    assert_ne!(
        c as usize, r as usize,
        "both handles resolved to the same code - the two .so files were not loaded separately"
    );
    // Both implementations must agree here...
    assert_eq!(
        call(c, b"aXbXc", b"X", b"YY"),
        Some(b"aYYbYYc".to_vec()),
        "C reference behaviour changed"
    );
    assert_eq!(call(r, b"aXbXc", b"X", b"YY"), Some(b"aYYbYYc".to_vec()));
    // ... and the comparison really is content-sensitive.
    assert_ne!(call(c, b"aXbXc", b"X", b"YY"), call(r, b"aXbXc", b"X", b"Y"));
}
