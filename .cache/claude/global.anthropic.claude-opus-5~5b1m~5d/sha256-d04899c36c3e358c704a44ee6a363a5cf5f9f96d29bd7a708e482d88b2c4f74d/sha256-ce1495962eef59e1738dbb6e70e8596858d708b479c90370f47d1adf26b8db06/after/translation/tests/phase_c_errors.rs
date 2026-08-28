//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Rows that abort the process (`assert`) or need allocator failure injection
//! run in a child process; see `tests/common/child.rs`.

#![allow(clippy::int_plus_one)]

mod common;

use common::child::{self, ChildSpec};
use common::*;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

unsafe extern "C" {
    fn free(p: *mut std::ffi::c_void);
    fn strlen(s: *const c_char) -> usize;
    fn mmap(
        addr: *mut std::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut std::ffi::c_void;
    fn mprotect(addr: *mut std::ffi::c_void, len: usize, prot: i32) -> i32;
}

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const PAGE: usize = 4096;

/// Place `bytes` + NUL so that the terminator is the very last readable byte
/// before a `PROT_NONE` guard page. Any read past the terminator segfaults.
unsafe fn guarded_input(bytes: &[u8]) -> *const c_char {
    assert!(bytes.len() + 1 <= PAGE, "input must fit in one page");
    unsafe {
        let base = mmap(
            std::ptr::null_mut(),
            2 * PAGE,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        assert!(base as isize != -1, "mmap failed");
        assert_eq!(
            mprotect(base.byte_add(PAGE), PAGE, PROT_NONE),
            0,
            "mprotect failed"
        );
        let start = (base as *mut u8).add(PAGE - (bytes.len() + 1));
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), start, bytes.len());
        *start.add(bytes.len()) = 0;
        start as *const c_char
    }
}

// ===========================================================================
// independent oracle: a third implementation of the C semantics, used to prove
// that C and Rust are not merely equal to each other but equal to what
// `c_src/src/lib.c` says.
// ===========================================================================

fn at(b: &[u8], i: usize) -> u8 {
    // out of range == the NUL terminator
    if i < b.len() { b[i] } else { 0 }
}
fn m_valid1(b: &[u8], i: usize) -> bool {
    at(b, i) & 0x80 == 0
}
fn m_valid2(b: &[u8], i: usize) -> bool {
    let b0 = at(b, i);
    (b0 & 0xE0) == 0xC0 && (b0 as i8) >= (0xC2u8 as i8) && (at(b, i + 1) & 0xC0) == 0x80
}
fn m_valid3(b: &[u8], i: usize) -> bool {
    let b0 = at(b, i);
    let b1 = at(b, i + 1);
    (b0 & 0xF0) == 0xE0
        && (b1 & 0xC0) == 0x80
        && (at(b, i + 2) & 0xC0) == 0x80
        && (b0 != 0xE0 || b1 >= 0xA0)
        && (b0 != 0xED || b1 < 0xA0)
        && (b0 != 0xEF || b1 <= 0xBF)
}
fn m_valid4(b: &[u8], i: usize) -> bool {
    let b0 = at(b, i);
    let b1 = at(b, i + 1);
    (b0 & 0xF8) == 0xF0
        && b0 <= 0xF4
        && (b1 & 0xC0) == 0x80
        && (at(b, i + 2) & 0xC0) == 0x80
        && (at(b, i + 3) & 0xC0) == 0x80
        && (b0 != 0xF0 || b1 >= 0x90)
        && (b0 != 0xF4 || b1 <= 0x8F)
}

/// Offset `w_utf8_drop` must return.
fn model_drop(b: &[u8]) -> usize {
    let mut i = 0;
    while at(b, i) != 0 {
        if m_valid1(b, i) {
            i += 1;
        } else if m_valid2(b, i) {
            i += 2;
        } else if m_valid3(b, i) {
            i += 3;
        } else if m_valid4(b, i) {
            i += 4;
        } else {
            return i;
        }
    }
    i
}

/// Payload `w_utf8_filter` must return.
fn model_filter(b: &[u8], replacement: bool) -> Vec<u8> {
    let k = model_drop(b);
    if k == b.len() {
        return b.to_vec();
    }
    let mut out = b[..k].to_vec();
    let mut i = k;
    while at(b, i) != 0 {
        let w = if m_valid1(b, i) {
            1
        } else if m_valid2(b, i) {
            2
        } else if m_valid3(b, i) {
            3
        } else if m_valid4(b, i) {
            4
        } else {
            0
        };
        if w == 0 {
            if replacement {
                out.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
            }
            i += 1;
        } else {
            out.extend_from_slice(&b[i..i + w]);
            i += w;
        }
    }
    out
}

/// The exact sequence of allocator requests the C makes, as `(kind, size)`.
fn model_allocs(b: &[u8], replacement: bool) -> Vec<(char, usize)> {
    let k = model_drop(b);
    if k == b.len() {
        return vec![('S', b.len())];
    }
    let mut v = vec![('M', b.len() + 1)];
    let mut size = b.len() + 1;
    let mut repl: usize = 0;
    let mut i = k;
    while at(b, i) != 0 {
        let w = if m_valid1(b, i) {
            1
        } else if m_valid2(b, i) {
            2
        } else if m_valid3(b, i) {
            3
        } else if m_valid4(b, i) {
            4
        } else {
            0
        };
        if w == 0 {
            if replacement {
                if repl < 3 {
                    size += 4096;
                    v.push(('R', size));
                    repl += 4096;
                }
                repl -= 3;
            }
            i += 1;
        } else {
            i += w;
        }
    }
    v
}

/// Assert C == Rust == oracle for both entry points.
fn check_against_model(p: &Pair, bytes: &[u8]) {
    diff_drop(p, bytes);
    let buf = cstr_buf(bytes);
    let base = buf.as_ptr() as *const c_char;
    let want_off = model_drop(bytes);
    let got = unsafe { (p.c.drop_)(base) } as usize - base as usize;
    assert_eq!(
        got,
        want_off,
        "oracle disagrees with C on w_utf8_drop for [{}]",
        hex_trunc(bytes, 64)
    );
    for r in [0u8, 1] {
        diff_filter(p, bytes, r);
        let want = model_filter(bytes, r != 0);
        unsafe {
            let cp = (p.c.filter)(base, r);
            assert!(!cp.is_null());
            let got = std::slice::from_raw_parts(cp as *const u8, strlen(cp)).to_vec();
            free(cp.cast());
            assert_eq!(
                got,
                want,
                "oracle disagrees with C on w_utf8_filter(replacement={r}) for [{}]",
                hex_trunc(bytes, 64)
            );
        }
    }
}

// ===========================================================================
// child entry point (no-op unless the DIFF_CHILD_* env vars are set)
// ===========================================================================

fn describe(p: *mut c_char) -> String {
    if p.is_null() {
        return "NULL".to_string();
    }
    unsafe {
        let n = strlen(p);
        let s = std::slice::from_raw_parts(p as *const u8, n);
        let h = child::fnv1a(s);
        let head = hex_trunc(&s[..n.min(32)], 32);
        free(p.cast());
        format!("PTR len={n} hash={h:#018x} head=[{head}]")
    }
}

#[test]
fn zz_child_entry() {
    let mode = match std::env::var(child::ENV_MODE) {
        Ok(m) => m,
        Err(_) => return, // normal test run: nothing to do
    };
    let imp_sel = std::env::var(child::ENV_IMPL).expect("DIFF_CHILD_IMPL");
    let arg: u64 = std::env::var(child::ENV_ARG)
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    let repl: u8 = std::env::var(child::ENV_REPL)
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    let out = PathBuf::from(std::env::var(child::ENV_OUT).expect("DIFF_CHILD_OUT"));
    let input: Option<Vec<u8>> = std::env::var(child::ENV_IN)
        .ok()
        .map(|p| std::fs::read(p).expect("read child input"));
    let imp = load_one(&imp_sel);

    let text = match mode.as_str() {
        "null_drop" => {
            let r = unsafe { (imp.drop_)(std::ptr::null()) };
            format!("UNEXPECTED-RETURN {r:p}")
        }
        "null_filter" => {
            let r = unsafe { (imp.filter)(std::ptr::null(), repl) };
            format!("UNEXPECTED-RETURN {r:p}")
        }
        "guard" => {
            let bytes = input.expect("DIFF_CHILD_IN");
            let mut acc = String::new();
            let chunks: Vec<&[u8]> = if bytes.is_empty() {
                vec![&[][..]]
            } else {
                bytes.chunks(PAGE - 1).collect()
            };
            unsafe {
                for chunk in chunks {
                    let base = guarded_input(chunk);
                    let off = (imp.drop_)(base) as usize - base as usize;
                    let q = (imp.filter)(base, repl);
                    acc.push_str(&format!("drop={off} filter={}\n", describe(q)));
                }
            }
            acc
        }
        "oom_malloc" | "oom_realloc" | "oom_strdup" | "nofail" | "trace" => {
            let fa_path = std::env::var(child::ENV_PRELOAD).expect("DIFF_CHILD_PRELOAD");
            let fa = child::load_failalloc(Path::new(&fa_path));
            let bytes = input.expect("DIFF_CHILD_IN");
            let buf = cstr_buf(&bytes);
            let base = buf.as_ptr() as *const c_char;
            // Warm up lazy PLT binding so that the measured/armed call only
            // performs the library's own allocations.
            unsafe {
                let w = (imp.filter)(base, repl);
                if !w.is_null() {
                    free(w.cast());
                }
            }
            if mode == "trace" {
                unsafe {
                    (fa.trace_begin)();
                    let p = (imp.filter)(base, repl);
                    (fa.trace_end)();
                    let n = (fa.trace_count)();
                    let ov = (fa.trace_overflow)();
                    let mut s = format!("count={n} overflow={ov}\n");
                    for i in 0..n {
                        let k = (fa.trace_kind)(i) as u8 as char;
                        s.push_str(&format!("{k} {}\n", (fa.trace_arg)(i)));
                    }
                    s.push_str(&describe(p));
                    s.push('\n');
                    s
                }
            } else {
                let (m, r, s) = match mode.as_str() {
                    "oom_malloc" => (arg.max(1) as i32, 0, 0),
                    "oom_realloc" => (0, arg.max(1) as i32, 0),
                    "oom_strdup" => (0, 0, arg.max(1) as i32),
                    _ => (0, 0, 0),
                };
                unsafe {
                    // Only requests as large as the input itself are eligible,
                    // so an unrelated small allocation from the Rust runtime
                    // cannot swallow the injected failure.
                    (fa.set_min_size)(bytes.len().max(1));
                    (fa.arm)(m, r, s);
                    let p = (imp.filter)(base, repl);
                    let fired = (fa.fired)();
                    (fa.disarm)();
                    format!("fired={fired} {}", describe(p))
                }
            }
        }
        other => panic!("unknown child mode {other:?}"),
    };
    std::fs::write(&out, text).expect("write child result");
    std::process::exit(0);
}

// ===========================================================================
// E1 / E2 — assert(string != NULL)
// ===========================================================================

fn assert_same_abort(c: &child::ChildResult, r: &child::ChildResult, line: u32, func: &str) {
    assert_eq!(
        c.signal,
        Some(libc_sigabrt()),
        "C child did not die from SIGABRT: {c:?}"
    );
    assert_eq!(
        r.signal,
        Some(libc_sigabrt()),
        "RUST child did not die from SIGABRT: {r:?}"
    );
    assert_eq!(c.signal, r.signal, "abort signal differs");
    assert!(c.result.is_none(), "C child unexpectedly returned: {c:?}");
    assert!(r.result.is_none(), "RUST child unexpectedly returned: {r:?}");
    for (name, res) in [("C", c), ("RUST", r)] {
        let e = &res.stderr;
        assert!(
            e.contains("Assertion") && e.contains("string != NULL"),
            "{name} stderr lacks the assertion text: {e:?}"
        );
        assert!(
            e.contains(&format!(":{line}: {func}:")),
            "{name} stderr lacks \":{line}: {func}:\": {e:?}"
        );
        assert!(
            e.contains("c_src/src/lib.c"),
            "{name} stderr lacks the source file name: {e:?}"
        );
    }
}

fn libc_sigabrt() -> i32 {
    6
}

#[test]
fn e1_null_drop_aborts() {
    let (c, r) = child::run_both(&ChildSpec {
        mode: "null_drop",
        ..Default::default()
    });
    assert_same_abort(&c, &r, 40, "w_utf8_drop");
}

#[test]
fn e2_null_filter_aborts() {
    for repl in [0u8, 1, 0xFF] {
        let (c, r) = child::run_both(&ChildSpec {
            mode: "null_filter",
            repl,
            ..Default::default()
        });
        assert_same_abort(&c, &r, 60, "w_utf8_filter");
    }
}

// ===========================================================================
// E3 — w_utf8_drop returns the first invalid byte
// ===========================================================================

#[test]
fn e3_drop_returns_first_invalid() {
    let p = pair();
    let mut rng = Rng::new(0xE003);
    for class in 0..INVALID_CLASSES {
        for _ in 0..50 {
            let prefix = gen_valid_n(&mut rng, 8);
            let k = prefix.len();
            let mut v = prefix;
            push_invalid(&mut v, &mut rng, class);
            v.extend_from_slice(&gen_valid_n(&mut rng, 8));
            let buf = cstr_buf(&v);
            let base = buf.as_ptr() as *const c_char;
            let (co, ro) = unsafe { ((p.c.drop_)(base), (p.rs.drop_)(base)) };
            let coff = co as usize - base as usize;
            let roff = ro as usize - base as usize;
            assert_eq!(coff, roff, "drop offset mismatch for [{}]", hex(&v));
            assert_eq!(
                coff,
                k,
                "drop should stop at the injected invalid byte (offset {k}) for [{}]",
                hex(&v)
            );
            assert_ne!(unsafe { *co }, 0, "drop must not return the terminator here");
            check_against_model(&p, &v);
        }
    }
}

// ===========================================================================
// E4 / E5 / E6 — allocator failure branches
// ===========================================================================

/// A string that takes the `strdup` fast path.
fn all_valid_input() -> Vec<u8> {
    let mut rng = Rng::new(0x4444);
    gen_valid(&mut rng, 4000, &[1, 2, 3, 4])
}

/// A string with an invalid byte, so `malloc`/`realloc` are used.
fn has_invalid_input() -> Vec<u8> {
    let mut rng = Rng::new(0x5555);
    let mut v = gen_valid(&mut rng, 8, &[1, 2, 3, 4]);
    // exactly 40 invalid bytes (=> exactly one realloc), padded with valid text
    // so the buffer is several KiB long.
    for _ in 0..40 {
        let cls = rng.below(INVALID_CLASSES);
        let before = v.len();
        push_invalid(&mut v, &mut rng, cls);
        assert!(v.len() > before);
        v.extend_from_slice(&gen_valid(&mut rng, 60, &[1, 2, 3, 4]));
    }
    v
}

fn assert_child_results_equal(tag: &str, c: &child::ChildResult, r: &child::ChildResult) {
    assert_eq!(c.signal, None, "[{tag}] C child crashed: {c:?}");
    assert_eq!(r.signal, None, "[{tag}] RUST child crashed: {r:?}");
    let cr = c.result.as_deref().unwrap_or("<missing>");
    let rr = r.result.as_deref().unwrap_or("<missing>");
    assert_eq!(cr, rr, "[{tag}] child result mismatch\nC   : {cr}\nRUST: {rr}");
}

#[test]
fn e4_strdup_oom() {
    let input = all_valid_input();
    let (c, r) = child::run_both(&ChildSpec {
        mode: "oom_strdup",
        arg: 1,
        repl: 0,
        input: Some(&input),
        preload: true,
        ..Default::default()
    });
    assert_child_results_equal("e4", &c, &r);
    assert_eq!(
        c.result.as_deref(),
        Some("fired=1 NULL"),
        "the strdup failure branch must fire exactly once and return NULL, got {:?}",
        c.result
    );
    // and with replacement = 1 (same path: the fast path ignores the flag)
    let (c, r) = child::run_both(&ChildSpec {
        mode: "oom_strdup",
        arg: 1,
        repl: 1,
        input: Some(&input),
        preload: true,
        ..Default::default()
    });
    assert_child_results_equal("e4-r1", &c, &r);
    assert_eq!(c.result.as_deref(), Some("fired=1 NULL"));
}

#[test]
fn e5_malloc_oom() {
    let input = has_invalid_input();
    for repl in [0u8, 1] {
        let (c, r) = child::run_both(&ChildSpec {
            mode: "oom_malloc",
            arg: 1,
            repl,
            input: Some(&input),
            preload: true,
            ..Default::default()
        });
        assert_child_results_equal(&format!("e5-r{repl}"), &c, &r);
        assert_eq!(
            c.result.as_deref(),
            Some("fired=1 NULL"),
            "the malloc failure branch must fire exactly once and return NULL"
        );
    }
    // sanity: without arming, the same call succeeds identically
    let (c, r) = child::run_both(&ChildSpec {
        mode: "nofail",
        repl: 1,
        input: Some(&input),
        preload: true,
        ..Default::default()
    });
    assert_child_results_equal("e5-nofail", &c, &r);
    assert!(
        c.result.as_deref().unwrap().starts_with("fired=0 PTR "),
        "unarmed call must succeed: {:?}",
        c.result
    );
}

#[test]
fn e6_realloc_oom() {
    let input = has_invalid_input();
    // fail the 1st, 2nd and 3rd realloc; with 40 invalid bytes only the first
    // realloc happens, so nth > 1 must NOT fail => identical non-NULL result.
    for nth in [1u64, 2, 3] {
        let (c, r) = child::run_both(&ChildSpec {
            mode: "oom_realloc",
            arg: nth,
            repl: 1,
            input: Some(&input),
            preload: true,
            ..Default::default()
        });
        assert_child_results_equal(&format!("e6-nth{nth}"), &c, &r);
        if nth == 1 {
            assert_eq!(
                c.result.as_deref(),
                Some("fired=1 NULL"),
                "the realloc failure branch must fire once and return NULL"
            );
        } else {
            // 40 invalid bytes => exactly one realloc, so arming the 2nd/3rd
            // must never fire and the call must succeed on both sides.
            assert!(
                c.result.as_deref().unwrap().starts_with("fired=0 PTR "),
                "arming realloc #{nth} must not fire: {:?}",
                c.result
            );
        }
    }
    // with replacement = 0 no realloc is ever performed: arming it must have
    // no effect on either implementation.
    let (c, r) = child::run_both(&ChildSpec {
        mode: "oom_realloc",
        arg: 1,
        repl: 0,
        input: Some(&input),
        preload: true,
        ..Default::default()
    });
    assert_child_results_equal("e6-r0", &c, &r);
    assert!(
        c.result.as_deref().unwrap().starts_with("fired=0 PTR "),
        "replacement=0 performs no realloc, so the call must succeed: {:?}",
        c.result
    );
    // a long invalid run performs many reallocs: failing the 2nd/5th/10th must
    // also return NULL on both sides.
    let long: Vec<u8> = (0..6000u32).map(|i| if i % 2 == 0 { 0x80 } else { 0xC0 }).collect();
    for nth in [1u64, 2, 3, 5] {
        let (c, r) = child::run_both(&ChildSpec {
            mode: "oom_realloc",
            arg: nth,
            repl: 1,
            input: Some(&long),
            preload: true,
            ..Default::default()
        });
        assert_child_results_equal(&format!("e6-long-nth{nth}"), &c, &r);
        assert_eq!(
            c.result.as_deref(),
            Some("fired=1 NULL"),
            "realloc #{nth} failure must fire once and return NULL"
        );
    }
}

// ===========================================================================
// E7 / E8 — the two behaviours of the `replacement` flag
// ===========================================================================

#[test]
fn e7_drop_mode_elides_invalid() {
    let p = pair();
    let mut rng = Rng::new(0xE007);
    for class in 0..INVALID_CLASSES {
        for _ in 0..40 {
            let mut v = gen_valid_n(&mut rng, 6);
            push_invalid(&mut v, &mut rng, class);
            v.extend_from_slice(&gen_valid_n(&mut rng, 6));
            diff_filter(&p, &v, 0);
            let want = model_filter(&v, false);
            assert!(
                !want.windows(3).any(|w| w == [0xEF, 0xBF, 0xBD])
                    || v.windows(3).any(|w| w == [0xEF, 0xBF, 0xBD]),
                "replacement=0 must not synthesise U+FFFD"
            );
            let buf = cstr_buf(&v);
            let base = buf.as_ptr() as *const c_char;
            unsafe {
                let cp = (p.c.filter)(base, 0);
                let got = std::slice::from_raw_parts(cp as *const u8, strlen(cp)).to_vec();
                free(cp.cast());
                assert_eq!(got, want, "C replacement=0 output for [{}]", hex(&v));
                assert!(got.len() < v.len(), "invalid bytes must have been dropped");
            }
        }
    }
}

#[test]
fn e8_replacement_mode_emits_fffd() {
    let p = pair();
    let mut rng = Rng::new(0xE008);
    for class in 0..INVALID_CLASSES {
        for _ in 0..40 {
            let mut v = gen_valid_n(&mut rng, 6);
            push_invalid(&mut v, &mut rng, class);
            v.extend_from_slice(&gen_valid_n(&mut rng, 6));
            diff_filter(&p, &v, 1);
            let with = model_filter(&v, true);
            let without = model_filter(&v, false);
            // each dropped byte becomes exactly 3 bytes
            assert_eq!(
                with.len() - without.len(),
                3 * (v.len() - without.len()),
                "one U+FFFD (3 bytes) per invalid *byte* for [{}]",
                hex(&v)
            );
            let buf = cstr_buf(&v);
            let base = buf.as_ptr() as *const c_char;
            unsafe {
                let cp = (p.c.filter)(base, 1);
                let got = std::slice::from_raw_parts(cp as *const u8, strlen(cp)).to_vec();
                free(cp.cast());
                assert_eq!(got, with, "C replacement=1 output for [{}]", hex(&v));
            }
        }
    }
}

// ===========================================================================
// E9..E25 — every individual clause of valid_1 .. valid_4
// ===========================================================================

/// Drive one clause: embed each probe in a valid prefix/suffix and compare
/// C, Rust and the oracle for both `replacement` settings.
fn clause_row(name: &str, probes: &[Vec<u8>]) {
    let p = pair();
    let mut rng = Rng::new(0xC1A0 ^ fnv(name));
    assert!(!probes.is_empty(), "clause {name} has no probes");
    for probe in probes {
        // bare
        check_against_model(&p, probe);
        // embedded
        for _ in 0..3 {
            let mut v = gen_valid_n(&mut rng, 6);
            v.extend_from_slice(probe);
            v.extend_from_slice(&gen_valid_n(&mut rng, 6));
            check_against_model(&p, &v);
        }
        // repeated
        let mut rep = Vec::new();
        for _ in 0..5 {
            rep.extend_from_slice(probe);
        }
        check_against_model(&p, &rep);
    }
}

fn fnv(s: &str) -> u64 {
    child::fnv1a(s.as_bytes())
}

#[test]
fn e9_valid1_high_bit() {
    // every byte with the high bit set fails valid_1
    let probes: Vec<Vec<u8>> = (0x80u8..=0xFF).map(|b| vec![b, 0x41]).collect();
    clause_row("e9", &probes);
}

#[test]
fn e10_valid2_lead_mask() {
    // (b0 & 0xE0) != 0xC0
    let mut probes = Vec::new();
    for b0 in 0x80u8..=0xFF {
        if (b0 & 0xE0) != 0xC0 {
            probes.push(vec![b0, 0x80]);
        }
    }
    clause_row("e10", &probes);
}

#[test]
fn e11_valid2_overlong_c0_c1() {
    // signed compare: exactly 0xC0 and 0xC1 are rejected
    let mut probes = Vec::new();
    for b0 in [0xC0u8, 0xC1] {
        for b1 in [0x80u8, 0xA5, 0xBF] {
            probes.push(vec![b0, b1]);
        }
    }
    // and 0xC2 (one step *inside* the range) must be accepted
    probes.push(vec![0xC2, 0x80]);
    probes.push(vec![0xC2, 0xBF]);
    clause_row("e11", &probes);
}

#[test]
fn e12_valid2_bad_cont() {
    let mut probes = Vec::new();
    for b0 in [0xC2u8, 0xD0, 0xDF] {
        for b1 in 1u8..=0xFF {
            if (b1 & 0xC0) != 0x80 {
                probes.push(vec![b0, b1]);
            }
        }
        probes.push(vec![b0]); // continuation is the NUL terminator
    }
    clause_row("e12", &probes);
}

#[test]
fn e13_valid3_lead_mask() {
    let mut probes = Vec::new();
    for b0 in 0x80u8..=0xFF {
        if (b0 & 0xF0) != 0xE0 {
            probes.push(vec![b0, 0x80, 0x80]);
        }
    }
    clause_row("e13", &probes);
}

#[test]
fn e14_valid3_bad_cont1() {
    let mut probes = Vec::new();
    for b0 in [0xE0u8, 0xE1, 0xED, 0xEF] {
        for b1 in 1u8..=0xFF {
            if (b1 & 0xC0) != 0x80 {
                probes.push(vec![b0, b1, 0x80]);
            }
        }
        probes.push(vec![b0]); // truncated: b1 is the terminator
    }
    clause_row("e14", &probes);
}

#[test]
fn e15_valid3_bad_cont2() {
    let mut probes = Vec::new();
    for b0 in [0xE1u8, 0xEC, 0xEE, 0xEF] {
        for b2 in 1u8..=0xFF {
            if (b2 & 0xC0) != 0x80 {
                probes.push(vec![b0, 0x80, b2]);
            }
        }
        probes.push(vec![b0, 0x80]); // truncated: b2 is the terminator
    }
    clause_row("e15", &probes);
}

#[test]
fn e16_valid3_overlong_e0() {
    let mut probes = Vec::new();
    for b1 in 0x80u8..=0x9F {
        probes.push(vec![0xE0, b1, 0x80]);
        probes.push(vec![0xE0, b1, 0xBF]);
    }
    // one step past the boundary is valid
    probes.push(vec![0xE0, 0xA0, 0x80]);
    clause_row("e16", &probes);
}

#[test]
fn e17_valid3_surrogate_ed() {
    let mut probes = Vec::new();
    for b1 in 0xA0u8..=0xBF {
        probes.push(vec![0xED, b1, 0x80]);
        probes.push(vec![0xED, b1, 0xBF]);
    }
    // one step below the boundary is valid
    probes.push(vec![0xED, 0x9F, 0xBF]);
    clause_row("e17", &probes);
}

#[test]
fn e18_valid3_ef_clause_unreachable() {
    // The clause `(x)[0] != 0xEF || (unsigned char)(x)[1] <= 0xBF` can never
    // reject: line 21 already requires (b1 & 0xC0) == 0x80, i.e. b1 <= 0xBF.
    // Prove it: every 0xEF sequence with legal continuations is accepted, and
    // every 0xEF sequence that *is* rejected is rejected by an earlier clause.
    let p = pair();
    for b1 in 1u8..=0xFF {
        for b2 in 1u8..=0xFF {
            let v = [0xEFu8, b1, b2];
            let legal_conts = (b1 & 0xC0) == 0x80 && (b2 & 0xC0) == 0x80;
            let off = model_drop(&v);
            assert_eq!(
                off == 3,
                legal_conts,
                "0xEF {b1:02X} {b2:02X}: acceptance must depend only on the \
                 continuation bytes, never on the b1 <= 0xBF clause"
            );
            check_against_model(&p, &v);
        }
    }
}

#[test]
fn e19_valid4_lead_mask() {
    let mut probes = Vec::new();
    for b0 in 0x80u8..=0xFF {
        if (b0 & 0xF8) != 0xF0 {
            probes.push(vec![b0, 0x80, 0x80, 0x80]);
        }
    }
    clause_row("e19", &probes);
}

#[test]
fn e20_valid4_lead_gt_f4() {
    let mut probes = Vec::new();
    for b0 in [0xF5u8, 0xF6, 0xF7] {
        for b1 in [0x80u8, 0xA0, 0xBF] {
            probes.push(vec![b0, b1, 0x80, 0x80]);
        }
    }
    // one step inside the range is valid
    probes.push(vec![0xF4, 0x8F, 0xBF, 0xBF]);
    clause_row("e20", &probes);
}

#[test]
fn e21_valid4_bad_cont1() {
    let mut probes = Vec::new();
    for b0 in [0xF0u8, 0xF1, 0xF4] {
        for b1 in 1u8..=0xFF {
            if (b1 & 0xC0) != 0x80 {
                probes.push(vec![b0, b1, 0x80, 0x80]);
            }
        }
        probes.push(vec![b0]);
    }
    clause_row("e21", &probes);
}

#[test]
fn e22_valid4_bad_cont2() {
    let mut probes = Vec::new();
    for b0 in [0xF1u8, 0xF2, 0xF3] {
        for b2 in 1u8..=0xFF {
            if (b2 & 0xC0) != 0x80 {
                probes.push(vec![b0, 0x80, b2, 0x80]);
            }
        }
        probes.push(vec![b0, 0x80]);
    }
    clause_row("e22", &probes);
}

#[test]
fn e23_valid4_bad_cont3() {
    let mut probes = Vec::new();
    for b0 in [0xF1u8, 0xF2, 0xF3] {
        for b3 in 1u8..=0xFF {
            if (b3 & 0xC0) != 0x80 {
                probes.push(vec![b0, 0x80, 0x80, b3]);
            }
        }
        probes.push(vec![b0, 0x80, 0x80]);
    }
    clause_row("e23", &probes);
}

#[test]
fn e24_valid4_overlong_f0() {
    let mut probes = Vec::new();
    for b1 in 0x80u8..=0x8F {
        probes.push(vec![0xF0, b1, 0x80, 0x80]);
        probes.push(vec![0xF0, b1, 0xBF, 0xBF]);
    }
    probes.push(vec![0xF0, 0x90, 0x80, 0x80]); // one step past: valid
    clause_row("e24", &probes);
}

#[test]
fn e25_valid4_f4_above_max() {
    let mut probes = Vec::new();
    for b1 in 0x90u8..=0xBF {
        probes.push(vec![0xF4, b1, 0x80, 0x80]);
        probes.push(vec![0xF4, b1, 0xBF, 0xBF]);
    }
    probes.push(vec![0xF4, 0x8F, 0xBF, 0xBF]); // one step below: valid
    clause_row("e25", &probes);
}

// ===========================================================================
// E26..E29 — generic FFI boundaries
// ===========================================================================

#[test]
fn e26_empty_string() {
    let p = pair();
    check_against_model(&p, b"");
    let buf = cstr_buf(b"");
    let base = buf.as_ptr() as *const c_char;
    for r in [0u8, 1, 2, 0x80, 0xFF] {
        diff_filter(&p, b"", r);
        unsafe {
            let cp = (p.c.filter)(base, r);
            let rp = (p.rs.filter)(base, r);
            assert!(!cp.is_null() && !rp.is_null());
            assert_eq!(*cp, 0, "C must return an empty string");
            assert_eq!(*rp, 0, "RUST must return an empty string");
            assert_ne!(cp as *const c_char, base, "must be a fresh allocation");
            assert_ne!(rp as *const c_char, base, "must be a fresh allocation");
            free(cp.cast());
            free(rp.cast());
        }
    }
    // drop("") returns the pointer to the terminator, i.e. the input itself
    let co = unsafe { (p.c.drop_)(base) };
    let ro = unsafe { (p.rs.drop_)(base) };
    assert_eq!(co, base);
    assert_eq!(ro, base);
}

#[test]
fn e27_noncanonical_bool_byte() {
    let p = pair();
    let mut rng = Rng::new(0xE027);
    for r in [2u8, 3, 4, 0x10, 0x7F, 0x80, 0x81, 0xFE, 0xFF] {
        for _ in 0..40 {
            let n = 1 + rng.below(30);
            let v = gen_mixed(&mut rng, n);
            diff_filter(&p, &v, r);
            // every non-zero byte must behave exactly like `true`
            let want = model_filter(&v, true);
            let buf = cstr_buf(&v);
            let base = buf.as_ptr() as *const c_char;
            unsafe {
                for (name, f) in [("C", p.c.filter), ("RUST", p.rs.filter)] {
                    let q = f(base, r);
                    assert!(!q.is_null());
                    let got = std::slice::from_raw_parts(q as *const u8, strlen(q)).to_vec();
                    free(q.cast());
                    assert_eq!(
                        got, want,
                        "{name}: replacement={r:#04x} must behave as true for [{}]",
                        hex_trunc(&v, 64)
                    );
                }
            }
        }
    }
}

#[test]
fn e28_noncanonical_bool_upper_bits() {
    let p = pair();
    let mut rng = Rng::new(0xE028);
    let cases: &[(u64, bool)] = &[
        (0x0000_0000_0000_0000, false),
        (0x0000_0000_0000_0100, false),
        (0x0000_0000_FFFF_FF00, false),
        (0xFFFF_FFFF_FFFF_FF00, false),
        (0x0000_00DE_ADBE_EF00, false),
        (0x0000_0000_0000_0001, true),
        (0x0000_0000_0000_01FF, true),
        (0x0000_0000_FFFF_FFFF, true),
        (0xFFFF_FFFF_FFFF_FFFF, true),
        (0x0000_00DE_ADBE_EF01, true),
    ];
    for &(r, expect_true) in cases {
        for _ in 0..30 {
            let n = 1 + rng.below(30);
            let v = gen_mixed(&mut rng, n);
            diff_filter_wide(&p, &v, r);
            let want = model_filter(&v, expect_true);
            let buf = cstr_buf(&v);
            let base = buf.as_ptr() as *const c_char;
            unsafe {
                for (name, f) in [("C", p.c.filter_wide), ("RUST", p.rs.filter_wide)] {
                    let q = f(base, r);
                    assert!(!q.is_null());
                    let got = std::slice::from_raw_parts(q as *const u8, strlen(q)).to_vec();
                    free(q.cast());
                    assert_eq!(
                        got, want,
                        "{name}: only the low byte of replacement={r:#018x} may be read"
                    );
                }
            }
        }
    }
}

#[test]
fn e29_lead_byte_then_nul() {
    let p = pair();
    // A lone lead byte, and a lead byte plus 1..3 continuations, always cut
    // short by the terminator. The scanner must stop at the NUL and must not
    // read past it.
    for lead in [
        0xC0u8, 0xC1, 0xC2, 0xDF, 0xE0, 0xE1, 0xEC, 0xED, 0xEE, 0xEF, 0xF0, 0xF1, 0xF4, 0xF5, 0xF8,
        0xFF,
    ] {
        check_against_model(&p, &[lead]);
        for c1 in [0x80u8, 0xA0, 0xBF] {
            check_against_model(&p, &[lead, c1]);
            for c2 in [0x80u8, 0xBF] {
                check_against_model(&p, &[lead, c1, c2]);
            }
        }
        // and with a valid prefix so the copy loop is involved
        check_against_model(&p, &[0x41, 0x42, lead]);
    }
}

// ===========================================================================
// E30 / E31 — allocation-arithmetic boundaries (compared as exact allocation
// traces recorded by the LD_PRELOAD interposer)
// ===========================================================================

fn trace_case(tag: &str, input: &[u8], repl: u8) {
    let (c, r) = child::run_both(&ChildSpec {
        mode: "trace",
        repl,
        input: Some(input),
        preload: true,
        ..Default::default()
    });
    assert_eq!(c.signal, None, "[{tag}] C child crashed: {c:?}");
    assert_eq!(r.signal, None, "[{tag}] RUST child crashed: {r:?}");
    let ct = c.result.clone().unwrap_or_else(|| panic!("[{tag}] no C trace: {c:?}"));
    let rt = r
        .result
        .clone()
        .unwrap_or_else(|| panic!("[{tag}] no RUST trace: {r:?}"));
    if ct != rt {
        // show the first differing line
        let mut msg = format!("[{tag}] allocation trace mismatch (replacement={repl})\n");
        for (i, (a, b)) in ct.lines().zip(rt.lines()).enumerate() {
            if a != b {
                msg.push_str(&format!("  line {i}: C={a:?} RUST={b:?}\n"));
                break;
            }
        }
        msg.push_str(&format!(
            "  C lines={} RUST lines={}\n",
            ct.lines().count(),
            rt.lines().count()
        ));
        panic!("{msg}");
    }
    // and against the oracle
    let want = model_allocs(input, repl != 0);
    let mut expect = format!("count={} overflow=0\n", want.len());
    for (k, n) in &want {
        expect.push_str(&format!("{k} {n}\n"));
    }
    assert!(
        ct.starts_with(&expect),
        "[{tag}] trace disagrees with the C model (replacement={repl})\n  got : {}\n  want: {}",
        ct.lines().take(6).collect::<Vec<_>>().join(" | "),
        expect.lines().take(6).collect::<Vec<_>>().join(" | ")
    );
}

#[test]
fn e30_replacement_inc_boundary() {
    // 4096 = 3*1365 + 1, so `repl` hits <3 again on replacement 1366, 2731, …
    for n in [1usize, 2, 3, 1364, 1365, 1366, 2730, 2731, 4096] {
        let run: Vec<u8> = std::iter::repeat_n(0x80u8, n).collect();
        trace_case(&format!("e30-run{n}"), &run, 1);
        trace_case(&format!("e30-run{n}-r0"), &run, 0);
        // interleaved with valid ASCII so `size` also grows from the input
        let mixed: Vec<u8> = (0..n).flat_map(|_| [0x41u8, 0xC0]).collect();
        trace_case(&format!("e30-mix{n}"), &mixed, 1);
    }
}

#[test]
fn e31_oversized_length() {
    let mut rng = Rng::new(0xE031);
    // 1 MiB fully valid -> single strdup of a large block
    let mut valid = Vec::with_capacity(1 << 20);
    while valid.len() < (1 << 20) {
        let c = gen_valid(&mut rng, 256, &[1, 2, 3, 4]);
        valid.extend_from_slice(&c);
    }
    trace_case("e31-valid", &valid, 1);
    // 1 MiB fully invalid -> malloc + hundreds of reallocs
    let invalid: Vec<u8> = std::iter::repeat_n(0xF8u8, 1 << 20).collect();
    trace_case("e31-invalid-r1", &invalid, 1);
    trace_case("e31-invalid-r0", &invalid, 0);
    // in-process byte comparison for the same inputs
    let p = pair();
    for r in [0u8, 1] {
        diff_filter(&p, &valid, r);
        diff_filter(&p, &invalid, r);
    }
    diff_drop(&p, &valid);
    diff_drop(&p, &invalid);
}

/// Extra (non-row) hardening: the exact allocation trace must match for a wide
/// range of randomised shapes, not only the boundary cases of E30/E31.
#[test]
fn alloc_trace_random_shapes() {
    let mut rng = Rng::new(0xA110C);
    for i in 0..12 {
        let n = 1 + rng.below(400);
        let v = gen_mixed(&mut rng, n);
        trace_case(&format!("rand{i}"), &v, if i % 2 == 0 { 1 } else { 0 });
    }
    for i in 0..4 {
        let v = gen_valid(&mut rng, 200, &[1, 2, 3, 4]);
        trace_case(&format!("randvalid{i}"), &v, 1);
    }
    for i in 0..4 {
        let v = gen_uniform(&mut rng, 3000);
        trace_case(&format!("randuniform{i}"), &v, 1);
    }
}

// ===========================================================================
// E32 — one step past the end of the buffer: neither implementation may read
// beyond the NUL terminator.
// ===========================================================================

/// The input is placed so that its terminator is the last readable byte before
/// a PROT_NONE guard page; any over-read raises SIGSEGV in the child.
fn guard_case(tag: &str, input: &[u8]) {
    for repl in [0u8, 1] {
        let (c, r) = child::run_both(&ChildSpec {
            mode: "guard",
            repl,
            input: Some(input),
            ..Default::default()
        });
        assert_eq!(
            c.signal, None,
            "[{tag}] the C implementation read past the terminator (signal {:?})\n{c:?}",
            c.signal
        );
        assert_eq!(
            r.signal, None,
            "[{tag}] the RUST implementation read past the terminator (signal {:?})\n{r:?}",
            r.signal
        );
        assert_child_results_equal(&format!("{tag}-r{repl}"), &c, &r);
        assert!(
            c.result.as_deref().map(|s| s.contains("drop=")).unwrap_or(false),
            "[{tag}] child produced no result: {c:?}"
        );
    }
}

#[test]
fn e32_no_read_past_terminator() {
    // Every truncated / lone lead byte: these are exactly the cases where the
    // validity macros are tempted to read x[1], x[2] or x[3] past the NUL.
    let mut probes: Vec<u8> = Vec::new();
    for lead in 0x80u8..=0xFF {
        probes.push(lead);
    }
    guard_case("e32-single-leads", &probes.clone());
    // each lead byte alone, terminated immediately
    for lead in [
        0xC0u8, 0xC1, 0xC2, 0xDF, 0xE0, 0xE1, 0xEC, 0xED, 0xEE, 0xEF, 0xF0, 0xF1, 0xF4, 0xF5, 0xF8,
        0xFF,
    ] {
        guard_case(&format!("e32-lead-{lead:02X}"), &[lead]);
        guard_case(&format!("e32-lead-{lead:02X}-c1"), &[lead, 0x80]);
        guard_case(&format!("e32-lead-{lead:02X}-c2"), &[lead, 0x80, 0x80]);
        guard_case(&format!("e32-lead-{lead:02X}-c3"), &[lead, 0xBF, 0xBF, 0xBF]);
    }
    // and a page-filling random buffer whose terminator sits on the boundary
    let mut rng = Rng::new(0xE032);
    for i in 0..3 {
        let v = gen_uniform(&mut rng, 4095);
        guard_case(&format!("e32-full-page-{i}"), &v);
    }
    for i in 0..3 {
        let v = gen_mixed(&mut rng, 600);
        let v = &v[..v.len().min(4095)];
        guard_case(&format!("e32-mixed-page-{i}"), v);
    }
    // empty string right at the boundary
    guard_case("e32-empty", b"");
}
