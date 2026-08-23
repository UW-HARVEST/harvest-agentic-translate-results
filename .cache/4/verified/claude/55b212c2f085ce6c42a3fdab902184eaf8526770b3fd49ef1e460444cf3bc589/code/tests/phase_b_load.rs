//! Phase B — differential tests for the decoder (CONFIGS.md rows 96..113).

mod common;
use common::tree::*;
use common::*;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::Mutex;

const DECODE_FLAGS: &[(&str, usize)] = &[
    ("none", 0),
    ("dup", JSON_REJECT_DUPLICATES),
    ("noeof", JSON_DISABLE_EOF_CHECK),
    ("any", JSON_DECODE_ANY),
    ("intreal", JSON_DECODE_INT_AS_REAL),
    ("nul", JSON_ALLOW_NUL),
    ("any|nul", JSON_DECODE_ANY | JSON_ALLOW_NUL),
    ("any|intreal", JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL),
    ("dup|noeof", JSON_REJECT_DUPLICATES | JSON_DISABLE_EOF_CHECK),
    ("all", 0x1F),
    ("allbits", usize::MAX),
];

/// Parse `text` with every decoder entry point and flag set, recording the
/// result *and* the whole 248 byte `json_error_t`.
unsafe fn probe_all_sources(api: &Api, rec: &mut Rec, tag: &str, text: &[u8]) {
    let z = cbuf(text);
    let pf = tmp_file("loadf");
    let pfile = tmp_file("loadfile");
    let cpfile = cs(pfile.to_str().unwrap());
    let mode = cs("rb");

    for (fname, flags) in DECODE_FLAGS {
        // ---- json_loads (NUL terminated) ----
        let mut e = JsonError::patterned();
        let j = (api.json_loads)(z.as_ptr() as *const c_char, *flags, &mut e);
        rec.json(&format!("{tag}.{fname}.loads"), j);
        rec_dump_all(api, rec, &format!("{tag}.{fname}.loads"), j);
        rec.error(&format!("{tag}.{fname}.loads_err"), &e);
        decref(api, j);

        // error == NULL must behave the same
        let j = (api.json_loads)(z.as_ptr() as *const c_char, *flags, ptr::null_mut());
        rec.json(&format!("{tag}.{fname}.loads_noerr"), j);
        decref(api, j);

        // ---- json_loadb with exact length, short length and over-length ----
        for l in [text.len(), text.len().saturating_sub(1), 0] {
            let mut e = JsonError::patterned();
            let j = (api.json_loadb)(z.as_ptr() as *const c_char, l, *flags, &mut e);
            rec.json(&format!("{tag}.{fname}.loadb{l}"), j);
            rec_dump_all(api, rec, &format!("{tag}.{fname}.loadb{l}"), j);
            rec.error(&format!("{tag}.{fname}.loadb{l}_err"), &e);
            decref(api, j);
        }

        // ---- json_loadf ----
        std::fs::write(&pf, text).unwrap();
        let cpf = cs(pf.to_str().unwrap());
        let fh = fopen(cpf.as_ptr(), mode.as_ptr());
        assert!(!fh.is_null());
        let mut e = JsonError::patterned();
        let j = (api.json_loadf)(fh, *flags, &mut e);
        fclose(fh);
        rec.json(&format!("{tag}.{fname}.loadf"), j);
        rec_dump_all(api, rec, &format!("{tag}.{fname}.loadf"), j);
        rec.error(&format!("{tag}.{fname}.loadf_err"), &e);
        decref(api, j);

        // ---- json_loadfd ----
        {
            use std::os::unix::io::AsRawFd;
            let file = std::fs::File::open(&pf).unwrap();
            let mut e = JsonError::patterned();
            let j = (api.json_loadfd)(file.as_raw_fd(), *flags, &mut e);
            drop(file);
            rec.json(&format!("{tag}.{fname}.loadfd"), j);
            rec_dump_all(api, rec, &format!("{tag}.{fname}.loadfd"), j);
            rec.error(&format!("{tag}.{fname}.loadfd_err"), &e);
            decref(api, j);
        }

        // ---- json_load_file ----
        std::fs::write(&pfile, text).unwrap();
        let mut e = JsonError::patterned();
        let j = (api.json_load_file)(cpfile.as_ptr(), *flags, &mut e);
        rec.json(&format!("{tag}.{fname}.loadfile"), j);
        rec_dump_all(api, rec, &format!("{tag}.{fname}.loadfile"), j);
        rec.error(&format!("{tag}.{fname}.loadfile_err"), &e);
        decref(api, j);

        // ---- json_load_callback with several chunk sizes ----
        for chunk in [1usize, 7, 1023, 1024, 1025] {
            cb_set(text, chunk, false);
            let mut e = JsonError::patterned();
            let j = (api.json_load_callback)(Some(cb_feed), ptr::null_mut(), *flags, &mut e);
            rec.json(&format!("{tag}.{fname}.cb{chunk}"), j);
            rec_dump_all(api, rec, &format!("{tag}.{fname}.cb{chunk}"), j);
            rec.error(&format!("{tag}.{fname}.cb{chunk}_err"), &e);
            decref(api, j);
        }
    }
    let _ = std::fs::remove_file(&pf);
    let _ = std::fs::remove_file(&pfile);
}

/* --------------------------------------------------- load callback state */

struct Feed {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
    err_at_end: bool,
}
static FEED: Mutex<Option<Feed>> = Mutex::new(None);

fn cb_set(data: &[u8], chunk: usize, err_at_end: bool) {
    *FEED.lock().unwrap() = Some(Feed {
        data: data.to_vec(),
        pos: 0,
        chunk,
        err_at_end,
    });
}

unsafe extern "C" fn cb_feed(buffer: *mut c_void, buflen: usize, _arg: *mut c_void) -> usize {
    let mut g = FEED.lock().unwrap();
    let f = g.as_mut().unwrap();
    let remaining = f.data.len() - f.pos;
    if remaining == 0 {
        return if f.err_at_end { usize::MAX } else { 0 };
    }
    let n = remaining.min(f.chunk).min(buflen);
    ptr::copy_nonoverlapping(f.data[f.pos..].as_ptr(), buffer as *mut u8, n);
    f.pos += n;
    n
}

/* ------------------------------------------------ rows 96..107 ---------- */

const DOCS: &[&str] = &[
    // containers
    "{}",
    "[]",
    "[1]",
    r#"{"a":1}"#,
    r#"{"a":1,"b":2,"c":3}"#,
    r#"[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17]"#,
    r#"{"k0":0,"k1":1,"k2":2,"k3":3,"k4":4,"k5":5,"k6":6,"k7":7,"k8":8,"k9":9}"#,
    r#"{"a":{"b":{"c":{"d":[1,2,{"e":null}]}}}}"#,
    // scalars (need JSON_DECODE_ANY)
    "null",
    "true",
    "false",
    "0",
    "-0",
    "1",
    "-1",
    "123456789",
    "9223372036854775807",
    "-9223372036854775808",
    "9223372036854775808",
    "-9223372036854775809",
    "0.5",
    "-0.5",
    "1e5",
    "1E5",
    "1e+5",
    "1e-5",
    "1.5e300",
    "1e400",
    "-1e400",
    "1e-400",
    "0.0",
    "-0.0",
    "1e16",
    "1e17",
    "0.1",
    "3.141592653589793",
    r#""""#,
    r#""abc""#,
    r#""\"\\\/\b\f\n\r\t""#,
    r#""\u0041\u00e9\u20ac\ud83d\ude00""#,
    r#""\uD83D\uDE00""#,
    r#""\u0000""#,
    r#""\uabcd\uABCD""#,
    // whitespace variants
    " \t\r\n[ \t\r\n1 \t\r\n, \t\r\n2 \t\r\n] \t\r\n",
    "{\n  \"a\" : \t1 ,\r\n \"b\":[\n]\n}\n",
    // duplicates
    r#"{"a":1,"a":2}"#,
    r#"{"a":1,"b":2,"a":3}"#,
    // trailing garbage
    "[1] x",
    "{} {}",
    "[1]]",
    "1 2",
    // multi-byte UTF-8 literals
    "[\"\u{80}\u{7ff}\u{800}\u{ffff}\u{10000}\u{10ffff}\"]",
    "{\"\u{263A}\":\"\u{1F600}\"}",
    // NUL byte in a string value (needs JSON_ALLOW_NUL)
    r#"["a\u0000b"]"#,
    r#"{"a\u0000b":1}"#,
    // errors
    "",
    " ",
    "[",
    "]",
    "{",
    "}",
    ",",
    ":",
    "[,]",
    "[1,]",
    "{,}",
    r#"{"a"}"#,
    r#"{"a":}"#,
    r#"{a:1}"#,
    r#"{1:2}"#,
    "[01]",
    "[-]",
    "[-x]",
    "[1.]",
    "[1.e5]",
    "[1e]",
    "[1e+]",
    "[.5]",
    "[+1]",
    "tru",
    "nul",
    "True",
    "NULL",
    "[tru]",
    "@",
    "#",
    "'x'",
    "[\"unterminated",
    "[\"a\nb\"]",
    "[\"a\tb\"]",
    "[\"\\x\"]",
    "[\"\\u12\"]",
    "[\"\\uZZZZ\"]",
    "[\"\\ud800\"]",
    "[\"\\udc00\"]",
    "[\"\\ud800\\u0041\"]",
    "[\"\\ud800\\ud800\"]",
];

#[test]
fn cfg96to107_documents_all_sources() {
    diff("cfg96-107 documents", |api, rec| unsafe {
        for (i, d) in DOCS.iter().enumerate() {
            probe_all_sources(api, rec, &format!("d{i}"), d.as_bytes());
        }
    });
}

#[test]
fn cfg105_invalid_utf8_inputs() {
    diff("cfg105 invalid UTF-8 inputs", |api, rec| unsafe {
        let cases: &[&[u8]] = &[
            b"[\"\xff\"]",
            b"[\"\xc2\"]",
            b"[\"\xc2\x41\"]",
            b"[\"\xe0\xa0\"]",
            b"[\"\xed\xa0\x80\"]",
            b"[\"\xf5\x80\x80\x80\"]",
            b"[\"\xc0\x80\"]",
            b"[\"\xf0\x90\x80\"]",
            b"\xff",
            b"[\xff]",
            b"{\"\xff\":1}",
            b"[\"a\xc2\x80b\"]",
        ];
        for (i, c) in cases.iter().enumerate() {
            probe_all_sources(api, rec, &format!("u{i}"), c);
        }
    });
}

#[test]
fn cfg107_nesting_depth_limits() {
    diff("cfg107 nesting depth", |api, rec| unsafe {
        for depth in [1usize, 2, 63, 64, 2046, 2047, 2048, 2049, 2100] {
            for open in ['[', '{'] {
                let mut s = String::new();
                for _ in 0..depth {
                    if open == '[' {
                        s.push('[');
                    } else {
                        s.push_str("{\"a\":");
                    }
                }
                s.push('1');
                for _ in 0..depth {
                    if open == '[' {
                        s.push(']');
                    } else {
                        s.push('}');
                    }
                }
                let z = cs(&s);
                let mut e = JsonError::patterned();
                let j = (api.json_loads)(z.as_ptr(), JSON_DECODE_ANY, &mut e);
                rec.json(&format!("d{depth}{open}"), j);
                rec.error(&format!("d{depth}{open}.err"), &e);
                if !j.is_null() {
                    // dump it back to prove the whole tree is identical
                    match dumps(api, j, JSON_ENCODE_ANY | JSON_COMPACT) {
                        None => rec.line("dump=NULL"),
                        Some(d) => rec.tag_u("dump_len", d.len()),
                    }
                }
                decref(api, j);
            }
        }
    });
}

#[test]
fn cfg108_loadb_embedded_nul_and_lengths() {
    diff("cfg108 json_loadb lengths", |api, rec| unsafe {
        let payloads: &[&[u8]] = &[
            b"[1,2,3]",
            b"[1,2,3]\0trailing",
            b"[\"a\0b\"]",
            b"{\"a\0b\":1}",
            b"\0",
            b"",
            b"[1]\0\0\0",
        ];
        for (i, p) in payloads.iter().enumerate() {
            for l in 0..=p.len() {
                for (fname, flags) in DECODE_FLAGS {
                    let mut e = JsonError::patterned();
                    let j = (api.json_loadb)(p.as_ptr() as *const c_char, l, *flags, &mut e);
                    rec.json(&format!("p{i}.l{l}.{fname}"), j);
                    rec_dump_all(api, rec, &format!("p{i}.l{l}.{fname}"), j);
                    rec.error(&format!("p{i}.l{l}.{fname}.err"), &e);
                    decref(api, j);
                }
            }
        }
    });
}

#[test]
fn cfg109and110_stdin_sources() {
    // `json_loadf(stdin)` / `json_loadfd(STDIN_FILENO)` select the "<stdin>"
    // error source.  Point stdin at /dev/null first so the input is empty and
    // deterministic for both libraries.
    diff("cfg109-110 stdin source", |api, rec| unsafe {
        let devnull = cs("/dev/null");
        let mode = cs("r");
        freopen(devnull.as_ptr(), mode.as_ptr(), stdin);
        for (fname, flags) in DECODE_FLAGS {
            let mut e = JsonError::patterned();
            let j = (api.json_loadf)(stdin, *flags, &mut e);
            rec.json(&format!("{fname}.loadf_stdin"), j);
            rec.error(&format!("{fname}.loadf_stdin_err"), &e);
            decref(api, j);

            let mut e = JsonError::patterned();
            let j = (api.json_loadfd)(0, *flags, &mut e);
            rec.json(&format!("{fname}.loadfd0"), j);
            rec.error(&format!("{fname}.loadfd0_err"), &e);
            decref(api, j);
        }
    });
}

#[test]
fn cfg112_load_callback_chunking() {
    diff("cfg112 load_callback chunking", |api, rec| unsafe {
        // A document longer than MAX_BUF_LEN (1024) so the refill path runs.
        let mut long = String::from("[");
        for i in 0..500 {
            if i > 0 {
                long.push(',');
            }
            long.push_str(&format!("{{\"key{i}\":\"value{i}\"}}"));
        }
        long.push(']');
        for chunk in [1usize, 2, 3, 7, 63, 64, 511, 512, 1023, 1024, 1025, 4096] {
            for err_at_end in [false, true] {
                cb_set(long.as_bytes(), chunk, err_at_end);
                let mut e = JsonError::patterned();
                let j = (api.json_load_callback)(Some(cb_feed), ptr::null_mut(), 0, &mut e);
                rec.json(&format!("c{chunk}.e{err_at_end}"), j);
                rec.error(&format!("c{chunk}.e{err_at_end}.err"), &e);
                if !j.is_null() {
                    match dumps(api, j, JSON_COMPACT) {
                        None => rec.line("dump=NULL"),
                        Some(d) => rec.tag_bytes("dump", &d),
                    }
                }
                decref(api, j);
            }
        }
        // truncated document delivered in chunks
        for cut in [0usize, 1, 5, 100, 1023, 1024, 1025] {
            let t = &long.as_bytes()[..cut.min(long.len())];
            cb_set(t, 1024, false);
            let mut e = JsonError::patterned();
            let j = (api.json_load_callback)(Some(cb_feed), ptr::null_mut(), 0, &mut e);
            rec.json(&format!("cut{cut}"), j);
            rec.error(&format!("cut{cut}.err"), &e);
            decref(api, j);
        }
    });
}

/* ------------------------------------------------- row 113: round trips - */

#[test]
fn cfg113_round_trip_random_documents() {
    diff("cfg113 round trip", |api, rec| unsafe {
        let mut rng = Rng::new(0xB130);
        for _ in 0..500 {
            let spec = rand_container(&mut rng, 3);
            let mut text = String::new();
            spec_to_text(&spec, &mut text);
            let z = cs(&text);
            for (fname, flags) in DECODE_FLAGS {
                let mut e = JsonError::patterned();
                let j = (api.json_loads)(z.as_ptr(), *flags, &mut e);
                rec.json(&format!("{fname}.j"), j);
                rec.error(&format!("{fname}.err"), &e);
                if !j.is_null() {
                    for ef in [
                        0usize,
                        JSON_COMPACT,
                        JSON_SORT_KEYS,
                        JSON_ENSURE_ASCII,
                        JSON_ESCAPE_SLASH,
                        json_indent(3),
                        JSON_SORT_KEYS | JSON_COMPACT | json_real_precision(7),
                    ] {
                        match dumps(api, j, ef) {
                            None => rec.line("redump=NULL"),
                            Some(d) => {
                                rec.tag_bytes("redump", &d);
                                // and re-parse the dump: must be stable
                                let z2 = cbuf(&d);
                                let j2 =
                                    (api.json_loads)(z2.as_ptr() as *const c_char, 0, ptr::null_mut());
                                rec.tag_i("stable", (api.json_equal)(j, j2) as i64);
                                decref(api, j2);
                            }
                        }
                    }
                }
                decref(api, j);
            }
        }
    });
}

/* ------------------------------- mutation fuzzing (valid + invalid mix) - */

#[test]
fn cfg102_mutation_fuzz() {
    diff("cfg102 mutation fuzz", |api, rec| unsafe {
        let mut rng = Rng::new(0x1020);
        let seeds: Vec<String> = {
            let mut r = Rng::new(0x1021);
            (0..40)
                .map(|_| {
                    let spec = rand_container(&mut r, 3);
                    let mut t = String::new();
                    spec_to_text(&spec, &mut t);
                    t
                })
                .collect()
        };
        for _ in 0..4000 {
            let base = seeds[rng.below(seeds.len())].as_bytes().to_vec();
            let mut m = base.clone();
            let nmut = 1 + rng.below(4);
            for _ in 0..nmut {
                if m.is_empty() {
                    break;
                }
                let pos = rng.below(m.len());
                match rng.below(4) {
                    0 => m[pos] = (rng.next_u32() & 0xFF) as u8,
                    1 => {
                        m.remove(pos);
                    }
                    2 => m.insert(pos, (rng.next_u32() & 0xFF) as u8),
                    _ => {
                        let c = b"[]{},:\"\\ 0123456789.eE+-tnufalse";
                        m[pos] = c[rng.below(c.len())];
                    }
                }
            }
            let flags = [0usize, 0x1F, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK]
                [rng.below(4)];
            let z = cbuf(&m);
            let mut e = JsonError::patterned();
            let j = (api.json_loads)(z.as_ptr() as *const c_char, flags, &mut e);
            rec.json("j", j);
            rec.error("err", &e);
            if !j.is_null() {
                match dumps(api, j, JSON_ENCODE_ANY | JSON_SORT_KEYS) {
                    None => rec.line("dump=NULL"),
                    Some(d) => rec.tag_bytes("dump", &d),
                }
            }
            decref(api, j);
            // same bytes through json_loadb (no NUL terminator semantics)
            let mut e = JsonError::patterned();
            let j = (api.json_loadb)(m.as_ptr() as *const c_char, m.len(), flags, &mut e);
            rec.json("jb", j);
            rec.error("errb", &e);
            decref(api, j);
        }
    });
}
