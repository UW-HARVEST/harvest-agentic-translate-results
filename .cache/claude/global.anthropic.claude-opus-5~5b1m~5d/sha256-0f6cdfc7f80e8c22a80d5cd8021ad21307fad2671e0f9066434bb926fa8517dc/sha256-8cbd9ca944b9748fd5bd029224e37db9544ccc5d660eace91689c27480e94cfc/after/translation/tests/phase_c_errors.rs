//! Phase C — error/rejection-path differential tests.
//! One test per row of `ERRORS.md`. Both implementations are driven only
//! through their `.so` exports.

mod harness;

use harness::{buf_from, same, same_bytes, Pair, Rng};

// ---------------------------------------------------------------------------
// Rows 1-5: divide_multiplier guards and truncation
// ---------------------------------------------------------------------------

#[test]
fn err_01_divide_by_zero_guard() {
    // b == 0 -> the division at lib.c:55 is skipped; multiplier unchanged.
    let mut r = Rng::new(0x0301);
    for _ in 0..50 {
        let p = Pair::fresh();
        let m = r.i32_any();
        same("err1: prime mul", p.c.mul(m, 1), p.rust.mul(m, 1));
        let a = r.i32_any();
        let cv = p.c.div(a, 0);
        let rv = p.rust.div(a, 0);
        same(&format!("err1: div({a},0) returns unchanged multiplier"), cv, rv);
        // Value must equal what mul left behind: guard truly skipped.
        same("err1: unchanged", cv, m.wrapping_mul(1));
    }
}

#[test]
fn err_02_divide_by_zero_still_counts() {
    // The rejected path still does operation_count++, observable via findrep's
    // `result += operation_count * 010`.
    let p = Pair::fresh();
    for i in 0..40 {
        same(
            &format!("err2: div(_,0) #{i}"),
            p.c.div(i, 0),
            p.rust.div(i, 0),
        );
        same(
            &format!("err2: findrep sees operation_count #{i}"),
            p.c.findrep(0, 0, 0, 0),
            p.rust.findrep(0, 0, 0, 0),
        );
    }
}

#[test]
fn err_03_divide_by_one() {
    let mut r = Rng::new(0x0303);
    for _ in 0..50 {
        let p = Pair::fresh();
        let m = r.i32_any();
        same("err3: prime", p.c.mul(m, 1), p.rust.mul(m, 1));
        same("err3: div(_,1)", p.c.div(0, 1), p.rust.div(0, 1));
        same("err3: div(_,1) again", p.c.div(0, 1), p.rust.div(0, 1));
    }
}

#[test]
fn err_04_divide_by_negative_one() {
    // Sign flip. Excludes multiplier == INT_MIN (ERRORS.md row U4: SIGFPE).
    let mut r = Rng::new(0x0304);
    for _ in 0..50 {
        let p = Pair::fresh();
        let mut m = r.i32_any();
        if m == i32::MIN {
            m = i32::MIN + 1;
        }
        same("err4: prime", p.c.mul(m, 1), p.rust.mul(m, 1));
        same("err4: div(_,-1)", p.c.div(0, -1), p.rust.div(0, -1));
        same("err4: div(_,-1) again", p.c.div(0, -1), p.rust.div(0, -1));
    }
}

#[test]
fn err_05_division_truncates_toward_zero() {
    // C truncates toward zero: -7 / 2 == -3. Verify explicitly plus randomly.
    let p = Pair::fresh();
    same("err5: prime -7", p.c.mul(-7, 1), p.rust.mul(-7, 1));
    let cv = p.c.div(0, 2);
    same("err5: -7/2", cv, p.rust.div(0, 2));
    same("err5: C truncates toward zero", cv, -3);

    let mut r = Rng::new(0x0305);
    for _ in 0..200 {
        let p = Pair::fresh();
        let m = r.i32_in(-1000, 1000);
        same("err5: prime", p.c.mul(m, 1), p.rust.mul(m, 1));
        let mut b = r.i32_in(-50, 50);
        if b == 0 {
            b = 3;
        }
        same(
            &format!("err5: {m}/{b}"),
            p.c.div(0, b),
            p.rust.div(0, b),
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 6-11: find_and_replace_char rejections
// ---------------------------------------------------------------------------

#[test]
fn err_06_replace_char_not_found() {
    let p = Pair::fresh();
    for ch in [b'q' as i32, b'Z' as i32, 1, 127] {
        let s = buf_from(b"abcdef", 32);
        let cv = p.c.replace(&s, ch);
        same_bytes(&format!("err6: not found {ch}"), &cv, &p.rust.replace(&s, ch));
        same_bytes("err6: buffer untouched", &cv, &s);
    }
}

#[test]
fn err_07_replace_empty_string() {
    let p = Pair::fresh();
    for ch in [0, 1, b'X' as i32, 255, -1, i32::MIN, i32::MAX] {
        let s = buf_from(b"", 16);
        let cv = p.c.replace(&s, ch);
        same_bytes(&format!("err7: empty, ch={ch}"), &cv, &p.rust.replace(&s, ch));
        same_bytes("err7: buffer untouched", &cv, &s);
    }
}

#[test]
fn err_08_replace_nul_never_found() {
    // search_char == 0: the NUL is outside the strlen window, so memchr
    // cannot find it and nothing is written.
    let p = Pair::fresh();
    for body in [&b""[..], b"a", b"abc", b"Octal: 0123"] {
        let s = buf_from(body, 32);
        let cv = p.c.replace(&s, 0);
        same_bytes(
            &format!("err8: ch=0 on {:?}", String::from_utf8_lossy(body)),
            &cv,
            &p.rust.replace(&s, 0),
        );
        same_bytes("err8: buffer untouched", &cv, &s);
    }
    // Also 0 reached as a multiple of 256, which narrows to 0.
    for ch in [256, 512, -256, 0x10000, i32::MIN] {
        let s = buf_from(b"abc", 32);
        let cv = p.c.replace(&s, ch);
        same_bytes(&format!("err8: ch={ch} narrows to 0"), &cv, &p.rust.replace(&s, ch));
        same_bytes("err8: buffer untouched", &cv, &s);
    }
}

#[test]
fn err_09_replace_char_narrowing() {
    let p = Pair::fresh();
    let s = buf_from(b"Octal: 0123, Decimal: 83", 64);
    // 'O' == 0x4F reached in several out-of-unsigned-char-range ways.
    for ch in [
        b'O' as i32,
        b'O' as i32 + 256,
        b'O' as i32 + 512,
        b'O' as i32 - 256,
        b'O' as i32 - 512,
        0x14F,
        0x7FFF_FF4F,
    ] {
        let cv = p.c.replace(&s, ch);
        same_bytes(&format!("err9: ch={ch:#x}"), &cv, &p.rust.replace(&s, ch));
        assert_eq!(cv[0], b'X', "err9: {ch:#x} should narrow to 'O'");
    }
    // Random out-of-range values.
    let mut r = Rng::new(0x0309);
    for _ in 0..500 {
        let ch = r.i32_any();
        let cv = p.c.replace(&s, ch);
        same_bytes(&format!("err9: random ch={ch}"), &cv, &p.rust.replace(&s, ch));
    }
}

#[test]
fn err_10_replace_char_is_x() {
    let p = Pair::fresh();
    for body in [&b"X"[..], b"aXbXc", b"abcX", b"XXXX"] {
        let s = buf_from(body, 32);
        let cv = p.c.replace(&s, b'X' as i32);
        same_bytes(
            &format!("err10: {:?}", String::from_utf8_lossy(body)),
            &cv,
            &p.rust.replace(&s, b'X' as i32),
        );
        same_bytes("err10: idempotent", &cv, &s);
    }
}

#[test]
fn err_11_replace_only_first() {
    let p = Pair::fresh();
    for (body, ch) in [
        (&b"aaaa"[..], b'a'),
        (b"abababab", b'b'),
        (b"zzz", b'z'),
        (b"Function pointer example with static vars", b'n'),
    ] {
        let s = buf_from(body, 64);
        let cv = p.c.replace(&s, ch as i32);
        same_bytes(
            &format!("err11: {:?} ch={}", String::from_utf8_lossy(body), ch as char),
            &cv,
            &p.rust.replace(&s, ch as i32),
        );
        // Exactly one byte differs from the input.
        let diffs = cv.iter().zip(s.iter()).filter(|(a, b)| a != b).count();
        assert_eq!(diffs, 1, "err11: exactly one replacement expected");
    }
}

// ---------------------------------------------------------------------------
// Rows 12-20: validate_and_normalize range checks
// ---------------------------------------------------------------------------

#[track_caller]
fn norm_eq(tag: &str, v: i32, expected: i32) {
    let p = Pair::fresh();
    let cv = p.c.normalize(v);
    same(&format!("{tag}: normalize({v})"), cv, p.rust.normalize(v));
    same(&format!("{tag}: normalize({v}) == {expected}"), cv, expected);
}

#[test]
fn err_12_normalize_zero_not_clamped() {
    norm_eq("err12", 0, 0);
}

#[test]
fn err_13_normalize_negative_not_clamped() {
    let mut r = Rng::new(0x0313);
    for v in [-1, -2, -63, -64, -100, -511, -512, i32::MIN + 1] {
        norm_eq("err13", v, v);
    }
    for _ in 0..300 {
        let v = r.i32_in(i32::MIN, -1);
        norm_eq("err13/rand", v, v);
    }
}

#[test]
fn err_14_normalize_lower_clamp() {
    for v in 1..=63 {
        norm_eq("err14", v, 0o100);
    }
}

#[test]
fn err_15_normalize_upper_clamp() {
    let mut r = Rng::new(0x0315);
    for v in [512, 513, 1000, 100000, i32::MAX - 1] {
        norm_eq("err15", v, 0o777);
    }
    for _ in 0..300 {
        norm_eq("err15/rand", r.i32_in(512, i32::MAX), 0o777);
    }
}

#[test]
fn err_16_normalize_lower_boundary() {
    norm_eq("err16", 0o100, 0o100); // `<` is strict: 64 is NOT clamped
}

#[test]
fn err_17_normalize_upper_boundary() {
    norm_eq("err17", 0o777, 0o777); // `>` is strict: 511 is NOT clamped
}

#[test]
fn err_18_normalize_one_past_boundaries() {
    norm_eq("err18", 0o77, 0o100); // 63 -> 64
    norm_eq("err18", 0o101, 0o101); // 65 unchanged
    norm_eq("err18", 0o776, 0o776); // 510 unchanged
    norm_eq("err18", 0o1000, 0o777); // 512 -> 511
}

#[test]
fn err_19_normalize_int_max() {
    norm_eq("err19", i32::MAX, 0o777);
}

#[test]
fn err_20_normalize_int_min() {
    norm_eq("err20", i32::MIN, i32::MIN);
}

// ---------------------------------------------------------------------------
// Rows 21-26: findrep guard rejections
// ---------------------------------------------------------------------------

#[test]
fn err_21_findrep_all_zero_params() {
    // active_params == 0: no operation_func is dispatched at all.
    let p = Pair::fresh();
    let cv = p.c.findrep(0, 0, 0, 0);
    same("err21: findrep(0,0,0,0)", cv, p.rust.findrep(0, 0, 0, 0));
    for i in 0..20 {
        same(
            &format!("err21: repeat #{i}"),
            p.c.findrep(0, 0, 0, 0),
            p.rust.findrep(0, 0, 0, 0),
        );
    }
}

#[test]
fn err_22a_findrep_sentinel_known_hits() {
    // Analytically derived inputs that make the computed result exactly 0 on
    // FRESH state, so `if (!result_exists) result = 0777;` (lib.c:169-171)
    // fires. Derivation for param1 = -9, rest 0:
    //   memchr('p' in "Function pointer...") = 9              -> result = 9
    //   active_params == 1 -> add(normalize(-9), 0) = -9      -> result = 0
    //   accumulator(-9) > 0150? no.  active_params >= 2? no.
    //   both_active (acc=-9, mult=1) -> result += -9 + 1      -> result = -8
    //   multiplier(1) > 0100? no.  result += operation_count(1) * 010 = 8
    //   -> result == 0 -> SENTINEL -> 0777
    for (a, b, c, d) in [(-9, 0, 0, 0), (0, -9, 0, 0)] {
        let p = Pair::fresh();
        let cv = p.c.findrep(a, b, c, d);
        same(
            &format!("err22a: findrep({a},{b},{c},{d})"),
            cv,
            p.rust.findrep(a, b, c, d),
        );
        same(
            &format!("err22a: findrep({a},{b},{c},{d}) must be the 0777 sentinel"),
            cv,
            0o777,
        );
    }
    // Neighbours must NOT be 511, proving the 511 above really is the sentinel
    // and not a coincidental natural result.
    for (a, b, c, d) in [(-8, 0, 0, 0), (-10, 0, 0, 0), (0, -8, 0, 0), (0, -10, 0, 0)] {
        let p = Pair::fresh();
        let cv = p.c.findrep(a, b, c, d);
        same(
            &format!("err22a: neighbour findrep({a},{b},{c},{d})"),
            cv,
            p.rust.findrep(a, b, c, d),
        );
        assert_ne!(cv, 0o777, "err22a: neighbour should not be the sentinel");
    }
}

#[test]
fn err_22b_findrep_sentinel_exhaustive_fresh_sweep() {
    // Exhaustively sweep every single-nonzero-parameter value in a wide window
    // on FRESH state, and every 2-parameter pair in a smaller window. This
    // covers the whole `result == 0` sentinel class rather than one input.
    let mut sentinel_hits = 0usize;
    for pos in 0..4 {
        for v in -700i32..=700 {
            let mut a = [0i32; 4];
            a[pos] = v;
            let p = Pair::fresh();
            let cv = p.c.findrep(a[0], a[1], a[2], a[3]);
            same(
                &format!("err22b: findrep({},{},{},{})", a[0], a[1], a[2], a[3]),
                cv,
                p.rust.findrep(a[0], a[1], a[2], a[3]),
            );
            if cv == 0o777 {
                sentinel_hits += 1;
            }
        }
    }
    for x in -40i32..=40 {
        for y in -40i32..=40 {
            let p = Pair::fresh();
            let cv = p.c.findrep(x, y, 0, 0);
            same(&format!("err22b: findrep({x},{y},0,0)"), cv, p.rust.findrep(x, y, 0, 0));
            if cv == 0o777 {
                sentinel_hits += 1;
            }
            let p = Pair::fresh();
            let cv = p.c.findrep(0, 0, x, y);
            same(&format!("err22b: findrep(0,0,{x},{y})"), cv, p.rust.findrep(0, 0, x, y));
            if cv == 0o777 {
                sentinel_hits += 1;
            }
            let p = Pair::fresh();
            let cv = p.c.findrep(x, 0, y, 0);
            same(&format!("err22b: findrep({x},0,{y},0)"), cv, p.rust.findrep(x, 0, y, 0));
            if cv == 0o777 {
                sentinel_hits += 1;
            }
        }
    }
    assert!(
        sentinel_hits > 0,
        "err22b: the sweep never reached the 0777 sentinel branch - \
         coverage regression, widen the sweep"
    );
    eprintln!("err22b: sentinel/511 results observed: {sentinel_hits}");
}

#[test]
fn err_22_findrep_zero_result_sentinel() {
    // Search for inputs whose computed result is 0 so that the
    // `if (!result_exists) result = 0777;` sentinel at lib.c:169-171 fires,
    // and verify both agree on the sentinel wherever it is hit.
    let mut r = Rng::new(0x0322);
    let mut sentinel_hits = 0usize;
    for _ in 0..3000 {
        let p = Pair::fresh();
        let (a, b, c, d) = (r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any());
        let cv = p.c.findrep(a, b, c, d);
        same(
            &format!("err22: findrep({a},{b},{c},{d})"),
            cv,
            p.rust.findrep(a, b, c, d),
        );
        if cv == 0o777 {
            sentinel_hits += 1;
        }
    }
    // The sentinel value 0777 must never be confused: a genuinely-zero result
    // is the only way findrep can return 0, and it never does.
    // (Both implementations agree either way; this just records coverage.)
    eprintln!("err22: results equal to 0777 (incl. sentinel): {sentinel_hits}");

    // Long shared-state sequence, which is where result==0 is most reachable.
    let p = Pair::fresh();
    let mut zero_free = true;
    for i in 0..2000 {
        let (a, b, c, d) = (
            r.i32_in(-3, 3),
            r.i32_in(-3, 3),
            r.i32_in(-3, 3),
            r.i32_in(-3, 3),
        );
        let cv = p.c.findrep(a, b, c, d);
        same(
            &format!("err22: seq #{i} findrep({a},{b},{c},{d})"),
            cv,
            p.rust.findrep(a, b, c, d),
        );
        if cv == 0 {
            zero_free = false;
        }
    }
    assert!(zero_free, "findrep must never return 0 (sentinel replaces it)");
}

#[test]
fn err_23_findrep_accumulator_guard_false() {
    // accumulator <= 0150 -> subtract_from_accumulator NOT dispatched.
    let mut r = Rng::new(0x0323);
    for _ in 0..80 {
        let p = Pair::fresh();
        // Negative params keep accumulator low (validate_and_normalize does
        // not clamp negatives, so the add is strongly negative).
        let (a, b) = (r.i32_in(-100000, -1), r.i32_in(-100000, -1));
        same(
            &format!("err23: findrep({a},{b},0,0)"),
            p.c.findrep(a, b, 0, 0),
            p.rust.findrep(a, b, 0, 0),
        );
    }
    // Exact boundary: accumulator == 104 (0150) -> `>` false, guard skipped.
    let p = Pair::fresh();
    same("err23: prime to exactly 104", p.c.add(104, 0), p.rust.add(104, 0));
    same(
        "err23: findrep with accumulator == 0150",
        p.c.findrep(0, 0, 0, 0),
        p.rust.findrep(0, 0, 0, 0),
    );
}

#[test]
fn err_24_findrep_multiplier_guard_false() {
    // multiplier <= 0100 -> divide_multiplier NOT dispatched.
    let p = Pair::fresh();
    // Fresh multiplier is 1, which is <= 64.
    same(
        "err24: fresh multiplier==1",
        p.c.findrep(0, 0, 0, 0),
        p.rust.findrep(0, 0, 0, 0),
    );
    // Exact boundary: multiplier == 64 -> `>` false.
    let p = Pair::fresh();
    same("err24: prime to 64", p.c.mul(64, 1), p.rust.mul(64, 1));
    same(
        "err24: findrep with multiplier == 0100",
        p.c.findrep(0, 0, 0, 0),
        p.rust.findrep(0, 0, 0, 0),
    );
    // One past: multiplier == 65 -> guard TRUE (contrast case).
    let p = Pair::fresh();
    same("err24: prime to 65", p.c.mul(65, 1), p.rust.mul(65, 1));
    same(
        "err24: findrep with multiplier == 65",
        p.c.findrep(0, 0, 0, 0),
        p.rust.findrep(0, 0, 0, 0),
    );
    // Negative multiplier is also <= 64.
    let mut r = Rng::new(0x0324);
    for _ in 0..60 {
        let p = Pair::fresh();
        let m = r.i32_in(-100000, -1);
        same("err24: prime negative", p.c.mul(m, 1), p.rust.mul(m, 1));
        same(
            &format!("err24: findrep with multiplier=={m}"),
            p.c.findrep(1, 1, 1, 1),
            p.rust.findrep(1, 1, 1, 1),
        );
    }
}

#[test]
fn err_25_findrep_multiplier_zero_latch() {
    // Once multiplier is 0 it can never leave 0: both_active stays false and
    // every later multiply/divide keeps it at 0.
    let p = Pair::fresh();
    let latched = p.c.mul(0, 5);
    same("err25: latch to zero", latched, p.rust.mul(0, 5));
    same("err25: multiplier is 0", latched, 0);
    let mut r = Rng::new(0x0325);
    for i in 0..200 {
        let (a, b, c, d) = (r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any());
        same(
            &format!("err25: findrep #{i} with multiplier==0"),
            p.c.findrep(a, b, c, d),
            p.rust.findrep(a, b, c, d),
        );
        let mut dv = r.i32_in(-9, 9);
        if dv == 0 {
            dv = 4;
        }
        let cv = p.c.div(a, dv);
        same(&format!("err25: div #{i}"), cv, p.rust.div(a, dv));
        same("err25: still zero", cv, 0);
    }
}

#[test]
fn err_26_findrep_accumulator_zero() {
    // accumulator == 0 -> has_accumulator == 0 -> both_active false.
    let p = Pair::fresh();
    same(
        "err26: fresh accumulator==0",
        p.c.findrep(0, 0, 0, 0),
        p.rust.findrep(0, 0, 0, 0),
    );
    // Drive accumulator back to exactly 0 then call findrep(0,0,0,0).
    let mut r = Rng::new(0x0326);
    for _ in 0..60 {
        let p = Pair::fresh();
        let v = r.i32_in(1, 100000);
        same("err26: add v", p.c.add(v, 0), p.rust.add(v, 0));
        same("err26: sub back to 0", p.c.sub(v, 0), p.rust.sub(v, 0));
        same(
            "err26: findrep with accumulator==0",
            p.c.findrep(0, 0, 0, 0),
            p.rust.findrep(0, 0, 0, 0),
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 27-29: signed-overflow UB paths (wrap identically in practice)
// ---------------------------------------------------------------------------

#[test]
fn err_27_accumulator_signed_overflow() {
    for (a, b) in [
        (i32::MAX, i32::MAX),
        (i32::MAX, 1),
        (i32::MIN, i32::MIN),
        (i32::MIN, -1),
        (i32::MAX, i32::MIN),
    ] {
        let p = Pair::fresh();
        same(
            &format!("err27: add({a},{b}) overflow"),
            p.c.add(a, b),
            p.rust.add(a, b),
        );
        same(
            &format!("err27: add({a},{b}) overflow again"),
            p.c.add(a, b),
            p.rust.add(a, b),
        );
        same(
            &format!("err27: sub({a},{b}) overflow"),
            p.c.sub(a, b),
            p.rust.sub(a, b),
        );
    }
    // Random walk that repeatedly overflows the accumulator.
    let p = Pair::fresh();
    let mut r = Rng::new(0x0327);
    for i in 0..1000 {
        let (a, b) = (r.i32_in(i32::MAX - 8, i32::MAX), r.i32_in(i32::MAX - 8, i32::MAX));
        same(&format!("err27: walk #{i}"), p.c.add(a, b), p.rust.add(a, b));
    }
}

#[test]
fn err_28_multiplier_signed_overflow() {
    for (a, b) in [
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MIN, -1),
        (65536, 65536),
        (-65536, 65536),
    ] {
        let p = Pair::fresh();
        same(
            &format!("err28: mul({a},{b}) overflow"),
            p.c.mul(a, b),
            p.rust.mul(a, b),
        );
        same(
            &format!("err28: mul({a},{b}) overflow again"),
            p.c.mul(a, b),
            p.rust.mul(a, b),
        );
    }
    let p = Pair::fresh();
    let mut r = Rng::new(0x0328);
    for i in 0..1000 {
        let (a, b) = (r.i32_any(), r.i32_any());
        same(&format!("err28: walk #{i}"), p.c.mul(a, b), p.rust.mul(a, b));
    }
}

#[test]
fn err_29_subtract_inner_overflow() {
    // `a - b` itself overflows before it is applied to accumulator.
    for (a, b) in [
        (i32::MIN, i32::MAX),
        (i32::MIN, 1),
        (i32::MAX, i32::MIN),
        (i32::MAX, -1),
        (0, i32::MIN),
    ] {
        let p = Pair::fresh();
        same(
            &format!("err29: sub({a},{b}) inner overflow"),
            p.c.sub(a, b),
            p.rust.sub(a, b),
        );
        same(
            &format!("err29: sub({a},{b}) again"),
            p.c.sub(a, b),
            p.rust.sub(a, b),
        );
        // And propagated into findrep.
        same(
            &format!("err29: findrep after sub({a},{b})"),
            p.c.findrep(a, b, a, b),
            p.rust.findrep(a, b, a, b),
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 30-32: process_octal_string formatting edge cases
// ---------------------------------------------------------------------------

#[test]
fn err_30_octal_negative_reinterpreted() {
    let p = Pair::fresh();
    let cv = p.c.octal(-1);
    same_bytes("err30: octal(-1)", &cv, &p.rust.octal(-1));
    let s = String::from_utf8_lossy(&cv[..cv.iter().position(|&b| b == 0).unwrap()]).to_string();
    assert_eq!(s, "Octal: 037777777777, Decimal: -1", "err30: exact C format");

    let mut r = Rng::new(0x0330);
    for _ in 0..500 {
        let v = r.i32_in(i32::MIN, -1);
        same_bytes(
            &format!("err30: octal({v})"),
            &p.c.octal(v),
            &p.rust.octal(v),
        );
    }
}

#[test]
fn err_31_octal_int_min_longest() {
    let p = Pair::fresh();
    let cv = p.c.octal(i32::MIN);
    same_bytes("err31: octal(INT_MIN)", &cv, &p.rust.octal(i32::MIN));
    let s = String::from_utf8_lossy(&cv[..cv.iter().position(|&b| b == 0).unwrap()]).to_string();
    assert_eq!(s, "Octal: 020000000000, Decimal: -2147483648");
    assert!(s.len() < 50, "err31: must still fit char buffer[50]");
}

#[test]
fn err_32_octal_zero_double_prefix() {
    let p = Pair::fresh();
    let cv = p.c.octal(0);
    same_bytes("err32: octal(0)", &cv, &p.rust.octal(0));
    let s = String::from_utf8_lossy(&cv[..cv.iter().position(|&b| b == 0).unwrap()]).to_string();
    assert_eq!(s, "Octal: 00, Decimal: 0", "err32: doubled 0 prefix");
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary boundaries required by Phase C, beyond the table:
// out-of-range "enum-like" ints, extreme lengths, one-past-range values.
// ---------------------------------------------------------------------------

#[test]
fn err_generic_out_of_range_int_arguments() {
    // This C API has no enums, but every `int` parameter accepts any 32-bit
    // value across the FFI boundary. Sweep the extremes through every entry
    // point that takes one, on fresh state, and require identical results.
    const X: &[i32] = &[
        i32::MIN,
        i32::MIN + 1,
        -0x8000_0000i64 as i32,
        -70000,
        -256,
        -255,
        -128,
        -127,
        -1,
        0,
        1,
        127,
        128,
        255,
        256,
        257,
        65535,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &v in X {
        let p = Pair::fresh();
        same(&format!("generic: normalize({v})"), p.c.normalize(v), p.rust.normalize(v));
        same_bytes(&format!("generic: octal({v})"), &p.c.octal(v), &p.rust.octal(v));
        let s = buf_from(b"Octal: 0123, Decimal: 83", 64);
        same_bytes(
            &format!("generic: replace(_, {v})"),
            &p.c.replace(&s, v),
            &p.rust.replace(&s, v),
        );
        same(&format!("generic: add({v},{v})"), p.c.add(v, v), p.rust.add(v, v));
        same(&format!("generic: mul({v},{v})"), p.c.mul(v, v), p.rust.mul(v, v));
        same(&format!("generic: sub({v},{v})"), p.c.sub(v, v), p.rust.sub(v, v));
        if v != 0 {
            // Skip only the SIGFPE case documented as ERRORS.md row U4.
            let p2 = Pair::fresh();
            same(&format!("generic: div(_,{v})"), p2.c.div(v, v), p2.rust.div(v, v));
        }
        same(
            &format!("generic: findrep({v},{v},{v},{v})"),
            p.c.findrep(v, v, v, v),
            p.rust.findrep(v, v, v, v),
        );
    }
}

#[test]
fn err_generic_zero_and_oversized_lengths() {
    let p = Pair::fresh();
    // Zero-length string.
    let empty = buf_from(b"", 4);
    same_bytes(
        "generic: replace on zero-length",
        &p.c.replace(&empty, b'a' as i32),
        &p.rust.replace(&empty, b'a' as i32),
    );
    // A long string (far longer than any internal buffer in lib.c) - only
    // find_and_replace_char reads caller-provided lengths, and it is bounded
    // by strlen, so this is well-defined.
    let long: Vec<u8> = (0..8192u32).map(|i| ((i % 251) + 1) as u8).collect();
    for ch in [1i32, 128, 251, 252, b'a' as i32] {
        let s = buf_from(&long, long.len() + 64);
        same_bytes(
            &format!("generic: replace on 8KiB string, ch={ch}"),
            &p.c.replace(&s, ch),
            &p.rust.replace(&s, ch),
        );
    }
    // Buffer exactly large enough for the longest process_octal_string output
    // (40 bytes + NUL); the harness uses 64, so also check the exact content
    // length matches between the two.
    for v in [0, i32::MIN, i32::MAX, -1] {
        let cv = p.c.octal(v);
        let rv = p.rust.octal(v);
        same_bytes(&format!("generic: octal({v}) full 64-byte buffer"), &cv, &rv);
        same(
            &format!("generic: octal({v}) strlen"),
            cv.iter().position(|&b| b == 0),
            rv.iter().position(|&b| b == 0),
        );
    }
}
