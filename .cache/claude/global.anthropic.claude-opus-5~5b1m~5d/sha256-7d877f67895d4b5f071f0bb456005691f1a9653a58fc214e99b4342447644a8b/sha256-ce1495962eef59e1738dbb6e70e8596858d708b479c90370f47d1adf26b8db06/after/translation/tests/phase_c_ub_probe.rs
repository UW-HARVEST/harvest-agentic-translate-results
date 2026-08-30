#![allow(non_snake_case)]
//! ERRORS.md rows 27 and 29 — the two rejection paths that are genuine
//! UNDEFINED BEHAVIOUR in the C source and therefore cannot be asserted
//! byte-for-byte:
//!
//!   * row 29: `c2GJK` with an out-of-range `C2_TYPE`. `c2MakeProxy` has no
//!     `default:` arm, so the local `c2Proxy` stays uninitialised and C reads
//!     indeterminate stack (`pA.count` may be any `int`).
//!   * row 27: `cache->count > 3` overflows the local `int saveA[3]`
//!     (lib.c:428) — stack corruption.
//!
//! Both are probed in an ISOLATED SUBPROCESS so that a crash on either side
//! cannot take the rest of the suite down. What we assert is the *class* of
//! behaviour: neither library may hang, and both must be reachable only by
//! calling `c2GJK` directly (every public entry point filters the enum
//! through a `default:` arm first, which IS asserted byte-for-byte in
//! `phase_c_errors.rs`).

mod common;
use common::*;

const MARK: &str = "UBPROBE_RESULT ";

fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

/// Inner mode: run one probe against one library and print the result.
#[test]
fn ub_inner() {
    let Ok(spec) = std::env::var("UB_PROBE") else {
        // Not the subprocess: nothing to do.
        return;
    };
    let (case, which) = spec.split_once(':').expect("UB_PROBE=case:lib");
    let l = libs();
    let lib = if which == "c" { &l.c } else { &l.r };
    let f = unsafe { *lib.get::<FnGJK>(b"c2GJK").unwrap() };

    let a = Blob::of_capsule(c2Capsule {
        a: v(-1.0, -2.0),
        b: v(3.0, 4.0),
        r: 1.5,
    });
    let b = Blob::of_capsule(c2Capsule {
        a: v(5.0, 6.0),
        b: v(7.0, 8.0),
        r: 2.5,
    });

    match case {
        // row 29 — out-of-range enum on both sides
        "bad_type" => {
            let out = call_gjk(f, &a, 7, None, &b, 9, None, 1, None);
            println!("{MARK}{:08x} {:08x} {:08x} {}", out.dist.to_bits(), out.a.x.to_bits(), out.b.x.to_bits(), out.iters);
        }
        // row 29 — one valid, one invalid
        "bad_type_b" => {
            let out = call_gjk(f, &a, C2_TYPE_CAPSULE, None, &b, 42, None, 1, None);
            println!("{MARK}{:08x} {:08x} {:08x} {}", out.dist.to_bits(), out.a.x.to_bits(), out.b.x.to_bits(), out.iters);
        }
        // row 27 — cache->count == 4 overflows saveA[3]
        "cache_overflow" => {
            let cache = c2GJKCache {
                metric: 1.0,
                count: 4,
                iA: [0, 1, 0],
                // iA[3] aliases iB[0]; iB[3] aliases `div`, whose bit pattern
                // is deliberately the small integer 1 so no wild index is used.
                iB: [1, 0, 1],
                div: f32::from_bits(1),
            };
            let out = call_gjk(f, &a, C2_TYPE_CAPSULE, None, &b, C2_TYPE_CAPSULE, None, 1, Some(cache));
            println!("{MARK}{:08x} {:08x} {:08x} {}", out.dist.to_bits(), out.a.x.to_bits(), out.b.x.to_bits(), out.iters);
        }
        other => panic!("unknown UB_PROBE case {other}"),
    }
}

#[derive(Debug)]
struct Probe {
    ok: bool,
    line: Option<String>,
}

fn run_probe(case: &str, which: &str) -> Probe {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", "ub_inner", "--nocapture"])
        .env("UB_PROBE", format!("{case}:{which}"))
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("spawn subprocess");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let line = text
        .lines()
        .find(|l| l.starts_with(MARK))
        .map(|l| l[MARK.len()..].trim().to_string());
    Probe {
        ok: out.status.success(),
        line,
    }
}

#[test]
fn ub_probe() {
    // The subprocess must not be the recursive inner invocation.
    if std::env::var("UB_PROBE").is_ok() {
        return;
    }
    let mut report = String::new();
    for case in ["bad_type", "bad_type_b", "cache_overflow"] {
        let c = run_probe(case, "c");
        let r = run_probe(case, "rust");
        let agree = c.ok == r.ok && c.line == r.line;
        report.push_str(&format!(
            "  {case:<16} C: ok={} {:?}\n  {:<18} Rust: ok={} {:?}   -> {}\n",
            c.ok,
            c.line,
            "",
            r.ok,
            r.line,
            if agree { "IDENTICAL" } else { "UB divergence (documented in ERRORS.md)" }
        ));
        // The only hard requirement: the probe must TERMINATE on both sides.
        // (`Command::output` would block forever on a hang, so reaching here
        // already proves that.)  A crash is permitted for these two rows
        // because the C behaviour is indeterminate by construction.
        assert!(
            c.ok || c.line.is_none(),
            "{case}: C printed a result but then failed — inconsistent"
        );
    }
    eprintln!("UB probe report (ERRORS.md rows 27, 29):\n{report}");
}

/// The UB rows are UNREACHABLE from every public entry point: `c2Collided`
/// and all the `c2*to*` predicates filter the enum first, and none of them
/// exposes the cache. This is the property that actually matters, and it is
/// asserted byte-for-byte here as well as in `phase_c_errors.rs`.
#[test]
fn ub_rows_unreachable_from_public_api() {
    let (cf, rf) = pair::<FnCollided>("c2Collided");
    let mut g = Rng::new(0x4001);
    for &ta in BAD_TYPES.iter().chain(ALL_TYPES.iter()) {
        for &tb in BAD_TYPES.iter().chain(ALL_TYPES.iter()) {
            for _ in 0..300 {
                let a = Blob::of_capsule(g.capsule());
                let b = Blob::of_capsule(g.capsule());
                let cv = unsafe { cf(a.ptr(), ta, b.ptr(), tb) };
                let rv = unsafe { rf(a.ptr(), ta, b.ptr(), tb) };
                same(&format!("c2Collided ta={ta} tb={tb}"), cv, rv);
                let valid = ALL_TYPES.contains(&ta) && ALL_TYPES.contains(&tb);
                if !valid {
                    assert_eq!(cv, 0, "invalid enum must be rejected with 0");
                }
            }
        }
    }
}
