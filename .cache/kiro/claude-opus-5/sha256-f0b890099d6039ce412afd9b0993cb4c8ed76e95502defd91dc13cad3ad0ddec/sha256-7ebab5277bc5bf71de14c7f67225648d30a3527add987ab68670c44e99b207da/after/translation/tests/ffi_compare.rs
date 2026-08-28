//! Bottom-up differential tests: every function is invoked through the
//! exported symbols of both shared objects and the results compared.

mod common;

use common::*;
use std::ffi::{CString, c_char, c_int, c_uint};

// ===========================================================================
// Level 0: parse_env_numeric
// ===========================================================================

#[test]
fn parse_env_numeric_matches() {
    let _g = guard();
    let (c_fn, rs_fn) = pair::<FnParseEnvNumeric>("parse_env_numeric");

    // (env value, default). `None` == variable not present at all.
    let values: [Option<&str>; 24] = [
        None,
        Some(""),
        Some("0"),
        Some("1"),
        Some("-1"),
        Some("42"),
        Some("0100"),          // atoi() is decimal-only: 100
        Some("012"),
        Some("  7"),           // leading whitespace is skipped
        Some("\t-13xyz"),
        Some("+5"),
        Some("2147483647"),
        Some("-2147483648"),
        Some("abc"),
        Some("1,2"),           // comma -> warning, default
        Some(",lead"),
        Some("trail,"),
        Some("1;2"),           // semicolon -> warning, default
        Some(";lead"),
        Some("trail;"),
        Some("3,4;5"),         // comma wins (checked first)
        Some("3;4,5"),         // comma still checked first
        Some("999999999999"),  // out of range for atoi
        Some("0x1F"),
    ];
    let defaults: [c_int; 5] = [0, 0o100, 0o12, -7, c_int::MAX];

    for name in ["PROG_MULTIPLIER", "SOME_OTHER_VAR", "PROG_BASE_OFFSET"] {
        let cname = CString::new(name).unwrap();
        for v in values {
            set_env(&[(name, v)]);
            for d in defaults {
                let cc = capture(|| unsafe { c_fn(cname.as_ptr(), d) });
                let rr = capture(|| unsafe { rs_fn(cname.as_ptr(), d) });
                assert_same(&format!("parse_env_numeric({name}={v:?}, {d})"), &cc, &rr);
            }
        }
    }

    // A name that is definitely absent from the environment.
    let absent = CString::new("__C2RUST_DEFINITELY_ABSENT__").unwrap();
    for d in defaults {
        let cc = capture(|| unsafe { c_fn(absent.as_ptr(), d) });
        let rr = capture(|| unsafe { rs_fn(absent.as_ptr(), d) });
        assert_same(&format!("parse_env_numeric(absent, {d})"), &cc, &rr);
    }
}

// ===========================================================================
// Level 0: init_config_from_env
//
// `struct ConfigFlags` is a single 4-byte bit-field storage unit; it is passed
// as a `*mut c_uint` so the raw bits can be compared.
// ===========================================================================

const VERBOSE_VALUES: [Option<&str>; 6] = [
    None,
    Some(""),
    Some("0"),
    Some("1"),
    Some("x1y"),
    Some("no digits"),
];

#[test]
fn init_config_from_env_matches() {
    let _g = guard();
    let (c_fn, rs_fn) = pair::<FnInitConfig>("init_config_from_env");

    // Pre-fill patterns exercise whether the untouched padding bits of the
    // storage unit are handled identically.
    let seeds: [c_uint; 4] = [0x0000_0000, 0xFFFF_FFFF, 0xDEAD_BEEF, 0x0000_00FF];

    for v in VERBOSE_VALUES {
        for d in VERBOSE_VALUES {
            for o in [None, Some(""), Some("0"), Some("anything")] {
                set_env(&[("PROG_VERBOSE", v), ("PROG_DEBUG", d), ("PROG_OPTIMIZE", o)]);
                for seed in seeds {
                    let mut cb: c_uint = seed;
                    let mut rb: c_uint = seed;
                    let cc = capture(|| unsafe { c_fn(&mut cb) });
                    let rr = capture(|| unsafe { rs_fn(&mut rb) });
                    assert_same(
                        &format!("init_config_from_env(v={v:?} d={d:?} o={o:?} seed={seed:#010x}) io"),
                        &cc,
                        &rr,
                    );
                    assert_eq!(
                        cb, rb,
                        "flag bits mismatch (v={v:?} d={d:?} o={o:?} seed={seed:#010x}): C={cb:#010x} Rs={rb:#010x}"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Level 1: perform_operation
// ===========================================================================

const INT_CASES: [c_int; 17] = [
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    7,
    -7,
    10,
    100,
    -100,
    12345,
    -12345,
    65536,
    c_int::MAX,
    c_int::MIN,
];

#[test]
fn perform_operation_matches() {
    let _g = guard();
    let (c_fn, rs_fn) = pair::<FnPerformOperation>("perform_operation");
    set_env(&[]);

    // Every combination of the eight bit-field bits.
    for bits in 0u32..256 {
        for &v1 in INT_CASES.iter() {
            for &v2 in INT_CASES.iter() {
                let mut cb: c_uint = bits;
                let mut rb: c_uint = bits;
                let cc = capture(|| unsafe { c_fn(v1, v2, &mut cb) });
                let rr = capture(|| unsafe { rs_fn(v1, v2, &mut rb) });
                assert_same(
                    &format!("perform_operation({v1}, {v2}, bits={bits:#04x})"),
                    &cc,
                    &rr,
                );
                assert_eq!(cb, rb, "flags mutated differently (bits={bits:#04x})");
            }
        }
    }
}

// ===========================================================================
// Level 1: apply_bit_operations
// ===========================================================================

#[test]
fn apply_bit_operations_matches() {
    let _g = guard();
    let (c_fn, rs_fn) = pair::<FnApplyBitOps>("apply_bit_operations");
    set_env(&[]);

    let mut values: Vec<c_int> = INT_CASES.to_vec();
    values.extend([
        0x0F,
        0x10,
        -0x10,
        0x4000_0000,
        -0x4000_0000,
        0x7FFF_FFF0,
        c_int::MIN + 1,
    ]);

    for bits in 0u32..256 {
        for &v in values.iter() {
            let mut cb: c_uint = bits;
            let mut rb: c_uint = bits;
            let cc = capture(|| unsafe { c_fn(v, &mut cb) });
            let rr = capture(|| unsafe { rs_fn(v, &mut rb) });
            assert_same(
                &format!("apply_bit_operations({v}, bits={bits:#04x})"),
                &cc,
                &rr,
            );
            assert_eq!(cb, rb, "flags mutated differently (bits={bits:#04x})");
        }
    }
}

// ===========================================================================
// Level 2: envy (the public API)
// ===========================================================================

fn envy_param_sets() -> Vec<[c_int; 4]> {
    let mut v = Vec::new();
    let small: [c_int; 9] = [0, 1, -1, 2, -2, 3, -100, 100, 7];
    for &a in &small {
        for &b in &small {
            v.push([a, b, 0, 0]);
            v.push([a, b, 1, 1]);
            v.push([a, b, -1, -1]);
            v.push([a, 0, b, 0]);
            v.push([0, a, 0, b]);
            v.push([a, b, a, b]);
            v.push([a, b, -b, -a]);
        }
    }
    // Boundary values (overflow / shift edge cases).
    let edge: [c_int; 8] = [
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX,
        c_int::MAX - 1,
        -0x4000_0000,
        0x4000_0000,
        0x7FFF_FFF0,
        -0x7FFF_FFF0,
    ];
    for &a in &edge {
        for &b in &edge {
            v.push([a, b, 0, 0]);
            v.push([a, b, 1, -1]);
            v.push([0, 0, a, b]);
            v.push([a, 0, 0, b]);
            v.push([1, 1, a, b]);
        }
    }
    v
}

fn envy_env_sets() -> Vec<Vec<(&'static str, Option<&'static str>)>> {
    let verbose: [Option<&str>; 4] = [None, Some("0"), Some("1"), Some("yes1")];
    let debug: [Option<&str>; 4] = [None, Some("0"), Some("1"), Some("a1b")];
    let optimize: [Option<&str>; 3] = [None, Some(""), Some("1")];
    let offsets: [Option<&str>; 6] = [
        None,
        Some("0"),
        Some("64"),
        Some("-200"),
        Some("1,2"),
        Some("9;9"),
    ];
    let mults: [Option<&str>; 6] = [
        None,
        Some("0"),
        Some("10"),
        Some("-3"),
        Some("5,5"),
        Some("bad"),
    ];

    let mut out = Vec::new();
    for &v in &verbose {
        for &d in &debug {
            for &o in &optimize {
                out.push(vec![
                    ("PROG_VERBOSE", v),
                    ("PROG_DEBUG", d),
                    ("PROG_OPTIMIZE", o),
                ]);
            }
        }
    }
    for &off in &offsets {
        for &m in &mults {
            for &v in &[None, Some("1")] {
                for &o in &[None, Some("1")] {
                    out.push(vec![
                        ("PROG_BASE_OFFSET", off),
                        ("PROG_MULTIPLIER", m),
                        ("PROG_VERBOSE", v),
                        ("PROG_OPTIMIZE", o),
                    ]);
                }
            }
        }
    }
    out
}

#[test]
fn envy_matches() {
    let _g = guard();
    let (c_fn, rs_fn) = pair::<FnEnvy>("envy");

    let params = envy_param_sets();
    let envs = envy_env_sets();

    // Small param probe against *every* environment combination.
    let probe: [[c_int; 4]; 14] = [
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [-1, -1, -1, -1],
        [3, 5, 7, 11],
        [-3, -5, -7, -11],
        [100, -100, 3, -9],
        [-1000, 0, 0, 0],
        [0, -1000, 0, 0],
        [0, 0, -1000, 0],
        [0, 0, 0, -1000],
        [c_int::MIN, 0, 0, 0],
        [c_int::MAX, c_int::MAX, 0, 0],
        [c_int::MIN, c_int::MIN, 1, 1],
        [7, -7, c_int::MIN, c_int::MAX],
    ];
    for env in &envs {
        set_env(env);
        for p in &probe {
            let cc = capture(|| unsafe { c_fn(p[0], p[1], p[2], p[3]) });
            let rr = capture(|| unsafe { rs_fn(p[0], p[1], p[2], p[3]) });
            assert_same(&format!("envy({:?}) env={:?}", p, env), &cc, &rr);
        }
    }

    // Full param sweep against representative environments (default, verbose,
    // optimize, verbose+debug+optimize, custom offsets).
    let key_envs: [Vec<(&str, Option<&str>)>; 5] = [
        vec![],
        vec![("PROG_VERBOSE", Some("1"))],
        vec![("PROG_OPTIMIZE", Some("1"))],
        vec![
            ("PROG_VERBOSE", Some("1")),
            ("PROG_DEBUG", Some("1")),
            ("PROG_OPTIMIZE", Some("1")),
        ],
        vec![
            ("PROG_BASE_OFFSET", Some("-500")),
            ("PROG_MULTIPLIER", Some("-13")),
        ],
    ];
    for env in &key_envs {
        set_env(env);
        for p in &params {
            let cc = capture(|| unsafe { c_fn(p[0], p[1], p[2], p[3]) });
            let rr = capture(|| unsafe { rs_fn(p[0], p[1], p[2], p[3]) });
            assert_same(&format!("envy({:?}) env={:?}", p, env), &cc, &rr);
        }
    }
}

/// Extra pass with verbose+debug enabled together, which is the only path that
/// prints on every branch (including the negative-result restore path).
#[test]
fn envy_verbose_debug_output_matches() {
    let _g = guard();
    let (c_fn, rs_fn) = pair::<FnEnvy>("envy");

    for opt in [None, Some("1")] {
        for off in [None, Some("-1000000"), Some("100000")] {
            set_env(&[
                ("PROG_VERBOSE", Some("1")),
                ("PROG_DEBUG", Some("1")),
                ("PROG_OPTIMIZE", opt),
                ("PROG_BASE_OFFSET", off),
                ("PROG_MULTIPLIER", Some("-9")),
            ]);
            for p in [
                [0, 0, 0, 0],
                [-1000, -1000, 0, 0],
                [5, 5, 5, 5],
                [-2000000, 1, 1, 1],
                [c_int::MIN, c_int::MIN, c_int::MIN, c_int::MIN],
                [c_int::MAX, c_int::MAX, c_int::MAX, c_int::MAX],
                [123456789, -987654321, 55, -77],
            ] {
                let cc = capture(|| unsafe { c_fn(p[0], p[1], p[2], p[3]) });
                let rr = capture(|| unsafe { rs_fn(p[0], p[1], p[2], p[3]) });
                assert_same(
                    &format!("envy verbose/debug({p:?}) opt={opt:?} off={off:?}"),
                    &cc,
                    &rr,
                );
                assert!(
                    !cc.stdout.is_empty(),
                    "expected verbose output for {p:?}"
                );
            }
        }
    }
}

/// Sanity check that the harness really is talking to two distinct libraries
/// and that both export the whole public surface.
#[test]
fn both_libraries_export_all_symbols() {
    let _g = guard();
    let l = libs();
    for name in [
        "envy",
        "parse_env_numeric",
        "init_config_from_env",
        "perform_operation",
        "apply_bit_operations",
    ] {
        let n = CString::new(name).unwrap();
        unsafe {
            l.c.get::<*const ()>(n.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("C .so missing {name}: {e}"));
            l.rs.get::<*const ()>(n.as_bytes_with_nul())
                .unwrap_or_else(|e| panic!("Rust .so missing {name}: {e}"));
        }
    }
    let _ = 0 as c_char;
}

/// Guards against a vacuous suite: every distinguishing diagnostic the C code
/// can emit must actually be observed (identically from both libraries) at
/// least once, which proves each branch was reached.
#[test]
fn all_c_branches_are_exercised() {
    let _g = guard();
    let (c_envy, rs_envy) = pair::<FnEnvy>("envy");
    let (c_parse, rs_parse) = pair::<FnParseEnvNumeric>("parse_env_numeric");

    let mut c_seen = String::new();
    let mut rs_seen = String::new();

    // envy: verbose / debug / restore-from-backup paths.
    for (off, mult) in [
        (Some("100"), Some("10")),
        (Some("-2000000000"), Some("-13")),
        (Some("0"), Some("0")),
    ] {
        for opt in [None, Some("1")] {
            set_env(&[
                ("PROG_VERBOSE", Some("1")),
                ("PROG_DEBUG", Some("1")),
                ("PROG_OPTIMIZE", opt),
                ("PROG_BASE_OFFSET", off),
                ("PROG_MULTIPLIER", mult),
            ]);
            for p in [
                [0, 0, 0, 0],
                [5, 5, 5, 5],
                [-2000000000, -2000000000, 0, 0],
                [-77, -88, -99, -111],
            ] {
                let cc = capture(|| unsafe { c_envy(p[0], p[1], p[2], p[3]) });
                let rr = capture(|| unsafe { rs_envy(p[0], p[1], p[2], p[3]) });
                assert_same(&format!("branch probe envy({p:?})"), &cc, &rr);
                c_seen.push_str(&cc.out_str());
                rs_seen.push_str(&rr.out_str());
            }
        }
    }

    // parse_env_numeric: both warning paths.
    let name = CString::new("PROG_MULTIPLIER").unwrap();
    for v in ["1,2", "1;2"] {
        set_env(&[("PROG_MULTIPLIER", Some(v))]);
        let cc = capture(|| unsafe { c_parse(name.as_ptr(), 0) });
        let rr = capture(|| unsafe { rs_parse(name.as_ptr(), 0) });
        assert_same(&format!("branch probe parse({v})"), &cc, &rr);
        c_seen.push_str(&cc.err_str());
        rs_seen.push_str(&rr.err_str());
    }

    for needle in [
        "Verbose mode enabled",
        "Base offset:",
        "Multiplier:",
        "Debug: Created state backup using memcpy",
        "Debug: Backup base_value =",
        "Debug: operation_mode = 755 (octal)",
        "Debug: result before adjustment =",
        "Found colon at position: 6",
        "Debug: Result string format validated",
        "Restored state from backup",
        "Final result:",
        "Configuration - Debug:",
        "Warning: Invalid character in PROG_MULTIPLIER",
        "Warning: Semicolon found in PROG_MULTIPLIER",
    ] {
        assert!(
            c_seen.contains(needle),
            "C branch never exercised: {needle:?}"
        );
        assert!(
            rs_seen.contains(needle),
            "Rust branch never exercised: {needle:?}"
        );
    }
}
