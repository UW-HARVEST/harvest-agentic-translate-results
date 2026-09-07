//! Phase C — error-path differential tests, one test per ERRORS.md row.
//! Both implementations are always reached through their `.so` exports.

mod common;
use common::*;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SEED: u64 = 0xE770_0000_0BAD_0001;

// ===================================================================
// Out-of-process harness (rows E1..E5 and the guard-page over-read row)
// ===================================================================

fn aux_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/aux")
}

fn build_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/phase_c_aux");
    std::fs::create_dir_all(&d).expect("create target/phase_c_aux");
    d
}

fn cc() -> String {
    std::env::var("CC").unwrap_or_else(|_| "cc".to_string())
}

fn compile(src: &str, out: &Path, extra: &[&str]) {
    if out.exists() {
        let s_m = std::fs::metadata(aux_dir().join(src))
            .and_then(|m| m.modified())
            .ok();
        let o_m = std::fs::metadata(out).and_then(|m| m.modified()).ok();
        if let (Some(s), Some(o)) = (s_m, o_m) {
            if o >= s {
                return;
            }
        }
    }
    let status = Command::new(cc())
        .arg(aux_dir().join(src))
        .arg("-o")
        .arg(out)
        .args(extra)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", cc()));
    assert!(status.success(), "compiling {src} failed");
}

// Tests run in parallel threads; compile each aux artifact exactly once so no
// test ever execs a half-written binary.
static HARNESS: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
static FAILALLOC: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

fn harness_bin() -> PathBuf {
    HARNESS
        .get_or_init(|| {
            let out = build_dir().join("harness");
            let _ = std::fs::remove_file(&out);
            compile("harness.c", &out, &["-O1", "-ldl"]);
            out
        })
        .clone()
}

fn failalloc_so() -> PathBuf {
    FAILALLOC
        .get_or_init(|| {
            let out = build_dir().join("failalloc.so");
            let _ = std::fs::remove_file(&out);
            compile("failalloc.c", &out, &["-O1", "-shared", "-fPIC", "-ldl"]);
            out
        })
        .clone()
}

/// Run the harness against one `.so`, optionally with the allocator interposer.
fn run_harness(so: &Path, args: &[&str], failalloc: Option<(&str, u32)>) -> Output {
    let mut cmd = Command::new(harness_bin());
    cmd.arg(so).args(args);
    if let Some((mode, skip)) = failalloc {
        cmd.env("LD_PRELOAD", failalloc_so());
        cmd.env("FAILALLOC_MODE", mode);
        cmd.env("FAILALLOC_SKIP", skip.to_string());
    }
    cmd.output().expect("run harness")
}

#[derive(Debug, PartialEq, Eq)]
struct Run {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
}

fn run(so: &Path, args: &[&str], failalloc: Option<(&str, u32)>) -> Run {
    use std::os::unix::process::ExitStatusExt;
    let o = run_harness(so, args, failalloc);
    Run {
        code: o.status.code(),
        signal: o.status.signal(),
        stdout: o.stdout,
    }
}

/// Differential harness run: identical exit status, identical signal, identical stdout.
#[track_caller]
fn check_harness(args: &[&str], failalloc: Option<(&str, u32)>) -> Run {
    let a = run(&c_so_path(), args, failalloc);
    let b = run(&rust_so_path(), args, failalloc);
    assert_eq!(
        a.signal, b.signal,
        "signal divergence for {args:?} failalloc={failalloc:?}: C={:?} RUST={:?}",
        a.signal, b.signal
    );
    assert_eq!(
        a.code, b.code,
        "exit-code divergence for {args:?} failalloc={failalloc:?}: C={:?} RUST={:?}",
        a.code, b.code
    );
    assert_eq!(
        String::from_utf8_lossy(&a.stdout),
        String::from_utf8_lossy(&b.stdout),
        "stdout divergence for {args:?} failalloc={failalloc:?}"
    );
    a
}

fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

// ---------------------------------------------------------------- E1
#[test]
fn e1_drop_null_pointer_aborts() {
    let r = check_harness(&["nullptr_drop"], None);
    // assert(string != NULL) is live (NDEBUG is not defined) -> SIGABRT.
    assert_eq!(
        r.signal,
        Some(6),
        "expected SIGABRT from the failed assert, got {r:?}"
    );
    assert_eq!(r.stdout, b"before\n", "must not return a value");
    // and the assertion message must name the same condition
    let ca = run_harness(&c_so_path(), &["nullptr_drop"], None);
    let ra = run_harness(&rust_so_path(), &["nullptr_drop"], None);
    for (name, o) in [("C", &ca), ("RUST", &ra)] {
        let e = String::from_utf8_lossy(&o.stderr);
        assert!(
            e.contains("string != NULL"),
            "{name} stderr should contain the assertion text, got: {e}"
        );
    }
}

// ---------------------------------------------------------------- E2
#[test]
fn e2_filter_null_pointer_aborts() {
    for flag in ["0", "1", "255"] {
        let r = check_harness(&["nullptr_filter", flag], None);
        assert_eq!(r.signal, Some(6), "expected SIGABRT, got {r:?}");
        assert_eq!(r.stdout, b"before\n");
    }
    let e = String::from_utf8_lossy(
        &run_harness(&rust_so_path(), &["nullptr_filter", "0"], None).stderr,
    )
    .to_string();
    assert!(e.contains("string != NULL"), "rust stderr: {e}");
}

// ---------------------------------------------------------------- E3
#[test]
fn e3_filter_malloc_failure_returns_null() {
    // Needs an input with at least one invalid byte so the malloc path is taken.
    for body in [
        vec![0xFFu8],
        vec![b'a', 0xC0, b'b'],
        b"hello \xED\xA0\x80 world".to_vec(),
    ] {
        let h = to_hex(&body);
        for flag in ["0", "1"] {
            let r = check_harness(&["filter", &h, flag], Some(("malloc", 0)));
            assert_eq!(
                r.stdout, b"NULL\n",
                "malloc failure must yield NULL for input {h} flag {flag}, got {r:?}"
            );
        }
    }
}

// ---------------------------------------------------------------- E4
#[test]
fn e4_filter_realloc_failure_returns_null() {
    // realloc is only reached with replacement != 0 and an invalid byte.
    for body in [
        vec![0xFFu8],
        vec![b'x', 0xC1, b'y', 0xF5],
        vec![0x80u8; 4000],
    ] {
        let h = to_hex(&body);
        let r = check_harness(&["filter", &h, "1"], Some(("realloc", 0)));
        assert_eq!(
            r.stdout, b"NULL\n",
            "first realloc failure must yield NULL for {} bytes, got {r:?}",
            body.len()
        );
        // With replacement == 0 realloc is never called, so the call must succeed
        // identically in both implementations.
        let r0 = check_harness(&["filter", &h, "0"], Some(("realloc", 0)));
        assert!(
            r0.stdout.starts_with(b"OK "),
            "replacement=0 must not call realloc, got {r0:?}"
        );
    }
    // A later realloc failing (second growth cycle) must also return NULL.
    let many = to_hex(&vec![0xFFu8; 4000]);
    let r = check_harness(&["filter", &many, "1"], Some(("realloc", 1)));
    assert_eq!(r.stdout, b"NULL\n", "second realloc failure: {r:?}");
    let r = check_harness(&["filter", &many, "1"], Some(("realloc", 2)));
    assert_eq!(r.stdout, b"NULL\n", "third realloc failure: {r:?}");
}

// ---------------------------------------------------------------- E5
#[test]
fn e5_filter_strdup_failure_returns_null() {
    // Fully valid input -> strdup fast path; the C does not check the result.
    for body in [
        b"".to_vec(),
        b"plain ascii".to_vec(),
        "héllo → 😀".as_bytes().to_vec(),
    ] {
        let h = to_hex(&body);
        for flag in ["0", "1"] {
            let r = check_harness(&["filter", &h, flag], Some(("strdup", 0)));
            assert_eq!(
                r.stdout, b"NULL\n",
                "strdup failure must yield NULL for {h} flag {flag}, got {r:?}"
            );
        }
    }
    // Sanity: without the interposer the same inputs succeed in both.
    for body in [b"".to_vec(), b"plain ascii".to_vec()] {
        let h = to_hex(&body);
        let r = check_harness(&["filter", &h, "0"], None);
        assert!(r.stdout.starts_with(b"OK "), "{r:?}");
    }
}

// ---------------------------------------------------------------- E6
#[test]
fn e6_empty_and_zero_length() {
    check_all(b"");
    assert_eq!(c().drop_offset(b"\0"), 0);
    assert_eq!(rust().drop_offset(b"\0"), 0);
    for flag in [0u8, 1, 0xFF] {
        assert_eq!(c().filter(b"\0", flag).unwrap(), Vec::<u8>::new());
        assert_eq!(rust().filter(b"\0", flag).unwrap(), Vec::<u8>::new());
    }
}

// ---------------------------------------------------------------- E7
#[test]
fn e7_bare_continuation_byte_first() {
    for b in 0x80u8..=0xBF {
        check_all(&[b]);
        check_all(&[b, 0x80]);
        check_all(&[b, b'a']);
        let mut s = vec![b, 0];
        assert_eq!(c().drop_offset(&s), 0);
        assert_eq!(rust().drop_offset(&s), 0);
        s.clear();
    }
}

// ---------------------------------------------------------------- E8
#[test]
fn e8_two_byte_lead_below_range() {
    for lead in [0xC0u8, 0xC1] {
        for b1 in 0x00u8..=0xFF {
            check_all(&[lead, b1]);
            let s = [lead, b1, 0];
            assert_eq!(c().drop_offset(&s), 0, "{lead:#04X} must be rejected");
            assert_eq!(rust().drop_offset(&s), 0);
        }
    }
}

// ---------------------------------------------------------------- E9
#[test]
fn e9_two_byte_lead_boundaries_accepted() {
    for lead in [0xC2u8, 0xC3, 0xDE, 0xDF] {
        for b1 in 0x80u8..=0xBF {
            let s = [lead, b1, 0];
            assert_eq!(c().drop_offset(&s), 2, "{lead:#04X} {b1:#04X} must be valid");
            assert_eq!(rust().drop_offset(&s), 2);
            check_all(&[lead, b1]);
            check_all(&[lead, b1, b'z']);
        }
    }
}

// --------------------------------------------------------------- E10
#[test]
fn e10_four_byte_lead_above_range() {
    for lead in [0xF5u8, 0xF6, 0xF7] {
        for b1 in [0x80u8, 0x8F, 0x90, 0xBF, 0x00, 0xFF] {
            check_all(&[lead, b1, 0x80, 0x80]);
            let s = [lead, b1, 0x80, 0x80, 0];
            assert_eq!(c().drop_offset(&s), 0, "{lead:#04X} must be rejected");
            assert_eq!(rust().drop_offset(&s), 0);
        }
    }
}

// --------------------------------------------------------------- E11
#[test]
fn e11_leads_matching_no_macro() {
    for lead in 0xF8u8..=0xFF {
        for b1 in [0x80u8, 0xBF, 0x00, 0xFF] {
            check_all(&[lead, b1, 0x80, 0x80]);
            let s = [lead, b1, 0x80, 0x80, 0];
            assert_eq!(c().drop_offset(&s), 0);
            assert_eq!(rust().drop_offset(&s), 0);
        }
    }
}

// ------------------------------------------------------ E12 / E13
#[test]
fn e12_e13_lead_e0_overlong_boundary() {
    for b1 in 0x00u8..=0xFF {
        for b2 in [0x80u8, 0xBF, 0x00, 0x7F, 0xC0] {
            check_all(&[0xE0, b1, b2]);
        }
    }
    // exactly one step below / at the boundary
    for (b1, expect) in [(0x9Fu8, 0usize), (0xA0, 3)] {
        let s = [0xE0u8, b1, 0x80, 0];
        assert_eq!(c().drop_offset(&s), expect, "E0 {b1:#04X}");
        assert_eq!(rust().drop_offset(&s), expect);
    }
}

// ------------------------------------------------------ E14 / E15
#[test]
fn e14_e15_lead_ed_surrogate_boundary() {
    for b1 in 0x00u8..=0xFF {
        for b2 in [0x80u8, 0xBF, 0x00, 0x7F, 0xC0] {
            check_all(&[0xED, b1, b2]);
        }
    }
    for (b1, expect) in [(0x9Fu8, 3usize), (0xA0, 0)] {
        let s = [0xEDu8, b1, 0x80, 0];
        assert_eq!(c().drop_offset(&s), expect, "ED {b1:#04X}");
        assert_eq!(rust().drop_offset(&s), expect);
    }
    // every surrogate half must be rejected
    for b1 in 0xA0u8..=0xBF {
        for b2 in 0x80u8..=0xBF {
            let s = [0xEDu8, b1, b2, 0];
            assert_eq!(c().drop_offset(&s), 0);
            assert_eq!(rust().drop_offset(&s), 0);
        }
    }
}

// ------------------------------------------------------ E16 / E17
#[test]
fn e16_e17_lead_f0_overlong_boundary() {
    for b1 in 0x00u8..=0xFF {
        for b2 in [0x80u8, 0xBF, 0x00, 0xC0] {
            check_all(&[0xF0, b1, b2, 0x80]);
        }
    }
    for (b1, expect) in [(0x8Fu8, 0usize), (0x90, 4)] {
        let s = [0xF0u8, b1, 0x80, 0x80, 0];
        assert_eq!(c().drop_offset(&s), expect, "F0 {b1:#04X}");
        assert_eq!(rust().drop_offset(&s), expect);
    }
}

// ------------------------------------------------------ E18 / E19
#[test]
fn e18_e19_lead_f4_upper_boundary() {
    for b1 in 0x00u8..=0xFF {
        for b2 in [0x80u8, 0xBF, 0x00, 0xC0] {
            check_all(&[0xF4, b1, b2, 0x80]);
        }
    }
    for (b1, expect) in [(0x8Fu8, 4usize), (0x90, 0)] {
        let s = [0xF4u8, b1, 0x80, 0x80, 0];
        assert_eq!(c().drop_offset(&s), expect, "F4 {b1:#04X}");
        assert_eq!(rust().drop_offset(&s), expect);
    }
    for b1 in 0x90u8..=0xBF {
        let s = [0xF4u8, b1, 0x80, 0x80, 0];
        assert_eq!(c().drop_offset(&s), 0);
        assert_eq!(rust().drop_offset(&s), 0);
    }
}

// --------------------------------------------------------------- E20
#[test]
fn e20_truncated_sequences_no_overread() {
    // In-process: every lead byte truncated at every point.
    for lead in 0xC0u8..=0xFF {
        for extra in 0usize..=3 {
            let mut v = vec![lead];
            for _ in 0..extra {
                v.push(0x80);
            }
            check_all(&v);
        }
    }
}

// --------------------------------------------------- E20 / CONFIGS C13
#[test]
fn e20_guard_page_detects_reads_past_terminator() {
    // The string ends exactly at a page boundary; the next page is PROT_NONE.
    // A read past the NUL terminator segfaults, which shows up as signal 11.
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for lead in 0xC0u8..=0xFF {
        cases.push(vec![lead]);
        cases.push(vec![lead, 0x80]);
        cases.push(vec![lead, 0x80, 0x80]);
        cases.push(vec![b'a', b'b', lead]);
    }
    cases.push(vec![]);
    cases.push(b"ascii".to_vec());
    cases.push("héllo → 😀".as_bytes().to_vec());

    for body in &cases {
        let h = to_hex(body);
        let r = check_harness(&["guard", &h], None);
        assert_eq!(
            r.signal, None,
            "read past the NUL terminator (signal {:?}) for input {h}",
            r.signal
        );
        assert_eq!(r.code, Some(0), "guard run failed for {h}: {r:?}");
    }
}

// --------------------------------------------------- E21 / CONFIGS C34
#[test]
fn e21_non_canonical_bool_values() {
    let flags: [u8; 8] = [0, 1, 2, 3, 0x10, 0x7F, 0x80, 0xFF];
    let mut rng = Rng::new(SEED + 21);

    // Hand-picked shapes plus randomized inputs, all containing invalid bytes.
    let mut bodies: Vec<Vec<u8>> = vec![
        vec![0xFFu8],
        vec![0xC0u8, 0xC1],
        b"a\xF5b".to_vec(),
        b"\xED\xA0\x80".to_vec(),
        vec![0x80u8; 2000],
    ];
    for _ in 0..400 {
        let n = rng.range(1, 40);
        let mut v = Vec::new();
        for _ in 0..n {
            if rng.range(0, 1) == 0 {
                push_valid_rand(&mut rng, &mut v);
            } else {
                v.push(invalid_lead(&mut rng));
            }
        }
        v.push(invalid_lead(&mut rng));
        bodies.push(v);
    }

    for body in &bodies {
        let mut s = body.clone();
        s.push(0);
        let mut baseline: Option<Vec<u8>> = None;
        for &f in &flags {
            check_filter(body, f);
            let out = c().filter(&s, f).unwrap();
            if f == 0 {
                // dropping mode: nothing else to compare against
                continue;
            }
            match &baseline {
                None => baseline = Some(out),
                Some(b) => assert_eq!(
                    b,
                    &out,
                    "C treats flag byte {f:#04X} differently from 1 for [{}]",
                    hex(body)
                ),
            }
        }
    }
}

// --------------------------------------------------------------- E22
#[test]
fn e22_oversized_input_many_replacements() {
    let mut rng = Rng::new(SEED + 22);
    for n in [4096usize, 8192, 20000, 50000] {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(invalid_lead(&mut rng));
        }
        check_filter(&v, 0);
        check_filter(&v, 1);
        let mut s = v.clone();
        s.push(0);
        // Every invalid byte becomes exactly EF BF BD.
        let out = rust().filter(&s, 1).unwrap();
        assert_eq!(out.len(), 3 * n);
        assert!(out.chunks(3).all(|c| c == [0xEF, 0xBF, 0xBD]));
        assert_eq!(c().filter(&s, 1).unwrap(), out);
        assert_eq!(c().filter(&s, 0).unwrap().len(), 0);
        assert_eq!(rust().filter(&s, 0).unwrap().len(), 0);
    }
}

// --------------------------------------------------------------- E23
#[test]
fn e23_interior_nul_terminates_scanning() {
    // Bytes after the first NUL must never be examined.
    let tails: [&[u8]; 5] = [b"", b"\xff", b"\x80\x80\x80", b"trailing", b"\xf5\xf5"];
    for lead in [
        vec![b'a'],
        vec![0xC2u8],
        vec![0xE1u8, 0x80],
        vec![0xF1u8, 0x80, 0x80],
        vec![0xFFu8],
        vec![],
    ] {
        for tail in tails {
            // buffer = lead ++ NUL ++ tail ++ NUL
            let mut buf = lead.clone();
            buf.push(0);
            buf.extend_from_slice(tail);
            buf.push(0);
            let a = c().drop_offset(&buf);
            let b = rust().drop_offset(&buf);
            assert_eq!(a, b, "drop divergence for [{}]", hex(&buf));
            assert!(
                a <= lead.len(),
                "scanning ran past the interior NUL: offset {a} for [{}]",
                hex(&buf)
            );
            for f in [0u8, 1] {
                let x = c().filter(&buf, f);
                let y = rust().filter(&buf, f);
                assert_eq!(x, y, "filter divergence for [{}] f={f}", hex(&buf));
                // output can never contain content from beyond the NUL
                assert!(x.unwrap().len() <= 3 * lead.len().max(1));
            }
        }
    }
}

// ------------------------------- extra: harness self-check + agreement
#[test]
fn harness_agrees_with_inprocess_results() {
    // Guards against the subprocess harness silently misreporting.
    let mut rng = Rng::new(SEED + 99);
    for _ in 0..150 {
        let n = rng.range(0, 24);
        let mut v = Vec::new();
        for _ in 0..n {
            let b = rng.byte();
            v.push(if b == 0 { 1 } else { b });
        }
        let h = to_hex(&v);
        let mut s = v.clone();
        s.push(0);

        let r = check_harness(&["drop", &h], None);
        let expect = format!("offset {}\n", c().drop_offset(&s));
        assert_eq!(String::from_utf8_lossy(&r.stdout), expect);

        for f in [0u8, 1] {
            let fs = f.to_string();
            let r = check_harness(&["filter", &h, &fs], None);
            let out = c().filter(&s, f).unwrap();
            assert_eq!(
                String::from_utf8_lossy(&r.stdout),
                format!("OK {}\n", to_hex(&out))
            );
        }
    }
}
