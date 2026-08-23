//! Differential tests for `load.c` — CONFIGS.md rows 71-89, ERRORS.md rows 113-156.
mod common;
use common::*;

/// `dtoa.c` is compiled WITHOUT `MULTIPLE_THREADS`, so `Balloc`'s `freelist`,
/// `p5s` and `dtoa_result` are unsynchronised mutable statics in BOTH libraries.
/// Any test that formats a real number must therefore run exclusively.
fn lock() -> std::sync::MutexGuard<'static, ()> {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match L.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}
use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::ptr;

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

/// The C parser (and therefore the translation) is recursive-descent with a
/// depth limit of `JSON_PARSER_MAX_DEPTH == 2048`. Unoptimised Rust frames are
/// far larger than the C ones, so the deep-nesting corpus entries need more than
/// the default test-thread stack. Run every test body on a big-stack thread.
fn big_stack<F: FnOnce() + Send + 'static>(f: F) {
    let _g = lock();
    std::thread::Builder::new()
        .stack_size(96 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

// ---------------------------------------------------------------------------
// Input corpus
// ---------------------------------------------------------------------------

/// Valid inputs plus every rejection the C `load.c` can produce.
/// Byte vectors (not `&str`) so NUL bytes and invalid UTF-8 can be included.
fn inputs() -> Vec<(&'static str, Vec<u8>)> {
    let mut v: Vec<(&'static str, Vec<u8>)> = Vec::new();
    macro_rules! push {
        ($n:expr, $s:expr) => {
            v.push(($n, ($s).as_bytes().to_vec()))
        };
    }

    // ---- valid: containers ------------------------------------------------
    push!("empty-obj", "{}");
    push!("empty-arr", "[]");
    push!("obj-1", r#"{"a":1}"#);
    push!("obj-many", r#"{"a":1,"b":2,"c":3,"d":4,"e":5,"f":6,"g":7,"h":8,"i":9}"#);
    push!("arr-1", "[1]");
    push!("arr-many", "[1,2,3,4,5,6,7,8,9,10]");
    push!("nested", r#"{"a":[{"b":[[[]]]},{"c":{}}],"d":[1,[2,[3,[4]]]]}"#);
    push!("obj-dup", r#"{"a":1,"a":2}"#);
    push!("obj-dup3", r#"{"k":1,"k":2,"k":3}"#);
    push!("ws", " \t\r\n { \t\r\n \"a\" \t : \t 1 \t } \t\r\n ");
    push!("arr-mixed", r#"[null,true,false,0,-0,1.5,"s",{},[]]"#);

    // ---- valid: scalars (need JSON_DECODE_ANY at top level) ---------------
    push!("bare-null", "null");
    push!("bare-true", "true");
    push!("bare-false", "false");
    push!("bare-int", "42");
    push!("bare-negint", "-42");
    push!("bare-zero", "0");
    push!("bare-negzero", "-0");
    push!("bare-real", "1.5");
    push!("bare-str", r#""hello""#);
    push!("bare-emptystr", r#""""#);

    // ---- numbers ----------------------------------------------------------
    push!("num-0", "[0]");
    push!("num-neg0", "[-0]");
    push!("num-1e10", "[1e10]");
    push!("num-1E10", "[1E10]");
    push!("num-1eplus10", "[1e+10]");
    push!("num-1eminus10", "[1e-10]");
    push!("num-1p0", "[1.0]");
    push!("num-0p5", "[0.5]");
    push!("num-frac-exp", "[1.25e3]");
    push!("num-big", "[9223372036854775807]");
    push!("num-big-neg", "[-9223372036854775808]");
    push!("num-overflow", "[9223372036854775808]");
    push!("num-overflow-neg", "[-9223372036854775809]");
    push!("num-overflow-huge", "[99999999999999999999999999]");
    push!("num-overflow-huge-neg", "[-99999999999999999999999999]");
    push!("num-real-overflow", "[1e999]");
    push!("num-real-overflow-neg", "[-1e999]");
    push!("num-real-underflow", "[1e-999]");
    push!("num-1e308", "[1e308]");
    push!("num-1e309", "[1e309]");
    push!("num-20digit", "[12345678901234567890]");
    push!("num-leadzero", "[01]");
    push!("num-leadzero2", "[00]");
    push!("num-neg-leadzero", "[-01]");
    push!("num-dot-nodigit", "[1.]");
    push!("num-dot-only", "[.]");
    push!("num-dot-first", "[.5]");
    push!("num-e-nodigit", "[1e]");
    push!("num-eplus-nodigit", "[1e+]");
    push!("num-eminus-nodigit", "[1e-]");
    push!("num-minus-only", "[-]");
    push!("num-minus-alpha", "[-a]");
    push!("num-plus", "[+1]");
    push!("num-many-digits", "[1234567890123456789012345678901234567890.5e-30]");

    // ---- strings ----------------------------------------------------------
    push!("str-escapes", r#"["\"\\\/\b\f\n\r\t"]"#);
    push!("str-u-bmp", r#"["\u0041\u00e9\u20ac\uFFFF"]"#);
    push!("str-u-zero", r#"["\u0000"]"#);
    push!("str-u-zero-mid", r#"["a\u0000b"]"#);
    push!("str-surrogate-pair", r#"["\uD834\uDD1E"]"#);
    push!("str-surrogate-pair-low", r#"["\uD800\uDC00"]"#);
    push!("str-surrogate-pair-high", r#"["\uDBFF\uDFFF"]"#);
    push!("str-lone-high", r#"["\uD834"]"#);
    push!("str-lone-high-then-esc", r#"["\uD834\n"]"#);
    push!("str-lone-high-then-u", r#"["\uD834\u0041"]"#);
    push!("str-lone-low", r#"["\uDD1E"]"#);
    push!("str-lone-low2", r#"["\uDC00"]"#);
    push!("str-lone-low3", r#"["\uDFFF"]"#);
    push!("str-bad-escape", r#"["\q"]"#);
    push!("str-bad-escape-u", r#"["\uZZZZ"]"#);
    push!("str-bad-escape-u2", r#"["\u00Z1"]"#);
    push!("str-bad-escape-u3", r#"["\u1"]"#);
    push!("str-trailing-backslash", "[\"\\");
    push!("str-unterminated", "[\"abc");
    push!("str-key-unterminated", "{\"abc");
    push!("str-utf8-2", "[\"\u{e9}\"]");
    push!("str-utf8-3", "[\"\u{20ac}\"]");
    push!("str-utf8-4", "[\"\u{1d11e}\"]");
    push!("str-utf8-key", "{\"\u{1d11e}\u{20ac}\u{e9}\":1}");
    push!("str-long", &format!("[\"{}\"]", "x".repeat(4096)));
    push!("str-slash", r#"["a/b"]"#);

    // control characters inside a string
    v.push(("str-raw-newline", b"[\"a\nb\"]".to_vec()));
    v.push(("str-raw-tab", b"[\"a\tb\"]".to_vec()));
    v.push(("str-raw-cr", b"[\"a\rb\"]".to_vec()));
    v.push(("str-raw-nul", b"[\"a\0b\"]".to_vec()));
    v.push(("str-raw-0x01", b"[\"a\x01b\"]".to_vec()));
    v.push(("str-raw-0x1f", b"[\"a\x1fb\"]".to_vec()));
    v.push(("str-raw-0x7f", b"[\"a\x7fb\"]".to_vec()));

    // ---- invalid UTF-8 in the byte stream --------------------------------
    v.push(("utf8-bad-80", b"[\"\x80\"]".to_vec()));
    v.push(("utf8-bad-bf", b"[\"\xbf\"]".to_vec()));
    v.push(("utf8-bad-c0", b"[\"\xc0\x80\"]".to_vec()));
    v.push(("utf8-bad-c1", b"[\"\xc1\xbf\"]".to_vec()));
    v.push(("utf8-bad-f5", b"[\"\xf5\x80\x80\x80\"]".to_vec()));
    v.push(("utf8-bad-ff", b"[\"\xff\"]".to_vec()));
    v.push(("utf8-truncated-2", b"[\"\xc2\"]".to_vec()));
    v.push(("utf8-truncated-3", b"[\"\xe2\x82\"]".to_vec()));
    v.push(("utf8-truncated-4", b"[\"\xf0\x9d\x84\"]".to_vec()));
    v.push(("utf8-surrogate", b"[\"\xed\xa0\x80\"]".to_vec()));
    v.push(("utf8-overlong-3", b"[\"\xe0\x80\x80\"]".to_vec()));
    v.push(("utf8-bad-continuation", b"[\"\xc2\x41\"]".to_vec()));
    v.push(("utf8-bad-outside-string", b"[\x80]".to_vec()));
    v.push(("utf8-bad-first-byte", b"\x80".to_vec()));

    // ---- syntax errors ----------------------------------------------------
    push!("empty", "");
    push!("ws-only", "   \t\n ");
    push!("obj-unterminated", "{");
    push!("arr-unterminated", "[");
    push!("obj-no-colon", r#"{"a" 1}"#);
    push!("obj-comma-for-colon", r#"{"a",1}"#);
    push!("obj-key-not-string", "{1:2}");
    push!("obj-key-true", "{true:2}");
    push!("obj-missing-close", r#"{"a":1"#);
    push!("obj-trailing-comma", r#"{"a":1,}"#);
    push!("obj-bad-after-value", r#"{"a":1 "b":2}"#);
    push!("arr-missing-close", "[1,2");
    push!("arr-trailing-comma", "[1,2,]");
    push!("arr-bad-after-value", "[1 2]");
    push!("close-mismatch", "[1}");
    push!("close-mismatch2", r#"{"a":1]"#);
    push!("bare-close-brace", "}");
    push!("bare-close-bracket", "]");
    push!("bare-comma", ",");
    push!("bare-colon", ":");
    push!("ident-tru", "[tru]");
    push!("ident-nul", "[nul]");
    push!("ident-True", "[True]");
    push!("ident-NULL", "[NULL]");
    push!("ident-nan", "[nan]");
    push!("ident-inf", "[Infinity]");
    push!("ident-undefined", "[undefined]");
    push!("garbage-at", "[@]");
    push!("garbage-hash", "[#]");
    push!("garbage-tilde", "[~]");
    push!("trailing-garbage", "{} x");
    push!("trailing-garbage2", "[] []");
    push!("trailing-brace", "{}}");
    push!("trailing-nul-then-garbage", "[1]\0[2]");
    push!("comment", "[1] // c");
    push!("obj-nul-in-key", "{\"a\\u0000b\":1}");
    push!("obj-nul-key-only", "{\"\\u0000\":1}");
    push!("multiline", "{\n  \"a\" : [\n    1,\n    @\n  ]\n}");
    push!("long-error-context", r#"[aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa]"#);
    push!("short-error-context", "[aaa]");

    // ---- depth ------------------------------------------------------------
    for (name, n) in [
        ("depth-1", 1usize),
        ("depth-100", 100),
        ("depth-2047", 2047),
        ("depth-2048", 2048),
        ("depth-2049", 2049),
        ("depth-3000", 3000),
    ] {
        let s = format!("{}{}", "[".repeat(n), "]".repeat(n));
        v.push((name, s.into_bytes()));
    }
    for (name, n) in [("odepth-2048", 2048usize), ("odepth-2049", 2049)] {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("{\"a\":");
        }
        s.push('1');
        for _ in 0..n {
            s.push('}');
        }
        v.push((name, s.into_bytes()));
    }
    v
}

const DECODE_FLAG_BITS: [usize; 5] = [
    JSON_REJECT_DUPLICATES,
    JSON_DISABLE_EOF_CHECK,
    JSON_DECODE_ANY,
    JSON_DECODE_INT_AS_REAL,
    JSON_ALLOW_NUL,
];

fn all_decode_flag_combos() -> Vec<usize> {
    (0..32u32)
        .map(|m| {
            let mut f = 0usize;
            for (i, b) in DECODE_FLAG_BITS.iter().enumerate() {
                if m & (1 << i) != 0 {
                    f |= b;
                }
            }
            f
        })
        .collect()
}

/// Load the same bytes into both libraries via `json_loads` and compare
/// everything an external caller can see.
#[track_caller]
fn cmp_loads(d: &Duo, tag: &str, bytes: &[u8], flags: usize) {
    let z = cbuf(bytes);
    unsafe {
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (d.c.json_loads)(z.as_ptr() as *const c_char, flags, &mut ce);
        let rj = (d.rs.json_loads)(z.as_ptr() as *const c_char, flags, &mut re);
        eq(&format!("{} [flags={:#x}] null-ness", tag, flags), cj.is_null(), rj.is_null());
        eq_err(&format!("{} [flags={:#x}]", tag, flags), &ce, &re);
        if !cj.is_null() {
            eq(
                &format!("{} [flags={:#x}] tree", tag, flags),
                describe(&d.c, cj),
                describe(&d.rs, rj),
            );
            let (cd, rd) = dumps_both(d, cj, rj, JSON_ENCODE_ANY | JSON_SORT_KEYS);
            eq(&format!("{} [flags={:#x}] dump-null", tag, flags), cd.is_none(), rd.is_none());
            if let (Some(a), Some(b)) = (&cd, &rd) {
                eq_bytes(&format!("{} [flags={:#x}] dump", tag, flags), a, b);
            }
        }
        decref(&d.c, cj);
        decref(&d.rs, rj);
    }
}

// ===========================================================================
// CONFIGS 71-82, ERRORS 113-143, 152-156 — json_loads over every flag combo
// ===========================================================================

#[test]
fn loads_all_inputs_all_decode_flag_combinations() { big_stack(loads_all_inputs_all_decode_flag_combinations_impl) }
fn loads_all_inputs_all_decode_flag_combinations_impl() {
    let d = duo();
    for (tag, bytes) in inputs() {
        for flags in all_decode_flag_combos() {
            cmp_loads(d, tag, &bytes, flags);
        }
    }
}

/// Unknown/reserved flag bits must be ignored exactly as the C ignores them.
#[test]
fn loads_unknown_flag_bits_ignored() { big_stack(loads_unknown_flag_bits_ignored_impl) }
fn loads_unknown_flag_bits_ignored_impl() {
    let d = duo();
    let noise = [
        1usize << 5,
        1 << 8,
        1 << 16,
        1 << 31,
        1 << 40,
        1 << 63,
        usize::MAX & !0x1F,
        usize::MAX,
    ];
    for (tag, bytes) in inputs().into_iter().take(60) {
        for extra in noise {
            cmp_loads(d, tag, &bytes, extra);
            cmp_loads(d, tag, &bytes, extra | JSON_DECODE_ANY);
        }
    }
}

#[test]
fn loads_randomized_json_texts() { big_stack(loads_randomized_json_texts_impl) }
fn loads_randomized_json_texts_impl() {
    let d = duo();
    let mut rng = Rng::new(0x10AD_5EED);
    for i in 0..4000 {
        // Random *bytes* — most are invalid; this is where the lexer's error
        // reporting (line/column/position/context) gets hammered.
        let n = rng.below(40);
        let mut b = Vec::with_capacity(n);
        for _ in 0..n {
            b.push(match rng.below(10) {
                0..=4 => {
                    let alphabet = b"{}[]:,\"\\ \t\n\r0123456789.eE+-abcdeflnrstuABCDEF";
                    alphabet[rng.below(alphabet.len())]
                }
                5..=7 => b"truefalsenull"[rng.below(13)],
                _ => (rng.next_u32() & 0xFF) as u8,
            });
        }
        let flags = all_decode_flag_combos()[rng.below(32)];
        cmp_loads(d, "rand-bytes", &b, flags);
        if i % 4 == 0 {
            cmp_loads(d, "rand-bytes-any", &b, flags | JSON_DECODE_ANY);
        }
    }
    // structurally valid randomized JSON
    for _ in 0..3000 {
        let txt = rand_json(&mut rng, 4);
        let flags = all_decode_flag_combos()[rng.below(32)];
        cmp_loads(d, "rand-json", txt.as_bytes(), flags);
    }
}

fn rand_json(rng: &mut Rng, depth: usize) -> String {
    if depth == 0 || rng.below(100) < 45 {
        match rng.below(10) {
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
                let n = rng.below(4);
                let s = String::from_utf8(rng.utf8_string(n)).unwrap();
                format!("{:?}", s)
            }
            _ => format!(r#""\u{:04X}""#, 0x20 + rng.below(0xD000)),
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

// ===========================================================================
// CONFIGS 83-84, ERRORS 145 — json_loadb
// ===========================================================================

#[test]
fn loadb_lengths_and_non_terminated() { big_stack(loadb_lengths_and_non_terminated_impl) }
fn loadb_lengths_and_non_terminated_impl() {
    let d = duo();
    unsafe {
        for (tag, bytes) in inputs() {
            let z = cbuf(&bytes);
            for buflen in [
                bytes.len(),
                bytes.len().saturating_sub(1),
                bytes.len() + 1, // includes the NUL from cbuf
                0,
                1,
            ] {
                if buflen > z.len() {
                    continue;
                }
                for flags in [0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK, JSON_ALLOW_NUL] {
                    let mut ce = json_error_t::new();
                    let mut re = json_error_t::new();
                    let cj =
                        (d.c.json_loadb)(z.as_ptr() as *const c_char, buflen, flags, &mut ce);
                    let rj =
                        (d.rs.json_loadb)(z.as_ptr() as *const c_char, buflen, flags, &mut re);
                    let what = format!("loadb {} len={} flags={:#x}", tag, buflen, flags);
                    eq(&format!("{} null", what), cj.is_null(), rj.is_null());
                    eq_err(&what, &ce, &re);
                    if !cj.is_null() {
                        eq(&what, describe(&d.c, cj), describe(&d.rs, rj));
                    }
                    decref(&d.c, cj);
                    decref(&d.rs, rj);
                }
            }
        }
        // a buffer that is NOT NUL-terminated: exactly `buflen` readable bytes
        for txt in [&b"{\"a\":1}"[..], &b"[1,2,3]"[..], &b"[1"[..], &b""[..]] {
            let exact: Vec<u8> = txt.to_vec();
            let p = if exact.is_empty() {
                b"".as_ptr()
            } else {
                exact.as_ptr()
            };
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let cj = (d.c.json_loadb)(p as *const c_char, exact.len(), 0, &mut ce);
            let rj = (d.rs.json_loadb)(p as *const c_char, exact.len(), 0, &mut re);
            eq("loadb exact null", cj.is_null(), rj.is_null());
            eq_err("loadb exact", &ce, &re);
            if !cj.is_null() {
                eq("loadb exact tree", describe(&d.c, cj), describe(&d.rs, rj));
            }
            decref(&d.c, cj);
            decref(&d.rs, rj);
        }
    }
}

// ===========================================================================
// CONFIGS 85-87, ERRORS 146-149 — json_loadf / json_loadfd / json_load_file
// ===========================================================================

fn temp_path(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("jansson_difftest_{}_{}", std::process::id(), name));
    p
}

#[test]
fn loadf_loadfd_loadfile_match_loads() { big_stack(loadf_loadfd_loadfile_match_loads_impl) }
fn loadf_loadfd_loadfile_match_loads_impl() {
    let d = duo();
    let path = temp_path("loadf");
    let cpath = cs(path.to_str().unwrap());
    let mode = cs("rb");
    unsafe {
        for (tag, bytes) in inputs() {
            std::fs::write(&path, &bytes).unwrap();
            for flags in [
                0usize,
                JSON_DECODE_ANY,
                JSON_DISABLE_EOF_CHECK,
                JSON_REJECT_DUPLICATES,
                JSON_ALLOW_NUL,
                JSON_DECODE_INT_AS_REAL,
                JSON_DECODE_ANY | JSON_ALLOW_NUL,
            ] {
                // --- json_loadf ---------------------------------------------
                let cf = fopen(cpath.as_ptr(), mode.as_ptr());
                let rf = fopen(cpath.as_ptr(), mode.as_ptr());
                assert!(!cf.is_null() && !rf.is_null());
                let mut ce = json_error_t::new();
                let mut re = json_error_t::new();
                let cj = (d.c.json_loadf)(cf, flags, &mut ce);
                let rj = (d.rs.json_loadf)(rf, flags, &mut re);
                let what = format!("loadf {} flags={:#x}", tag, flags);
                eq(&format!("{} null", what), cj.is_null(), rj.is_null());
                eq_err(&what, &ce, &re);
                if !cj.is_null() {
                    eq(&what, describe(&d.c, cj), describe(&d.rs, rj));
                }
                decref(&d.c, cj);
                decref(&d.rs, rj);
                fclose(cf);
                fclose(rf);

                // --- json_loadfd --------------------------------------------
                let cfd = std::fs::File::open(&path).unwrap();
                let rfd = std::fs::File::open(&path).unwrap();
                let mut ce = json_error_t::new();
                let mut re = json_error_t::new();
                let cj = (d.c.json_loadfd)(cfd.as_raw_fd(), flags, &mut ce);
                let rj = (d.rs.json_loadfd)(rfd.as_raw_fd(), flags, &mut re);
                let what = format!("loadfd {} flags={:#x}", tag, flags);
                eq(&format!("{} null", what), cj.is_null(), rj.is_null());
                eq_err(&what, &ce, &re);
                if !cj.is_null() {
                    eq(&what, describe(&d.c, cj), describe(&d.rs, rj));
                }
                decref(&d.c, cj);
                decref(&d.rs, rj);
                drop(cfd);
                drop(rfd);

                // --- json_load_file -----------------------------------------
                let mut ce = json_error_t::new();
                let mut re = json_error_t::new();
                let cj = (d.c.json_load_file)(cpath.as_ptr(), flags, &mut ce);
                let rj = (d.rs.json_load_file)(cpath.as_ptr(), flags, &mut re);
                let what = format!("load_file {} flags={:#x}", tag, flags);
                eq(&format!("{} null", what), cj.is_null(), rj.is_null());
                eq_err(&what, &ce, &re);
                if !cj.is_null() {
                    eq(&what, describe(&d.c, cj), describe(&d.rs, rj));
                }
                decref(&d.c, cj);
                decref(&d.rs, rj);
            }
        }
        let _ = std::fs::remove_file(&path);
    }
}

/// ERRORS 149: `fopen` failure, including the `strerror(errno)` text and the
/// `error->source` truncation for very long paths (ERRORS 247).
#[test]
fn load_file_open_failures() { big_stack(load_file_open_failures_impl) }
fn load_file_open_failures_impl() {
    let d = duo();
    unsafe {
        let long = format!("/nonexistent/{}", "d".repeat(300));
        for p in [
            "/definitely/does/not/exist.json",
            "/",
            "/dev/null/nope",
            "",
            "/proc/self/mem/nope",
            long.as_str(),
            &"a".repeat(79),
            &"b".repeat(80),
            &"c".repeat(81),
        ] {
            let cp = cs(p);
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let cj = (d.c.json_load_file)(cp.as_ptr(), 0, &mut ce);
            let rj = (d.rs.json_load_file)(cp.as_ptr(), 0, &mut re);
            eq(&format!("load_file {:?} null", p), cj.is_null(), rj.is_null());
            eq_err(&format!("load_file {:?}", p), &ce, &re);
            decref(&d.c, cj);
            decref(&d.rs, rj);
        }
        // a directory: fopen("dir","rb") succeeds on Linux but reads fail
        let dir = cs("/tmp");
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (d.c.json_load_file)(dir.as_ptr(), 0, &mut ce);
        let rj = (d.rs.json_load_file)(dir.as_ptr(), 0, &mut re);
        eq("load_file dir null", cj.is_null(), rj.is_null());
        eq_err("load_file dir", &ce, &re);
        decref(&d.c, cj);
        decref(&d.rs, rj);
    }
}

// ===========================================================================
// CONFIGS 88, ERRORS 150-151 — json_load_callback
// ===========================================================================

#[repr(C)]
struct CbState {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
    /// return `(size_t)-1` once this many chunks have been served
    fail_after: usize,
    served: usize,
}

unsafe extern "C" fn load_cb(buffer: *mut c_void, buflen: usize, data: *mut c_void) -> usize {
    let st = &mut *(data as *mut CbState);
    if st.fail_after != usize::MAX && st.served >= st.fail_after {
        return usize::MAX;
    }
    st.served += 1;
    let remaining = st.data.len() - st.pos;
    let n = remaining.min(st.chunk).min(buflen);
    if n > 0 {
        std::ptr::copy_nonoverlapping(st.data.as_ptr().add(st.pos), buffer as *mut u8, n);
        st.pos += n;
    }
    n
}

#[test]
fn load_callback_chunk_sizes() { big_stack(load_callback_chunk_sizes_impl) }
fn load_callback_chunk_sizes_impl() {
    let d = duo();
    unsafe {
        for (tag, bytes) in inputs() {
            for chunk in [1usize, 2, 7, 64, 1023, 1024, 4096] {
                for flags in [0usize, JSON_DECODE_ANY, JSON_ALLOW_NUL] {
                    let mut cst = CbState {
                        data: bytes.clone(),
                        pos: 0,
                        chunk,
                        fail_after: usize::MAX,
                        served: 0,
                    };
                    let mut rst = CbState {
                        data: bytes.clone(),
                        pos: 0,
                        chunk,
                        fail_after: usize::MAX,
                        served: 0,
                    };
                    let mut ce = json_error_t::new();
                    let mut re = json_error_t::new();
                    let cj = (d.c.json_load_callback)(
                        Some(load_cb),
                        &mut cst as *mut _ as *mut c_void,
                        flags,
                        &mut ce,
                    );
                    let rj = (d.rs.json_load_callback)(
                        Some(load_cb),
                        &mut rst as *mut _ as *mut c_void,
                        flags,
                        &mut re,
                    );
                    let what = format!("load_callback {} chunk={} flags={:#x}", tag, chunk, flags);
                    eq(&format!("{} null", what), cj.is_null(), rj.is_null());
                    eq_err(&what, &ce, &re);
                    if !cj.is_null() {
                        eq(&what, describe(&d.c, cj), describe(&d.rs, rj));
                    }
                    // the two libraries must have consumed the input identically
                    eq(&format!("{} pos", what), cst.pos, rst.pos);
                    eq(&format!("{} served", what), cst.served, rst.served);
                    decref(&d.c, cj);
                    decref(&d.rs, rj);
                }
            }
        }
    }
}

/// ERRORS 151: the callback signals EOF (`0`) or an error (`(size_t)-1`)
/// part-way through the input.
#[test]
fn load_callback_early_eof_and_error() { big_stack(load_callback_early_eof_and_error_impl) }
fn load_callback_early_eof_and_error_impl() {
    let d = duo();
    unsafe {
        let text = br#"{"aaa":[1,2,3],"bbb":{"ccc":"dddddddd"}}"#.to_vec();
        for fail_after in 0usize..8 {
            for chunk in [1usize, 3, 8] {
                let mut cst = CbState {
                    data: text.clone(),
                    pos: 0,
                    chunk,
                    fail_after,
                    served: 0,
                };
                let mut rst = CbState {
                    data: text.clone(),
                    pos: 0,
                    chunk,
                    fail_after,
                    served: 0,
                };
                let mut ce = json_error_t::new();
                let mut re = json_error_t::new();
                let cj = (d.c.json_load_callback)(
                    Some(load_cb),
                    &mut cst as *mut _ as *mut c_void,
                    0,
                    &mut ce,
                );
                let rj = (d.rs.json_load_callback)(
                    Some(load_cb),
                    &mut rst as *mut _ as *mut c_void,
                    0,
                    &mut re,
                );
                let what = format!("load_callback fail_after={} chunk={}", fail_after, chunk);
                eq(&format!("{} null", what), cj.is_null(), rj.is_null());
                eq_err(&what, &ce, &re);
                eq(&format!("{} served", what), cst.served, rst.served);
                decref(&d.c, cj);
                decref(&d.rs, rj);
            }
        }
    }
}

// ===========================================================================
// ERRORS 144-148, 150 — NULL / invalid arguments, and error == NULL
// ===========================================================================

#[test]
fn loader_null_and_invalid_arguments() { big_stack(loader_null_and_invalid_arguments_impl) }
fn loader_null_and_invalid_arguments_impl() {
    let d = duo();
    unsafe {
        // json_loads(NULL)
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (d.c.json_loads)(ptr::null(), 0, &mut ce);
        let rj = (d.rs.json_loads)(ptr::null(), 0, &mut re);
        eq("loads(NULL) null", cj.is_null(), rj.is_null());
        assert!(cj.is_null());
        eq_err("loads(NULL)", &ce, &re);
        eq("loads(NULL) code", ce.code(), json_error_invalid_argument);

        // json_loadb(NULL)
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        for len in [0usize, 1, 100, usize::MAX] {
            let cj = (d.c.json_loadb)(ptr::null(), len, 0, &mut ce);
            let rj = (d.rs.json_loadb)(ptr::null(), len, 0, &mut re);
            eq("loadb(NULL) null", cj.is_null(), rj.is_null());
            eq_err("loadb(NULL)", &ce, &re);
            ce = json_error_t::new();
            re = json_error_t::new();
        }

        // json_loadf(NULL)
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (d.c.json_loadf)(ptr::null_mut(), 0, &mut ce);
        let rj = (d.rs.json_loadf)(ptr::null_mut(), 0, &mut re);
        eq("loadf(NULL) null", cj.is_null(), rj.is_null());
        eq_err("loadf(NULL)", &ce, &re);

        // json_loadfd with negative fds
        for fd in [-1i32, -2, i32::MIN] {
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let cj = (d.c.json_loadfd)(fd, 0, &mut ce);
            let rj = (d.rs.json_loadfd)(fd, 0, &mut re);
            eq(&format!("loadfd({}) null", fd), cj.is_null(), rj.is_null());
            eq_err(&format!("loadfd({})", fd), &ce, &re);
        }
        // a valid-looking but closed fd: read() fails -> EOF -> parse error
        for fd in [999999i32, 100000] {
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let cj = (d.c.json_loadfd)(fd, 0, &mut ce);
            let rj = (d.rs.json_loadfd)(fd, 0, &mut re);
            eq(&format!("loadfd({}) null", fd), cj.is_null(), rj.is_null());
            eq_err(&format!("loadfd({})", fd), &ce, &re);
        }
        // fd 0 (stdin) selects the "<stdin>" source string
        let devnull = std::fs::File::open("/dev/null").unwrap();
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (d.c.json_loadfd)(0, 0, &mut ce);
        let rj = (d.rs.json_loadfd)(0, 0, &mut re);
        eq("loadfd(0) source", ce.source_str(), re.source_str());
        eq("loadfd(0) source is <stdin>", ce.source_str(), "<stdin>".to_string());
        eq("loadfd(0) null", cj.is_null(), rj.is_null());
        decref(&d.c, cj);
        decref(&d.rs, rj);
        drop(devnull);

        // json_load_file(NULL)
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (d.c.json_load_file)(ptr::null(), 0, &mut ce);
        let rj = (d.rs.json_load_file)(ptr::null(), 0, &mut re);
        eq("load_file(NULL) null", cj.is_null(), rj.is_null());
        eq_err("load_file(NULL)", &ce, &re);

        // json_load_callback(NULL)
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (d.c.json_load_callback)(None, ptr::null_mut(), 0, &mut ce);
        let rj = (d.rs.json_load_callback)(None, ptr::null_mut(), 0, &mut re);
        eq("load_callback(NULL) null", cj.is_null(), rj.is_null());
        eq_err("load_callback(NULL)", &ce, &re);
        eq(
            "load_callback(NULL) source",
            ce.source_str(),
            "<callback>".to_string(),
        );
    }
}

/// CONFIGS 89: `error == NULL` on both the success and the failure path.
#[test]
fn loaders_with_null_error_struct() { big_stack(loaders_with_null_error_struct_impl) }
fn loaders_with_null_error_struct_impl() {
    let d = duo();
    unsafe {
        for (tag, bytes) in inputs() {
            let z = cbuf(&bytes);
            for flags in [0usize, JSON_DECODE_ANY] {
                let cj = (d.c.json_loads)(z.as_ptr() as *const c_char, flags, ptr::null_mut());
                let rj = (d.rs.json_loads)(z.as_ptr() as *const c_char, flags, ptr::null_mut());
                eq(
                    &format!("loads(err=NULL) {} flags={:#x}", tag, flags),
                    cj.is_null(),
                    rj.is_null(),
                );
                if !cj.is_null() {
                    eq(
                        &format!("loads(err=NULL) tree {}", tag),
                        describe(&d.c, cj),
                        describe(&d.rs, rj),
                    );
                }
                decref(&d.c, cj);
                decref(&d.rs, rj);
            }
        }
        // and the NULL-argument paths with error == NULL (ERRORS 244, 248)
        assert!((d.c.json_loads)(ptr::null(), 0, ptr::null_mut()).is_null());
        assert!((d.rs.json_loads)(ptr::null(), 0, ptr::null_mut()).is_null());
        assert!((d.c.json_loadb)(ptr::null(), 0, 0, ptr::null_mut()).is_null());
        assert!((d.rs.json_loadb)(ptr::null(), 0, 0, ptr::null_mut()).is_null());
        assert!((d.c.json_loadf)(ptr::null_mut(), 0, ptr::null_mut()).is_null());
        assert!((d.rs.json_loadf)(ptr::null_mut(), 0, ptr::null_mut()).is_null());
        assert!((d.c.json_loadfd)(-1, 0, ptr::null_mut()).is_null());
        assert!((d.rs.json_loadfd)(-1, 0, ptr::null_mut()).is_null());
        assert!((d.c.json_load_file)(ptr::null(), 0, ptr::null_mut()).is_null());
        assert!((d.rs.json_load_file)(ptr::null(), 0, ptr::null_mut()).is_null());
        assert!((d.c.json_load_callback)(None, ptr::null_mut(), 0, ptr::null_mut()).is_null());
        assert!((d.rs.json_load_callback)(None, ptr::null_mut(), 0, ptr::null_mut()).is_null());
    }
}

/// ERRORS 152-156: the `error_set` context rules — code rewriting at EOF,
/// the 20-byte `saved_text` context limit, and the UTF-8-error special case.
#[test]
fn error_context_rules() { big_stack(error_context_rules_impl) }
fn error_context_rules_impl() {
    let d = duo();
    let cases: Vec<Vec<u8>> = vec![
        // empty saved_text + invalid_syntax -> premature_end_of_input (152)
        b"".to_vec(),
        b"   ".to_vec(),
        b"[".to_vec(),
        b"{".to_vec(),
        b"[1,".to_vec(),
        b"{\"a\":".to_vec(),
        // saved_text of exactly 20 / 21 bytes (153 vs 154)
        format!("[{}]", "a".repeat(19)).into_bytes(),
        format!("[{}]", "a".repeat(20)).into_bytes(),
        format!("[{}]", "a".repeat(21)).into_bytes(),
        format!("[{}]", "a".repeat(100)).into_bytes(),
        format!("[\"{}\"x]", "a".repeat(18)).into_bytes(),
        format!("[\"{}\"x]", "a".repeat(19)).into_bytes(),
        // STREAM_STATE_ERROR with empty saved_text (155)
        b"\x80".to_vec(),
        b"\xff".to_vec(),
        b"  \x80".to_vec(),
        // non-error stream state, empty saved_text (156)
        b"[1".to_vec(),
        // very long message (250)
        format!("{{\"{}\":}}", "k".repeat(200)).into_bytes(),
    ];
    for (i, b) in cases.iter().enumerate() {
        for flags in all_decode_flag_combos() {
            cmp_loads(d, &format!("err-ctx-{}", i), b, flags);
        }
    }
}

/// CONFIGS 81: line / column / position accounting, including multi-byte UTF-8
/// column counting and `\n` handling with `lex_unget`.
#[test]
fn error_position_accounting() { big_stack(error_position_accounting_impl) }
fn error_position_accounting_impl() {
    let d = duo();
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for lead in ["", " ", "\n", "\n\n", "\t", "\r\n", "  \n  "] {
        for body in [
            "[@]",
            "[1,@]",
            "{\"a\":@}",
            "[\n@]",
            "[1\n2]",
            "{\"a\"\n\n1}",
            "[\"\u{e9}\u{20ac}\u{1d11e}\"@]",
            "[\u{e9}]",
            "[1.]",
            "[01]",
            "[\"a\nb\"]",
        ] {
            cases.push(format!("{}{}", lead, body).into_bytes());
        }
    }
    // success path also records error->position
    for ok in ["{}", "[]", "[1,2,3]", "{\"a\":1}", " \n [1] \n ", "[\"\u{1d11e}\"]"] {
        cases.push(ok.as_bytes().to_vec());
    }
    for (i, b) in cases.iter().enumerate() {
        for flags in [
            0usize,
            JSON_DECODE_ANY,
            JSON_DISABLE_EOF_CHECK,
            JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
        ] {
            cmp_loads(d, &format!("pos-{}", i), b, flags);
        }
    }
}

/// A pre-populated `json_error_t` must not be overwritten (ERRORS 249) and the
/// `error->position` write on the success path must match (CONFIGS 74).
#[test]
fn error_struct_reuse_and_success_position() { big_stack(error_struct_reuse_and_success_position_impl) }
fn error_struct_reuse_and_success_position_impl() {
    let d = duo();
    unsafe {
        for txt in ["{}", "[1,2]", "[", "[@]"] {
            let z = cs(txt);
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            // pre-set both structs identically via the public jsonp_error_set
            let msg = cs("preexisting");
            (d.c.jsonp_error_set)(&mut ce, 7, 8, 9, json_error_wrong_type, msg.as_ptr());
            (d.rs.jsonp_error_set)(&mut re, 7, 8, 9, json_error_wrong_type, msg.as_ptr());
            eq_err(&format!("preset {}", txt), &ce, &re);
            let cj = (d.c.json_loads)(z.as_ptr(), JSON_DECODE_ANY, &mut ce);
            let rj = (d.rs.json_loads)(z.as_ptr(), JSON_DECODE_ANY, &mut re);
            eq(&format!("reuse {} null", txt), cj.is_null(), rj.is_null());
            eq_err(&format!("reuse {}", txt), &ce, &re);
            decref(&d.c, cj);
            decref(&d.rs, rj);
        }
    }
}

/// Round trip: `json_loads` -> `json_dumps` -> `json_loads` must be stable and
/// identical between the two libraries (CONFIGS 111).
#[test]
fn load_dump_load_roundtrip() { big_stack(load_dump_load_roundtrip_impl) }
fn load_dump_load_roundtrip_impl() {
    let d = duo();
    let mut rng = Rng::new(0x2007_7101u64);
    unsafe {
        let mut texts: Vec<String> = Vec::new();
        for _ in 0..1500 {
            texts.push(rand_json(&mut rng, 4));
        }
        for (i, t) in texts.iter().enumerate() {
            let z = cs(t);
            for eflags in [
                0usize,
                JSON_COMPACT,
                json_indent(4),
                JSON_ENSURE_ASCII,
                JSON_SORT_KEYS,
                JSON_ESCAPE_SLASH,
                JSON_ENCODE_ANY | JSON_COMPACT | JSON_SORT_KEYS,
            ] {
                let cj = (d.c.json_loads)(z.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
                let rj = (d.rs.json_loads)(z.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
                if cj.is_null() {
                    eq(&format!("rt#{} both null", i), rj.is_null(), true);
                    decref(&d.rs, rj);
                    continue;
                }
                let (cd, rd) = dumps_both(d, cj, rj, eflags | JSON_ENCODE_ANY);
                eq(&format!("rt#{} dump null", i), cd.is_none(), rd.is_none());
                if let (Some(a), Some(b)) = (&cd, &rd) {
                    eq_bytes(&format!("rt#{} dump", i), a, b);
                    // reload
                    let za = cbuf(a);
                    let cj2 =
                        (d.c.json_loads)(za.as_ptr() as *const c_char, JSON_DECODE_ANY, ptr::null_mut());
                    let rj2 = (d.rs.json_loads)(
                        za.as_ptr() as *const c_char,
                        JSON_DECODE_ANY,
                        ptr::null_mut(),
                    );
                    eq(&format!("rt#{} reload null", i), cj2.is_null(), rj2.is_null());
                    if !cj2.is_null() {
                        eq(
                            &format!("rt#{} reload tree", i),
                            describe(&d.c, cj2),
                            describe(&d.rs, rj2),
                        );
                        eq(
                            &format!("rt#{} equal", i),
                            (d.c.json_equal)(cj, cj2),
                            (d.rs.json_equal)(rj, rj2),
                        );
                    }
                    decref(&d.c, cj2);
                    decref(&d.rs, rj2);
                }
                decref(&d.c, cj);
                decref(&d.rs, rj);
            }
        }
    }
}

/// Big inputs, so the growth paths of `strbuffer`, the array table and the
/// hashtable are all crossed inside the parser (CONFIGS 80, 102).
#[test]
fn large_inputs() { big_stack(large_inputs_impl) }
fn large_inputs_impl() {
    let d = duo();
    let mut texts: Vec<String> = Vec::new();
    texts.push(format!("[{}]", (0..2000).map(|i| i.to_string()).collect::<Vec<_>>().join(",")));
    texts.push(format!(
        "{{{}}}",
        (0..2000)
            .map(|i| format!("\"k{}\":{}", i, i))
            .collect::<Vec<_>>()
            .join(",")
    ));
    texts.push(format!("[\"{}\"]", "abc".repeat(20000)));
    texts.push(format!("[\"{}\"]", "\\u00e9".repeat(5000)));
    texts.push(format!("[\"{}\"]", "\\uD834\\uDD1E".repeat(3000)));
    texts.push(format!(
        "{{{}}}",
        (0..500)
            .map(|i| format!("\"{}\":[{}]", "k".repeat(i % 40 + 1), i))
            .collect::<Vec<_>>()
            .join(",")
    ));
    for (i, t) in texts.iter().enumerate() {
        for flags in [0usize, JSON_SORT_KEYS, JSON_REJECT_DUPLICATES, JSON_ALLOW_NUL] {
            cmp_loads(d, &format!("large-{}", i), t.as_bytes(), flags);
        }
    }
    // and via every other entry point once
    let mut f = std::fs::File::create(temp_path("large")).unwrap();
    f.write_all(texts[0].as_bytes()).unwrap();
    drop(f);
    let path = temp_path("large");
    let cpath = cs(path.to_str().unwrap());
    unsafe {
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (d.c.json_load_file)(cpath.as_ptr(), 0, &mut ce);
        let rj = (d.rs.json_load_file)(cpath.as_ptr(), 0, &mut re);
        eq("large load_file null", cj.is_null(), rj.is_null());
        eq_err("large load_file", &ce, &re);
        eq("large load_file tree", describe(&d.c, cj), describe(&d.rs, rj));
        decref(&d.c, cj);
        decref(&d.rs, rj);
    }
    let _ = std::fs::remove_file(&path);
}
