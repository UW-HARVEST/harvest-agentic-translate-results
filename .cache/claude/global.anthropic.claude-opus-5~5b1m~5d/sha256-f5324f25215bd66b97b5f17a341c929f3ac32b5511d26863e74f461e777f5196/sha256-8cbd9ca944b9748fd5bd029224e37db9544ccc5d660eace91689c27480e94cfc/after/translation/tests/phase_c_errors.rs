//! Phase C — error/rejection-path differential tests, one test per ERRORS.md row.
//!
//! Rows that provoke undefined behaviour in the C (NULL derefs, out-of-bounds
//! writes) are executed in a forked child process so that *crash parity* can be
//! asserted without taking down the test runner.

mod common;

use common::*;
use std::ffi::c_char;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const SEED: u64 = 0xC0FFEE_0000_0C01;

// ---------------------------------------------------------------------------
// Child-process probe infrastructure (crash / stderr parity)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct ProbeResult {
    code: Option<i32>,
    signal: Option<i32>,
    stderr: Vec<u8>,
}

fn run_probe(which: &str, side: &str) -> ProbeResult {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "probe_child",
            "--nocapture",
            "--test-threads=1",
            "-q",
        ])
        .env("DIFF_PROBE", which)
        .env("DIFF_SIDE", side)
        .output()
        .expect("spawn probe child");
    ProbeResult {
        code: out.status.code(),
        signal: out.status.signal(),
        stderr: out.stderr,
    }
}

/// Asserts C and Rust behave identically for a probe (same exit code / signal).
fn assert_probe_parity(row: &str, which: &str) -> ProbeResult {
    let c = run_probe(which, "c");
    let r = run_probe(which, "rust");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{row}] probe {which:?}: termination diverged\n C   = code {:?} signal {:?}\n \
         Rust = code {:?} signal {:?}\n C stderr={:?}\n Rust stderr={:?}",
        c.code,
        c.signal,
        r.code,
        r.signal,
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    c
}

/// Asserts C and Rust produce identical stderr for a probe.
fn assert_probe_stderr_parity(row: &str, which: &str) {
    let c = run_probe(which, "c");
    let r = run_probe(which, "rust");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{row}] probe {which:?}: termination diverged"
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{row}] probe {which:?}: STDERR diverged\n C   = {:?}\n Rust= {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
}

/// The probe body. A no-op unless `DIFF_PROBE` is set, so it is inert during a
/// normal test run and becomes the payload when re-executed as a child.
#[test]
fn probe_child() {
    let which = match std::env::var("DIFF_PROBE") {
        Ok(w) => w,
        Err(_) => return,
    };
    let side = std::env::var("DIFF_SIDE").unwrap_or_default();
    let imp = if side == "c" { c_impl() } else { rust_impl() };

    match which.as_str() {
        // ERRORS.md row 26 — parse_uname_string(NULL, &osd)
        "parse_null_uname" => unsafe {
            let mut osd = os_data::zeroed();
            (imp.parse_uname_string)(std::ptr::null_mut(), &mut osd);
            println!("survived {:?}", osd.os_name);
        },
        // ERRORS.md row 27 — get_os_arch(NULL)
        "arch_null" => unsafe {
            let p = imp.get_arch_raw(std::ptr::null_mut());
            println!("survived {p:?}");
        },
        // ERRORS.md row 28 — w_regexec with pmatch == NULL, nmatch > 0
        "regexec_null_pmatch" => unsafe {
            let pat = c"^([0-9]+)".as_ptr();
            let sub = c"123".as_ptr();
            let r = imp.regexec_raw(pat, sub, 2, std::ptr::null_mut());
            println!("survived {r}");
        },
        "regexec_null_pmatch_n1" => unsafe {
            let pat = c"^([0-9]+)".as_ptr();
            let sub = c"123".as_ptr();
            let r = imp.regexec_raw(pat, sub, 1, std::ptr::null_mut());
            println!("survived {r}");
        },
        // nmatch == 0 with NULL pmatch is fine (nothing written).
        "regexec_null_pmatch_n0" => unsafe {
            let pat = c"^([0-9]+)".as_ptr();
            let sub = c"123".as_ptr();
            let r = imp.regexec_raw(pat, sub, 0, std::ptr::null_mut());
            println!("ret {r}");
        },
        // ERRORS.md row 29 — SIZE_MAX nmatch
        "regexec_nmatch_max" => {
            let (ret, pm) = imp.regexec(Some(PAT_BUILD), Some(b"1.2.3.4"), usize::MAX, &pm_init(8));
            println!("ret {ret} pm {pm:?}");
        }
        // ERRORS.md row 6 — invalid regex: the stderr message.
        "bad_regex_stderr" => {
            for pat in BAD_PATTERNS {
                let (ret, _) = imp.regexec(Some(pat), Some(b"abc"), 2, &pm_init(2));
                println!("ret {ret}");
            }
        }
        // ERRORS.md rows 22-24 — 1-byte heap underflow writes.
        "heap_underflow_name" => {
            let o = imp.parse(b"Linux [");
            println!("{}", o.snapshot.describe());
        }
        "heap_underflow_version" => {
            let o = imp.parse(b"L [n: ");
            println!("{}", o.snapshot.describe());
        }
        "heap_underflow_codename" => {
            let o = imp.parse(b"L [n: 1.2 ()");
            println!("{}", o.snapshot.describe());
        }
        other => panic!("unknown probe {other}"),
    }
}

/// Patterns that make `regcomp` fail (ERRORS.md row 6).
const BAD_PATTERNS: &[&[u8]] = &[
    br"[",
    br"(",
    br")",
    br"a{2,1}",
    br"*",
    br"\",
    br"[z-a]",
    br"a**+",
    br"[[:bogus:]]",
    br"(a",
    br"a)",
    br"{1}",
    br"a{",
    br"[a",
    br"(()",
];

// ===========================================================================
// Row 1 / 2 — get_os_arch returns NULL
// ===========================================================================

#[test]
fn err01_get_os_arch_no_match_returns_null() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..4000 {
        let s = rng.safe_word(0, 60);
        let a = c.get_arch(&s);
        let b = r.get_arch(&s);
        assert_eq!(a, None, "err01/#{i}: C unexpectedly matched {s:?}");
        assert_eq!(b, None, "err01/#{i}: Rust unexpectedly matched {s:?}");
    }
    for s in [
        &b"Linux ppc64le 5.4"[..],
        b"riscv64",
        b"mips",
        b"s390x",
        b"powerpc",
        b"ppc64",
        b"alpha",
    ] {
        assert_arch_eq("err01/fixed", s);
        assert_eq!(c.get_arch(s), None);
        assert_eq!(r.get_arch(s), None);
    }
}

#[test]
fn err02_get_os_arch_empty_returns_null() {
    let (c, r) = both();
    assert_eq!(c.get_arch(b""), None);
    assert_eq!(r.get_arch(b""), None);
    assert_arch_eq("err02", b"");
}

// ===========================================================================
// Rows 3-5 — w_regexec NULL guard
// ===========================================================================

#[test]
fn err03_w_regexec_null_pattern() {
    let (c, r) = both();
    let init = pm_init(2);
    let (rc, pc) = c.regexec(None, Some(b"123"), 2, &init);
    let (rr, pr) = r.regexec(None, Some(b"123"), 2, &init);
    assert_eq!(rc, 0, "err03: C must return 0 for NULL pattern");
    assert_eq!(rr, 0, "err03: Rust must return 0 for NULL pattern");
    assert_eq!(pc, init, "err03: C must leave pmatch untouched");
    assert_eq!(pr, init, "err03: Rust must leave pmatch untouched");
    assert_regexec_eq("err03", None, Some(b"123"), 2, &init);
    assert_regexec_eq("err03/n0", None, Some(b""), 0, &init);
    assert_regexec_eq("err03/n8", None, Some(b"abc"), 8, &pm_init(8));
}

#[test]
fn err04_w_regexec_null_string() {
    let (c, r) = both();
    let init = pm_init(2);
    let (rc, pc) = c.regexec(Some(PAT_MAJOR), None, 2, &init);
    let (rr, pr) = r.regexec(Some(PAT_MAJOR), None, 2, &init);
    assert_eq!(rc, 0, "err04: C must return 0 for NULL string");
    assert_eq!(rr, 0, "err04: Rust must return 0 for NULL string");
    assert_eq!(pc, init);
    assert_eq!(pr, init);
    assert_regexec_eq("err04", Some(PAT_MAJOR), None, 2, &init);
    // Note: the NULL guard runs *before* regcomp, so an invalid pattern with a
    // NULL string must NOT print anything.
    for p in BAD_PATTERNS {
        assert_regexec_eq("err04/bad", Some(p), None, 2, &init);
    }
}

#[test]
fn err05_w_regexec_both_null() {
    let (c, r) = both();
    let init = pm_init(4);
    let (rc, pc) = c.regexec(None, None, 4, &init);
    let (rr, pr) = r.regexec(None, None, 4, &init);
    assert_eq!((rc, &pc), (0, &init));
    assert_eq!((rr, &pr), (0, &init));
    assert_regexec_eq("err05", None, None, 4, &init);
    assert_regexec_eq("err05/n0", None, None, 0, &init);
}

// ===========================================================================
// Row 6 — regcomp failure: return 0 AND the exact stderr message
// ===========================================================================

#[test]
fn err06_w_regexec_regcomp_failure_return_value() {
    let (c, r) = both();
    let init = pm_init(2);
    for (i, pat) in BAD_PATTERNS.iter().enumerate() {
        let (rc, pc) = c.regexec(Some(pat), Some(b"abc"), 2, &init);
        let (rr, pr) = r.regexec(Some(pat), Some(b"abc"), 2, &init);
        assert_eq!(
            rc,
            rr,
            "err06/#{i}: return diverged for pattern {:?} (C={rc}, Rust={rr})",
            String::from_utf8_lossy(pat)
        );
        assert_eq!(
            pc,
            pr,
            "err06/#{i}: pmatch diverged for pattern {:?}",
            String::from_utf8_lossy(pat)
        );
    }
}

#[test]
fn err06b_w_regexec_regcomp_failure_stderr_message() {
    // Runs in child processes so stderr can be captured and compared verbatim.
    assert_probe_stderr_parity("err06b", "bad_regex_stderr");
    // Sanity: the message really is emitted (so the parity check is meaningful).
    let c = run_probe("bad_regex_stderr", "c");
    let needle = b"Couldn't compile regular expression '";
    let hits = c
        .stderr
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count();
    assert!(
        hits > 0,
        "err06b: expected the C diagnostic on stderr, got {:?}",
        String::from_utf8_lossy(&c.stderr)
    );
    let r = run_probe("bad_regex_stderr", "rust");
    let rhits = r
        .stderr
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count();
    assert_eq!(
        hits, rhits,
        "err06b: diagnostic count diverged (C={hits}, Rust={rhits})"
    );
}

// ===========================================================================
// Row 7 — valid pattern, no match
// ===========================================================================

#[test]
fn err07_w_regexec_no_match_returns_zero() {
    let (c, r) = both();
    let cases: &[(&[u8], &[u8])] = &[
        (PAT_MAJOR, b"abc"),
        (PAT_MAJOR, b""),
        (PAT_MINOR, b"10"),
        (PAT_BUILD, b"10.0"),
        (br"^a$", b"b"),
        (br"^zzz", b"aaa"),
    ];
    for (i, (p, s)) in cases.iter().enumerate() {
        let (rc, _) = c.regexec(Some(p), Some(s), 2, &pm_init(2));
        let (rr, _) = r.regexec(Some(p), Some(s), 2, &pm_init(2));
        assert_eq!(rc, 0, "err07/#{i}: C should not match");
        assert_eq!(rr, 0, "err07/#{i}: Rust should not match");
        assert_regexec_eq(&format!("err07/#{i}"), Some(p), Some(s), 2, &pm_init(2));
    }
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..3000 {
        let s = rng.safe_word(0, 20);
        for p in [PAT_MAJOR, PAT_MINOR, PAT_BUILD] {
            assert_regexec_eq(&format!("err07/rand{i}"), Some(p), Some(&s), 2, &pm_init(2));
        }
    }
}

// ===========================================================================
// Row 8 — nmatch == 0
// ===========================================================================

#[test]
fn err08_w_regexec_nmatch_zero() {
    let init = pm_init(4);
    for p in [PAT_MAJOR, PAT_MINOR, PAT_BUILD, &br"(a)(b)"[..]] {
        for s in [&b"1.2.3"[..], b"abc", b"", b"ab"] {
            assert_regexec_eq("err08", Some(p), Some(s), 0, &init);
        }
    }
    // With nmatch == 0 the array must be completely untouched.
    let (c, r) = both();
    let (rc, pc) = c.regexec(Some(PAT_MAJOR), Some(b"1.2.3"), 0, &init);
    let (rr, pr) = r.regexec(Some(PAT_MAJOR), Some(b"1.2.3"), 0, &init);
    assert_eq!(pc, init, "err08: C wrote pmatch with nmatch=0");
    assert_eq!(pr, init, "err08: Rust wrote pmatch with nmatch=0");
    assert_eq!(rc, rr);
    // NULL pmatch with nmatch == 0 must also be safe and identical.
    let a = assert_probe_parity("err08/null-n0", "regexec_null_pmatch_n0");
    assert_eq!(a.signal, None, "err08: nmatch=0 with NULL pmatch crashed");
}

// ===========================================================================
// Row 9 — osd == NULL
// ===========================================================================

#[test]
fn err09_parse_null_osd() {
    let (c, r) = both();
    let cases: &[&[u8]] = &[
        b"W [Ver: 10.0.19041]",
        b"L [ubuntu: 20.04 (focal)]",
        b"L x86_64",
        b"",
        b"nothing",
    ];
    for (i, u) in cases.iter().enumerate() {
        let a = c.parse_null_osd(u);
        let b = r.parse_null_osd(u);
        let mut untouched = u.to_vec();
        untouched.push(0);
        assert_eq!(
            a, untouched,
            "err09/#{i}: C mutated uname despite NULL osd"
        );
        assert_eq!(a, b, "err09/#{i}: diverged");
    }
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..3000 {
        let u = fuzz_like(&mut rng);
        let a = c.parse_null_osd(&u);
        let b = r.parse_null_osd(&u);
        assert_eq!(a, b, "err09/rand{i}: diverged for {u:?}");
    }
}

// ===========================================================================
// Row 10 — no separator at all
// ===========================================================================

#[test]
fn err10_no_separator_only_arch_set() {
    let (c, r) = both();
    let sc = Sentinels::new();
    let sr = Sentinels::new();
    let cases: &[&[u8]] = &[
        b"Linux x86_64",
        b"Linux",
        b"",
        b"[Ver: 1.2.3]",
        b"[ubuntu: 20.04]",
        b"no-separators-here",
    ];
    for (i, u) in cases.iter().enumerate() {
        let a = c.parse_sent(u, &sc);
        let b = r.parse_sent(u, &sr);
        // Only os_arch (index 8) may differ from its sentinel.
        for k in 0..8 {
            assert_eq!(
                a.snapshot.fields[k],
                Some(format!("<<untouched-{k}>>").into_bytes()),
                "err10/#{i}: C wrote {} for {:?}",
                FIELD_NAMES[k],
                String::from_utf8_lossy(u)
            );
        }
        assert_eq!(a, b, "err10/#{i}: diverged");
    }
}

// ===========================================================================
// Rows 11-13 — Windows branch regex failures
// ===========================================================================

#[test]
fn err11_windows_non_numeric_version() {
    let (c, r) = both();
    let cases: &[&[u8]] = &[
        b"W [Ver: abc]",
        b"W [Ver: .1.2]",
        b"W [Ver: -1.2.3]",
        b"W [Ver: v10.0.1]",
        b"W [Ver: ]",
        b"W [Ver:  ]",
    ];
    for (i, u) in cases.iter().enumerate() {
        let a = c.parse(u);
        let b = r.parse(u);
        assert_eq!(a.snapshot.fields[2], None, "err11/#{i}: C set os_major");
        assert_eq!(a.snapshot.fields[3], None, "err11/#{i}: C set os_minor");
        assert_eq!(a.snapshot.fields[6], None, "err11/#{i}: C set os_build");
        assert_eq!(
            a.snapshot.fields[5],
            Some(b"windows".to_vec()),
            "err11/#{i}: os_platform"
        );
        assert_eq!(a, b, "err11/#{i}: diverged");
    }
}

#[test]
fn err12_windows_major_only() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..2000 {
        let mut u = Vec::from(&b"W [Ver: "[..]);
        u.extend_from_slice(&rng.digits(1, 6));
        u.push(b']');
        let a = c.parse(&u);
        let b = r.parse(&u);
        assert!(a.snapshot.fields[2].is_some(), "err12/#{i}: os_major unset");
        assert_eq!(a.snapshot.fields[3], None, "err12/#{i}: os_minor set");
        assert_eq!(a.snapshot.fields[6], None, "err12/#{i}: os_build set");
        assert_eq!(a, b, "err12/#{i}: diverged");
    }
}

#[test]
fn err13_windows_major_minor_only() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..2000 {
        let mut u = Vec::from(&b"W [Ver: "[..]);
        u.extend_from_slice(&rng.digits(1, 5));
        u.push(b'.');
        u.extend_from_slice(&rng.digits(1, 5));
        u.push(b']');
        let a = c.parse(&u);
        let b = r.parse(&u);
        assert!(a.snapshot.fields[2].is_some(), "err13/#{i}: os_major unset");
        assert!(a.snapshot.fields[3].is_some(), "err13/#{i}: os_minor unset");
        assert_eq!(a.snapshot.fields[6], None, "err13/#{i}: os_build set");
        assert_eq!(a, b, "err13/#{i}: diverged");
    }
}

// ===========================================================================
// Rows 14-18 — Unix branch missing sub-separators
// ===========================================================================

#[test]
fn err14_unix_no_colon() {
    let (c, r) = both();
    let cases: &[&[u8]] = &[b"Linux [ubuntu]", b"L [x]", b"L [ab]", b"L [:]", b"L [:x]"];
    for (i, u) in cases.iter().enumerate() {
        let a = c.parse(u);
        let b = r.parse(u);
        assert_eq!(a.snapshot.fields[1], None, "err14/#{i}: os_version set");
        assert_eq!(a.snapshot.fields[2], None, "err14/#{i}: os_major set");
        assert_eq!(a.snapshot.fields[3], None, "err14/#{i}: os_minor set");
        assert_eq!(a.snapshot.fields[4], None, "err14/#{i}: os_codename set");
        assert_eq!(a, b, "err14/#{i}: diverged");
    }
}

#[test]
fn err15_unix_no_codename() {
    let (c, r) = both();
    let cases: &[&[u8]] = &[b"Linux [ubuntu: 20.04]", b"L [n: 1]", b"L [n: (x)]"];
    for (i, u) in cases.iter().enumerate() {
        let a = c.parse(u);
        let b = r.parse(u);
        assert_eq!(a.snapshot.fields[4], None, "err15/#{i}: os_codename set");
        assert_eq!(a, b, "err15/#{i}: diverged");
    }
}

#[test]
fn err16_unix_no_pipe_platform() {
    let (c, r) = both();
    let cases: &[&[u8]] = &[b"Linux [ubuntu: 20.04]", b"L [n]", b"L [n: 1.2 (c)]"];
    for (i, u) in cases.iter().enumerate() {
        let a = c.parse(u);
        let b = r.parse(u);
        assert_eq!(a.snapshot.fields[5], None, "err16/#{i}: os_platform set");
        assert_eq!(a, b, "err16/#{i}: diverged");
    }
}

#[test]
fn err17_unix_non_numeric_version() {
    let (c, r) = both();
    let cases: &[&[u8]] = &[
        b"L [name: notaversion]",
        b"L [n: .1]",
        b"L [n: -1.2]",
        b"L [n: abc (x)]",
    ];
    for (i, u) in cases.iter().enumerate() {
        let a = c.parse(u);
        let b = r.parse(u);
        assert_eq!(a.snapshot.fields[2], None, "err17/#{i}: os_major set");
        assert_eq!(a.snapshot.fields[3], None, "err17/#{i}: os_minor set");
        assert_eq!(a, b, "err17/#{i}: diverged");
    }
}

#[test]
fn err18_unix_major_only() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..2000 {
        let mut u = Vec::from(&b"L [n: "[..]);
        u.extend_from_slice(&rng.digits(1, 6));
        u.push(b']');
        let a = c.parse(&u);
        let b = r.parse(&u);
        assert!(a.snapshot.fields[2].is_some(), "err18/#{i}: os_major unset");
        assert_eq!(a.snapshot.fields[3], None, "err18/#{i}: os_minor set");
        assert_eq!(a, b, "err18/#{i}: diverged");
    }
}

// ===========================================================================
// Row 19 — os_uname never written
// ===========================================================================

#[test]
fn err19_os_uname_never_written() {
    let (c, r) = both();
    let sc = Sentinels::new();
    let sr = Sentinels::new();
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..4000 {
        let u = fuzz_like(&mut rng);
        let a = c.parse_sent(&u, &sc);
        let b = r.parse_sent(&u, &sr);
        assert_eq!(
            a.snapshot.fields[7],
            Some(b"<<untouched-7>>".to_vec()),
            "err19/#{i}: C wrote os_uname"
        );
        assert_eq!(
            b.snapshot.fields[7],
            Some(b"<<untouched-7>>".to_vec()),
            "err19/#{i}: Rust wrote os_uname"
        );
        assert_eq!(a, b, "err19/#{i}: diverged");
    }
}

// ===========================================================================
// Row 20 — Windows branch never sets os_arch / os_codename
// ===========================================================================

#[test]
fn err20_windows_never_sets_arch_or_codename() {
    let (c, r) = both();
    let sc = Sentinels::new();
    let sr = Sentinels::new();
    for (ai, arch) in ARCHS.iter().enumerate() {
        let mut u = Vec::from(&b"W "[..]);
        u.extend_from_slice(arch);
        u.extend_from_slice(b" [Ver: 10.0.1 (build)]");
        let a = c.parse(&u);
        let b = r.parse(&u);
        assert_eq!(a.snapshot.fields[8], None, "err20/#{ai}: C set os_arch");
        assert_eq!(a.snapshot.fields[4], None, "err20/#{ai}: C set os_codename");
        assert_eq!(a, b, "err20/#{ai}: diverged");

        let sa = c.parse_sent(&u, &sc);
        let sb = r.parse_sent(&u, &sr);
        assert_eq!(
            sa.snapshot.fields[8],
            Some(b"<<untouched-8>>".to_vec()),
            "err20/#{ai}: os_arch not left untouched"
        );
        assert_eq!(sa, sb, "err20/#{ai}: sentinel run diverged");
    }
}

// ===========================================================================
// Row 21 — in-buffer underflow write (Windows branch, empty version)
// ===========================================================================

#[test]
fn err21_windows_empty_version_underflow_in_buffer() {
    // `*(str_tmp - 1)` lands on the space of " [Ver: ", i.e. inside the
    // caller's buffer -> safe to run in-process, and the buffer comparison in
    // `assert_parse_eq` proves the exact byte that was clobbered.
    let (c, r) = both();
    for (i, u) in [&b"W [Ver: "[..], b" [Ver: ", b"[Ver:  [Ver: "]
        .iter()
        .enumerate()
    {
        let a = c.parse(u);
        let b = r.parse(u);
        assert_eq!(
            a, b,
            "err21/#{i}: diverged for {:?}\n C   ={:?}\n Rust={:?}",
            String::from_utf8_lossy(u),
            a.buffer,
            b.buffer
        );
        assert_parse_both_inits(&format!("err21/#{i}"), u);
    }
    // Verify the clobber really happened in C: the byte before the version
    // start (offset 16 guard + strlen(prefix)) must now be NUL.
    let a = c.parse(b"W [Ver: ");
    assert_eq!(
        a.buffer[16 + b"W [Ver: ".len() - 1],
        0,
        "err21: expected the C underflow write inside the uname buffer"
    );
}

// ===========================================================================
// Rows 22-24 — 1-byte heap underflow (run isolated)
// ===========================================================================

#[test]
fn err22_heap_underflow_os_name() {
    assert_probe_parity("err22", "heap_underflow_name");
    // Also compare the observable field values in-process (the corrupted block
    // is leaked, never freed, so the process survives).
    assert_parse_eq("err22", b"Linux [");
    assert_parse_eq("err22b", b" [");
    assert_parse_eq("err22c", b"a [");
}

#[test]
fn err23_heap_underflow_os_version() {
    assert_probe_parity("err23", "heap_underflow_version");
    assert_parse_eq("err23", b"L [n: ");
    assert_parse_eq("err23b", b" [: ");
    assert_parse_eq("err23c", b"L [n|p: ");
}

#[test]
fn err24_heap_underflow_os_codename() {
    assert_probe_parity("err24", "heap_underflow_codename");
    assert_parse_eq("err24", b"L [n: 1.2 ()");
    assert_parse_eq("err24b", b"L [n: 1.2 (x");
    assert_parse_eq("err24c", b"L [n: 1.2 ( ");
}

// ===========================================================================
// Row 25 — non-participating group yields "" (not NULL)
// ===========================================================================

#[test]
fn err25_non_participating_group_yields_empty_string() {
    let (c, r) = both();
    // The build regex's inner group `(\.[0-9]+)*` does not participate when the
    // build has exactly one component, but group 1 always does; assert C/Rust
    // agree for every arity and check the group-2 handling via w_regexec.
    for (i, v) in [
        &b"1.2.3"[..],
        b"1.2.3.4",
        b"1.2.3.4.5",
        b"1.2.30",
        b"0.0.0",
    ]
    .iter()
    .enumerate()
    {
        assert_regexec_eq(
            &format!("err25/build{i}"),
            Some(PAT_BUILD),
            Some(v),
            3,
            &pm_init(3),
        );
        let mut u = Vec::from(&b"W [Ver: "[..]);
        u.extend_from_slice(v);
        u.push(b']');
        assert_parse_eq(&format!("err25/parse{i}"), &u);
    }
    // A pattern where group 1 itself does not participate: C then computes
    // match_size = -1 - -1 = 0, mallocs 1 byte and snprintf's an empty string.
    for (i, (p, s)) in [
        (&br"^(a)|b"[..], &b"b"[..]),
        (br"(x)?y", b"y"),
        (br"^(q)|[0-9]+", b"55"),
    ]
    .iter()
    .enumerate()
    {
        let (rc, pc) = c.regexec(Some(p), Some(s), 2, &pm_init(2));
        let (rr, pr) = r.regexec(Some(p), Some(s), 2, &pm_init(2));
        assert_eq!(rc, rr, "err25/np{i}: return diverged");
        assert_eq!(pc, pr, "err25/np{i}: pmatch diverged");
        if rc != 0 {
            assert_eq!(
                pc[1],
                regmatch_t { rm_so: -1, rm_eo: -1 },
                "err25/np{i}: expected a non-participating group"
            );
        }
    }
}

// ===========================================================================
// Rows 26-28 — unchecked NULL pointers: crash parity
// ===========================================================================

#[test]
fn err26_parse_null_uname_crash_parity() {
    let p = assert_probe_parity("err26", "parse_null_uname");
    assert!(
        p.signal.is_some(),
        "err26: expected the C to fault on a NULL uname, got exit {:?}",
        p.code
    );
}

#[test]
fn err27_get_os_arch_null_crash_parity() {
    let p = assert_probe_parity("err27", "arch_null");
    assert!(
        p.signal.is_some(),
        "err27: expected the C to fault on a NULL os_header, got exit {:?}",
        p.code
    );
}

#[test]
fn err28_regexec_null_pmatch_crash_parity() {
    assert_probe_parity("err28/n2", "regexec_null_pmatch");
    assert_probe_parity("err28/n1", "regexec_null_pmatch_n1");
}

// ===========================================================================
// Row 29 — oversized nmatch
// ===========================================================================

#[test]
fn err29_oversized_nmatch() {
    let big = [8usize, 16, 32, 64, 128, 1024];
    for &n in big.iter() {
        for p in [PAT_MAJOR, PAT_MINOR, PAT_BUILD, &br"(a)(b)(c)"[..]] {
            for s in [&b"1.2.3.4"[..], b"abc", b"", b"abc123"] {
                assert_regexec_eq(&format!("err29/n{n}"), Some(p), Some(s), n, &pm_init(n));
            }
        }
    }
    // usize::MAX in an isolated child (glibc clamps, but assert parity anyway).
    assert_probe_parity("err29/max", "regexec_nmatch_max");
}

#[test]
fn err29b_oversized_nmatch_stdout_parity() {
    // Compare the *stdout payload* of the SIZE_MAX probe between C and Rust.
    let exe = std::env::current_exe().unwrap();
    let run = |side: &str| -> Vec<u8> {
        let out = Command::new(&exe)
            .args(["--exact", "probe_child", "--nocapture", "--test-threads=1"])
            .env("DIFF_PROBE", "regexec_nmatch_max")
            .env("DIFF_SIDE", side)
            .output()
            .unwrap();
        // libtest prefixes the line with "test probe_child ... "; keep only the
        // payload starting at "ret ".
        out.stdout
            .split(|&b| b == b'\n')
            .filter_map(|l| {
                l.windows(4)
                    .position(|w| w == b"ret ")
                    .map(|p| l[p..].to_vec())
            })
            .collect::<Vec<_>>()
            .concat()
    };
    let a = run("c");
    let b = run("rust");
    assert!(!a.is_empty(), "err29b: probe produced no output");
    assert_eq!(
        a,
        b,
        "err29b: SIZE_MAX nmatch diverged\n C   = {:?}\n Rust= {:?}",
        String::from_utf8_lossy(&a),
        String::from_utf8_lossy(&b)
    );
}

// ===========================================================================
// Row 30 — both separators present: " [Ver: " wins
// ===========================================================================

#[test]
fn err30_ver_marker_takes_precedence() {
    let (c, r) = both();
    let sc = Sentinels::new();
    let sr = Sentinels::new();
    let cases: &[&[u8]] = &[
        b"N x86_64 [Ver: 10.0.1] [ubuntu: 20.04 (focal)]",
        b"N [ubuntu: 20.04 (focal)] [Ver: 10.0.1]",
        b"N x86_64 [a: b] [Ver: 1.2.3]",
    ];
    for (i, u) in cases.iter().enumerate() {
        let a = c.parse(u);
        let b = r.parse(u);
        assert_eq!(
            a.snapshot.fields[5],
            Some(b"windows".to_vec()),
            "err30/#{i}: expected the Windows branch"
        );
        assert_eq!(a.snapshot.fields[8], None, "err30/#{i}: os_arch set");
        assert_eq!(a.snapshot.fields[4], None, "err30/#{i}: os_codename set");
        assert_eq!(a, b, "err30/#{i}: diverged");
        let sa = c.parse_sent(u, &sc);
        let sb = r.parse_sent(u, &sr);
        assert_eq!(sa, sb, "err30/#{i}: sentinel run diverged");
    }
}

// ===========================================================================
// Generic boundary sweep (required even though not in ERRORS.md)
// ===========================================================================

#[test]
fn generic_boundaries_sweep() {
    let (c, r) = both();

    // Zero-length / one-past-range inputs for every entry point.
    assert_arch_eq("generic/arch-empty", b"");
    assert_regexec_eq("generic/empty-pat", Some(b""), Some(b""), 2, &pm_init(2));
    assert_regexec_eq("generic/empty-pat2", Some(b""), Some(b"x"), 2, &pm_init(2));
    assert_parse_both_inits("generic/parse-empty", b"");

    // Embedded NUL handling: everything after the first NUL is invisible to C.
    let with_nul: &[u8] = b"L [n: 1.2]\0trailing [Ver: 9.9.9]";
    let a = c.parse(with_nul);
    let b = r.parse(with_nul);
    assert_eq!(a, b, "generic: embedded-NUL input diverged");

    // Every single byte value as a 1-char uname.
    for byte in 1u8..=255 {
        assert_parse_eq(&format!("generic/byte{byte}"), &[byte]);
        assert_arch_eq(&format!("generic/arch-byte{byte}"), &[byte]);
    }

    // Separators split across the very start / very end of the buffer.
    let edge: &[&[u8]] = &[
        b" ", b" [", b" [V", b" [Ve", b" [Ver", b" [Ver:", b" [Ver: ", b"[Ver: ", b"Ver: ", b": ",
        b" (", b"|", b"]", b" []", b" [Ver: ]", b" [: ]", b" [ (]",
    ];
    for (i, u) in edge.iter().enumerate() {
        assert_parse_both_inits(&format!("generic/edge{i}"), u);
    }

    // `nmatch` one step past the group count for each internal pattern.
    for (pi, (p, nsub)) in [(PAT_MAJOR, 1usize), (PAT_MINOR, 1), (PAT_BUILD, 2)]
        .iter()
        .enumerate()
    {
        for n in 0..=(nsub + 2) {
            assert_regexec_eq(
                &format!("generic/pat{pi}/n{n}"),
                Some(p),
                Some(b"1.2.3.4"),
                n,
                &pm_init(nsub + 3),
            );
        }
    }

    // A pointer that is non-NULL but points at an immediately-terminated
    // string (the smallest legal input) for the raw entry points.
    unsafe {
        let empty = c"";
        let pa = c.get_arch_raw(empty.as_ptr() as *mut c_char);
        let pb = r.get_arch_raw(empty.as_ptr() as *mut c_char);
        assert_eq!(pa.is_null(), pb.is_null(), "generic: get_os_arch(\"\")");
        assert!(pa.is_null());
    }
}

/// Same grammar-driven generator as Phase B (kept local to this test binary).
fn fuzz_like(rng: &mut Rng) -> Vec<u8> {
    let mut u = Vec::new();
    for _ in 0..rng.range(0, 6) {
        match rng.below(14) {
            0 => u.extend_from_slice(b" [Ver: "),
            1 => u.extend_from_slice(b" ["),
            2 => u.extend_from_slice(b": "),
            3 => u.extend_from_slice(b" ("),
            4 => u.push(b'|'),
            5 => u.push(b']'),
            6 => u.push(b')'),
            7 => u.extend_from_slice(rng.pick(&ARCHS)),
            8 => u.extend_from_slice(&rng.digits(1, 5)),
            9 => {
                let d1 = rng.digits(1, 3);
                let d2 = rng.digits(1, 3);
                u.extend_from_slice(&d1);
                u.push(b'.');
                u.extend_from_slice(&d2);
            }
            10 => {
                for k in 0..rng.range(1, 5) {
                    if k > 0 {
                        u.push(b'.');
                    }
                    u.extend_from_slice(&rng.digits(1, 3));
                }
            }
            11 => u.push(b'.'),
            12 => u.push(b' '),
            _ => u.extend_from_slice(&rng.safe_word(0, 10)),
        }
    }
    u
}
