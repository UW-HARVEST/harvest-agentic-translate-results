//! Phase B/C — load.c: every decoder flag, every source, every parse error.
//! CONFIGS rows 73-90 · ERRORS rows 151-193.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn tmpfile() -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fwrite(p: *const c_void, sz: usize, n: usize, f: *mut c_void) -> usize;
    fn fflush(f: *mut c_void) -> c_int;
    fn rewind(f: *mut c_void);
    fn fileno(f: *mut c_void) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
}

/// Everything an external caller can observe from a decode attempt.
type LoadObs = (bool, String, (c_int, c_int, c_int, String, String, i32, Vec<u8>));

unsafe fn observe(api: &'static Api, j: *mut JsonT, err: &JsonError) -> LoadObs {
    unsafe {
        let ok = !j.is_null();
        let sh = if ok { shape(api, j) } else { String::new() };
        (ok, sh, err.snapshot())
    }
}

unsafe fn loads_obs(api: &'static Api, text: &[u8], flags: usize) -> LoadObs {
    unsafe {
        let buf = cbytes(text);
        let mut err = JsonError::default();
        let j = (api.json_loads)(buf.as_ptr() as *const c_char, flags, &mut err);
        let o = observe(api, j, &err);
        decref(api, j);
        o
    }
}

unsafe fn loadb_obs(api: &'static Api, text: &[u8], buflen: usize, flags: usize) -> LoadObs {
    unsafe {
        let buf = cbytes(text);
        let mut err = JsonError::default();
        let j = (api.json_loadb)(buf.as_ptr() as *const c_char, buflen, flags, &mut err);
        let o = observe(api, j, &err);
        decref(api, j);
        o
    }
}

#[track_caller]
unsafe fn diff_loads(text: &[u8], flags: usize) {
    unsafe {
        let co = loads_obs(c(), text, flags);
        let ro = loads_obs(r(), text, flags);
        assert_eq!(
            co,
            ro,
            "json_loads({:?}, {flags:#x})\n  C   = {co:?}\n  RUST= {ro:?}",
            String::from_utf8_lossy(text)
        );
    }
}

/// Valid documents (objects/arrays at the root) plus scalar roots for ANY.
fn valid_corpus() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "{}".into(),
        "[]".into(),
        "  {}  ".into(),
        "\t\r\n[]\n".into(),
        "[1,2,3]".into(),
        "[ 1 , 2 , 3 ]".into(),
        r#"{"a":1,"b":[2,3],"c":{"d":null}}"#.into(),
        r#"{"":0}"#.into(),
        "[[[[[[[[[[1]]]]]]]]]]".into(),
        r#"[0,-0,1,-1,10,100,1000]"#.into(),
        r#"[0.0,-0.0,1.5,-1.5,1e5,1E5,1e+5,1e-5,1.5e10,1.5E-10,-1.5e-10]"#.into(),
        r#"[9223372036854775807,-9223372036854775808]"#.into(),
        r#"["","a","abc","\"","\\","\/","\b","\f","\n","\r","\t"]"#.into(),
        r#"["\u0041","\u00e9","\u20ac","\ud83d\ude00","\uffff","\u0000"]"#.into(),
        r#"["héllo","€uro","😀"]"#.into(),
        r#"[true,false,null]"#.into(),
        r#"{"dup":1,"dup":2}"#.into(),
        r#"{"k1":1,"k2":2,"k3":3,"k4":4,"k5":5,"k6":6,"k7":7,"k8":8,"k9":9,"k10":10}"#.into(),
    ];
    // scalar roots (need JSON_DECODE_ANY)
    v.extend([
        "1".into(),
        "-1".into(),
        "1.5".into(),
        "\"s\"".into(),
        "true".into(),
        "false".into(),
        "null".into(),
    ]);
    v
}

/* ---- CONFIGS 73/74/75/76/77/78/79/80/81/82/83: flags × corpus ---- */

#[test]
fn loads_all_decoder_flag_combinations() {
    let _g = dtoa_guard();
    unsafe {
        // 2^5 == 32 combinations of the five decoder flags.
        for flags in 0usize..32 {
            for text in valid_corpus() {
                diff_loads(text.as_bytes(), flags);
            }
            // trailing data (exercises JSON_DISABLE_EOF_CHECK)
            for text in [
                "{} trailing",
                "[]{}",
                "[1,2]garbage",
                "1 2",
                "null null",
                "{}\n\n",
                "[] ",
            ] {
                diff_loads(text.as_bytes(), flags);
            }
            // duplicate keys (exercises JSON_REJECT_DUPLICATES)
            for text in [
                r#"{"a":1,"a":2}"#,
                r#"{"a":1,"b":2,"a":3}"#,
                r#"{"a":{"x":1,"x":2}}"#,
            ] {
                diff_loads(text.as_bytes(), flags);
            }
            // \u0000 (exercises JSON_ALLOW_NUL)
            for text in [
                r#"["\u0000"]"#,
                r#"["a\u0000b"]"#,
                r#"{"k":"\u0000"}"#,
                r#"{"\u0000":1}"#,
            ] {
                diff_loads(text.as_bytes(), flags);
            }
        }
    }
}

/* ---- CONFIGS 90: randomized round-trips, decode flags × encode flags ---- */

#[test]
fn loads_dumps_roundtrip_randomized() {
    let _g = dtoa_guard();
    unsafe {
        let mut rng = Rng::new(0x10AD_0001);
        let dec_flags = [
            0usize,
            JSON_DECODE_ANY,
            JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
            JSON_DECODE_ANY | JSON_REJECT_DUPLICATES,
            JSON_DECODE_ANY | JSON_ALLOW_NUL,
            JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
            0x1F,
        ];
        let enc_flags = [
            0usize,
            JSON_COMPACT,
            json_indent(2),
            JSON_SORT_KEYS,
            JSON_ENSURE_ASCII,
            JSON_ESCAPE_SLASH,
            JSON_SORT_KEYS | JSON_COMPACT | JSON_ENSURE_ASCII,
            json_indent(4) | JSON_SORT_KEYS,
            JSON_EMBED,
            json_real_precision(5),
            json_real_precision(17),
        ];
        for trial in 0..3000 {
            let text = gen_json(&mut rng, 4);
            for &df in &dec_flags {
                let buf = cbytes(text.as_bytes());
                let mut cerr = JsonError::default();
                let mut rerr = JsonError::default();
                let cj = (c().json_loads)(buf.as_ptr() as *const c_char, df, &mut cerr);
                let rj = (r().json_loads)(buf.as_ptr() as *const c_char, df, &mut rerr);
                assert_eq!(
                    cj.is_null(),
                    rj.is_null(),
                    "trial {trial}: loads({text:?}, {df:#x}) null-ness"
                );
                assert_eq!(
                    cerr.snapshot(),
                    rerr.snapshot(),
                    "trial {trial}: loads({text:?}, {df:#x}) error struct"
                );
                if cj.is_null() {
                    continue;
                }
                assert_eq!(
                    shape(c(), cj),
                    shape(r(), rj),
                    "trial {trial}: loads({text:?}, {df:#x}) shape"
                );
                for &ef in &enc_flags {
                    let f = ef | JSON_ENCODE_ANY;
                    assert_bytes_eq(
                        &format!("trial {trial}: dec={df:#x} enc={f:#x} src={text:?}"),
                        &dumps(c(), cj, f),
                        &dumps(r(), rj, f),
                    );
                }
                // second-generation round trip: dump then re-parse
                if let Some(d) = dumps(c(), cj, JSON_ENCODE_ANY | JSON_COMPACT) {
                    diff_loads(&d, df);
                }
                decref(c(), cj);
                decref(r(), rj);
            }
        }
    }
}

/* ---- CONFIGS 80: number shapes ---- */

#[test]
fn loads_number_shapes() {
    let _g = dtoa_guard();
    unsafe {
        let mut lits: Vec<String> = vec![
            "0".into(), "-0".into(), "1".into(), "-1".into(),
            "0.0".into(), "-0.0".into(), "0.5".into(), "-0.5".into(),
            "1e0".into(), "1E0".into(), "1e+0".into(), "1e-0".into(),
            "1e5".into(), "1E5".into(), "1e+5".into(), "1e-5".into(),
            "1.5e5".into(), "1.5E-5".into(), "-1.5e-10".into(),
            "9223372036854775807".into(),
            "-9223372036854775808".into(),
            "9223372036854775808".into(),      // ERRORS 184
            "-9223372036854775809".into(),     // ERRORS 185
            "99999999999999999999999999".into(),
            "-99999999999999999999999999".into(),
            "1e309".into(),                    // ERRORS 186
            "-1e309".into(),
            "1e999".into(),
            "1e-999".into(),
            "1e-400".into(),
            "01".into(),                       // ERRORS 180
            "00".into(),
            "-01".into(),
            "1.".into(),                       // ERRORS 182
            ".1".into(),
            "-".into(),                        // ERRORS 181
            "-.".into(),
            "-e5".into(),
            "1e".into(),                       // ERRORS 183
            "1e+".into(),
            "1e-".into(),
            "1E".into(),
            "+1".into(),
            "1.2.3".into(),
            "0x10".into(),
            "1_000".into(),
            "Infinity".into(),
            "NaN".into(),
            "1e1000000000000".into(),
            "0.00000000000000000000001".into(),
            "1.7976931348623157e308".into(),
            "1.7976931348623159e308".into(),
            "4.9406564584124654e-324".into(),
            "2.2250738585072011e-308".into(),
        ];
        let mut rng = Rng::new(0x10AD_0002);
        for _ in 0..8000 {
            let sign = if rng.bool() { "-" } else { "" };
            let ip = rng.next_u64() % 100_000_000_000_000_000;
            let fp = rng.next_u64() % 100_000_000_000_000_000;
            let ex = rng.range_i64(-340, 340);
            lits.push(match rng.below(4) {
                0 => format!("{sign}{ip}"),
                1 => format!("{sign}{ip}.{fp}"),
                2 => format!("{sign}{ip}e{ex}"),
                _ => format!("{sign}{ip}.{fp}e{ex}"),
            });
        }
        for lit in &lits {
            for flags in [
                JSON_DECODE_ANY,
                JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
            ] {
                diff_loads(lit.as_bytes(), flags);
                diff_loads(format!("[{lit}]").as_bytes(), flags & !JSON_DECODE_ANY);
                diff_loads(format!(r#"{{"n":{lit}}}"#).as_bytes(), flags);
            }
        }
    }
}

/* ---- CONFIGS 81 · ERRORS 175-179, 189: string escapes ---- */

#[test]
fn loads_string_escapes_and_errors() {
    let _g = dtoa_guard();
    unsafe {
        let mut cases: Vec<String> = vec![
            r#""a""#.into(),
            r#""\"""#.into(),
            r#""\\""#.into(),
            r#""\/""#.into(),
            r#""\b\f\n\r\t""#.into(),
            r#""\u0041""#.into(),
            r#""\u00e9""#.into(),
            r#""\u0000""#.into(),
            r#""\uFFFF""#.into(),
            r#""\ufffd""#.into(),
            r#""\uD83D\uDE00""#.into(),
            r#""\ud83d\ude00""#.into(),
            r#""\uD800\uDC00""#.into(),
            r#""\uDBFF\uDFFF""#.into(),
            // ERRORS 177: high surrogate without a second \u
            r#""\uD800""#.into(),
            r#""\uD800x""#.into(),
            r#""\uD800\\""#.into(),
            r#""\uDBFF""#.into(),
            // ERRORS 178: high surrogate + non-low surrogate
            r#""\uD800\u0041""#.into(),
            r#""\uD800\uD800""#.into(),
            r#""\uD800\uE000""#.into(),
            // ERRORS 179: lone low surrogate
            r#""\uDC00""#.into(),
            r#""\uDFFF""#.into(),
            // ERRORS 175: bad escape char
            r#""\x""#.into(),
            r#""\a""#.into(),
            r#""\ ""#.into(),
            r#""\""#.into(),
            r#""\U0041""#.into(),
            // ERRORS 176/189: bad \u
            r#""\u""#.into(),
            r#""\u0""#.into(),
            r#""\u00""#.into(),
            r#""\u004""#.into(),
            r#""\uZZZZ""#.into(),
            r#""\u00g1""#.into(),
            r#""\u 041""#.into(),
            // ERRORS 172: unterminated
            r#""abc"#.into(),
            r#"""#.into(),
            // ERRORS 173/174: raw control chars
            "\"a\nb\"".into(),
            "\"a\tb\"".into(),
            "\"a\rb\"".into(),
            "\"\u{1}\"".into(),
            "\"\u{1f}\"".into(),
        ];
        // ERRORS 174: every raw control byte 0x00..0x1F
        for b in 0u8..0x20 {
            cases.push(format!("\"a{}b\"", b as char));
        }
        // random \uXXXX sequences
        let mut rng = Rng::new(0x10AD_0003);
        for _ in 0..4000 {
            let a = rng.next_u32() & 0xFFFF;
            let b = rng.next_u32() & 0xFFFF;
            cases.push(format!(r#""\u{a:04x}""#));
            cases.push(format!(r#""\u{a:04X}\u{b:04X}""#));
        }
        for case in &cases {
            for flags in [
                JSON_DECODE_ANY,
                JSON_DECODE_ANY | JSON_ALLOW_NUL,
            ] {
                diff_loads(case.as_bytes(), flags);
                diff_loads(format!("[{case}]").as_bytes(), flags);
                diff_loads(format!("{{{case}:1}}").as_bytes(), flags);
            }
        }
    }
}

/* ---- CONFIGS 82 · ERRORS 158, 159: raw UTF-8 in the input stream ---- */

#[test]
fn loads_utf8_input_and_invalid_bytes() {
    let _g = dtoa_guard();
    unsafe {
        let mut rng = Rng::new(0x10AD_0004);
        let mut cases: Vec<Vec<u8>> = vec![
            "[\"é\"]".into(),
            "[\"€\"]".into(),
            "[\"😀\"]".into(),
            "[\"a€b😀c\"]".into(),
            b"[\"\xC2\"]".to_vec(),          // truncated 2-byte
            b"[\"\xE2\x82\"]".to_vec(),      // truncated 3-byte
            b"[\"\xF0\x9F\x92\"]".to_vec(),  // truncated 4-byte
            b"[\"\x80\"]".to_vec(),          // stray continuation
            b"[\"\xC0\x80\"]".to_vec(),      // overlong
            b"[\"\xED\xA0\x80\"]".to_vec(),  // surrogate
            b"[\"\xF5\x80\x80\x80\"]".to_vec(),
            b"[\"\xFF\"]".to_vec(),
            b"[\"\xFE\xFF\"]".to_vec(),
            b"\xEF\xBB\xBF[]".to_vec(),      // BOM
            b"[\xC2\x80]".to_vec(),          // multi-byte outside a string
            b"\xFF".to_vec(),
        ];
        for _ in 0..3000 {
            let mut v = b"[\"".to_vec();
            let n = 1 + rng.below(8);
            v.extend(rng.bytes(n));
            v.extend(b"\"]");
            cases.push(v);
        }
        for case in &cases {
            for flags in [0usize, JSON_DECODE_ANY, JSON_DECODE_ANY | JSON_ALLOW_NUL] {
                // json_loads stops at the first NUL, so use loadb for byte-exact
                // coverage as well.
                diff_loads(case, flags);
                let co = loadb_obs(c(), case, case.len(), flags);
                let ro = loadb_obs(r(), case, case.len(), flags);
                assert_eq!(
                    co, ro,
                    "json_loadb({case:02x?}, {}, {flags:#x})",
                    case.len()
                );
            }
        }
    }
}

/* ---- CONFIGS 83: line / column / position tracking ---- */

#[test]
fn loads_error_position_tracking() {
    let _g = dtoa_guard();
    unsafe {
        let cases: Vec<String> = vec![
            "{".into(),
            "[".into(),
            "[1".into(),
            "[1,".into(),
            "[1,]".into(),
            "{\"a\"".into(),
            "{\"a\":".into(),
            "{\"a\":}".into(),
            "{\"a\" 1}".into(),
            "{a:1}".into(),
            "{1:2}".into(),
            "[1 2]".into(),
            "[,]".into(),
            "{,}".into(),
            "\n\n\n[".into(),
            "\n  [\n    1,\n    x\n  ]".into(),
            "[\n1,\n2\n".into(),
            "  \t\n  {\n  \"k\" \n  }".into(),
            "[\"é\",\n\"€\",\nx]".into(),
            "[\"😀\", x]".into(),
            "".into(),
            " ".into(),
            "\n".into(),
            "\r\n".into(),
            "]".into(),
            "}".into(),
            ",".into(),
            ":".into(),
            "@".into(),
            "#".into(),
            "tru".into(),
            "truex".into(),
            "nul".into(),
            "NULL".into(),
            "TRUE".into(),
            "falsey".into(),
            "[tru]".into(),
            "{\"a\":tru}".into(),
            "very long identifier that is not a keyword at all".into(),
            "[".repeat(50),
            "]".repeat(50),
        ];
        for case in &cases {
            for flags in 0usize..32 {
                diff_loads(case.as_bytes(), flags);
            }
        }
        // ERRORS 191: saved text longer than 20 bytes suppresses the
        // " near '...'" suffix.
        for n in 1usize..40 {
            let ident = "z".repeat(n);
            diff_loads(ident.as_bytes(), JSON_DECODE_ANY);
            diff_loads(format!("[{ident}]").as_bytes(), 0);
            diff_loads(format!("\"{}", ident).as_bytes(), JSON_DECODE_ANY);
        }
    }
}

/* ---- CONFIGS 84 · ERRORS 162: parse depth ---- */

#[test]
fn loads_nesting_depth_limit() {
    let _g = dtoa_guard();
    unsafe {
        for &d in &[1usize, 2, 10, 100, 1000, 2046, 2047, 2048, 2049, 2100, 4096] {
            for (open, close) in [("[", "]"), ("{\"k\":", "}")] {
                let text = format!("{}{}", open.repeat(d), close.repeat(d));
                let buf = cbytes(text.as_bytes());
                let mut cerr = JsonError::default();
                let mut rerr = JsonError::default();
                let cj = (c().json_loads)(buf.as_ptr() as *const c_char, 0, &mut cerr);
                let rj = (r().json_loads)(buf.as_ptr() as *const c_char, 0, &mut rerr);
                assert_eq!(cj.is_null(), rj.is_null(), "depth {d} {open:?} null-ness");
                assert_eq!(cerr.snapshot(), rerr.snapshot(), "depth {d} {open:?} error");
                if !cj.is_null() {
                    assert_eq!(shape(c(), cj), shape(r(), rj), "depth {d} {open:?} shape");
                }
                decref(c(), cj);
                decref(r(), rj);
            }
        }
    }
}

/* ---- CONFIGS 85 · ERRORS 152: json_loadb ---- */

#[test]
fn json_loadb_buflen_variants() {
    let _g = dtoa_guard();
    unsafe {
        for text in valid_corpus() {
            let b = text.as_bytes();
            for len in 0..=b.len() {
                for flags in [0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY] {
                    let co = loadb_obs(c(), b, len, flags);
                    let ro = loadb_obs(r(), b, len, flags);
                    assert_eq!(co, ro, "json_loadb({text:?}, {len}, {flags:#x})");
                }
            }
        }
        // embedded NUL bytes inside the buffer
        for raw in [
            b"[1,\0 2]".to_vec(),
            b"{\"a\0b\":1}".to_vec(),
            b"\0".to_vec(),
            b"[]\0trailing".to_vec(),
            b"[\"a\0b\"]".to_vec(),
        ] {
            for flags in [0usize, JSON_ALLOW_NUL, JSON_DISABLE_EOF_CHECK, 0x1F] {
                let co = loadb_obs(c(), &raw, raw.len(), flags);
                let ro = loadb_obs(r(), &raw, raw.len(), flags);
                assert_eq!(co, ro, "json_loadb({raw:02x?}, {}, {flags:#x})", raw.len());
            }
        }
        // ERRORS 152: NULL buffer
        for api in both() {
            let mut err = JsonError::default();
            let j = (api.json_loadb)(std::ptr::null(), 10, 0, &mut err);
            assert!(j.is_null());
            assert_eq!(err.code(), E_INVALID_ARGUMENT, "{}", api.tag);
            assert_eq!(err.text_str(), "wrong arguments");
            assert_eq!(err.source_str(), "<buffer>");
        }
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        (c().json_loadb)(std::ptr::null(), 10, 0, &mut ce);
        (r().json_loadb)(std::ptr::null(), 10, 0, &mut re);
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 152");
    }
}

/* ---- CONFIGS 86/87/88 · ERRORS 153-156: file / fd sources ---- */

#[test]
fn json_loadf_loadfd_load_file() {
    let _g = dtoa_guard();
    unsafe {
        let dir = std::env::temp_dir();
        let mut texts = valid_corpus();
        let mut rng = Rng::new(0x10AD_0005);
        for _ in 0..300 {
            texts.push(gen_json(&mut rng, 4));
        }
        texts.extend([
            "{".into(),
            "[1,".into(),
            "".into(),
            "garbage".into(),
            "[] trailing".into(),
        ]);

        for (i, text) in texts.iter().enumerate() {
            for flags in [0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY] {
                // --- json_loadf ---
                let co = via_file(c(), text.as_bytes(), flags);
                let ro = via_file(r(), text.as_bytes(), flags);
                assert_eq!(co, ro, "json_loadf({text:?}, {flags:#x})");
                // --- json_loadfd ---
                let co = via_fd(c(), text.as_bytes(), flags);
                let ro = via_fd(r(), text.as_bytes(), flags);
                assert_eq!(co, ro, "json_loadfd({text:?}, {flags:#x})");
                // --- json_load_file ---
                let path = dir.join(format!("jansson_load_{i}_{flags}.json"));
                std::fs::write(&path, text.as_bytes()).unwrap();
                let ps = cs(path.to_str().unwrap());
                let mut cerr = JsonError::default();
                let mut rerr = JsonError::default();
                let cj = (c().json_load_file)(ps.as_ptr(), flags, &mut cerr);
                let rj = (r().json_load_file)(ps.as_ptr(), flags, &mut rerr);
                let coo = observe(c(), cj, &cerr);
                let roo = observe(r(), rj, &rerr);
                assert_eq!(coo, roo, "json_load_file({text:?}, {flags:#x})");
                decref(c(), cj);
                decref(r(), rj);
                let _ = std::fs::remove_file(&path);
            }
        }

        // ERRORS 153: json_loadf(NULL)
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        let cj = (c().json_loadf)(std::ptr::null_mut(), 0, &mut ce);
        let rj = (r().json_loadf)(std::ptr::null_mut(), 0, &mut re);
        assert!(cj.is_null() && rj.is_null());
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 153");
        assert_eq!(ce.code(), E_INVALID_ARGUMENT);
        assert_eq!(ce.source_str(), "<stream>");

        // ERRORS 154: json_loadfd(negative)
        for fd in [-1i32, -2, i32::MIN] {
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let cj = (c().json_loadfd)(fd, 0, &mut ce);
            let rj = (r().json_loadfd)(fd, 0, &mut re);
            assert!(cj.is_null() && rj.is_null());
            assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 154 (fd={fd})");
            assert_eq!(ce.code(), E_INVALID_ARGUMENT);
        }
        // a valid-but-unreadable fd
        for fd in [999_999i32] {
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let cj = (c().json_loadfd)(fd, JSON_DECODE_ANY, &mut ce);
            let rj = (r().json_loadfd)(fd, JSON_DECODE_ANY, &mut re);
            assert_eq!(cj.is_null(), rj.is_null());
            assert_eq!(ce.snapshot(), re.snapshot(), "bogus fd {fd}");
            decref(c(), cj);
            decref(r(), rj);
        }

        // ERRORS 155: json_load_file(NULL)
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        let cj = (c().json_load_file)(std::ptr::null(), 0, &mut ce);
        let rj = (r().json_load_file)(std::ptr::null(), 0, &mut re);
        assert!(cj.is_null() && rj.is_null());
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 155");
        assert_eq!(ce.code(), E_INVALID_ARGUMENT);

        // ERRORS 156: fopen failure (message embeds strerror(errno))
        for p in [
            "/nonexistent-dir-xyz/nope.json",
            "/proc/self/nonexistent",
            "/",
        ] {
            let ps = cs(p);
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let cj = (c().json_load_file)(ps.as_ptr(), 0, &mut ce);
            let rj = (r().json_load_file)(ps.as_ptr(), 0, &mut re);
            assert_eq!(cj.is_null(), rj.is_null(), "ERRORS 156 ({p})");
            assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 156 ({p})");
            decref(c(), cj);
            decref(r(), rj);
        }
        // a very long path exercises the source-truncation branch (ERRORS 196)
        let long = format!("/tmp/{}/x.json", "d".repeat(200));
        let ps = cs(&long);
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        (c().json_load_file)(ps.as_ptr(), 0, &mut ce);
        (r().json_load_file)(ps.as_ptr(), 0, &mut re);
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 196: long source path");
    }
}

unsafe fn via_file(api: &'static Api, bytes: &[u8], flags: usize) -> LoadObs {
    unsafe {
        let fp = tmpfile();
        assert!(!fp.is_null());
        if !bytes.is_empty() {
            fwrite(bytes.as_ptr() as *const c_void, 1, bytes.len(), fp);
        }
        fflush(fp);
        rewind(fp);
        let mut err = JsonError::default();
        let j = (api.json_loadf)(fp, flags, &mut err);
        let o = observe(api, j, &err);
        decref(api, j);
        fclose(fp);
        o
    }
}

unsafe fn via_fd(api: &'static Api, bytes: &[u8], flags: usize) -> LoadObs {
    unsafe {
        let fp = tmpfile();
        assert!(!fp.is_null());
        if !bytes.is_empty() {
            fwrite(bytes.as_ptr() as *const c_void, 1, bytes.len(), fp);
        }
        fflush(fp);
        let fd = fileno(fp);
        lseek(fd, 0, 0);
        let mut err = JsonError::default();
        let j = (api.json_loadfd)(fd, flags, &mut err);
        let o = observe(api, j, &err);
        decref(api, j);
        fclose(fp);
        o
    }
}

/* ---- CONFIGS 89 · ERRORS 157: json_load_callback ---- */

struct Feed {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
    /// return (size_t)-1 once we reach this call index
    fail_at: usize,
    calls: usize,
}

unsafe extern "C" fn feed_cb(buf: *mut c_void, buflen: usize, d: *mut c_void) -> usize {
    unsafe {
        let f = &mut *(d as *mut Feed);
        f.calls += 1;
        if f.calls > f.fail_at {
            return usize::MAX;
        }
        let n = f.chunk.min(buflen).min(f.data.len() - f.pos);
        if n == 0 {
            return 0;
        }
        std::ptr::copy_nonoverlapping(f.data.as_ptr().add(f.pos), buf as *mut u8, n);
        f.pos += n;
        n
    }
}

#[test]
fn json_load_callback_chunk_sizes() {
    let _g = dtoa_guard();
    unsafe {
        let mut texts = valid_corpus();
        let mut rng = Rng::new(0x10AD_0006);
        for _ in 0..200 {
            texts.push(gen_json(&mut rng, 4));
        }
        // long documents that must span several MAX_BUF_LEN (1024) refills
        texts.push(format!("[{}]", (0..600).map(|i| i.to_string()).collect::<Vec<_>>().join(",")));
        texts.push(format!("[\"{}\"]", "y".repeat(3000)));
        texts.extend(["{".into(), "".into(), "junk".into()]);

        for text in &texts {
            for &chunk in &[1usize, 2, 7, 100, 1023, 1024, 4096] {
                for flags in [0usize, JSON_DECODE_ANY] {
                    let mut cf = Feed {
                        data: text.as_bytes().to_vec(),
                        pos: 0,
                        chunk,
                        fail_at: usize::MAX,
                        calls: 0,
                    };
                    let mut rf = Feed {
                        data: text.as_bytes().to_vec(),
                        pos: 0,
                        chunk,
                        fail_at: usize::MAX,
                        calls: 0,
                    };
                    let mut cerr = JsonError::default();
                    let mut rerr = JsonError::default();
                    let cj = (c().json_load_callback)(
                        Some(feed_cb),
                        &mut cf as *mut Feed as *mut c_void,
                        flags,
                        &mut cerr,
                    );
                    let rj = (r().json_load_callback)(
                        Some(feed_cb),
                        &mut rf as *mut Feed as *mut c_void,
                        flags,
                        &mut rerr,
                    );
                    let co = observe(c(), cj, &cerr);
                    let ro = observe(r(), rj, &rerr);
                    assert_eq!(
                        co, ro,
                        "json_load_callback({text:?}, chunk={chunk}, {flags:#x})"
                    );
                    assert_eq!(
                        cf.calls, rf.calls,
                        "callback call count ({text:?}, chunk={chunk})"
                    );
                    assert_eq!(cf.pos, rf.pos, "callback bytes consumed");
                    decref(c(), cj);
                    decref(r(), rj);
                }
            }
            // callback signalling failure with (size_t)-1
            for fail_at in [0usize, 1, 2] {
                let mut cf = Feed {
                    data: text.as_bytes().to_vec(),
                    pos: 0,
                    chunk: 4,
                    fail_at,
                    calls: 0,
                };
                let mut rf = Feed {
                    data: text.as_bytes().to_vec(),
                    pos: 0,
                    chunk: 4,
                    fail_at,
                    calls: 0,
                };
                let mut cerr = JsonError::default();
                let mut rerr = JsonError::default();
                let cj = (c().json_load_callback)(
                    Some(feed_cb),
                    &mut cf as *mut Feed as *mut c_void,
                    JSON_DECODE_ANY,
                    &mut cerr,
                );
                let rj = (r().json_load_callback)(
                    Some(feed_cb),
                    &mut rf as *mut Feed as *mut c_void,
                    JSON_DECODE_ANY,
                    &mut rerr,
                );
                assert_eq!(
                    observe(c(), cj, &cerr),
                    observe(r(), rj, &rerr),
                    "json_load_callback fail_at={fail_at} ({text:?})"
                );
                decref(c(), cj);
                decref(r(), rj);
            }
        }

        // ERRORS 157: NULL callback
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        let cj = (c().json_load_callback)(None, std::ptr::null_mut(), 0, &mut ce);
        let rj = (r().json_load_callback)(None, std::ptr::null_mut(), 0, &mut re);
        assert!(cj.is_null() && rj.is_null());
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 157");
        assert_eq!(ce.code(), E_INVALID_ARGUMENT);
        assert_eq!(ce.source_str(), "<callback>");
    }
}

/* ---- ERRORS 151: json_loads(NULL) ---- */

#[test]
fn json_loads_null_input() {
    unsafe {
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        let cj = (c().json_loads)(std::ptr::null(), 0, &mut ce);
        let rj = (r().json_loads)(std::ptr::null(), 0, &mut re);
        assert!(cj.is_null() && rj.is_null());
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 151");
        assert_eq!(ce.code(), E_INVALID_ARGUMENT);
        assert_eq!(ce.text_str(), "wrong arguments");
        assert_eq!(ce.source_str(), "<string>");
        // NULL error struct must also be accepted
        assert!((c().json_loads)(std::ptr::null(), 0, std::ptr::null_mut()).is_null());
        assert!((r().json_loads)(std::ptr::null(), 0, std::ptr::null_mut()).is_null());
    }
}

/* ---- ERRORS 160, 161, 163-171: structural parse errors ---- */

#[test]
fn loads_structural_errors() {
    let _g = dtoa_guard();
    unsafe {
        let cases: Vec<&str> = vec![
            // ERRORS 160: no DECODE_ANY and non-container root
            "1", "\"s\"", "true", "false", "null", "1.5", "-1",
            // ERRORS 161: trailing data
            "{} x", "[] []", "1 1", "[]]",
            // ERRORS 166: bad token after '{'
            "{1:2}", "{[]:1}", "{true:1}", "{,}", "{:}", "{]}",
            // ERRORS 169: missing ':'
            r#"{"a" 1}"#, r#"{"a","b"}"#, r#"{"a"}"#,
            // ERRORS 170: missing '}'
            r#"{"a":1"#, r#"{"a":1,"#, r#"{"a":1,}"#, r#"{"a":1]"#,
            // ERRORS 171: missing ']'
            "[1", "[1,", "[1,]", "[1}",
            // ERRORS 165: unexpected token
            "[:]", "[}]", r#"{"a"::1}"#, "[,1]",
            // ERRORS 164: invalid token
            "[@]", "[#]", "[$]", "[foo]", "[Null]",
            // ERRORS 167: NUL byte in an object key
            r#"{"\u0000":1}"#, r#"{"a\u0000b":1}"#,
            // ERRORS 163: \u0000 in a value without ALLOW_NUL
            r#"["\u0000"]"#, r#"{"k":"a\u0000"}"#,
            // ERRORS 168: duplicate key (needs REJECT_DUPLICATES)
            r#"{"a":1,"a":2}"#,
            // empty / whitespace only (ERRORS 193)
            "", " ", "\n", "\t\r\n ",
        ];
        for case in &cases {
            for flags in 0usize..32 {
                diff_loads(case.as_bytes(), flags);
            }
        }
    }
}
