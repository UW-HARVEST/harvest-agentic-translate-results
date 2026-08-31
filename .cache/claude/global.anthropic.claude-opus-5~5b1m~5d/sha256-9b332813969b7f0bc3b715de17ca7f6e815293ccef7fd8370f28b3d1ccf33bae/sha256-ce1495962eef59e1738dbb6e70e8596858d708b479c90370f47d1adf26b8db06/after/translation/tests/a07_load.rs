//! Differential tests for `src/load.c` — CONFIGS.md rows 89..163
//! ("B. Decoding — load.c").
//!
//! Every case runs the *same* input bytes and the *same* flags through both
//! libraries and compares three independent observables:
//!
//!   1. whether the call returned NULL,
//!   2. the **complete byte image** of the caller's `json_error_t`
//!      (`json_error_t::raw()`), which pins line / column / position / source /
//!      message text / error code all at once — and, because the struct starts
//!      out `poisoned()`, also pins *which bytes the library wrote at all*,
//!   3. when the parse succeeded, a canonical re-dump of the resulting tree
//!      with `JSON_SORT_KEYS | JSON_ENCODE_ANY`, compared byte-for-byte.
//!
//! Randomised generators produce valid and malformed JSON text; each flag
//! combination gets hundreds of inputs rather than one hand-picked value.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;

// ---------------------------------------------------------------------------
// libc bits the load entry points need (a real FILE*, a real fd)
// ---------------------------------------------------------------------------

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fclose(f: *mut FILE) -> c_int;
}

/// The canonical re-dump used to compare two parsed trees. `JSON_SORT_KEYS`
/// removes any dependency on hash order (which is already pinned by `both()`,
/// but sorting makes a divergence in *content* obvious rather than a
/// divergence in *order*); `JSON_ENCODE_ANY` is required so bare scalars
/// parsed with `JSON_DECODE_ANY` can be dumped at all.
const CANON: size_t = JSON_SORT_KEYS | JSON_ENCODE_ANY;

// ---------------------------------------------------------------------------
// Observable snapshot of one decode call
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Snap {
    null: bool,
    err: (c_int, c_int, c_int, String, String, c_int),
    err_raw: Vec<u8>,
    dump_str: Option<String>,
    dump: Option<Vec<u8>>,
}

/// Snapshot a decode result. Does **not** decref `j`; the caller owns it.
unsafe fn snap(api: &Api, j: *mut json_t, err: &json_error_t) -> Snap {
    let (dump, dump_str) = if j.is_null() {
        (None, None)
    } else {
        let p = (api.json_dumps)(j, CANON);
        let b = cbytes(p);
        jfree(api, p as *mut c_void);
        let s = b.as_ref().map(|v| String::from_utf8_lossy(v).into_owned());
        (b, s)
    };
    Snap {
        null: j.is_null(),
        err: err.snapshot(),
        err_raw: err.raw(),
        dump_str,
        dump,
    }
}

impl Snap {
    fn compare(&self, other: &Snap, ctx: &str) {
        diff_eq!(self.null, other.null, "returned-NULL differs — {ctx}");
        diff_eq!(
            self.err.clone(),
            other.err.clone(),
            "json_error_t (line,col,pos,source,text,code) differs — {ctx}"
        );
        diff_eq!(
            self.err_raw.clone(),
            other.err_raw.clone(),
            "json_error_t raw byte image differs — {ctx}"
        );
        diff_eq!(
            self.dump_str.clone(),
            other.dump_str.clone(),
            "canonical re-dump differs — {ctx}"
        );
        diff_eq!(
            self.dump.clone(),
            other.dump.clone(),
            "canonical re-dump bytes differ — {ctx}"
        );
    }

    fn code(&self) -> c_int {
        self.err.5
    }
    fn text(&self) -> &str {
        &self.err.4
    }
}

/// Printable, unambiguous rendering of arbitrary input bytes.
fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &x in b.iter().take(300) {
        match x {
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(x as char),
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    if b.len() > 300 {
        s.push_str(&format!("...<{} bytes total>", b.len()));
    }
    s
}

// ---------------------------------------------------------------------------
// The six decoding entry points, each as a differential comparison
// ---------------------------------------------------------------------------

/// `json_loads` on both libraries. Returns the C-side snapshot so a caller can
/// additionally assert the ground-truth behaviour the CONFIGS row demands.
unsafe fn cmp_loads(c: &Api, r: &Api, text: &[u8], flags: size_t, ctx: &str) -> Snap {
    let buf = cs_bytes(text);
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();
    let cj = (c.json_loads)(buf.as_ptr(), flags, &mut ce);
    let rj = (r.json_loads)(buf.as_ptr(), flags, &mut re);
    let cs_ = snap(c, cj, &ce);
    let rs_ = snap(r, rj, &re);
    let full = format!(
        "json_loads(flags={flags:#x}) input={:?} [{ctx}]",
        show(text)
    );
    cs_.compare(&rs_, &full);
    decref(c, cj);
    decref(r, rj);
    cs_
}

unsafe fn cmp_loadb(
    c: &Api,
    r: &Api,
    text: &[u8],
    buflen: size_t,
    flags: size_t,
    ctx: &str,
) -> Snap {
    // Deliberately NOT NUL-terminated beyond the buffer: json_loadb must honour
    // buflen only.
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();
    let p = if text.is_empty() {
        b"".as_ptr() as *const c_char
    } else {
        text.as_ptr() as *const c_char
    };
    let cj = (c.json_loadb)(p, buflen, flags, &mut ce);
    let rj = (r.json_loadb)(p, buflen, flags, &mut re);
    let cs_ = snap(c, cj, &ce);
    let rs_ = snap(r, rj, &re);
    let full = format!(
        "json_loadb(buflen={buflen}, flags={flags:#x}) input={:?} [{ctx}]",
        show(text)
    );
    cs_.compare(&rs_, &full);
    decref(c, cj);
    decref(r, rj);
    cs_
}

fn tmp_path(tag: &str) -> std::path::PathBuf {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    std::path::Path::new(&dir).join(format!("a07_load_{tag}_{}.json", std::process::id()))
}

/// `json_loadf` through a real `FILE*` obtained from `fopen`. The file is
/// reopened for each library so both see the same stream position.
unsafe fn cmp_loadf(c: &Api, r: &Api, text: &[u8], flags: size_t, path: &std::path::Path, ctx: &str) {
    std::fs::write(path, text).expect("write temp file");
    let cpath = cs(path.to_str().unwrap());
    let mode = cs("rb");

    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();

    let cf = fopen(cpath.as_ptr(), mode.as_ptr());
    assert!(!cf.is_null(), "fopen failed for {}", path.display());
    let cj = (c.json_loadf)(cf, flags, &mut ce);
    fclose(cf);

    let rf = fopen(cpath.as_ptr(), mode.as_ptr());
    assert!(!rf.is_null());
    let rj = (r.json_loadf)(rf, flags, &mut re);
    fclose(rf);

    let cs_ = snap(c, cj, &ce);
    let rs_ = snap(r, rj, &re);
    cs_.compare(
        &rs_,
        &format!("json_loadf(flags={flags:#x}) input={:?} [{ctx}]", show(text)),
    );
    decref(c, cj);
    decref(r, rj);
}

/// `json_loadfd` through a real file descriptor.
unsafe fn cmp_loadfd(
    c: &Api,
    r: &Api,
    text: &[u8],
    flags: size_t,
    path: &std::path::Path,
    ctx: &str,
) {
    std::fs::write(path, text).expect("write temp file");

    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();

    let f1 = std::fs::File::open(path).expect("open fd");
    let cj = (c.json_loadfd)(f1.as_raw_fd(), flags, &mut ce);
    drop(f1);

    let f2 = std::fs::File::open(path).expect("open fd");
    let rj = (r.json_loadfd)(f2.as_raw_fd(), flags, &mut re);
    drop(f2);

    let cs_ = snap(c, cj, &ce);
    let rs_ = snap(r, rj, &re);
    cs_.compare(
        &rs_,
        &format!("json_loadfd(flags={flags:#x}) input={:?} [{ctx}]", show(text)),
    );
    decref(c, cj);
    decref(r, rj);
}

/// `json_load_file`.
unsafe fn cmp_load_file(
    c: &Api,
    r: &Api,
    text: &[u8],
    flags: size_t,
    path: &std::path::Path,
    ctx: &str,
) {
    std::fs::write(path, text).expect("write temp file");
    let cpath = cs(path.to_str().unwrap());
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();
    let cj = (c.json_load_file)(cpath.as_ptr(), flags, &mut ce);
    let rj = (r.json_load_file)(cpath.as_ptr(), flags, &mut re);
    let cs_ = snap(c, cj, &ce);
    let rs_ = snap(r, rj, &re);
    cs_.compare(
        &rs_,
        &format!(
            "json_load_file(flags={flags:#x}) input={:?} [{ctx}]",
            show(text)
        ),
    );
    decref(c, cj);
    decref(r, rj);
}

// ---- json_load_callback -----------------------------------------------------

const CB_ZERO: c_int = 0; // returns 0 immediately (empty input)
const CB_ONE_BYTE: c_int = 1; // one byte per call
const CB_BIG: c_int = 2; // as much as the buffer allows
const CB_SHORT: c_int = 3; // stops short after `stop_after` bytes
const CB_ERROR: c_int = 4; // returns (size_t)-1

#[repr(C)]
struct CbState {
    data: *const u8,
    len: usize,
    pos: usize,
    stop_after: usize,
    mode: c_int,
    calls: usize,
}

unsafe extern "C" fn loader_cb(buf: *mut c_void, buflen: size_t, arg: *mut c_void) -> size_t {
    let st = arg as *mut CbState;
    (*st).calls += 1;
    match (*st).mode {
        CB_ZERO => 0,
        CB_ERROR => usize::MAX,
        CB_ONE_BYTE => {
            if (*st).pos >= (*st).len {
                return 0;
            }
            *(buf as *mut u8) = *(*st).data.add((*st).pos);
            (*st).pos += 1;
            1
        }
        CB_BIG => {
            let n = core::cmp::min(buflen, (*st).len - (*st).pos);
            if n == 0 {
                return 0;
            }
            core::ptr::copy_nonoverlapping((*st).data.add((*st).pos), buf as *mut u8, n);
            (*st).pos += n;
            n
        }
        CB_SHORT => {
            let limit = core::cmp::min((*st).stop_after, (*st).len);
            if (*st).pos >= limit {
                return 0;
            }
            let n = core::cmp::min(buflen, limit - (*st).pos);
            core::ptr::copy_nonoverlapping((*st).data.add((*st).pos), buf as *mut u8, n);
            (*st).pos += n;
            n
        }
        _ => 0,
    }
}

unsafe fn cmp_load_callback(
    c: &Api,
    r: &Api,
    text: &[u8],
    mode: c_int,
    stop_after: usize,
    flags: size_t,
    ctx: &str,
) {
    let mk = || CbState {
        data: if text.is_empty() {
            b"".as_ptr()
        } else {
            text.as_ptr()
        },
        len: text.len(),
        pos: 0,
        stop_after,
        mode,
        calls: 0,
    };
    let mut cst = mk();
    let mut rst = mk();
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();
    let cj = (c.json_load_callback)(
        Some(loader_cb),
        &mut cst as *mut CbState as *mut c_void,
        flags,
        &mut ce,
    );
    let rj = (r.json_load_callback)(
        Some(loader_cb),
        &mut rst as *mut CbState as *mut c_void,
        flags,
        &mut re,
    );
    let cs_ = snap(c, cj, &ce);
    let rs_ = snap(r, rj, &re);
    let full = format!(
        "json_load_callback(mode={mode}, stop_after={stop_after}, flags={flags:#x}) \
         input={:?} [{ctx}]",
        show(text)
    );
    cs_.compare(&rs_, &full);
    // The callback protocol itself must be driven identically: same number of
    // callback invocations and same number of bytes consumed.
    diff_eq!(cst.calls, rst.calls, "callback invocation count — {full}");
    diff_eq!(cst.pos, rst.pos, "callback bytes consumed — {full}");
    decref(c, cj);
    decref(r, rj);
}

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

fn ws(rng: &mut Rng, out: &mut Vec<u8>) {
    // All four characters the lex_scan skip loop accepts, in random runs;
    // '\n' additionally exercises line/last_column bookkeeping.
    let n = match rng.below(6) {
        0 => 1,
        1 => 2,
        2 => 3,
        _ => 0,
    };
    for _ in 0..n {
        let ch = *rng.choice(&[b' ', b'\t', b'\n', b'\r']);
        out.push(ch);
    }
}

fn push_u_escape(out: &mut Vec<u8>, v: u32, upper: bool) {
    out.extend_from_slice(b"\\u");
    let digits: &[u8] = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    for shift in [12u32, 8, 4, 0] {
        out.push(digits[((v >> shift) & 0xF) as usize]);
    }
}

fn push_utf8(out: &mut Vec<u8>, cp: u32) {
    if let Some(ch) = char::from_u32(cp) {
        let mut b = [0u8; 4];
        out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
    }
}

/// A random *valid* JSON string literal, covering: raw ASCII, every short
/// escape form, `\uXXXX` in each `utf8_encode` width, valid surrogate pairs,
/// and raw 2/3/4-byte UTF-8.
fn gen_string(rng: &mut Rng, out: &mut Vec<u8>, allow_nul: bool) {
    out.push(b'"');
    let n = rng.below(10);
    for _ in 0..n {
        match rng.below(11) {
            0 | 1 => {
                let mut ch = 0x20 + rng.below(0x5f) as u8;
                if ch == b'"' || ch == b'\\' {
                    ch = b'x';
                }
                out.push(ch);
            }
            2 => {
                let e: &[u8] = rng.choice(&[
                    &b"\\\""[..],
                    b"\\\\",
                    b"\\/",
                    b"\\b",
                    b"\\f",
                    b"\\n",
                    b"\\r",
                    b"\\t",
                ]);
                out.extend_from_slice(e);
            }
            3 | 4 => {
                // \uXXXX, non-surrogate. Covers 1-, 2- and 3-byte utf8_encode.
                let mut v = match rng.below(4) {
                    0 => rng.below(0x80) as u32,
                    1 => 0x80 + rng.below(0x780) as u32,
                    2 => 0x800 + rng.below(0xf800) as u32,
                    _ => *rng.choice(&[0x41u32, 0x7f, 0x80, 0x7ff, 0x800, 0xffff, 0xfffd]),
                };
                if (0xD800..=0xDFFF).contains(&v) {
                    v = 0x41;
                }
                if v == 0 && !allow_nul {
                    v = 0x41;
                }
                push_u_escape(out, v, rng.bool());
            }
            5 => {
                // valid surrogate pair -> 4-byte UTF-8
                let hi = 0xD800 + rng.below(0x400) as u32;
                let lo = 0xDC00 + rng.below(0x400) as u32;
                push_u_escape(out, hi, rng.bool());
                push_u_escape(out, lo, rng.bool());
            }
            6 | 7 | 8 => {
                // raw multi-byte UTF-8 straight through stream_get
                let cp = match rng.below(4) {
                    0 => 0x80 + rng.below(0x780) as u32,
                    1 => 0x800 + rng.below(0xd800 - 0x800) as u32,
                    2 => 0xe000 + rng.below(0x10000 - 0xe000) as u32,
                    _ => 0x10000 + rng.below(0x100000) as u32,
                };
                push_utf8(out, cp);
            }
            9 => {
                if allow_nul && rng.below(3) == 0 {
                    push_u_escape(out, 0, rng.bool());
                } else {
                    let m = rng.below(40);
                    for _ in 0..m {
                        out.push(b'a' + rng.below(26) as u8);
                    }
                }
            }
            _ => {
                let m = rng.below(6);
                for _ in 0..m {
                    out.push(b'0' + rng.below(10) as u8);
                }
            }
        }
    }
    out.push(b'"');
}

const NUMBER_SPECIALS: &[&[u8]] = &[
    b"0",
    b"-0",
    b"0.0",
    b"-0.0",
    b"0e0",
    b"-0e0",
    b"1",
    b"-1",
    b"42",
    b"-42",
    b"9007199254740993",
    b"9223372036854775807",
    b"-9223372036854775808",
    b"9223372036854775808",
    b"-9223372036854775809",
    b"99999999999999999999999999999999",
    b"1e2",
    b"1E2",
    b"1e+2",
    b"1e-2",
    b"1.5e5",
    b"1.5E-5",
    b"-1.5e+3",
    b"1e309",
    b"-1e309",
    b"1e999999",
    b"1e-309",
    b"1e-999999",
    b"-1e-999999",
    b"1.7976931348623157e308",
    b"1.7976931348623159e308",
    b"2.2250738585072011e-308",
    b"5e-324",
];

/// Every number form the lexer distinguishes: sign, leading zero, digit runs,
/// fraction, `e`/`E` with `+`/`-`/bare, int64 boundaries and beyond.
fn gen_number(rng: &mut Rng, out: &mut Vec<u8>) {
    if rng.below(3) == 0 {
        out.extend_from_slice(rng.choice(NUMBER_SPECIALS));
        return;
    }
    if rng.bool() {
        out.push(b'-');
    }
    if rng.below(6) == 0 {
        out.push(b'0');
    } else {
        let n = 1 + rng.below(21);
        out.push(b'1' + rng.below(9) as u8);
        for _ in 1..n {
            out.push(b'0' + rng.below(10) as u8);
        }
    }
    if rng.bool() {
        out.push(b'.');
        let n = 1 + rng.below(20);
        for _ in 0..n {
            out.push(b'0' + rng.below(10) as u8);
        }
    }
    if rng.bool() {
        out.push(if rng.bool() { b'e' } else { b'E' });
        match rng.below(3) {
            0 => out.push(b'+'),
            1 => out.push(b'-'),
            _ => {}
        }
        let n = 1 + rng.below(4);
        for _ in 0..n {
            out.push(b'0' + rng.below(10) as u8);
        }
    }
}

fn gen_scalar(rng: &mut Rng, out: &mut Vec<u8>, allow_nul: bool) {
    match rng.below(8) {
        0 | 1 | 2 => gen_string(rng, out, allow_nul),
        3 | 4 | 5 => gen_number(rng, out),
        6 => {
            let s: &[u8] = rng.choice(&[&b"true"[..], b"false"]);
            out.extend_from_slice(s);
        }
        _ => out.extend_from_slice(b"null"),
    }
}

fn gen_object(rng: &mut Rng, depth: usize, out: &mut Vec<u8>, allow_nul: bool) {
    out.push(b'{');
    let n = rng.below(4);
    for i in 0..n {
        if i > 0 {
            ws(rng, out);
            out.push(b',');
        }
        ws(rng, out);
        // Keys are drawn from a small pool a good fraction of the time so that
        // duplicate keys (and therefore JSON_REJECT_DUPLICATES) fire often.
        if rng.below(3) == 0 {
            let k: &[u8] = rng.choice(&[&b"\"a\""[..], b"\"b\"", b"\"k\"", b"\"\\u0041\"", b"\"A\""]);
            out.extend_from_slice(k);
        } else {
            gen_string(rng, out, false);
        }
        ws(rng, out);
        out.push(b':');
        ws(rng, out);
        gen_element(rng, depth, out, allow_nul);
    }
    ws(rng, out);
    out.push(b'}');
}

fn gen_array(rng: &mut Rng, depth: usize, out: &mut Vec<u8>, allow_nul: bool) {
    out.push(b'[');
    let n = rng.below(5);
    for i in 0..n {
        if i > 0 {
            ws(rng, out);
            out.push(b',');
        }
        ws(rng, out);
        gen_element(rng, depth, out, allow_nul);
    }
    ws(rng, out);
    out.push(b']');
}

fn gen_element(rng: &mut Rng, depth: usize, out: &mut Vec<u8>, allow_nul: bool) {
    if depth == 0 {
        gen_scalar(rng, out, allow_nul);
    } else {
        match rng.below(6) {
            0 => gen_object(rng, depth - 1, out, allow_nul),
            1 => gen_array(rng, depth - 1, out, allow_nul),
            _ => gen_scalar(rng, out, allow_nul),
        }
    }
}

/// A complete, valid JSON document. 4 out of 5 documents have a container at
/// the top so that they parse with `flags = 0` too.
fn gen_doc(rng: &mut Rng, allow_nul: bool) -> Vec<u8> {
    let mut out = Vec::new();
    ws(rng, &mut out);
    let depth = rng.below(4);
    if rng.below(5) == 0 {
        gen_scalar(rng, &mut out, allow_nul);
    } else if rng.bool() {
        gen_object(rng, depth, &mut out, allow_nul);
    } else {
        gen_array(rng, depth, &mut out, allow_nul);
    }
    ws(rng, &mut out);
    out
}

const GARBAGE: &[u8] = b"{}[]:,\"\\ \t\n\rabcdeghinopqrstuvxyzABCDEFTN0123456789+-.eE'`~!@#$%^&*()_=|<>?/;\x00\x01\x0b\x1f\x7f\x80\xc0\xc1\xe2\xed\xf5\xff";

const MALFORMED_FIXED: &[&[u8]] = &[
    // truncated / unclosed containers
    b"",
    b"[",
    b"{",
    b"[1",
    b"[1,",
    b"[1,2",
    b"{\"a\"",
    b"{\"a\":",
    b"{\"a\":1",
    b"{\"a\":1,",
    b"[[[",
    b"{\"a\":[1,{\"b\":2",
    // trailing garbage
    b"[1] x",
    b"{\"a\":1} garbage",
    b"[1][2]",
    b"[1,2] [3]",
    b"null null",
    b"true false",
    // bad escapes
    b"[\"\\x\"]",
    b"[\"\\a\"]",
    b"[\"\\ \"]",
    b"[\"\\\"]",
    b"[\"\\u\"]",
    b"[\"\\u12\"]",
    b"[\"\\u123\"]",
    b"[\"\\u12g4\"]",
    b"[\"\\uZZZZ\"]",
    b"[\"\\u00g0\"]",
    // lone / broken surrogates
    b"[\"\\ud834\"]",
    b"[\"\\ud834abc\"]",
    b"[\"\\ud834\\u0041\"]",
    b"[\"\\udd1e\"]",
    b"[\"\\udc00\"]",
    b"[\"\\ud800\\ud800\"]",
    b"[\"\\udfff\"]",
    // bad numbers
    b"[01]",
    b"[00]",
    b"[-01]",
    b"[-]",
    b"[1.]",
    b"[1e]",
    b"[1e+]",
    b"[1e-]",
    b"[.1]",
    b"[+1]",
    b"[1..2]",
    b"[1e1e1]",
    b"[--1]",
    b"[0x10]",
    b"[1e309]",
    b"[9223372036854775808]",
    // bad tokens / structure
    b"[True]",
    b"[TRUE]",
    b"[nulll]",
    b"[nul]",
    b"[tru]",
    b"[undefined]",
    b"[NaN]",
    b"[Infinity]",
    b"{1:2}",
    b"{,}",
    b"{\"a\" 1}",
    b"{\"a\":1,}",
    b"[1 2]",
    b"[1,]",
    b"[,1]",
    b"}",
    b"]",
    b":",
    b",",
    b"[\"abc",
    b"[\"",
    // invalid UTF-8
    b"[\"\x80\"]",
    b"[\"\xc0\x41\"]",
    b"[\"\xe2\x82\"]",
    b"[\"\xc0\x80\"]",
    b"[\"\xff\"]",
    b"[\"\xed\xa0\x80\"]",
    b"[\xc3\xa9]",
    b"\xef\xbb\xbf[1]",
];

/// A random malformed document: either one of the hand-written cases above, or
/// a mutation of a valid one (truncation, byte flip/insert/delete, trailing
/// garbage, stripped closers, injected bad escape, duplicated byte).
fn gen_malformed(rng: &mut Rng) -> Vec<u8> {
    if rng.below(3) == 0 {
        return rng.choice(MALFORMED_FIXED).to_vec();
    }
    let mut d = gen_doc(rng, true);
    if d.is_empty() {
        d.extend_from_slice(b"[1]");
    }
    match rng.below(10) {
        0 => {
            let k = rng.below(d.len());
            d.truncate(k);
        }
        1 => {
            let k = rng.below(d.len());
            d.remove(k);
        }
        2 => {
            let k = rng.below(d.len());
            d[k] = *rng.choice(GARBAGE);
        }
        3 => {
            let k = rng.below(d.len() + 1);
            let g = *rng.choice(GARBAGE);
            d.insert(k, g);
        }
        4 => {
            let t: &[u8] = rng.choice(&[
                &b" x"[..],
                b"]",
                b"}",
                b",",
                b"[1]",
                b"garbage",
                b" \x00[2]",
                b"\xff",
            ]);
            d.extend_from_slice(t);
        }
        5 => {
            let n = 1 + rng.below(3);
            let l = d.len().saturating_sub(n);
            d.truncate(l);
        }
        6 => {
            let k = rng.below(d.len() + 1);
            let e: &[u8] = rng.choice(&[
                &b"\"\\x\""[..],
                b"\"\\ud834\"",
                b"\"\\u12\"",
                b"\"\\\"",
                b"\"\x01\"",
                b"\"\x80\"",
            ]);
            for (i, b) in e.iter().enumerate() {
                d.insert(k + i, *b);
            }
        }
        7 => {
            let k = rng.below(d.len());
            let b = d[k];
            d.insert(k, b);
        }
        8 => {
            // splice a bad number in
            let k = rng.below(d.len() + 1);
            let e: &[u8] = rng.choice(&[&b"01"[..], b"1.", b"1e", b"-", b"+2", b".5", b"1e+"]);
            for (i, b) in e.iter().enumerate() {
                d.insert(k + i, *b);
            }
        }
        _ => {
            // raw control character injected inside
            let k = rng.below(d.len() + 1);
            let g = *rng.choice(b"\x00\x01\x02\x07\x0a\x0b\x0c\x0d\x1f");
            d.insert(k, g);
        }
    }
    d
}

/// The decoding-flag combinations every randomised loop sweeps.
const FLAG_SETS: &[size_t] = &[
    0,
    JSON_REJECT_DUPLICATES,
    JSON_DISABLE_EOF_CHECK,
    JSON_DECODE_ANY,
    JSON_DECODE_INT_AS_REAL,
    JSON_ALLOW_NUL,
    JSON_DECODE_ANY | JSON_ALLOW_NUL,
    JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
    JSON_REJECT_DUPLICATES | JSON_DECODE_ANY,
    JSON_ALLOW_NUL | JSON_REJECT_DUPLICATES,
    JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY,
    JSON_REJECT_DUPLICATES
        | JSON_DISABLE_EOF_CHECK
        | JSON_DECODE_ANY
        | JSON_DECODE_INT_AS_REAL
        | JSON_ALLOW_NUL,
];

// ===========================================================================
// Rows 89-93 — object/array shapes
// ===========================================================================

#[test]
fn rows_89_93_container_shapes() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // rows 89, 90: the two early-return paths.
        let s = cmp_loads(c, r, b"{}", 0, "row 89 empty object");
        assert!(!s.null, "C: {{}} must parse");
        assert_eq!(s.dump_str.as_deref(), Some("{}"), "C: {{}} dumps as {{}}");
        let s = cmp_loads(c, r, b"[]", 0, "row 90 empty array");
        assert_eq!(s.dump_str.as_deref(), Some("[]"));

        // row 91: single member; row 92: three members with two comma branches.
        let s = cmp_loads(c, r, b"{\"a\":1}", 0, "row 91");
        assert_eq!(s.dump_str.as_deref(), Some("{\"a\": 1}"));
        let s = cmp_loads(c, r, b"{\"a\":1,\"b\":2,\"c\":3}", 0, "row 92");
        assert_eq!(s.dump_str.as_deref(), Some("{\"a\": 1, \"b\": 2, \"c\": 3}"));

        // row 93: one vs many array elements.
        let s = cmp_loads(c, r, b"[1]", 0, "row 93 single");
        assert_eq!(s.dump_str.as_deref(), Some("[1]"));
        let s = cmp_loads(c, r, b"[1,2,3,4,5]", 0, "row 93 many");
        assert_eq!(s.dump_str.as_deref(), Some("[1, 2, 3, 4, 5]"));

        // Randomised: many member/element counts, nested both ways.
        let mut rng = Rng::new(0x0089_0093);
        for i in 0..600 {
            let mut d = Vec::new();
            let n = rng.below(12);
            if rng.bool() {
                d.push(b'{');
                for k in 0..n {
                    if k > 0 {
                        d.push(b',');
                    }
                    d.extend_from_slice(format!("\"k{k}\":").as_bytes());
                    gen_element(&mut rng, 2, &mut d, false);
                }
                d.push(b'}');
            } else {
                d.push(b'[');
                for k in 0..n {
                    if k > 0 {
                        d.push(b',');
                    }
                    gen_element(&mut rng, 2, &mut d, false);
                }
                d.push(b']');
            }
            cmp_loads(c, r, &d, 0, &format!("row 89-93 random #{i}"));
        }
    }
}

// ===========================================================================
// Rows 94-95 — whitespace handling
// ===========================================================================

#[test]
fn rows_94_95_whitespace() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let padded = b" \t\r\n{ \t\r\n \"a\" \t\r\n : \t\r\n 1 \t\r\n , \t\r\n \"b\" \t\r\n : \t\r\n [ \t\r\n ] \t\r\n } \t\r\n ";
        let a = cmp_loads(c, r, padded, 0, "row 94 whitespace everywhere");
        let b = cmp_loads(c, r, b"{\"a\":1,\"b\":[]}", 0, "row 94 whitespace-free");
        assert_eq!(a.dump, b.dump, "C: whitespace must not change the value");

        // row 95: trailing whitespace only, EOF check still passes.
        let s = cmp_loads(c, r, b"{\"a\":1}\n\t\r ", 0, "row 95 trailing whitespace");
        assert!(!s.null, "C: trailing whitespace is fine");

        // Randomised whitespace insertion: the value must never change, only
        // the reported position/line/column.
        let mut rng = Rng::new(0x0094_0095);
        for i in 0..600 {
            let core_doc = {
                let mut v = Vec::new();
                if rng.bool() {
                    gen_object(&mut rng, 2, &mut v, false);
                } else {
                    gen_array(&mut rng, 2, &mut v, false);
                }
                v
            };
            let plain = cmp_loads(c, r, &core_doc, 0, &format!("row 94 random #{i} plain"));
            // Wrap in leading/trailing whitespace (interior whitespace is
            // already produced by the generator).
            let mut padded2 = Vec::new();
            ws(&mut rng, &mut padded2);
            padded2.extend_from_slice(&core_doc);
            ws(&mut rng, &mut padded2);
            let wrapped = cmp_loads(c, r, &padded2, 0, &format!("row 95 random #{i} padded"));
            if !plain.null && !wrapped.null {
                assert_eq!(
                    plain.dump, wrapped.dump,
                    "C: leading/trailing whitespace changed the value for {:?}",
                    show(&core_doc)
                );
            }
        }
    }
}

// ===========================================================================
// Rows 96-100 — the EOF check and JSON_DISABLE_EOF_CHECK
// ===========================================================================

#[test]
fn rows_96_100_eof_check() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // row 96
        for t in [&b"{\"a\":1} garbage"[..], b"[1,2] [3]"] {
            let s = cmp_loads(c, r, t, 0, "row 96 trailing garbage");
            assert!(s.null, "C: trailing garbage must fail");
            assert_eq!(
                s.code(),
                JSON_ERROR_END_OF_INPUT_EXPECTED,
                "C: expected end_of_input_expected for {:?}",
                show(t)
            );
        }
        // row 97
        for t in [&b"{\"a\":1} garbage"[..], b"[1,2]}}}"] {
            let s = cmp_loads(
                c,
                r,
                t,
                JSON_DISABLE_EOF_CHECK,
                "row 97 DISABLE_EOF_CHECK",
            );
            assert!(!s.null, "C: DISABLE_EOF_CHECK must accept {:?}", show(t));
        }
        let s = cmp_loads(c, r, b"[1,2]}}}", JSON_DISABLE_EOF_CHECK, "row 97 value");
        assert_eq!(s.dump_str.as_deref(), Some("[1, 2]"), "C: only first value");

        // row 98: the flag must not alter a document that already ends cleanly.
        let a = cmp_loads(c, r, b"{\"a\":1}   ", 0, "row 98 flags=0");
        let b = cmp_loads(
            c,
            r,
            b"{\"a\":1}   ",
            JSON_DISABLE_EOF_CHECK,
            "row 98 DISABLE_EOF_CHECK",
        );
        assert_eq!(a.dump, b.dump, "C: DISABLE_EOF_CHECK changed the value");

        // row 99: concatenated values, position is bytes consumed by the first.
        let s = cmp_loads(c, r, b"[1][2][3]", JSON_DISABLE_EOF_CHECK, "row 99");
        assert_eq!(s.dump_str.as_deref(), Some("[1]"), "C: only [1] returned");
        assert_eq!(s.err.2, 3, "C: position == bytes consumed for the first value");

        // row 100: same input with the EOF check on.
        let s = cmp_loads(c, r, b"[1][2]", 0, "row 100");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_END_OF_INPUT_EXPECTED);

        // Randomised: a valid doc followed by another valid doc, with and
        // without the flag.
        let mut rng = Rng::new(0x0096_0100);
        for i in 0..400 {
            let mut d = Vec::new();
            if rng.bool() {
                gen_object(&mut rng, 1, &mut d, false);
            } else {
                gen_array(&mut rng, 1, &mut d, false);
            }
            let first = d.clone();
            ws(&mut rng, &mut d);
            if rng.bool() {
                gen_object(&mut rng, 1, &mut d, false);
            } else {
                gen_array(&mut rng, 1, &mut d, false);
            }
            let with = cmp_loads(
                c,
                r,
                &d,
                JSON_DISABLE_EOF_CHECK,
                &format!("row 97/99 random #{i}"),
            );
            let without = cmp_loads(c, r, &d, 0, &format!("row 96/100 random #{i}"));
            assert!(
                without.null,
                "C: concatenated values must fail without DISABLE_EOF_CHECK: {:?}",
                show(&d)
            );
            let solo = cmp_loads(c, r, &first, 0, &format!("row 99 random #{i} first only"));
            assert_eq!(
                with.dump, solo.dump,
                "C: DISABLE_EOF_CHECK must return exactly the first value for {:?}",
                show(&d)
            );
        }
    }
}

// ===========================================================================
// Rows 101-107 — duplicate keys and JSON_REJECT_DUPLICATES
// ===========================================================================

#[test]
fn rows_101_107_duplicate_keys() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // row 101: last one wins when the flag is off.
        let s = cmp_loads(c, r, b"{\"a\":1,\"a\":2,\"a\":3}", 0, "row 101");
        assert_eq!(s.dump_str.as_deref(), Some("{\"a\": 3}"), "C: LAST wins");

        // row 102
        let s = cmp_loads(
            c,
            r,
            b"{\"a\":1,\"a\":2}",
            JSON_REJECT_DUPLICATES,
            "row 102",
        );
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_DUPLICATE_KEY);

        // row 103: no duplicates -> identical to flags=0.
        let a = cmp_loads(c, r, b"{\"a\":1,\"b\":2}", 0, "row 103 flags=0");
        let b = cmp_loads(
            c,
            r,
            b"{\"a\":1,\"b\":2}",
            JSON_REJECT_DUPLICATES,
            "row 103 REJECT_DUPLICATES",
        );
        assert_eq!(a.dump, b.dump);

        // rows 104, 105: nested in an object / inside an array.
        for t in [
            &b"{\"x\":{\"a\":1,\"a\":2}}"[..],
            b"[{\"k\":1,\"k\":2}]",
            b"{\"x\":[{\"k\":1,\"k\":2}]}",
            b"[[[{\"z\":0,\"z\":1}]]]",
        ] {
            let s = cmp_loads(c, r, t, JSON_REJECT_DUPLICATES, "rows 104/105 nested");
            assert!(s.null, "C: nested duplicate must fail: {:?}", show(t));
            assert_eq!(s.code(), JSON_ERROR_DUPLICATE_KEY);
        }

        // row 106: keys equal only after escape decoding.
        for t in [
            &b"{\"a\":1,\"\\u0061\":2}"[..],
            b"{\"\\u0061\":1,\"a\":2}",
            b"{\"\\u0041\":1,\"A\":2}",
            b"{\"a\\/b\":1,\"a/b\":2}",
        ] {
            let s = cmp_loads(c, r, t, JSON_REJECT_DUPLICATES, "row 106 decoded dup");
            assert!(
                s.null,
                "C: escape-decoded duplicate must fail: {:?}",
                show(t)
            );
            assert_eq!(s.code(), JSON_ERROR_DUPLICATE_KEY);
            let s2 = cmp_loads(c, r, t, 0, "row 106 flags=0");
            assert!(!s2.null, "C: without the flag it must succeed");
        }

        // row 107: REJECT_DUPLICATES | DECODE_ANY act independently.
        let f = JSON_REJECT_DUPLICATES | JSON_DECODE_ANY;
        let s = cmp_loads(
            c,
            r,
            b"{\"o\":{\"a\":1,\"a\":2},\"arr\":[{\"b\":1}]}",
            f,
            "row 107 nested dup",
        );
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_DUPLICATE_KEY);
        let s = cmp_loads(c, r, b"42", f, "row 107 bare scalar");
        assert_eq!(s.dump_str.as_deref(), Some("42"));
        let s = cmp_loads(
            c,
            r,
            b"{\"o\":{\"a\":1},\"arr\":[{\"b\":1}]}",
            f,
            "row 107 dup-free",
        );
        assert!(!s.null);

        // Randomised: documents built from a tiny key pool so duplicates occur
        // at every nesting level, under both settings.
        let mut rng = Rng::new(0x0101_0107);
        let keys: &[&str] = &["a", "b", "\\u0061", "\\u0062", "A", "\\u0041", "k"];
        for i in 0..500 {
            let mut d = Vec::new();
            d.push(b'{');
            let n = rng.below(6);
            for j in 0..n {
                if j > 0 {
                    d.push(b',');
                }
                d.push(b'"');
                d.extend_from_slice(rng.choice(keys).as_bytes());
                d.extend_from_slice(b"\":");
                gen_element(&mut rng, 2, &mut d, false);
            }
            d.push(b'}');
            for &f in &[
                0,
                JSON_REJECT_DUPLICATES,
                JSON_REJECT_DUPLICATES | JSON_DECODE_ANY,
            ] {
                cmp_loads(c, r, &d, f, &format!("rows 101-107 random #{i}"));
            }
        }
    }
}

// ===========================================================================
// Rows 108-116 — JSON_DECODE_ANY and the top-level type check
// ===========================================================================

#[test]
fn rows_108_116_decode_any() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // row 108: bare scalars rejected with flags=0.
        for t in [&b"\"str\""[..], b"1", b"1.5", b"true", b"false", b"null"] {
            let s = cmp_loads(c, r, t, 0, "row 108 bare scalar, flags=0");
            assert!(s.null, "C: bare scalar must fail: {:?}", show(t));
            assert_eq!(s.code(), JSON_ERROR_INVALID_SYNTAX);
            assert!(
                s.text().contains("'[' or '{' expected"),
                "C: unexpected message {:?}",
                s.text()
            );
        }

        // row 109: every top-level type accepted with DECODE_ANY.
        let cases: &[(&[u8], c_int)] = &[
            (b"\"str\"", JSON_STRING),
            (b"12", JSON_INTEGER),
            (b"1.5", JSON_REAL),
            (b"true", JSON_TRUE),
            (b"false", JSON_FALSE),
            (b"null", JSON_NULL),
            (b"[]", JSON_ARRAY),
            (b"{}", JSON_OBJECT),
        ];
        for (t, ty) in cases {
            let s = cmp_loads(c, r, t, JSON_DECODE_ANY, "row 109");
            assert!(!s.null, "C: DECODE_ANY must accept {:?}", show(t));
            // Confirm the type directly as well as through the dump.
            let buf = cs_bytes(t);
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let cj = (c.json_loads)(buf.as_ptr(), JSON_DECODE_ANY, &mut ce);
            let rj = (r.json_loads)(buf.as_ptr(), JSON_DECODE_ANY, &mut re);
            assert!(!cj.is_null());
            diff_eq!(typeof_(cj), typeof_(rj), "json_typeof of {:?}", show(t));
            assert_eq!(typeof_(cj), *ty, "C: wrong type for {:?}", show(t));
            decref(c, cj);
            decref(r, rj);
        }

        // row 110: DECODE_ANY is a no-op for containers.
        for t in [&b"[1,2]"[..], b"{\"a\":1}"] {
            let a = cmp_loads(c, r, t, 0, "row 110 flags=0");
            let b = cmp_loads(c, r, t, JSON_DECODE_ANY, "row 110 DECODE_ANY");
            assert_eq!(a.dump, b.dump, "C: DECODE_ANY changed a container result");
        }

        // row 111
        let s = cmp_loads(c, r, b"  \t\n true \r\n ", JSON_DECODE_ANY, "row 111 ok");
        assert_eq!(s.dump_str.as_deref(), Some("true"));
        for t in [&b"  \t\n "[..], b"", b"\n\n\n", b"\t"] {
            let s = cmp_loads(c, r, t, JSON_DECODE_ANY, "row 111 whitespace only");
            assert!(s.null, "C: whitespace-only must fail: {:?}", show(t));
            // rows 112/113: the empty-saved_text remap.
            assert_eq!(
                s.code(),
                JSON_ERROR_PREMATURE_END_OF_INPUT,
                "C: invalid_syntax must be remapped to premature_end_of_input for {:?}",
                show(t)
            );
        }

        // rows 112, 113: empty input under both flag settings.
        for f in [0, JSON_DECODE_ANY] {
            let s = cmp_loads(c, r, b"", f, "rows 112/113 empty input");
            assert!(s.null);
            assert_eq!(s.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
        }

        // row 114
        let f = JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK;
        let s = cmp_loads(c, r, b"true false null", f, "row 114");
        assert_eq!(s.dump_str.as_deref(), Some("true"));
        let s = cmp_loads(c, r, b"1 2 3", f, "row 114 numbers");
        assert_eq!(s.dump_str.as_deref(), Some("1"));

        // row 115
        let s = cmp_loads(c, r, b"true false", JSON_DECODE_ANY, "row 115");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_END_OF_INPUT_EXPECTED);

        // row 116: bare number, no delimiter before EOF.
        let s = cmp_loads(c, r, b"12345", f, "row 116");
        assert_eq!(s.dump_str.as_deref(), Some("12345"));
        for t in [&b"12345"[..], b"-7", b"1.5", b"1e3", b"0"] {
            cmp_loads(c, r, t, f, "row 116 no delimiter");
            cmp_loads(c, r, t, JSON_DECODE_ANY, "row 116 EOF check on");
        }

        // Randomised sweep of every scalar top level under the relevant flags.
        let mut rng = Rng::new(0x0108_0116);
        for &f in &[
            0,
            JSON_DECODE_ANY,
            JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
            JSON_DISABLE_EOF_CHECK,
        ] {
            for i in 0..250 {
                let mut d = Vec::new();
                ws(&mut rng, &mut d);
                gen_scalar(&mut rng, &mut d, false);
                if rng.below(3) == 0 {
                    ws(&mut rng, &mut d);
                    gen_scalar(&mut rng, &mut d, false);
                }
                ws(&mut rng, &mut d);
                cmp_loads(c, r, &d, f, &format!("rows 108-116 random #{i}"));
            }
        }
    }
}

// ===========================================================================
// Rows 117-126 — integers, int64 bounds, JSON_DECODE_INT_AS_REAL
// ===========================================================================

#[test]
fn rows_117_126_integers_and_int_as_real() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // row 117
        let s = cmp_loads(
            c,
            r,
            b"[0, -0, 1, -1, 42, -42, 9007199254740993]",
            0,
            "row 117",
        );
        assert_eq!(
            s.dump_str.as_deref(),
            Some("[0, 0, 1, -1, 42, -42, 9007199254740993]"),
            "C: exact int64 values, 9007199254740993 not rounded"
        );

        // row 118
        let s = cmp_loads(
            c,
            r,
            b"[9223372036854775807, -9223372036854775808]",
            0,
            "row 118",
        );
        assert_eq!(
            s.dump_str.as_deref(),
            Some("[9223372036854775807, -9223372036854775808]")
        );

        // row 119: one past the bounds, and the two distinct messages.
        let s = cmp_loads(c, r, b"[9223372036854775808]", 0, "row 119 positive");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_NUMERIC_OVERFLOW);
        assert!(
            s.text().contains("too big integer"),
            "C: got {:?}",
            s.text()
        );
        let s = cmp_loads(c, r, b"[-9223372036854775809]", 0, "row 119 negative");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_NUMERIC_OVERFLOW);
        assert!(
            s.text().contains("too big negative integer"),
            "C: got {:?}",
            s.text()
        );

        // row 120
        let s = cmp_loads(
            c,
            r,
            b"[99999999999999999999999999999999]",
            0,
            "row 120",
        );
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_NUMERIC_OVERFLOW);

        // row 121
        let s = cmp_loads(
            c,
            r,
            b"[0, -0, 1, -1, 42, -42, 9007199254740993]",
            JSON_DECODE_INT_AS_REAL,
            "row 121",
        );
        assert!(!s.null);
        let d = s.dump_str.clone().unwrap();
        assert!(
            d.contains("9007199254740992.0"),
            "C: 9007199254740993 must become 9007199254740992.0, got {d}"
        );
        assert!(d.contains("0.0") && d.contains("1.0"), "C: all reals, got {d}");

        // row 122: overflowing integers become reals with the flag.
        let s = cmp_loads(
            c,
            r,
            b"[9223372036854775808, -9223372036854775809, 99999999999999999999]",
            JSON_DECODE_INT_AS_REAL,
            "row 122",
        );
        assert!(
            !s.null,
            "C: INT_AS_REAL must accept int64-overflowing literals"
        );

        // row 123
        let s = cmp_loads(
            c,
            r,
            b"[9223372036854775807, -9223372036854775808]",
            JSON_DECODE_INT_AS_REAL,
            "row 123",
        );
        assert_eq!(
            s.dump_str.as_deref(),
            Some("[9.223372036854776e18, -9.223372036854776e18]"),
            "C: int64 bounds round to +-2^63 as reals"
        );

        // row 124: exponents bypass the integer path regardless of the flag.
        for f in [0, JSON_DECODE_INT_AS_REAL] {
            let s = cmp_loads(c, r, b"[1e2, 1E2, 1e+2, 1e-2, 0e0, -0e0]", f, "row 124");
            assert!(!s.null);
            assert_eq!(
                s.dump_str.as_deref(),
                Some("[100.0, 100.0, 100.0, 0.01, 0.0, -0.0]"),
                "C: all reals regardless of INT_AS_REAL"
            );
        }
        // row 125: fractions likewise.
        for f in [0, JSON_DECODE_INT_AS_REAL] {
            let s = cmp_loads(c, r, b"[1.0, -1.5, 0.5, 0.0]", f, "row 125");
            assert_eq!(s.dump_str.as_deref(), Some("[1.0, -1.5, 0.5, 0.0]"));
        }

        // row 126
        let f = JSON_DECODE_INT_AS_REAL | JSON_DECODE_ANY;
        for t in [&b"42"[..], b"-42", b"0", b"9223372036854775808", b"1e3"] {
            let s = cmp_loads(c, r, t, f, "row 126 INT_AS_REAL|DECODE_ANY");
            assert!(!s.null, "C: must parse {:?}", show(t));
            assert!(
                s.dump_str.as_deref().unwrap().contains('.')
                    || s.dump_str.as_deref().unwrap().contains('e'),
                "C: {:?} must dump as a real, got {:?}",
                show(t),
                s.dump_str
            );
        }
        for (t, want_null) in [
            (&b"42"[..], false),
            (b"-42", false),
            (b"0", false),
            (b"9223372036854775808", true),
        ] {
            let s = cmp_loads(c, r, t, JSON_DECODE_ANY, "row 126 DECODE_ANY only");
            assert_eq!(
                s.null,
                want_null,
                "C: DECODE_ANY-only classification of {:?}",
                show(t)
            );
            if !want_null {
                assert!(
                    !s.dump_str.as_deref().unwrap().contains('.'),
                    "C: {:?} must stay an integer",
                    show(t)
                );
            } else {
                assert_eq!(s.code(), JSON_ERROR_NUMERIC_OVERFLOW);
            }
        }

        // Randomised: random integer literals near/over the int64 bounds under
        // both settings.
        let mut rng = Rng::new(0x0117_0126);
        for &f in &[
            0,
            JSON_DECODE_INT_AS_REAL,
            JSON_DECODE_ANY,
            JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
        ] {
            for i in 0..250 {
                let mut d = Vec::new();
                let container = rng.below(3) != 0;
                if container {
                    d.push(b'[');
                }
                let n = 1 + rng.below(4);
                for k in 0..n {
                    if k > 0 {
                        d.push(b',');
                    }
                    if rng.below(4) == 0 {
                        // exact int64 boundary neighbourhoods
                        let v = rng.json_int();
                        d.extend_from_slice(format!("{v}").as_bytes());
                    } else {
                        gen_number(&mut rng, &mut d);
                    }
                    if !container {
                        break;
                    }
                }
                if container {
                    d.push(b']');
                }
                cmp_loads(c, r, &d, f, &format!("rows 117-126 random #{i}"));
            }
        }
    }
}

// ===========================================================================
// Rows 127-133 — number syntax and real overflow/underflow
// ===========================================================================

#[test]
fn rows_127_133_number_syntax() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // row 127: leading zeros rejected...
        for t in [&b"[01]"[..], b"[00]", b"[-01]", b"[000]", b"[-00]"] {
            let s = cmp_loads(c, r, t, 0, "row 127 leading zero");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert_eq!(s.code(), JSON_ERROR_INVALID_SYNTAX);
        }
        // ...but these do parse.
        for t in [&b"[0]"[..], b"[-0]", b"[0.5]", b"[0e1]", b"[-0.0]"] {
            let s = cmp_loads(c, r, t, 0, "row 127 valid zero form");
            assert!(!s.null, "C: {:?} must parse", show(t));
        }

        // row 128
        for t in [&b"[-]"[..], b"[1.]", b"[1e]", b"[1e+]", b"[1e-]", b"[1E]", b"[-.]"] {
            let s = cmp_loads(c, r, t, 0, "row 128 bad number body");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert_eq!(
                s.code(),
                JSON_ERROR_INVALID_SYNTAX,
                "C: {:?} gave {:?}",
                show(t),
                s.text()
            );
        }

        // row 129: all exponent spellings.
        let s = cmp_loads(
            c,
            r,
            b"[1e5, 1E5, 1e+5, 1E+5, 1e-5, 1E-5, 1.5e5, 1.5E-5, 0e0, -1.5e+3]",
            0,
            "row 129",
        );
        assert_eq!(
            s.dump_str.as_deref(),
            Some(
                "[100000.0, 100000.0, 100000.0, 100000.0, 1e-5, 1e-5, \
                 150000.0, 1.5e-5, 0.0, -1500.0]"
            ),
            "C: exponent spellings"
        );

        // row 130: overflow to infinity is an error.
        for t in [&b"[1e309]"[..], b"[-1e309]", b"[1e999999]", b"[-1e999999]"] {
            let s = cmp_loads(c, r, t, 0, "row 130 real overflow");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert_eq!(s.code(), JSON_ERROR_NUMERIC_OVERFLOW);
            assert!(
                s.text().contains("real number overflow"),
                "C: got {:?}",
                s.text()
            );
        }

        // row 131: underflow is NOT an error.
        let s = cmp_loads(
            c,
            r,
            b"[1e-309, 1e-999999, -1e-999999]",
            0,
            "row 131 underflow",
        );
        assert!(!s.null, "C: underflow must succeed");
        let d = s.dump_str.clone().unwrap();
        assert!(
            d.contains("-0.0"),
            "C: sign of negative zero must survive, got {d}"
        );

        // row 132: huge plain integer, flag on vs off -> different errors.
        let big: Vec<u8> = {
            let mut v = Vec::new();
            v.push(b'[');
            v.push(b'1');
            for _ in 0..400 {
                v.push(b'0');
            }
            v.push(b']');
            v
        };
        let with = cmp_loads(c, r, &big, JSON_DECODE_INT_AS_REAL, "row 132 INT_AS_REAL");
        let without = cmp_loads(c, r, &big, 0, "row 132 flags=0");
        assert!(with.null && without.null, "C: both must fail");
        assert!(
            with.text().contains("real number overflow"),
            "C: INT_AS_REAL message {:?}",
            with.text()
        );
        assert!(
            without.text().contains("too big integer"),
            "C: flags=0 message {:?}",
            without.text()
        );
        assert_ne!(
            with.text(),
            without.text(),
            "C: the two overflow messages must differ"
        );

        // row 133: DBL_MAX boundary.
        let s = cmp_loads(c, r, b"[1.7976931348623157e308]", 0, "row 133 ok");
        assert!(!s.null, "C: DBL_MAX must parse");
        let s = cmp_loads(c, r, b"[1.7976931348623159e308]", 0, "row 133 overflow");
        assert!(s.null, "C: just past DBL_MAX must overflow");
        assert_eq!(s.code(), JSON_ERROR_NUMERIC_OVERFLOW);

        // Randomised number forms, valid and invalid, in every flag setting.
        let mut rng = Rng::new(0x0127_0133);
        let bad_tails: &[&[u8]] = &[b"", b".", b"e", b"e+", b"e-", b"E", b"..", b"e1e1", b"-"];
        for &f in &[0, JSON_DECODE_INT_AS_REAL, JSON_DECODE_ANY] {
            for i in 0..250 {
                let mut d = Vec::new();
                d.push(b'[');
                gen_number(&mut rng, &mut d);
                if rng.below(3) == 0 {
                    d.extend_from_slice(rng.choice(bad_tails));
                }
                d.push(b']');
                cmp_loads(c, r, &d, f, &format!("rows 127-133 random #{i}"));

                // Also a leading-zero mutation.
                let mut z = Vec::new();
                z.push(b'[');
                if rng.bool() {
                    z.push(b'-');
                }
                z.push(b'0');
                for _ in 0..(1 + rng.below(3)) {
                    z.push(b'0' + rng.below(10) as u8);
                }
                z.push(b']');
                cmp_loads(c, r, &z, f, &format!("row 127 random #{i}"));
            }
        }
    }
}

// ===========================================================================
// Rows 134-144 — strings, escapes, surrogates, raw UTF-8
// ===========================================================================

#[test]
fn rows_134_144_strings_and_escapes() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // row 134
        let s = cmp_loads(c, r, b"\"\"", JSON_DECODE_ANY, "row 134 empty string");
        assert_eq!(s.dump_str.as_deref(), Some("\"\""));
        let s = cmp_loads(c, r, b"{\"k\":\"\"}", JSON_DECODE_ANY, "row 134 as value");
        assert_eq!(s.dump_str.as_deref(), Some("{\"k\": \"\"}"));
        // json_string_length must be 0 on both.
        let buf = cs_bytes(b"\"\"");
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (c.json_loads)(buf.as_ptr(), JSON_DECODE_ANY, &mut ce);
        let rj = (r.json_loads)(buf.as_ptr(), JSON_DECODE_ANY, &mut re);
        diff_eq!(
            (c.json_string_length)(cj),
            (r.json_string_length)(rj),
            "json_string_length of \"\""
        );
        assert_eq!((c.json_string_length)(cj), 0);
        decref(c, cj);
        decref(r, rj);

        // row 135
        let long: Vec<u8> = {
            let mut v = vec![b'"'];
            v.extend(std::iter::repeat(b'x').take(1000));
            v.push(b'"');
            v
        };
        cmp_loads(c, r, b"\"a\"", JSON_DECODE_ANY, "row 135 len 1");
        let s = cmp_loads(c, r, &long, JSON_DECODE_ANY, "row 135 len 1000");
        assert_eq!(s.dump.as_ref().unwrap().len(), 1002);

        // row 136: every short escape.
        let s = cmp_loads(
            c,
            r,
            b"[\"\\\" \\\\ \\/ \\b \\f \\n \\r \\t\"]",
            0,
            "row 136 short escapes",
        );
        assert!(!s.null);
        // Confirm the decoded bytes exactly.
        let buf = cs_bytes(b"[\"\\\" \\\\ \\/ \\b \\f \\n \\r \\t\"]");
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (c.json_loads)(buf.as_ptr(), 0, &mut ce);
        let rj = (r.json_loads)(buf.as_ptr(), 0, &mut re);
        let cv = (c.json_array_get)(cj, 0);
        let rv = (r.json_array_get)(rj, 0);
        let clen = (c.json_string_length)(cv);
        let cptr = (c.json_string_value)(cv) as *const u8;
        let cbuf: Vec<u8> = (0..clen).map(|i| *cptr.add(i)).collect();
        let rlen = (r.json_string_length)(rv);
        let rptr = (r.json_string_value)(rv) as *const u8;
        let rbuf: Vec<u8> = (0..rlen).map(|i| *rptr.add(i)).collect();
        diff_eq!(cbuf.clone(), rbuf.clone(), "decoded short-escape bytes");
        assert_eq!(
            cbuf,
            b"\" \\ / \x08 \x0c \n \r \t".to_vec(),
            "C: decoded escape bytes"
        );
        decref(c, cj);
        decref(r, rj);

        // row 137: invalid escapes.
        let mut bad: Vec<Vec<u8>> = vec![
            b"[\"\\x\"]".to_vec(),
            b"[\"\\a\"]".to_vec(),
            b"[\"\\ \"]".to_vec(),
            b"[\"\\\"]".to_vec(),
            b"[\"\\U0041\"]".to_vec(),
            b"[\"\\0\"]".to_vec(),
        ];
        bad.push(b"[\"\\\n\"]".to_vec());
        for t in &bad {
            let s = cmp_loads(c, r, t, 0, "row 137 invalid escape");
            assert!(s.null, "C: {:?} must fail", show(t));
        }

        // row 138: \uXXXX in each utf8_encode width, and case-insensitivity.
        let cases: &[(&[u8], &[u8])] = &[
            (b"[\"\\u0041\"]", b"A"),
            (b"[\"\\u00e9\"]", "\u{e9}".as_bytes()),
            (b"[\"\\u20ac\"]", "\u{20ac}".as_bytes()),
            (b"[\"\\u007f\"]", b"\x7f"),
            (b"[\"\\u0080\"]", "\u{80}".as_bytes()),
            (b"[\"\\u07ff\"]", "\u{7ff}".as_bytes()),
            (b"[\"\\u0800\"]", "\u{800}".as_bytes()),
            (b"[\"\\uffff\"]", "\u{ffff}".as_bytes()),
        ];
        for (t, want) in cases {
            let s = cmp_loads(c, r, t, 0, "row 138 \\uXXXX");
            assert!(!s.null, "C: {:?} must parse", show(t));
            let buf = cs_bytes(t);
            let mut e = json_error_t::new();
            let cj = (c.json_loads)(buf.as_ptr(), 0, &mut e);
            let cv = (c.json_array_get)(cj, 0);
            let l = (c.json_string_length)(cv);
            let p = (c.json_string_value)(cv) as *const u8;
            let got: Vec<u8> = (0..l).map(|i| *p.add(i)).collect();
            assert_eq!(got, want.to_vec(), "C: decoded bytes of {:?}", show(t));
            decref(c, cj);
        }
        let a = cmp_loads(c, r, b"[\"\\u00E9\"]", 0, "row 138 upper hex");
        let b = cmp_loads(c, r, b"[\"\\u00e9\"]", 0, "row 138 lower hex");
        assert_eq!(a.dump, b.dump, "C: hex case must not matter");

        // row 139: valid surrogate pairs.
        let pairs: &[(&[u8], &str)] = &[
            (b"[\"\\ud834\\udd1e\"]", "\u{1d11e}"),
            (b"[\"\\ud800\\udc00\"]", "\u{10000}"),
            (b"[\"\\udbff\\udfff\"]", "\u{10ffff}"),
            (b"[\"\\uD834\\uDD1E\"]", "\u{1d11e}"),
        ];
        for (t, want) in pairs {
            let s = cmp_loads(c, r, t, 0, "row 139 surrogate pair");
            assert!(!s.null, "C: {:?} must parse", show(t));
            let buf = cs_bytes(t);
            let mut e = json_error_t::new();
            let cj = (c.json_loads)(buf.as_ptr(), 0, &mut e);
            let cv = (c.json_array_get)(cj, 0);
            let l = (c.json_string_length)(cv);
            let p = (c.json_string_value)(cv) as *const u8;
            let got: Vec<u8> = (0..l).map(|i| *p.add(i)).collect();
            assert_eq!(got, want.as_bytes().to_vec(), "C: 4-byte UTF-8 output");
            assert_eq!(l, 4);
            decref(c, cj);
        }

        // row 140: broken surrogates.
        for t in [
            &b"[\"\\ud834\"]"[..],
            b"[\"\\ud834abc\"]",
            b"[\"\\ud834\\u0041\"]",
            b"[\"\\udd1e\"]",
            b"[\"\\udc00\"]",
            b"[\"\\udfff\"]",
            b"[\"\\ud800\\ud800\"]",
            b"[\"\\ud834\\\\\"]",
        ] {
            let s = cmp_loads(c, r, t, 0, "row 140 broken surrogate");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert_eq!(s.code(), JSON_ERROR_INVALID_SYNTAX);
        }

        // row 141: malformed \u escapes.
        for t in [
            &b"[\"\\u\"]"[..],
            b"[\"\\u12\"]",
            b"[\"\\u123\"]",
            b"[\"\\u12g4\"]",
            b"[\"\\uZZZZ\"]",
            b"[\"\\u 123\"]",
            b"[\"\\u123\\u1234\"]",
        ] {
            let s = cmp_loads(c, r, t, 0, "row 141 malformed \\u");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert_eq!(s.code(), JSON_ERROR_INVALID_SYNTAX);
        }

        // row 142: raw multi-byte UTF-8 passthrough.
        let raws: &[&str] = &[
            "[\"\u{e9}\"]",
            "[\"\u{20ac}\"]",
            "[\"\u{1d11e}\"]",
            "[\"a\u{e9}b\u{20ac}c\u{1d11e}d\"]",
            "[\"\u{80}\u{7ff}\u{800}\u{ffff}\u{10000}\u{10ffff}\"]",
        ];
        for t in raws {
            let s = cmp_loads(c, r, t.as_bytes(), 0, "row 142 raw UTF-8");
            assert!(!s.null, "C: {:?} must parse", show(t.as_bytes()));
        }

        // row 143: invalid UTF-8.
        for t in [
            &b"[\"\x80\"]"[..],
            b"[\"\xc0\x41\"]",
            b"[\"\xe2\x82\"]",
            b"[\"\xc0\x80\"]",
            b"[\"\xff\"]",
            b"[\"\xed\xa0\x80\"]",
            b"[\"\xf8\x88\x80\x80\x80\"]",
            b"[\"\xc1\xbf\"]",
            b"[\"\xf4\x90\x80\x80\"]",
        ] {
            let s = cmp_loads(c, r, t, 0, "row 143 invalid UTF-8");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert_eq!(
                s.code(),
                JSON_ERROR_INVALID_UTF8,
                "C: {:?} gave {:?}",
                show(t),
                s.text()
            );
            assert!(
                s.text().starts_with("unable to decode byte 0x"),
                "C: UTF-8 error message: {:?}",
                s.text()
            );
        }

        // row 143, second half: the *no-context* message path. When the bad
        // byte is the very first thing lex_scan reads, `saved_text` is still
        // empty and `stream.state == STREAM_STATE_ERROR`, so error_set() takes
        // the `result = msg_text` branch and appends no " near ..." context.
        for t in [&b"\x80"[..], b"\xff", b"\xc0\x41", b"\xe2\x82"] {
            let s = cmp_loads(c, r, t, JSON_DECODE_ANY, "row 143 no-context path");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert_eq!(s.code(), JSON_ERROR_INVALID_UTF8);
            assert!(
                !s.text().contains(" near "),
                "C: a UTF-8 error with empty saved_text carries no context: {:?}",
                s.text()
            );
        }

        // row 144: raw multi-byte UTF-8 as a stray token outside a string.
        for t in [
            "[\u{e9}]".as_bytes(),
            "[\u{20ac}]".as_bytes(),
            "[\u{1d11e}]".as_bytes(),
            "\u{e9}".as_bytes(),
            "{\u{20ac}:1}".as_bytes(),
        ] {
            let s = cmp_loads(c, r, t, 0, "row 144 stray UTF-8 token");
            assert!(s.null, "C: {:?} must fail", show(t));
        }
        let s = cmp_loads(c, r, "[\u{e9}]".as_bytes(), 0, "row 144 message");
        assert!(
            s.text().contains("invalid token") && s.text().contains('\u{e9}'),
            "C: lex_save_cached must flush the whole sequence: {:?}",
            s.text()
        );

        // Randomised strings: every escape form, surrogate pairs, raw UTF-8.
        let mut rng = Rng::new(0x0134_0144);
        for &f in &[0, JSON_DECODE_ANY, JSON_DECODE_ANY | JSON_ALLOW_NUL] {
            for i in 0..250 {
                let mut d = Vec::new();
                d.push(b'[');
                let n = 1 + rng.below(4);
                for k in 0..n {
                    if k > 0 {
                        d.push(b',');
                    }
                    gen_string(&mut rng, &mut d, f & JSON_ALLOW_NUL != 0);
                }
                d.push(b']');
                cmp_loads(c, r, &d, f, &format!("rows 134-142 random #{i}"));
            }
        }
        // Randomised invalid UTF-8 / broken escapes.
        for i in 0..300 {
            let mut d = Vec::new();
            d.extend_from_slice(b"[\"");
            let n = 1 + rng.below(6);
            for _ in 0..n {
                match rng.below(6) {
                    0 => d.push(0x80 + rng.below(0x80) as u8),
                    1 => d.extend_from_slice(&[0xc0 + rng.below(0x20) as u8]),
                    2 => d.extend_from_slice(&[0xe0 + rng.below(0x10) as u8, 0x80]),
                    3 => d.extend_from_slice(&[0xf0 + rng.below(0x10) as u8, 0x80, 0x80]),
                    4 => {
                        d.push(b'\\');
                        d.push(0x20 + rng.below(0x5f) as u8);
                    }
                    _ => {
                        d.extend_from_slice(b"\\u");
                        for _ in 0..rng.below(4) {
                            d.push(*rng.choice(b"0123456789abcdefABCDEFgzZ \\\""));
                        }
                    }
                }
            }
            d.extend_from_slice(b"\"]");
            cmp_loads(c, r, &d, 0, &format!("rows 143 random #{i}"));
        }
    }
}

// ===========================================================================
// Rows 145-153 — NUL characters and JSON_ALLOW_NUL
// ===========================================================================

/// The raw bytes of a parsed string value, plus its length.
unsafe fn string_bytes(api: &Api, j: *mut json_t) -> (size_t, Vec<u8>) {
    let l = (api.json_string_length)(j);
    let p = (api.json_string_value)(j) as *const u8;
    ((l), (0..l).map(|i| *p.add(i)).collect())
}

#[test]
fn rows_145_153_nul_handling() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // row 145
        let s = cmp_loads(c, r, b"[\"\\u0000\"]", 0, "row 145");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_NULL_CHARACTER);
        assert!(
            s.text().contains("\\u0000 is not allowed without JSON_ALLOW_NUL"),
            "C: got {:?}",
            s.text()
        );

        // row 146
        let s = cmp_loads(c, r, b"[\"\\u0000\"]", JSON_ALLOW_NUL, "row 146");
        assert!(!s.null, "C: ALLOW_NUL must accept an embedded NUL");
        let buf = cs_bytes(b"[\"\\u0000\"]");
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (c.json_loads)(buf.as_ptr(), JSON_ALLOW_NUL, &mut ce);
        let rj = (r.json_loads)(buf.as_ptr(), JSON_ALLOW_NUL, &mut re);
        let cv = (c.json_array_get)(cj, 0);
        let rv = (r.json_array_get)(rj, 0);
        diff_eq!(string_bytes(c, cv), string_bytes(r, rv), "row 146 NUL string");
        assert_eq!(string_bytes(c, cv), (1usize, vec![0u8]));
        // strlen == 0 while length == 1
        assert_eq!(
            cbytes((c.json_string_value)(cv)).map(|v| v.len()),
            Some(0),
            "C: strlen(value) == 0"
        );
        decref(c, cj);
        decref(r, rj);

        // row 147: NUL in leading / trailing / multiple interior positions.
        let cases: &[(&[u8], size_t, &[u8])] = &[
            (b"[\"\\u0000abc\"]", 4, b"\0abc"),
            (b"[\"abc\\u0000\"]", 4, b"abc\0"),
            (b"[\"a\\u0000b\\u0000c\"]", 5, b"a\0b\0c"),
        ];
        for (t, wantlen, wantbytes) in cases {
            let s = cmp_loads(c, r, t, JSON_ALLOW_NUL, "row 147");
            assert!(!s.null, "C: {:?} must parse with ALLOW_NUL", show(t));
            let buf = cs_bytes(t);
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let cj = (c.json_loads)(buf.as_ptr(), JSON_ALLOW_NUL, &mut ce);
            let rj = (r.json_loads)(buf.as_ptr(), JSON_ALLOW_NUL, &mut re);
            let cv = (c.json_array_get)(cj, 0);
            let rv = (r.json_array_get)(rj, 0);
            diff_eq!(string_bytes(c, cv), string_bytes(r, rv), "row 147 {:?}", show(t));
            assert_eq!(string_bytes(c, cv), (*wantlen, wantbytes.to_vec()));
            decref(c, cj);
            decref(r, rj);
            // Without the flag it must fail.
            let s = cmp_loads(c, r, t, 0, "row 147 no flag");
            assert!(s.null);
            assert_eq!(s.code(), JSON_ERROR_NULL_CHARACTER);
        }

        // rows 148, 149: NUL in an object key, with and without ALLOW_NUL.
        let key_docs: &[&[u8]] = &[
            b"{\"a\\u0000b\":1}",
            b"{\"\\u0000\":1}",
            b"{\"k\\u0000\":1}",
        ];
        for t in key_docs {
            let with = cmp_loads(c, r, t, JSON_ALLOW_NUL, "row 148 key NUL, ALLOW_NUL");
            let without = cmp_loads(c, r, t, 0, "row 149 key NUL, flags=0");
            assert!(with.null && without.null, "C: {:?} must fail both ways", show(t));
            assert_eq!(with.code(), JSON_ERROR_NULL_BYTE_IN_KEY);
            assert_eq!(
                without.code(),
                JSON_ERROR_NULL_BYTE_IN_KEY,
                "C: the key check is flag-independent"
            );
            assert_eq!(with.code(), without.code());
        }

        // row 150: NUL only in a value.
        let s = cmp_loads(c, r, b"{\"k\":\"a\\u0000b\"}", JSON_ALLOW_NUL, "row 150");
        assert!(!s.null);
        let buf = cs_bytes(b"{\"k\":\"a\\u0000b\"}");
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (c.json_loads)(buf.as_ptr(), JSON_ALLOW_NUL, &mut ce);
        let rj = (r.json_loads)(buf.as_ptr(), JSON_ALLOW_NUL, &mut re);
        let ck = cs("k");
        let cv = (c.json_object_get)(cj, ck.as_ptr());
        let rv = (r.json_object_get)(rj, ck.as_ptr());
        diff_eq!(string_bytes(c, cv), string_bytes(r, rv), "row 150 value");
        assert_eq!(string_bytes(c, cv).0, 3, "C: value length 3");
        decref(c, cj);
        decref(r, rj);

        // row 151: NUL-in-key beats duplicate-key.
        let f = JSON_ALLOW_NUL | JSON_REJECT_DUPLICATES;
        let s = cmp_loads(c, r, b"{\"a\\u0000b\":1,\"a\\u0000b\":2}", f, "row 151");
        assert!(s.null);
        assert_eq!(
            s.code(),
            JSON_ERROR_NULL_BYTE_IN_KEY,
            "C: NUL check precedes the duplicate check, got {:?}",
            s.text()
        );

        // row 152
        let s = cmp_loads(c, r, b"{\"a\":1,\"a\\u0000\":2}", f, "row 152 second key NUL");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_NULL_BYTE_IN_KEY);
        let s = cmp_loads(c, r, b"{\"a\\u0000\":1,\"a\":2}", f, "row 152 first key NUL");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_NULL_BYTE_IN_KEY);
        let s = cmp_loads(c, r, b"{\"a\":1,\"a\":2}", f, "row 152 clean dup");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_DUPLICATE_KEY);

        // row 153: bare NUL string top level.
        let s = cmp_loads(
            c,
            r,
            b"\"\\u0000\"",
            JSON_ALLOW_NUL | JSON_DECODE_ANY,
            "row 153 with ALLOW_NUL",
        );
        assert!(!s.null);
        let buf = cs_bytes(b"\"\\u0000\"");
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let cj = (c.json_loads)(buf.as_ptr(), JSON_ALLOW_NUL | JSON_DECODE_ANY, &mut ce);
        let rj = (r.json_loads)(buf.as_ptr(), JSON_ALLOW_NUL | JSON_DECODE_ANY, &mut re);
        diff_eq!(string_bytes(c, cj), string_bytes(r, rj), "row 153 top-level");
        assert_eq!(string_bytes(c, cj), (1usize, vec![0u8]));
        decref(c, cj);
        decref(r, rj);
        let s = cmp_loads(c, r, b"\"\\u0000\"", JSON_DECODE_ANY, "row 153 without");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_NULL_CHARACTER);

        // Randomised: NUL escapes sprinkled through keys and values.
        let mut rng = Rng::new(0x0145_0153);
        for &f in &[
            0,
            JSON_ALLOW_NUL,
            JSON_ALLOW_NUL | JSON_REJECT_DUPLICATES,
            JSON_ALLOW_NUL | JSON_DECODE_ANY,
            JSON_DECODE_ANY,
        ] {
            for i in 0..250 {
                let mut d = Vec::new();
                match rng.below(3) {
                    0 => {
                        // object with a possibly-NUL key
                        d.push(b'{');
                        d.push(b'"');
                        for _ in 0..(1 + rng.below(3)) {
                            if rng.below(3) == 0 {
                                d.extend_from_slice(b"\\u0000");
                            } else {
                                d.push(b'a' + rng.below(26) as u8);
                            }
                        }
                        d.extend_from_slice(b"\":");
                        gen_string(&mut rng, &mut d, true);
                        d.push(b'}');
                    }
                    1 => {
                        d.push(b'[');
                        gen_string(&mut rng, &mut d, true);
                        d.push(b']');
                    }
                    _ => gen_string(&mut rng, &mut d, true),
                }
                cmp_loads(c, r, &d, f, &format!("rows 145-153 random #{i}"));
            }
        }
    }
}

// ===========================================================================
// Rows 154-157 — nesting depth
// ===========================================================================

/// The deep-nesting cases recurse `JSON_PARSER_MAX_DEPTH` frames inside both
/// libraries, so they run on a thread with a generous stack.
fn on_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .expect("spawn deep-nesting thread")
        .join()
        .expect("deep-nesting thread panicked");
}

fn nest_array(n: usize, inner: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(2 * n + inner.len());
    for _ in 0..n {
        v.push(b'[');
    }
    v.extend_from_slice(inner);
    for _ in 0..n {
        v.push(b']');
    }
    v
}

fn nest_object(n: usize, inner: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    for _ in 0..n {
        v.extend_from_slice(b"{\"a\":");
    }
    v.extend_from_slice(inner);
    for _ in 0..n {
        v.push(b'}');
    }
    v
}

fn nest_mixed(n: usize, inner: &[u8]) -> Vec<u8> {
    let mut open = Vec::new();
    let mut close = Vec::new();
    for i in 0..n {
        if i % 2 == 0 {
            open.push(b'[');
            close.insert(0, b']');
        } else {
            open.extend_from_slice(b"{\"a\":");
            close.insert(0, b'}');
        }
    }
    let mut v = open;
    v.extend_from_slice(inner);
    v.extend_from_slice(&close);
    v
}

#[test]
fn rows_154_157_nesting_depth() {
    let _g = global_state_lock();
    on_big_stack(|| {
        let (c, r) = both();
        unsafe {
            // row 154: shallow depths all succeed.
            let s = cmp_loads(c, r, b"[1]", 0, "row 154 depth 1");
            assert!(!s.null);
            let s = cmp_loads(c, r, b"[[[[[1]]]]]", 0, "row 154 depth 5");
            assert!(!s.null);
            for n in [1usize, 2, 3, 5, 10, 50, 100] {
                for (name, doc) in [
                    ("array", nest_array(n, b"")),
                    ("object", nest_object(n, b"1")),
                    ("mixed", nest_mixed(n, b"1")),
                ] {
                    let s = cmp_loads(c, r, &doc, 0, &format!("row 154 {name} depth {n}"));
                    assert!(!s.null, "C: depth {n} {name} must parse");
                }
            }

            // row 155: exactly JSON_PARSER_MAX_DEPTH.
            let doc = nest_array(JSON_PARSER_MAX_DEPTH, b"");
            let s = cmp_loads(c, r, &doc, 0, "row 155 exactly MAX_DEPTH");
            assert!(
                !s.null,
                "C: {} nested arrays must parse (depth == MAX)",
                JSON_PARSER_MAX_DEPTH
            );
            let want: String = "[".repeat(JSON_PARSER_MAX_DEPTH)
                + &"]".repeat(JSON_PARSER_MAX_DEPTH);
            assert_eq!(
                s.dump_str.as_deref(),
                Some(want.as_str()),
                "C: innermost value must be an empty array"
            );

            // row 156: one past the cap, for all three nesting shapes.
            for (name, doc) in [
                ("array", nest_array(JSON_PARSER_MAX_DEPTH + 1, b"")),
                ("object", nest_object(JSON_PARSER_MAX_DEPTH + 1, b"1")),
                ("mixed", nest_mixed(JSON_PARSER_MAX_DEPTH + 1, b"1")),
            ] {
                let s = cmp_loads(c, r, &doc, 0, &format!("row 156 {name} MAX+1"));
                assert!(s.null, "C: {name} depth MAX+1 must fail");
                assert_eq!(s.code(), JSON_ERROR_STACK_OVERFLOW);
                assert!(
                    s.text().contains("maximum parsing depth reached"),
                    "C: got {:?}",
                    s.text()
                );
            }
            // Also the object/mixed exactly-at-cap variants must succeed.
            for (name, doc) in [
                ("object", nest_object(JSON_PARSER_MAX_DEPTH, b"1")),
                ("mixed", nest_mixed(JSON_PARSER_MAX_DEPTH, b"1")),
            ] {
                // depth(inner scalar) = MAX + 1 -> must FAIL.
                let s = cmp_loads(c, r, &doc, 0, &format!("row 157 {name} MAX brackets + scalar"));
                assert!(
                    s.null,
                    "C: {name}: MAX containers plus a scalar is depth MAX+1"
                );
                assert_eq!(s.code(), JSON_ERROR_STACK_OVERFLOW);
            }

            // row 157: the off-by-one on a bare scalar innermost value.
            let doc = nest_array(JSON_PARSER_MAX_DEPTH, b"1");
            let s = cmp_loads(c, r, &doc, JSON_DECODE_ANY, "row 157 MAX brackets + scalar");
            assert!(s.null, "C: MAX brackets + scalar == depth MAX+1 must fail");
            assert_eq!(s.code(), JSON_ERROR_STACK_OVERFLOW);

            let doc = nest_array(JSON_PARSER_MAX_DEPTH - 1, b"1");
            let s = cmp_loads(
                c,
                r,
                &doc,
                JSON_DECODE_ANY,
                "row 157 MAX-1 brackets + scalar",
            );
            assert!(!s.null, "C: MAX-1 brackets + scalar == depth MAX must parse");

            // A bare scalar with DECODE_ANY is depth 1.
            let s = cmp_loads(c, r, b"42", JSON_DECODE_ANY, "row 157 bare scalar depth 1");
            assert!(!s.null);

            // Randomised depths straddling the cap.
            let mut rng = Rng::new(0x0154_0157);
            for i in 0..220 {
                let n = match rng.below(4) {
                    0 => 1 + rng.below(20),
                    1 => JSON_PARSER_MAX_DEPTH - 3 + rng.below(7),
                    2 => JSON_PARSER_MAX_DEPTH - 1 + rng.below(3),
                    _ => 1 + rng.below(JSON_PARSER_MAX_DEPTH + 4),
                };
                let inner: &[u8] = if rng.bool() { b"" } else { b"1" };
                let doc = match rng.below(3) {
                    0 => nest_array(n, inner),
                    1 => nest_object(n, if inner.is_empty() { b"1" } else { inner }),
                    _ => nest_mixed(n, if inner.is_empty() { b"1" } else { inner }),
                };
                let f = if rng.bool() { 0 } else { JSON_DECODE_ANY };
                cmp_loads(c, r, &doc, f, &format!("rows 154-157 random #{i} depth {n}"));
            }
        }
    });
}

// ===========================================================================
// Rows 158-163 — parser and lexer error branches
// ===========================================================================

#[test]
fn rows_158_163_error_branches() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // row 158: parse_object error branches with their exact messages.
        let cases: &[(&[u8], &str)] = &[
            (b"{1:2}", "string or '}' expected"),
            (b"{,}", "string or '}' expected"),
            (b"{true:1}", "string or '}' expected"),
            (b"{\"a\" 1}", "':' expected"),
            (b"{\"a\",1}", "':' expected"),
            (b"{\"a\":1,}", "string or '}' expected"),
            (b"{\"a\":1 \"b\":2}", "'}' expected"),
        ];
        for (t, msg) in cases {
            let s = cmp_loads(c, r, t, 0, "row 158 parse_object");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert!(
                s.text().contains(msg),
                "C: {:?} expected message {msg:?}, got {:?}",
                show(t),
                s.text()
            );
        }
        // `{"a":1` hits "'}' expected" and is remapped to premature end.
        let s = cmp_loads(c, r, b"{\"a\":1", 0, "row 158 EOF in object");
        assert!(s.null);
        assert_eq!(
            s.code(),
            JSON_ERROR_PREMATURE_END_OF_INPUT,
            "C: remapped, got {:?}",
            s.text()
        );
        assert!(
            s.text().contains("'}' expected") && s.text().contains("near end of file"),
            "C: got {:?}",
            s.text()
        );

        // row 159: parse_array error branches.
        for (t, msg) in [
            (&b"[1 2]"[..], "']' expected"),
            // After the comma the ']' is handed to parse_value, which falls
            // through to `default:` — so this is "unexpected token", not
            // "']' expected".
            (b"[1,]", "unexpected token"),
            (b"[1 2 3]", "']' expected"),
            (b"[1,2,]", "unexpected token"),
        ] {
            let s = cmp_loads(c, r, t, 0, "row 159 parse_array");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert!(
                s.text().contains(msg),
                "C: {:?} expected {msg:?}, got {:?}",
                show(t),
                s.text()
            );
        }
        let s = cmp_loads(c, r, b"[1,2", 0, "row 159 EOF in array");
        assert!(s.null);
        assert_eq!(s.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
        assert!(s.text().contains("']' expected"), "C: got {:?}", s.text());
        let s = cmp_loads(c, r, b"[,1]", 0, "row 159 leading comma");
        assert!(s.null);
        assert!(
            s.text().contains("unexpected token") || s.text().contains("invalid token"),
            "C: got {:?}",
            s.text()
        );

        // row 160: identifier-like tokens.
        for t in [&b"[true]"[..], b"[false]", b"[null]"] {
            let s = cmp_loads(c, r, t, 0, "row 160 valid keyword");
            assert!(!s.null, "C: {:?} must parse", show(t));
        }
        for (t, ident) in [
            (&b"[True]"[..], "True"),
            (b"[TRUE]", "TRUE"),
            (b"[nulll]", "nulll"),
            (b"[nul]", "nul"),
            (b"[tru]", "tru"),
            (b"[undefined]", "undefined"),
            (b"[NaN]", "NaN"),
            (b"[Infinity]", "Infinity"),
            (b"[falsey]", "falsey"),
        ] {
            let s = cmp_loads(c, r, t, 0, "row 160 invalid identifier");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert!(
                s.text().contains("invalid token") && s.text().contains(ident),
                "C: error text must include the whole identifier {ident:?}, got {:?}",
                s.text()
            );
        }

        // row 161: single-character punctuation documents.
        for t in [&b"{"[..], b"["] {
            let s = cmp_loads(c, r, t, 0, "row 161 lone opener");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert_eq!(
                s.code(),
                JSON_ERROR_PREMATURE_END_OF_INPUT,
                "C: {:?} gave {:?}",
                show(t),
                s.text()
            );
        }
        for t in [&b"}"[..], b"]", b":", b","] {
            let s = cmp_loads(c, r, t, 0, "row 161 lone closer/separator");
            assert!(s.null, "C: {:?} must fail", show(t));
            assert_eq!(s.code(), JSON_ERROR_INVALID_SYNTAX);
            assert!(
                s.text().contains("'[' or '{' expected"),
                "C: {:?} gave {:?}",
                show(t),
                s.text()
            );
        }

        // row 162: unterminated strings.
        for t in [&b"[\"abc"[..], b"[\"", b"\"", b"{\"a", b"[\"abc\\"] {
            let s = cmp_loads(c, r, t, 0, "row 162 unterminated string");
            assert!(s.null, "C: {:?} must fail", show(t));
        }
        let s = cmp_loads(c, r, b"[\"abc", 0, "row 162 message");
        assert_eq!(s.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
        assert!(
            s.text().contains("premature end of input"),
            "C: got {:?}",
            s.text()
        );

        // row 163: raw control characters inside a string. json_loadb is used so
        // a real 0x00 byte can be injected.
        for b in 0u8..=0x1f {
            let doc = [b'[', b'"', b'a', b, b'"', b']'];
            let s = cmp_loadb(c, r, &doc, doc.len(), 0, "row 163 raw control char");
            assert!(s.null, "C: raw control char {b:#04x} must fail");
            assert_eq!(s.code(), JSON_ERROR_INVALID_SYNTAX);
            if b == 0x0a {
                assert!(
                    s.text().contains("unexpected newline"),
                    "C: 0x0a message {:?}",
                    s.text()
                );
            } else if b == 0 {
                // 0x00 is only reachable through json_loadb; string_get would
                // treat it as EOF.
                assert!(
                    s.text().contains("control character 0x0"),
                    "C: 0x00 message {:?}",
                    s.text()
                );
            } else {
                assert!(
                    s.text().contains(&format!("control character 0x{b:x}")),
                    "C: {b:#04x} message {:?}",
                    s.text()
                );
            }
        }

        // Randomised malformed documents across every flag combination.
        let mut rng = Rng::new(0x0158_0163);
        for &f in FLAG_SETS {
            for i in 0..250 {
                let d = gen_malformed(&mut rng);
                cmp_loads(c, r, &d, f, &format!("rows 158-163 random #{i}"));
            }
        }
    }
}

// ===========================================================================
// Randomised valid documents, every flag combination
// ===========================================================================

#[test]
fn random_valid_documents_every_flag_combination() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_1001);
    unsafe {
        for &f in FLAG_SETS {
            for i in 0..250 {
                let allow_nul = rng.below(6) == 0;
                let d = gen_doc(&mut rng, allow_nul);
                cmp_loads(c, r, &d, f, &format!("valid random flags={f:#x} #{i}"));
            }
        }
    }
}

#[test]
fn random_malformed_documents_every_flag_combination() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_1002);
    unsafe {
        for &f in FLAG_SETS {
            for i in 0..250 {
                let d = gen_malformed(&mut rng);
                cmp_loads(c, r, &d, f, &format!("malformed random flags={f:#x} #{i}"));
            }
        }
    }
}

// ===========================================================================
// All decoding entry points
// ===========================================================================

#[test]
fn json_loadb_buflen_variants() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_2001);
    unsafe {
        // Fixed cases first: 0, shorter, exact, and "exactly the string".
        for t in [&b"[1,2,3]"[..], b"{\"a\":1}", b"[]", b"{}", b"\"x\"", b"12"] {
            cmp_loadb(c, r, t, 0, 0, "buflen 0");
            cmp_loadb(c, r, t, 0, JSON_DECODE_ANY, "buflen 0, DECODE_ANY");
            cmp_loadb(c, r, t, t.len(), 0, "buflen exact");
            cmp_loadb(c, r, t, t.len(), JSON_DECODE_ANY, "buflen exact, DECODE_ANY");
            for cut in 1..t.len() {
                cmp_loadb(c, r, t, cut, 0, "buflen short");
                cmp_loadb(c, r, t, cut, JSON_DISABLE_EOF_CHECK, "buflen short, no EOF check");
            }
        }
        // Embedded NUL: json_loadb must see past it (unlike json_loads).
        let with_nul = b"[1,\x002]";
        cmp_loadb(c, r, with_nul, with_nul.len(), 0, "embedded NUL in buffer");
        cmp_loads(c, r, with_nul, 0, "embedded NUL via json_loads (stops at NUL)");

        // Randomised: valid and malformed docs at every buflen, every flag set.
        for &f in FLAG_SETS {
            for i in 0..250 {
                let d = if rng.bool() {
                    { let an = rng.below(6) == 0; gen_doc(&mut rng, an) }
                } else {
                    gen_malformed(&mut rng)
                };
                let n = d.len();
                let buflen = match rng.below(5) {
                    0 => 0,
                    1 => n,
                    2 => {
                        if n == 0 {
                            0
                        } else {
                            rng.below(n)
                        }
                    }
                    3 => n / 2,
                    _ => n.saturating_sub(1),
                };
                cmp_loadb(c, r, &d, buflen, f, &format!("loadb random #{i}"));
            }
        }
    }
}

#[test]
fn json_loadf_and_loadfd_and_load_file() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_2002);
    let pf = tmp_path("loadf");
    let pd = tmp_path("loadfd");
    let pl = tmp_path("loadfile");
    unsafe {
        for t in [
            &b"[1,2,3]"[..],
            b"{\"a\":1,\"b\":[2,3]}",
            b"",
            b"[",
            b"[1] junk",
            b"[\"\\ud834\\udd1e\"]",
            b"[\"\xff\"]",
            b"12345",
        ] {
            for f in [0, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK] {
                cmp_loadf(c, r, t, f, &pf, "loadf fixed");
                cmp_loadfd(c, r, t, f, &pd, "loadfd fixed");
                cmp_load_file(c, r, t, f, &pl, "load_file fixed");
            }
        }

        // A file that does not exist -> json_error_cannot_open_file, including
        // the strerror() text.
        let missing = tmp_path("definitely_missing_subdir/nope");
        let cpath = cs(missing.to_str().unwrap());
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        let cj = (c.json_load_file)(cpath.as_ptr(), 0, &mut ce);
        let rj = (r.json_load_file)(cpath.as_ptr(), 0, &mut re);
        let cs_ = snap(c, cj, &ce);
        let rs_ = snap(r, rj, &re);
        cs_.compare(&rs_, "json_load_file on a missing path");
        assert!(cs_.null);
        assert_eq!(cs_.code(), JSON_ERROR_CANNOT_OPEN_FILE);
        decref(c, cj);
        decref(r, rj);

        // NULL arguments.
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        let cj = (c.json_load_file)(std::ptr::null(), 0, &mut ce);
        let rj = (r.json_load_file)(std::ptr::null(), 0, &mut re);
        snap(c, cj, &ce).compare(&snap(r, rj, &re), "json_load_file(NULL)");
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        let cj = (c.json_loads)(std::ptr::null(), 0, &mut ce);
        let rj = (r.json_loads)(std::ptr::null(), 0, &mut re);
        snap(c, cj, &ce).compare(&snap(r, rj, &re), "json_loads(NULL)");
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        let cj = (c.json_loadb)(std::ptr::null(), 5, 0, &mut ce);
        let rj = (r.json_loadb)(std::ptr::null(), 5, 0, &mut re);
        snap(c, cj, &ce).compare(&snap(r, rj, &re), "json_loadb(NULL)");
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        let cj = (c.json_loadf)(std::ptr::null_mut(), 0, &mut ce);
        let rj = (r.json_loadf)(std::ptr::null_mut(), 0, &mut re);
        snap(c, cj, &ce).compare(&snap(r, rj, &re), "json_loadf(NULL)");
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        let cj = (c.json_loadfd)(-1, 0, &mut ce);
        let rj = (r.json_loadfd)(-1, 0, &mut re);
        snap(c, cj, &ce).compare(&snap(r, rj, &re), "json_loadfd(-1)");
        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        let cj = (c.json_load_callback)(None, std::ptr::null_mut(), 0, &mut ce);
        let rj = (r.json_load_callback)(None, std::ptr::null_mut(), 0, &mut re);
        snap(c, cj, &ce).compare(&snap(r, rj, &re), "json_load_callback(NULL)");

        // Randomised documents through all three file-backed entry points.
        for &f in FLAG_SETS {
            for i in 0..70 {
                let d = if rng.bool() {
                    { let an = rng.below(6) == 0; gen_doc(&mut rng, an) }
                } else {
                    gen_malformed(&mut rng)
                };
                cmp_loadf(c, r, &d, f, &pf, &format!("loadf random #{i}"));
                cmp_loadfd(c, r, &d, f, &pd, &format!("loadfd random #{i}"));
                cmp_load_file(c, r, &d, f, &pl, &format!("load_file random #{i}"));
            }
        }
    }
    let _ = std::fs::remove_file(&pf);
    let _ = std::fs::remove_file(&pd);
    let _ = std::fs::remove_file(&pl);
}

#[test]
fn json_load_callback_all_feeding_strategies() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_2003);
    unsafe {
        // A document longer than MAX_BUF_LEN (1024) so callback_get has to
        // refill more than once even in "big chunk" mode.
        let long_doc: Vec<u8> = {
            let mut v = vec![b'['];
            for i in 0..600 {
                if i > 0 {
                    v.push(b',');
                }
                v.extend_from_slice(format!("{i}").as_bytes());
            }
            v.push(b']');
            v
        };
        let fixed: Vec<Vec<u8>> = vec![
            b"[1,2,3]".to_vec(),
            b"{\"a\":1}".to_vec(),
            b"".to_vec(),
            b"[".to_vec(),
            b"[1] junk".to_vec(),
            b"\"str\"".to_vec(),
            b"[\"\\ud834\\udd1e\"]".to_vec(),
            long_doc.clone(),
        ];
        for t in &fixed {
            for f in [0, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK] {
                // returns 0 immediately -> looks like empty input
                cmp_load_callback(c, r, t, CB_ZERO, 0, f, "cb returns 0");
                // returns (size_t)-1 -> also EOF
                cmp_load_callback(c, r, t, CB_ERROR, 0, f, "cb returns -1");
                // one byte per call
                cmp_load_callback(c, r, t, CB_ONE_BYTE, 0, f, "cb 1 byte");
                // as much as fits
                cmp_load_callback(c, r, t, CB_BIG, 0, f, "cb big chunks");
                // stop short partway through
                for frac in [1usize, 2, 3] {
                    let stop = t.len() * frac / 4;
                    cmp_load_callback(c, r, t, CB_SHORT, stop, f, "cb stops short");
                }
                cmp_load_callback(c, r, t, CB_SHORT, t.len(), f, "cb stops at end");
            }
        }

        // Randomised across every flag set and every feeding strategy.
        for &f in FLAG_SETS {
            for i in 0..250 {
                let d = if rng.bool() {
                    { let an = rng.below(6) == 0; gen_doc(&mut rng, an) }
                } else {
                    gen_malformed(&mut rng)
                };
                let mode = *rng.choice(&[CB_ZERO, CB_ONE_BYTE, CB_BIG, CB_SHORT, CB_ERROR]);
                let stop = if d.is_empty() { 0 } else { rng.below(d.len() + 1) };
                cmp_load_callback(c, r, &d, mode, stop, f, &format!("cb random #{i}"));
            }
        }
    }
}

// ===========================================================================
// Round trips: load -> dump -> load, stable in and across both libraries
// ===========================================================================

#[test]
fn round_trip_load_dump_load_is_stable() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_3001);
    unsafe {
        let reload = JSON_DECODE_ANY | JSON_ALLOW_NUL;
        for &f in &[
            0,
            JSON_DECODE_ANY,
            JSON_DECODE_ANY | JSON_ALLOW_NUL,
            JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
            JSON_REJECT_DUPLICATES | JSON_DECODE_ANY,
        ] {
            for i in 0..250 {
                let allow_nul = rng.below(5) == 0;
                let d = gen_doc(&mut rng, allow_nul);
                let ctx = format!("round trip flags={f:#x} #{i}");

                // Pass 1: parse and dump on both.
                let buf = cs_bytes(&d);
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let cj = (c.json_loads)(buf.as_ptr(), f, &mut ce);
                let rj = (r.json_loads)(buf.as_ptr(), f, &mut re);
                let c1 = snap(c, cj, &ce);
                let r1 = snap(r, rj, &re);
                c1.compare(&r1, &format!("{ctx}: pass 1 on {:?}", show(&d)));
                if c1.null {
                    decref(c, cj);
                    decref(r, rj);
                    continue;
                }
                let dumped = c1.dump.clone().unwrap();

                // Pass 2: reload the canonical dump on both and dump again.
                let buf2 = cs_bytes(&dumped);
                let mut ce2 = json_error_t::poisoned();
                let mut re2 = json_error_t::poisoned();
                let cj2 = (c.json_loads)(buf2.as_ptr(), f | reload, &mut ce2);
                let rj2 = (r.json_loads)(buf2.as_ptr(), f | reload, &mut re2);
                let c2 = snap(c, cj2, &ce2);
                let r2 = snap(r, rj2, &re2);
                c2.compare(&r2, &format!("{ctx}: pass 2 on {:?}", show(&dumped)));
                assert!(
                    !c2.null,
                    "C: canonical dump {:?} must reparse (from {:?})",
                    show(&dumped),
                    show(&d)
                );
                assert_eq!(
                    c2.dump.as_ref().unwrap(),
                    &dumped,
                    "C: dump is not a fixed point for {:?}",
                    show(&d)
                );

                // Pass 3: json_equal across the two reload results, and against
                // the originals, inside each library.
                diff_eq!(
                    (c.json_equal)(cj, cj2),
                    (r.json_equal)(rj, rj2),
                    "{ctx}: json_equal(original, reloaded) on {:?}",
                    show(&d)
                );

                decref(c, cj);
                decref(r, rj);
                decref(c, cj2);
                decref(r, rj2);
            }
        }
    }
}

/// Round trips through the file/buffer/callback entry points too: the value a
/// document produces must not depend on which entry point read it.
#[test]
fn same_document_through_every_entry_point() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_3002);
    let pf = tmp_path("entrypoints_f");
    let pd = tmp_path("entrypoints_fd");
    let pl = tmp_path("entrypoints_file");
    unsafe {
        for i in 0..220 {
            let d = if rng.below(4) == 0 {
                gen_malformed(&mut rng)
            } else {
                { let an = rng.below(6) == 0; gen_doc(&mut rng, an) }
            };
            // Documents containing a NUL byte read differently through
            // json_loads (which stops at the NUL), so skip the cross-entry
            // comparison for those but still run each entry point differentially.
            let has_nul = d.contains(&0);
            let f = *rng.choice(FLAG_SETS);
            let ctx = format!("entry-point sweep #{i}");

            let via_loads = cmp_loads(c, r, &d, f, &ctx);
            let via_loadb = cmp_loadb(c, r, &d, d.len(), f, &ctx);
            cmp_loadf(c, r, &d, f, &pf, &ctx);
            cmp_loadfd(c, r, &d, f, &pd, &ctx);
            cmp_load_file(c, r, &d, f, &pl, &ctx);
            cmp_load_callback(c, r, &d, CB_ONE_BYTE, 0, f, &ctx);
            cmp_load_callback(c, r, &d, CB_BIG, 0, f, &ctx);

            if !has_nul {
                // json_loads and json_loadb(exact length) must agree on the
                // value (the error `source` field differs by design, so compare
                // the dump only).
                assert_eq!(
                    via_loads.dump, via_loadb.dump,
                    "C: json_loads and json_loadb disagree on {:?}",
                    show(&d)
                );
            }
        }
    }
    let _ = std::fs::remove_file(&pf);
    let _ = std::fs::remove_file(&pd);
    let _ = std::fs::remove_file(&pl);
}

// ===========================================================================
// Extra stress on the error-reporting corners of load.c
// ===========================================================================

/// `error_set()` only appends `" near '<token>'"` when
/// `lex->saved_text.length <= 20`; longer tokens get the bare message. And a
/// `source` of 80 bytes or more is rewritten as `"..." + tail`. Both gates are
/// easy to get wrong in a translation, and both are visible in the raw
/// `json_error_t` image.
#[test]
fn error_context_gate_and_long_tokens() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_4001);
    unsafe {
        // Identifiers of every length from 1 to 40: at length 20 the context is
        // still appended, at 21 it is dropped.
        for n in 1..=40usize {
            let ident: String = std::iter::repeat('q').take(n).collect();
            let doc = format!("[{ident}]");
            let s = cmp_loads(c, r, doc.as_bytes(), 0, &format!("identifier len {n}"));
            assert!(s.null, "C: {ident} is not a keyword");
            if n <= 20 {
                assert!(
                    s.text().contains(&format!("near '{ident}'")),
                    "C: length {n} must keep the context, got {:?}",
                    s.text()
                );
            } else {
                assert!(
                    !s.text().contains(" near "),
                    "C: length {n} must drop the context, got {:?}",
                    s.text()
                );
            }
        }
        // Same gate through a bad number token and a bad string token.
        for n in 1..=30usize {
            let digits: String = std::iter::repeat('7').take(n).collect();
            for doc in [
                format!("[{digits}.]"),
                format!("[{digits}e]"),
                format!("[0{digits}]"),
                format!("[\"{digits}\\q\"]"),
            ] {
                cmp_loads(c, r, doc.as_bytes(), 0, &format!("token len {n}"));
                cmp_loads(
                    c,
                    r,
                    doc.as_bytes(),
                    JSON_DECODE_INT_AS_REAL,
                    &format!("token len {n} INT_AS_REAL"),
                );
            }
        }

        // Very long tokens: strbuffer has to grow many times, and the overflow
        // messages must still be byte-identical.
        for n in [100usize, 1000, 5000, 40_000] {
            let s: String = std::iter::repeat('x').take(n).collect();
            cmp_loads(c, r, format!("[\"{s}\"]").as_bytes(), 0, "long string");
            let d: String = std::iter::repeat('9').take(n).collect();
            cmp_loads(c, r, format!("[{d}]").as_bytes(), 0, "long integer");
            cmp_loads(
                c,
                r,
                format!("[{d}]").as_bytes(),
                JSON_DECODE_INT_AS_REAL,
                "long integer as real",
            );
            cmp_loads(c, r, format!("[1.{d}]").as_bytes(), 0, "long fraction");
            let a: String = std::iter::repeat('z').take(n).collect();
            cmp_loads(c, r, format!("[{a}]").as_bytes(), 0, "long identifier");
        }

        // Randomised long/short tokens, valid and broken, under every flag set.
        for &f in FLAG_SETS {
            for i in 0..200 {
                let n = rng.below(60);
                let kind = rng.below(5);
                let body: String = match kind {
                    0 => std::iter::repeat('a').take(n).collect(),
                    1 => std::iter::repeat('5').take(n).collect(),
                    2 => format!("\"{}\"", "u".repeat(n)),
                    3 => format!("{}.{}", "1".repeat(n / 2 + 1), "2".repeat(n / 2)),
                    _ => format!("1e{}", "9".repeat(n / 2 + 1)),
                };
                let doc = if rng.bool() {
                    format!("[{body}]")
                } else {
                    format!("{{\"k\":{body}}}")
                };
                cmp_loads(c, r, doc.as_bytes(), f, &format!("long-token random #{i}"));
            }
        }
    }
}

#[test]
fn json_load_file_source_field_truncation() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        // Paths of many lengths straddling JSON_ERROR_SOURCE_LENGTH (80), so
        // both branches of jsonp_error_set_source run and the resulting
        // `error.source` bytes are compared exactly.
        // Kept under the 255-byte filename limit; TMPDIR already contributes
        // enough that both branches of jsonp_error_set_source are reached.
        for pad in [0usize, 1, 5, 20, 40, 60, 70, 78, 79, 80, 81, 120, 200] {
            let name = format!("a07_src_{}_{}.json", std::process::id(), "p".repeat(pad));
            let path = std::path::Path::new(&dir).join(&name);
            for body in [&b"[1,2]"[..], b"[", b""] {
                std::fs::write(&path, body).expect("write");
                let cpath = cs(path.to_str().unwrap());
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                let cj = (c.json_load_file)(cpath.as_ptr(), 0, &mut ce);
                let rj = (r.json_load_file)(cpath.as_ptr(), 0, &mut re);
                snap(c, cj, &ce).compare(
                    &snap(r, rj, &re),
                    &format!("json_load_file source len {} body {:?}", cpath.as_bytes().len(), show(body)),
                );
                decref(c, cj);
                decref(r, rj);
            }
            let _ = std::fs::remove_file(&path);

            // Also the "cannot open" path with the same long name, which formats
            // the path plus strerror() into a 160 byte buffer (so it truncates).
            let missing = std::path::Path::new(&dir).join(format!("missing_{name}/x"));
            let cpath = cs(missing.to_str().unwrap());
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let cj = (c.json_load_file)(cpath.as_ptr(), 0, &mut ce);
            let rj = (r.json_load_file)(cpath.as_ptr(), 0, &mut re);
            snap(c, cj, &ce).compare(
                &snap(r, rj, &re),
                &format!("json_load_file cannot-open, path len {}", cpath.as_bytes().len()),
            );
            decref(c, cj);
            decref(r, rj);
        }
    }
}

/// `stream_get` counts *codepoints* for `column` (it only increments when
/// `utf8_check_first` says the byte starts a sequence) and restores
/// `last_column` when a `'\n'` is ungotten. Errors placed after multi-byte
/// characters and after newlines therefore pin that bookkeeping.
#[test]
fn line_and_column_tracking_with_multibyte_and_newlines() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_4002);
    unsafe {
        let fixed: &[&str] = &[
            "[\n1,\n2,\nx]",
            "[\"\u{20ac}\u{20ac}\u{20ac}\",\nq]",
            "{\n\"\u{1d11e}\": 1,\n\"b\" 2\n}",
            "[1\n]",
            "[1\n,2\n]",
            "[\n\n\n\n1 2]",
            "\n\n\n[",
            "[\"a\u{e9}b\",\n\"c\u{20ac}d\",\n1.]",
            "[\u{10ffff}]",
            "[1,\r\n2,\r\nzzz]",
            "{\"a\":\n\n1,\n\n\"a\"\n\n:2}",
        ];
        for t in fixed {
            for f in [0, JSON_DECODE_ANY, JSON_REJECT_DUPLICATES] {
                cmp_loads(c, r, t.as_bytes(), f, "line/column fixed");
            }
        }

        // Randomised: a valid document with newlines injected, then a byte
        // corrupted so the error lands at a random line/column.
        for &f in FLAG_SETS {
            for i in 0..200 {
                let mut d: Vec<u8> = Vec::new();
                d.push(b'[');
                let n = 1 + rng.below(6);
                for k in 0..n {
                    if k > 0 {
                        d.push(b',');
                    }
                    for _ in 0..(1 + rng.below(3)) {
                        d.push(*rng.choice(&[b'\n', b'\r', b' ', b'\t']));
                    }
                    if rng.bool() {
                        // a string with multi-byte characters, so `column`
                        // counts codepoints rather than bytes
                        d.push(b'"');
                        for _ in 0..rng.below(5) {
                            let cp = *rng.choice(&[
                                0xe9u32, 0x20ac, 0x1d11e, 0x7f, 0x80, 0x7ff, 0x800, 0x10ffff,
                            ]);
                            push_utf8(&mut d, cp);
                        }
                        d.push(b'"');
                    } else {
                        gen_number(&mut rng, &mut d);
                    }
                }
                for _ in 0..rng.below(3) {
                    d.push(b'\n');
                }
                d.push(b']');
                cmp_loads(c, r, &d, f, &format!("line/column random #{i} clean"));
                // now corrupt one byte
                let mut bad = d.clone();
                let k = rng.below(bad.len());
                bad[k] = *rng.choice(GARBAGE);
                cmp_loads(c, r, &bad, f, &format!("line/column random #{i} corrupt"));
            }
        }
    }
}

/// Truncation exactly inside a multi-byte UTF-8 sequence: `stream_get` calls
/// `get()` `count - 1` more times, each returning EOF, stores those `-1`s as
/// `0xFF` bytes and then fails `utf8_check_full`. The reported byte in the
/// message is the *first* byte of the sequence, not the one that failed.
#[test]
fn eof_inside_multibyte_sequence() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0007_4003);
    unsafe {
        let seqs: &[&[u8]] = &[
            b"\xc3",
            b"\xc3\xa9",
            b"\xe2",
            b"\xe2\x82",
            b"\xe2\x82\xac",
            b"\xf0",
            b"\xf0\x9f",
            b"\xf0\x9f\x98",
            b"\xf0\x9f\x98\x80",
        ];
        for s in seqs {
            for prefix in [&b""[..], b"[", b"[\"", b"[\"a", b"{\"", b"[1,\""] {
                let mut d = prefix.to_vec();
                d.extend_from_slice(s);
                for f in [0, JSON_DECODE_ANY] {
                    cmp_loads(c, r, &d, f, "EOF inside multi-byte (loads)");
                    cmp_loadb(c, r, &d, d.len(), f, "EOF inside multi-byte (loadb)");
                    // buflen cutting the sequence in every position
                    for cut in 0..=d.len() {
                        cmp_loadb(c, r, &d, cut, f, "loadb cutting multi-byte");
                    }
                    cmp_load_callback(c, r, &d, CB_ONE_BYTE, 0, f, "cb 1 byte, multi-byte cut");
                    cmp_load_callback(c, r, &d, CB_BIG, 0, f, "cb big, multi-byte cut");
                }
            }
        }

        // Randomised: a document containing multi-byte characters, truncated at
        // every possible offset.
        for i in 0..200 {
            let mut d = Vec::new();
            d.extend_from_slice(b"[\"");
            for _ in 0..(1 + rng.below(6)) {
                let cp = *rng.choice(&[0xe9u32, 0x20ac, 0x1d11e, 0x7ff, 0x800, 0x10000]);
                push_utf8(&mut d, cp);
            }
            d.extend_from_slice(b"\"]");
            for cut in 0..=d.len() {
                let t = &d[..cut];
                cmp_loads(c, r, t, 0, &format!("truncated multi-byte #{i} at {cut}"));
                cmp_loadb(c, r, &d, cut, 0, &format!("loadb multi-byte #{i} at {cut}"));
            }
        }
    }
}
