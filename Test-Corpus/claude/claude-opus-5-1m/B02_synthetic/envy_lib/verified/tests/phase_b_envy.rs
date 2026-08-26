// Phase B — valid-path differential tests for the top-level entry point `envy`.
//
// Covers CONFIGS.md rows 23-32.  One `#[test]` function only (see the comment
// in phase_b_lowlevel.rs for why).  The per-row table printed at the end is the
// check-off evidence.

mod harness;

use harness::*;

#[test]
fn phase_b_envy() {
    let _guard = GLOBAL.lock().unwrap();
    let (c, r) = load_impls();
    println!("C   : {}", c.path.display());
    println!("RUST: {}", r.path.display());

    let mut fails: Vec<String> = Vec::new();
    let mut rows = Rows::new("CONFIGS.md (envy)");
    let mut n = 0usize;
    let mut rng = Rng::new(SEED ^ 0xA5A5_A5A5);
    clear_prog_env();
    let mut cap = Capture::new("phase-b-envy");

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

    let debug_states_wide = [None, Some(""), Some("0"), Some("1"), Some("a1")];

    // ------------------------------------------------------------------
    // Row 23 — the flag states with both numeric variables at their defaults.
    // ------------------------------------------------------------------
    row!(23, "envy: 3x3x2 flag states, default offsets", {
        for v in VERBOSE_STATES_WIDE {
            for d in debug_states_wide {
                for o in OPTIMIZE_STATES_WIDE {
                    let env = EnvCfg {
                        verbose: v,
                        debug: d,
                        optimize: o,
                        base_offset: None,
                        multiplier: None,
                    };
                    apply_env(&env);
                    for _ in 0..10 {
                        let p1 = rng.next_interesting_i32();
                        let p2 = rng.next_interesting_i32();
                        let p3 = rng.next_interesting_i32();
                        let p4 = rng.next_interesting_i32();
                        let label =
                            format!("row23 envy env={env:?} p=({p1},{p2},{p3},{p4})");
                        differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                            call_envy(imp, p1, p2, p3, p4)
                        });
                        n += 1;
                    }
                }
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Rows 24-26 — the (param3 == 0) x (param4 == 0) branch combinations.
    // ------------------------------------------------------------------
    for (rownum, cases, desc) in [
        (
            24u32,
            vec![(0, 0)],
            "envy: param3 == 0 and param4 == 0 (both terms skipped)",
        ),
        (
            25,
            vec![
                (5, 0),
                (-3, 0),
                (i32::MIN, 0),
                (i32::MAX, 0),
                (0, 7),
                (0, -9),
                (0, i32::MIN),
                (0, i32::MAX),
            ],
            "envy: exactly one of param3/param4 non-zero",
        ),
        (
            26,
            vec![
                (5, 7),
                (-3, -9),
                (1, i32::MIN),
                (i32::MIN, 1),
                (i32::MAX, i32::MAX),
                (i32::MIN, i32::MIN),
                (-1, -1),
            ],
            "envy: param3 != 0 and param4 != 0 (both terms)",
        ),
    ] {
        row!(rownum, desc, {
            for v in VERBOSE_STATES_WIDE {
                for d in debug_states_wide {
                    for o in OPTIMIZE_STATES_WIDE {
                        let env = EnvCfg {
                            verbose: v,
                            debug: d,
                            optimize: o,
                            base_offset: None,
                            multiplier: None,
                        };
                        apply_env(&env);
                        for (p3, p4) in &cases {
                            for p1 in [0, 1, -1, 12345, -12345] {
                                let p2 = 100;
                                let label = format!(
                                    "row{rownum} envy env={env:?} p=({p1},{p2},{p3},{p4})"
                                );
                                let (p3, p4) = (*p3, *p4);
                                differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                                    call_envy(imp, p1, p2, p3, p4)
                                });
                                n += 1;
                            }
                            for _ in 0..2 {
                                let p1 = rng.next_interesting_i32();
                                let p2 = rng.next_interesting_i32();
                                let (p3, p4) = (*p3, *p4);
                                let label = format!(
                                    "row{rownum} envy random env={env:?} p=({p1},{p2},{p3},{p4})"
                                );
                                differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                                    call_envy(imp, p1, p2, p3, p4)
                                });
                                n += 1;
                            }
                        }
                    }
                }
            }
            clear_prog_env();
        });
    }

    // ------------------------------------------------------------------
    // Row 27 — every PROG_BASE_OFFSET shape x the flag states.
    // ------------------------------------------------------------------
    row!(27, "envy: all PROG_BASE_OFFSET shapes x flag states", {
        for shape in NUMERIC_SHAPES {
            for v in VERBOSE_STATES {
                for d in DEBUG_STATES {
                    for o in OPTIMIZE_STATES {
                        let env = EnvCfg {
                            verbose: v,
                            debug: d,
                            optimize: o,
                            base_offset: shape,
                            multiplier: None,
                        };
                        apply_env(&env);
                        for _ in 0..3 {
                            let p1 = rng.next_interesting_i32();
                            let p2 = rng.next_interesting_i32();
                            let p3 = rng.next_interesting_i32();
                            let p4 = rng.next_interesting_i32();
                            let label =
                                format!("row27 envy env={env:?} p=({p1},{p2},{p3},{p4})");
                            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                                call_envy(imp, p1, p2, p3, p4)
                            });
                            n += 1;
                        }
                    }
                }
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 28 — every PROG_MULTIPLIER shape, param3 != 0 so it is used.
    // ------------------------------------------------------------------
    row!(28, "envy: all PROG_MULTIPLIER shapes, param3 != 0", {
        for shape in NUMERIC_SHAPES {
            for v in VERBOSE_STATES {
                for d in DEBUG_STATES {
                    for o in OPTIMIZE_STATES {
                        let env = EnvCfg {
                            verbose: v,
                            debug: d,
                            optimize: o,
                            base_offset: None,
                            multiplier: shape,
                        };
                        apply_env(&env);
                        for _ in 0..3 {
                            let p1 = rng.next_interesting_i32();
                            let p2 = rng.next_interesting_i32();
                            let mut p3 = rng.next_interesting_i32();
                            if p3 == 0 {
                                p3 = 7;
                            }
                            let p4 = rng.next_interesting_i32();
                            let label =
                                format!("row28 envy env={env:?} p=({p1},{p2},{p3},{p4})");
                            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                                call_envy(imp, p1, p2, p3, p4)
                            });
                            n += 1;
                        }
                    }
                }
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 29 — both numeric variables set at the same time (cross product;
    // two stderr warnings in a single call when both are poisoned).
    // ------------------------------------------------------------------
    row!(29, "envy: PROG_BASE_OFFSET x PROG_MULTIPLIER shapes", {
        for base in NUMERIC_SHAPES {
            for mult in NUMERIC_SHAPES {
                let env = EnvCfg {
                    verbose: Some("1"),
                    debug: Some("1"),
                    optimize: None,
                    base_offset: base,
                    multiplier: mult,
                };
                apply_env(&env);
                for _ in 0..2 {
                    let p1 = rng.next_interesting_i32();
                    let p2 = rng.next_interesting_i32();
                    let p3 = rng.next_interesting_i32();
                    let p4 = rng.next_interesting_i32();
                    let label = format!("row29 envy env={env:?} p=({p1},{p2},{p3},{p4})");
                    differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                        call_envy(imp, p1, p2, p3, p4)
                    });
                    n += 1;
                }
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 30 — configurations that force the `result < 0` restore path.
    // ------------------------------------------------------------------
    row!(30, "envy: negative result -> backup restore path", {
        for base in [Some("-2147483648"), Some("-1000000"), Some("-1"), None] {
            for v in VERBOSE_STATES {
                for o in OPTIMIZE_STATES {
                    let env = EnvCfg {
                        verbose: v,
                        debug: Some("1"),
                        optimize: o,
                        base_offset: base,
                        multiplier: Some("1000000"),
                    };
                    apply_env(&env);
                    for p1 in [0, 1, -1, i32::MIN, i32::MAX, -99999] {
                        for (p2, p3, p4) in [
                            (-1000, -1000, -1000),
                            (i32::MIN, 0, 0),
                            (0, i32::MIN, 0),
                            (0, 0, i32::MIN),
                            (-7, 3, -5),
                        ] {
                            let label =
                                format!("row30 envy env={env:?} p=({p1},{p2},{p3},{p4})");
                            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                                call_envy(imp, p1, p2, p3, p4)
                            });
                            n += 1;
                        }
                    }
                }
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 31 — boundary parameter cross-product (7^4) x optimize on/off.
    // ------------------------------------------------------------------
    row!(31, "envy: 7^4 boundary parameter cross-product", {
        let edges: [i32; 7] = [0, 1, -1, i32::MAX, i32::MIN, 0x4000_0000, -0x4000_0000];
        for o in OPTIMIZE_STATES {
            let env = EnvCfg {
                verbose: None,
                debug: None,
                optimize: o,
                base_offset: None,
                multiplier: None,
            };
            apply_env(&env);
            for p1 in edges {
                for p2 in edges {
                    for p3 in edges {
                        for p4 in edges {
                            let label =
                                format!("row31 envy env={env:?} p=({p1},{p2},{p3},{p4})");
                            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                                call_envy(imp, p1, p2, p3, p4)
                            });
                            n += 1;
                        }
                    }
                }
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 32 — full random fuzz over all five variables and all four params.
    // ------------------------------------------------------------------
    row!(32, "envy: randomised fuzz over all five variables", {
        let random_values: [Option<&'static str>; 12] = [
            None,
            Some(""),
            Some("0"),
            Some("1"),
            Some("2"),
            Some("-1"),
            Some("64"),
            Some("abc"),
            Some("1,000"),
            Some("5;5"),
            Some("2147483647"),
            Some("-2147483648"),
        ];
        for _ in 0..4000 {
            let env = EnvCfg {
                verbose: *rng.choice(&random_values),
                debug: *rng.choice(&random_values),
                optimize: *rng.choice(&random_values),
                base_offset: *rng.choice(&random_values),
                multiplier: *rng.choice(&random_values),
            };
            apply_env(&env);
            let p1 = rng.next_interesting_i32();
            let p2 = rng.next_interesting_i32();
            let p3 = rng.next_interesting_i32();
            let p4 = rng.next_interesting_i32();
            let label = format!("row32 fuzz env={env:?} p=({p1},{p2},{p3},{p4})");
            differential(&mut fails, &mut cap, &c, &r, &label, |imp| {
                call_envy(imp, p1, p2, p3, p4)
            });
            n += 1;
        }
        clear_prog_env();
    });

    drop(cap);
    rows.print();
    rows.assert_covers(&[23, 24, 25, 26, 27, 28, 29, 30, 31, 32]);
    report(fails, n, "phase B (envy)");
}
