//! Differential tests for `dump.c` — CONFIGS.md rows 90-111, ERRORS.md rows 96-112.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
}

/// `do_dump` recurses; unoptimised Rust frames are much larger than the C's, so
/// the deep-nesting cases need more than the default test-thread stack.
/// `dtoa.c` is compiled WITHOUT `MULTIPLE_THREADS`, so `Balloc`'s `freelist`,
/// `p5s` and `dtoa_result` are unsynchronised mutable statics in BOTH libraries.
/// Any test that formats a real number therefore has to run exclusively, or the
/// two libraries race independently and produce (equally garbage) different
/// output. Take a process-wide lock in every test.
fn lock() -> std::sync::MutexGuard<'static, ()> {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match L.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

fn big_stack<F: FnOnce() + Send + 'static>(f: F) {
    let _g = lock();
    std::thread::Builder::new()
        .stack_size(96 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

fn temp_path(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("jansson_dumptest_{}_{}", std::process::id(), name));
    p
}

// ---------------------------------------------------------------------------
// Value corpus — built identically in both libraries by parsing the same JSON
// text with JSON_DECODE_ANY (so the trees, and their insertion orders, match).
// ---------------------------------------------------------------------------

fn json_texts() -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = Vec::new();
    macro_rules! p {
        ($n:expr, $s:expr) => {
            v.push(($n, ($s).to_string()))
        };
    }

    // scalars (need JSON_ENCODE_ANY when dumped at top level)
    p!("null", "null");
    p!("true", "true");
    p!("false", "false");
    p!("int0", "0");
    p!("int-neg", "-1");
    p!("int-max", "9223372036854775807");
    p!("int-min", "-9223372036854775808");
    p!("real0", "0.0");
    p!("real-neg0", "-0.0");
    p!("real-half", "0.5");
    p!("real-third", "0.3333333333333333");
    p!("real-1e16", "1e16");
    p!("real-1e17", "1e17");
    p!("real-1e-4", "1e-4");
    p!("real-1e-5", "1e-5");
    p!("real-1e300", "1e300");
    p!("real-1e-300", "1e-300");
    p!("real-5e-324", "5e-324");
    p!("real-max", "1.7976931348623157e308");
    p!("real-int-like", "1.0");
    p!("real-big-int-like", "123456789012345680.0");
    p!("str-empty", r#""""#);
    p!("str-ascii", r#""hello world""#);
    p!("str-escapes", r#""\"\\\b\f\n\r\t""#);
    p!("str-slash", r#""a/b/c""#);
    p!("str-ctrl", r#""\u0001\u001f\u007f\u0000""#);
    p!("str-u2", r#""é""#);
    p!("str-u3", r#""€""#);
    p!("str-u4", r#""𝄞""#);
    p!("str-mixed", r#""aéb€c𝄞d/e""#);
    p!("str-bmp-max", r#""￿""#);
    p!("str-astral-max", r#""􏿿""#);

    // containers
    p!("arr-empty", "[]");
    p!("obj-empty", "{}");
    p!("arr-1", "[1]");
    p!("arr-many", "[1,2,3,4,5]");
    p!("obj-1", r#"{"a":1}"#);
    p!("obj-many", r#"{"a":1,"b":2,"c":3}"#);
    p!("obj-sortkeys", r#"{"b":1,"a":2,"ab":3,"aa":4,"A":5,"":6,"a":7}"#);
    p!("obj-prefix-keys", r#"{"a":1,"aa":2,"aaa":3,"ab":4,"b":5,"":6,"é":7,"z":8}"#);
    p!("nested-3", r#"{"a":{"b":{"c":[1,[2,[3]]]}}}"#);
    p!("nested-mixed", r#"[{"a":[]},{"b":{}},[[{}]],[],{}]"#);
    p!("arr-all-types", r#"[null,true,false,0,1.5,"s",[],{}]"#);
    p!("obj-all-types", r#"{"n":null,"t":true,"f":false,"i":0,"r":1.5,"s":"x","a":[],"o":{}}"#);
    p!("arr-nested-empty", "[[],[[]],[[[]]]]");
    p!("obj-nested-empty", r#"{"a":{},"b":{"c":{}}}"#);

    // sizes crossing growth boundaries
    v.push((
        "arr-100",
        format!("[{}]", (0..100).map(|i| i.to_string()).collect::<Vec<_>>().join(",")),
    ));
    v.push((
        "obj-100",
        format!(
            "{{{}}}",
            (0..100).map(|i| format!("\"k{:03}\":{}", i, i)).collect::<Vec<_>>().join(",")
        ),
    ));
    v.push((
        "arr-1000",
        format!("[{}]", (0..1000).map(|i| i.to_string()).collect::<Vec<_>>().join(",")),
    ));
    v.push(("str-4k", format!("\"{}\"", "x".repeat(4096))));
    v.push(("str-utf8-1k", format!("\"{}\"", "\\u00e9\\u20ac".repeat(500))));
    // deep nesting (well under JSON_PARSER_MAX_DEPTH)
    for n in [10usize, 64, 200, 1000] {
        v.push((
            match n {
                10 => "deep-10",
                64 => "deep-64",
                200 => "deep-200",
                _ => "deep-1000",
            },
            format!("{}1{}", "[".repeat(n), "]".repeat(n)),
        ));
    }
    v
}

/// Build the same value in both libraries. Returns `(c, rust)`.
fn parse2(d: &Duo, text: &str) -> (*mut json_t, *mut json_t) {
    let z = cs(text);
    unsafe {
        let f = JSON_DECODE_ANY | JSON_ALLOW_NUL;
        let c = (d.c.json_loads)(z.as_ptr(), f, ptr::null_mut());
        let r = (d.rs.json_loads)(z.as_ptr(), f, ptr::null_mut());
        assert!(!c.is_null(), "C failed to parse corpus entry {:?}", text);
        assert!(!r.is_null(), "RUST failed to parse corpus entry {:?}", text);
        (c, r)
    }
}

fn free2(d: &Duo, c: *mut json_t, r: *mut json_t) {
    decref(&d.c, c);
    decref(&d.rs, r);
}

const ENCODE_FLAGS: &[(&str, usize)] = &[
    ("none", 0),
    ("compact", JSON_COMPACT),
    ("ensure_ascii", JSON_ENSURE_ASCII),
    ("sort_keys", JSON_SORT_KEYS),
    ("preserve_order", JSON_PRESERVE_ORDER),
    ("escape_slash", JSON_ESCAPE_SLASH),
    ("embed", JSON_EMBED),
    ("any", JSON_ENCODE_ANY),
];

/// A broad but bounded set of flag words, always including `JSON_ENCODE_ANY` so
/// scalars are dumpable too.
fn flag_sets() -> Vec<usize> {
    let mut v = vec![0usize];
    for &(_, f) in ENCODE_FLAGS {
        v.push(f);
    }
    for n in 0..=31usize {
        v.push(json_indent(n));
        v.push(json_real_precision(n));
    }
    v.push(JSON_COMPACT | JSON_SORT_KEYS);
    v.push(JSON_COMPACT | json_indent(2));
    v.push(JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH);
    v.push(JSON_SORT_KEYS | JSON_ENSURE_ASCII | json_indent(4));
    v.push(JSON_EMBED | json_indent(2));
    v.push(JSON_EMBED | JSON_COMPACT);
    v.push(JSON_PRESERVE_ORDER | JSON_SORT_KEYS);
    v.push(json_indent(31) | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH | JSON_SORT_KEYS);
    v.push(json_real_precision(17) | json_indent(1));
    let base: Vec<usize> = v.clone();
    for f in base {
        v.push(f | JSON_ENCODE_ANY);
    }
    v.sort();
    v.dedup();
    v
}

#[track_caller]
fn cmp_dumps(d: &Duo, tag: &str, c: *mut json_t, r: *mut json_t, flags: usize) {
    let (cd, rd) = dumps_both(d, c, r, flags);
    let what = format!("json_dumps {} flags={:#x}", tag, flags);
    eq(&format!("{} null", what), cd.is_none(), rd.is_none());
    if let (Some(a), Some(b)) = (&cd, &rd) {
        eq_bytes(&what, a, b);
    }
}

// ===========================================================================
// CONFIGS 90-102 — json_dumps across the whole flag surface
// ===========================================================================

#[test]
fn dumps_all_values_all_flag_sets() {
    big_stack(dumps_all_values_all_flag_sets_impl)
}
fn dumps_all_values_all_flag_sets_impl() {
    let d = duo();
    let flags = flag_sets();
    for (tag, text) in json_texts() {
        let (c, r) = parse2(d, &text);
        for &f in &flags {
            cmp_dumps(d, tag, c, r, f);
        }
        free2(d, c, r);
    }
}

/// CONFIGS 92: indent masking (`n & 0x1F`) and the 32-space chunking loop in
/// `dump_indent` when `depth * ws_count > sizeof(whitespace) - 1`.
#[test]
fn dumps_indent_masking_and_chunking() {
    big_stack(dumps_indent_masking_and_chunking_impl)
}
fn dumps_indent_masking_and_chunking_impl() {
    let d = duo();
    // deep enough that depth*indent far exceeds 32
    let (c, r) = parse2(d, r#"[[[[[[[[[[1,2]]]]]]]]]]"#);
    for n in [
        0usize, 1, 2, 3, 4, 5, 8, 16, 31, 32, 33, 63, 64, 65, 100, 255, 256, 1000,
        usize::MAX,
    ] {
        // JSON_INDENT(n) masks with 0x1F; pass both the masked and the raw value
        cmp_dumps(d, "indent-masked", c, r, json_indent(n));
        cmp_dumps(d, "indent-raw", c, r, n & JSON_MAX_INDENT);
    }
    free2(d, c, r);

    let (c, r) = parse2(d, r#"{"a":{"b":{"c":{"d":{"e":{"f":{"g":{"h":[1,2,3]}}}}}}}}"#);
    for n in 0..=31usize {
        cmp_dumps(d, "indent-obj", c, r, json_indent(n));
        cmp_dumps(d, "indent-obj-compact", c, r, json_indent(n) | JSON_COMPACT);
        cmp_dumps(d, "indent-obj-sorted", c, r, json_indent(n) | JSON_SORT_KEYS);
    }
    free2(d, c, r);
}

/// CONFIGS 99 / ERRORS 97, 255: `JSON_REAL_PRECISION(n)` for every n, over many
/// randomized reals. Some precisions make `jsonp_dtostr` overflow the 25-byte
/// buffer and `do_dump` returns -1 -> `json_dumps` returns NULL.
#[test]
fn dumps_real_precision_all_values() {
    big_stack(dumps_real_precision_all_values_impl)
}
fn dumps_real_precision_all_values_impl() {
    let d = duo();
    let mut rng = Rng::new(0x0DEC_1234);
    let mut vals: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        1.0 / 3.0,
        2.0 / 3.0,
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        5e-324,
        1e16,
        1e17,
        1e-4,
        1e-5,
        123456789012345678.0,
        -123456789012345678.0,
        1e300,
        -1e300,
        1e-300,
        9.87654321e-321,
    ];
    for _ in 0..3000 {
        vals.push(rng.tame_f64());
        vals.push(rng.finite_f64());
    }
    unsafe {
        for (i, &v) in vals.iter().enumerate() {
            let c = (d.c.json_real)(v);
            let r = (d.rs.json_real)(v);
            eq(&format!("json_real null #{}", i), c.is_null(), r.is_null());
            if c.is_null() {
                continue;
            }
            for n in 0..=31usize {
                let f = json_real_precision(n) | JSON_ENCODE_ANY;
                cmp_dumps(d, &format!("real#{} prec={}", i, n), c, r, f);
            }
            // and inside containers, where the same code path is reached with a
            // non-zero depth
            let ca = (d.c.json_array)();
            let ra = (d.rs.json_array)();
            (d.c.json_array_append_new)(ca, incref(c));
            (d.rs.json_array_append_new)(ra, incref(r));
            for n in [0usize, 1, 6, 15, 17, 20, 24, 25, 31] {
                cmp_dumps(
                    d,
                    &format!("real-in-arr#{} prec={}", i, n),
                    ca,
                    ra,
                    json_real_precision(n),
                );
            }
            free2(d, ca, ra);
            free2(d, c, r);
        }
    }
}

/// CONFIGS 94: `JSON_ENSURE_ASCII` surrogate-pair emission, and the interaction
/// with `JSON_ESCAPE_SLASH` (CONFIGS 98) over many randomized strings.
#[test]
fn dumps_string_escaping_randomized() {
    big_stack(dumps_string_escaping_randomized_impl)
}
fn dumps_string_escaping_randomized_impl() {
    let d = duo();
    let mut rng = Rng::new(0x57_E5CA);
    unsafe {
        // every single codepoint class, plus randomized mixtures
        let mut samples: Vec<Vec<u8>> = Vec::new();
        for cp in (0u32..0x80).chain([
            0x80, 0xFF, 0x100, 0x7FF, 0x800, 0xFFF, 0xD7FF, 0xE000, 0xFFFD, 0xFFFF, 0x10000,
            0x1D11E, 0x10FFFE, 0x10FFFF,
        ]) {
            if let Some(ch) = char::from_u32(cp) {
                samples.push(ch.to_string().into_bytes());
            }
        }
        for _ in 0..2000 {
            let n = rng.below(24);
            samples.push(rng.utf8_string(n));
        }
        for _ in 0..500 {
            // strings built only from characters that need escaping
            let n = 1 + rng.below(12);
            let pool: [&str; 12] = [
                "\\", "\"", "/", "\u{8}", "\u{c}", "\n", "\r", "\t", "\u{1}", "\u{1f}",
                "\u{7f}", "\u{0}",
            ];
            let mut s = Vec::new();
            for _ in 0..n {
                s.extend_from_slice(pool[rng.below(pool.len())].as_bytes());
            }
            samples.push(s);
        }
        for (i, s) in samples.iter().enumerate() {
            let c = (d.c.json_stringn_nocheck)(s.as_ptr() as *const c_char, s.len());
            let r = (d.rs.json_stringn_nocheck)(s.as_ptr() as *const c_char, s.len());
            assert!(!c.is_null() && !r.is_null());
            for f in [
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
                JSON_ENCODE_ANY | JSON_ESCAPE_SLASH,
                JSON_ENCODE_ANY | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
                JSON_ENCODE_ANY | JSON_COMPACT | JSON_ENSURE_ASCII,
            ] {
                cmp_dumps(d, &format!("str#{}", i), c, r, f);
            }
            // as an object KEY too (dump_string is used for keys as well)
            let co = (d.c.json_object)();
            let ro = (d.rs.json_object)();
            (d.c.json_object_setn_new_nocheck)(
                co,
                s.as_ptr() as *const c_char,
                s.len(),
                (d.c.json_integer)(1),
            );
            (d.rs.json_object_setn_new_nocheck)(
                ro,
                s.as_ptr() as *const c_char,
                s.len(),
                (d.rs.json_integer)(1),
            );
            for f in [
                0usize,
                JSON_ENSURE_ASCII,
                JSON_ESCAPE_SLASH,
                JSON_SORT_KEYS,
                JSON_SORT_KEYS | JSON_ENSURE_ASCII,
            ] {
                cmp_dumps(d, &format!("key#{}", i), co, ro, f);
            }
            free2(d, co, ro);
            free2(d, c, r);
        }
    }
}

/// ERRORS 101: `dump_string` -> `utf8_iterate` returns NULL for invalid UTF-8,
/// so `do_dump` returns -1 and `json_dumps` returns NULL.
#[test]
fn dumps_invalid_utf8_strings_rejected() {
    let d = duo();
    let _g = lock();
    unsafe {
        let bad: Vec<Vec<u8>> = vec![
            vec![0x80],
            vec![0xBF],
            vec![0xC0, 0x80],
            vec![0xC1, 0xBF],
            vec![0xC2],
            vec![0xE2, 0x82],
            vec![0xE0, 0x80, 0x80],
            vec![0xED, 0xA0, 0x80],
            vec![0xED, 0xBF, 0xBF],
            vec![0xF0, 0x80, 0x80, 0x80],
            vec![0xF4, 0x90, 0x80, 0x80],
            vec![0xF5, 0x80, 0x80, 0x80],
            vec![0xFF],
            vec![b'o', b'k', 0x80, b'a', b'f', b't'],
            vec![0xF0, 0x9D, 0x84], // truncated astral
        ];
        for (i, s) in bad.iter().enumerate() {
            let c = (d.c.json_stringn_nocheck)(s.as_ptr() as *const c_char, s.len());
            let r = (d.rs.json_stringn_nocheck)(s.as_ptr() as *const c_char, s.len());
            for f in [
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
                JSON_ENCODE_ANY | JSON_ESCAPE_SLASH,
            ] {
                let (cd, rd) = dumps_both(d, c, r, f);
                eq(
                    &format!("bad-utf8 #{} flags={:#x} null", i, f),
                    cd.is_none(),
                    rd.is_none(),
                );
                assert!(cd.is_none(), "C must reject invalid UTF-8 in a string value");
            }
            // and as a key
            let co = (d.c.json_object)();
            let ro = (d.rs.json_object)();
            (d.c.json_object_setn_new_nocheck)(
                co,
                s.as_ptr() as *const c_char,
                s.len(),
                (d.c.json_integer)(1),
            );
            (d.rs.json_object_setn_new_nocheck)(
                ro,
                s.as_ptr() as *const c_char,
                s.len(),
                (d.rs.json_integer)(1),
            );
            for f in [0usize, JSON_SORT_KEYS, JSON_ENSURE_ASCII] {
                let (cd, rd) = dumps_both(d, co, ro, f);
                eq(
                    &format!("bad-utf8 key #{} flags={:#x} null", i, f),
                    cd.is_none(),
                    rd.is_none(),
                );
                if let (Some(a), Some(b)) = (&cd, &rd) {
                    eq_bytes(&format!("bad-utf8 key #{} flags={:#x}", i, f), a, b);
                }
            }
            free2(d, co, ro);
            free2(d, c, r);
        }
    }
}

/// CONFIGS 109: integers at the `MAX_INTEGER_STR_LENGTH` (25) boundary.
#[test]
fn dumps_integers() {
    let d = duo();
    let _g = lock();
    let mut rng = Rng::new(0x1007);
    unsafe {
        let mut ints: Vec<i64> = vec![
            0,
            1,
            -1,
            9,
            -9,
            10,
            -10,
            i64::MAX,
            i64::MIN,
            i64::MAX - 1,
            i64::MIN + 1,
            1_000_000_000_000_000_000,
            -1_000_000_000_000_000_000,
        ];
        for _ in 0..3000 {
            ints.push(rng.next_u64() as i64);
        }
        for &i in &ints {
            let c = (d.c.json_integer)(i);
            let r = (d.rs.json_integer)(i);
            for f in [JSON_ENCODE_ANY, JSON_ENCODE_ANY | JSON_COMPACT] {
                cmp_dumps(d, &format!("int {}", i), c, r, f);
            }
            free2(d, c, r);
        }
    }
}

/// CONFIGS 100: `JSON_EMBED` suppresses the outer brackets/braces at depth 0
/// only — verify on arrays, objects, nested values and scalars.
#[test]
fn dumps_embed_flag() {
    let d = duo();
    let _g = lock();
    for (tag, text) in json_texts() {
        let (c, r) = parse2(d, &text);
        for f in [
            JSON_EMBED,
            JSON_EMBED | JSON_ENCODE_ANY,
            JSON_EMBED | JSON_COMPACT,
            JSON_EMBED | json_indent(2),
            JSON_EMBED | JSON_SORT_KEYS,
            JSON_EMBED | JSON_ENCODE_ANY | JSON_ENSURE_ASCII | json_indent(3),
        ] {
            cmp_dumps(d, tag, c, r, f);
        }
        free2(d, c, r);
    }
}

/// Unknown / reserved flag bits must be ignored identically.
#[test]
fn dumps_unknown_flag_bits_ignored() {
    let d = duo();
    let _g = lock();
    let noise = [
        1usize << 17,
        1 << 20,
        1 << 31,
        1 << 40,
        1 << 63,
        usize::MAX,
        usize::MAX & !JSON_EMBED,
    ];
    for (tag, text) in json_texts().into_iter().take(40) {
        let (c, r) = parse2(d, &text);
        for extra in noise {
            cmp_dumps(d, tag, c, r, extra);
            cmp_dumps(d, tag, c, r, extra & !(JSON_EMBED | 0x1F | 0xF800));
            cmp_dumps(d, tag, c, r, extra | JSON_ENCODE_ANY);
        }
        free2(d, c, r);
    }
}

// ===========================================================================
// CONFIGS 103 / ERRORS 105-106 — json_dumpb
// ===========================================================================

#[test]
fn dumpb_all_sizes() {
    big_stack(dumpb_all_sizes_impl)
}
fn dumpb_all_sizes_impl() {
    let d = duo();
    unsafe {
        for (tag, text) in json_texts() {
            let (c, r) = parse2(d, &text);
            for f in [
                0usize,
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_COMPACT,
                JSON_ENCODE_ANY | json_indent(2),
                JSON_ENCODE_ANY | JSON_SORT_KEYS,
            ] {
                // reference length from json_dumps
                let (cd, rd) = dumps_both(d, c, r, f);
                eq(&format!("dumpb ref null {} {:#x}", tag, f), cd.is_none(), rd.is_none());
                let want = match &cd {
                    Some(v) => v.clone(),
                    None => Vec::new(),
                };
                let n = want.len();
                let sizes: Vec<usize> = if n == 0 {
                    vec![0, 1, 16]
                } else {
                    vec![0, 1, n / 2, n - 1, n, n + 1, n + 64]
                };
                for size in sizes {
                    let mut cbuf_ = vec![0xAAu8; size + 64];
                    let mut rbuf_ = vec![0xAAu8; size + 64];
                    let cn = (d.c.json_dumpb)(c, cbuf_.as_mut_ptr() as *mut c_char, size, f);
                    let rn = (d.rs.json_dumpb)(r, rbuf_.as_mut_ptr() as *mut c_char, size, f);
                    let what = format!("dumpb {} flags={:#x} size={}", tag, f, size);
                    eq(&format!("{} ret", what), cn, rn);
                    eq_bytes(&format!("{} buffer", what), &cbuf_, &rbuf_);
                    if cd.is_some() {
                        eq(&format!("{} ret == dumps len", what), cn, n);
                        if size >= n {
                            eq_bytes(&format!("{} content", what), &want, &cbuf_[..n]);
                        }
                        // bytes past `size` must never be written
                        assert!(
                            cbuf_[size..].iter().all(|&b| b == 0xAA),
                            "C wrote past `size` in {}",
                            what
                        );
                        assert!(
                            rbuf_[size..].iter().all(|&b| b == 0xAA),
                            "RUST wrote past `size` in {}",
                            what
                        );
                    }
                }
                // NULL buffer with size 0 (the C never dereferences it)
                let cn = (d.c.json_dumpb)(c, ptr::null_mut(), 0, f);
                let rn = (d.rs.json_dumpb)(r, ptr::null_mut(), 0, f);
                eq(&format!("dumpb NULL buf {} {:#x}", tag, f), cn, rn);
            }
            free2(d, c, r);
        }
    }
}

// ===========================================================================
// CONFIGS 104-106 / ERRORS 102-103, 107-109 — dumpf / dumpfd / dump_file
// ===========================================================================

#[test]
fn dumpf_dumpfd_dumpfile_match_dumps() {
    big_stack(dumpf_dumpfd_dumpfile_match_dumps_impl)
}
fn dumpf_dumpfd_dumpfile_match_dumps_impl() {
    let d = duo();
    let cpathbuf = temp_path("c");
    let rpathbuf = temp_path("r");
    let cpath = cs(cpathbuf.to_str().unwrap());
    let rpath = cs(rpathbuf.to_str().unwrap());
    let wmode = cs("wb");
    unsafe {
        for (tag, text) in json_texts() {
            let (c, r) = parse2(d, &text);
            for f in [
                0usize,
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_COMPACT,
                JSON_ENCODE_ANY | json_indent(3),
                JSON_ENCODE_ANY | JSON_SORT_KEYS | JSON_ENSURE_ASCII,
                JSON_ENCODE_ANY | JSON_EMBED,
                JSON_ENCODE_ANY | json_real_precision(5),
            ] {
                let (cd, _rd) = dumps_both(d, c, r, f);

                // --- json_dumpf --------------------------------------------
                let cf = fopen(cpath.as_ptr(), wmode.as_ptr());
                let rf = fopen(rpath.as_ptr(), wmode.as_ptr());
                assert!(!cf.is_null() && !rf.is_null());
                let crc = (d.c.json_dumpf)(c, cf, f);
                let rrc = (d.rs.json_dumpf)(r, rf, f);
                fflush(cf);
                fflush(rf);
                fclose(cf);
                fclose(rf);
                let what = format!("dumpf {} flags={:#x}", tag, f);
                eq(&format!("{} ret", what), crc, rrc);
                let cb = std::fs::read(&cpathbuf).unwrap();
                let rb = std::fs::read(&rpathbuf).unwrap();
                eq_bytes(&what, &cb, &rb);
                if let Some(expect) = &cd {
                    eq_bytes(&format!("{} == dumps", what), expect, &cb);
                }

                // --- json_dumpfd -------------------------------------------
                {
                    let cfile = std::fs::File::create(&cpathbuf).unwrap();
                    let rfile = std::fs::File::create(&rpathbuf).unwrap();
                    use std::os::unix::io::AsRawFd;
                    let crc = (d.c.json_dumpfd)(c, cfile.as_raw_fd(), f);
                    let rrc = (d.rs.json_dumpfd)(r, rfile.as_raw_fd(), f);
                    drop(cfile);
                    drop(rfile);
                    let what = format!("dumpfd {} flags={:#x}", tag, f);
                    eq(&format!("{} ret", what), crc, rrc);
                    let cb = std::fs::read(&cpathbuf).unwrap();
                    let rb = std::fs::read(&rpathbuf).unwrap();
                    eq_bytes(&what, &cb, &rb);
                    if let Some(expect) = &cd {
                        eq_bytes(&format!("{} == dumps", what), expect, &cb);
                    }
                }

                // --- json_dump_file ----------------------------------------
                let crc = (d.c.json_dump_file)(c, cpath.as_ptr(), f);
                let rrc = (d.rs.json_dump_file)(r, rpath.as_ptr(), f);
                let what = format!("dump_file {} flags={:#x}", tag, f);
                eq(&format!("{} ret", what), crc, rrc);
                let cb = std::fs::read(&cpathbuf).unwrap();
                let rb = std::fs::read(&rpathbuf).unwrap();
                eq_bytes(&what, &cb, &rb);
            }
            free2(d, c, r);
        }
        let _ = std::fs::remove_file(&cpathbuf);
        let _ = std::fs::remove_file(&rpathbuf);
    }
}

/// ERRORS 102, 107: `dump_to_file` when `fwrite` fails (read-only `FILE*`).
/// ERRORS 103, 108: `dump_to_fd` when `write` fails (bad / read-only fd).
/// ERRORS 109: `json_dump_file` when `fopen` fails.
#[test]
fn dump_output_failures() {
    let d = duo();
    let _g = lock();
    let path = temp_path("ro");
    std::fs::write(&path, b"x").unwrap();
    let cpath = cs(path.to_str().unwrap());
    let rmode = cs("rb");
    unsafe {
        for (tag, text) in [
            ("obj", r#"{"a":1}"#),
            ("arr", "[1,2,3]"),
            ("empty-arr", "[]"),
            ("empty-obj", "{}"),
            ("scalar", "42"),
            ("deep", r#"{"a":[1,{"b":[2]}]}"#),
        ] {
            let (c, r) = parse2(d, text);
            for f in [JSON_ENCODE_ANY, JSON_ENCODE_ANY | json_indent(2)] {
                // read-only FILE* -> fwrite fails
                let cf = fopen(cpath.as_ptr(), rmode.as_ptr());
                let rf = fopen(cpath.as_ptr(), rmode.as_ptr());
                assert!(!cf.is_null() && !rf.is_null());
                eq(
                    &format!("dumpf read-only {} {:#x}", tag, f),
                    (d.c.json_dumpf)(c, cf, f),
                    (d.rs.json_dumpf)(r, rf, f),
                );
                fclose(cf);
                fclose(rf);

                // NULL FILE* -> fwrite on NULL is UB, so only exercise the fd form
                // bad fds -> write fails
                for fd in [-1i32, -2, i32::MIN, 999999] {
                    eq(
                        &format!("dumpfd fd={} {} {:#x}", fd, tag, f),
                        (d.c.json_dumpfd)(c, fd, f),
                        (d.rs.json_dumpfd)(r, fd, f),
                    );
                }
                // a read-only fd -> write fails with EBADF
                {
                    use std::os::unix::io::AsRawFd;
                    let cfile = std::fs::File::open(&path).unwrap();
                    let rfile = std::fs::File::open(&path).unwrap();
                    eq(
                        &format!("dumpfd read-only {} {:#x}", tag, f),
                        (d.c.json_dumpfd)(c, cfile.as_raw_fd(), f),
                        (d.rs.json_dumpfd)(r, rfile.as_raw_fd(), f),
                    );
                }
                // json_dump_file with an unopenable path
                for p in [
                    "/definitely/does/not/exist/out.json",
                    "/",
                    "",
                    "/proc/self/mem/nope",
                ] {
                    let pp = cs(p);
                    eq(
                        &format!("dump_file {:?} {} {:#x}", p, tag, f),
                        (d.c.json_dump_file)(c, pp.as_ptr(), f),
                        (d.rs.json_dump_file)(r, pp.as_ptr(), f),
                    );
                }
            }
            free2(d, c, r);
        }
    }
    let _ = std::fs::remove_file(&path);
}

// ===========================================================================
// CONFIGS 107-108 / ERRORS 112 — json_dump_callback
// ===========================================================================

struct Chunks {
    /// one entry per callback invocation: the exact bytes handed over
    calls: Vec<Vec<u8>>,
    /// return non-zero on this invocation index (usize::MAX = never)
    fail_at: usize,
    n: usize,
}

unsafe extern "C" fn rec_cb(buffer: *const c_char, size: usize, data: *mut c_void) -> c_int {
    let st = &mut *(data as *mut Chunks);
    let bytes = if buffer.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(buffer as *const u8, size).to_vec()
    };
    st.calls.push(bytes);
    let idx = st.n;
    st.n += 1;
    if idx == st.fail_at {
        -1
    } else {
        0
    }
}

#[test]
fn dump_callback_chunk_boundaries() {
    big_stack(dump_callback_chunk_boundaries_impl)
}
fn dump_callback_chunk_boundaries_impl() {
    let d = duo();
    unsafe {
        for (tag, text) in json_texts() {
            let (c, r) = parse2(d, &text);
            for f in [
                0usize,
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_COMPACT,
                JSON_ENCODE_ANY | json_indent(4),
                JSON_ENCODE_ANY | JSON_SORT_KEYS,
                JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
                JSON_ENCODE_ANY | JSON_EMBED,
                JSON_ENCODE_ANY | json_indent(31),
            ] {
                let mut cst = Chunks { calls: Vec::new(), fail_at: usize::MAX, n: 0 };
                let mut rst = Chunks { calls: Vec::new(), fail_at: usize::MAX, n: 0 };
                let crc = (d.c.json_dump_callback)(
                    c,
                    Some(rec_cb),
                    &mut cst as *mut _ as *mut c_void,
                    f,
                );
                let rrc = (d.rs.json_dump_callback)(
                    r,
                    Some(rec_cb),
                    &mut rst as *mut _ as *mut c_void,
                    f,
                );
                let what = format!("dump_callback {} flags={:#x}", tag, f);
                eq(&format!("{} ret", what), crc, rrc);
                // The CHUNK BOUNDARIES must match, not just the concatenation.
                eq(&format!("{} n_calls", what), cst.calls.len(), rst.calls.len());
                eq(&format!("{} chunks", what), &cst.calls, &rst.calls);
            }
            free2(d, c, r);
        }
    }
}

/// ERRORS 112: the user callback fails on the k-th chunk, for every k.
#[test]
fn dump_callback_failure_at_each_chunk() {
    big_stack(dump_callback_failure_at_each_chunk_impl)
}
fn dump_callback_failure_at_each_chunk_impl() {
    let d = duo();
    unsafe {
        for (tag, text) in [
            ("obj", r#"{"a":1,"b":[2,3],"c":{"d":"e"}}"#),
            ("arr", r#"[1,2,[3,[4]],{"x":5}]"#),
            ("empty-arr", "[]"),
            ("empty-obj", "{}"),
            ("scalar-int", "7"),
            ("scalar-str", r#""s""#),
            ("scalar-null", "null"),
            ("nested-empty", r#"{"a":[],"b":{}}"#),
        ] {
            let (c, r) = parse2(d, text);
            for f in [
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_COMPACT,
                JSON_ENCODE_ANY | json_indent(2),
                JSON_ENCODE_ANY | JSON_SORT_KEYS,
            ] {
                // how many chunks are there in total?
                let mut probe = Chunks { calls: Vec::new(), fail_at: usize::MAX, n: 0 };
                (d.c.json_dump_callback)(c, Some(rec_cb), &mut probe as *mut _ as *mut c_void, f);
                let total = probe.calls.len();
                for k in 0..total + 2 {
                    let mut cst = Chunks { calls: Vec::new(), fail_at: k, n: 0 };
                    let mut rst = Chunks { calls: Vec::new(), fail_at: k, n: 0 };
                    let crc = (d.c.json_dump_callback)(
                        c,
                        Some(rec_cb),
                        &mut cst as *mut _ as *mut c_void,
                        f,
                    );
                    let rrc = (d.rs.json_dump_callback)(
                        r,
                        Some(rec_cb),
                        &mut rst as *mut _ as *mut c_void,
                        f,
                    );
                    let what = format!("dump_callback {} flags={:#x} fail_at={}", tag, f, k);
                    eq(&format!("{} ret", what), crc, rrc);
                    eq(&format!("{} chunks", what), &cst.calls, &rst.calls);
                }
            }
            free2(d, c, r);
        }
    }
}

// ===========================================================================
// ERRORS 96, 98-100, 104, 110-111 — rejections
// ===========================================================================

/// ERRORS 110-111: `!JSON_ENCODE_ANY` with a non-container (or NULL) root.
/// ERRORS 96: `json == NULL` with `JSON_ENCODE_ANY` reaches `do_dump` and -1.
#[test]
fn dump_rejects_non_containers_without_encode_any() {
    let d = duo();
    let _g = lock();
    unsafe {
        let scalars = ["null", "true", "false", "0", "1.5", r#""s""#];
        for t in scalars {
            let (c, r) = parse2(d, t);
            for f in [0usize, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS, JSON_EMBED] {
                let (cd, rd) = dumps_both(d, c, r, f);
                eq(&format!("dumps {} flags={:#x} null", t, f), cd.is_none(), rd.is_none());
                assert!(cd.is_none(), "C must reject a bare {} without ENCODE_ANY", t);
                eq(
                    &format!("dumpb {} flags={:#x}", t, f),
                    (d.c.json_dumpb)(c, ptr::null_mut(), 0, f),
                    (d.rs.json_dumpb)(r, ptr::null_mut(), 0, f),
                );
                eq(
                    &format!("dumpfd {} flags={:#x}", t, f),
                    (d.c.json_dumpfd)(c, -1, f),
                    (d.rs.json_dumpfd)(r, -1, f),
                );
                let mut cst = Chunks { calls: Vec::new(), fail_at: usize::MAX, n: 0 };
                let mut rst = Chunks { calls: Vec::new(), fail_at: usize::MAX, n: 0 };
                eq(
                    &format!("dump_callback {} flags={:#x}", t, f),
                    (d.c.json_dump_callback)(
                        c,
                        Some(rec_cb),
                        &mut cst as *mut _ as *mut c_void,
                        f,
                    ),
                    (d.rs.json_dump_callback)(
                        r,
                        Some(rec_cb),
                        &mut rst as *mut _ as *mut c_void,
                        f,
                    ),
                );
                eq(&format!("dump_callback {} chunks", t), &cst.calls, &rst.calls);
            }
            free2(d, c, r);
        }
        // json == NULL, with and without ENCODE_ANY
        for f in [0usize, JSON_ENCODE_ANY, JSON_ENCODE_ANY | JSON_COMPACT] {
            let (cd, rd) = dumps_both(d, ptr::null_mut(), ptr::null_mut(), f);
            eq(&format!("dumps(NULL) flags={:#x}", f), cd.is_none(), rd.is_none());
            assert!(cd.is_none());
            eq(
                &format!("dumpb(NULL) flags={:#x}", f),
                (d.c.json_dumpb)(ptr::null(), ptr::null_mut(), 0, f),
                (d.rs.json_dumpb)(ptr::null(), ptr::null_mut(), 0, f),
            );
            eq(
                &format!("dumpfd(NULL) flags={:#x}", f),
                (d.c.json_dumpfd)(ptr::null(), -1, f),
                (d.rs.json_dumpfd)(ptr::null(), -1, f),
            );
            let mut cst = Chunks { calls: Vec::new(), fail_at: usize::MAX, n: 0 };
            let mut rst = Chunks { calls: Vec::new(), fail_at: usize::MAX, n: 0 };
            eq(
                &format!("dump_callback(NULL) flags={:#x}", f),
                (d.c.json_dump_callback)(
                    ptr::null(),
                    Some(rec_cb),
                    &mut cst as *mut _ as *mut c_void,
                    f,
                ),
                (d.rs.json_dump_callback)(
                    ptr::null(),
                    Some(rec_cb),
                    &mut rst as *mut _ as *mut c_void,
                    f,
                ),
            );
            eq("dump_callback(NULL) chunks", &cst.calls, &rst.calls);
        }
    }
}

/// ERRORS 100, 256: a fabricated `json_t` with an out-of-range `json_type`
/// reaches `do_dump`'s `default:` branch.
#[test]
fn dump_out_of_range_json_type() {
    let d = duo();
    let _g = lock();
    unsafe {
        for ty in [8i32, 9, 42, 127, 255, 1000, -1, i32::MIN, i32::MAX] {
            let mut bogus = json_t {
                type_: ty,
                refcount: usize::MAX,
            };
            let p = &mut bogus as *mut json_t;
            for f in [0usize, JSON_ENCODE_ANY, JSON_ENCODE_ANY | JSON_COMPACT] {
                let (cd, rd) = dumps_both(d, p, p, f);
                eq(
                    &format!("dumps bogus ty={} flags={:#x}", ty, f),
                    cd.is_none(),
                    rd.is_none(),
                );
                assert!(cd.is_none(), "C must reject json_type {}", ty);
                eq(
                    &format!("dumpb bogus ty={} flags={:#x}", ty, f),
                    (d.c.json_dumpb)(p, ptr::null_mut(), 0, f),
                    (d.rs.json_dumpb)(p, ptr::null_mut(), 0, f),
                );
                eq(
                    &format!("dumpfd bogus ty={} flags={:#x}", ty, f),
                    (d.c.json_dumpfd)(p, -1, f),
                    (d.rs.json_dumpfd)(p, -1, f),
                );
            }
        }
        // an out-of-range type nested inside a real container
        for ty in [8i32, 42, -1] {
            let mut bogus = json_t {
                type_: ty,
                refcount: usize::MAX,
            };
            let p = &mut bogus as *mut json_t;
            let ca = (d.c.json_array)();
            let ra = (d.rs.json_array)();
            (d.c.json_array_append_new)(ca, p);
            (d.rs.json_array_append_new)(ra, p);
            for f in [0usize, json_indent(2), JSON_COMPACT] {
                let (cd, rd) = dumps_both(d, ca, ra, f);
                eq(
                    &format!("dumps nested bogus ty={} flags={:#x}", ty, f),
                    cd.is_none(),
                    rd.is_none(),
                );
            }
            (d.c.json_array_clear)(ca);
            (d.rs.json_array_clear)(ra);
            free2(d, ca, ra);
        }
    }
}

/// ERRORS 98-99: circular references detected by `jsonp_loop_check`.
#[test]
fn dump_detects_circular_references() {
    let d = duo();
    let _g = lock();
    unsafe {
        // arr -> arr
        let mut rets = Vec::new();
        for l in d.both() {
            let a = (l.json_array)();
            let b = (l.json_array)();
            assert_eq!((l.json_array_append_new)(a, incref(b)), 0);
            assert_eq!((l.json_array_append_new)(b, incref(a)), 0);
            let mut nulls = Vec::new();
            for f in [0usize, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS, JSON_EMBED] {
                let s = (l.json_dumps)(a, f);
                nulls.push(s.is_null());
                if !s.is_null() {
                    (l.jsonp_free)(s as *mut c_void);
                }
                // and the other entry points
                nulls.push((l.json_dumpb)(a, ptr::null_mut(), 0, f) == 0);
                nulls.push((l.json_dumpfd)(a, -1, f) == -1);
            }
            rets.push(nulls);
            (l.json_array_clear)(a);
            (l.json_array_clear)(b);
            decref(l, a);
            decref(l, b);
        }
        eq("array cycle dump results", &rets[0], &rets[1]);
        assert!(rets[0].iter().all(|&x| x), "C must reject an array cycle");

        // obj -> obj
        let mut rets = Vec::new();
        for l in d.both() {
            let o1 = (l.json_object)();
            let o2 = (l.json_object)();
            assert_eq!((l.json_object_set_new)(o1, cs("x").as_ptr(), incref(o2)), 0);
            assert_eq!((l.json_object_set_new)(o2, cs("y").as_ptr(), incref(o1)), 0);
            let mut nulls = Vec::new();
            for f in [0usize, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS] {
                let s = (l.json_dumps)(o1, f);
                nulls.push(s.is_null());
                if !s.is_null() {
                    (l.jsonp_free)(s as *mut c_void);
                }
            }
            rets.push(nulls);
            (l.json_object_clear)(o1);
            (l.json_object_clear)(o2);
            decref(l, o1);
            decref(l, o2);
        }
        eq("object cycle dump results", &rets[0], &rets[1]);
        assert!(rets[0].iter().all(|&x| x), "C must reject an object cycle");

        // a DAG (shared but acyclic) must SUCCEED, and identically
        let mut dumps = Vec::new();
        for l in d.both() {
            let shared = (l.json_array)();
            (l.json_array_append_new)(shared, (l.json_integer)(1));
            let a = (l.json_array)();
            (l.json_array_append_new)(a, incref(shared));
            (l.json_array_append_new)(a, incref(shared));
            let o = (l.json_object)();
            (l.json_object_set_new)(o, cs("p").as_ptr(), incref(shared));
            (l.json_object_set_new)(o, cs("q").as_ptr(), incref(shared));
            (l.json_array_append_new)(a, o);
            let mut per_flag = Vec::new();
            for f in [0usize, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS] {
                let s = (l.json_dumps)(a, f);
                assert!(!s.is_null(), "{}: DAG must dump", l.which);
                per_flag.push(cstr_bytes(s));
                (l.jsonp_free)(s as *mut c_void);
            }
            dumps.push(per_flag);
            decref(l, a);
            decref(l, shared);
        }
        eq("DAG dumps", &dumps[0], &dumps[1]);
    }
}

// ===========================================================================
// CONFIGS 111 — full round trip over randomized values
// ===========================================================================

#[test]
fn randomized_dump_roundtrip() {
    big_stack(randomized_dump_roundtrip_impl)
}
fn randomized_dump_roundtrip_impl() {
    let d = duo();
    let mut rng = Rng::new(0xD0_9911);
    let flags = flag_sets();
    unsafe {
        for round in 0..2500 {
            let text = rand_json(&mut rng, 4);
            let z = cs(&text);
            let c = (d.c.json_loads)(z.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
            let r = (d.rs.json_loads)(z.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
            eq(&format!("rt#{} parse null", round), c.is_null(), r.is_null());
            if c.is_null() {
                decref(&d.rs, r);
                continue;
            }
            // a handful of flag words per round, deterministically chosen
            for _ in 0..6 {
                let f = flags[rng.below(flags.len())] | JSON_ENCODE_ANY;
                let (cd, rd) = dumps_both(d, c, r, f);
                eq(&format!("rt#{} dump null f={:#x}", round, f), cd.is_none(), rd.is_none());
                if let (Some(a), Some(b)) = (&cd, &rd) {
                    eq_bytes(&format!("rt#{} dump f={:#x}", round, f), a, b);
                    // reparse and re-dump canonically
                    let za = cbuf(a);
                    let c2 = (d.c.json_loads)(
                        za.as_ptr() as *const c_char,
                        JSON_DECODE_ANY,
                        ptr::null_mut(),
                    );
                    let r2 = (d.rs.json_loads)(
                        za.as_ptr() as *const c_char,
                        JSON_DECODE_ANY,
                        ptr::null_mut(),
                    );
                    eq(&format!("rt#{} reparse null", round), c2.is_null(), r2.is_null());
                    if !c2.is_null() {
                        eq(
                            &format!("rt#{} reparse tree", round),
                            describe(&d.c, c2),
                            describe(&d.rs, r2),
                        );
                        // JSON_EMBED output is not re-parseable as a whole value,
                        // and JSON_REAL_PRECISION is lossy, so only require value
                        // equality for the lossless flag words.
                        if f & JSON_EMBED == 0 && json_real_precision(31) & f == 0 {
                            eq(
                                &format!("rt#{} equal f={:#x}", round, f),
                                (d.c.json_equal)(c, c2),
                                (d.rs.json_equal)(r, r2),
                            );
                        }
                    }
                    decref(&d.c, c2);
                    decref(&d.rs, r2);
                }
            }
            decref(&d.c, c);
            decref(&d.rs, r);
        }
    }
}

fn rand_json(rng: &mut Rng, depth: usize) -> String {
    if depth == 0 || rng.below(100) < 45 {
        match rng.below(11) {
            0 => "null".into(),
            1 => "true".into(),
            2 => "false".into(),
            3 => format!("{}", rng.next_u64() as i64),
            4 => format!("{}", rng.range_i64(-1000, 1000)),
            5 => format!("{:e}", rng.tame_f64()),
            6 => format!("{}", rng.tame_f64()),
            7 => {
                let n = rng.below(10);
                let s = String::from_utf8(rng.ascii_string(n)).unwrap();
                format!("{:?}", s)
            }
            8 => {
                let n = rng.below(5);
                let s = String::from_utf8(rng.utf8_string(n)).unwrap();
                format!("{:?}", s)
            }
            9 => format!(r#""\u{:04X}""#, rng.below(0xD000)),
            _ => {
                let hi = 0xD800 + rng.below(0x400);
                let lo = 0xDC00 + rng.below(0x400);
                format!(r#""\u{:04X}\u{:04X}""#, hi, lo)
            }
        }
    } else if rng.bool() {
        let n = rng.below(5);
        let items: Vec<String> = (0..n).map(|_| rand_json(rng, depth - 1)).collect();
        format!("[{}]", items.join(","))
    } else {
        let n = rng.below(5);
        let items: Vec<String> = (0..n)
            .map(|i| {
                let k = if rng.below(8) == 0 {
                    format!("dup{}", i % 2)
                } else {
                    let kn = 1 + rng.below(5);
                    String::from_utf8(rng.ascii_string(kn))
                        .unwrap()
                        .replace('\\', "s")
                        .replace('"', "q")
                };
                format!("{:?}:{}", k, rand_json(rng, depth - 1))
            })
            .collect();
        format!("{{{}}}", items.join(","))
    }
}

/// CONFIGS 95: `JSON_SORT_KEYS` with keys that differ only in length, only in a
/// prefix, or contain bytes above 0x7F (the C's `compare_keys` uses `memcmp` on
/// `min(len1,len2)` then `len1 - len2`, so ordering is by *unsigned* bytes but
/// the length tie-break is a signed `int` subtraction).
#[test]
fn dumps_sort_keys_orderings() {
    let d = duo();
    let _g = lock();
    let mut rng = Rng::new(0x50_5712);
    unsafe {
        let mut keysets: Vec<Vec<Vec<u8>>> = vec![
            vec![b"b".to_vec(), b"a".to_vec()],
            vec![b"a".to_vec(), b"aa".to_vec(), b"aaa".to_vec()],
            vec![b"aaa".to_vec(), b"aa".to_vec(), b"a".to_vec()],
            vec![b"".to_vec(), b"a".to_vec(), b"".to_vec()],
            vec![b"ab".to_vec(), b"aa".to_vec(), b"ac".to_vec(), b"a".to_vec()],
            vec![b"A".to_vec(), b"a".to_vec(), b"Z".to_vec(), b"z".to_vec()],
            vec![vec![0x7F], vec![0x20], vec![0x30]],
            vec!["é".as_bytes().to_vec(), "€".as_bytes().to_vec(), b"z".to_vec()],
            vec![
                b"key".to_vec(),
                b"key1".to_vec(),
                b"key10".to_vec(),
                b"key2".to_vec(),
            ],
            // long keys, so `int len` in `struct key_len` matters
            vec![vec![b'x'; 300], vec![b'x'; 301], vec![b'y'; 299]],
            // embedded NULs: `compare_keys` uses memcmp over min(len1,len2) and
            // only then the `len1 - len2` tie-break, so a NUL inside a key is an
            // ordinary byte here (and `dump_string` emits it as \u0000).
            vec![b"a\0b".to_vec(), b"a".to_vec(), b"a\0".to_vec(), b"a\0a".to_vec()],
            vec![b"\0".to_vec(), b"".to_vec(), b"\0\0".to_vec(), b"\0a".to_vec()],
            vec![b"k\0".to_vec(), b"k\0\0".to_vec(), b"k".to_vec(), b"kk".to_vec()],
            // keys equal on their common prefix but of many different lengths
            vec![
                b"p".to_vec(), b"pp".to_vec(), b"ppp".to_vec(), b"pppp".to_vec(),
                b"ppppp".to_vec(), b"".to_vec(),
            ],
            // high bytes (memcmp is unsigned, the tie-break is a signed int)
            vec![vec![0xFF], vec![0x01], vec![0x7F], vec![0x80], vec![0xFF, 0x00]],
        ];
        for _ in 0..400 {
            let n = 1 + rng.below(12);
            let mut ks = Vec::new();
            for _ in 0..n {
                let kn = rng.below(6);
                ks.push(rng.ascii_string(kn));
            }
            keysets.push(ks);
        }
        for (i, ks) in keysets.iter().enumerate() {
            let co = (d.c.json_object)();
            let ro = (d.rs.json_object)();
            for (j, k) in ks.iter().enumerate() {
                let mut pad = k.clone();
                pad.push(0);
                (d.c.json_object_setn_new_nocheck)(
                    co,
                    pad.as_ptr() as *const c_char,
                    k.len(),
                    (d.c.json_integer)(j as i64),
                );
                (d.rs.json_object_setn_new_nocheck)(
                    ro,
                    pad.as_ptr() as *const c_char,
                    k.len(),
                    (d.rs.json_integer)(j as i64),
                );
            }
            for f in [
                JSON_SORT_KEYS,
                JSON_SORT_KEYS | JSON_COMPACT,
                JSON_SORT_KEYS | json_indent(2),
                JSON_SORT_KEYS | JSON_ENSURE_ASCII,
                0,
            ] {
                cmp_dumps(d, &format!("sortkeys#{}", i), co, ro, f);
            }
            free2(d, co, ro);
        }
    }
}
