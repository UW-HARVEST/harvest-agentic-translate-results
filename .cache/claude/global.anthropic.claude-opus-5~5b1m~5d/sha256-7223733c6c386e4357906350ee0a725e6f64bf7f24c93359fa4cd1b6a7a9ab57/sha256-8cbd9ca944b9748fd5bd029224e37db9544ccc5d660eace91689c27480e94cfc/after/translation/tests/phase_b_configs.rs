//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares the returned `int` and the
//! stdout bytes. Property-style rows use a fixed PRNG seed so any failure is
//! reproducible.

mod harness;

use harness::*;

// ---------------------------------------------------------------------------
// Rows 1–7: start_ptr == NULL, stop_ptr == NULL (both defaults)
// ---------------------------------------------------------------------------

fn row01_both_null_empty_string() {
    // len == 0 -> start = 0, stop = 0, prints just "\n".
    assert_same_and("row01", &Call::new(b"", None, None), 0, b"\n");
}

fn row02_both_null_len_one() {
    for b in 1u16..=255 {
        let s = [b as u8];
        check("row02", &Call::new(s, None, None));
    }
}

fn row03_both_null_short_ascii() {
    let mut rng = Rng::new(0x0303_0303);
    for _ in 0..ITERS {
        let len = rng.range(2, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        check("row03", &Call::new(s, None, None));
    }
}

fn row04_both_null_long_string() {
    let mut rng = Rng::new(0x0404_0404);
    for _ in 0..ITERS {
        let len = rng.range(256, 1024);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        check("row04", &Call::new(s, None, None));
    }
}

fn row05_both_null_non_utf8_bytes() {
    let mut rng = Rng::new(0x0505_0505);
    for _ in 0..ITERS {
        let len = rng.range(1, 128);
        let s = gen_string(&mut rng, len, Flavor::AnyByte);
        check("row05", &Call::new(s, None, None));
    }
}

fn row06_both_null_format_specifier_payload() {
    // The payload is an *argument* to "%.*s", never a format string: the C must
    // print it verbatim and so must the Rust.
    for lit in [
        &b"%s"[..],
        b"%n",
        b"%d %d %d %d %d %d %d %d",
        b"%%",
        b"%1000000d",
        b"%.*s",
        b"100%% done",
    ] {
        check("row06", &Call::new(lit, None, None));
    }
    let mut rng = Rng::new(0x0606_0606);
    for _ in 0..ITERS {
        let len = rng.range(1, 96);
        let s = gen_string(&mut rng, len, Flavor::FormatSpecifiers);
        check("row06", &Call::new(s, None, None));
    }
}

fn row07_both_null_whitespace_payload() {
    let mut rng = Rng::new(0x0707_0707);
    for _ in 0..ITERS {
        let len = rng.range(1, 96);
        let s = gen_string(&mut rng, len, Flavor::Whitespace);
        check("row07", &Call::new(s, None, None));
    }
}

// ---------------------------------------------------------------------------
// Rows 8–10: start_ptr == NULL, stop_ptr != NULL (prefix slices)
// ---------------------------------------------------------------------------

fn row08_null_start_min_stop() {
    let mut rng = Rng::new(0x0808_0808);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        check("row08", &Call::new(s, None, Some(1)));
    }
}

fn row09_null_start_random_prefix() {
    let mut rng = Rng::new(0x0909_0909);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let stop = rng.range(1, len) as i32;
        check("row09", &Call::new(s, None, Some(stop)));
    }
}

fn row10_null_start_stop_at_len() {
    let mut rng = Rng::new(0x0A0A_0A0A);
    for _ in 0..ITERS {
        let len = rng.range(1, 96);
        let s = gen_string(&mut rng, len, Flavor::AnyByte);
        check("row10", &Call::new(s, None, Some(len as i32)));
    }
}

// ---------------------------------------------------------------------------
// Rows 11–15: start_ptr != NULL, stop_ptr == NULL (suffix slices)
// ---------------------------------------------------------------------------

fn row11_start_zero_null_stop() {
    let mut rng = Rng::new(0x0B0B_0B0B);
    for _ in 0..ITERS {
        let len = rng.range(0, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        check("row11", &Call::new(s, Some(0), None));
    }
}

fn row12_random_valid_start_null_stop() {
    let mut rng = Rng::new(0x0C0C_0C0C);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let start = rng.range(0, len) as i32;
        check("row12", &Call::new(s, Some(start), None));
    }
}

fn row13_start_at_len_null_stop() {
    // start == len is VALID (`len > len` is false); width 0, prints just "\n".
    let mut rng = Rng::new(0x0D0D_0D0D);
    assert_same_and("row13", &Call::new(b"", Some(0), None), 0, b"\n");
    for _ in 0..ITERS {
        let len = rng.range(0, 96);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        assert_same_and("row13", &Call::new(s, Some(len as i32), None), 0, b"\n");
    }
}

fn row14_start_at_len_minus_one_null_stop() {
    let mut rng = Rng::new(0x0E0E_0E0E);
    for _ in 0..ITERS {
        let len = rng.range(1, 96);
        let s = gen_string(&mut rng, len, Flavor::AnyByte);
        check("row14", &Call::new(s, Some(len as i32 - 1), None));
    }
}

fn row15_long_string_random_start_null_stop() {
    let mut rng = Rng::new(0x0F0F_0F0F);
    for _ in 0..ITERS {
        let len = rng.range(256, 1024);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let start = rng.range(0, len) as i32;
        check("row15", &Call::new(s, Some(start), None));
    }
}

// ---------------------------------------------------------------------------
// Rows 16–24: both bounds explicit
// ---------------------------------------------------------------------------

fn row16_full_range_both_explicit() {
    let mut rng = Rng::new(0x1010_1010);
    for _ in 0..ITERS {
        let len = rng.range(1, 128);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        check("row16", &Call::new(s, Some(0), Some(len as i32)));
    }
}

fn row17_width_one_slices() {
    let mut rng = Rng::new(0x1111_1111);
    for _ in 0..ITERS {
        let len = rng.range(1, 96);
        let s = gen_string(&mut rng, len, Flavor::AnyByte);
        let start = rng.range(0, len - 1) as i32;
        check("row17", &Call::new(s, Some(start), Some(start + 1)));
    }
}

fn row18_random_interior_slices() {
    let mut rng = Rng::new(0x1212_1212);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let start = rng.range(0, len - 1);
        let stop = rng.range(start + 1, len);
        check("row18", &Call::new(s, Some(start as i32), Some(stop as i32)));
    }
}

fn row19_stop_at_len_random_start() {
    let mut rng = Rng::new(0x1313_1313);
    for _ in 0..ITERS {
        let len = rng.range(1, 96);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let start = rng.range(0, len - 1) as i32;
        check("row19", &Call::new(s, Some(start), Some(len as i32)));
    }
}

fn row20_last_character_both_boundaries() {
    let mut rng = Rng::new(0x1414_1414);
    for _ in 0..ITERS {
        let len = rng.range(1, 96);
        let s = gen_string(&mut rng, len, Flavor::AnyByte);
        check("row20", &Call::new(s, Some(len as i32 - 1), Some(len as i32)));
    }
}

fn row21_long_string_random_pair() {
    let mut rng = Rng::new(0x1515_1515);
    for _ in 0..ITERS {
        let len = rng.range(256, 1024);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        let start = rng.range(0, len - 1);
        let stop = rng.range(start + 1, len);
        check("row21", &Call::new(s, Some(start as i32), Some(stop as i32)));
    }
}

fn row22_non_utf8_random_pair() {
    let mut rng = Rng::new(0x1616_1616);
    for _ in 0..ITERS {
        let len = rng.range(1, 128);
        // Force real multi-byte UTF-8 sequences so slices can split them.
        let mut s = gen_string(&mut rng, len, Flavor::AnyByte);
        s.extend_from_slice("héllo→🌍".as_bytes());
        let n = s.len();
        let start = rng.range(0, n - 1);
        let stop = rng.range(start + 1, n);
        check("row22", &Call::new(s, Some(start as i32), Some(stop as i32)));
    }
}

fn row23_format_payload_random_pair() {
    let mut rng = Rng::new(0x1717_1717);
    for _ in 0..ITERS {
        let len = rng.range(2, 96);
        let s = gen_string(&mut rng, len, Flavor::FormatSpecifiers);
        let n = s.len();
        let start = rng.range(0, n - 1);
        let stop = rng.range(start + 1, n);
        check("row23", &Call::new(s, Some(start as i32), Some(stop as i32)));
    }
}

fn row24_single_char_only_valid_slice() {
    for b in 1u16..=255 {
        check("row24", &Call::new([b as u8], Some(0), Some(1)));
    }
}

// ---------------------------------------------------------------------------
// Row 25: aliased bound pointers (start_ptr == stop_ptr)
// ---------------------------------------------------------------------------

fn row25_aliased_bound_pointers() {
    use std::ffi::{c_char, c_int};

    fn invoke_aliased(f: &SliceFn, s: &[u8], v: c_int) -> c_int {
        let mut buf = s.to_vec();
        buf.push(0);
        let mut val: c_int = v;
        let p = &mut val as *mut c_int;
        // SAFETY: same live `int` handed to both bound parameters — legal C.
        unsafe { f(buf.as_mut_ptr() as *mut c_char, p, p) }
    }

    let _g = io_guard();
    let cf = c_slice();
    let rf = rust_slice();

    let mut rng = Rng::new(0x1919_1919);
    for _ in 0..ITERS {
        let len = rng.range(0, 48);
        let s = gen_string(&mut rng, len, Flavor::Ascii);
        // Mix in-range and out-of-range values.
        let v: c_int = if rng.bool_p(2, 3) {
            rng.range(0, len) as c_int
        } else {
            rng.i32_any()
        };

        let (c_ret, c_out) = capture_stdout(|| invoke_aliased(&cf, &s, v));
        let (r_ret, r_out) = capture_stdout(|| invoke_aliased(&rf, &s, v));
        assert_eq!(
            (c_ret, &c_out),
            (r_ret, &r_out),
            "[row25] aliased pointers diverged for len={len} v={v}\n  C: {:?}\n  R: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
        // With start == stop the signed check must always reject (when the
        // unsigned bound check does not fire first).
        let expect: &[u8] = if (v as i64 as u64) > len as u64 {
            ERR_START
        } else {
            ERR_STOP_ORDER
        };
        assert_eq!(c_ret, 1, "[row25] expected rejection for len={len} v={v}");
        assert_eq!(
            c_out,
            expect,
            "[row25] wrong diagnostic for len={len} v={v}: {:?}",
            String::from_utf8_lossy(&c_out)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 26: repeated / interleaved calls — no hidden state
// ---------------------------------------------------------------------------

fn row26_repeated_interleaved_calls_no_hidden_state() {
    use std::ffi::{c_char, c_int};

    // A scripted sequence mixing successes and all three error kinds, executed
    // as ONE batch inside a single stdout capture, so any per-call buffering or
    // residual-state difference shows up in the concatenated bytes.
    let script: Vec<Call> = vec![
        Call::new(b"hello world", None, None),
        Call::new(b"hello world", Some(6), None),
        Call::new(b"hello world", None, Some(5)),
        Call::new(b"hello world", Some(99), None),
        Call::new(b"hello world", None, Some(99)),
        Call::new(b"hello world", Some(5), Some(2)),
        Call::new(b"hello world", Some(3), Some(3)),
        Call::new(b"", None, None),
        Call::new(b"", Some(0), None),
        Call::new(b"", None, Some(0)),
        Call::new(b"x", Some(0), Some(1)),
        Call::new(b"hello world", Some(-1), None),
        Call::new(b"hello world", None, Some(-1)),
        Call::new(b"abc", Some(0), Some(3)),
    ];

    fn run_batch(f: &SliceFn, script: &[Call]) -> Vec<c_int> {
        script
            .iter()
            .map(|call| {
                let mut buf = call.s.clone();
                buf.push(0);
                let mut sv: c_int = call.start.unwrap_or(0);
                let mut pv: c_int = call.stop.unwrap_or(0);
                let sp = if call.start.is_some() {
                    &mut sv as *mut c_int
                } else {
                    std::ptr::null_mut()
                };
                let pp = if call.stop.is_some() {
                    &mut pv as *mut c_int
                } else {
                    std::ptr::null_mut()
                };
                // SAFETY: NUL-terminated buffer, live-or-null bound pointers.
                unsafe { f(buf.as_mut_ptr() as *mut c_char, sp, pp) }
            })
            .collect()
    }

    let _g = io_guard();
    let cf = c_slice();
    let rf = rust_slice();

    let (c_rets, c_out) = capture_stdout(|| run_batch(&cf, &script));
    let (r_rets, r_out) = capture_stdout(|| run_batch(&rf, &script));

    assert_eq!(c_rets, r_rets, "[row26] return-code sequence diverged");
    assert_eq!(
        c_out,
        r_out,
        "[row26] batched stdout diverged\n  C: {:?}\n  R: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );

    // Pin the absolute expectation against the transcribed model.
    let mut want_out = Vec::new();
    let mut want_rets = Vec::new();
    for call in &script {
        let (r, o) = model(call);
        want_rets.push(r);
        want_out.extend_from_slice(&o);
    }
    assert_eq!(c_rets, want_rets, "[row26] return codes differ from model");
    assert_eq!(
        c_out,
        want_out,
        "[row26] stdout differs from model\n  got:  {:?}\n  want: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&want_out)
    );

    // Idempotence: running the batch again must reproduce the same bytes.
    let (c_rets2, c_out2) = capture_stdout(|| run_batch(&cf, &script));
    let (r_rets2, r_out2) = capture_stdout(|| run_batch(&rf, &script));
    assert_eq!(c_rets, c_rets2);
    assert_eq!(c_out, c_out2);
    assert_eq!(r_rets, r_rets2);
    assert_eq!(r_out, r_out2);
}

// ---------------------------------------------------------------------------
// Row 27: full-domain fuzz (valid x invalid cross-product)
// ---------------------------------------------------------------------------

fn row27_full_domain_fuzz() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    let flavors = [
        Flavor::Ascii,
        Flavor::AnyByte,
        Flavor::FormatSpecifiers,
        Flavor::Whitespace,
    ];

    let mut ok = 0usize;
    let mut err_start = 0usize;
    let mut err_stop_off = 0usize;
    let mut err_order = 0usize;

    for _ in 0..4000 {
        let len = rng.range(0, 80);
        let flavor = flavors[(rng.next_u64() % 4) as usize];
        let s = gen_string(&mut rng, len, flavor);
        let n = s.len();

        // Each bound: null with p = 1/3, otherwise a value from a mix of
        // in-range, near-boundary and wholly arbitrary i32s.
        let pick = |rng: &mut Rng| -> Option<i32> {
            if rng.bool_p(1, 3) {
                return None;
            }
            Some(match rng.next_u64() % 8 {
                0 | 1 | 2 => rng.range(0, n) as i32,             // in range
                3 => n as i32 + 1,                               // one past
                4 => -(rng.range(1, 4) as i32),                  // small negative
                5 => i32::MAX - rng.range(0, 2) as i32,          // extreme high
                6 => i32::MIN + rng.range(0, 2) as i32,          // extreme low
                _ => rng.i32_any(),                              // anything
            })
        };
        let start = pick(&mut rng);
        let stop = pick(&mut rng);

        let call = Call::new(s, start, stop);
        let (ret, out) = model(&call);
        match (ret, out.as_slice()) {
            (0, _) => ok += 1,
            (_, o) if o == ERR_START => err_start += 1,
            (_, o) if o == ERR_STOP_OFF => err_stop_off += 1,
            (_, o) if o == ERR_STOP_ORDER => err_order += 1,
            _ => unreachable!(),
        }
        assert_same_and("row27", &call, ret, &out);
    }

    // Make sure the fuzz actually reached every branch of the C function.
    assert!(ok > 100, "fuzz never hit the success path enough: {ok}");
    assert!(err_start > 50, "fuzz under-covered the start error: {err_start}");
    assert!(
        err_stop_off > 50,
        "fuzz under-covered the stop-off-end error: {err_stop_off}"
    );
    assert!(
        err_order > 50,
        "fuzz under-covered the stop-order error: {err_order}"
    );
}

// ---------------------------------------------------------------------------
// Sequential runner (`harness = false`): fd 1 is redirected around every
// library call, so the cases must not run concurrently.
// ---------------------------------------------------------------------------

fn main() {
    harness::run_all(
        "phase_b_configs",
        &[
            ("row01_both_null_empty_string", row01_both_null_empty_string as fn()),
            ("row02_both_null_len_one", row02_both_null_len_one as fn()),
            ("row03_both_null_short_ascii", row03_both_null_short_ascii as fn()),
            ("row04_both_null_long_string", row04_both_null_long_string as fn()),
            ("row05_both_null_non_utf8_bytes", row05_both_null_non_utf8_bytes as fn()),
            ("row06_both_null_format_specifier_payload", row06_both_null_format_specifier_payload as fn()),
            ("row07_both_null_whitespace_payload", row07_both_null_whitespace_payload as fn()),
            ("row08_null_start_min_stop", row08_null_start_min_stop as fn()),
            ("row09_null_start_random_prefix", row09_null_start_random_prefix as fn()),
            ("row10_null_start_stop_at_len", row10_null_start_stop_at_len as fn()),
            ("row11_start_zero_null_stop", row11_start_zero_null_stop as fn()),
            ("row12_random_valid_start_null_stop", row12_random_valid_start_null_stop as fn()),
            ("row13_start_at_len_null_stop", row13_start_at_len_null_stop as fn()),
            ("row14_start_at_len_minus_one_null_stop", row14_start_at_len_minus_one_null_stop as fn()),
            ("row15_long_string_random_start_null_stop", row15_long_string_random_start_null_stop as fn()),
            ("row16_full_range_both_explicit", row16_full_range_both_explicit as fn()),
            ("row17_width_one_slices", row17_width_one_slices as fn()),
            ("row18_random_interior_slices", row18_random_interior_slices as fn()),
            ("row19_stop_at_len_random_start", row19_stop_at_len_random_start as fn()),
            ("row20_last_character_both_boundaries", row20_last_character_both_boundaries as fn()),
            ("row21_long_string_random_pair", row21_long_string_random_pair as fn()),
            ("row22_non_utf8_random_pair", row22_non_utf8_random_pair as fn()),
            ("row23_format_payload_random_pair", row23_format_payload_random_pair as fn()),
            ("row24_single_char_only_valid_slice", row24_single_char_only_valid_slice as fn()),
            ("row25_aliased_bound_pointers", row25_aliased_bound_pointers as fn()),
            ("row26_repeated_interleaved_calls_no_hidden_state", row26_repeated_interleaved_calls_no_hidden_state as fn()),
            ("row27_full_domain_fuzz", row27_full_domain_fuzz as fn()),
        ],
    );
}
