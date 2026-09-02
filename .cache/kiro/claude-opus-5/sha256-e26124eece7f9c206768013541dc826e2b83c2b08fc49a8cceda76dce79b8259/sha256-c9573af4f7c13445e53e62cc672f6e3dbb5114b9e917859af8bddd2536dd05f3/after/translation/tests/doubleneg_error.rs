//! Phase C — `ERRORS.md` rows 21-24: `doubleneg`'s rejection branches.
//!
//! A SINGLE `#[test]` in its own binary, for the same fd-1 reason as
//! `tests/doubleneg_valid.rs`.

mod harness;

use std::ffi::{c_char, c_int};

use harness::{Rng, apis, diff_doubleneg};

#[test]
fn doubleneg_error_paths() {
    let p = apis();

    // -----------------------------------------------------------------------
    // Rows 21 & 22 — the "not found" branches.
    //
    // `doubleneg` always calls `create_numeric_buffer(buffer, 256, param1)`,
    // which stores `(char)((param1 + 7*i) % 256)`. Because C's `%` keeps the
    // value congruent mod 256 and 7 is invertible mod 256, those 256 bytes are
    // a permutation of ALL 256 byte values for EVERY `param1` (including the
    // signed-overflow cases, since 256 divides 2^32). Therefore `memchr` over
    // the 256-byte buffer can never miss: `pos < 0` (line 112 false) and
    // `direct_search == NULL` (line 121 false) are UNREACHABLE from
    // `doubleneg`.
    //
    // The differential requirement is that BOTH implementations agree on that,
    // i.e. neither ever takes the branch the other skips. Verified directly on
    // the generated buffers below, and implied by the byte-identical stdout
    // (the "Value %d not found" line would appear in one and not the other).
    // -----------------------------------------------------------------------
    let mut rng = Rng::new(0x2122);
    let mut seeds: Vec<c_int> = vec![0, 1, -1, 42, 100, 43, i32::MIN, i32::MAX, 255, 256, -256];
    for _ in 0..3000 {
        seeds.push(rng.spicy_i32());
    }
    for seed in seeds {
        let mut bc = vec![0i8; 256];
        let mut br = vec![0i8; 256];
        unsafe {
            (p.c.create_numeric_buffer)(bc.as_mut_ptr() as *mut c_char, 256, seed);
            (p.rust.create_numeric_buffer)(br.as_mut_ptr() as *mut c_char, 256, seed);
        }
        assert_eq!(bc, br, "rows21/22: buffer differs for seed={seed}");
        for probe in 0..256i32 {
            let c = unsafe { (p.c.find_value_in_buffer)(bc.as_ptr() as *const c_char, 256, probe) };
            let r =
                unsafe { (p.rust.find_value_in_buffer)(br.as_ptr() as *const c_char, 256, probe) };
            assert_eq!(c, r, "rows21/22: seed={seed} probe={probe}");
            assert!(
                c >= 0,
                "rows21/22: byte {probe} missing from C's buffer (seed={seed}) — the \
                 not-found branch IS reachable, revisit ERRORS.md"
            );
        }
    }

    // The "not found" / "NULL" branches ARE reachable in the underlying
    // primitive; exercise them there so the rejection itself is covered
    // differentially (a shorter buffer cannot contain every byte).
    for size in [1usize, 2, 17, 128, 255] {
        for seed in [0, 1, -1, 42, i32::MIN, i32::MAX] {
            let mut bc = vec![0i8; size];
            let mut br = vec![0i8; size];
            unsafe {
                (p.c.create_numeric_buffer)(bc.as_mut_ptr() as *mut c_char, size as c_int, seed);
                (p.rust.create_numeric_buffer)(br.as_mut_ptr() as *mut c_char, size as c_int, seed);
            }
            assert_eq!(bc, br, "rows21/22 primitive: size={size} seed={seed}");
            let mut misses = 0;
            for probe in 0..256i32 {
                let c =
                    unsafe { (p.c.find_value_in_buffer)(bc.as_ptr() as *const c_char, size, probe) };
                let r = unsafe {
                    (p.rust.find_value_in_buffer)(br.as_ptr() as *const c_char, size, probe)
                };
                assert_eq!(c, r, "rows21/22 primitive: size={size} probe={probe}");
                if c < 0 {
                    misses += 1;
                }
            }
            assert!(
                misses > 0,
                "rows21/22 primitive: expected the -1 sentinel for size={size}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Row 23 — param2 == 0: feeds `b == 0` into `calculate_with_doubles` (so
    // the division is skipped) and makes every `search_byte` in the combined
    // loop equal `param1 % 256`.
    // -----------------------------------------------------------------------
    let mut rng = Rng::new(0x2300);
    let mut p2zero: Vec<[c_int; 4]> = vec![
        [0, 0, 0, 0],
        [1, 0, 0, 0],
        [-1, 0, 0, 0],
        [42, 0, 42, 42],
        [i32::MAX, 0, i32::MAX, i32::MAX],
        [i32::MIN, 0, i32::MIN, i32::MIN],
        [100, 0, -9, 7],
        [-100, 0, 9, -7],
    ];
    for _ in 0..60 {
        p2zero.push([rng.spicy_i32(), 0, rng.spicy_i32(), rng.spicy_i32()]);
    }
    for [a, b, c, d] in p2zero {
        diff_doubleneg(a, b, c, d, "row23 param2 == 0");
    }

    // -----------------------------------------------------------------------
    // Row 24 — parameter extremes: signed overflow in `param1 + i*param2`,
    // truncating `% 256` on negatives, `% 1000` on INT_MIN.
    // -----------------------------------------------------------------------
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in &extremes {
        for &b in &extremes {
            diff_doubleneg(a, b, i32::MIN, i32::MAX, "row24 extremes (c=MIN, d=MAX)");
            diff_doubleneg(a, b, i32::MAX, i32::MIN, "row24 extremes (c=MAX, d=MIN)");
        }
    }
    for &c in &extremes {
        for &d in &extremes {
            diff_doubleneg(i32::MIN, i32::MAX, c, d, "row24 extremes (a=MIN, b=MAX)");
        }
    }

    // -----------------------------------------------------------------------
    // Generic FFI boundary sweep for `doubleneg`: every parameter position
    // driven with the same probe set used in `err_int_parameter_sweep`.
    // -----------------------------------------------------------------------
    const PROBES: [c_int; 13] = [
        i32::MIN,
        i32::MIN + 1,
        -1000000,
        -256,
        -255,
        -1,
        0,
        1,
        42,
        255,
        256,
        1000000,
        i32::MAX,
    ];
    for &v in &PROBES {
        diff_doubleneg(v, 3, 5, 7, "sweep param1");
        diff_doubleneg(3, v, 5, 7, "sweep param2");
        diff_doubleneg(3, 5, v, 7, "sweep param3");
        diff_doubleneg(3, 5, 7, v, "sweep param4");
    }
}
