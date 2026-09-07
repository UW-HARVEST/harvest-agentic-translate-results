// Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//
// Phase B = `phase_b_*`, one test per row of CONFIGS.md.
// Phase C = `phase_c_*`, one test per row of ERRORS.md.

mod harness;

use harness::{assert_same, c_api, capture, rust_api, Rng, SEED};
use std::ffi::c_char;

const GOOD_LINE: &[u8] = b"helperGood1 string\n";
const BAD_TEXT: &[u8] = b"helperBad string";

// ===========================================================================
// Phase B — valid-path differential tests (CONFIGS.md)
// ===========================================================================

// --- Row 1: printLine, non-NULL, length 0 ---------------------------------
#[test]
fn phase_b_row01_printline_empty() {
    let out = assert_same("row01 empty string", |api| api.print_line(b""));
    assert_eq!(out, b"\n", "empty string must yield exactly one newline");
}

// --- Row 2: printLine, length 1, randomised over non-NUL bytes ------------
#[test]
fn phase_b_row02_printline_len1_random() {
    let mut rng = Rng::new(SEED ^ 0x02);
    for i in 0..512 {
        let b = rng.nonnul_byte();
        let out = assert_same(&format!("row02 #{i} byte={b:#04x}"), |api| api.print_line(&[b]));
        assert_eq!(out, vec![b, b'\n']);
    }
}

// --- Row 3: printLine, random printable ASCII, len 1..=64 -----------------
#[test]
fn phase_b_row03_printline_random_ascii() {
    let mut rng = Rng::new(SEED ^ 0x03);
    for i in 0..512 {
        let len = rng.range_incl(1, 64);
        let payload = rng.printable_bytes(len);
        let out = assert_same(&format!("row03 #{i} len={len}"), |api| api.print_line(&payload));
        let mut want = payload.clone();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

// --- Row 4: printLine, full 0x01..=0xFF range, len 1..=256 ----------------
#[test]
fn phase_b_row04_printline_random_binary() {
    let mut rng = Rng::new(SEED ^ 0x04);
    for i in 0..512 {
        let len = rng.range_incl(1, 256);
        let payload = rng.nonnul_bytes(len);
        let out = assert_same(&format!("row04 #{i} len={len}"), |api| api.print_line(&payload));
        let mut want = payload.clone();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

// --- Row 5: printLine, exhaustive single-byte sweep 0x01..=0xFF ------------
#[test]
fn phase_b_row05_printline_all_single_bytes() {
    for b in 1u8..=255 {
        let out = assert_same(&format!("row05 byte={b:#04x}"), |api| api.print_line(&[b]));
        assert_eq!(out, vec![b, b'\n'], "single byte {b:#04x} round-trip");
    }
}

// --- Row 6: printLine, stdio buffer boundary lengths ----------------------
#[test]
fn phase_b_row06_printline_buffer_boundaries() {
    let mut rng = Rng::new(SEED ^ 0x06);
    for len in [4095usize, 4096, 4097, 8191, 8192, 8193] {
        let payload = rng.nonnul_bytes(len);
        let out = assert_same(&format!("row06 len={len}"), |api| api.print_line(&payload));
        assert_eq!(out.len(), len + 1);
        assert_eq!(&out[..len], &payload[..]);
        assert_eq!(out[len], b'\n');
    }
}

// --- Row 7: printLine, large payloads -------------------------------------
#[test]
fn phase_b_row07_printline_large() {
    let mut rng = Rng::new(SEED ^ 0x07);
    for len in [64 * 1024usize, 1024 * 1024] {
        let payload = rng.nonnul_bytes(len);
        let out = assert_same(&format!("row07 len={len}"), |api| api.print_line(&payload));
        assert_eq!(out.len(), len + 1);
        assert_eq!(&out[..len], &payload[..]);
    }
}

// --- Row 8: printLine, embedded newlines ----------------------------------
#[test]
fn phase_b_row08_printline_embedded_newlines() {
    let mut rng = Rng::new(SEED ^ 0x08);
    for i in 0..256 {
        let len = rng.range_incl(1, 128);
        let mut payload = rng.printable_bytes(len);
        let n_newlines = rng.range_incl(1, 8);
        for _ in 0..n_newlines {
            let pos = rng.below(payload.len());
            payload[pos] = b'\n';
        }
        let out = assert_same(&format!("row08 #{i} len={len}"), |api| api.print_line(&payload));
        let mut want = payload.clone();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

// --- Row 9: printLine, format-specifier bytes in the payload --------------
#[test]
fn phase_b_row09_printline_format_specifiers() {
    let specs: [&[u8]; 8] = [b"%s", b"%d", b"%n", b"%%", b"%99999d", b"%p", b"%1$s", b"%.*s"];
    // Fixed, hand-built shapes first.
    for s in specs {
        let out = assert_same(&format!("row09 fixed {:?}", String::from_utf8_lossy(s)), |api| {
            api.print_line(s)
        });
        let mut want = s.to_vec();
        want.push(b'\n');
        assert_eq!(out, want, "format specifier must be emitted verbatim");
    }
    // Then randomised placement inside random padding.
    let mut rng = Rng::new(SEED ^ 0x09);
    for i in 0..256 {
        let n_pad = rng.range_incl(0, 48);
        let mut payload = rng.printable_bytes(n_pad);
        let n = rng.range_incl(1, 4);
        for _ in 0..n {
            let s = specs[rng.below(specs.len())];
            let pos = rng.range_incl(0, payload.len());
            let tail = payload.split_off(pos);
            payload.extend_from_slice(s);
            payload.extend_from_slice(&tail);
        }
        let out = assert_same(&format!("row09 #{i}"), |api| api.print_line(&payload));
        let mut want = payload.clone();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

// --- Row 10: printLine, interior NUL (truncation) -------------------------
#[test]
fn phase_b_row10_printline_interior_nul() {
    let mut rng = Rng::new(SEED ^ 0x0A);
    for i in 0..256 {
        let n_prefix = rng.range_incl(0, 32);
        let prefix = rng.nonnul_bytes(n_prefix);
        let n_suffix = rng.range_incl(1, 32);
        let suffix = rng.nonnul_bytes(n_suffix);
        let mut payload = prefix.clone();
        payload.push(0);
        payload.extend_from_slice(&suffix);
        let out = assert_same(&format!("row10 #{i} prefix={}", prefix.len()), |api| {
            api.print_line(&payload)
        });
        let mut want = prefix.clone();
        want.push(b'\n');
        assert_eq!(out, want, "output must stop at the first interior NUL");
    }
}

// --- Row 11: printLine with the library's own literals -------------------
#[test]
fn phase_b_row11_printline_own_literals() {
    let out = assert_same("row11 helperBad literal", |api| api.print_line(BAD_TEXT));
    assert_eq!(out, b"helperBad string\n");
    let out = assert_same("row11 helperGood1 literal", |api| {
        api.print_line(b"helperGood1 string")
    });
    assert_eq!(out, GOOD_LINE);
}

// --- Row 12: printLine, uniform / alternating byte patterns --------------
#[test]
fn phase_b_row12_printline_patterns() {
    for len in 1usize..=32 {
        for pat in [0xFFu8, 0x01, 0x80, 0x7F] {
            let payload = vec![pat; len];
            let out = assert_same(&format!("row12 pat={pat:#04x} len={len}"), |api| {
                api.print_line(&payload)
            });
            assert_eq!(out.len(), len + 1);
        }
        let alt: Vec<u8> = (0..len).map(|i| if i % 2 == 0 { 0x01 } else { 0xFF }).collect();
        let out = assert_same(&format!("row12 alt len={len}"), |api| api.print_line(&alt));
        assert_eq!(out.len(), len + 1);
    }
}

// --- Row 13: good(), single call -----------------------------------------
#[test]
fn phase_b_row13_good_single() {
    let out = assert_same("row13 good once", |api| api.good());
    assert_eq!(out, GOOD_LINE, "good() must print the static .data string");
}

// --- Row 14: good(), 100 calls (idempotence) ------------------------------
#[test]
fn phase_b_row14_good_many() {
    let out = assert_same("row14 good x100", |api| {
        for _ in 0..100 {
            api.good();
        }
    });
    let want: Vec<u8> = GOOD_LINE.repeat(100);
    assert_eq!(out, want, "good() must be idempotent across repeated calls");
}

// --- Row 15: bad(), single call ------------------------------------------
#[test]
fn phase_b_row15_bad_single() {
    let out = assert_same("row15 bad once", |api| api.bad());
    // Whatever the C does, the Rust must match. Record and pin the C behaviour.
    assert!(
        !out.windows(BAD_TEXT.len()).any(|w| w == BAD_TEXT),
        "reference C build's helperBad() returns NULL, so no text is printed; got {}",
        harness::show(&out)
    );
    assert!(out.is_empty(), "expected silence from bad(), got {}", harness::show(&out));
}

// --- Row 16: bad(), 100 calls -------------------------------------------
#[test]
fn phase_b_row16_bad_many() {
    let out = assert_same("row16 bad x100", |api| {
        for _ in 0..100 {
            api.bad();
        }
    });
    assert!(out.is_empty(), "bad() must stay silent, got {}", harness::show(&out));
}

// --- Row 17: driver(1) ---------------------------------------------------
#[test]
fn phase_b_row17_driver_true() {
    let out = assert_same("row17 driver(1)", |api| api.driver(1));
    assert_eq!(out, GOOD_LINE);
}

// --- Row 18: driver(0) ---------------------------------------------------
#[test]
fn phase_b_row18_driver_false() {
    let out = assert_same("row18 driver(0)", |api| api.driver(0));
    assert!(out.is_empty(), "driver(0) routes to bad(); got {}", harness::show(&out));
}

// --- Row 19: driver, 1024 random non-zero i32 ---------------------------
#[test]
fn phase_b_row19_driver_random_nonzero() {
    let mut rng = Rng::new(SEED ^ 0x13);
    for i in 0..1024 {
        let mut v = rng.next_i32();
        while v == 0 {
            v = rng.next_i32();
        }
        let out = assert_same(&format!("row19 #{i} useGood={v}"), |api| api.driver(v));
        assert_eq!(out, GOOD_LINE, "any non-zero int must select good(): {v}");
    }
}

// --- Row 20: driver, boundary values -----------------------------------
#[test]
fn phase_b_row20_driver_boundaries() {
    let cases: [i32; 10] = [
        -1,
        2,
        -2,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0x8000_0000u32 as i32,
        0x0001_0000,
        0xFFFF_0000u32 as i32,
    ];
    for v in cases {
        let out = assert_same(&format!("row20 useGood={v}"), |api| api.driver(v));
        assert_eq!(out, GOOD_LINE, "non-zero boundary value {v} must select good()");
    }
}

// --- Row 21: driver, random sequence of 256 mixed values ---------------
#[test]
fn phase_b_row21_driver_random_sequence() {
    let mut rng = Rng::new(SEED ^ 0x15);
    let seq: Vec<i32> = (0..256)
        .map(|_| if rng.next_u64() % 3 == 0 { 0 } else { rng.next_i32() })
        .collect();
    let out = assert_same("row21 driver sequence", |api| {
        for &v in &seq {
            api.driver(v);
        }
    });
    let mut want = Vec::new();
    for &v in &seq {
        if v != 0 {
            want.extend_from_slice(GOOD_LINE);
        }
    }
    assert_eq!(out, want, "composed driver sequence must not leak state");
}

// --- Row 22: random interleaving of all four entry points ---------------
#[test]
fn phase_b_row22_mixed_pipeline() {
    let mut rng = Rng::new(SEED ^ 0x16);
    #[derive(Clone, Debug)]
    enum Op {
        Driver(i32),
        Good,
        Bad,
        Print(Vec<u8>),
        PrintNull,
    }
    let ops: Vec<Op> = (0..256)
        .map(|_| match rng.below(5) {
            0 => Op::Driver(if rng.next_u64() % 2 == 0 { 0 } else { rng.next_i32() }),
            1 => Op::Good,
            2 => Op::Bad,
            3 => {
                let len = rng.range_incl(0, 40);
                Op::Print(rng.nonnul_bytes(len))
            }
            _ => Op::PrintNull,
        })
        .collect();
    let out = assert_same("row22 mixed pipeline", |api| {
        for op in &ops {
            match op {
                Op::Driver(v) => api.driver(*v),
                Op::Good => api.good(),
                Op::Bad => api.bad(),
                Op::Print(p) => api.print_line(p),
                Op::PrintNull => unsafe { api.print_line_raw(std::ptr::null()) },
            }
        }
    });
    let mut want = Vec::new();
    for op in &ops {
        match op {
            Op::Driver(v) if *v != 0 => want.extend_from_slice(GOOD_LINE),
            Op::Driver(_) => {}
            Op::Good => want.extend_from_slice(GOOD_LINE),
            Op::Bad => {}
            Op::Print(p) => {
                let stop = p.iter().position(|&b| b == 0).unwrap_or(p.len());
                want.extend_from_slice(&p[..stop]);
                want.push(b'\n');
            }
            Op::PrintNull => {}
        }
    }
    assert_eq!(out, want, "full composed pipeline must match the C model");
}

// --- Row 23: ordering — static buffer must not be mutated --------------
#[test]
fn phase_b_row23_ordering_static_stability() {
    let mut rng = Rng::new(SEED ^ 0x17);
    for i in 0..128 {
        let n = rng.range_incl(1, 40);
        let payload = rng.nonnul_bytes(n);
        let out = assert_same(&format!("row23 #{i}"), |api| {
            api.good();
            api.print_line(&payload);
            api.bad();
            api.driver(0);
            api.driver(1);
            api.good();
        });
        let mut want = Vec::new();
        want.extend_from_slice(GOOD_LINE);
        want.extend_from_slice(&payload);
        want.push(b'\n');
        want.extend_from_slice(GOOD_LINE);
        want.extend_from_slice(GOOD_LINE);
        assert_eq!(out, want, ".data string must be unchanged by neighbouring calls");
    }
}

// --- Row 24: dlopen only, no call --------------------------------------
#[test]
fn phase_b_row24_dlopen_is_silent() {
    let c_out = capture(|| {
        let _ = c_api();
    });
    let rust_out = capture(|| {
        let _ = rust_api();
    });
    assert_eq!(c_out, rust_out);
    assert!(c_out.is_empty(), "loading the library must not print anything");
}

// ===========================================================================
// Phase C — error-path differential tests (ERRORS.md)
// ===========================================================================

// --- ERRORS row 1 & 4: printLine(NULL) ---------------------------------
#[test]
fn phase_c_row01_row04_printline_null() {
    let out = assert_same("errors row1/4 printLine(NULL)", |api| unsafe {
        api.print_line_raw(std::ptr::null())
    });
    assert!(out.is_empty(), "printLine(NULL) must write nothing, got {}", harness::show(&out));

    // Also repeated, and interleaved with valid calls, to prove the guard is
    // stateless in both implementations.
    let out = assert_same("errors row1/4 printLine(NULL) x64", |api| unsafe {
        for _ in 0..64 {
            api.print_line_raw(std::ptr::null());
        }
    });
    assert!(out.is_empty());

    let out = assert_same("errors row1/4 NULL between valid", |api| unsafe {
        api.print_line(b"a");
        api.print_line_raw(std::ptr::null());
        api.print_line(b"b");
    });
    assert_eq!(out, b"a\nb\n");
}

// --- ERRORS row 2: bad() / CWE-562 dangling pointer -------------------
#[test]
fn phase_c_row02_bad_dangling_pointer() {
    let out = assert_same("errors row2 bad()", |api| api.bad());
    assert!(
        out.is_empty(),
        "the reference C build returns NULL from helperBad(), so bad() is silent; got {}",
        harness::show(&out)
    );
    assert!(
        !out.windows(BAD_TEXT.len()).any(|w| w == BAD_TEXT),
        "\"helperBad string\" must never reach the output"
    );
}

// --- ERRORS row 3: driver(0) reaches the defective path ---------------
#[test]
fn phase_c_row03_driver_zero() {
    let out = assert_same("errors row3 driver(0)", |api| api.driver(0));
    assert!(out.is_empty());
    // 0 is the ONLY value that takes this path.
    let out = assert_same("errors row3 driver(0) x64", |api| {
        for _ in 0..64 {
            api.driver(0);
        }
    });
    assert!(out.is_empty());
}

// --- ERRORS row 5: zero length is ACCEPTED, not rejected --------------
#[test]
fn phase_c_row05_zero_length_accepted() {
    let out = assert_same("errors row5 empty", |api| api.print_line(b""));
    assert_eq!(out, b"\n", "the guard is on the pointer, never on the length");
    let out = assert_same("errors row5 empty x8", |api| {
        for _ in 0..8 {
            api.print_line(b"");
        }
    });
    assert_eq!(out, b"\n".repeat(8));
}

// --- ERRORS row 6: oversized lengths, no limit exists ----------------
#[test]
fn phase_c_row06_oversized_lengths() {
    let mut rng = Rng::new(SEED ^ 0x26);
    for len in [1usize, 4095, 4096, 4097, 65535, 1024 * 1024] {
        let payload = rng.nonnul_bytes(len);
        let out = assert_same(&format!("errors row6 len={len}"), |api| api.print_line(&payload));
        assert_eq!(out.len(), len + 1, "no truncation at len={len}");
        assert_eq!(&out[..len], &payload[..]);
    }
}

// --- ERRORS row 7: non-UTF-8 / binary payloads ----------------------
#[test]
fn phase_c_row07_invalid_utf8() {
    let fixed: [&[u8]; 8] = [
        &[0x80],
        &[0xFE],
        &[0xFF],
        &[0x80, 0xFE, 0xFF],
        &[0xC0, 0x80],           // overlong NUL encoding
        &[0xED, 0xA0, 0x80],     // UTF-16 surrogate
        &[0xF5, 0x80, 0x80, 0x80], // > U+10FFFF
        &[0xE0, 0x80],           // truncated sequence
    ];
    for p in fixed {
        let out = assert_same(&format!("errors row7 {p:02x?}"), |api| api.print_line(p));
        let mut want = p.to_vec();
        want.push(b'\n');
        assert_eq!(out, want, "invalid UTF-8 must pass through verbatim");
    }
    let mut rng = Rng::new(SEED ^ 0x27);
    for i in 0..256 {
        // High bytes only: negative values when `char` is signed.
        let len = rng.range_incl(1, 64);
        let payload: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u64() as u8 & 0x7f)).collect();
        let out = assert_same(&format!("errors row7 #{i}"), |api| api.print_line(&payload));
        let mut want = payload.clone();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

// --- ERRORS row 8: interior NUL -------------------------------------
#[test]
fn phase_c_row08_interior_nul() {
    let out = assert_same("errors row8 aaa\\0bbb", |api| api.print_line(b"aaa\0bbb"));
    assert_eq!(out, b"aaa\n");
    let out = assert_same("errors row8 leading NUL", |api| api.print_line(b"\0bbb"));
    assert_eq!(out, b"\n");
}

// --- ERRORS row 9: format specifiers not interpreted ---------------
#[test]
fn phase_c_row09_format_specifiers_not_interpreted() {
    for s in [
        &b"%n"[..],
        &b"%s"[..],
        &b"%d %d %d %d %d %d %d %d"[..],
        &b"%99999999d"[..],
        &b"%n%n%n%n"[..],
        &b"100%"[..],
        &b"%"[..],
    ] {
        let out = assert_same(&format!("errors row9 {:?}", String::from_utf8_lossy(s)), |api| {
            api.print_line(s)
        });
        let mut want = s.to_vec();
        want.push(b'\n');
        assert_eq!(out, want, "payload must never be used as a format string");
    }
}

// --- ERRORS row 11: every int is in range for driver ---------------
#[test]
fn phase_c_row11_driver_all_int_values_valid() {
    let cases: [i32; 13] = [
        0,
        1,
        -1,
        2,
        -2,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        0x8000_0000u32 as i32,
        0x0001_0000,
        0xFFFF_0000u32 as i32,
        0x7FFF_FFFE,
    ];
    for v in cases {
        let out = assert_same(&format!("errors row11 useGood={v}"), |api| api.driver(v));
        if v == 0 {
            assert!(out.is_empty(), "only 0 is false");
        } else {
            assert_eq!(out, GOOD_LINE, "{v} must behave like true");
        }
    }
    let mut rng = Rng::new(SEED ^ 0x2B);
    for i in 0..512 {
        let v = rng.next_i32();
        let out = assert_same(&format!("errors row11 rand #{i} useGood={v}"), |api| api.driver(v));
        if v == 0 {
            assert!(out.is_empty());
        } else {
            assert_eq!(out, GOOD_LINE);
        }
    }
}

// --- ERRORS row 12: out-of-range "enum" values across the FFI -------
#[test]
fn phase_c_row12_driver_out_of_range_enum_values() {
    // If `useGood` were modelled as a C enum with variants {0, 1}, these would
    // all be out-of-range. C accepts any int; the Rust must too (no panic, no
    // `unreachable!`, no transmute UB).
    for v in [2i32, 3, 4, 7, 255, 256, 1000, -1, -255, i32::MIN, i32::MAX, 0x7f, 0x80] {
        let out = assert_same(&format!("errors row12 enum-ish useGood={v}"), |api| api.driver(v));
        assert_eq!(out, GOOD_LINE, "out-of-range enum value {v} must act like non-zero");
    }
}

// --- ERRORS row 13: garbage in the upper half of the argument register ---
#[test]
fn phase_c_row13_driver_upper_half_garbage() {
    // Call through a 64-bit-argument signature: only the low 32 bits are the
    // `int` parameter, so a value like 0xDEAD_BEEF_0000_0000 must be read as 0.
    type FnDriver64 = unsafe extern "C" fn(u64);
    let call = |api: &harness::Api, raw: u64| {
        // Re-cast the exported symbol to a 64-bit-arg signature.
        let f: FnDriver64 = unsafe { std::mem::transmute(api_driver_addr(api)) };
        unsafe { f(raw) }
    };
    for raw in [
        0xDEAD_BEEF_0000_0000u64,
        0xFFFF_FFFF_0000_0000,
        0x0000_0001_0000_0000,
        0xDEAD_BEEF_0000_0001,
        0xFFFF_FFFF_FFFF_FFFF,
    ] {
        let c_out = capture(|| call(c_api(), raw));
        let rust_out = capture(|| call(rust_api(), raw));
        assert_eq!(
            c_out,
            rust_out,
            "raw arg {raw:#018x}: C {} vs Rust {}",
            harness::show(&c_out),
            harness::show(&rust_out)
        );
        if raw as u32 == 0 {
            assert!(c_out.is_empty(), "low 32 bits zero -> bad() path");
        } else {
            assert_eq!(c_out, GOOD_LINE);
        }
    }
}

fn api_driver_addr(api: &harness::Api) -> *const () {
    // The `Api` struct stores the resolved function pointer; expose it as an
    // address so the test above can re-type it. Uses the public wrapper to keep
    // `Api`'s fields private.
    api.driver_addr()
}

// --- ERRORS row 14/15: repeated + interleaved invocation ------------
#[test]
fn phase_c_row14_row15_idempotence_and_ordering() {
    let out = assert_same("errors row14 good/bad x100 interleaved", |api| {
        for _ in 0..100 {
            api.good();
            api.bad();
        }
    });
    assert_eq!(out, GOOD_LINE.repeat(100));

    let out = assert_same("errors row15 ordering", |api| {
        api.bad();
        api.good();
        api.print_line(b"helperGood1 string");
        api.bad();
        api.good();
    });
    let mut want = Vec::new();
    want.extend_from_slice(GOOD_LINE);
    want.extend_from_slice(GOOD_LINE);
    want.extend_from_slice(GOOD_LINE);
    assert_eq!(out, want);
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite
// ===========================================================================

/// Prints (and sanity-checks) exactly which two `.so` files this run loaded, so
/// a "pass" can never be attributed to the wrong build profile.
#[test]
fn phase_d_audit_loaded_libraries() {
    let (c, r) = harness::loaded_paths();
    eprintln!("C    .so: {}", c.display());
    eprintln!("Rust .so: {}", r.display());

    let want_profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let r_str = r.to_string_lossy();
    assert!(
        r_str.contains(&format!("/target/{want_profile}/")),
        "this test binary was built for the `{want_profile}` profile but loaded {r_str}"
    );
    assert!(c.to_string_lossy().contains("c_src/build/"), "unexpected C .so: {c:?}");
}

#[test]
fn phase_d_symbol_parity() {
    // Both libraries must export exactly `printLine`, `bad`, `good`, `driver`.
    // Resolution happens in `Api::load`, which panics on any missing symbol.
    let _ = c_api();
    let _ = rust_api();

    // And the Rust `.so` must not be missing anything the C `.so` exports:
    // compare dynamic symbol tables directly.
    let names = ["printLine", "bad", "good", "driver"];
    for n in names {
        let mut sym = n.as_bytes().to_vec();
        sym.push(0);
        assert!(
            c_api().has_symbol(&sym),
            "C .so unexpectedly missing {n}"
        );
        assert!(
            rust_api().has_symbol(&sym),
            "Rust .so missing exported symbol {n}"
        );
    }
}

#[test]
fn phase_d_no_extra_null_deref_paths() {
    // A last sanity check that `printLine`'s only guard is the pointer, using a
    // non-NULL pointer to a lone NUL located at the very end of a page-aligned
    // allocation (so any read past the NUL would fault in both builds).
    let mut boxed = vec![0u8; 1];
    let p = boxed.as_mut_ptr() as *const c_char;
    let c_out = capture(|| unsafe { c_api().print_line_raw(p) });
    let rust_out = capture(|| unsafe { rust_api().print_line_raw(p) });
    assert_eq!(c_out, rust_out);
    assert_eq!(c_out, b"\n");
}
