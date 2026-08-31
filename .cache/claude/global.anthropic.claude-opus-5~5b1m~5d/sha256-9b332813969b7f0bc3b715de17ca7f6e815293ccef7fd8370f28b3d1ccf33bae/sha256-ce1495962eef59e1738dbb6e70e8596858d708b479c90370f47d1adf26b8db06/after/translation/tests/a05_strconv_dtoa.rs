//! Differential tests for src/strconv.c and the exported entry points of
//! src/dtoa.c.
//!
//! This is the numeric-formatting core: `jsonp_dtostr` decides the exact bytes
//! every JSON real is encoded as, and `dtoa`/`dtoa_r` is the shortest-round-trip
//! digit generator underneath it. A one-digit difference here changes the output
//! of `json_dumps` for that value, so every comparison is byte-exact.
//!
//! Note on `jsonp_strtod`: the C contains a LIVE
//! `assert(end == strbuffer->value + strbuffer->length)` (the build defines no
//! NDEBUG), so it must only ever be handed a buffer whose entire contents are
//! consumed by `strtod`. Trailing garbage aborts the C process, so the
//! valid-path tests below always append exactly one complete numeric literal.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// jsonp_dtostr — the function that formats every JSON real
// ===========================================================================

/// `MAX_REAL_STR_LENGTH` in dump.c is 25, which is the buffer size real callers
/// use; the tests also probe smaller buffers to reach the "too short" arm.
const REAL_BUF: usize = 64;

/// Call `jsonp_dtostr` on both libraries and compare the return value AND the
/// full buffer image, so trailing bytes past the terminator are compared too.
unsafe fn cmp_dtostr(c: &Api, r: &Api, size: usize, value: f64, prec: c_int, ctx: &str) {
    // Poison identically so "did not write" is distinguishable from "wrote 0".
    let mut cbuf = [0xAAu8; REAL_BUF];
    let mut rbuf = [0xAAu8; REAL_BUF];
    let cret = (c.jsonp_dtostr)(cbuf.as_mut_ptr() as *mut c_char, size, value, prec);
    let rret = (r.jsonp_dtostr)(rbuf.as_mut_ptr() as *mut c_char, size, value, prec);
    diff_eq!(cret, rret, "jsonp_dtostr return [{ctx}]");
    diff_eq!(cbuf, rbuf, "jsonp_dtostr buffer image [{ctx}]");
    // When it succeeded the return is the string length; check that invariant
    // holds in the C and that the Rust agrees on the produced text.
    if cret >= 0 {
        let ctext = cbytes(cbuf.as_ptr() as *const c_char);
        let rtext = cbytes(rbuf.as_ptr() as *const c_char);
        diff_eq!(ctext.clone(), rtext, "jsonp_dtostr text [{ctx}]");
        assert_eq!(
            ctext.as_ref().unwrap().len() as c_int,
            cret,
            "C: jsonp_dtostr return must equal strlen [{ctx}]"
        );
    }
}

/// The doubles the C code branches on, plus every classically hard value.
fn interesting_doubles() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.1,
        -0.1,
        0.5,
        2.0,
        10.0,
        100.0,
        1e15,
        1e16,
        1e17, // decpt > 16 switches to exponent form
        1e-4,
        1e-5, // decpt <= -4 switches to exponent form
        1.0 / 3.0,
        2.0 / 3.0,
        f64::MIN_POSITIVE,     // smallest normal
        5e-324,                // smallest subnormal
        1e-308,
        1e308,
        f64::MAX,
        -f64::MAX,
        2.2250738585072011e-308, // the classic strtod/PHP hang value
        9007199254740992.0,      // 2^53
        9007199254740993.0,      // 2^53 + 1 (not representable)
        0.500000000000000166533453693773481063544750213623046875,
        1.7976931348623157e308,
        4.9406564584124654e-324,
        123456789012345678.0,
        1.2345678901234567,
        3.141592653589793,
        2.718281828459045,
        1e22,
        1e23, // 1e23 is a famous dtoa edge case
        8.98846567431158e307,
    ];
    // Powers of two and ten across the whole exponent range.
    for e in -1074i32..=1023 {
        if e % 37 == 0 {
            v.push(libm_ldexp(1.0, e));
        }
    }
    for e in -320i32..=308 {
        if e % 11 == 0 {
            v.push(format!("1e{e}").parse().unwrap());
        }
    }
    v
}

fn libm_ldexp(x: f64, e: i32) -> f64 {
    // 2^e without libm: build the multiplier by repeated squaring in f64.
    let mut r = x;
    let mut n = e;
    while n > 0 {
        r *= 2.0;
        n -= 1;
    }
    while n < 0 {
        r /= 2.0;
        n += 1;
    }
    r
}

#[test]
fn jsonp_dtostr_precision_sweep_over_interesting_doubles() {
    let _g = global_state_lock();
    let (c, r) = both();
    // `precision == 0` selects dtoa mode 0 (shortest round-trip); anything else
    // selects mode 2 with that many digits. JSON_REAL_PRECISION caps at 31.
    for &v in &interesting_doubles() {
        for prec in 0..=31 {
            unsafe {
                cmp_dtostr(c, r, REAL_BUF, v, prec, &format!("value={v:e} prec={prec}"));
            }
        }
    }
}

#[test]
fn jsonp_dtostr_randomised_doubles() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_0001);
    unsafe {
        for i in 0..40000 {
            let v = rng.real();
            let prec = rng.below(32) as c_int;
            cmp_dtostr(c, r, REAL_BUF, v, prec, &format!("iter={i} value={v:e} prec={prec}"));
        }
    }
}

#[test]
fn jsonp_dtostr_random_bit_patterns() {
    let _g = global_state_lock();
    // Every finite double is legal input; random bit patterns explore the
    // mantissa/exponent space far more thoroughly than decimal literals.
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_0002);
    unsafe {
        for i in 0..40000 {
            let bits = rng.next_u64();
            let v = f64::from_bits(bits);
            if !v.is_finite() {
                continue; // dump.c never passes inf/nan (json_real rejects them)
            }
            let prec = rng.below(32) as c_int;
            cmp_dtostr(
                c,
                r,
                REAL_BUF,
                v,
                prec,
                &format!("iter={i} bits={bits:#018x} prec={prec}"),
            );
        }
    }
}

#[test]
fn jsonp_dtostr_buffer_too_short_boundary() {
    let _g = global_state_lock();
    // The length check is
        //   3 + (vdigits_end - vdigits_start) + (use_exp ? 5 : 0) > size
        // so for each value there is an exact smallest size that succeeds. Sweep
        // every size from 0 upward and require both to flip from -1 to success at
        // the SAME point, with identical output.
    let (c, r) = both();
    for &v in &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.1,
        -0.1,
        1e308,
        -1e308,
        5e-324,
        1.0 / 3.0,
        1.2345678901234567,
        1e-5,
        1e17,
        f64::MAX,
    ] {
        for prec in [0, 1, 6, 17, 25, 31] {
            for size in 0usize..=40 {
                unsafe {
                    cmp_dtostr(
                        c,
                        r,
                        size,
                        v,
                        prec,
                        &format!("value={v:e} prec={prec} size={size}"),
                    );
                }
            }
        }
    }
}

#[test]
fn jsonp_dtostr_matches_at_dump_buffer_size_25() {
    let _g = global_state_lock();
    // dump.c uses `char buffer[MAX_REAL_STR_LENGTH]` with MAX_REAL_STR_LENGTH
    // 25. At precision >= 22 some values no longer fit and jsonp_dtostr returns
    // -1, which makes json_dumps fail. Both must agree exactly on where.
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_0003);
    unsafe {
        for &v in &interesting_doubles() {
            for prec in 0..=31 {
                cmp_dtostr(c, r, 25, v, prec, &format!("size=25 value={v:e} prec={prec}"));
            }
        }
        for i in 0..20000 {
            let v = rng.real();
            let prec = rng.below(32) as c_int;
            cmp_dtostr(c, r, 25, v, prec, &format!("size=25 iter={i} value={v:e} prec={prec}"));
        }
    }
}

#[test]
fn jsonp_dtostr_negative_precision() {
    let _g = global_state_lock();
    // `precision` arrives as an int; dump.c only ever passes 0..31, but the
    // symbol is exported so a negative value is a real input the C handles
    // (mode 2 with a negative ndigits). Both must do the same thing.
    let (c, r) = both();
    unsafe {
        for prec in [-1, -2, -17, -100, i32::MIN + 1] {
            for &v in &[0.0, 1.0, -1.0, 0.1, 1e308, 5e-324, 1.0 / 3.0] {
                cmp_dtostr(c, r, REAL_BUF, v, prec, &format!("value={v:e} prec={prec}"));
            }
        }
    }
}

#[test]
fn jsonp_dtostr_large_precision_beyond_31() {
    let _g = global_state_lock();
    // JSON_REAL_PRECISION masks to 0x1F so dump.c cannot exceed 31, but a
    // direct caller can. Compare well past that.
    let (c, r) = both();
    unsafe {
        for prec in [32, 40, 64, 100, 1000] {
            for &v in &[0.0, 1.0, -1.0, 0.1, 1e308, 5e-324, 1.0 / 3.0, f64::MAX] {
                cmp_dtostr(c, r, REAL_BUF, v, prec, &format!("value={v:e} prec={prec}"));
            }
        }
    }
}

// ===========================================================================
// jsonp_strtod — the function that parses every JSON real
// ===========================================================================

/// Build a strbuffer containing exactly `text`, then run `jsonp_strtod`.
/// Returns (return code, parsed bits) so -0.0 stays distinct from 0.0.
unsafe fn run_strtod(api: &Api, text: &str) -> (c_int, u64) {
    let mut sb = strbuffer_t::zeroed();
    assert_eq!((api.strbuffer_init)(&mut sb), 0);
    let bytes = text.as_bytes();
    assert_eq!(
        (api.strbuffer_append_bytes)(&mut sb, bytes.as_ptr() as *const c_char, bytes.len()),
        0
    );
    let mut out: f64 = f64::from_bits(0xDEAD_BEEF_DEAD_BEEF);
    let ret = (api.jsonp_strtod)(&mut sb, &mut out);
    (api.strbuffer_close)(&mut sb);
    (ret, out.to_bits())
}

/// Numeric literals that `strtod` consumes ENTIRELY, so the live assert in
/// `jsonp_strtod` is satisfied. (A literal with trailing garbage would abort
/// the C process, so it belongs in ERRORS.md's untestable list, not here.)
fn strtod_inputs() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "0", "-0", "1", "-1", "12345", "-12345", "0.0", "-0.0", "0.5", "1.5", "-1.5",
        "3.14159265358979", "0.1", "1e0", "1E0", "1e+0", "1e-0", "1e10", "1e-10", "1e308",
        "1e-308", "1e309", "1e-309", "-1e309", "1e-400", "1e400", "-1e400",
        "1.7976931348623157e308", "2.2250738585072014e-308", "4.9406564584124654e-324",
        "2.2250738585072011e-308", "9007199254740993", "1e23", "8.98846567431158e307",
        "0.500000000000000166533453693773481063544750213623046875",
        "123456789012345678901234567890", "0.000000000000000000000000001",
        "1.0000000000000000000000000000000000000000001",
        "-0.0000000000000000000000000000000000001e-300",
        "1e1000", "-1e1000", "0e0", "0e999999", "0.0e-999999",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    // Systematic exponent sweep, both signs, with and without a fraction.
    for e in -330i32..=310 {
        if e % 7 == 0 {
            v.push(format!("1e{e}"));
            v.push(format!("-1.25e{e}"));
            v.push(format!("9.9999999999999999e{e}"));
        }
    }
    v
}

#[test]
fn jsonp_strtod_over_interesting_literals() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for t in strtod_inputs() {
            let (cret, cbits) = run_strtod(c, &t);
            let (rret, rbits) = run_strtod(r, &t);
            diff_eq!(cret, rret, "jsonp_strtod({t:?}) return");
            // Compare BITS: this distinguishes 0.0 from -0.0 and catches a
            // one-ULP difference that a printed comparison would hide.
            diff_eq!(cbits, rbits, "jsonp_strtod({t:?}) result bits");
        }
    }
}

#[test]
fn jsonp_strtod_randomised_decimal_strings() {
    let _g = global_state_lock();
    // Random long decimal strings are exactly what forces dtoa's slow
    // correction paths (bigcomp / big-integer fallback).
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_0004);
    unsafe {
        for i in 0..20000 {
            let ndigits = 1 + rng.below(30);
            let mut s = String::new();
            if rng.bool() {
                s.push('-');
            }
            for j in 0..ndigits {
                let d = rng.below(10);
                // Avoid a leading zero followed by more digits, which JSON
                // disallows and which would take a different strtod path.
                if j == 0 && d == 0 && ndigits > 1 {
                    s.push('1');
                } else {
                    s.push((b'0' + d as u8) as char);
                }
            }
            if rng.bool() {
                s.push('.');
                for _ in 0..1 + rng.below(30) {
                    s.push((b'0' + rng.below(10) as u8) as char);
                }
            }
            if rng.bool() {
                s.push(if rng.bool() { 'e' } else { 'E' });
                match rng.below(3) {
                    0 => s.push('+'),
                    1 => s.push('-'),
                    _ => {}
                }
                s.push_str(&rng.range(0, 340).to_string());
            }
            let (cret, cbits) = run_strtod(c, &s);
            let (rret, rbits) = run_strtod(r, &s);
            diff_eq!(cret, rret, "iter {i}: jsonp_strtod({s:?}) return");
            diff_eq!(cbits, rbits, "iter {i}: jsonp_strtod({s:?}) result bits");
        }
    }
}

#[test]
fn jsonp_strtod_round_trips_dtostr_output() {
    let _g = global_state_lock();
    // The property the library depends on: dumping a real and re-parsing it must
    // give back the same double. Verified through BOTH libraries, and the two
    // must agree on the intermediate text as well.
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_0005);
    unsafe {
        let mut values = interesting_doubles();
        for _ in 0..5000 {
            values.push(rng.real());
        }
        for v in values {
            // precision 0 == shortest round-trip, which must be exact.
            let mut cbuf = [0u8; REAL_BUF];
            let mut rbuf = [0u8; REAL_BUF];
            let cn = (c.jsonp_dtostr)(cbuf.as_mut_ptr() as *mut c_char, REAL_BUF, v, 0);
            let rn = (r.jsonp_dtostr)(rbuf.as_mut_ptr() as *mut c_char, REAL_BUF, v, 0);
            diff_eq!(cn, rn, "dtostr return for {v:e}");
            let ctext = cbytes(cbuf.as_ptr() as *const c_char).unwrap();
            let rtext = cbytes(rbuf.as_ptr() as *const c_char).unwrap();
            diff_eq!(ctext.clone(), rtext, "dtostr text for {v:e}");

            let s = String::from_utf8(ctext).unwrap();
            let (cret, cbits) = run_strtod(c, &s);
            let (rret, rbits) = run_strtod(r, &s);
            diff_eq!(cret, rret, "re-parse of {s:?} return");
            diff_eq!(cbits, rbits, "re-parse of {s:?} bits");
            assert_eq!(
                cbits,
                v.to_bits(),
                "C: shortest round-trip must be exact for {v:e} (got {s:?})"
            );
        }
    }
}

// ===========================================================================
// dtoa / dtoa_r / freedtoa
// ===========================================================================

/// Compare a `dtoa` call: the digit string, `*decpt`, `*sign` and the offset of
/// `*rve` (the end-of-digits pointer) all form part of the contract.
unsafe fn cmp_dtoa(c: &Api, r: &Api, v: f64, mode: c_int, ndigits: c_int, ctx: &str) {
    let mut cdecpt: c_int = -12345;
    let mut csign: c_int = -12345;
    let mut crve: *mut c_char = std::ptr::null_mut();
    let mut rdecpt: c_int = -12345;
    let mut rsign: c_int = -12345;
    let mut rrve: *mut c_char = std::ptr::null_mut();

    let cs_ = (c.dtoa)(v, mode, ndigits, &mut cdecpt, &mut csign, &mut crve);
    let rs_ = (r.dtoa)(v, mode, ndigits, &mut rdecpt, &mut rsign, &mut rrve);

    diff_eq!(cs_.is_null(), rs_.is_null(), "dtoa null-ness [{ctx}]");
    diff_eq!(cbytes(cs_), cbytes(rs_), "dtoa digits [{ctx}]");
    diff_eq!(cdecpt, rdecpt, "dtoa *decpt [{ctx}]");
    diff_eq!(csign, rsign, "dtoa *sign [{ctx}]");
    // rve is a pointer INTO the returned buffer; compare it as an offset.
    let coff = if crve.is_null() || cs_.is_null() {
        None
    } else {
        Some(crve as usize - cs_ as usize)
    };
    let roff = if rrve.is_null() || rs_.is_null() {
        None
    } else {
        Some(rrve as usize - rs_ as usize)
    };
    diff_eq!(coff, roff, "dtoa *rve offset [{ctx}]");

    // Each library owns its own dtoa freelist, so free with the matching one.
    if !cs_.is_null() {
        (c.freedtoa)(cs_);
    }
    if !rs_.is_null() {
        (r.freedtoa)(rs_);
    }
}

#[test]
fn dtoa_mode_and_ndigits_sweep() {
    let _g = global_state_lock();
    let (c, r) = both();
    // dtoa documents modes 0..9 (values >= 5 fold onto 2/3 with a "leftright"
    // variation). ndigits is only consulted for modes 2..5.
    for &v in &interesting_doubles() {
        for mode in 0..=9 {
            for ndigits in [0, 1, 2, 5, 17, 18, 30] {
                unsafe {
                    cmp_dtoa(
                        c,
                        r,
                        v,
                        mode,
                        ndigits,
                        &format!("value={v:e} mode={mode} ndigits={ndigits}"),
                    );
                }
            }
        }
    }
}

#[test]
fn dtoa_randomised() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_0006);
    unsafe {
        for i in 0..20000 {
            let v = rng.real();
            let mode = rng.below(10) as c_int;
            let ndigits = rng.below(35) as c_int;
            cmp_dtoa(
                c,
                r,
                v,
                mode,
                ndigits,
                &format!("iter={i} value={v:e} mode={mode} ndigits={ndigits}"),
            );
        }
    }
}

#[test]
fn dtoa_random_bit_patterns() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_0007);
    unsafe {
        for i in 0..20000 {
            let bits = rng.next_u64();
            let v = f64::from_bits(bits);
            let mode = rng.below(10) as c_int;
            let ndigits = rng.below(20) as c_int;
            cmp_dtoa(
                c,
                r,
                v,
                mode,
                ndigits,
                &format!("iter={i} bits={bits:#018x} mode={mode} ndigits={ndigits}"),
            );
        }
    }
}

#[test]
fn dtoa_infinities_and_nans() {
    let _g = global_state_lock();
    // dtoa has explicit branches for the all-ones exponent, emitting "Infinity"
    // or "NaN" and setting decpt to 9999.
    let (c, r) = both();
    let specials = [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001), // quiet NaN, payload 1
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xfff8_0000_0000_0000), // negative quiet NaN
    ];
    for (i, &v) in specials.iter().enumerate() {
        for mode in 0..=5 {
            unsafe {
                cmp_dtoa(c, r, v, mode, 17, &format!("special#{i} bits={:#018x} mode={mode}", v.to_bits()));
            }
        }
    }
}

#[test]
fn dtoa_negative_ndigits() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for ndigits in [-1, -2, -10, -100] {
            for mode in [2, 3, 4, 5] {
                for &v in &[0.0, 1.0, -1.0, 0.1, 1e308, 5e-324, 1.0 / 3.0] {
                    cmp_dtoa(
                        c,
                        r,
                        v,
                        mode,
                        ndigits,
                        &format!("value={v:e} mode={mode} ndigits={ndigits}"),
                    );
                }
            }
        }
    }
}

#[test]
fn dtoa_out_of_range_modes() {
    let _g = global_state_lock();
    // `mode` is an int and the C clamps/folds anything above 9; a caller can
    // pass any value, so out-of-range modes are real input.
    let (c, r) = both();
    unsafe {
        for mode in [10, 11, 42, 1000, -1, -5, i32::MIN + 1, i32::MAX] {
            for &v in &[0.0, -0.0, 1.0, -1.0, 0.1, 1e308, 5e-324, 1.0 / 3.0] {
                cmp_dtoa(c, r, v, mode, 17, &format!("value={v:e} mode={mode}"));
            }
        }
    }
}

/// `dtoa_r` writes into a caller buffer; compare the result, the out-params and
/// the whole buffer image.
unsafe fn cmp_dtoa_r(
    c: &Api,
    r: &Api,
    v: f64,
    mode: c_int,
    ndigits: c_int,
    blen: usize,
    ctx: &str,
) {
    const CAP: usize = 128;
    let mut cbuf = [0xAAu8; CAP];
    let mut rbuf = [0xAAu8; CAP];
    let mut cdecpt: c_int = -12345;
    let mut csign: c_int = -12345;
    let mut crve: *mut c_char = std::ptr::null_mut();
    let mut rdecpt: c_int = -12345;
    let mut rsign: c_int = -12345;
    let mut rrve: *mut c_char = std::ptr::null_mut();

    let cs_ = (c.dtoa_r)(
        v, mode, ndigits, &mut cdecpt, &mut csign, &mut crve,
        cbuf.as_mut_ptr() as *mut c_char, blen,
    );
    let rs_ = (r.dtoa_r)(
        v, mode, ndigits, &mut rdecpt, &mut rsign, &mut rrve,
        rbuf.as_mut_ptr() as *mut c_char, blen,
    );

    diff_eq!(cs_.is_null(), rs_.is_null(), "dtoa_r null-ness [{ctx}]");
    diff_eq!(cbytes(cs_), cbytes(rs_), "dtoa_r digits [{ctx}]");
    diff_eq!(cdecpt, rdecpt, "dtoa_r *decpt [{ctx}]");
    diff_eq!(csign, rsign, "dtoa_r *sign [{ctx}]");
    diff_eq!(cbuf, rbuf, "dtoa_r buffer image [{ctx}]");
    let coff = if crve.is_null() || cs_.is_null() {
        None
    } else {
        Some(crve as usize - cs_ as usize)
    };
    let roff = if rrve.is_null() || rs_.is_null() {
        None
    } else {
        Some(rrve as usize - rs_ as usize)
    };
    diff_eq!(coff, roff, "dtoa_r *rve offset [{ctx}]");
    // dtoa_r returns the caller buffer when it fits, so there is nothing to
    // free in that case; it never allocates when it succeeds into `buf`.
}

#[test]
fn dtoa_r_buffer_length_sweep() {
    let _g = global_state_lock();
    // strconv.c calls dtoa_r with a 25-byte buffer, and treats NULL as
    // "should not happen". Sweep the length so both agree exactly on where the
    // buffer becomes too small.
    let (c, r) = both();
    for &v in &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.1,
        1e308,
        -1e308,
        5e-324,
        1.0 / 3.0,
        1.2345678901234567,
        f64::MAX,
        9007199254740993.0,
    ] {
        for mode in [0, 2, 3] {
            for ndigits in [0, 1, 17, 25] {
                for blen in 0usize..=40 {
                    unsafe {
                        cmp_dtoa_r(
                            c, r, v, mode, ndigits, blen,
                            &format!("value={v:e} mode={mode} ndigits={ndigits} blen={blen}"),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn dtoa_r_randomised() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_0008);
    unsafe {
        for i in 0..20000 {
            let v = rng.real();
            let mode = rng.below(10) as c_int;
            let ndigits = rng.below(30) as c_int;
            let blen = rng.below(60);
            cmp_dtoa_r(
                c, r, v, mode, ndigits, blen,
                &format!("iter={i} value={v:e} mode={mode} ndigits={ndigits} blen={blen}"),
            );
        }
    }
}

#[test]
fn dtoa_r_at_strconv_buffer_size_25() {
    let _g = global_state_lock();
    // The exact configuration strconv.c uses.
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_0009);
    unsafe {
        for &v in &interesting_doubles() {
            for prec in 0..=31 {
                let mode = if prec == 0 { 0 } else { 2 };
                cmp_dtoa_r(c, r, v, mode, prec, 25, &format!("value={v:e} prec={prec}"));
            }
        }
        for i in 0..20000 {
            let v = rng.real();
            let prec = rng.below(32) as c_int;
            let mode = if prec == 0 { 0 } else { 2 };
            cmp_dtoa_r(c, r, v, mode, prec, 25, &format!("iter={i} value={v:e} prec={prec}"));
        }
    }
}

#[test]
fn freedtoa_on_dtoa_results() {
    let _g = global_state_lock();
    // Allocate and free many dtoa results so each library's internal freelist
    // is exercised and reused (a mismatched freelist would corrupt later calls,
    // which the interleaved comparison below would catch).
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_000A);
    unsafe {
        for i in 0..5000 {
            let v = rng.real();
            let mode = rng.below(10) as c_int;
            let ndigits = rng.below(30) as c_int;
            let mut cd = 0;
            let mut cs2 = 0;
            let mut crve = std::ptr::null_mut();
            let mut rd = 0;
            let mut rs2 = 0;
            let mut rrve = std::ptr::null_mut();
            let cp = (c.dtoa)(v, mode, ndigits, &mut cd, &mut cs2, &mut crve);
            let rp = (r.dtoa)(v, mode, ndigits, &mut rd, &mut rs2, &mut rrve);
            diff_eq!(cbytes(cp), cbytes(rp), "iter {i}: digits before free");
            if !cp.is_null() {
                (c.freedtoa)(cp);
            }
            if !rp.is_null() {
                (r.freedtoa)(rp);
            }
            // Immediately allocate again so the just-freed block is reused.
            let cp2 = (c.dtoa)(v, 0, 0, &mut cd, &mut cs2, &mut crve);
            let rp2 = (r.dtoa)(v, 0, 0, &mut rd, &mut rs2, &mut rrve);
            diff_eq!(cbytes(cp2), cbytes(rp2), "iter {i}: digits after freelist reuse");
            if !cp2.is_null() {
                (c.freedtoa)(cp2);
            }
            if !rp2.is_null() {
                (r.freedtoa)(rp2);
            }
        }
    }
}

// ===========================================================================
// gethex — hex float parsing
// ===========================================================================

/// `gethex(&s, &rv, rounding, sign)` is called by strtod with `s` pointing at
/// the leading '0' of a `"0x..."` literal. It returns `void`, advancing `*s` and
/// storing the parsed value through `rv`.
///
/// PRECONDITION: the C starts with `s0 = *sp + 2`, skipping the `"0x"` prefix
/// **unconditionally and without checking the length**. Handing it a string
/// shorter than two characters therefore reads past the end of the buffer —
/// undefined behaviour in the C itself, not a behaviour a translation can or
/// should reproduce. The C's own call site guarantees the prefix (it only calls
/// gethex once it has seen `s[0]=='0'` and `s[1]` in `{'x','X'}`), so every
/// input below is at least two characters long.
unsafe fn cmp_gethex(c: &Api, r: &Api, text: &str, rounding: c_int, sign: c_int) {
    let cstr = cs(text);
    let rstr = cs(text);

    let mut cp: *const c_char = cstr.as_ptr();
    let mut rp: *const c_char = rstr.as_ptr();
    // U is a union { double; ULong[2]; } — 8 bytes. Start from a known pattern.
    let mut cu: f64 = f64::from_bits(0xDEAD_BEEF_DEAD_BEEF);
    let mut ru: f64 = f64::from_bits(0xDEAD_BEEF_DEAD_BEEF);

    // gethex returns void: everything observable is in `rvp` and `*sp`.
    (c.gethex)(&mut cp, &mut cu as *mut f64 as *mut c_void, rounding, sign);
    (r.gethex)(&mut rp, &mut ru as *mut f64 as *mut c_void, rounding, sign);

    let ctx = format!("gethex({text:?}, rounding={rounding}, sign={sign})");
    diff_eq!(cu.to_bits(), ru.to_bits(), "{ctx} value bits");
    // How far the parser advanced is part of the contract.
    diff_eq!(
        cp as usize - cstr.as_ptr() as usize,
        rp as usize - rstr.as_ptr() as usize,
        "{ctx} advance"
    );
}

#[test]
fn gethex_valid_and_malformed_hex_floats() {
    let _g = global_state_lock();
    let (c, r) = both();
    let inputs = [
        "0x1p0", "0X1P0", "0x1p1", "0x1p-1", "0x1p+1", "0x1.8p3", "0x.8p1", "0x8p-1",
        "0x0p0", "0x0", "0x1", "0xf", "0xF", "0xff", "0xFFFFFFFF", "0xdeadbeef",
        "0x1.0000000000000p0", "0x1.fffffffffffffp1023", "0x1p-1074", "0x1p-1075",
        "0x1p1024", "0x1p-1023", "0x10000000000000000p0",
        "0x1.921fb54442d18p+1", // pi
        // malformed / edge forms
        "0x", "0xp0", "0x.", "0x.p0", "0xg", "0x1p", "0x1p+", "0x1p-", "0x1pz",
        "0x1.p0", "0x.1p0", "0x1..2p0", "0x1p0p0", "0x1e5", "0x1P",
        // NOTE: "0" (and any 1-character string) is deliberately absent — see
        // the precondition on cmp_gethex; it would read past the buffer.
        "0x00000000000000000000001p0",
        "0x1.0000000000000000000000000000001p0",
    ];
    for t in inputs {
        for rounding in 0..=3 {
            for sign in [0, 1] {
                unsafe {
                    cmp_gethex(c, r, t, rounding, sign);
                }
            }
        }
    }
}

#[test]
fn gethex_randomised() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_000B);
    unsafe {
        for _ in 0..8000 {
            let mut s = String::from(if rng.bool() { "0x" } else { "0X" });
            for _ in 0..rng.below(20) {
                s.push(*rng.choice(&[
                    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd',
                    'e', 'f', 'A', 'B', 'C', 'D', 'E', 'F',
                ]));
            }
            if rng.bool() {
                s.push('.');
                for _ in 0..rng.below(20) {
                    s.push(*rng.choice(&['0', '1', '7', '8', '9', 'a', 'f', 'F']));
                }
            }
            if rng.bool() {
                s.push(if rng.bool() { 'p' } else { 'P' });
                match rng.below(3) {
                    0 => s.push('+'),
                    1 => s.push('-'),
                    _ => {}
                }
                s.push_str(&rng.range(0, 1200).to_string());
            }
            let rounding = rng.below(4) as c_int;
            let sign = rng.below(2) as c_int;
            cmp_gethex(c, r, &s, rounding, sign);
        }
    }
}

#[test]
fn gethex_out_of_range_rounding() {
    let _g = global_state_lock();
    // `rounding` is an int; only 0..3 are meaningful but any value is accepted
    // across the FFI boundary, so both must treat the rest identically.
    let (c, r) = both();
    unsafe {
        for rounding in [4, 5, 100, -1, -100, i32::MIN + 1, i32::MAX] {
            for t in ["0x1p0", "0x1.8p3", "0x1.fffffffffffffp1023", "0x1p-1075"] {
                for sign in [0, 1] {
                    cmp_gethex(c, r, t, rounding, sign);
                }
            }
        }
        // Likewise a `sign` outside {0, 1}.
        for sign in [2, -1, 100, i32::MIN + 1, i32::MAX] {
            for t in ["0x1p0", "0x1.8p3"] {
                cmp_gethex(c, r, t, 1, sign);
            }
        }
    }
}

// ===========================================================================
// strtod__unused — dtoa's own strtod implementation
// ===========================================================================

/// Compare `strtod__unused`: the parsed value bits AND how far `endptr` moved.
unsafe fn cmp_strtod_unused(c: &Api, r: &Api, text: &str) {
    let cstr = cs(text);
    let rstr = cs(text);
    let mut cend: *mut c_char = std::ptr::null_mut();
    let mut rend: *mut c_char = std::ptr::null_mut();
    let cv = (c.strtod__unused)(cstr.as_ptr(), &mut cend);
    let rv = (r.strtod__unused)(rstr.as_ptr(), &mut rend);
    diff_eq!(cv.to_bits(), rv.to_bits(), "strtod__unused({text:?}) value bits");
    let coff = if cend.is_null() {
        None
    } else {
        Some(cend as usize - cstr.as_ptr() as usize)
    };
    let roff = if rend.is_null() {
        None
    } else {
        Some(rend as usize - rstr.as_ptr() as usize)
    };
    diff_eq!(coff, roff, "strtod__unused({text:?}) endptr offset");
}

#[test]
fn strtod_unused_over_literals_including_trailing_garbage() {
    let _g = global_state_lock();
    // Unlike jsonp_strtod there is no assert here, so trailing garbage is a
    // legitimate input and the endptr is the observable that matters.
    let (c, r) = both();
    let mut inputs: Vec<String> = strtod_inputs();
    inputs.extend(
        [
            "", " ", "   1.5", "\t\n 2.5", "1.5abc", "abc", "+1.5", "-", "+", ".", ".5",
            "5.", "1e", "1e+", "1e-", "1ex", "0x1p3", "0X1P3", "0x", "0xg", "inf",
            "infinity", "INF", "Infinity", "nan", "NAN", "NaN", "nan(123)", "-inf",
            "-nan", "1.5e", "1_000", "1,5", "--1", "1e5e5", "0b101", "0777",
            "1e99999999999999999999", "-1e99999999999999999999",
            "00000000000000000000001", "0.00000000000000000000001",
        ]
        .into_iter()
        .map(String::from),
    );
    unsafe {
        for t in inputs {
            cmp_strtod_unused(c, r, &t);
        }
    }
}

#[test]
fn strtod_unused_randomised() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x5C_000C);
    unsafe {
        for _ in 0..15000 {
            // Random strings from a numeric-ish alphabet: covers valid numbers,
            // partial parses and outright rejections.
            let n = rng.below(24);
            let s: String = (0..n)
                .map(|_| {
                    *rng.choice(&[
                        '0', '1', '2', '5', '9', '.', 'e', 'E', '+', '-', 'x', 'X', 'p',
                        'P', 'a', 'f', 'i', 'n', ' ', 'F', '\t',
                    ])
                })
                .collect();
            cmp_strtod_unused(c, r, &s);
        }
    }
}

#[test]
fn strtod_unused_hard_round_trip_cases() {
    let _g = global_state_lock();
    // The values known to force dtoa's 64/96-bit and bigcomp correction paths.
    let (c, r) = both();
    let hard = [
        "9007199254740993",
        "1e23",
        "8.98846567431158e307",
        "2.2250738585072011e-308",
        "0.500000000000000166533453693773481063544750213623046875",
        "1.0000000000000000000000000000000000000000001",
        "7.8459735791271921e65",
        "3.5844466002796428e+298",
        "179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497792",
        "2.47032822920623272e-324",
        "6.63089969твcorrupt", // deliberately truncated/garbage tail
    ];
    unsafe {
        for t in hard {
            cmp_strtod_unused(c, r, t);
        }
        // Random 30-digit mantissas with wide exponents.
        let mut rng = Rng::new(0x5C_000D);
        for _ in 0..5000 {
            let mut s = String::new();
            for i in 0..30 {
                let d = rng.below(10);
                if i == 0 && d == 0 {
                    s.push('1');
                } else {
                    s.push((b'0' + d as u8) as char);
                }
            }
            s.push('.');
            for _ in 0..rng.below(30) {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
            s.push('e');
            s.push_str(&rng.range(-330, 330).to_string());
            cmp_strtod_unused(c, r, &s);
        }
    }
}

#[test]
fn dtoa_divmax_data_symbol() {
    let _g = global_state_lock();
    let (c, r) = both();
    diff_eq!(c.dtoa_divmax(), r.dtoa_divmax(), "dtoa_divmax");
    assert_eq!(c.dtoa_divmax(), 2, "C: dtoa_divmax initialises to 2");
}
