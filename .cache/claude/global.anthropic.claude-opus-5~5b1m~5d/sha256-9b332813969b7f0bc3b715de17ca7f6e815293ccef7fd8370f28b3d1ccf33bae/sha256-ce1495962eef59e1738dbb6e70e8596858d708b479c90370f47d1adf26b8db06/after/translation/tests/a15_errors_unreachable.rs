//! Phase C — the residual ERRORS.md rows that CANNOT be exercised in-process,
//! and the guards that make them unreachable.
//!
//! Rows 297, 311, 314, 323, 324, 325, 342, 346 and 349 all describe conditions
//! that, in the C itself, either
//!   * abort the process via a live `assert()` (the build defines no `NDEBUG`),
//!   * dereference NULL / read out of bounds, i.e. are undefined behaviour in
//!     the C and therefore have no defined behaviour for a translation to match,
//!   * or are dead code that a correct caller cannot reach.
//!
//! A differential test that merely confirmed "both implementations crash" would
//! prove nothing about equivalence, and faking a passing assertion would be
//! worse than leaving the row open. So for each row this file EITHER
//!   (a) verifies, through the FFI, the guard that makes the row unreachable, or
//!   (b) verifies the observable half of the row that IS reachable,
//! and documents the rest by quoting the C.
//!
//! These rows are marked `[-] <reason>` in ERRORS.md, not `[x]`.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// Row 297 — hashtable_set's key-length overflow guard is DEAD CODE
// ===========================================================================

#[test]
fn row_297_key_length_guard_is_dead_code() {
    let _g = global_state_lock();
    // The guard lives in init_pair:
    //
    //     if (key_len >= (size_t)-1 - offsetof(pair_t, key))
    //         return NULL;          /* "Avoid an overflow if the key is very long" */
    //
    // But hashtable_set computes the hash BEFORE calling init_pair:
    //
    //     hash = hash_str(key, key_len);        <-- reads key_len bytes
    //     ...
    //     pair = init_pair(value, key, key_len, hash);
    //
    // so any key_len big enough to satisfy the guard makes `hashlittle` read
    // ~SIZE_MAX bytes and segfault first. The guard is therefore unreachable
    // without undefined behaviour, and is not testable.
    //
    // What IS verifiable is that the neighbourhood below the guard behaves
    // identically: genuinely large keys are accepted and round-trip.
    let (c, r) = both();
    unsafe {
        let mut cht = Box::new(hashtable_t::zeroed());
        let mut rht = Box::new(hashtable_t::zeroed());
        assert_eq!((c.hashtable_init)(&mut *cht), 0);
        assert_eq!((r.hashtable_init)(&mut *rht), 0);

        for len in [1usize, 255, 4096, 65_536, 1_000_000] {
            let key = vec![b'k'; len];
            diff_eq!(
                (c.hashtable_set)(&mut *cht, key.as_ptr() as *const c_char, len, (c.json_integer)(len as i64)),
                (r.hashtable_set)(&mut *rht, key.as_ptr() as *const c_char, len, (r.json_integer)(len as i64)),
                "hashtable_set with a {len}-byte key"
            );
            diff_eq!(
                (c.json_integer_value)(
                    (c.hashtable_get)(&mut *cht, key.as_ptr() as *const c_char, len) as *mut json_t
                ),
                (r.json_integer_value)(
                    (r.hashtable_get)(&mut *rht, key.as_ptr() as *const c_char, len) as *mut json_t
                ),
                "hashtable_get with a {len}-byte key"
            );
        }
        diff_eq!((cht.size, cht.order), (rht.size, rht.order), "state with large keys");
        (c.hashtable_close)(&mut *cht);
        (r.hashtable_close)(&mut *rht);
    }
}

// ===========================================================================
// Row 311 — json_set_alloc_funcs does no validation
// ===========================================================================

#[test]
fn row_311_set_alloc_funcs_accepts_null_without_validation() {
    let _g = global_state_lock();
    // The row says: "accepted with no validation; every later allocation calls a
    // NULL pointer => crash". The CRASH half is undefined behaviour and is not
    // testable. The ACCEPTANCE half is perfectly observable, and it is the part a
    // translation could get wrong (e.g. by defensively substituting libc malloc,
    // or by panicking): install NULL hooks, read them straight back through the
    // getters, and restore the originals before anything allocates.
    let (c, r) = both();
    unsafe {
        let (mut cm0, mut crl0, mut cf0) = (None, None, None);
        let (mut rm0, mut rrl0, mut rf0) = (None, None, None);
        (c.json_get_alloc_funcs2)(&mut cm0, &mut crl0, &mut cf0);
        (r.json_get_alloc_funcs2)(&mut rm0, &mut rrl0, &mut rf0);

        // --- 2-arg setter with NULL malloc and NULL free
        (c.json_set_alloc_funcs)(None, None);
        (r.json_set_alloc_funcs)(None, None);
        let (mut cm, mut crl, mut cf) = (None, None, None);
        let (mut rm, mut rrl, mut rf) = (None, None, None);
        (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
        (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);
        diff_eq!(
            (cm.is_none(), crl.is_none(), cf.is_none()),
            (rm.is_none(), rrl.is_none(), rf.is_none()),
            "set_alloc_funcs(NULL, NULL) stores NULL in all three slots"
        );
        assert!(
            cm.is_none() && crl.is_none() && cf.is_none(),
            "C: set_alloc_funcs must store the NULLs verbatim (and NULL the realloc slot)"
        );

        // --- 3-arg setter with all three NULL
        (c.json_set_alloc_funcs2)(None, None, None);
        (r.json_set_alloc_funcs2)(None, None, None);
        let (mut cm, mut crl, mut cf) = (None, None, None);
        let (mut rm, mut rrl, mut rf) = (None, None, None);
        (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
        (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);
        diff_eq!(
            (cm.is_none(), crl.is_none(), cf.is_none()),
            (rm.is_none(), rrl.is_none(), rf.is_none()),
            "set_alloc_funcs2(NULL, NULL, NULL) stores NULL in all three slots"
        );

        // --- mixed: a real malloc with a NULL free, and vice versa
        (c.json_set_alloc_funcs2)(Some(plain_malloc), None, None);
        (r.json_set_alloc_funcs2)(Some(plain_malloc), None, None);
        let (mut cm, mut crl, mut cf) = (None, None, None);
        let (mut rm, mut rrl, mut rf) = (None, None, None);
        (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
        (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);
        diff_eq!(
            (cm.is_some(), crl.is_none(), cf.is_none()),
            (rm.is_some(), rrl.is_some() == false, rf.is_none()),
            "set_alloc_funcs2 with a partially-NULL trio"
        );

        // --- restore BEFORE any allocation happens
        (c.json_set_alloc_funcs2)(cm0, crl0, cf0);
        (r.json_set_alloc_funcs2)(rm0, rrl0, rf0);
        let o = (c.json_object)();
        assert!(!o.is_null(), "C allocator not restored");
        decref(c, o);
        let o = (r.json_object)();
        assert!(!o.is_null(), "Rust allocator not restored");
        decref(r, o);
    }
}

extern "C" {
    fn malloc(n: size_t) -> *mut c_void;
}
unsafe extern "C" fn plain_malloc(n: size_t) -> *mut c_void {
    malloc(n)
}

// ===========================================================================
// Row 314 — jsonp_strtod's assert on incomplete input
// ===========================================================================

#[test]
fn row_314_jsonp_strtod_assert_guard_is_upheld_by_every_caller() {
    let _g = global_state_lock();
    // jsonp_strtod contains a LIVE assert (the build passes no -DNDEBUG):
    //
    //     value = strtod(strbuffer->value, &end);
    //     assert(end == strbuffer->value + strbuffer->length);
    //
    // Handing it "abc" or "1x" aborts the process with SIGABRT, so the row is
    // not testable in-process and a "both abort" test would prove nothing.
    //
    // What IS verifiable is the guard: the library's only caller
    // (lex_scan_number in load.c) has already validated the literal
    // character-by-character, so the buffer is always fully consumed. Drive that
    // path over every number form the lexer accepts and confirm both
    // implementations agree — if either mis-lexed a number it would either abort
    // here or produce a different value.
    let (c, r) = both();
    let mut rng = Rng::new(0xE_0314);
    unsafe {
        let mut texts: Vec<String> = vec![
            "0", "-0", "1", "-1", "0.0", "-0.0", "1.5", "-1.5", "1e0", "1E0", "1e+0",
            "1e-0", "1e10", "1E10", "1e+10", "1e-10", "0.1", "1.25e3", "-9.5E-7",
            "123456789.123456789", "1e308", "1e-308", "1e309", "1e-309",
            "1.7976931348623157e308", "4.9406564584124654e-324",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        // Randomised, but only forms lex_scan_number actually accepts.
        for _ in 0..4000 {
            let mut s = String::new();
            if rng.bool() {
                s.push('-');
            }
            let intpart = rng.below(10);
            if intpart == 0 {
                s.push('0');
            } else {
                s.push((b'0' + intpart as u8) as char);
                for _ in 0..rng.below(18) {
                    s.push((b'0' + rng.below(10) as u8) as char);
                }
            }
            if rng.bool() {
                s.push('.');
                for _ in 0..1 + rng.below(18) {
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
                s.push_str(&rng.range(0, 330).to_string());
            }
            texts.push(s);
        }

        for t in texts {
            // (a) directly, through a fully-consumed strbuffer — the guard holds
            let run = |api: &Api| -> (c_int, u64) {
                let mut sb = strbuffer_t::zeroed();
                (api.strbuffer_init)(&mut sb);
                let b = t.as_bytes();
                (api.strbuffer_append_bytes)(&mut sb, b.as_ptr() as *const c_char, b.len());
                let mut out = 0.0f64;
                let ret = (api.jsonp_strtod)(&mut sb, &mut out);
                (api.strbuffer_close)(&mut sb);
                (ret, out.to_bits())
            };
            diff_eq!(run(c), run(r), "jsonp_strtod({t:?}) via a complete strbuffer");

            // (b) through the real caller, which is what the guard protects
            let doc = format!("[{t}]");
            let d = cs(&doc);
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let cj = (c.json_loads)(d.as_ptr(), 0, &mut ce);
            let rj = (r.json_loads)(d.as_ptr(), 0, &mut re);
            diff_eq!(cj.is_null(), rj.is_null(), "json_loads({doc:?}) null-ness");
            diff_eq!(ce.raw(), re.raw(), "json_loads({doc:?}) error image");
            if !cj.is_null() {
                let cd = (c.json_dumps)(cj, 0);
                let rd = (r.json_dumps)(rj, 0);
                diff_eq!(cbytes(cd), cbytes(rd), "json_loads({doc:?}) re-dump");
                jfree(c, cd as *mut c_void);
                jfree(r, rd as *mut c_void);
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 323 / 324 / 325 — dtoa's missing NULL checks
// ===========================================================================

#[test]
fn rows_323_324_dtoa_always_allocates_and_never_returns_null_for_finite_input() {
    let _g = global_state_lock();
    // Row 323: `Balloc` does not check MALLOC's result — `rv->sign = rv->wds = 0`
    // dereferences NULL on OOM and segfaults. Undefined behaviour in the C, so
    // not differentially testable.
    //
    // Row 324: `dtoa` forwards to `dtoa_r(..., buf = 0, blen = 0)`, so it always
    // ALLOCATES; the `blen`/`nrv_alloc` short-buffer failure paths that
    // `dtoa_r` exposes therefore cannot trigger through `dtoa`, and the only
    // failure left is row 323's crash.
    //
    // Row 324's positive content is verifiable and worth pinning: `dtoa` must
    // never return NULL, for ANY input, at any mode/ndigits.
    let (c, r) = both();
    let mut rng = Rng::new(0xE_0324);
    unsafe {
        let mut values: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.1, 1e308, -1e308, 5e-324, f64::MAX, -f64::MAX,
            f64::MIN_POSITIVE, 1.0 / 3.0,
            f64::INFINITY, f64::NEG_INFINITY, f64::NAN, -f64::NAN,
        ];
        for _ in 0..2000 {
            values.push(rng.real());
            values.push(f64::from_bits(rng.next_u64()));
        }
        for v in values {
            for mode in 0..=9 {
                for ndigits in [0, 1, 17, 30] {
                    let (mut cd, mut cs2) = (-12345, -12345);
                    let (mut rd, mut rs2) = (-12345, -12345);
                    let mut crve: *mut c_char = std::ptr::null_mut();
                    let mut rrve: *mut c_char = std::ptr::null_mut();
                    let cp = (c.dtoa)(v, mode, ndigits, &mut cd, &mut cs2, &mut crve);
                    let rp = (r.dtoa)(v, mode, ndigits, &mut rd, &mut rs2, &mut rrve);
                    let ctx = format!("bits={:#018x} mode={mode} ndigits={ndigits}", v.to_bits());
                    // Row 324: dtoa allocates, so it cannot fail on buffer size.
                    assert!(!cp.is_null(), "C: dtoa returned NULL [{ctx}]");
                    diff_eq!(cp.is_null(), rp.is_null(), "dtoa null-ness [{ctx}]");
                    diff_eq!(cbytes(cp), cbytes(rp), "dtoa digits [{ctx}]");
                    diff_eq!(cd, rd, "dtoa *decpt [{ctx}]");
                    diff_eq!(cs2, rs2, "dtoa *sign [{ctx}]");
                    (c.freedtoa)(cp);
                    (r.freedtoa)(rp);
                }
            }
        }
    }
}

#[test]
fn row_325_freedtoa_requires_a_non_null_pointer() {
    let _g = global_state_lock();
    // freedtoa's first act is to recover the block header:
    //
    //     Bigint *b = (Bigint*)((int*)s - 1);
    //
    // With `s == NULL` that is a NULL dereference — undefined behaviour in the C,
    // with no defined result for the Rust to reproduce, so the row is marked
    // `[-]` rather than tested.
    //
    // The reachable, meaningful property is that freedtoa correctly recycles a
    // real dtoa result: allocate and free many times, interleaved, and require
    // the digits produced AFTER each recycle to keep matching. A freelist
    // mismatch between the two implementations would show up here as diverging
    // output (or a crash) within a few iterations.
    let (c, r) = both();
    let mut rng = Rng::new(0xE_0325);
    unsafe {
        for i in 0..20000 {
            let v = rng.real();
            let mode = rng.below(10) as c_int;
            let ndigits = rng.below(30) as c_int;
            let (mut cd, mut cs2) = (0, 0);
            let (mut rd, mut rs2) = (0, 0);
            let mut crve: *mut c_char = std::ptr::null_mut();
            let mut rrve: *mut c_char = std::ptr::null_mut();
            let cp = (c.dtoa)(v, mode, ndigits, &mut cd, &mut cs2, &mut crve);
            let rp = (r.dtoa)(v, mode, ndigits, &mut rd, &mut rs2, &mut rrve);
            diff_eq!(cbytes(cp), cbytes(rp), "iter {i}: digits before free");
            (c.freedtoa)(cp);
            (r.freedtoa)(rp);
            // Immediately reuse the just-freed block at a different size class.
            let big = rng.below(30) as c_int;
            let cp2 = (c.dtoa)(v, 2, big, &mut cd, &mut cs2, &mut crve);
            let rp2 = (r.dtoa)(v, 2, big, &mut rd, &mut rs2, &mut rrve);
            diff_eq!(cbytes(cp2), cbytes(rp2), "iter {i}: digits after recycle");
            diff_eq!(cd, rd, "iter {i}: *decpt after recycle");
            (c.freedtoa)(cp2);
            (r.freedtoa)(rp2);
        }
    }
}

// ===========================================================================
// Rows 342 / 346 / 349 — load.c's internal lexer asserts
// ===========================================================================

#[test]
fn rows_342_346_349_lexer_asserts_are_unreachable_by_construction() {
    let _g = global_state_lock();
    // Row 349: `assert(str[0] == 'u')` at the top of decode_unicode_escape.
    // Row 342: the non-hex-digit branch inside decode_unicode_escape, whose
    //          caller turns -1 into "invalid Unicode escape".
    // Row 346: `assert` in stream_unget on the buffer position / byte identity.
    //
    // All three are internal invariants: lex_scan_string's FIRST pass has
    // already checked that a `\u` escape is followed by exactly four hex digits
    // (and that the escape character itself is legal) before the second pass
    // calls decode_unicode_escape, and every unget is paired with a preceding
    // get of the same byte. Reaching them requires corrupting library-internal
    // state, so they abort rather than return, and are marked `[-]`.
    //
    // The guards themselves are fully testable from outside, which is what this
    // test does: every malformed `\u` escape must be rejected by the FIRST pass,
    // with both implementations producing the identical error — never an abort.
    let (c, r) = both();
    unsafe {
        let mut cases: Vec<String> = Vec::new();
        // Truncated escapes at every length.
        for n in 0..4 {
            cases.push(format!("[\"\\u{}\"]", "0".repeat(n)));
            cases.push(format!("[\"\\u{}", "0".repeat(n)));
        }
        // A non-hex digit in each of the four positions, for a spread of
        // offending characters (including ones adjacent to the hex ranges).
        for pos in 0..4 {
            for bad in [
                'g', 'G', 'z', 'Z', '/', ':', '@', '`', 'x', ' ', '\t', '"', '\\', '-',
                '+', '.', '\u{7f}', '!', '~', '\u{0}' as char,
            ] {
                if bad == '\u{0}' {
                    continue; // needs cs_bytes; covered separately below
                }
                let mut digits = ['0'; 4];
                digits[pos] = bad;
                cases.push(format!("[\"\\u{}\"]", digits.iter().collect::<String>()));
            }
        }
        // Every non-hex ASCII byte in the first position.
        for b in 0x21u8..0x7f {
            if b.is_ascii_hexdigit() || b == b'"' || b == b'\\' {
                continue;
            }
            cases.push(format!("[\"\\u{}000\"]", b as char));
        }
        // Every illegal escape character (row 344's guard, same first pass).
        for b in 0x20u8..0x7f {
            if b"\"\\/bfnrtu".contains(&b) {
                continue;
            }
            cases.push(format!("[\"\\{}\"]", b as char));
        }
        // Surrogate handling: all four broken-pair shapes.
        cases.extend(
            [
                r#"["\ud834"]"#,          // lone high surrogate
                r#"["\udd1e"]"#,          // lone low surrogate
                r#"["\ud834\u0041"]"#,    // high not followed by a low
                r#"["\ud834\ud834"]"#,    // high followed by another high
                r#"["\ud834x"]"#,         // high followed by a raw char
                r#"["\ud834\"]"#,         // high followed by a bare backslash
                r#"["\ud834\u"]"#,        // high followed by a truncated escape
                r#"["\ud834\udd1e"]"#,    // VALID pair — must succeed
                r#"["\u0000"]"#,          // NUL escape (needs JSON_ALLOW_NUL)
            ]
            .iter()
            .map(|s| s.to_string()),
        );

        for doc in cases {
            for flags in [0, JSON_ALLOW_NUL, JSON_DECODE_ANY, JSON_ALLOW_NUL | JSON_DECODE_ANY] {
                let d = cs(&doc);
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let cj = (c.json_loads)(d.as_ptr(), flags, &mut ce);
                let rj = (r.json_loads)(d.as_ptr(), flags, &mut re);
                diff_eq!(
                    cj.is_null(),
                    rj.is_null(),
                    "json_loads({doc:?}, flags={flags:#x}) null-ness"
                );
                // The full error image pins code, text, line, column and position.
                diff_eq!(
                    ce.raw(),
                    re.raw(),
                    "json_loads({doc:?}, flags={flags:#x}) error image"
                );
                if !cj.is_null() {
                    let cd = (c.json_dumps)(cj, JSON_ENCODE_ANY | JSON_ENSURE_ASCII);
                    let rd = (r.json_dumps)(rj, JSON_ENCODE_ANY | JSON_ENSURE_ASCII);
                    diff_eq!(
                        cbytes(cd),
                        cbytes(rd),
                        "json_loads({doc:?}, flags={flags:#x}) re-dump"
                    );
                    jfree(c, cd as *mut c_void);
                    jfree(r, rd as *mut c_void);
                }
                decref(c, cj);
                decref(r, rj);
            }
        }

        // A NUL byte inside the escape digits, which needs a length-carrying
        // entry point to express at all.
        let raw = b"[\"\\u00\x0000\"]";
        let buf = cs_bytes(raw);
        for flags in [0, JSON_ALLOW_NUL] {
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let cj = (c.json_loadb)(buf.as_ptr(), raw.len(), flags, &mut ce);
            let rj = (r.json_loadb)(buf.as_ptr(), raw.len(), flags, &mut re);
            diff_eq!(cj.is_null(), rj.is_null(), "NUL in \\u digits null-ness");
            diff_eq!(ce.raw(), re.raw(), "NUL in \\u digits error image");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

#[test]
fn row_346_unget_invariant_holds_across_every_token_boundary() {
    let _g = global_state_lock();
    // stream_unget's assert encodes "every unget returns the byte that was just
    // read". The lexer ungets at token boundaries (after a number, after an
    // identifier, after skipping whitespace), so exercising every boundary shape
    // — including multi-byte UTF-8 immediately before a boundary, and newlines
    // that make the lexer restore `last_column` — is the strongest available
    // check that both implementations keep the invariant.
    let (c, r) = both();
    let mut rng = Rng::new(0xE_0346);
    unsafe {
        let fragments = [
            "1", "-1", "1.5", "1e5", "0", "true", "false", "null", "\"s\"", "[]", "{}",
            "[1]", "{\"k\":1}", "\"\\u00e9\"", "\"é\"", "\"€\"", "\"😀\"", "\"a\\nb\"",
        ];
        let seps = ["", " ", "\t", "\n", "\r", "\r\n", "  \n\t ", ",", ":"];
        for _ in 0..6000 {
            // Assemble a document from fragments and separators; many will be
            // malformed, which is exactly where the unget paths are busiest.
            let mut doc = String::new();
            let n = 1 + rng.below(6);
            for _ in 0..n {
                doc.push_str(rng.choice(&fragments));
                doc.push_str(rng.choice(&seps));
            }
            if rng.bool() {
                doc = format!("[{doc}]");
            } else if rng.bool() {
                doc = format!("{{{doc}}}");
            }
            let flags = *rng.choice(&[
                0,
                JSON_DECODE_ANY,
                JSON_DISABLE_EOF_CHECK,
                JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
                JSON_REJECT_DUPLICATES,
                JSON_ALLOW_NUL,
                JSON_DECODE_INT_AS_REAL,
            ]);
            let d = cs(&doc);
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let cj = (c.json_loads)(d.as_ptr(), flags, &mut ce);
            let rj = (r.json_loads)(d.as_ptr(), flags, &mut re);
            diff_eq!(cj.is_null(), rj.is_null(), "json_loads({doc:?}, {flags:#x}) null-ness");
            diff_eq!(ce.raw(), re.raw(), "json_loads({doc:?}, {flags:#x}) error image");
            if !cj.is_null() {
                let cd = (c.json_dumps)(cj, JSON_ENCODE_ANY | JSON_SORT_KEYS);
                let rd = (r.json_dumps)(rj, JSON_ENCODE_ANY | JSON_SORT_KEYS);
                diff_eq!(cbytes(cd), cbytes(rd), "json_loads({doc:?}, {flags:#x}) re-dump");
                jfree(c, cd as *mut c_void);
                jfree(r, rd as *mut c_void);
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}
