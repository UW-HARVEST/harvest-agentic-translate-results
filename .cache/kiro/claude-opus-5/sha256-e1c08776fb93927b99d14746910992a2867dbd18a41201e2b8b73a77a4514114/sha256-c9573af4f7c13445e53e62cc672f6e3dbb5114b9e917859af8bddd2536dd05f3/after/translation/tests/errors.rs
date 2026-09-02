//! Phase C — error-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`, plus the generic FFI boundary cases
//! (null pointers, zero / oversized lengths, one-past-range values).
//!
//! Rows that end in a crash or in non-termination cannot be observed in-process,
//! so those are run in a child process (this same test binary re-executed with
//! `PROBE_*` environment variables, see `zz_probe_child`), and the C and Rust
//! runs are compared on terminating signal / timeout instead of return value.
//!
//! Allocator-failure rows are made deterministic with an `LD_PRELOAD` interposer
//! (`tests/support/failalloc.c`) that fails the k-th allocation performed inside
//! the call, so "the realloc on line 62 fails" is an input we can actually
//! construct for both implementations.

mod common;

use common::SearchAndReplaceFn;
use std::ffi::{c_char, c_void};
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

extern "C" {
    fn free(p: *mut c_void);
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ---------------------------------------------------------------------------
// child-side probe

type ArmFn = unsafe extern "C" fn(i64);
type DisarmFn = unsafe extern "C" fn();
type CountFn = unsafe extern "C" fn() -> i64;
type TraceFn = unsafe extern "C" fn(*mut c_char, usize) -> usize;

/// Not a real test: the entry point used when this binary is re-executed as a
/// probe child. Without `PROBE_SO` in the environment it does nothing.
#[test]
fn zz_probe_child() {
    let Ok(so) = std::env::var("PROBE_SO") else {
        return;
    };
    let out = std::env::var("PROBE_OUT").expect("PROBE_OUT");

    let lib = unsafe { libloading::Library::new(&so) }.expect("dlopen target .so");
    let f: SearchAndReplaceFn = unsafe { *lib.get(b"searchAndReplace\0").expect("symbol") };

    let orig = arg_from_env("PROBE_ORIG");
    let search = arg_from_env("PROBE_SEARCH");
    let value = arg_from_env("PROBE_VALUE");
    let p = |a: &Option<Vec<u8>>| match a {
        None => std::ptr::null(),
        Some(v) => v.as_ptr() as *const c_char,
    };

    let shim = std::env::var("PROBE_SHIM").ok().map(|s| {
        let l = unsafe { libloading::Library::new(&s) }.expect("dlopen shim");
        let arm: ArmFn = unsafe { *l.get(b"failalloc_arm\0").unwrap() };
        let disarm: DisarmFn = unsafe { *l.get(b"failalloc_disarm\0").unwrap() };
        let count: CountFn = unsafe { *l.get(b"failalloc_count\0").unwrap() };
        let trace: TraceFn = unsafe { *l.get(b"failalloc_trace\0").unwrap() };
        (l, arm, disarm, count, trace)
    });
    let fail_at: i64 = std::env::var("PROBE_FAIL_AT")
        .ok()
        .map(|s| s.parse().unwrap())
        .unwrap_or(0);

    if let Some((_, arm, ..)) = &shim {
        unsafe { arm(fail_at) };
    }
    let ret = unsafe { f(p(&orig), p(&search), p(&value)) };
    let mut count = -1i64;
    let mut trace = String::new();
    if let Some((_, _, disarm, cnt, tr)) = &shim {
        unsafe { disarm() };
        count = unsafe { cnt() };
        let mut buf = vec![0u8; 1 << 16];
        let n = unsafe { tr(buf.as_mut_ptr() as *mut c_char, buf.len()) };
        trace = String::from_utf8_lossy(&buf[..n]).into_owned();
    }

    let result = if ret.is_null() {
        "NULL".to_string()
    } else {
        let mut bytes = Vec::new();
        unsafe {
            let mut q = ret as *const u8;
            while *q != 0 {
                bytes.push(*q);
                q = q.add(1);
            }
            free(ret as *mut c_void);
        }
        format!("OK {}", hex(&bytes))
    };

    let mut file = std::fs::File::create(&out).expect("create PROBE_OUT");
    writeln!(file, "{result}").unwrap();
    writeln!(file, "count={count}").unwrap();
    writeln!(file, "trace={trace}").unwrap();
    file.sync_all().unwrap();
    drop(file);
    std::process::exit(0);
}

fn arg_from_env(key: &str) -> Option<Vec<u8>> {
    match std::env::var(key) {
        Err(_) => None,
        Ok(s) if s == "NULL" => None,
        Ok(s) => {
            let mut v = unhex(&s);
            v.push(0);
            Some(v)
        }
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|c| format!("{c:02x}")).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

// ---------------------------------------------------------------------------
// parent-side probe driver

#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    /// Call returned; `value` is `None` for a NULL return.
    Returned {
        value: Option<Vec<u8>>,
        count: i64,
        trace: String,
    },
    Signal(i32),
    Exit(i32),
    TimedOut,
}

impl Outcome {
    fn value(&self) -> &Option<Vec<u8>> {
        match self {
            Outcome::Returned { value, .. } => value,
            other => panic!("expected a returned value, got {other:?}"),
        }
    }
    fn count(&self) -> i64 {
        match self {
            Outcome::Returned { count, .. } => *count,
            other => panic!("expected a returned value, got {other:?}"),
        }
    }
    fn trace(&self) -> &str {
        match self {
            Outcome::Returned { trace, .. } => trace,
            other => panic!("expected a returned value, got {other:?}"),
        }
    }
}

/// `None` means "pass a NULL pointer".
type Arg<'a> = Option<&'a [u8]>;

fn shim_so() -> &'static PathBuf {
    static S: OnceLock<PathBuf> = OnceLock::new();
    S.get_or_init(|| {
        let dir = manifest_dir().join("target/test-support");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("failalloc.so");
        let src = manifest_dir().join("tests/support/failalloc.c");
        let st = Command::new("cc")
            .args(["-shared", "-fPIC", "-O1", "-o"])
            .arg(&out)
            .arg(&src)
            .arg("-ldl")
            .status()
            .expect("run cc to build the LD_PRELOAD test shim");
        assert!(st.success(), "failed to build {}", src.display());
        assert!(out.is_file());
        out
    })
}

fn so_path(which: &str) -> PathBuf {
    match which {
        "c" => common::c_so_path(),
        "rust-release" => manifest_dir().join("target/release/libdriver.so"),
        "rust-debug" => manifest_dir().join("target/debug/libdriver.so"),
        other => panic!("unknown impl {other}"),
    }
}

/// The implementations compared by every error-path row.
fn all_impls() -> Vec<&'static str> {
    let mut v = vec!["c", "rust-release"];
    if so_path("rust-debug").is_file() {
        v.push("rust-debug");
    }
    v
}

fn run_probe(
    which: &str,
    orig: Arg,
    search: Arg,
    value: Arg,
    fail_at: Option<i64>,
    timeout: Duration,
) -> Outcome {
    let so = so_path(which);
    assert!(so.is_file(), "missing {}", so.display());
    let tmp = manifest_dir().join("target/test-support");
    std::fs::create_dir_all(&tmp).unwrap();
    let out = tmp.join(format!(
        "probe-{which}-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_file(&out);

    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(exe);
    cmd.args(["zz_probe_child", "--exact", "--test-threads=1", "--nocapture"])
        .env("PROBE_SO", &so)
        .env("PROBE_OUT", &out)
        .env(
            "PROBE_ORIG",
            orig.map(hex).unwrap_or_else(|| "NULL".into()),
        )
        .env(
            "PROBE_SEARCH",
            search.map(hex).unwrap_or_else(|| "NULL".into()),
        )
        .env(
            "PROBE_VALUE",
            value.map(hex).unwrap_or_else(|| "NULL".into()),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(k) = fail_at {
        cmd.env("PROBE_SHIM", shim_so())
            .env("PROBE_FAIL_AT", k.to_string())
            .env("LD_PRELOAD", shim_so());
    }

    let mut child = cmd.spawn().expect("spawn probe child");
    let start = Instant::now();
    let status = loop {
        match child.try_wait().unwrap() {
            Some(s) => break s,
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Outcome::TimedOut;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    };

    if let Some(sig) = status.signal() {
        return Outcome::Signal(sig);
    }
    let code = status.code().unwrap_or(-1);
    if code != 0 || !out.is_file() {
        return Outcome::Exit(code);
    }
    let text = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("");
    let value = if first == "NULL" {
        None
    } else {
        Some(unhex(first.trim_start_matches("OK ").trim()))
    };
    let count: i64 = lines
        .next()
        .unwrap_or("count=-1")
        .trim_start_matches("count=")
        .parse()
        .unwrap_or(-1);
    let trace = normalize_trace(lines.next().unwrap_or("trace=").trim_start_matches("trace="));
    Outcome::Returned {
        value,
        count,
        trace,
    }
}

/// Normalises the allocation trace before comparing C against Rust.
///
/// `malloc` and `realloc` are folded into a single class `a`, because
/// `realloc(NULL, n)` is *defined* to behave like `malloc(n)` (C17 7.22.3.5) and
/// LLVM performs exactly that fold in the release build wherever `tmp` is
/// provably NULL: the C `.so` reports `r:3` for the first allocation of a match
/// at offset 0 while the optimised Rust `.so` reports `m:3`. Same size, same
/// index, same result — an allocator-name difference with no observable effect
/// (the unoptimised Rust `.so` emits `r:3` like the C). `strdup` keeps its own
/// class `d`, since substituting it would change what is copied, and the
/// requested SIZES and their ORDER are still compared exactly.
fn normalize_trace(t: &str) -> String {
    t.split(',')
        .filter(|e| !e.is_empty())
        .map(|e| {
            let (op, size) = e.split_once(':').unwrap_or(("?", e));
            let class = match op {
                "m" | "r" => "a",
                other => other,
            };
            format!("{class}:{size}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

const T: Duration = Duration::from_secs(30);

/// Run the same probe against every implementation and require identical outcomes.
fn assert_same_outcome(
    label: &str,
    orig: Arg,
    search: Arg,
    value: Arg,
    fail_at: Option<i64>,
    timeout: Duration,
) -> Outcome {
    let mut expected: Option<(&str, Outcome)> = None;
    for which in all_impls() {
        let got = run_probe(which, orig, search, value, fail_at, timeout);
        match &expected {
            None => expected = Some((which, got)),
            Some((ref_name, want)) => assert_eq!(
                want, &got,
                "{label}: {ref_name} and {which} disagree\n  {ref_name} -> {want:?}\n  {which} -> {got:?}"
            ),
        }
    }
    expected.unwrap().1
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 1-6 — allocator failure

#[test]
fn row01_strdup_failure_returns_null() {
    // No match => line 24 strdup is the only allocation; failing it returns NULL.
    let out = assert_same_outcome(
        "row01",
        Some(b"abc"),
        Some(b"XY"),
        Some(b"z"),
        Some(1),
        T,
    );
    assert_eq!(out.value(), &None, "expected NULL from a failed strdup");
    assert_eq!(out.count(), 1);
    assert_eq!(out.trace(), "d:4", "strdup of \"abc\" asks for 4 bytes");
}

#[test]
fn row02_prefix_malloc_failure_returns_null() {
    // inx_start == 2 > 0 => line 34 malloc(inx_start + 1) is allocation #1.
    let out = assert_same_outcome(
        "row02",
        Some(b"aaXYbb"),
        Some(b"XY"),
        Some(b"z"),
        Some(1),
        T,
    );
    assert_eq!(out.value(), &None);
    assert_eq!(out.trace(), "a:3");
}

#[test]
fn row03_replacement_realloc_failure_returns_null() {
    // Match at offset 0 => no prefix malloc, so allocation #1 is the line 45
    // realloc (with tmp == NULL, i.e. acting as malloc).
    let out = assert_same_outcome(
        "row03",
        Some(b"XYaa"),
        Some(b"XY"),
        Some(b"zzz"),
        Some(1),
        T,
    );
    assert_eq!(out.value(), &None);
    assert_eq!(out.trace(), "a:4");

    // Same branch, reached as allocation #2 when a prefix malloc precedes it.
    let out = assert_same_outcome(
        "row03b",
        Some(b"aaXYbb"),
        Some(b"XY"),
        Some(b"zzz"),
        Some(2),
        T,
    );
    assert_eq!(out.value(), &None);
    assert_eq!(out.trace(), "a:3,a:6");
}

#[test]
fn row04_gap_realloc_failure_returns_null() {
    // "aa XY bbbb XY cc": alloc #1 prefix malloc(3), #2 replacement realloc(4),
    // #3 gap realloc(4 + 4) <- the line 62 branch, failed here.
    let out = assert_same_outcome(
        "row04",
        Some(b"aaXYbbbbXYcc"),
        Some(b"XY"),
        Some(b"z"),
        Some(3),
        T,
    );
    assert_eq!(out.value(), &None);
    assert_eq!(out.trace(), "a:3,a:4,a:8");
}

#[test]
fn row05_tail_realloc_failure_returns_null() {
    // "aa XY bb": #1 prefix malloc(3), #2 replacement realloc(4),
    // #3 tail realloc(4 + 2) <- the line 80 branch, failed here.
    let out = assert_same_outcome(
        "row05",
        Some(b"aaXYbb"),
        Some(b"XY"),
        Some(b"z"),
        Some(3),
        T,
    );
    assert_eq!(out.value(), &None);
    assert_eq!(out.trace(), "a:3,a:4,a:6");
}

#[test]
fn row06_allocation_failure_sweep_matches() {
    // A shape that exercises prefix + several replacements + gaps + tail, then
    // fail each allocation index in turn: C and Rust must fail at the same k,
    // request the same sizes, and succeed identically past the end.
    let cases: [(&[u8], &[u8], &[u8]); 6] = [
        (b"aaXYbbXYccXYdd", b"XY", b"zzz"),
        (b"XYXYXY", b"XY", b"q"),
        (b"aaXY", b"XY", b""),
        (b"XY", b"XY", b"long-replacement"),
        (b"abcabcabc", b"abc", b"abc"),
        (b"hello world", b"o", b"0"),
    ];
    for (orig, search, value) in cases {
        let full = assert_same_outcome("row06/base", Some(orig), Some(search), Some(value), Some(0), T);
        let n = full.count();
        assert!(n >= 1, "expected at least one allocation for {orig:?}");
        for k in 1..=n + 2 {
            let out = assert_same_outcome(
                &format!("row06 k={k} orig={orig:?}"),
                Some(orig),
                Some(search),
                Some(value),
                Some(k),
                T,
            );
            if k <= n {
                assert_eq!(
                    out.value(),
                    &None,
                    "failing allocation #{k} of {n} must yield NULL for {orig:?}"
                );
            } else {
                assert_eq!(
                    out.value(),
                    full.value(),
                    "failing a non-existent allocation #{k} must not change the result"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 7-9 — unchecked NULL arguments

#[test]
fn row07_null_orig_same_signal() {
    let out = assert_same_outcome("row07", None, Some(b"a"), Some(b"b"), None, T);
    assert_eq!(out, Outcome::Signal(11), "expected SIGSEGV from strlen(NULL)");
}

#[test]
fn row08_null_search_same_signal() {
    let out = assert_same_outcome("row08", Some(b"abc"), None, Some(b"b"), None, T);
    assert_eq!(out, Outcome::Signal(11));
}

#[test]
fn row09_null_value_same_signal() {
    // value is unused when there is no match, but strlen(value) runs first.
    let out = assert_same_outcome("row09", Some(b"abc"), Some(b"XY"), None, None, T);
    assert_eq!(out, Outcome::Signal(11));
}

#[test]
fn row07_09_all_null_same_signal() {
    let out = assert_same_outcome("row07-09/all-null", None, None, None, None, T);
    assert_eq!(out, Outcome::Signal(11));
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 10-11 — empty search never terminates

#[test]
fn row10_empty_search_empty_value_both_hang() {
    // strstr(orig, "") always matches at 0 and nothing advances: infinite loop
    // with no memory growth. Both implementations must still be running.
    for orig in [b"".as_slice(), b"abc".as_slice()] {
        let mut outcomes = Vec::new();
        for which in all_impls() {
            let o = run_probe(
                which,
                Some(orig),
                Some(b""),
                Some(b""),
                None,
                Duration::from_secs(2),
            );
            outcomes.push((which, o));
        }
        for (which, o) in &outcomes {
            assert_eq!(
                o,
                &Outcome::TimedOut,
                "{which} terminated for orig={orig:?}, search=\"\" — the C loops forever"
            );
        }
    }
}

#[test]
fn row11_empty_search_nonempty_value_both_exhaust_allocator() {
    // Same non-terminating loop, but total_bytes_allocated grows by value_len
    // every iteration, so the line 45 realloc is retried forever. Instead of
    // waiting for a real OOM we fail the 200th allocation: both must then
    // return NULL after exactly 200 allocations.
    let out = assert_same_outcome(
        "row11",
        Some(b"abc"),
        Some(b""),
        Some(b"zz"),
        Some(200),
        T,
    );
    assert_eq!(out.value(), &None);
    assert_eq!(out.count(), 200);
    // First allocation is realloc(NULL, 1 + 2), then +2 each iteration.
    assert!(out.trace().starts_with("a:3,a:5,a:7,"), "{}", out.trace());
}

// ---------------------------------------------------------------------------
// ERRORS.md row 12 — no enum / integer parameters exist to be out of range

#[test]
fn row12_no_enum_parameters_documented() {
    let header = std::fs::read_to_string(
        manifest_dir().parent().unwrap().join("c_src/include/lib.h"),
    )
    .unwrap();
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| l.contains("searchAndReplace"))
        .collect();
    assert_eq!(decls.len(), 1, "unexpected public surface: {decls:?}");
    let params = decls[0]
        .split_once('(')
        .unwrap()
        .1
        .rsplit_once(')')
        .unwrap()
        .0;
    for p in params.split(',') {
        assert!(
            p.contains('*'),
            "parameter `{p}` is not a pointer — an out-of-range enum/int test \
             would be required for it, see ERRORS.md row 12"
        );
    }
    assert!(
        !header.contains("enum") && !header.contains("typedef"),
        "header gained an enum/typedef; ERRORS.md row 12 must be revisited"
    );
}

// ---------------------------------------------------------------------------
// generic FFI boundary cases (in-process, no crash expected)

#[test]
fn boundary_zero_and_oversized_lengths() {
    // zero-length orig / value, and a search far longer than orig.
    common::assert_same(b"", b"a", b"");
    common::assert_same(b"", b"a", b"replacement");
    common::assert_same(b"a", b"a", b"");
    common::assert_same(b"a", b"aa", b"");
    let big = vec![b'x'; 4096];
    common::assert_same(b"x", &big, b"y");
    common::assert_same(&big, b"x", b"");
    common::assert_same(&big, &big, b"z");
    let mut nearly = big.clone();
    *nearly.last_mut().unwrap() = b'y';
    common::assert_same(&big, &nearly, b"z"); // one byte past a full match
}

#[test]
fn boundary_one_past_match_positions() {
    // search that matches only at the very first / very last possible offset,
    // and searches one byte too long to fit at those offsets.
    for n in 1..=8usize {
        let orig = vec![b'a'; n];
        for slen in 1..=n + 1 {
            let search = vec![b'a'; slen];
            common::assert_same(&orig, &search, b"");
            common::assert_same(&orig, &search, b"Z");
            common::assert_same(&orig, &search, b"ZZZ");
        }
    }
}

#[test]
fn boundary_extreme_bytes() {
    // 0x01 and 0xFF at the edges of the alphabet, and a search made of 0xFF.
    common::assert_same(&[0x01, 0xff, 0x01], &[0xff], &[0x01]);
    common::assert_same(&[0xff, 0xff, 0xff], &[0xff, 0xff], &[0x80]);
    common::assert_same(&[0x80; 16], &[0x80; 4], &[0x7f; 3]);
}

#[test]
fn alloc_trace_parity() {
    // Stronger than return-value equality: the C and Rust implementations must
    // perform the same allocation calls, in the same order, for the same sizes.
    let cases: [(&[u8], &[u8], &[u8]); 8] = [
        (b"abc", b"XY", b"z"),
        (b"XYaa", b"XY", b""),
        (b"aaXY", b"XY", b"zzz"),
        (b"aaXYbbXYcc", b"XY", b"z"),
        (b"XYXY", b"XY", b"QQQ"),
        (b"aaaa", b"aa", b"b"),
        (b"hello world hello", b"hello", b"bye"),
        (b"", b"a", b"b"),
    ];
    for (orig, search, value) in cases {
        let out = assert_same_outcome(
            &format!("trace {orig:?}"),
            Some(orig),
            Some(search),
            Some(value),
            Some(0),
            T,
        );
        assert!(out.count() >= 1);
        eprintln!("{orig:?} -> count={} trace={}", out.count(), out.trace());
    }
}
