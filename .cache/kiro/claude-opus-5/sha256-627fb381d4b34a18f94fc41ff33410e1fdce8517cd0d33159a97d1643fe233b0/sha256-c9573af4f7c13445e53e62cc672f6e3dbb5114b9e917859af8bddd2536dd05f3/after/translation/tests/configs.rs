//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH shared objects
//! through their exported symbols and compares the captured stdout byte for
//! byte. Inputs are randomized with a fixed seed, so a failure is reproducible.
//!
//! The whole harness lock is held for the duration of each test so that the two
//! libraries' internal `the_house` state advances in exact lockstep.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

const INT_MAX: i64 = i32::MAX as i64;
const INT_MIN: i64 = i32::MIN as i64;

fn dec(v: i64) -> Vec<u8> {
    v.to_string().into_bytes()
}

/// Non-digit suffixes that must be *accepted* (the C only checks `endp != str`).
const GARBAGE: [&[u8]; 12] = [
    b"abc", b" 8", b".9", b",", b"!", b"\t7", b"xyz", b"-1", b"+2", b"e10", b"%d", b"\n\n",
];

// ---------------------------------------------------------------------------
// Rows 1-7: the low-level `run` entry point
// ---------------------------------------------------------------------------

/// CONFIGS row 1 — `run(0)`, repeated.
#[test]
fn row01_run_zero() {
    let p = pair();
    for i in 0..32 {
        let (c, r) = p.run_step(0);
        assert!(is_four_house_lines(&c), "iteration {i}: bad C output shape");
        same(&format!("row01 run(0) #{i}"), &c, &r);
    }
}

/// CONFIGS row 2 — `run` over the full random `i32` range.
#[test]
fn row02_run_full_range_random() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x02);
    for i in 0..256 {
        let n = rng.next_i32();
        let (c, r) = p.run_step(n);
        assert!(is_four_house_lines(&c), "iteration {i}: bad C output shape");
        same(&format!("row02 run({n}) #{i}"), &c, &r);
    }
}

/// CONFIGS row 3 — small positive increments.
#[test]
fn row03_run_small_positive() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x03);
    for i in 0..128 {
        let n = rng.range_i64(1, 1000) as i32;
        let (c, r) = p.run_step(n);
        same(&format!("row03 run({n}) #{i}"), &c, &r);
    }
}

/// CONFIGS row 4 — small negative increments.
#[test]
fn row04_run_small_negative() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x04);
    for i in 0..128 {
        let n = rng.range_i64(-1000, -1) as i32;
        let (c, r) = p.run_step(n);
        same(&format!("row04 run({n}) #{i}"), &c, &r);
    }
}

/// CONFIGS row 5 — every boundary value, applied twice each.
#[test]
fn row05_run_boundaries() {
    let p = pair();
    let vals: [i32; 12] = [
        0,
        1,
        -1,
        2,
        -2,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        65535,
        65536,
        -65536,
    ];
    for pass in 0..2 {
        for &n in &vals {
            let (c, r) = p.run_step(n);
            assert!(is_four_house_lines(&c), "bad C output for run({n})");
            same(&format!("row05 pass{pass} run({n})"), &c, &r);
        }
    }
}

/// CONFIGS row 6 — repeated `INT_MAX`, wrapping `bedrooms` every call.
#[test]
fn row06_run_int_max_repeated() {
    let p = pair();
    for i in 0..16 {
        let (c, r) = p.run_step(i32::MAX);
        same(&format!("row06 run(INT_MAX) #{i}"), &c, &r);
    }
}

/// CONFIGS row 7 — deep accumulated state (wide `%d` / `%.1f` fields).
#[test]
fn row07_run_deep_state() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x07);
    let mut last = Vec::new();
    for i in 0..512 {
        let n = rng.range_i64(-5, 5) as i32;
        let (c, r) = p.run_step(n);
        same(&format!("row07 run({n}) #{i}"), &c, &r);
        last = c;
    }
    let (floors, _, bathrooms) = parse_last_state(&last).expect("parse state");
    assert!(
        floors > 500 && bathrooms > 500.0,
        "row07 did not reach deep state: floors={floors} bathrooms={bathrooms}"
    );
}

// ---------------------------------------------------------------------------
// Rows 8-17, 21, 22: the `driver` entry point
// ---------------------------------------------------------------------------

/// CONFIGS row 8 — unsigned valid decimals.
#[test]
fn row08_driver_unsigned_random() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x08);
    for i in 0..256 {
        let v = rng.range_i64(0, INT_MAX);
        let s = dec(v);
        let (c, r) = p.driver_step_raw(&s);
        assert_ne!(c, ERROR_LINE, "C rejected valid input {v}");
        same(&format!("row08 driver({v}) #{i}"), &c, &r);
    }
}

/// CONFIGS row 9 — negative valid decimals.
#[test]
fn row09_driver_negative_random() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x09);
    for i in 0..256 {
        let v = rng.range_i64(INT_MIN, -1);
        let s = dec(v);
        let (c, r) = p.driver_step_raw(&s);
        assert_ne!(c, ERROR_LINE, "C rejected valid input {v}");
        same(&format!("row09 driver({v}) #{i}"), &c, &r);
    }
}

/// CONFIGS row 10 — explicit `+` sign.
#[test]
fn row10_driver_plus_sign() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x0A);
    for i in 0..128 {
        let v = rng.range_i64(0, INT_MAX);
        let mut s = vec![b'+'];
        s.extend_from_slice(&dec(v));
        let (c, r) = p.driver_step_raw(&s);
        assert_ne!(c, ERROR_LINE, "C rejected valid input +{v}");
        same(&format!("row10 driver(+{v}) #{i}"), &c, &r);
    }
}

/// CONFIGS row 11 — leading whitespace.
#[test]
fn row11_driver_leading_whitespace() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x0B);
    for i in 0..256 {
        let v = rng.range_i64(INT_MIN, INT_MAX);
        let mut s = ws(&mut rng, 8);
        if rng.bool() && v >= 0 {
            s.push(b'+');
        }
        s.extend_from_slice(&dec(v));
        let (c, r) = p.driver_step_raw(&s);
        assert_ne!(
            c,
            ERROR_LINE,
            "C rejected {:?}",
            String::from_utf8_lossy(&s)
        );
        same(&format!("row11 driver({:?}) #{i}", String::from_utf8_lossy(&s)), &c, &r);
    }
}

/// CONFIGS row 12 — leading zeros, still base 10.
#[test]
fn row12_driver_leading_zeros() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x0C);
    for i in 0..128 {
        let v = rng.range_i64(0, INT_MAX);
        let zeros = rng.range_usize(1, 20);
        let mut s = Vec::new();
        if rng.bool() {
            s.push(b'-');
        }
        s.extend(std::iter::repeat(b'0').take(zeros));
        s.extend_from_slice(&dec(v));
        let (c, r) = p.driver_step_raw(&s);
        assert_ne!(
            c,
            ERROR_LINE,
            "C rejected {:?}",
            String::from_utf8_lossy(&s)
        );
        same(&format!("row12 driver({:?}) #{i}", String::from_utf8_lossy(&s)), &c, &r);
    }
}

/// CONFIGS row 13 — trailing garbage is ACCEPTED (`endp != str` only).
#[test]
fn row13_driver_trailing_garbage() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x0D);
    for i in 0..256 {
        let v = rng.range_i64(INT_MIN, INT_MAX);
        let mut s = dec(v);
        s.extend_from_slice(rng.pick(&GARBAGE));
        let (c, r) = p.driver_step_raw(&s);
        assert_ne!(
            c,
            ERROR_LINE,
            "C must accept a valid prefix with garbage: {:?}",
            String::from_utf8_lossy(&s)
        );
        same(&format!("row13 driver({:?}) #{i}", String::from_utf8_lossy(&s)), &c, &r);
    }
}

/// CONFIGS row 14 — `0x…` under base 10 parses as `0` and is accepted.
#[test]
fn row14_driver_hex_looking() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x0E);
    let hexdigits = b"0123456789abcdefABCDEF";
    for i in 0..64 {
        let mut s = Vec::new();
        if rng.bool() {
            s.push(b'-');
        }
        s.push(b'0');
        s.push(if rng.bool() { b'x' } else { b'X' });
        let n = rng.range_usize(0, 6);
        for _ in 0..n {
            s.push(*rng.pick(hexdigits));
        }
        let (c, r) = p.driver_step_raw(&s);
        assert_ne!(
            c,
            ERROR_LINE,
            "C must accept {:?} as 0",
            String::from_utf8_lossy(&s)
        );
        same(&format!("row14 driver({:?}) #{i}", String::from_utf8_lossy(&s)), &c, &r);
    }
}

/// CONFIGS row 15 — exact accepted-limit boundaries, with decorations.
#[test]
fn row15_driver_boundary_magnitudes() {
    let p = pair();
    let bases: [&[u8]; 10] = [
        b"0",
        b"-0",
        b"+0",
        b"1",
        b"-1",
        b"2147483647",
        b"-2147483648",
        b"2147483646",
        b"-2147483647",
        b"+2147483647",
    ];
    let decorations: [&[u8]; 4] = [b"", b" ", b"\t", b"  \n "];
    for base in bases {
        for deco in decorations {
            let mut s = deco.to_vec();
            s.extend_from_slice(base);
            let (c, r) = p.driver_step_raw(&s);
            assert_ne!(
                c,
                ERROR_LINE,
                "C rejected boundary input {:?}",
                String::from_utf8_lossy(&s)
            );
            same(&format!("row15 driver({:?})", String::from_utf8_lossy(&s)), &c, &r);
        }
    }
}

/// CONFIGS row 16 — embedded NUL truncates the parse.
#[test]
fn row16_driver_embedded_nul() {
    let p = pair();
    let cases: [&[u8]; 8] = [
        b"123\x00456",
        b"0\x00999",
        b"-7\x00abc",
        b"\x00123",
        b"  42\x00\x00",
        b"+5\x00-5",
        b"2147483647\x001",
        b"abc\x0012",
    ];
    for s in cases {
        let (c, r) = p.driver_step_raw(s);
        same(&format!("row16 driver({:?})", String::from_utf8_lossy(s)), &c, &r);
    }
}

/// CONFIGS row 17 — long-but-valid buffers (no length limit exists).
#[test]
fn row17_driver_long_valid() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x11);
    for i in 0..32 {
        let v = rng.range_i64(INT_MIN, INT_MAX);
        let pad = rng.range_usize(1, 4000);
        let mut s = ws(&mut rng, 4);
        if v < 0 {
            s.push(b'-');
        }
        s.extend(std::iter::repeat(b'0').take(pad));
        s.extend_from_slice(&dec(v.abs().min(INT_MAX)));
        let (c, r) = p.driver_step_raw(&s);
        assert_ne!(c, ERROR_LINE, "C rejected a long valid input (len {})", s.len());
        same(&format!("row17 driver(len={}) #{i}", s.len()), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// Rows 18-20: composed pipeline / state hand-off
// ---------------------------------------------------------------------------

/// CONFIGS row 18 — `driver` and `run` interleaved, all inputs valid.
#[test]
fn row18_interleaved_valid() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x12);
    for i in 0..256 {
        if rng.bool() {
            let v = rng.range_i64(INT_MIN, INT_MAX);
            let (c, r) = p.driver_step_raw(&dec(v));
            same(&format!("row18 driver({v}) #{i}"), &c, &r);
        } else {
            let n = rng.next_i32();
            let (c, r) = p.run_step(n);
            same(&format!("row18 run({n}) #{i}"), &c, &r);
        }
    }
}

/// CONFIGS row 19 — interleaved, with a mix of valid and rejected `driver`
/// inputs, so the two libraries' state only stays in sync if the error path
/// really does skip both `run` calls in both implementations.
#[test]
fn row19_interleaved_mixed_validity() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x13);
    let invalid: [&[u8]; 8] = [
        b"",
        b"abc",
        b"   ",
        b"+",
        b"-",
        b"99999999999999999999",
        b"2147483648",
        b"-2147483649",
    ];
    let mut n_err = 0usize;
    let mut n_ok = 0usize;
    for i in 0..256 {
        let kind = rng.range_usize(0, 2);
        match kind {
            0 => {
                let v = rng.range_i64(INT_MIN, INT_MAX);
                let (c, r) = p.driver_step_raw(&dec(v));
                same(&format!("row19 driver({v}) #{i}"), &c, &r);
                n_ok += 1;
            }
            1 => {
                let s = *rng.pick(&invalid);
                let (c, r) = p.driver_step_raw(s);
                assert_eq!(
                    c,
                    ERROR_LINE,
                    "expected C rejection for {:?}",
                    String::from_utf8_lossy(s)
                );
                same(&format!("row19 driver({:?}) #{i}", String::from_utf8_lossy(s)), &c, &r);
                n_err += 1;
            }
            _ => {
                let n = rng.next_i32();
                let (c, r) = p.run_step(n);
                same(&format!("row19 run({n}) #{i}"), &c, &r);
            }
        }
    }
    assert!(n_err > 20 && n_ok > 20, "row19 corpus was not mixed enough");
}

/// CONFIGS row 20 — land `bedrooms` on exact boundary values, computed from the
/// state read back out of the previous line.
#[test]
fn row20_run_exact_state_boundaries() {
    let p = pair();
    for round in 0..4 {
        for &target in &[i32::MAX, i32::MIN, 0i32, -1i32] {
            // Read the live state without perturbing bedrooms.
            let (c0, r0) = p.run_step(0);
            same(&format!("row20 probe round{round}"), &c0, &r0);
            let (_, bedrooms, _) = parse_last_state(&c0).expect("parse state");
            let delta = target.wrapping_sub(bedrooms);

            let (c, r) = p.run_step(delta);
            same(&format!("row20 round{round} target={target} delta={delta}"), &c, &r);

            let (_, got, _) = parse_last_state(&c).expect("parse state");
            assert_eq!(got, target, "row20 failed to hit the target state");
        }
    }
}

/// CONFIGS row 21 — identical input repeatedly; the state must keep advancing.
#[test]
fn row21_driver_repeated_identical() {
    let p = pair();
    let mut seen = std::collections::HashSet::new();
    for i in 0..64 {
        let (c, r) = p.driver_step_raw(b"7");
        same(&format!("row21 driver(7) #{i}"), &c, &r);
        seen.insert(c);
    }
    assert!(
        seen.len() > 1,
        "state never advanced — the_house looks like it is being reset"
    );
}

/// CONFIGS row 22 — mixed-shape property fuzz over the whole `driver` surface.
#[test]
fn row22_driver_mixed_corpus_fuzz() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x16);
    let mut rejected = 0usize;
    let mut accepted = 0usize;
    for i in 0..1024 {
        let s = mixed_input(&mut rng);
        let (c, r) = p.driver_step_raw(&s);
        same(&format!("row22 driver({:?}) #{i}", String::from_utf8_lossy(&s)), &c, &r);
        if c == ERROR_LINE {
            rejected += 1;
        } else {
            accepted += 1;
        }
    }
    assert!(
        rejected > 50 && accepted > 50,
        "fuzz corpus was one-sided: {accepted} accepted / {rejected} rejected"
    );
}

/// Generator mixing every shape `CONFIGS.md` enumerates.
fn mixed_input(rng: &mut Rng) -> Vec<u8> {
    let mut s = Vec::new();
    // optional leading whitespace
    if rng.range_usize(0, 3) == 0 {
        s.extend_from_slice(&ws(rng, 5));
    }
    match rng.range_usize(0, 9) {
        0 => {} // empty / whitespace only
        1 => s.push(*rng.pick(&[b'+', b'-'])),
        2 => s.extend_from_slice(rng.pick(&GARBAGE)),
        3 => {
            // huge magnitude -> ERANGE
            let digits = rng.range_usize(20, 40);
            if rng.bool() {
                s.push(b'-');
            }
            for _ in 0..digits {
                s.push(b'0' + rng.range_usize(1, 9) as u8);
            }
        }
        4 => {
            // just past INT range but inside long
            let v = if rng.bool() {
                rng.range_i64(INT_MAX + 1, INT_MAX + 1_000_000)
            } else {
                rng.range_i64(INT_MIN - 1_000_000, INT_MIN - 1)
            };
            s.extend_from_slice(&dec(v));
        }
        5 => {
            // long boundaries
            let v = *rng.pick(&[i64::MAX, i64::MIN, i64::MAX - 1, i64::MIN + 1]);
            s.extend_from_slice(&dec(v));
        }
        6 => {
            // valid with garbage suffix
            let v = rng.range_i64(INT_MIN, INT_MAX);
            s.extend_from_slice(&dec(v));
            s.extend_from_slice(rng.pick(&GARBAGE));
        }
        7 => {
            // hex-looking
            s.push(b'0');
            s.push(b'x');
            s.push(*rng.pick(b"0123456789abcdef"));
        }
        8 => {
            // plain valid
            let v = rng.range_i64(INT_MIN, INT_MAX);
            s.extend_from_slice(&dec(v));
        }
        _ => {
            // leading zeros
            s.extend(std::iter::repeat(b'0').take(rng.range_usize(1, 25)));
            s.extend_from_slice(&dec(rng.range_i64(0, INT_MAX)));
        }
    }
    s
}
