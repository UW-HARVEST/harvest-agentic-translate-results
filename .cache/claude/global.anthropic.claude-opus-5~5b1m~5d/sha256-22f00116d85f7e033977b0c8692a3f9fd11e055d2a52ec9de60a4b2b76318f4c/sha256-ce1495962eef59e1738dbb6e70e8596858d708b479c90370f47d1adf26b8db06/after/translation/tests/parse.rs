//! Phase B — CONFIGS.md rows 35–48: the four parse entry points across the full
//! option cross-product, every `buffer_length` relation, BOM/whitespace
//! handling, every value shape, the parse→print round trip, and
//! `cJSON_GetErrorPtr` (compared as an offset into the caller's buffer).
#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::{c_char, c_int};

/// A NUL-terminated document with 16 extra NUL bytes of padding, so that
/// `buffer_length` values larger than `strlen + 1` stay in bounds and both
/// libraries observe identical bytes.
struct Doc {
    buf: Vec<u8>,
    text_len: usize,
}

impl Doc {
    fn new(text: &[u8]) -> Doc {
        let mut buf = text.to_vec();
        buf.push(0);
        buf.extend_from_slice(&[0u8; 16]);
        Doc {
            buf,
            text_len: text.len(),
        }
    }
    fn ptr(&self) -> *const c_char {
        self.buf.as_ptr() as *const c_char
    }
    /// The lengths cJSON distinguishes: shorter than the text, exactly the
    /// text, text+NUL (canonical), and longer.
    fn lengths(&self) -> Vec<usize> {
        let n = self.text_len;
        let mut v = vec![0, 1, n / 2, n.saturating_sub(1), n, n + 1, n + 2, n + 8];
        v.sort_unstable();
        v.dedup();
        v
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Parse,
    WithLength(usize),
    WithOpts { want_end: bool, rnt: c_int },
    WithLengthOpts { len: usize, want_end: bool, rnt: c_int },
}

#[derive(Debug, PartialEq)]
struct ParseResult {
    is_null: bool,
    snapshot: Option<NodeSnap>,
    printed: Option<Vec<u8>>,
    unformatted: Option<Vec<u8>>,
    /// `*return_parse_end - value`, or `None` when `return_parse_end` was not
    /// requested / left untouched.
    end_offset: Option<isize>,
    /// `cJSON_GetErrorPtr() - value`, or `None` when it returned NULL.
    error_offset: Option<isize>,
    error_is_null: bool,
}

unsafe fn do_parse(api: &Api, doc: &Doc, mode: Mode) -> ParseResult {
    let base = doc.ptr();
    let sentinel = 0xDEAD_BEEFusize as *const c_char;
    let mut end: *const c_char = sentinel;

    let item = match mode {
        Mode::Parse => (api.cJSON_Parse)(base),
        Mode::WithLength(len) => (api.cJSON_ParseWithLength)(base, len),
        Mode::WithOpts { want_end, rnt } => (api.cJSON_ParseWithOpts)(
            base,
            if want_end { &mut end } else { std::ptr::null_mut() },
            rnt,
        ),
        Mode::WithLengthOpts { len, want_end, rnt } => (api.cJSON_ParseWithLengthOpts)(
            base,
            len,
            if want_end { &mut end } else { std::ptr::null_mut() },
            rnt,
        ),
    };

    let err = (api.cJSON_GetErrorPtr)();
    let res = ParseResult {
        is_null: item.is_null(),
        snapshot: snap(item),
        printed: print_and_take(api, item),
        unformatted: print_unformatted_and_take(api, item),
        end_offset: if end == sentinel {
            None
        } else {
            Some(end as isize - base as isize)
        },
        error_offset: if err.is_null() {
            None
        } else {
            Some(err as isize - base as isize)
        },
        error_is_null: err.is_null(),
    };
    (api.cJSON_Delete)(item);
    res
}

fn check(c: &Api, r: &Api, text: &[u8], mode: Mode) {
    // `global_error` is per-library process-wide state; hold the lock across
    // both calls so concurrent tests cannot interleave.
    let _guard = lock_global_state();
    let doc = Doc::new(text);
    unsafe {
        let a = do_parse(c, &doc, mode);
        let b = do_parse(r, &doc, mode);
        if a != b {
            panic!(
                "parse mismatch\ninput = {:?} (len {})\nmode  = {mode:?}\nC     = {a:#?}\nRust  = {b:#?}",
                String::from_utf8_lossy(text),
                text.len()
            );
        }
    }
}

/// Runs one document through the FULL entry-point × option cross-product
/// (CONFIGS rows 35–40).
fn check_all_modes(c: &Api, r: &Api, text: &[u8]) {
    let doc = Doc::new(text);
    check(c, r, text, Mode::Parse);
    for len in doc.lengths() {
        check(c, r, text, Mode::WithLength(len));
    }
    for want_end in [false, true] {
        for rnt in [0, 1, 2, -1] {
            check(c, r, text, Mode::WithOpts { want_end, rnt });
            for len in doc.lengths() {
                check(
                    c,
                    r,
                    text,
                    Mode::WithLengthOpts { len, want_end, rnt },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// document corpus
// ---------------------------------------------------------------------------

fn valid_documents() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"null".to_vec(),
        b"true".to_vec(),
        b"false".to_vec(),
        b"0".to_vec(),
        b"-0".to_vec(),
        b"1".to_vec(),
        b"-1".to_vec(),
        b"123".to_vec(),
        b"1.5".to_vec(),
        b"-1.5e10".to_vec(),
        b"1E+2".to_vec(),
        b"1e-7".to_vec(),
        b"0.000001".to_vec(),
        b"1e309".to_vec(),
        b"-1e309".to_vec(),
        b"1e-400".to_vec(),
        b"2147483647".to_vec(),
        b"2147483648".to_vec(),
        b"-2147483648".to_vec(),
        b"-2147483649".to_vec(),
        b"9007199254740993".to_vec(),
        b"123456789012345678901234567890".to_vec(),
        b"0123".to_vec(),
        b"1.".to_vec(),
        b"00".to_vec(),
        b"-0.0".to_vec(),
        b"\"\"".to_vec(),
        b"\"a\"".to_vec(),
        b"\"hello world\"".to_vec(),
        br#""\b\f\n\r\t\"\\\/""#.to_vec(),
        br#""\u0000""#.to_vec(),
        br#""\u0041""#.to_vec(),
        br#""\u00e9""#.to_vec(),
        br#""\u07ff""#.to_vec(),
        br#""\u0800""#.to_vec(),
        br#""\uffff""#.to_vec(),
        br#""\ud800\udc00""#.to_vec(),
        br#""\udbff\udfff""#.to_vec(),
        br#""\uD83D\uDE00""#.to_vec(),
        br#""\uZZZZ""#.to_vec(), // parse_hex4 → 0 → embeds a NUL byte
        br#""\u12G4""#.to_vec(),
        b"\"\x80\xff\"".to_vec(),
        b"\"\xc3\xa9\xe4\xb8\xad\"".to_vec(),
        b"[]".to_vec(),
        b"[1]".to_vec(),
        b"[1,2,3]".to_vec(),
        b"[ 1 , 2 , 3 ]".to_vec(),
        b"[[[[[]]]]]".to_vec(),
        b"[null,true,false,0,\"\",[],{}]".to_vec(),
        b"{}".to_vec(),
        b"{\"a\":1}".to_vec(),
        b"{ \"a\" : 1 , \"b\" : 2 }".to_vec(),
        b"{\"a\":1,\"a\":2}".to_vec(),
        b"{\"A\":1,\"a\":2}".to_vec(),
        b"{\"\":1}".to_vec(),
        b"{\"a\":{\"b\":{\"c\":[1,2,{\"d\":null}]}}}".to_vec(),
        b"\t\r\n {\"a\" : [ 1 , 2 ] } \t\r\n ".to_vec(),
        b"\x01\x02[1]".to_vec(), // bytes <= 32 count as whitespace
        b"\x1f[1]".to_vec(),
        b"[1] trailing".to_vec(),
        b"1 2".to_vec(),
        b"nullnull".to_vec(),
        b"truex".to_vec(),
        b"falsey".to_vec(),
        // BOM (row 41)
        b"\xEF\xBB\xBF{\"a\":1}".to_vec(),
        b"\xEF\xBB\xBF[1]".to_vec(),
        b"\xEF\xBB\xBFnull".to_vec(),
        b"\xEF\xBB\xBF".to_vec(),
        b"\xEF\xBB".to_vec(),
        b"\xEF\xBB\xBF1".to_vec(),
        // near-BOM
        b"\xEF\xBB\xBE[1]".to_vec(),
    ];
    // whitespace-only and empty
    v.push(b"".to_vec());
    v.push(b" ".to_vec());
    v.push(b"   \t\r\n".to_vec());
    v
}

fn invalid_documents() -> Vec<Vec<u8>> {
    vec![
        b"-".to_vec(),
        b"-e".to_vec(),
        b"-.".to_vec(),
        b"-+".to_vec(),
        b"--1".to_vec(),
        b"+1".to_vec(),
        b".5".to_vec(),
        b"x".to_vec(),
        b"nul".to_vec(),
        b"tru".to_vec(),
        b"fals".to_vec(),
        b"NULL".to_vec(),
        b"TRUE".to_vec(),
        b"'a'".to_vec(),
        b"}".to_vec(),
        b"]".to_vec(),
        b",".to_vec(),
        b":".to_vec(),
        b"\"abc".to_vec(),
        b"\"abc\\".to_vec(),
        br#""\q""#.to_vec(),
        br#""\x41""#.to_vec(),
        br#""\ ""#.to_vec(),
        br#""\U0041""#.to_vec(),
        br#""\u12""#.to_vec(),
        br#""\u""#.to_vec(),
        br#""\u123""#.to_vec(),
        br#""\udc00""#.to_vec(),
        br#""\udfff""#.to_vec(),
        br#""\ud800\u12""#.to_vec(),
        br#""\ud800xxxxxx""#.to_vec(),
        br#""\ud800\ud800""#.to_vec(),
        br#""\ud800A""#.to_vec(),
        br#""\ud800""#.to_vec(),
        br#""\ud800\n1234""#.to_vec(),
        b"[".to_vec(),
        b"[,]".to_vec(),
        b"[1,]".to_vec(),
        b"[1,,2]".to_vec(),
        b"[x]".to_vec(),
        b"[1".to_vec(),
        b"[1 ".to_vec(),
        b"[1,2".to_vec(),
        b"[1}".to_vec(),
        b"{".to_vec(),
        b"{a".to_vec(),
        b"{x:1}".to_vec(),
        b"{1:2}".to_vec(),
        b"{'a':1}".to_vec(),
        b"{\"a:1}".to_vec(),
        b"{\"a\" 1}".to_vec(),
        b"{\"a\"}".to_vec(),
        b"{\"a\",1}".to_vec(),
        b"{\"a\":}".to_vec(),
        b"{\"a\":x}".to_vec(),
        b"{\"a\":,}".to_vec(),
        b"{\"a\":1".to_vec(),
        b"{\"a\":1 ".to_vec(),
        b"{\"a\":1,".to_vec(),
        b"{\"a\":1,}".to_vec(),
        b"{\"a\":1]".to_vec(),
    ]
}

// ---------------------------------------------------------------------------
// rows 35–43, 48 — the corpus through every entry point / option combination
// ---------------------------------------------------------------------------
#[test]
fn cfg35_43_valid_documents_all_modes() {
    let (c, r) = both();
    for doc in valid_documents() {
        check_all_modes(&c, &r, &doc);
    }
}

#[test]
fn cfg35_43_invalid_documents_all_modes() {
    let (c, r) = both();
    for doc in invalid_documents() {
        check_all_modes(&c, &r, &doc);
    }
}

// ---------------------------------------------------------------------------
// row 44 — strings: every escape and every raw byte
// ---------------------------------------------------------------------------
#[test]
fn cfg44_parse_strings_exhaustive() {
    let (c, r) = both();
    // every possible escape byte after a backslash
    for b in 1u16..=255 {
        let mut t = Vec::new();
        t.push(b'"');
        t.push(b'\\');
        t.push(b as u8);
        t.extend_from_slice(b"0000\"");
        check_all_modes(&c, &r, &t);
    }
    // every raw byte inside a string literal
    for b in 1u16..=255 {
        if b as u8 == b'"' || b as u8 == b'\\' {
            continue;
        }
        let t = vec![b'"', b as u8, b'"'];
        check(&c, &r, &t, Mode::Parse);
        check(&c, &r, &t, Mode::WithOpts { want_end: true, rnt: 1 });
    }
    // \uXXXX across the whole BMP boundary set, plus surrogate combinations
    let interesting: Vec<u32> = vec![
        0x0000, 0x0001, 0x001F, 0x0020, 0x007F, 0x0080, 0x00FF, 0x07FF, 0x0800, 0x0FFF, 0x1000,
        0xD7FF, 0xD800, 0xD801, 0xDBFF, 0xDC00, 0xDC01, 0xDFFF, 0xE000, 0xFFFD, 0xFFFF,
    ];
    for &hi in &interesting {
        let t = format!("\"\\u{hi:04x}\"").into_bytes();
        check_all_modes(&c, &r, &t);
        for &lo in &interesting {
            let t = format!("\"\\u{hi:04x}\\u{lo:04x}\"").into_bytes();
            check(&c, &r, &t, Mode::Parse);
            check(&c, &r, &t, Mode::WithOpts { want_end: true, rnt: 1 });
        }
    }
    // upper/lower-case hex digits and invalid hex digits in each of the 4 slots
    for slot in 0..4 {
        for ch in [b'0', b'9', b'a', b'f', b'A', b'F', b'g', b'G', b'/', b':', b'@', b'`', 0x80] {
            let mut hex = *b"1234";
            hex[slot] = ch;
            let mut t = vec![b'"', b'\\', b'u'];
            t.extend_from_slice(&hex);
            t.push(b'"');
            check(&c, &r, &t, Mode::Parse);
            check(&c, &r, &t, Mode::WithOpts { want_end: true, rnt: 1 });
        }
    }
}

// ---------------------------------------------------------------------------
// row 43 — numbers: exhaustive shapes plus randomized round-trip
// ---------------------------------------------------------------------------
#[test]
fn cfg43_parse_numbers() {
    let (c, r) = both();
    let mut texts: Vec<Vec<u8>> = Vec::new();
    for s in [
        "0", "-0", "1", "-1", "10", "-10", "007", "1.0", "1.00", "0.1", "-0.1", ".1", "1.",
        "1e0", "1e1", "1E1", "1e+1", "1e-1", "1e10", "1e100", "1e308", "1e309", "1e-308",
        "1e-324", "1e-400", "-1e309", "2147483647", "2147483648", "-2147483648", "-2147483649",
        "4294967296", "9223372036854775807", "1.7976931348623157e308", "5e-324",
        "2.2250738585072014e-308", "0.30000000000000004", "3.141592653589793",
        "123456789012345", "1234567890123456", "12345678901234567",
        "1e", "1e+", "1e-", "1.2.3", "1-2", "1+2", "--1", "1ee1", "0x10", "1_000",
    ] {
        texts.push(s.as_bytes().to_vec());
        texts.push(format!("[{s}]").into_bytes());
        texts.push(format!("{{\"n\":{s}}}").into_bytes());
    }
    for t in &texts {
        check_all_modes(&c, &r, t);
    }

    // randomized: print a random double with maximum precision, parse it back
    let mut rng = Rng::new(0x4343_4343);
    for _ in 0..3000 {
        let d = if rng.bool() { rng.json_f64() } else { rng.any_f64() };
        if !d.is_finite() {
            continue;
        }
        let t = format!("{d:.*e}", 20).into_bytes();
        check(&c, &r, &t, Mode::Parse);
        let t = format!("{d}").into_bytes();
        check(&c, &r, &t, Mode::Parse);
    }
}

// ---------------------------------------------------------------------------
// rows 45, 46 — deep nesting up to and past CJSON_NESTING_LIMIT
// ---------------------------------------------------------------------------
#[test]
fn cfg45_46_nesting() {
    let (c, r) = both();
    for depth in [1usize, 2, 10, 100, 500, 998, 999, 1000, 1001, 1002, 1500] {
        let arr: Vec<u8> = {
            let mut v = vec![b'['; depth];
            v.extend(std::iter::repeat(b']').take(depth));
            v
        };
        check(&c, &r, &arr, Mode::Parse);
        check(&c, &r, &arr, Mode::WithOpts { want_end: true, rnt: 1 });

        let obj: Vec<u8> = {
            let mut v = Vec::new();
            for _ in 0..depth {
                v.extend_from_slice(b"{\"a\":");
            }
            v.push(b'1');
            for _ in 0..depth {
                v.push(b'}');
            }
            v
        };
        check(&c, &r, &obj, Mode::Parse);
        check(&c, &r, &obj, Mode::WithOpts { want_end: true, rnt: 1 });

        // alternating array/object nesting
        let alt: Vec<u8> = {
            let mut open = Vec::new();
            let mut close = Vec::new();
            for i in 0..depth {
                if i % 2 == 0 {
                    open.extend_from_slice(b"[");
                    close.insert(0, b']');
                } else {
                    open.extend_from_slice(b"{\"k\":");
                    close.insert(0, b'}');
                }
            }
            open.extend_from_slice(&close);
            open
        };
        check(&c, &r, &alt, Mode::Parse);
    }
}

// ---------------------------------------------------------------------------
// row 47 — parse → print round trip over generated documents
// ---------------------------------------------------------------------------
#[test]
fn cfg47_round_trip_generated() {
    let (c, r) = both();
    let mut rng = Rng::new(0x4747_4747);
    // Generate documents by building a random tree with the C library and
    // printing it, then feeding the text back through both parsers.
    for depth in [0usize, 1, 2, 3, 4] {
        for i in 0..120 {
            let spec = rand_spec(&mut rng, depth);
            unsafe {
                let bc = build(&c, &spec);
                let texts: Vec<Vec<u8>> = [
                    print_and_take(&c, bc.root),
                    print_unformatted_and_take(&c, bc.root),
                ]
                .into_iter()
                .flatten()
                .collect();
                bc.delete();
                for t in texts {
                    // Raw items can print non-JSON, so a re-parse may fail; the
                    // point is that BOTH sides fail (or succeed) identically.
                    check(&c, &r, &t, Mode::Parse);
                    check(&c, &r, &t, Mode::WithOpts { want_end: true, rnt: 1 });
                    check(
                        &c,
                        &r,
                        &t,
                        Mode::WithLengthOpts {
                            len: t.len() + 1,
                            want_end: true,
                            rnt: 1,
                        },
                    );
                    let _ = i;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 35–48 — mutation fuzzing: valid documents with random byte damage
// ---------------------------------------------------------------------------
#[test]
fn cfg35_48_mutation_fuzz() {
    let (c, r) = both();
    let mut rng = Rng::new(0x3548_3548);
    let seeds = valid_documents();
    let alphabet: Vec<u8> = {
        let mut v: Vec<u8> = b"{}[],:\"\\ \t\r\n0123456789-+.eEnullotrufaseXx/\x01".to_vec();
        for b in [0x7f, 0x80, 0xc3, 0xa9, 0xef, 0xbb, 0xbf, 0xff] {
            v.push(b);
        }
        v
    };
    for round in 0..6000 {
        let seed = &seeds[rng.below(seeds.len())];
        let mut t = seed.clone();
        let muts = 1 + rng.below(4);
        for _ in 0..muts {
            match rng.below(4) {
                0 if !t.is_empty() => {
                    let i = rng.below(t.len());
                    t[i] = alphabet[rng.below(alphabet.len())];
                }
                1 => {
                    let i = rng.below(t.len() + 1);
                    t.insert(i, alphabet[rng.below(alphabet.len())]);
                }
                2 if !t.is_empty() => {
                    let i = rng.below(t.len());
                    t.remove(i);
                }
                _ if !t.is_empty() => {
                    let n = rng.below(t.len()) + 1;
                    t.truncate(n);
                }
                _ => {}
            }
        }
        if t.contains(&0) {
            continue;
        }
        let mode = match round % 5 {
            0 => Mode::Parse,
            1 => Mode::WithOpts { want_end: true, rnt: 0 },
            2 => Mode::WithOpts { want_end: true, rnt: 1 },
            3 => Mode::WithLength(t.len() + 1),
            _ => Mode::WithLengthOpts {
                len: t.len(),
                want_end: true,
                rnt: 1,
            },
        };
        check(&c, &r, &t, mode);
    }
}

// ---------------------------------------------------------------------------
// row 48 — cJSON_GetErrorPtr state machine, including the "not reset" quirk of
// cJSON_ParseWithOpts(NULL, …)
// ---------------------------------------------------------------------------
#[test]
fn cfg48_error_ptr_state_machine() {
    let (c, r) = both();
    let _guard = lock_global_state();
    {
        // Identical call sequence on each side, results compared afterwards.
        fn run(api: &Api) -> Vec<(bool, isize)> {
            let mut out = Vec::new();
            unsafe {
                let bad = Doc::new(b"[1,2,x]");
                let good = Doc::new(b"[1,2,3]");

                // 1. failing parse leaves a non-NULL error pointer
                let it = (api.cJSON_Parse)(bad.ptr());
                assert!(it.is_null());
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize - bad.ptr() as isize));

                // 2. cJSON_Parse(NULL) — ParseWithOpts returns early WITHOUT
                //    resetting global_error, so the previous pointer survives.
                let it = (api.cJSON_Parse)(std::ptr::null());
                assert!(it.is_null());
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize - bad.ptr() as isize));

                // 3. successful parse resets it to NULL
                let it = (api.cJSON_Parse)(good.ptr());
                assert!(!it.is_null());
                (api.cJSON_Delete)(it);
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize));

                // 4. ParseWithLengthOpts(NULL, …) resets to {NULL, 0}
                let it = (api.cJSON_Parse)(bad.ptr());
                assert!(it.is_null());
                let it2 = (api.cJSON_ParseWithLengthOpts)(
                    std::ptr::null(),
                    10,
                    std::ptr::null_mut(),
                    0,
                );
                assert!(it2.is_null());
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize));

                // 5. buffer_length == 0 sets global_error = {value, 0}
                let it = (api.cJSON_ParseWithLength)(good.ptr(), 0);
                assert!(it.is_null());
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize - good.ptr() as isize));

                // 6. a parse that fails at the very end: position = length - 1
                let trunc = Doc::new(b"[1,2");
                let it = (api.cJSON_ParseWithLength)(trunc.ptr(), 4);
                assert!(it.is_null());
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize - trunc.ptr() as isize));

                // 7. require_null_terminated rejection
                let extra = Doc::new(b"[1] junk");
                let it = (api.cJSON_ParseWithOpts)(extra.ptr(), std::ptr::null_mut(), 1);
                assert!(it.is_null());
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize - extra.ptr() as isize));
            }
            out
        }
        let a = run(&c);
        let b = run(&r);
        assert_eq!(a, b, "cJSON_GetErrorPtr state machine differs");
    }
}

// ---------------------------------------------------------------------------
// row 42 — whitespace handling at the offset == length boundary
// ---------------------------------------------------------------------------
#[test]
fn cfg42_whitespace_boundaries() {
    let (c, r) = both();
    // Every byte <= 32 is whitespace for buffer_skip_whitespace.
    for b in 1u16..=40 {
        let b = b as u8;
        for tmpl in [
            vec![b, b'1'],
            vec![b'1', b],
            vec![b, b'[', b, b'1', b, b']', b],
            vec![b, b'{', b, b'"', b'a', b'"', b, b':', b, b'1', b, b'}', b],
            vec![b],
            vec![b, b],
        ] {
            check_all_modes(&c, &r, &tmpl);
        }
    }
}
