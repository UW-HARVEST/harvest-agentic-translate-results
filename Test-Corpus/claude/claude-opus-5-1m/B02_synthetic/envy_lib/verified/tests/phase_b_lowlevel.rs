// Phase B — valid-path differential tests for the LOW-LEVEL entry points:
// `parse_env_numeric`, `init_config_from_env`, `perform_operation` and
// `apply_bit_operations`, plus the hand-composed pipeline.
//
// Covers CONFIGS.md rows 1-22 and row 33.  The per-row table printed at the end
// is the check-off evidence.
//
// The whole phase runs inside a single `#[test]` function on purpose: the
// harness redirects the process-wide file descriptors 1 and 2 in order to
// capture what the shared objects print, so nothing else may run (or print)
// concurrently.

mod harness;

use harness::*;
use std::ffi::CString;

#[test]
fn phase_b_lowlevel() {
    let _guard = GLOBAL.lock().unwrap();
    let (c, r) = load_impls();
    println!("C   : {}", c.path.display());
    println!("RUST: {}", r.path.display());

    let mut fails: Vec<String> = Vec::new();
    let mut rows = Rows::new("CONFIGS.md (low-level)");
    let mut n = 0usize;
    let mut rng = Rng::new(SEED);
    clear_prog_env();

    let mut cap = Capture::new("phase-b-low");

    // The capture harness must really see the library's output, for both the C
    // and the Rust shared object, otherwise every stdout/stderr comparison
    // below would pass vacuously.
    for imp in [&c, &r] {
        if let Err(e) = self_check_capture(&mut cap, imp) {
            drop(cap);
            panic!("{e}");
        }
    }
    clear_prog_env();

    macro_rules! row {
        ($num:expr, $name:expr, $body:block) => {{
            let before_f = fails.len();
            let before_n = n;
            $body
            rows.add($num, $name, n - before_n, fails.len() - before_f);
        }};
    }

    let base = CString::new("PROG_BASE_OFFSET").unwrap();
    let mult = CString::new("PROG_MULTIPLIER").unwrap();
    let defaults: [i32; 8] = [0, 1, -1, 64, 10, i32::MAX, i32::MIN, 0x1234_5678];

    // A single `parse_env_numeric` sweep: set `PROG_BASE_OFFSET` (and, for the
    // second name, `PROG_MULTIPLIER`) to `value` and compare over all defaults.
    macro_rules! parse_sweep {
        ($values:expr, $tag:expr) => {{
            for value in $values {
                for (name, cname) in [("PROG_BASE_OFFSET", &base), ("PROG_MULTIPLIER", &mult)] {
                    put_env(name, Some(value));
                    for d in defaults {
                        let label = format!("{} name={name} value={value:?} default={d}", $tag);
                        differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                            call_parse(imp, cname, d)
                        });
                        n += 1;
                    }
                    put_env(name, None);
                }
            }
        }};
    }

    // ------------------------------------------------------------------
    // Row 1 — variable absent.
    // ------------------------------------------------------------------
    row!(1, "parse_env_numeric: variable absent", {
        clear_prog_env();
        for (name, cname) in [
            ("PROG_BASE_OFFSET", &base),
            ("PROG_MULTIPLIER", &mult),
        ] {
            for d in defaults {
                let label = format!("row1 absent name={name} default={d}");
                differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                    call_parse(imp, cname, d)
                });
                n += 1;
            }
        }
        for _ in 0..100 {
            let d = rng.next_i32();
            let label = format!("row1 absent random default={d}");
            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                call_parse(imp, &base, d)
            });
            n += 1;
        }
    });

    // ------------------------------------------------------------------
    // Row 2 — plain non-negative decimal values.
    // ------------------------------------------------------------------
    row!(2, "parse_env_numeric: non-negative decimal", {
        parse_sweep!(["0", "1", "7", "10", "64", "100", "65535", "2147483647"], "row2");
        for _ in 0..300 {
            let v = (rng.next_u32() >> 1).to_string();
            put_env("PROG_BASE_OFFSET", Some(&v));
            let d = rng.next_interesting_i32();
            let label = format!("row2 random value={v:?} default={d}");
            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                call_parse(imp, &base, d)
            });
            n += 1;
        }
        put_env("PROG_BASE_OFFSET", None);
    });

    // ------------------------------------------------------------------
    // Row 3 — negative decimal values.
    // ------------------------------------------------------------------
    row!(3, "parse_env_numeric: negative decimal", {
        parse_sweep!(["-0", "-1", "-7", "-100000", "-2147483648"], "row3");
        for _ in 0..300 {
            let v = format!("-{}", rng.next_u32() >> 1);
            put_env("PROG_BASE_OFFSET", Some(&v));
            let d = rng.next_interesting_i32();
            let label = format!("row3 random value={v:?} default={d}");
            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                call_parse(imp, &base, d)
            });
            n += 1;
        }
        put_env("PROG_BASE_OFFSET", None);
    });

    // ------------------------------------------------------------------
    // Row 4 — whitespace / explicit sign / leading zeros (atoi is decimal!).
    // ------------------------------------------------------------------
    row!(4, "parse_env_numeric: whitespace/sign/leading zeros", {
        parse_sweep!(
            [
                "  42", "\t-9", "+7", "007", "0100", " 0", "   -0", "+0", "  +2147483647",
                "\n5", "\r6", "\u{b}7", "\u{c}8", "  -2147483648"
            ],
            "row4"
        );
    });

    // ------------------------------------------------------------------
    // Row 5 — empty value and empty/absent name.
    // ------------------------------------------------------------------
    row!(5, "parse_env_numeric: empty value / empty name", {
        parse_sweep!([""], "row5");
        let empty = CString::new("").unwrap();
        let unused = CString::new("DIFFTEST_NEVER_SET").unwrap();
        for name in [&empty, &unused] {
            for d in defaults {
                let label = format!("row5 name={name:?} default={d}");
                differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                    call_parse(imp, name, d)
                });
                n += 1;
            }
        }
    });

    // ------------------------------------------------------------------
    // Row 6 — value contains ','.
    // ------------------------------------------------------------------
    row!(6, "parse_env_numeric: value contains ','", {
        parse_sweep!([",", "1,", ",1", "1,2", "a,b", ",,", "1,2,3", "-1,000"], "row6");
        for _ in 0..200 {
            let v = format!("{},{}", rng.next_u32() as i32, rng.next_u32() as i32);
            put_env("PROG_BASE_OFFSET", Some(&v));
            let d = rng.next_interesting_i32();
            let label = format!("row6 random value={v:?} default={d}");
            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                call_parse(imp, &base, d)
            });
            n += 1;
        }
        put_env("PROG_BASE_OFFSET", None);
    });

    // ------------------------------------------------------------------
    // Row 7 — value contains ';' only.
    // ------------------------------------------------------------------
    row!(7, "parse_env_numeric: value contains ';'", {
        parse_sweep!([";", "1;", ";1", "3;4", ";;", "abc;"], "row7");
        for _ in 0..200 {
            let v = format!("{};{}", rng.next_u32() as i32, rng.next_u32() as i32);
            put_env("PROG_MULTIPLIER", Some(&v));
            let d = rng.next_interesting_i32();
            let label = format!("row7 random value={v:?} default={d}");
            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                call_parse(imp, &mult, d)
            });
            n += 1;
        }
        put_env("PROG_MULTIPLIER", None);
    });

    // ------------------------------------------------------------------
    // Row 8 — value contains both ',' and ';' (comma is checked first).
    // ------------------------------------------------------------------
    row!(8, "parse_env_numeric: ',' and ';' together", {
        parse_sweep!([",;", ";,", "1;2,3", "1,2;3", ";1,", ",1;"], "row8");
    });

    // ------------------------------------------------------------------
    // Row 9 — atoi garbage / trailing garbage / overflow.
    // ------------------------------------------------------------------
    row!(9, "parse_env_numeric: garbage and overflow", {
        parse_sweep!(
            [
                "abc",
                "12abc",
                "12 34",
                "0x10",
                "9999999999",
                "-9999999999",
                "99999999999999999999999999999999999999",
                "2147483648",
                "-2147483649",
                "1e5",
                "--1",
                "+-1",
                ".5",
                "NaN"
            ],
            "row9"
        );
    });

    // ------------------------------------------------------------------
    // Rows 10/11 — init_config_from_env over the environment states, with a
    // zeroed destination (row 10) and with garbage in the destination
    // (row 11, padding-bit preservation).
    // ------------------------------------------------------------------
    let debug_states_wide = [None, Some(""), Some("0"), Some("1"), Some("zz1")];
    row!(10, "init_config_from_env: 3x3x2 env states, prefill 0", {
        for v in VERBOSE_STATES_WIDE {
            for d in debug_states_wide {
                for o in OPTIMIZE_STATES_WIDE {
                    put_env("PROG_VERBOSE", v);
                    put_env("PROG_DEBUG", d);
                    put_env("PROG_OPTIMIZE", o);
                    let label = format!("row10 init V={v:?} D={d:?} O={o:?}");
                    differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                        call_init(imp, 0)
                    });
                    n += 1;
                }
            }
        }
        clear_prog_env();
    });

    row!(11, "init_config_from_env: garbage prefill preserved", {
        let prefills: [u32; 6] = [
            0xFFFF_FFFF,
            0xDEAD_BEEF,
            0x0000_00FF,
            0x8000_0080,
            0x5555_5555,
            0xAAAA_AA00,
        ];
        for v in VERBOSE_STATES_WIDE {
            for d in debug_states_wide {
                for o in OPTIMIZE_STATES_WIDE {
                    put_env("PROG_VERBOSE", v);
                    put_env("PROG_DEBUG", d);
                    put_env("PROG_OPTIMIZE", o);
                    for prefill in prefills.iter().copied().chain((0..4).map(|_| rng.next_u32())) {
                        let label = format!(
                            "row11 init V={v:?} D={d:?} O={o:?} prefill=0x{prefill:08x}"
                        );
                        differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                            call_init(imp, prefill)
                        });
                        n += 1;
                    }
                }
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Rows 12-15 — perform_operation: optimize x debug matrix.
    // ------------------------------------------------------------------
    for (rownum, optimize, debug, desc) in [
        (12u32, false, false, "perform_operation: optimize=0 debug=0, log_level 0..7"),
        (13, false, true, "perform_operation: optimize=0 debug=1, log_level 0..7"),
        (14, true, false, "perform_operation: optimize=1 debug=0"),
        (15, true, true, "perform_operation: optimize=1 debug=1"),
    ] {
        row!(rownum, desc, {
            for log_level in 0..8u32 {
                for cache in [false, true] {
                    for verbose in [false, true] {
                        let bits = flags(verbose, debug, optimize, cache, log_level);
                        for v1 in BOUNDARY_I32 {
                            for v2 in BOUNDARY_I32 {
                                let label = format!(
                                    "row{rownum} perform bits=0x{bits:02x} v1={v1} v2={v2}"
                                );
                                differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                                    call_perform(imp, v1, v2, bits)
                                });
                                n += 1;
                            }
                        }
                        for _ in 0..12 {
                            let v1 = rng.next_interesting_i32();
                            let v2 = rng.next_interesting_i32();
                            let label = format!(
                                "row{rownum} perform random bits=0x{bits:02x} v1={v1} v2={v2}"
                            );
                            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                                call_perform(imp, v1, v2, bits)
                            });
                            n += 1;
                        }
                    }
                }
            }
        });
    }

    // ------------------------------------------------------------------
    // Row 16 — all 256 low-byte flag patterns.
    // Row 17 — the same with garbage padding bits.
    // ------------------------------------------------------------------
    row!(16, "perform_operation: all 256 flag byte patterns", {
        for low in 0..256u32 {
            for _ in 0..4 {
                let v1 = rng.next_interesting_i32();
                let v2 = rng.next_interesting_i32();
                let label = format!("row16 perform bits=0x{low:02x} v1={v1} v2={v2}");
                differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                    call_perform(imp, v1, v2, low)
                });
                n += 1;
            }
        }
    });

    row!(17, "perform_operation: garbage padding bits ignored", {
        for low in 0..256u32 {
            for pad in [0xDEAD_BE00u32, 0xFFFF_FF00, 0x1234_5600] {
                let bits = pad | low;
                for _ in 0..2 {
                    let v1 = rng.next_interesting_i32();
                    let v2 = rng.next_interesting_i32();
                    let label = format!("row17 perform bits=0x{bits:08x} v1={v1} v2={v2}");
                    differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                        call_perform(imp, v1, v2, bits)
                    });
                    n += 1;
                }
            }
        }
    });

    // ------------------------------------------------------------------
    // Rows 18-21 — apply_bit_operations: verbose x cache_enabled matrix.
    // ------------------------------------------------------------------
    let shift_edges: [i32; 9] = [
        0x3FFF_FFFF,
        0x4000_0000,
        0x4000_0001,
        -0x4000_0000,
        -0x4000_0001,
        0x7FFF_FFFF,
        0x0000_000F,
        -16,
        -15,
    ];
    for (rownum, verbose, cache, desc) in [
        (18u32, false, false, "apply_bit_operations: verbose=0 cache=0"),
        (19, false, true, "apply_bit_operations: verbose=0 cache=1"),
        (20, true, false, "apply_bit_operations: verbose=1 cache=0"),
        (21, true, true, "apply_bit_operations: verbose=1 cache=1"),
    ] {
        row!(rownum, desc, {
            for log_level in [0u32, 3, 7] {
                for debug in [false, true] {
                    let bits = flags(verbose, debug, false, cache, log_level);
                    for value in BOUNDARY_I32.iter().copied().chain(shift_edges) {
                        let label =
                            format!("row{rownum} apply bits=0x{bits:02x} value={value}");
                        differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                            call_apply(imp, value, bits)
                        });
                        n += 1;
                    }
                    for _ in 0..10 {
                        let value = rng.next_interesting_i32();
                        let label =
                            format!("row{rownum} apply random bits=0x{bits:02x} value={value}");
                        differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                            call_apply(imp, value, bits)
                        });
                        n += 1;
                    }
                }
            }
        });
    }

    // ------------------------------------------------------------------
    // Row 22 — all 256 flag byte patterns x garbage padding.
    // ------------------------------------------------------------------
    row!(22, "apply_bit_operations: all 256 patterns + padding", {
        for low in 0..256u32 {
            for pad in [0x0000_0000u32, 0xDEAD_BE00, 0xFFFF_FF00] {
                let bits = pad | low;
                for _ in 0..3 {
                    let value = rng.next_interesting_i32();
                    let label = format!("row22 apply bits=0x{bits:08x} value={value}");
                    differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                        call_apply(imp, value, bits)
                    });
                    n += 1;
                }
            }
        }
    });

    // ------------------------------------------------------------------
    // Row 33 — the pipeline, composed by hand out of the low-level entry
    // points exactly like `envy` composes it, plus cross-implementation
    // chains (C flags -> Rust arithmetic and vice versa).
    // ------------------------------------------------------------------
    row!(33, "pipeline: init -> perform -> apply -> + base_offset", {
        for v in VERBOSE_STATES {
            for d in DEBUG_STATES {
                for o in OPTIMIZE_STATES {
                    for base_v in [None, Some("64"), Some("-1000000"), Some("x,y"), Some("abc")] {
                        for mult_v in [None, Some("10"), Some("-3"), Some("9;9")] {
                            let env = EnvCfg {
                                verbose: v,
                                debug: d,
                                optimize: o,
                                base_offset: base_v,
                                multiplier: mult_v,
                            };
                            apply_env(&env);
                            for _ in 0..3 {
                                let p1 = rng.next_interesting_i32();
                                let p2 = rng.next_interesting_i32();
                                let p3 = rng.next_interesting_i32();
                                let p4 = rng.next_interesting_i32();
                                let label =
                                    format!("row33 pipeline env={env:?} p={p1},{p2},{p3},{p4}");

                                // Pure chain: every step from one implementation.
                                differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                                    pipeline(imp, imp, imp, &base, &mult, p1, p2, p3, p4)
                                });
                                n += 1;

                                // Cross chain A: flags + parse from C, arithmetic
                                // from the implementation under test.
                                let a = format!("{label} [cross: init/parse from C]");
                                differential(&mut fails, &mut cap, &c, &r, &a, |imp| {
                                    pipeline(&c, imp, &c, &base, &mult, p1, p2, p3, p4)
                                });
                                n += 1;

                                // Cross chain B: arithmetic from C, flags + parse
                                // from the implementation under test.
                                let b = format!("{label} [cross: arithmetic from C]");
                                differential(&mut fails, &mut cap, &c, &r, &b, |imp| {
                                    pipeline(imp, &c, imp, &base, &mult, p1, p2, p3, p4)
                                });
                                n += 1;
                            }
                        }
                    }
                }
            }
        }
        clear_prog_env();
    });

    drop(cap);
    rows.print();
    rows.assert_covers(&[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 33,
    ]);
    report(fails, n, "phase B (low-level entry points)");
}

/// Re-implements `envy`'s body out of the exported low-level functions.
/// `flags_from` runs `init_config_from_env`, `arith` runs `perform_operation`
/// and `apply_bit_operations`, `parse_from` runs `parse_env_numeric`.
#[allow(clippy::too_many_arguments)]
fn pipeline(
    flags_from: &Impl,
    arith: &Impl,
    parse_from: &Impl,
    base_name: &CString,
    mult_name: &CString,
    p1: i32,
    p2: i32,
    p3: i32,
    p4: i32,
) -> i64 {
    let mut bits: u32 = 0;
    unsafe { (flags_from.init_config_from_env)(&mut bits) };
    let base_offset = unsafe { (parse_from.parse_env_numeric)(base_name.as_ptr(), 0o100) };
    let multiplier = unsafe { (parse_from.parse_env_numeric)(mult_name.as_ptr(), 0o12) };

    let mut result = unsafe { (arith.perform_operation)(p1, p2, &mut bits) };
    if p3 != 0 {
        result = result.wrapping_add(p3.wrapping_mul(multiplier));
    }
    if p4 != 0 {
        result = result.wrapping_add(p4 >> 2);
    }
    result = unsafe { (arith.apply_bit_operations)(result, &mut bits) };
    result = result.wrapping_add(base_offset);
    if result < 0 {
        result = p1;
    }
    ((bits as u64) << 32 | (result as u32 as u64)) as i64
}
