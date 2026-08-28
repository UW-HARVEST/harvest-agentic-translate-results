//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Every row drives BOTH shared libraries through their exported
//! `decode_base64` symbol with many randomized inputs (fixed seed) and asserts
//! the returned allocations are byte-for-byte identical.

mod common;

use common::*;

/// Build a random string from `set` whose length is `len`.
fn s(rng: &mut Rng, set: &[u8], len: usize) -> Vec<u8> {
    rng.bytes_from(set, len)
}

/// Random length congruent to `r` (mod 4), in `[lo, hi]`.
fn len_mod4(rng: &mut Rng, r: usize, lo: usize, hi: usize) -> usize {
    loop {
        let n = rng.range(lo, hi);
        if n % 4 == r && n > 0 {
            return n;
        }
    }
}

const ITERS: usize = 400;

// --------------------------------------------------------------------------
// B01–B06: l % 4 == 0, no padding, one row per `decode` character class.
// --------------------------------------------------------------------------

fn alphabet_row(row: &str, set: &[u8], seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..ITERS {
        let n = len_mod4(&mut rng, 0, 4, 128);
        assert_both_ok(row, &s(&mut rng, set, n));
    }
}

#[test]
fn b01_mod4_zero_uppercase() {
    alphabet_row("B01", UPPER, 0x1111);
}

#[test]
fn b02_mod4_zero_lowercase() {
    alphabet_row("B02", LOWER, 0x2222);
}

#[test]
fn b03_mod4_zero_digits() {
    alphabet_row("B03", DIGIT, 0x3333);
}

#[test]
fn b04_mod4_zero_plus() {
    alphabet_row("B04", PLUS, 0x4444);
}

#[test]
fn b05_mod4_zero_slash_fallthrough() {
    // '/' has no explicit branch in `decode`; it hits the `return 63`
    // fall-through, same as '='.
    alphabet_row("B05", SLASH, 0x5555);
}

#[test]
fn b06_mod4_zero_full_alphabet() {
    alphabet_row("B06", B64, 0x6666);
    // Also sweep every alphabet character in every one of the 4 quartet slots.
    let mut rng = Rng::new(0x6667);
    for &ch in B64 {
        for slot in 0..4 {
            let mut v = s(&mut rng, B64, 4);
            v[slot] = ch;
            assert_both_ok("B06", &v);
        }
    }
}

// --------------------------------------------------------------------------
// B07–B10: tail handling — the `if (k + n < l)` defaults.
// --------------------------------------------------------------------------

fn mod4_row(row: &str, r: usize, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..ITERS {
        let n = len_mod4(&mut rng, r, 1, 129);
        assert_both_ok(row, &s(&mut rng, B64, n));
    }
}

#[test]
fn b07_mod4_one() {
    mod4_row("B07", 1, 0x7777);
}

#[test]
fn b08_mod4_two() {
    mod4_row("B08", 2, 0x8888);
}

#[test]
fn b09_mod4_three() {
    mod4_row("B09", 3, 0x9999);
}

#[test]
fn b10_tiny_lengths_exhaustive_alphabet() {
    // Lengths 1..=5 over the full alphabet, exhaustive for 1 and 2 chars.
    for &a in B64 {
        assert_both_ok("B10", &[a]);
        for &b in B64 {
            assert_both_ok("B10", &[a, b]);
        }
    }
    let mut rng = Rng::new(0xAAAA);
    for len in 1..=5 {
        for _ in 0..ITERS {
            assert_both_ok("B10", &s(&mut rng, B64, len));
        }
    }
}

// --------------------------------------------------------------------------
// B11–B17: '=' padding, canonical and pathological.
// --------------------------------------------------------------------------

#[test]
fn b11_single_trailing_pad() {
    let mut rng = Rng::new(0xB111);
    for _ in 0..ITERS {
        let groups = rng.range(1, 24);
        let mut v = s(&mut rng, B64, groups * 4);
        *v.last_mut().unwrap() = b'=';
        assert_both_ok("B11", &v);
    }
}

#[test]
fn b12_double_trailing_pad() {
    let mut rng = Rng::new(0xB122);
    for _ in 0..ITERS {
        let groups = rng.range(1, 24);
        let mut v = s(&mut rng, B64, groups * 4);
        let n = v.len();
        v[n - 2] = b'=';
        v[n - 1] = b'=';
        assert_both_ok("B12", &v);
    }
}

#[test]
fn b13_pad_in_the_middle() {
    // The C never stops at '='; it keeps decoding subsequent quartets.
    let mut rng = Rng::new(0xB133);
    for _ in 0..ITERS {
        let groups = rng.range(2, 16);
        let mut v = s(&mut rng, B64, groups * 4);
        let g = rng.below(groups - 1);
        v[g * 4 + 2] = b'=';
        v[g * 4 + 3] = b'=';
        assert_both_ok("B13", &v);
    }
    // Hand-built canonical shapes as well.
    for probe in [
        &b"QUJD=EVG"[..],
        &b"QQ==QQ=="[..],
        &b"QQ==AAAA"[..],
        &b"AAAA===="[..],
        &b"=AAAAAAA"[..],
        &b"A=A=A=A="[..],
        &b"==AAAA=="[..],
    ] {
        assert_both_ok("B13", probe);
    }
}

#[test]
fn b14_pad_at_slot_two() {
    // c2 == '=' -> decode('=') == 63, and BOTH later bytes are still emitted
    // because the suppression checks only look at c3 and c4.
    let mut rng = Rng::new(0xB144);
    for _ in 0..ITERS {
        let groups = rng.range(1, 16);
        let mut v = s(&mut rng, B64, groups * 4);
        let g = rng.below(groups);
        v[g * 4 + 1] = b'=';
        assert_both_ok("B14", &v);
    }
}

#[test]
fn b15_leading_pad() {
    let mut rng = Rng::new(0xB155);
    for _ in 0..ITERS {
        let n = rng.range(1, 64);
        let mut v = s(&mut rng, B64, n);
        v[0] = b'=';
        assert_both_ok("B15", &v);
    }
}

#[test]
fn b16_only_pads() {
    for n in 1..=8 {
        assert_both_ok("B16", &vec![b'='; n]);
    }
    for n in 1..=8 {
        assert_both_ok("B16", &vec![b'/'; n]);
    }
}

#[test]
fn b17_random_pads_everywhere() {
    let mut rng = Rng::new(0xB177);
    let mut set = B64.to_vec();
    // '=' over-represented so pads land in every slot frequently.
    for _ in 0..32 {
        set.push(b'=');
    }
    for _ in 0..(ITERS * 3) {
        let n = rng.range(1, 96);
        assert_both_ok("B17", &s(&mut rng, &set, n));
    }
}

// --------------------------------------------------------------------------
// B18–B20: ignored (non-base64) characters — POSIX "ignore" behaviour.
// --------------------------------------------------------------------------

#[test]
fn b18_leading_ignored_chars() {
    let mut rng = Rng::new(0xB188);
    for _ in 0..ITERS {
        let junk = rng.range(1, 16);
        let good = rng.range(1, 32);
        let mut v = s(&mut rng, NON_B64, junk);
        v.extend_from_slice(&s(&mut rng, B64, good));
        assert_both_ok("B18", &v);
    }
}

#[test]
fn b19_trailing_ignored_chars() {
    let mut rng = Rng::new(0xB199);
    for _ in 0..ITERS {
        let good = rng.range(1, 32);
        let junk = rng.range(1, 16);
        let mut v = s(&mut rng, B64, good);
        v.extend_from_slice(&s(&mut rng, NON_B64, junk));
        assert_both_ok("B19", &v);
    }
}

#[test]
fn b20_interspersed_ignored_chars() {
    let mut rng = Rng::new(0xB200);
    for _ in 0..(ITERS * 2) {
        let n = rng.range(1, 96);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            if rng.below(3) == 0 {
                v.push(rng.pick(NON_B64));
            } else {
                v.push(rng.pick(B64));
            }
        }
        assert_both_ok("B20", &v);
    }
    // MIME line-wrapped shape: 76-char lines separated by CRLF.
    let mut rng = Rng::new(0xB201);
    for _ in 0..64 {
        let plen = rng.range(1, 200);
        let payload = rng.nonnul_bytes(plen);
        let enc = b64_encode(&payload);
        let mut wrapped = Vec::new();
        for (i, ch) in enc.iter().enumerate() {
            if i > 0 && i % 76 == 0 {
                wrapped.extend_from_slice(b"\r\n");
            }
            wrapped.push(*ch);
        }
        wrapped.extend_from_slice(b"\n");
        assert_both_ok("B20", &wrapped);
    }
}

// --------------------------------------------------------------------------
// B21–B24: no-base64-chars, negative chars, control chars.
// --------------------------------------------------------------------------

#[test]
fn b21_no_base64_chars_returns_empty_buffer() {
    // Non-empty input, zero base64 chars => l == 0 => loop never runs =>
    // non-NULL, all-zero buffer. This is NOT an error path.
    for probe in [&b"!"[..], &b"!!!"[..], &b" "[..], &b"\n\r\t"[..], &b"@[`{"[..], &b"----------"[..]] {
        assert_both_ok("B21", probe);
        let (c, _r) = run_both(probe);
        assert!(!matches!(c, Outcome::Null), "C must not reject {probe:02x?}");
    }
    let mut rng = Rng::new(0xB211);
    for _ in 0..ITERS {
        let n = rng.range(1, 64);
        assert_both_ok("B21", &s(&mut rng, NON_B64, n));
    }
}

#[test]
fn b22_high_bytes_only() {
    // 0x80..=0xFF are negative `char`s: every range check in is_base64 and
    // decode must fail for them.
    let high: Vec<u8> = (0x80u16..=0xff).map(|b| b as u8).collect();
    assert_both_ok("B22", &high);
    let mut rng = Rng::new(0xB222);
    for _ in 0..ITERS {
        let n = rng.range(1, 64);
        assert_both_ok("B22", &s(&mut rng, &high, n));
    }
    for b in 0x80u16..=0xff {
        assert_both_ok("B22", &[b as u8]);
        assert_both_ok("B22", &[b as u8, b as u8, b as u8, b as u8]);
    }
}

#[test]
fn b23_high_bytes_mixed_with_base64() {
    let mut rng = Rng::new(0xB233);
    let mut set = B64.to_vec();
    set.push(b'=');
    for b in 0x80u16..=0xff {
        set.push(b as u8);
    }
    for _ in 0..(ITERS * 3) {
        let n = rng.range(1, 96);
        assert_both_ok("B23", &s(&mut rng, &set, n));
    }
    // Every high byte inserted into each slot of a 4-char quartet.
    for b in 0x80u16..=0xff {
        for slot in 0..4 {
            let mut v = b"QWJj".to_vec();
            v[slot] = b as u8;
            assert_both_ok("B23", &v);
        }
    }
}

#[test]
fn b24_control_bytes_mixed() {
    let mut ctrl: Vec<u8> = (0x01u8..=0x1f).collect();
    ctrl.push(0x7f);
    assert_both_ok("B24", &ctrl);
    let mut rng = Rng::new(0xB244);
    let mut set = B64.to_vec();
    set.extend_from_slice(&ctrl);
    for _ in 0..(ITERS * 2) {
        let n = rng.range(1, 96);
        assert_both_ok("B24", &s(&mut rng, &set, n));
    }
}

// --------------------------------------------------------------------------
// B25–B26: exhaustive 1- and 2-byte sweeps.
// --------------------------------------------------------------------------

#[test]
fn b25_every_single_byte_input() {
    for b in 0x01u16..=0xff {
        assert_same("B25", &[b as u8]);
    }
}

#[test]
fn b26_every_two_byte_input() {
    for a in 0x01u16..=0xff {
        for b in 0x01u16..=0xff {
            assert_same("B26", &[a as u8, b as u8]);
        }
    }
}

// --------------------------------------------------------------------------
// B27–B28: fully random bytes, all axes interacting.
// --------------------------------------------------------------------------

#[test]
fn b27_random_bytes_short() {
    let mut rng = Rng::new(0xB277);
    for _ in 0..5000 {
        let n = rng.range(1, 64);
        assert_same("B27", &rng.nonnul_bytes(n));
    }
}

#[test]
fn b28_random_bytes_long() {
    let mut rng = Rng::new(0xB288);
    for _ in 0..300 {
        let n = rng.range(1, 4096);
        assert_same("B28", &rng.nonnul_bytes(n));
    }
}

// --------------------------------------------------------------------------
// B29: embedded NUL truncation.
// --------------------------------------------------------------------------

#[test]
fn b29_embedded_nul_truncates() {
    let mut rng = Rng::new(0xB299);
    for _ in 0..ITERS {
        let head = rng.range(1, 32);
        let tail = rng.range(1, 32);
        let mut v = s(&mut rng, B64, head);
        v.push(0);
        v.extend_from_slice(&s(&mut rng, B64, tail));
        // run_both sizes the comparison window from strlen(), i.e. `head`.
        assert_both_ok("B29", &v);
    }
    // NUL right after a full quartet, and mid-quartet.
    for probe in [&b"QUJD\0QUJD"[..], &b"QU\0JD"[..], &b"Q\0"[..]] {
        assert_both_ok("B29", probe);
    }
}

// --------------------------------------------------------------------------
// B30: large inputs.
// --------------------------------------------------------------------------

#[test]
fn b30_large_inputs() {
    let mut rng = Rng::new(0xB300);
    for &n in &[4096usize, 65536, 1 << 20] {
        assert_both_ok("B30", &s(&mut rng, B64, n));
    }
    // Sizes straddling the quartet boundary at scale.
    for n in [4093usize, 4094, 4095, 4096, 4097, 65535, 65537] {
        assert_both_ok("B30", &s(&mut rng, B64, n));
    }
    // Large input full of ignored characters (l == 0 at scale).
    assert_both_ok("B30", &s(&mut rng, NON_B64, 100_000));
    // Large mixed input.
    let mut set = B64.to_vec();
    set.extend_from_slice(NON_B64);
    set.push(b'=');
    assert_both_ok("B30", &s(&mut rng, &set, 300_000));
}

// --------------------------------------------------------------------------
// B31: decoded output containing NUL bytes.
// --------------------------------------------------------------------------

#[test]
fn b31_output_with_embedded_nuls() {
    // "AAAA" decodes to 00 00 00 — a C-string comparison would see "" for
    // both sides regardless of a bug, so full-allocation equality matters.
    for probe in [
        &b"AAAA"[..],
        &b"AAAAAAAA"[..],
        &b"AAA="[..],
        &b"AA=="[..],
        &b"A"[..],
        &b"QQBB"[..],
        &b"AABBAABB"[..],
    ] {
        assert_both_ok("B31", probe);
        let (c, r) = run_both(probe);
        // Confirm the row really does produce interior NULs (so it is a
        // meaningful test of the full-buffer comparison).
        if let (Outcome::Buf(cb), Outcome::Buf(rb)) = (&c, &r) {
            assert_eq!(cb, rb, "B31 full-buffer mismatch for {probe:02x?}");
        }
    }
    let mut rng = Rng::new(0xB311);
    for _ in 0..ITERS {
        let n = len_mod4(&mut rng, 0, 4, 64);
        // Bias toward 'A' so many zero bits are produced.
        let set = b"AAAAAAAAB/+Qq0";
        assert_both_ok("B31", &s(&mut rng, set, n));
    }
}

// --------------------------------------------------------------------------
// B32: encode/decode round trip over random binary payloads.
// --------------------------------------------------------------------------

#[test]
fn b32_round_trip_random_payloads() {
    let mut rng = Rng::new(0xB322);
    for _ in 0..1000 {
        // len % 3 spans all three padding classes.
        let n = rng.range(1, 300);
        let payload = rng.nonnul_bytes(n);
        let enc = b64_encode(&payload);
        assert_both_ok("B32", &enc);
        // The C's own decode of a canonical encoding should reproduce the
        // payload prefix; assert that too, as a sanity check on the harness.
        let (c, r) = run_both(&enc);
        assert_eq!(c, r, "B32 divergence for payload len {n}");
        if let Outcome::Buf(b) = c {
            assert!(
                b.starts_with(&payload),
                "B32: C decode did not reproduce payload (len {n})"
            );
        }
    }
    // Payloads containing zero bytes too (round trip through NUL-heavy data).
    for _ in 0..200 {
        let n = rng.range(1, 90);
        let payload: Vec<u8> = (0..n).map(|_| if rng.below(2) == 0 { 0 } else { rng.pick(B64) }).collect();
        let enc = b64_encode(&payload);
        assert_both_ok("B32", &enc);
    }
}

// --------------------------------------------------------------------------
// B33: repeated / interleaved calls, both libraries live at once.
// --------------------------------------------------------------------------

#[test]
fn b33_repeated_interleaved_calls() {
    let cf = common::c_decode();
    let rf = common::rust_decode();
    let mut rng = Rng::new(0xB333);
    // Hold several buffers from both libraries alive simultaneously, then
    // compare — catches any shared/static state or cross-call interference.
    for _ in 0..200 {
        let inputs: Vec<Vec<u8>> = (0..8)
            .map(|_| {
                let n = rng.range(1, 48);
                let mut v = rng.nonnul_bytes(n);
                v.push(0);
                v
            })
            .collect();
        let mut c_ptrs = Vec::new();
        let mut r_ptrs = Vec::new();
        unsafe {
            for inp in &inputs {
                c_ptrs.push(cf(inp.as_ptr() as *const std::ffi::c_char));
                r_ptrs.push(rf(inp.as_ptr() as *const std::ffi::c_char));
            }
            for (i, inp) in inputs.iter().enumerate() {
                let strlen = inp.iter().position(|&b| b == 0).unwrap();
                let alloc = strlen + 14;
                let cp = c_ptrs[i];
                let rp = r_ptrs[i];
                assert_eq!(cp.is_null(), rp.is_null(), "B33 nullness mismatch");
                if !cp.is_null() {
                    let cb = std::slice::from_raw_parts(cp as *const u8, alloc);
                    let rb = std::slice::from_raw_parts(rp as *const u8, alloc);
                    assert_eq!(cb, rb, "B33 mismatch for {:02x?}", &inp[..strlen]);
                }
            }
            for p in c_ptrs.into_iter().chain(r_ptrs.into_iter()) {
                if !p.is_null() {
                    libc_free(p as *mut std::ffi::c_void);
                }
            }
        }
    }
}

extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut std::ffi::c_void);
}
