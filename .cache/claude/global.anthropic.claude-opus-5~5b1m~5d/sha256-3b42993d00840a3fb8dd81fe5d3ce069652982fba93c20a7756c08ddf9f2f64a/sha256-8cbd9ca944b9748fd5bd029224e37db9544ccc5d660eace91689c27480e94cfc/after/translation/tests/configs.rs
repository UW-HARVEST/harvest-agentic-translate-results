// Phase B — valid-path differential tests.
//
// One #[test] per row of CONFIGS.md (C1..C50). Every call goes through the
// exported C symbols of BOTH the C .so and the Rust .so, loaded with
// libloading; return values, stdout bytes, stderr bytes and (where a struct is
// passed) the callee-visible struct bytes are compared.
//
// Every row uses many randomized inputs from a fixed-seed SplitMix64 PRNG.

mod common;
use common::*;

// ===========================================================================
// parse_env_numeric  (C1..C10)
// ===========================================================================

#[test]
fn c1_parse_env_unset_returns_default() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC001);
    // Randomized defaults ...
    for i in 0..64 {
        let d = rng.interesting_i32();
        diff_parse_env(&format!("C1[{i}]"), "PROG_BASE_OFFSET", d);
        diff_parse_env(&format!("C1[{i}]"), "PROG_MULTIPLIER", d);
    }
    // ... plus the extremes.
    for d in BOUNDARY_I32 {
        diff_parse_env("C1/bounds", "PROG_BASE_OFFSET", d);
    }
}

#[test]
fn c2_parse_env_plain_decimal() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC002);
    for i in 0..64 {
        let v = rng.next_i32();
        let s = v.to_string();
        env_set("PROG_BASE_OFFSET", &s);
        diff_parse_env(&format!("C2[{i}] v={s}"), "PROG_BASE_OFFSET", 999);
    }
    for v in BOUNDARY_I32 {
        let s = v.to_string();
        env_set("PROG_BASE_OFFSET", &s);
        diff_parse_env(&format!("C2/bounds v={s}"), "PROG_BASE_OFFSET", 999);
    }
    env_clear_all();
}

#[test]
fn c3_parse_env_empty_value() {
    let _g = lock();
    env_clear_all();
    for name in ENV_NAMES {
        env_set(name, "");
        for d in BOUNDARY_I32 {
            diff_parse_env(&format!("C3 {name}"), name, d);
        }
    }
    env_clear_all();
}

#[test]
fn c4_parse_env_comma_rejected() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC004);
    // Comma at the start, in the middle and at the end of random digit strings.
    for i in 0..64 {
        let digits: String = (0..(1 + rng.below(9)))
            .map(|_| (b'0' + (rng.below(10) as u8)) as char)
            .collect();
        let pos = rng.below(digits.len() as u64 + 1) as usize;
        let mut v = digits.clone();
        v.insert(pos, ',');
        env_set("PROG_BASE_OFFSET", &v);
        diff_parse_env(&format!("C4[{i}] v={v}"), "PROG_BASE_OFFSET", 64);
    }
    for v in [",", ",1", "1,", "1,2", "12,34,56", ",,,"] {
        env_set("PROG_MULTIPLIER", v);
        diff_parse_env(&format!("C4 v={v}"), "PROG_MULTIPLIER", 10);
    }
    env_clear_all();
}

#[test]
fn c5_parse_env_semicolon_rejected() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC005);
    for i in 0..64 {
        let digits: String = (0..(1 + rng.below(9)))
            .map(|_| (b'0' + (rng.below(10) as u8)) as char)
            .collect();
        let pos = rng.below(digits.len() as u64 + 1) as usize;
        let mut v = digits.clone();
        v.insert(pos, ';');
        env_set("PROG_BASE_OFFSET", &v);
        diff_parse_env(&format!("C5[{i}] v={v}"), "PROG_BASE_OFFSET", 64);
    }
    for v in [";", ";1", "1;", "1;2", ";;;"] {
        env_set("PROG_MULTIPLIER", v);
        diff_parse_env(&format!("C5 v={v}"), "PROG_MULTIPLIER", 10);
    }
    env_clear_all();
}

#[test]
fn c6_parse_env_comma_and_semicolon_comma_wins() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC006);
    for i in 0..48 {
        let a = rng.below(1000);
        let b = rng.below(1000);
        for v in [format!("{a},{b};7"), format!("{a};{b},7")] {
            env_set("PROG_BASE_OFFSET", &v);
            diff_parse_env(&format!("C6[{i}] v={v}"), "PROG_BASE_OFFSET", 64);
        }
    }
    for v in [",;", ";,", "1,2;3", "1;2,3"] {
        env_set("PROG_BASE_OFFSET", v);
        diff_parse_env(&format!("C6 v={v}"), "PROG_BASE_OFFSET", 64);
    }
    env_clear_all();
}

#[test]
fn c7_parse_env_atoi_shapes() {
    let _g = lock();
    env_clear_all();
    let shapes = [
        " 42", "  -42", "\t7", "\n7", "+7", "-0", "0", "007", "12abc", "abc",
        "abc12", "0x1f", "0X1F", "1e3", "1.9", "-1.9", ".5", "--3", "+-3", "-",
        "+", " ", "   ", "2147483647", "-2147483648", "1 2", "1|2", "1&2",
        "1\n2", "١٢٣", "0b101", "  +0009xyz",
    ];
    let mut rng = Rng::new(0xC007);
    for s in shapes {
        env_set("PROG_BASE_OFFSET", s);
        let d = rng.interesting_i32();
        diff_parse_env(&format!("C7 v={s:?}"), "PROG_BASE_OFFSET", d);
        diff_parse_env(&format!("C7 v={s:?} d=0"), "PROG_BASE_OFFSET", 0);
    }
    env_clear_all();
}

#[test]
fn c8_parse_env_atoi_overflow() {
    let _g = lock();
    env_clear_all();
    let shapes = [
        "2147483648",
        "-2147483649",
        "4294967296",
        "99999999999999999999",
        "-99999999999999999999",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "184467440737095516160",
        "000000000002147483648",
    ];
    for s in shapes {
        env_set("PROG_MULTIPLIER", s);
        diff_parse_env(&format!("C8 v={s}"), "PROG_MULTIPLIER", -1);
    }
    env_clear_all();
}

#[test]
fn c9_parse_env_very_long_value() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC009);
    for len in [255usize, 256, 257, 300, 1024] {
        // digits only
        let v: String = (0..len)
            .map(|_| (b'0' + (rng.below(10) as u8)) as char)
            .collect();
        env_set("PROG_BASE_OFFSET", &v);
        diff_parse_env(&format!("C9 len={len} digits"), "PROG_BASE_OFFSET", 64);
        // same but with a comma buried near the end (rejection path on a long value)
        let mut v2 = v.clone();
        v2.insert(len - 1, ',');
        env_set("PROG_BASE_OFFSET", &v2);
        diff_parse_env(&format!("C9 len={len} comma"), "PROG_BASE_OFFSET", 64);
        // and with a semicolon
        let mut v3 = v.clone();
        v3.insert(len / 2, ';');
        env_set("PROG_BASE_OFFSET", &v3);
        diff_parse_env(&format!("C9 len={len} semi"), "PROG_BASE_OFFSET", 64);
    }
    env_clear_all();
}

#[test]
fn c10_parse_env_odd_names() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC010);
    for name in ["", "PROG_DOES_NOT_EXIST_XYZ", "=", "PATH", "HOME", "SHLVL"] {
        for _ in 0..8 {
            let d = rng.interesting_i32();
            diff_parse_env(&format!("C10 name={name:?}"), name, d);
        }
    }
    // A name that IS set, containing unusual characters in its value.
    env_set("PROG_OPTIMIZE", "wat");
    diff_parse_env("C10 PROG_OPTIMIZE", "PROG_OPTIMIZE", 5);
    env_clear_all();
}

// ===========================================================================
// init_config_from_env  (C11..C14)
// ===========================================================================

/// The three states of PROG_VERBOSE / PROG_DEBUG the C distinguishes.
const FLAG_ENV_STATES: [Option<&str>; 3] = [None, Some("1"), Some("nope")];
/// The three states of PROG_OPTIMIZE the C distinguishes.
const OPT_ENV_STATES: [Option<&str>; 3] = [None, Some(""), Some("yes")];

fn for_each_27_env_combo(mut f: impl FnMut(usize, Option<&str>, Option<&str>, Option<&str>)) {
    let mut n = 0;
    for v in FLAG_ENV_STATES {
        for d in FLAG_ENV_STATES {
            for o in OPT_ENV_STATES {
                env_apply(&[
                    ("PROG_VERBOSE", v),
                    ("PROG_DEBUG", d),
                    ("PROG_OPTIMIZE", o),
                ]);
                f(n, v, d, o);
                n += 1;
            }
        }
    }
    assert_eq!(n, 27, "the 3x3x3 option cross-product must be complete");
    env_clear_all();
}

#[test]
fn c11_init_config_27_combos_zeroed() {
    let _g = lock();
    env_clear_all();
    for_each_27_env_combo(|n, v, d, o| {
        diff_init_config(&format!("C11[{n}] V={v:?} D={d:?} O={o:?}"), Flags([0; 4]));
    });
}

#[test]
fn c12_init_config_27_combos_dirty_ff() {
    let _g = lock();
    env_clear_all();
    for_each_27_env_combo(|n, v, d, o| {
        diff_init_config(
            &format!("C12[{n}] V={v:?} D={d:?} O={o:?}"),
            Flags([0xFF; 4]),
        );
    });
}

#[test]
fn c13_init_config_27_combos_random_initial() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC013);
    for_each_27_env_combo(|n, v, d, o| {
        for k in 0..8 {
            let r = rng.next_u32().to_le_bytes();
            diff_init_config(
                &format!("C13[{n}/{k}] V={v:?} D={d:?} O={o:?} init={r:?}"),
                Flags(r),
            );
        }
    });
    // Also every one of the 256 possible byte-0 starting values.
    env_apply(&[("PROG_VERBOSE", None), ("PROG_DEBUG", None), ("PROG_OPTIMIZE", None)]);
    for b in 0u16..256 {
        diff_init_config(&format!("C13/byte0={b}"), Flags([b as u8, 0xAA, 0x55, 0x0F]));
    }
    env_clear_all();
}

#[test]
fn c14_init_config_literal_one_detection() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC014);
    // Strings that DO contain '1' at a random position.
    for i in 0..48 {
        let n = 1 + rng.below(10) as usize;
        let mut s: Vec<char> = (0..n)
            .map(|_| {
                let alphabet = b"023456789abcxyzTRUEfalse_-+";
                alphabet[rng.below(alphabet.len() as u64) as usize] as char
            })
            .collect();
        let with_one = {
            let mut t = s.clone();
            t.insert(rng.below(n as u64 + 1) as usize, '1');
            t.into_iter().collect::<String>()
        };
        let without_one: String = s.drain(..).collect();
        for (vv, dd) in [
            (with_one.as_str(), without_one.as_str()),
            (without_one.as_str(), with_one.as_str()),
            (with_one.as_str(), with_one.as_str()),
            (without_one.as_str(), without_one.as_str()),
        ] {
            env_apply(&[
                ("PROG_VERBOSE", Some(vv)),
                ("PROG_DEBUG", Some(dd)),
                ("PROG_OPTIMIZE", None),
            ]);
            diff_init_config(&format!("C14[{i}] V={vv:?} D={dd:?}"), Flags([0; 4]));
            diff_init_config(&format!("C14[{i}]/ff V={vv:?} D={dd:?}"), Flags([0xFF; 4]));
        }
    }
    // Classic falsy-looking strings that nonetheless matter.
    for s in ["0", "true", "TRUE", "yes", "on", "off", "false", "", "10", "01", "-1", "21"] {
        env_apply(&[
            ("PROG_VERBOSE", Some(s)),
            ("PROG_DEBUG", Some(s)),
            ("PROG_OPTIMIZE", Some(s)),
        ]);
        diff_init_config(&format!("C14 all={s:?}"), Flags([0; 4]));
    }
    env_clear_all();
}

// ===========================================================================
// perform_operation  (C15..C21)
// ===========================================================================

#[test]
fn c15_perform_op_optimize_no_debug() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC015);
    let flags = Flags::from_byte0(flag_byte(false, false, true, true, 3));
    for i in 0..256 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff_perform_op(&format!("C15[{i}]"), a, b, flags);
    }
    // deliberate overflow pairs for val1 + val2
    for a in BOUNDARY_I32 {
        for b in BOUNDARY_I32 {
            diff_perform_op("C15/bounds", a, b, flags);
        }
    }
}

#[test]
fn c16_perform_op_optimize_with_debug() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC016);
    let flags = Flags::from_byte0(flag_byte(false, true, true, true, 3));
    for i in 0..128 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff_perform_op(&format!("C16[{i}]"), a, b, flags);
    }
    for a in BOUNDARY_I32 {
        for b in BOUNDARY_I32 {
            diff_perform_op("C16/bounds", a, b, flags);
        }
    }
}

#[test]
fn c17_perform_op_no_optimize_all_log_levels() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC017);
    for ll in 0u8..8 {
        let flags = Flags::from_byte0(flag_byte(false, false, false, true, ll));
        for i in 0..64 {
            let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
            diff_perform_op(&format!("C17 ll={ll} [{i}]"), a, b, flags);
        }
    }
}

#[test]
fn c18_perform_op_no_optimize_debug_all_log_levels() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC018);
    for ll in 0u8..8 {
        let flags = Flags::from_byte0(flag_byte(false, true, false, true, ll));
        for i in 0..48 {
            let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
            diff_perform_op(&format!("C18 ll={ll} [{i}]"), a, b, flags);
        }
        for a in BOUNDARY_I32 {
            diff_perform_op(&format!("C18/bounds ll={ll}"), a, a, flags);
        }
    }
}

#[test]
fn c19_perform_op_all_256_flag_bytes() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC019);
    for b0 in 0u16..256 {
        let flags = Flags::from_byte0(b0 as u8);
        for i in 0..8 {
            let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
            diff_perform_op(&format!("C19 byte0={b0:#04x} [{i}]"), a, b, flags);
        }
        diff_perform_op(&format!("C19 byte0={b0:#04x} min"), i32::MIN, i32::MIN, flags);
        diff_perform_op(&format!("C19 byte0={b0:#04x} max"), i32::MAX, i32::MAX, flags);
    }
}

#[test]
fn c20_perform_op_boundary_matrix() {
    let _g = lock();
    env_clear_all();
    for ll in 0u8..8 {
        let flags = Flags::from_byte0(flag_byte(false, false, false, true, ll));
        for a in BOUNDARY_I32 {
            for b in BOUNDARY_I32 {
                diff_perform_op(&format!("C20 ll={ll}"), a, b, flags);
            }
        }
    }
}

#[test]
fn c21_perform_op_dirty_padding() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC021);
    for b0 in 0u16..256 {
        for pad in [[0xFFu8, 0xFF, 0xFF], [0xAA, 0x55, 0x00], [0x01, 0x00, 0x80]] {
            let flags = Flags([b0 as u8, pad[0], pad[1], pad[2]]);
            let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
            diff_perform_op(&format!("C21 byte0={b0:#04x} pad={pad:?}"), a, b, flags);
        }
    }
}

// ===========================================================================
// apply_bit_operations  (C22..C27)
// ===========================================================================

fn bits_row(name: &str, seed: u64, verbose: bool, cache: bool) {
    let mut rng = Rng::new(seed);
    // log_level / debug / reserved must be irrelevant here — vary them anyway.
    for ll in 0u8..8 {
        for dbg in [false, true] {
            for rsv in [0u8, B_RESERVED] {
                let mut b = flag_byte(verbose, dbg, false, cache, ll);
                b |= rsv;
                let flags = Flags::from_byte0(b);
                for i in 0..16 {
                    let v = rng.interesting_i32();
                    diff_apply_bits(&format!("{name} b={b:#04x} [{i}]"), v, flags);
                }
                for v in BOUNDARY_I32 {
                    diff_apply_bits(&format!("{name} b={b:#04x} bound"), v, flags);
                }
                // values that specifically overflow a << 1
                for v in [
                    0x4000_0000i32,
                    0x4000_0001,
                    0x7FFF_FFFF,
                    -0x4000_0000,
                    -0x4000_0001,
                    i32::MIN,
                    -1,
                    0x8000_0000u32 as i32,
                ] {
                    diff_apply_bits(&format!("{name} b={b:#04x} shift"), v, flags);
                }
            }
        }
    }
}

#[test]
fn c22_apply_bits_no_verbose_no_cache() {
    let _g = lock();
    env_clear_all();
    bits_row("C22", 0xC022, false, false);
}

#[test]
fn c23_apply_bits_no_verbose_cache() {
    let _g = lock();
    env_clear_all();
    bits_row("C23", 0xC023, false, true);
}

#[test]
fn c24_apply_bits_verbose_no_cache() {
    let _g = lock();
    env_clear_all();
    bits_row("C24", 0xC024, true, false);
}

#[test]
fn c25_apply_bits_verbose_cache() {
    let _g = lock();
    env_clear_all();
    bits_row("C25", 0xC025, true, true);
}

#[test]
fn c26_apply_bits_all_256_flag_bytes() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC026);
    for b0 in 0u16..256 {
        let flags = Flags::from_byte0(b0 as u8);
        for i in 0..8 {
            let v = rng.interesting_i32();
            diff_apply_bits(&format!("C26 byte0={b0:#04x} [{i}]"), v, flags);
        }
        for v in [i32::MIN, i32::MAX, -1, 0, 1, 0x4000_0000] {
            diff_apply_bits(&format!("C26 byte0={b0:#04x} bound"), v, flags);
        }
    }
}

#[test]
fn c27_apply_bits_dirty_padding() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC027);
    for b0 in 0u16..256 {
        for pad in [[0xFFu8, 0xFF, 0xFF], [0xAA, 0x55, 0x00]] {
            let flags = Flags([b0 as u8, pad[0], pad[1], pad[2]]);
            let v = rng.interesting_i32();
            diff_apply_bits(&format!("C27 byte0={b0:#04x} pad={pad:?}"), v, flags);
        }
    }
}

// ===========================================================================
// envy  (C28..C47)
// ===========================================================================

fn envy_sweep(name: &str, seed: u64, n: usize) {
    let mut rng = Rng::new(seed);
    for i in 0..n {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        diff_envy(&format!("{name}[{i}]"), p[0], p[1], p[2], p[3]);
    }
    // A fixed set of structurally interesting tuples for every env config.
    for t in [
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [-1, -1, -1, -1],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MIN, i32::MAX, i32::MIN, i32::MAX],
        [0, -160, 0, 0],
        [0, -128, 0, 0],
        [7, 0, 0, -3],
        [0, 0, 0, -1],
        [-5, 0, 0, 0],
    ] {
        diff_envy(&format!("{name}/fixed"), t[0], t[1], t[2], t[3]);
    }
}

#[test]
fn c28_envy_no_env_all_defaults() {
    let _g = lock();
    env_clear_all();
    envy_sweep("C28", 0xC028, 200);
}

#[test]
fn c29_envy_optimize_set() {
    let _g = lock();
    env_clear_all();
    for o in ["", "1", "0", "anything"] {
        env_apply(&[("PROG_OPTIMIZE", Some(o))]);
        envy_sweep(&format!("C29 O={o:?}"), 0xC029, 64);
    }
    env_clear_all();
}

#[test]
fn c30_envy_verbose_set() {
    let _g = lock();
    env_clear_all();
    env_apply(&[("PROG_VERBOSE", Some("1"))]);
    envy_sweep("C30", 0xC030, 200);
    env_clear_all();
}

#[test]
fn c31_envy_debug_set() {
    let _g = lock();
    env_clear_all();
    env_apply(&[("PROG_DEBUG", Some("1"))]);
    envy_sweep("C31", 0xC031, 200);
    env_clear_all();
}

#[test]
fn c32_envy_verbose_debug_optimize_all_set() {
    let _g = lock();
    env_clear_all();
    env_apply(&[
        ("PROG_VERBOSE", Some("1")),
        ("PROG_DEBUG", Some("1")),
        ("PROG_OPTIMIZE", Some("1")),
    ]);
    envy_sweep("C32", 0xC032, 200);
    // and verbose+debug without optimize (multiply path with full output)
    env_apply(&[
        ("PROG_VERBOSE", Some("yes1")),
        ("PROG_DEBUG", Some("x1x")),
        ("PROG_OPTIMIZE", None),
    ]);
    envy_sweep("C32/noopt", 0xC032, 100);
    env_clear_all();
}

#[test]
fn c33_envy_flags_set_without_literal_one() {
    let _g = lock();
    env_clear_all();
    for s in ["0", "true", "yes", "", "off", "22"] {
        env_apply(&[
            ("PROG_VERBOSE", Some(s)),
            ("PROG_DEBUG", Some(s)),
            ("PROG_OPTIMIZE", None),
        ]);
        envy_sweep(&format!("C33 s={s:?}"), 0xC033, 32);
    }
    env_clear_all();
}

#[test]
fn c34_envy_base_offset_override() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC034);
    for i in 0..40 {
        let b = rng.interesting_i32();
        env_apply(&[("PROG_BASE_OFFSET", Some(&b.to_string()))]);
        envy_sweep(&format!("C34[{i}] base={b}"), 0xC034 ^ i as u64, 8);
    }
    for b in ["0", "64", "-64", "0100", "2147483647", "-2147483648"] {
        env_apply(&[("PROG_BASE_OFFSET", Some(b))]);
        envy_sweep(&format!("C34 base={b}"), 0xC034, 8);
    }
    env_clear_all();
}

#[test]
fn c35_envy_multiplier_override() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC035);
    for i in 0..40 {
        let m = rng.interesting_i32();
        env_apply(&[("PROG_MULTIPLIER", Some(&m.to_string()))]);
        envy_sweep(&format!("C35[{i}] mult={m}"), 0xC035 ^ i as u64, 8);
    }
    for m in ["0", "1", "-1", "10", "012", "2147483647", "-2147483648"] {
        env_apply(&[("PROG_MULTIPLIER", Some(m))]);
        envy_sweep(&format!("C35 mult={m}"), 0xC035, 8);
    }
    env_clear_all();
}

#[test]
fn c36_envy_base_and_multiplier_override() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC036);
    for i in 0..64 {
        let b = rng.interesting_i32();
        let m = rng.interesting_i32();
        env_apply(&[
            ("PROG_BASE_OFFSET", Some(&b.to_string())),
            ("PROG_MULTIPLIER", Some(&m.to_string())),
        ]);
        envy_sweep(&format!("C36[{i}] base={b} mult={m}"), 0xC036 ^ i as u64, 6);
    }
    env_clear_all();
}

#[test]
fn c37_envy_base_offset_comma_rejected() {
    let _g = lock();
    env_clear_all();
    for v in ["1,2", ",", "999,", ",999"] {
        env_apply(&[("PROG_BASE_OFFSET", Some(v))]);
        envy_sweep(&format!("C37 v={v:?}"), 0xC037, 24);
        // also with verbose on so the stdout AND stderr interleaving is checked
        env_apply(&[
            ("PROG_BASE_OFFSET", Some(v)),
            ("PROG_VERBOSE", Some("1")),
        ]);
        envy_sweep(&format!("C37 v={v:?} verbose"), 0xC037, 24);
    }
    env_clear_all();
}

#[test]
fn c38_envy_multiplier_semicolon_rejected() {
    let _g = lock();
    env_clear_all();
    for v in ["3;4", ";", "5;", ";5"] {
        env_apply(&[("PROG_MULTIPLIER", Some(v))]);
        envy_sweep(&format!("C38 v={v:?}"), 0xC038, 24);
        env_apply(&[("PROG_MULTIPLIER", Some(v)), ("PROG_VERBOSE", Some("1"))]);
        envy_sweep(&format!("C38 v={v:?} verbose"), 0xC038, 24);
    }
    env_clear_all();
}

#[test]
fn c39_envy_both_env_rejected() {
    let _g = lock();
    env_clear_all();
    for (b, m) in [
        ("1,2", "3;4"),
        ("1;2", "3,4"),
        (",", ","),
        (";", ";"),
        ("1,2;3", "4;5,6"),
    ] {
        env_apply(&[
            ("PROG_BASE_OFFSET", Some(b)),
            ("PROG_MULTIPLIER", Some(m)),
        ]);
        envy_sweep(&format!("C39 b={b:?} m={m:?}"), 0xC039, 24);
        env_apply(&[
            ("PROG_BASE_OFFSET", Some(b)),
            ("PROG_MULTIPLIER", Some(m)),
            ("PROG_VERBOSE", Some("1")),
            ("PROG_DEBUG", Some("1")),
        ]);
        envy_sweep(&format!("C39 b={b:?} m={m:?} v+d"), 0xC039, 24);
    }
    env_clear_all();
}

#[test]
fn c40_envy_empty_env_values() {
    let _g = lock();
    env_clear_all();
    env_apply(&[
        ("PROG_BASE_OFFSET", Some("")),
        ("PROG_MULTIPLIER", Some("")),
    ]);
    envy_sweep("C40", 0xC040, 64);
    env_apply(&[("PROG_BASE_OFFSET", Some("")), ("PROG_MULTIPLIER", None)]);
    envy_sweep("C40/base-only", 0xC040, 32);
    env_apply(&[("PROG_BASE_OFFSET", None), ("PROG_MULTIPLIER", Some(""))]);
    envy_sweep("C40/mult-only", 0xC040, 32);
    env_clear_all();
}

#[test]
fn c41_envy_param3_param4_zero_guards() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC041);
    for verbose in [None, Some("1")] {
        env_apply(&[("PROG_VERBOSE", verbose)]);
        for i in 0..64 {
            let p1 = rng.interesting_i32();
            let p2 = rng.interesting_i32();
            let nz3 = {
                let mut v = rng.interesting_i32();
                if v == 0 {
                    v = 17;
                }
                v
            };
            let nz4 = {
                let mut v = rng.interesting_i32();
                if v == 0 {
                    v = -17;
                }
                v
            };
            let tag = format!("C41[{i}] V={verbose:?}");
            diff_envy(&format!("{tag} 0/0"), p1, p2, 0, 0);
            diff_envy(&format!("{tag} 0/nz"), p1, p2, 0, nz4);
            diff_envy(&format!("{tag} nz/0"), p1, p2, nz3, 0);
            diff_envy(&format!("{tag} nz/nz"), p1, p2, nz3, nz4);
        }
    }
    env_clear_all();
}

#[test]
fn c42_envy_negative_param4_arithmetic_shift() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC042);
    for i in 0..128 {
        let p4 = -(1 + (rng.next_u32() % 0x7FFF_FFFF) as i32);
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        let p3 = rng.interesting_i32();
        diff_envy(&format!("C42[{i}]"), p1, p2, p3, p4);
    }
    for p4 in [-1, -2, -3, -4, -5, -7, -8, -9, i32::MIN, i32::MIN + 1, i32::MIN + 3] {
        diff_envy(&format!("C42 p4={p4}"), 0, 0, 0, p4);
        diff_envy(&format!("C42 p4={p4} v"), 3, 5, 7, p4);
    }
    env_clear_all();
}

/// Model of the pre-rollback `result` with the DEFAULT environment. Used only
/// to *search* for inputs that land on interesting sides of `result < 0`; the
/// assertion itself is always the C-vs-Rust differential.
fn default_env_pre_rollback(p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let mut r = p1.wrapping_mul(3).wrapping_add(p2.wrapping_div(2));
    if p3 != 0 {
        r = r.wrapping_add(p3.wrapping_mul(10));
    }
    if p4 != 0 {
        r = r.wrapping_add(p4 >> 2);
    }
    r |= 0x0F;
    r.wrapping_add(64)
}

#[test]
fn c43_envy_negative_result_rollback() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC043);
    let mut found = 0usize;
    let mut tries = 0usize;
    // Randomized search for tuples whose pre-rollback result is negative.
    while found < 128 && tries < 200_000 {
        tries += 1;
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        let p3 = rng.interesting_i32();
        let p4 = rng.interesting_i32();
        if default_env_pre_rollback(p1, p2, p3, p4) < 0 {
            found += 1;
            // verbose off ...
            env_apply(&[]);
            let r = diff_envy(&format!("C43[{found}] quiet"), p1, p2, p3, p4);
            assert_eq!(r, p1, "rollback must return param1 (C43)");
            // ... and verbose on, which adds the "Restored state" line.
            env_apply(&[("PROG_VERBOSE", Some("1"))]);
            diff_envy(&format!("C43[{found}] verbose"), p1, p2, p3, p4);
            env_apply(&[("PROG_DEBUG", Some("1"))]);
            diff_envy(&format!("C43[{found}] debug"), p1, p2, p3, p4);
        }
    }
    assert!(found >= 128, "search found only {found} negative-result tuples");
    env_clear_all();
}

#[test]
fn c44_envy_rollback_boundary() {
    let _g = lock();
    env_clear_all();
    // `apply_bit_operations` ORs in 0x0F and `base_offset` defaults to 64, so
    // with the default environment `result` is always == 15 (mod 16). The two
    // values straddling the `result < 0` test that ARE reachable are therefore
    // 15 (kept) and -1 (rolled back). Both are constructed exactly here, and
    // the reachability claim is re-derived by an exhaustive small search.
    let mut hit_minus_one = 0;
    let mut hit_fifteen = 0;
    for p2 in -400i32..=0 {
        let r = default_env_pre_rollback(0, p2, 0, 0);
        if r == -1 {
            hit_minus_one += 1;
            let got = diff_envy(&format!("C44 result=-1 p2={p2}"), 0, p2, 0, 0);
            assert_eq!(got, 0, "result==-1 must roll back to param1");
        }
        if r == 15 {
            hit_fifteen += 1;
            let got = diff_envy(&format!("C44 result=15 p2={p2}"), 0, p2, 0, 0);
            assert_eq!(got, 15, "result==15 must NOT roll back");
        }
    }
    assert!(hit_minus_one > 0 && hit_fifteen > 0);

    // The same boundary with a non-zero param1 so the rollback value is visible.
    for p1 in [-7i32, -1, 1, 7, 12345] {
        for p2 in -400i32..=0 {
            let r = default_env_pre_rollback(p1, p2, 0, 0);
            if r == -1 || r == 15 || r == -17 || r == 31 {
                let got = diff_envy(&format!("C44 p1={p1} p2={p2} r={r}"), p1, p2, 0, 0);
                assert_eq!(got, if r < 0 { p1 } else { r });
            }
        }
    }
    // With verbose on (extra output on the rollback path).
    env_apply(&[("PROG_VERBOSE", Some("1"))]);
    for p2 in -400i32..=0 {
        let r = default_env_pre_rollback(0, p2, 0, 0);
        if r == -1 || r == 15 {
            diff_envy(&format!("C44/verbose p2={p2}"), 0, p2, 0, 0);
        }
    }
    env_clear_all();
}

#[test]
fn c45_envy_param_boundary_cross_product() {
    let _g = lock();
    env_clear_all();
    let bounds = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, 2, i32::MAX - 1, i32::MAX];
    // Quiet (no output) — the full 9^4 = 6561 cross-product.
    for &a in &bounds {
        for &b in &bounds {
            for &c in &bounds {
                for &d in &bounds {
                    diff_envy("C45/quiet", a, b, c, d);
                }
            }
        }
    }
    // With every output path on, a reduced cross-product (9^2 x 4 corners).
    env_apply(&[
        ("PROG_VERBOSE", Some("1")),
        ("PROG_DEBUG", Some("1")),
    ]);
    for &a in &bounds {
        for &b in &bounds {
            for (c, d) in [(0, 0), (1, -1), (i32::MIN, i32::MAX), (i32::MAX, i32::MIN)] {
                diff_envy("C45/loud", a, b, c, d);
            }
        }
    }
    // Same again with optimize on (the add path).
    env_apply(&[
        ("PROG_VERBOSE", Some("1")),
        ("PROG_DEBUG", Some("1")),
        ("PROG_OPTIMIZE", Some("1")),
    ]);
    for &a in &bounds {
        for &b in &bounds {
            for (c, d) in [(0, 0), (1, -1), (i32::MIN, i32::MAX)] {
                diff_envy("C45/loud+opt", a, b, c, d);
            }
        }
    }
    env_clear_all();
}

#[test]
fn c46_envy_extreme_env_numerics() {
    let _g = lock();
    env_clear_all();
    let extremes = [
        "2147483647",
        "-2147483648",
        "2147483646",
        "-2147483647",
        "1073741824",
        "-1073741824",
        "0",
        "1",
        "-1",
    ];
    let mut rng = Rng::new(0xC046);
    for b in extremes {
        for m in extremes {
            env_apply(&[
                ("PROG_BASE_OFFSET", Some(b)),
                ("PROG_MULTIPLIER", Some(m)),
            ]);
            for _ in 0..3 {
                let p = [
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                ];
                diff_envy(&format!("C46 b={b} m={m}"), p[0], p[1], p[2], p[3]);
            }
            for t in [
                [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
                [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
                [1, 1, 1, 1],
                [0, 0, 0, 0],
            ] {
                diff_envy(&format!("C46 b={b} m={m} fixed"), t[0], t[1], t[2], t[3]);
            }
        }
    }
    env_clear_all();
}

#[test]
fn c47_envy_full_env_cross_product() {
    let _g = lock();
    env_clear_all();
    // verbose(3) x debug(3) x optimize(3) x base_offset(5) x multiplier(5)
    let num_states: [Option<&str>; 5] = [None, Some("123"), Some("1,2"), Some("3;4"), Some("")];
    let mut rng = Rng::new(0xC047);
    let mut combos = 0usize;
    for v in FLAG_ENV_STATES {
        for d in FLAG_ENV_STATES {
            for o in OPT_ENV_STATES {
                for b in num_states {
                    for m in num_states {
                        env_apply(&[
                            ("PROG_VERBOSE", v),
                            ("PROG_DEBUG", d),
                            ("PROG_OPTIMIZE", o),
                            ("PROG_BASE_OFFSET", b),
                            ("PROG_MULTIPLIER", m),
                        ]);
                        let tag = format!("C47 V={v:?} D={d:?} O={o:?} B={b:?} M={m:?}");
                        for _ in 0..4 {
                            let p = [
                                rng.interesting_i32(),
                                rng.interesting_i32(),
                                rng.interesting_i32(),
                                rng.interesting_i32(),
                            ];
                            diff_envy(&tag, p[0], p[1], p[2], p[3]);
                        }
                        for t in [
                            [0, 0, 0, 0],
                            [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
                            [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
                            [0, -160, 0, 0],
                        ] {
                            diff_envy(&tag, t[0], t[1], t[2], t[3]);
                        }
                        combos += 1;
                    }
                }
            }
        }
    }
    assert_eq!(combos, 3 * 3 * 3 * 5 * 5, "full env cross-product");
    env_clear_all();
}

// ===========================================================================
// Cross-cutting  (C48..C50)
// ===========================================================================

#[test]
fn c48_low_level_pipeline_composed_by_caller() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xC048);
    for_each_27_env_combo(|n, v, d, o| {
        for k in 0..6 {
            let p = [
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ];
            for initial in [Flags([0; 4]), Flags([0xFF; 4])] {
                diff_pipeline(
                    &format!("C48[{n}/{k}] V={v:?} D={d:?} O={o:?} init={initial:?}"),
                    p[0],
                    p[1],
                    p[2],
                    p[3],
                    initial,
                );
            }
        }
    });

    // Same pipeline but with the env numerics rejected / overridden, so the
    // stderr warnings appear in the middle of the composed run.
    for (b, m) in [
        (Some("1,2"), Some("3;4")),
        (Some("500"), Some("-3")),
        (Some(""), Some("")),
        (None, None),
    ] {
        env_apply(&[
            ("PROG_VERBOSE", Some("1")),
            ("PROG_DEBUG", Some("1")),
            ("PROG_BASE_OFFSET", b),
            ("PROG_MULTIPLIER", m),
        ]);
        for _ in 0..8 {
            let p = [
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ];
            diff_pipeline(
                &format!("C48/env b={b:?} m={m:?}"),
                p[0],
                p[1],
                p[2],
                p[3],
                Flags([0; 4]),
            );
        }
    }

    // Drive the pipeline with cache_enabled = 0 and log_level != 3 — states
    // `init_config_from_env` can never produce, reachable only via the
    // low-level exports.
    env_clear_all();
    for ll in 0u8..8 {
        for cache in [false, true] {
            for verbose in [false, true] {
                for dbg in [false, true] {
                    let flags = Flags::from_byte0(flag_byte(verbose, dbg, false, cache, ll));
                    for _ in 0..4 {
                        let a = rng.interesting_i32();
                        let b = rng.interesting_i32();
                        let mut r = diff_perform_op("C48/manual", a, b, flags);
                        r = diff_apply_bits("C48/manual", r, flags);
                        // feed the output back in, twice, to compound any drift
                        r = diff_apply_bits("C48/manual2", r, flags);
                        let _ = diff_perform_op("C48/manual3", r, a, flags);
                    }
                }
            }
        }
    }
    env_clear_all();
}

#[test]
fn c49_symbol_parity_and_load() {
    let _g = lock();
    let l = libs();
    for lib in [&l.c, &l.rs] {
        let _ = lib.envy();
        let _ = lib.parse_env_numeric();
        let _ = lib.init_config_from_env();
        let _ = lib.perform_operation();
        let _ = lib.apply_bit_operations();
        assert!(matches!(lib.tag, "C" | "Rust"));
    }
}

#[test]
fn c50_no_driver_binary_is_built() {
    // c_src/CMakeLists.txt contains only `add_library(... SHARED src/lib.c)`
    // and translation/Cargo.toml declares only `crate-type = ["cdylib"]`, so
    // there is no executable whose stdout could be compared. Assert that this
    // is still true, so the gate is revisited if a driver is ever added.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let cmake = std::fs::read_to_string(root.join("c_src/CMakeLists.txt")).unwrap();
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable — the binary-stdout gate must be implemented"
    );
    let toml = std::fs::read_to_string(root.join("translation/Cargo.toml")).unwrap();
    assert!(
        !toml.contains("[[bin]]"),
        "translation now builds a binary — the binary-stdout gate must be implemented"
    );
    // main.rs / src/bin would also produce a binary.
    assert!(!root.join("translation/src/main.rs").exists());
    assert!(!root.join("translation/src/bin").exists());
}
