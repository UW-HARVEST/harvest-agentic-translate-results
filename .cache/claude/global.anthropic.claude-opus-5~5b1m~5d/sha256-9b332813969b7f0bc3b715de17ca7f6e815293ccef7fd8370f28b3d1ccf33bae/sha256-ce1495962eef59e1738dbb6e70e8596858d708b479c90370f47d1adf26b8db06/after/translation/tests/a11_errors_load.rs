//! Phase C — error-path differential tests for `src/load.c`.
//!
//! Covers ERRORS.md rows **145-197, 339, 340, 341, 343, 344, 345, 347, 348 and
//! 355**: every rejection, every "wrong arguments" guard, every lexer/parser
//! error message, the numeric-overflow and UTF-8 rejection paths, the
//! out-of-memory paths that are only reachable with a budgeted allocator, and
//! the "unknown flag bits" surface.
//!
//! Three observables are compared for every case:
//!
//!   a) whether the returned `json_t*` is NULL,
//!   b) the **complete byte image** of the caller's `json_error_t` (`.raw()`),
//!      starting from `json_error_t::poisoned()` so that "the library did not
//!      write here" is distinguishable from "it wrote a NUL". That single
//!      comparison pins `line`, `column`, `position`, `source`, `text` and the
//!      error-code byte at `text[159]` simultaneously,
//!   c) the numeric `json_error_code()` value, asserted against the exact
//!      number the ERRORS.md row documents — so the test proves "the same
//!      error", not merely "both failed".
//!
//! In addition, the generic FFI boundary of every `json_load*` entry point is
//! swept even where ERRORS.md has no row: NULL input pointer, NULL *error*
//! pointer (which must be tolerated), zero length, `(size_t)-1` length, a bad
//! file descriptor, a path that is a directory, and flag words with undefined
//! bits set. A C `size_t flags` parameter accepts any value at all, so
//! `0x20`, `0x8000`, `SIZE_MAX` and random 64-bit words are real inputs that
//! both implementations must fold identically.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fclose(f: *mut FILE) -> c_int;
    fn malloc(n: size_t) -> *mut c_void;
    fn realloc(p: *mut c_void, n: size_t) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// The five decoder flag bits jansson actually looks at
/// (`JSON_REJECT_DUPLICATES | JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY |
/// JSON_DECODE_INT_AS_REAL | JSON_ALLOW_NUL`). Every other bit of the `size_t`
/// is dead on the decoding side. Note that `JSON_INDENT(n)` is `n & 0x1F`, so
/// an *encoder* indent value ALIASES these bits rather than being ignored —
/// which is itself behaviour both libraries must reproduce.
const DECODE_FLAG_MASK: size_t = 0x1F;

// ---------------------------------------------------------------------------
// Observation of a single decode call
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct Obs {
    null: bool,
    snap: (c_int, c_int, c_int, String, String, c_int),
    raw: Vec<u8>,
}

impl Obs {
    fn code(&self) -> c_int {
        self.snap.5
    }
    fn text(&self) -> &str {
        &self.snap.4
    }
    fn source(&self) -> &str {
        &self.snap.3
    }
    fn line(&self) -> c_int {
        self.snap.0
    }
    fn column(&self) -> c_int {
        self.snap.1
    }
    fn position(&self) -> c_int {
        self.snap.2
    }
}

fn obs(j: *const json_t, err: &json_error_t) -> Obs {
    Obs {
        null: j.is_null(),
        snap: err.snapshot(),
        raw: err.raw(),
    }
}

/// Compare the C observation against the Rust one. `snap` first (readable),
/// then the full raw image (strongest).
fn cmp(co: &Obs, ro: &Obs, ctx: &str) {
    diff_eq!(co.null, ro.null, "returned-NULL differs — {ctx}");
    diff_eq!(
        co.snap.clone(),
        ro.snap.clone(),
        "json_error_t (line,col,pos,source,text,code) differs — {ctx}"
    );
    diff_eq!(
        co.raw.clone(),
        ro.raw.clone(),
        "json_error_t raw byte image differs — {ctx}"
    );
}

/// Printable, unambiguous rendering of arbitrary input bytes.
fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &x in b.iter().take(120) {
        match x {
            b'\\' => s.push_str("\\\\"),
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(x as char),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    if b.len() > 120 {
        s.push_str(&format!("...<{} bytes total>", b.len()));
    }
    s
}

// ---------------------------------------------------------------------------
// One differential wrapper per entry point
// ---------------------------------------------------------------------------

unsafe fn d_loads(c: &Api, r: &Api, text: &[u8], flags: size_t, ctx: &str) -> Obs {
    let buf = cs_bytes(text);
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();
    let cj = (c.json_loads)(buf.as_ptr(), flags, &mut ce);
    let rj = (r.json_loads)(buf.as_ptr(), flags, &mut re);
    let (co, ro) = (obs(cj, &ce), obs(rj, &re));
    cmp(
        &co,
        &ro,
        &format!(
            "json_loads(flags={flags:#x}) input={:?} [{ctx}]",
            show(text)
        ),
    );
    decref(c, cj);
    decref(r, rj);
    co
}

/// `json_loads` with a raw pointer (so NULL can be passed).
unsafe fn d_loads_ptr(
    c: &Api,
    r: &Api,
    p: *const c_char,
    flags: size_t,
    ctx: &str,
) -> Obs {
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();
    let cj = (c.json_loads)(p, flags, &mut ce);
    let rj = (r.json_loads)(p, flags, &mut re);
    let (co, ro) = (obs(cj, &ce), obs(rj, &re));
    cmp(&co, &ro, &format!("json_loads(ptr={p:?}, flags={flags:#x}) [{ctx}]"));
    decref(c, cj);
    decref(r, rj);
    co
}

unsafe fn d_loadb(
    c: &Api,
    r: &Api,
    p: *const c_char,
    buflen: size_t,
    flags: size_t,
    ctx: &str,
) -> Obs {
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();
    let cj = (c.json_loadb)(p, buflen, flags, &mut ce);
    let rj = (r.json_loadb)(p, buflen, flags, &mut re);
    let (co, ro) = (obs(cj, &ce), obs(rj, &re));
    cmp(
        &co,
        &ro,
        &format!("json_loadb(buflen={buflen:#x}, flags={flags:#x}) [{ctx}]"),
    );
    decref(c, cj);
    decref(r, rj);
    co
}

fn tmp_path(tag: &str) -> std::path::PathBuf {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    std::path::Path::new(&dir).join(format!("a11_load_{tag}_{}.json", std::process::id()))
}

/// `json_loadf` on the given bytes written to a temp file. A fresh `FILE*` is
/// opened for each library so both see the same stream position.
unsafe fn d_loadf_bytes(
    c: &Api,
    r: &Api,
    text: &[u8],
    flags: size_t,
    path: &std::path::Path,
    ctx: &str,
) -> Obs {
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

    let (co, ro) = (obs(cj, &ce), obs(rj, &re));
    cmp(
        &co,
        &ro,
        &format!("json_loadf(flags={flags:#x}) input={:?} [{ctx}]", show(text)),
    );
    decref(c, cj);
    decref(r, rj);
    co
}

unsafe fn d_loadfd_bytes(
    c: &Api,
    r: &Api,
    text: &[u8],
    flags: size_t,
    path: &std::path::Path,
    ctx: &str,
) -> Obs {
    std::fs::write(path, text).expect("write temp file");
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();

    let f1 = std::fs::File::open(path).expect("open fd");
    let cj = (c.json_loadfd)(f1.as_raw_fd(), flags, &mut ce);
    drop(f1);
    let f2 = std::fs::File::open(path).expect("open fd");
    let rj = (r.json_loadfd)(f2.as_raw_fd(), flags, &mut re);
    drop(f2);

    let (co, ro) = (obs(cj, &ce), obs(rj, &re));
    cmp(
        &co,
        &ro,
        &format!("json_loadfd(flags={flags:#x}) input={:?} [{ctx}]", show(text)),
    );
    decref(c, cj);
    decref(r, rj);
    co
}

/// `json_loadfd` on a bare descriptor number (so bad values can be passed).
unsafe fn d_loadfd_raw(c: &Api, r: &Api, fd: c_int, flags: size_t, ctx: &str) -> Obs {
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();
    let cj = (c.json_loadfd)(fd, flags, &mut ce);
    let rj = (r.json_loadfd)(fd, flags, &mut re);
    let (co, ro) = (obs(cj, &ce), obs(rj, &re));
    cmp(&co, &ro, &format!("json_loadfd(fd={fd}, flags={flags:#x}) [{ctx}]"));
    decref(c, cj);
    decref(r, rj);
    co
}

unsafe fn d_load_file_path(
    c: &Api,
    r: &Api,
    p: *const c_char,
    flags: size_t,
    ctx: &str,
) -> Obs {
    let mut ce = json_error_t::poisoned();
    let mut re = json_error_t::poisoned();
    let cj = (c.json_load_file)(p, flags, &mut ce);
    let rj = (r.json_load_file)(p, flags, &mut re);
    let (co, ro) = (obs(cj, &ce), obs(rj, &re));
    cmp(&co, &ro, &format!("json_load_file(flags={flags:#x}) [{ctx}]"));
    decref(c, cj);
    decref(r, rj);
    co
}

unsafe fn d_load_file_bytes(
    c: &Api,
    r: &Api,
    text: &[u8],
    flags: size_t,
    path: &std::path::Path,
    ctx: &str,
) -> Obs {
    std::fs::write(path, text).expect("write temp file");
    let cpath = cs(path.to_str().unwrap());
    d_load_file_path(
        c,
        r,
        cpath.as_ptr(),
        flags,
        &format!("input={:?} [{ctx}]", show(text)),
    )
}

// ---- json_load_callback ----------------------------------------------------

const CB_ZERO: c_int = 0; // return 0 immediately  -> EOF
const CB_MINUS_ONE: c_int = 1; // return (size_t)-1 -> EOF
const CB_FEED: c_int = 2; // feed the whole buffer, then 0
const CB_TRUNCATE: c_int = 3; // feed `stop_after` bytes, then 0

#[repr(C)]
struct CbState {
    data: *const u8,
    len: usize,
    pos: usize,
    stop_after: usize,
    mode: c_int,
    calls: usize,
}

unsafe extern "C" fn cb(buf: *mut c_void, buflen: size_t, arg: *mut c_void) -> size_t {
    let st = arg as *mut CbState;
    (*st).calls += 1;
    match (*st).mode {
        CB_ZERO => 0,
        CB_MINUS_ONE => usize::MAX,
        CB_FEED => {
            let n = core::cmp::min(buflen, (*st).len - (*st).pos);
            if n == 0 {
                return 0;
            }
            core::ptr::copy_nonoverlapping((*st).data.add((*st).pos), buf as *mut u8, n);
            (*st).pos += n;
            n
        }
        CB_TRUNCATE => {
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

unsafe fn d_load_callback(
    c: &Api,
    r: &Api,
    f: json_load_callback_t,
    text: &[u8],
    mode: c_int,
    stop_after: usize,
    flags: size_t,
    ctx: &str,
) -> Obs {
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
    let cj = (c.json_load_callback)(f, &mut cst as *mut CbState as *mut c_void, flags, &mut ce);
    let rj = (r.json_load_callback)(f, &mut rst as *mut CbState as *mut c_void, flags, &mut re);
    let (co, ro) = (obs(cj, &ce), obs(rj, &re));
    let full = format!(
        "json_load_callback(mode={mode}, stop_after={stop_after}, flags={flags:#x}) \
         input={:?} [{ctx}]",
        show(text)
    );
    cmp(&co, &ro, &full);
    diff_eq!(cst.calls, rst.calls, "callback invocation count — {full}");
    diff_eq!(cst.pos, rst.pos, "callback bytes consumed — {full}");
    decref(c, cj);
    decref(r, rj);
    co
}

// ===========================================================================
// Rows 145-151 — the `wrong arguments` / `cannot open file` guards
// ===========================================================================

#[test]
fn rows_145_151_null_argument_guards() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Every guard runs BEFORE lex_init, so `flags` is irrelevant; sweep a
        // few flag words anyway, including ones with undefined bits set, to
        // prove the guard is not flag-dependent.
        for &flags in &[
            0usize,
            JSON_DECODE_ANY,
            JSON_REJECT_DUPLICATES | JSON_ALLOW_NUL,
            0x20,
            0x8000,
            usize::MAX,
        ] {
            // --- row 145: json_loads(NULL)
            let o = d_loads_ptr(c, r, std::ptr::null(), flags, "row 145");
            assert!(o.null, "C: json_loads(NULL) must return NULL");
            assert_eq!(o.code(), 4, "row 145: code must be json_error_invalid_argument");
            assert_eq!(o.code(), JSON_ERROR_INVALID_ARGUMENT);
            assert_eq!(o.text(), "wrong arguments", "row 145 text");
            assert_eq!(o.source(), "<string>", "row 145 source");
            assert_eq!(
                (o.line(), o.column(), o.position()),
                (-1, -1, 0),
                "row 145 line/column/position"
            );

            // --- row 146: json_loadb(NULL, ...) for several buflens
            for &buflen in &[0usize, 1, 3, usize::MAX] {
                let o = d_loadb(c, r, std::ptr::null(), buflen, flags, "row 146");
                assert!(o.null, "C: json_loadb(NULL) must return NULL");
                assert_eq!(o.code(), 4, "row 146 code");
                assert_eq!(o.text(), "wrong arguments", "row 146 text");
                assert_eq!(o.source(), "<buffer>", "row 146 source");
                assert_eq!((o.line(), o.column(), o.position()), (-1, -1, 0));
            }

            // --- row 147: json_loadf(NULL)
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let cj = (c.json_loadf)(std::ptr::null_mut(), flags, &mut ce);
            let rj = (r.json_loadf)(std::ptr::null_mut(), flags, &mut re);
            let (co, ro) = (obs(cj, &ce), obs(rj, &re));
            cmp(&co, &ro, &format!("json_loadf(NULL, flags={flags:#x}) row 147"));
            assert!(co.null, "C: json_loadf(NULL) must return NULL");
            assert_eq!(co.code(), 4, "row 147 code");
            assert_eq!(co.text(), "wrong arguments", "row 147 text");
            assert_eq!(co.source(), "<stream>", "row 147 source");
            decref(c, cj);
            decref(r, rj);

            // --- row 148: json_loadfd with a negative descriptor
            for fd in [-1, -2, -1000, c_int::MIN] {
                let o = d_loadfd_raw(c, r, fd, flags, "row 148");
                assert!(o.null, "C: json_loadfd({fd}) must return NULL");
                assert_eq!(o.code(), 4, "row 148 code for fd={fd}");
                assert_eq!(o.text(), "wrong arguments", "row 148 text");
                assert_eq!(o.source(), "<stream>", "row 148 source");
                assert_eq!((o.line(), o.column(), o.position()), (-1, -1, 0));
            }

            // --- row 149: json_load_file(NULL) — jsonp_error_init(error, NULL)
            // leaves source EMPTY, which is the interesting half of this row.
            let o = d_load_file_path(c, r, std::ptr::null(), flags, "row 149");
            assert!(o.null, "C: json_load_file(NULL) must return NULL");
            assert_eq!(o.code(), 4, "row 149 code");
            assert_eq!(o.text(), "wrong arguments", "row 149 text");
            assert_eq!(o.source(), "", "row 149: source must be the empty string");
            assert_eq!((o.line(), o.column(), o.position()), (-1, -1, 0));

            // --- row 151: json_load_callback(NULL)
            let o = d_load_callback(c, r, None, b"[1]", CB_FEED, 0, flags, "row 151");
            assert!(o.null, "C: json_load_callback(NULL) must return NULL");
            assert_eq!(o.code(), 4, "row 151 code");
            assert_eq!(o.text(), "wrong arguments", "row 151 text");
            assert_eq!(o.source(), "<callback>", "row 151 source");
            assert_eq!((o.line(), o.column(), o.position()), (-1, -1, 0));
        }
    }
}

#[test]
fn row_150_fopen_failure() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // ERRORS.md 150: code 3, text "unable to open <path>: <strerror(errno)>",
        // source = the path.
        let cases: &[(&str, &str)] = &[
            ("/nonexistent/x.json", "No such file or directory"),
            ("/nonexistent-directory-abcdef/y", "No such file or directory"),
            ("", "No such file or directory"),
            ("/proc/self/mem/not-a-dir", "Not a directory"),
        ];
        for &(path, errstr) in cases {
            let p = cs(path);
            let o = d_load_file_path(c, r, p.as_ptr(), 0, &format!("row 150 {path:?}"));
            assert!(o.null, "C: json_load_file({path:?}) must return NULL");
            assert_eq!(o.code(), 3, "row 150 code for {path:?}");
            assert_eq!(o.code(), JSON_ERROR_CANNOT_OPEN_FILE);
            assert_eq!(
                o.text(),
                format!("unable to open {path}: {errstr}"),
                "row 150 text for {path:?}"
            );
            assert_eq!(o.source(), path, "row 150 source must be the path");
            assert_eq!(
                (o.line(), o.column(), o.position()),
                (-1, -1, 0),
                "row 150 line/column/position"
            );
        }

        // A path long enough to be truncated by jsonp_error_set_source, so the
        // `source` field of a `cannot_open_file` error goes through the
        // "..."-prefix path as well.
        for len in [78usize, 79, 80, 81, 200] {
            let path = format!("/nonexistent-{}", "z".repeat(len));
            let p = cs(&path);
            let o = d_load_file_path(c, r, p.as_ptr(), 0, &format!("row 150 long path len={len}"));
            assert!(o.null);
            assert_eq!(o.code(), 3, "row 150 long-path code");
        }
    }
}

// ===========================================================================
// Row 152 — the load callback signalling EOF before a complete value
// ===========================================================================

#[test]
fn row_152_callback_signals_eof() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Both `0` and `(size_t)-1` are mapped to EOF by callback_get.
        for (mode, label) in [(CB_ZERO, "returns 0"), (CB_MINUS_ONE, "returns (size_t)-1")] {
            let o = d_load_callback(
                c,
                r,
                Some(cb),
                b"[1,2,3]",
                mode,
                0,
                0,
                &format!("row 152 callback {label}"),
            );
            assert!(o.null, "C: callback that {label} must fail");
            assert_eq!(o.code(), 6, "row 152 code ({label})");
            assert_eq!(o.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
            assert_eq!(
                o.text(),
                "'[' or '{' expected near end of file",
                "row 152 text ({label})"
            );
            assert_eq!(o.source(), "<callback>");
        }

        // Truncation partway through a value: EOF arrives mid-document.
        for stop in [1usize, 2, 3, 4, 5, 6] {
            let o = d_load_callback(
                c,
                r,
                Some(cb),
                b"[1,2,3]",
                CB_TRUNCATE,
                stop,
                0,
                &format!("row 152 truncated after {stop}"),
            );
            assert!(o.null, "C: truncation after {stop} bytes must fail");
            assert_eq!(o.code(), 6, "row 152 truncated code (stop={stop})");
        }
        // The same two EOF signals under every decoder flag word, so the row is
        // not proven only for flags == 0.
        for &flags in &[
            0usize,
            JSON_DECODE_ANY,
            JSON_DISABLE_EOF_CHECK,
            JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
            usize::MAX,
        ] {
            for mode in [CB_ZERO, CB_MINUS_ONE] {
                let o = d_load_callback(
                    c, r, Some(cb), b"[1]", mode, 0, flags, "row 152 flag sweep",
                );
                assert!(o.null, "C: an immediate EOF must fail (flags {flags:#x})");
                assert_eq!(o.code(), 6, "row 152 code (flags {flags:#x}, mode {mode})");
            }
        }
    }
}

// ===========================================================================
// Rows 153-156 — document-level structure rejections
// ===========================================================================

#[test]
fn rows_153_156_document_level() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- row 153: no JSON_DECODE_ANY and the first token is a scalar.
        let scalars: &[(&[u8], &str)] = &[
            (b"1", "'[' or '{' expected near '1'"),
            (b"\"x\"", "'[' or '{' expected near '\"x\"'"),
            (b"true", "'[' or '{' expected near 'true'"),
            (b"false", "'[' or '{' expected near 'false'"),
            (b"null", "'[' or '{' expected near 'null'"),
            (b"-1", "'[' or '{' expected near '-1'"),
            (b"1.5", "'[' or '{' expected near '1.5'"),
        ];
        for &(t, want) in scalars {
            let o = d_loads(c, r, t, 0, "row 153");
            assert!(o.null, "C: {:?} must be rejected without DECODE_ANY", show(t));
            assert_eq!(o.code(), 8, "row 153 code for {:?}", show(t));
            assert_eq!(o.code(), JSON_ERROR_INVALID_SYNTAX);
            assert_eq!(o.text(), want, "row 153 text for {:?}", show(t));
        }
        // The documented line/column/position for the single-character case.
        let o = d_loads(c, r, b"1", 0, "row 153 position");
        assert_eq!(
            (o.line(), o.column(), o.position()),
            (1, 1, 1),
            "row 153: line 1, column 1, position 1"
        );

        // --- row 154: empty input, no JSON_DECODE_ANY. invalid_syntax is
        // upgraded to premature_end_of_input because there is no saved text.
        let o = d_loads(c, r, b"", 0, "row 154");
        assert!(o.null);
        assert_eq!(o.code(), 6, "row 154 code");
        assert_eq!(o.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
        assert_eq!(o.text(), "'[' or '{' expected near end of file", "row 154 text");
        assert_eq!(o.position(), 0, "row 154 position");
        assert_eq!((o.line(), o.column()), (1, 0), "row 154 line/column");
        // Whitespace-only input takes the same branch (but advances position).
        for t in [&b" "[..], b"\t", b"\n", b"\r\n  \t"] {
            let o = d_loads(c, r, t, 0, "row 154 whitespace only");
            assert!(o.null);
            assert_eq!(o.code(), 6, "row 154 whitespace code for {:?}", show(t));
        }

        // --- row 155: empty input WITH JSON_DECODE_ANY reaches parse_value's
        // `default:` arm on TOKEN_EOF instead.
        let o = d_loads(c, r, b"", JSON_DECODE_ANY, "row 155");
        assert!(o.null);
        assert_eq!(o.code(), 6, "row 155 code");
        assert_eq!(o.text(), "unexpected token near end of file", "row 155 text");
        for t in [&b" "[..], b"\n\n", b"\t\r"] {
            let o = d_loads(c, r, t, JSON_DECODE_ANY, "row 155 whitespace only");
            assert!(o.null);
            assert_eq!(o.code(), 6);
            assert_eq!(o.text(), "unexpected token near end of file");
        }

        // --- row 156: trailing content with the EOF check enabled.
        let cases: &[(&[u8], &str)] = &[
            (b"[1] x", "end of file expected near 'x'"),
            (b"{} {}", "end of file expected near '{'"),
            (b"[1][2]", "end of file expected near '['"),
            (b"[1],", "end of file expected near ','"),
            (b"{}]", "end of file expected near ']'"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 156");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 7, "row 156 code for {:?}", show(t));
            assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED);
            assert_eq!(o.text(), want, "row 156 text for {:?}", show(t));
        }
        // With JSON_DISABLE_EOF_CHECK the very same inputs must SUCCEED, which
        // proves the row is about the EOF check and not about the trailing byte.
        for &(t, _) in cases {
            let o = d_loads(c, r, t, JSON_DISABLE_EOF_CHECK, "row 156 with DISABLE_EOF_CHECK");
            assert!(
                !o.null,
                "C: {:?} must parse with JSON_DISABLE_EOF_CHECK",
                show(t)
            );
        }
    }
}

// ===========================================================================
// Row 157 — nesting deeper than JSON_PARSER_MAX_DEPTH
// ===========================================================================

fn nest(open: u8, close: u8, n: usize) -> Vec<u8> {
    let mut v = vec![open; n];
    v.extend(std::iter::repeat(close).take(n));
    v
}

#[test]
fn row_157_maximum_parsing_depth() {
    let _g = global_state_lock();
    // MAX_DEPTH+1 recursive frames inside both libraries need a big stack.
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            let (c, r) = both();
            unsafe {
                // 2049 '[' characters — the exact trigger in the ERRORS.md row.
                let doc = nest(b'[', b']', JSON_PARSER_MAX_DEPTH + 1);
                let o = d_loads(c, r, &doc, 0, "row 157 2049 nested arrays");
                assert!(o.null, "C: depth MAX+1 must be rejected");
                assert_eq!(o.code(), 2, "row 157 code");
                assert_eq!(o.code(), JSON_ERROR_STACK_OVERFLOW);
                assert_eq!(
                    o.text(),
                    "maximum parsing depth reached near '['",
                    "row 157 text"
                );
                assert_eq!(
                    (o.column(), o.position()),
                    (
                        (JSON_PARSER_MAX_DEPTH + 1) as c_int,
                        (JSON_PARSER_MAX_DEPTH + 1) as c_int
                    ),
                    "row 157: column and position must both be 2049"
                );

                // Exactly at the cap must still be accepted — the boundary, so
                // an off-by-one in the Rust port is caught in both directions.
                let doc = nest(b'[', b']', JSON_PARSER_MAX_DEPTH);
                let o = d_loads(c, r, &doc, 0, "row 157 exactly MAX");
                assert!(!o.null, "C: depth MAX must be accepted");

                // Object nesting and a scalar innermost value reach the same
                // error via a different path through parse_value.
                let mut doc = Vec::new();
                for _ in 0..(JSON_PARSER_MAX_DEPTH + 1) {
                    doc.extend_from_slice(b"{\"a\":");
                }
                doc.extend_from_slice(b"1");
                for _ in 0..(JSON_PARSER_MAX_DEPTH + 1) {
                    doc.push(b'}');
                }
                let o = d_loads(c, r, &doc, 0, "row 157 objects MAX+1");
                assert!(o.null);
                assert_eq!(o.code(), 2, "row 157 object nesting code");

                // The same document through json_loadb and json_load_callback,
                // to prove the depth counter is per-parse and not per-stream.
                let doc = nest(b'[', b']', JSON_PARSER_MAX_DEPTH + 1);
                let o = d_loadb(
                    c,
                    r,
                    doc.as_ptr() as *const c_char,
                    doc.len(),
                    0,
                    "row 157 via json_loadb",
                );
                assert!(o.null);
                assert_eq!(o.code(), 2);
                let o = d_load_callback(
                    c,
                    r,
                    Some(cb),
                    &doc,
                    CB_FEED,
                    0,
                    0,
                    "row 157 via json_load_callback",
                );
                assert!(o.null);
                assert_eq!(o.code(), 2);
            }
        })
        .expect("spawn deep-nesting thread")
        .join()
        .expect("deep-nesting thread panicked");
}

// ===========================================================================
// Rows 158, 165-171, 173, 174 — parser rejections
// ===========================================================================

#[test]
fn row_158_nul_character_in_string_value() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let o = d_loads(c, r, b"[\"a\\u0000b\"]", 0, "row 158");
        assert!(o.null, "C: \\u0000 without JSON_ALLOW_NUL must be rejected");
        assert_eq!(o.code(), 11, "row 158 code");
        assert_eq!(o.code(), JSON_ERROR_NULL_CHARACTER);
        assert_eq!(
            o.text(),
            "\\u0000 is not allowed without JSON_ALLOW_NUL near '\"a\\u0000b\"'",
            "row 158 text"
        );
        // Every flag combination that does NOT include JSON_ALLOW_NUL must
        // reject; the one that does must accept.
        for extra in [
            0usize,
            JSON_REJECT_DUPLICATES,
            JSON_DISABLE_EOF_CHECK,
            JSON_DECODE_ANY,
            JSON_DECODE_INT_AS_REAL,
            0x20,
            0x8000,
        ] {
            let o = d_loads(c, r, b"[\"a\\u0000b\"]", extra, "row 158 flags");
            assert!(o.null, "C: must reject with flags {extra:#x}");
            assert_eq!(o.code(), 11, "row 158 code with flags {extra:#x}");
        }
        let o = d_loads(c, r, b"[\"a\\u0000b\"]", JSON_ALLOW_NUL, "row 158 with ALLOW_NUL");
        assert!(!o.null, "C: JSON_ALLOW_NUL must accept a NUL in a string value");
        // A bare "\u0000" at top level with DECODE_ANY takes the same branch.
        let o = d_loads(c, r, b"\"\\u0000\"", JSON_DECODE_ANY, "row 158 bare");
        assert!(o.null);
        assert_eq!(o.code(), 11);
    }
}

#[test]
fn rows_159_161_invalid_and_unexpected_tokens() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- row 159: unknown bareword (the whole identifier is eaten first).
        let cases: &[(&[u8], size_t, &str)] = &[
            (b"[tru]", 0, "invalid token near 'tru'"),
            (b"[fals]", 0, "invalid token near 'fals'"),
            (b"[nul]", 0, "invalid token near 'nul'"),
            (b"[TRUE]", 0, "invalid token near 'TRUE'"),
            (b"[Null]", 0, "invalid token near 'Null'"),
            (b"nul", JSON_DECODE_ANY, "invalid token near 'nul'"),
            (b"[truex]", 0, "invalid token near 'truex'"),
            (b"[abcdefghij]", 0, "invalid token near 'abcdefghij'"),
        ];
        for &(t, f, want) in cases {
            let o = d_loads(c, r, t, f, "row 159");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 159 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 159 text for {:?}", show(t));
        }

        // --- row 160: a byte that starts no token at all.
        for b in [b'@', b'#', b'*', b'(', b')', b'\'', b'+', b'|', b'~', b'%', b';'] {
            let doc = [b'[', b, b']'];
            let o = d_loads(c, r, &doc, 0, "row 160");
            assert!(o.null, "C: byte {b:#04x} must not start a token");
            assert_eq!(o.code(), 8, "row 160 code for byte {b:#04x}");
            assert_eq!(
                o.text(),
                format!("invalid token near '{}'", b as char),
                "row 160 text for byte {b:#04x}"
            );
        }

        // --- row 161: a structural token where a value is expected.
        let cases: &[(&[u8], &str)] = &[
            (b"[,]", "unexpected token near ','"),
            (b"[:]", "unexpected token near ':'"),
            (b"[1,2,]", "unexpected token near ']'"),
            (b"[}]", "unexpected token near '}'"),
            (b"[1,]", "unexpected token near ']'"),
            (b"{\"a\":}", "unexpected token near '}'"),
            (b"{\"a\":,}", "unexpected token near ','"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 161");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 161 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 161 text for {:?}", show(t));
        }
    }
}

#[test]
fn rows_165_171_object_rejections() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- row 165: member position is neither a string nor '}'.
        let cases: &[(&[u8], &str)] = &[
            (b"{1:2}", "string or '}' expected near '1'"),
            (b"{,}", "string or '}' expected near ','"),
            (b"{\"a\":1,}", "string or '}' expected near '}'"),
            (b"{true:1}", "string or '}' expected near 'true'"),
            (b"{[]:1}", "string or '}' expected near '['"),
            (b"{:1}", "string or '}' expected near ':'"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 165");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 165 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 165 text for {:?}", show(t));
        }

        // --- row 166: NUL byte inside an object KEY. Rejected even WITH
        // JSON_ALLOW_NUL — the key check in parse_object is unconditional.
        for extra in [0usize, JSON_ALLOW_NUL, JSON_ALLOW_NUL | JSON_REJECT_DUPLICATES, 0x8000] {
            let o = d_loads(c, r, b"{\"a\\u0000b\":1}", extra, "row 166");
            assert!(o.null, "C: NUL in key must be rejected (flags {extra:#x})");
            assert_eq!(o.code(), 13, "row 166 code (flags {extra:#x})");
            assert_eq!(o.code(), JSON_ERROR_NULL_BYTE_IN_KEY);
            assert_eq!(
                o.text(),
                "NUL byte in object key not supported near '\"a\\u0000b\"'",
                "row 166 text"
            );
        }

        // --- row 167: duplicate key with JSON_REJECT_DUPLICATES.
        let o = d_loads(
            c,
            r,
            b"{\"a\":1,\"a\":2}",
            JSON_REJECT_DUPLICATES,
            "row 167",
        );
        assert!(o.null, "C: duplicate key must be rejected");
        assert_eq!(o.code(), 14, "row 167 code");
        assert_eq!(o.code(), JSON_ERROR_DUPLICATE_KEY);
        assert_eq!(o.text(), "duplicate object key near '\"a\"'", "row 167 text");
        // Without the flag the very same document is accepted.
        let o = d_loads(c, r, b"{\"a\":1,\"a\":2}", 0, "row 167 without the flag");
        assert!(!o.null, "C: duplicates are accepted without the flag");

        // --- row 168: token after a key is not ':'.
        let cases: &[(&[u8], &str)] = &[
            (b"{\"a\" 1}", "':' expected near '1'"),
            (b"{\"a\",1}", "':' expected near ','"),
            (b"{\"a\"}", "':' expected near '}'"),
            (b"{\"a\"\"b\"}", "':' expected near '\"b\"'"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 168");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 168 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 168 text for {:?}", show(t));
        }

        // --- row 170: after a member, neither ',' nor '}'.
        let cases: &[(&[u8], &str)] = &[
            (b"{\"a\":1 \"b\":2}", "'}' expected near '\"b\"'"),
            (b"{\"a\":1 2}", "'}' expected near '2'"),
            (b"{\"a\":1]", "'}' expected near ']'"),
            (b"{\"a\":1:2}", "'}' expected near ':'"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 170");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 170 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 170 text for {:?}", show(t));
        }

        // --- row 171: object not terminated before EOF. invalid_syntax is
        // upgraded to premature_end_of_input because saved_text is empty.
        for t in [&b"{\"a\":1"[..], b"{\"a\":1 ", b"{\"a\":{\"b\":2}"] {
            let o = d_loads(c, r, t, 0, "row 171");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 6, "row 171 code for {:?}", show(t));
            assert_eq!(
                o.text(),
                "'}' expected near end of file",
                "row 171 text for {:?}",
                show(t)
            );
        }
        // `{` alone reaches the "string or '}' expected" branch at EOF instead.
        let o = d_loads(c, r, b"{", 0, "row 171 bare brace");
        assert!(o.null);
        assert_eq!(o.code(), 6);
        assert_eq!(o.text(), "string or '}' expected near end of file");
    }
}

#[test]
fn rows_173_174_array_rejections() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- row 173: after an element, neither ',' nor ']'.
        let cases: &[(&[u8], &str)] = &[
            (b"[1 2]", "']' expected near '2'"),
            (b"[1:2]", "']' expected near ':'"),
            (b"[1}", "']' expected near '}'"),
            (b"[[1] [2]]", "']' expected near '['"),
            (b"[\"a\" \"b\"]", "']' expected near '\"b\"'"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 173");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 173 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 173 text for {:?}", show(t));
        }

        // --- row 174: array not terminated before EOF; documented position 4.
        let o = d_loads(c, r, b"[1,2", 0, "row 174");
        assert!(o.null, "C: \"[1,2\" must be rejected");
        assert_eq!(o.code(), 6, "row 174 code");
        assert_eq!(o.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
        assert_eq!(o.text(), "']' expected near end of file", "row 174 text");
        assert_eq!(o.position(), 4, "row 174 position");
        for t in [&b"[1"[..], b"[1,2,3", b"[[1],[2]", b"[ "] {
            let o = d_loads(c, r, t, 0, "row 174 more");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 6, "row 174 code for {:?}", show(t));
        }
        // `[` alone: parse_array's `while (lex->token)` loop never runs because
        // lex_scan already returned TOKEN_EOF (0), so it lands on "']' expected".
        let o = d_loads(c, r, b"[", 0, "row 174 bare bracket");
        assert!(o.null);
        assert_eq!(o.code(), 6);
        assert_eq!(o.text(), "']' expected near end of file");
    }
}

// ===========================================================================
// Rows 176-184 — string-literal rejections
// ===========================================================================

#[test]
fn rows_176_184_string_literal_rejections() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- row 176: EOF inside a string literal.
        let o = d_loads(c, r, b"[\"abc", 0, "row 176");
        assert!(o.null);
        assert_eq!(o.code(), 6, "row 176 code");
        assert_eq!(o.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
        assert_eq!(
            o.text(),
            "premature end of input near '\"abc'",
            "row 176 text"
        );
        for t in [&b"[\""[..], b"[\"a", b"{\"key", b"[\"\\\"", b"[\"abcdefghij"] {
            let o = d_loads(c, r, t, 0, "row 176 more");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 6, "row 176 code for {:?}", show(t));
        }

        // --- row 177: a raw newline inside a string literal.
        let o = d_loads(c, r, b"[\"a\nb\"]", 0, "row 177");
        assert!(o.null);
        assert_eq!(o.code(), 8, "row 177 code");
        assert_eq!(o.text(), "unexpected newline near '\"a'", "row 177 text");
        let o = d_loads(c, r, b"[\"\n\"]", 0, "row 177 immediate newline");
        assert!(o.null);
        assert_eq!(o.code(), 8);
        assert_eq!(o.text(), "unexpected newline near '\"'");

        // --- row 178: every other raw control byte 0x01..0x1F. (0x00 ends the
        // NUL-terminated string handed to json_loads, so it is covered by
        // json_loadb below.)
        for b in 1u8..=0x1F {
            if b == b'\n' {
                continue;
            }
            let doc = [b'[', b'"', b'a', b, b'"', b']'];
            let o = d_loads(c, r, &doc, 0, "row 178");
            assert!(o.null, "C: control byte {b:#04x} must be rejected");
            assert_eq!(o.code(), 8, "row 178 code for {b:#04x}");
            assert_eq!(
                o.text(),
                format!("control character 0x{:x} near '\"a'", b),
                "row 178 text for byte {b:#04x}"
            );
        }
        // The documented TAB case, spelled out.
        let o = d_loads(c, r, b"[\"a\tb\"]", 0, "row 178 tab");
        assert_eq!(o.text(), "control character 0x9 near '\"a'", "row 178 tab text");
        // A raw NUL byte inside a string, reachable only through json_loadb.
        let doc = b"[\"a\0b\"]";
        let o = d_loadb(
            c,
            r,
            doc.as_ptr() as *const c_char,
            doc.len(),
            0,
            "row 178 raw NUL via json_loadb",
        );
        assert!(o.null);
        assert_eq!(o.code(), 8, "row 178 raw NUL code");
        assert_eq!(o.text(), "control character 0x0 near '\"a'", "row 178 raw NUL text");

        // --- row 179: `\u` not followed by four hex digits.
        let cases: &[(&[u8], &str)] = &[
            (b"[\"\\u12\"]", "invalid escape near '\"\\u12\"'"),
            (b"[\"\\uZZZZ\"]", "invalid escape near '\"\\uZ'"),
            (b"[\"\\u\"]", "invalid escape near '\"\\u\"'"),
            (b"[\"\\u123\"]", "invalid escape near '\"\\u123\"'"),
            (b"[\"\\u123g\"]", "invalid escape near '\"\\u123g'"),
            (b"[\"\\uD800\\uZZZZ\"]", "invalid escape near '\"\\uD800\\uZ'"),
            (b"[\"\\u 123\"]", "invalid escape near '\"\\u '"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 179");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 179 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 179 text for {:?}", show(t));
        }

        // --- row 180: backslash followed by an illegal escape character.
        for b in [b'x', b'a', b'0', b'U', b'\'', b'!', b'B', b'N'] {
            let doc = [b'[', b'"', b'\\', b, b'"', b']'];
            let o = d_loads(c, r, &doc, 0, "row 180");
            assert!(o.null, "C: escape \\{} must be rejected", b as char);
            assert_eq!(o.code(), 8, "row 180 code for \\{}", b as char);
            assert_eq!(
                o.text(),
                format!("invalid escape near '\"\\{}'", b as char),
                "row 180 text for \\{}",
                b as char
            );
        }

        // --- row 181: backslash at end of input. lex_get_save returns EOF, so
        // `c` is neither 'u' nor a legal escape and the else arm fires.
        let o = d_loads(c, r, b"[\"a\\", 0, "row 181");
        assert!(o.null);
        assert_eq!(o.code(), 8, "row 181 code");
        assert_eq!(o.text(), "invalid escape near '\"a\\'", "row 181 text");
        let o = d_loads(c, r, b"[\"\\", 0, "row 181 immediate");
        assert!(o.null);
        assert_eq!(o.code(), 8);
        assert_eq!(o.text(), "invalid escape near '\"\\'");

        // --- row 182: high surrogate not followed by a \u escape.
        let cases: &[(&[u8], &str)] = &[
            (b"[\"\\uD800\"]", "invalid Unicode '\\uD800' near '\"\\uD800\"'"),
            (b"[\"\\uD800x\"]", "invalid Unicode '\\uD800' near '\"\\uD800x\"'"),
            (b"[\"\\uDBFF\"]", "invalid Unicode '\\uDBFF' near '\"\\uDBFF\"'"),
            (b"[\"\\uD800\\n\"]", "invalid Unicode '\\uD800' near '\"\\uD800\\n\"'"),
            (b"[\"\\ud800\"]", "invalid Unicode '\\uD800' near '\"\\ud800\"'"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 182");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 182 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 182 text for {:?}", show(t));
        }

        // --- row 183: high surrogate followed by a \u escape out of DC00..DFFF.
        let cases: &[(&[u8], &str)] = &[
            (
                b"[\"\\uD800\\u0041\"]",
                "invalid Unicode '\\uD800\\u0041' near '\"\\uD800\\u0041\"'",
            ),
            (
                b"[\"\\uD800\\uD800\"]",
                "invalid Unicode '\\uD800\\uD800' near '\"\\uD800\\uD800\"'",
            ),
            (
                b"[\"\\uDBFF\\uE000\"]",
                "invalid Unicode '\\uDBFF\\uE000' near '\"\\uDBFF\\uE000\"'",
            ),
            (
                b"[\"\\uD800\\u0000\"]",
                "invalid Unicode '\\uD800\\u0000' near '\"\\uD800\\u0000\"'",
            ),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 183");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 183 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 183 text for {:?}", show(t));
        }
        // The boundaries of the valid second-surrogate range must be ACCEPTED,
        // so the rejection is not merely "any pair fails".
        for t in [&b"[\"\\uD800\\uDC00\"]"[..], b"[\"\\uDBFF\\uDFFF\"]"] {
            let o = d_loads(c, r, t, 0, "row 183 valid pair");
            assert!(!o.null, "C: {:?} is a valid surrogate pair", show(t));
        }

        // --- row 184: lone LOW surrogate.
        let cases: &[(&[u8], &str)] = &[
            (b"[\"\\uDC00\"]", "invalid Unicode '\\uDC00' near '\"\\uDC00\"'"),
            (b"[\"\\uDFFF\"]", "invalid Unicode '\\uDFFF' near '\"\\uDFFF\"'"),
            (
                b"[\"\\uDC00\\uDC00\"]",
                "invalid Unicode '\\uDC00' near '\"\\uDC00\\uDC00\"'",
            ),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 184");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 184 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 184 text for {:?}", show(t));
        }
        // Just outside the surrogate block on both sides must be accepted.
        for t in [&b"[\"\\uD7FF\"]"[..], b"[\"\\uE000\"]"] {
            let o = d_loads(c, r, t, 0, "row 184 boundary accept");
            assert!(!o.null, "C: {:?} must be accepted", show(t));
        }
    }
}

// ===========================================================================
// Rows 186-192 — number-literal rejections
// ===========================================================================

#[test]
fn rows_186_192_number_rejections() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- row 186: leading zero followed by a digit.
        for &(t, want) in &[
            (&b"[01]"[..], "invalid token near '0'"),
            (b"[00]", "invalid token near '0'"),
            (b"[-01]", "invalid token near '-0'"),
            (b"[012345]", "invalid token near '0'"),
        ] {
            let o = d_loads(c, r, t, 0, "row 186");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 186 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 186 text for {:?}", show(t));
        }

        // --- row 187: '-' not followed by a digit.
        for &(t, want) in &[
            (&b"[-]"[..], "invalid token near '-'"),
            (b"[-x]", "invalid token near '-'"),
            (b"[-.5]", "invalid token near '-'"),
            (b"[- 1]", "invalid token near '-'"),
            (b"[-,]", "invalid token near '-'"),
        ] {
            let o = d_loads(c, r, t, 0, "row 187");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 187 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 187 text for {:?}", show(t));
        }
        // '-' at end of input, where lex_get_save returns EOF.
        let o = d_loads(c, r, b"[-", 0, "row 187 EOF after minus");
        assert!(o.null);
        assert_eq!(o.code(), 8, "row 187 EOF code");
        assert_eq!(o.text(), "invalid token near '-'");

        // --- row 188: '.' not followed by a digit.
        for &(t, want) in &[
            (&b"[1.]"[..], "invalid token near '1.'"),
            (b"[1.e5]", "invalid token near '1.'"),
            (b"[0.]", "invalid token near '0.'"),
            (b"[-1.]", "invalid token near '-1.'"),
            (b"[1.x]", "invalid token near '1.'"),
        ] {
            let o = d_loads(c, r, t, 0, "row 188");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 188 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 188 text for {:?}", show(t));
        }

        // --- row 189: exponent marker not followed by a digit.
        for &(t, want) in &[
            (&b"[1e]"[..], "invalid token near '1e'"),
            (b"[1e+]", "invalid token near '1e+'"),
            (b"[1e-]", "invalid token near '1e-'"),
            (b"[1E]", "invalid token near '1E'"),
            (b"[1E+]", "invalid token near '1E+'"),
            (b"[1.5e]", "invalid token near '1.5e'"),
            (b"[1ex]", "invalid token near '1e'"),
        ] {
            let o = d_loads(c, r, t, 0, "row 189");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 189 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 189 text for {:?}", show(t));
        }

        // --- row 190: integer literal above JSON_INTEGER_MAX.
        for &(t, want) in &[
            (
                &b"[9223372036854775808]"[..],
                "too big integer near '9223372036854775808'",
            ),
            (
                b"[9223372036854775809]",
                "too big integer near '9223372036854775809'",
            ),
            (
                b"[99999999999999999999]",
                "too big integer near '99999999999999999999'",
            ),
        ] {
            let o = d_loads(c, r, t, 0, "row 190");
            assert!(o.null, "C: {:?} must overflow", show(t));
            assert_eq!(o.code(), 15, "row 190 code for {:?}", show(t));
            assert_eq!(o.code(), JSON_ERROR_NUMERIC_OVERFLOW);
            assert_eq!(o.text(), want, "row 190 text for {:?}", show(t));
        }
        // JSON_INTEGER_MAX itself must be accepted.
        let o = d_loads(c, r, b"[9223372036854775807]", 0, "row 190 boundary");
        assert!(!o.null, "C: JSON_INTEGER_MAX must be accepted");
        // With JSON_DECODE_INT_AS_REAL the very same literals become reals and
        // are accepted, which proves the row is about the integer path.
        for t in [&b"[9223372036854775808]"[..], b"[99999999999999999999]"] {
            let o = d_loads(c, r, t, JSON_DECODE_INT_AS_REAL, "row 190 as real");
            assert!(!o.null, "C: {:?} must parse as a real", show(t));
        }

        // --- row 191: integer literal below JSON_INTEGER_MIN.
        // Note the 20-byte context gate in error_set(): "-9223372036854775809"
        // is exactly 20 saved bytes so it still gets a `near` clause, while
        // "-99999999999999999999" is 21 and therefore gets none.
        for &(t, want) in &[
            (
                &b"[-9223372036854775809]"[..],
                "too big negative integer near '-9223372036854775809'",
            ),
            (b"[-99999999999999999999]", "too big negative integer"),
        ] {
            let o = d_loads(c, r, t, 0, "row 191");
            assert!(o.null, "C: {:?} must overflow", show(t));
            assert_eq!(o.code(), 15, "row 191 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 191 text for {:?}", show(t));
        }
        let o = d_loads(c, r, b"[-9223372036854775808]", 0, "row 191 boundary");
        assert!(!o.null, "C: JSON_INTEGER_MIN must be accepted");

        // --- row 192: real literal overflowing a double.
        for &(t, want) in &[
            (&b"[1e999]"[..], "real number overflow near '1e999'"),
            (b"[-1e999]", "real number overflow near '-1e999'"),
            (b"[1e309]", "real number overflow near '1e309'"),
            (b"[1.5e400]", "real number overflow near '1.5e400'"),
        ] {
            let o = d_loads(c, r, t, 0, "row 192");
            assert!(o.null, "C: {:?} must overflow", show(t));
            assert_eq!(o.code(), 15, "row 192 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 192 text for {:?}", show(t));
            // Same with JSON_DECODE_ANY, as the row calls out.
            let o = d_loads(c, r, &t[1..t.len() - 1], JSON_DECODE_ANY, "row 192 bare");
            assert!(o.null);
            assert_eq!(o.code(), 15, "row 192 bare code");
        }
        // Underflow is NOT an error (jsonp_strtod only rejects ±HUGE_VAL).
        for t in [&b"[1e-999]"[..], b"[-1e-999]", b"[1e-400]"] {
            let o = d_loads(c, r, t, 0, "row 192 underflow is fine");
            assert!(!o.null, "C: {:?} must NOT be an error", show(t));
        }
        // A literal too long for the 20-byte error-context gate: saved_text
        // longer than 20 characters suppresses the "near" clause entirely.
        let long = b"[1.00000000000000000000000000000e999]";
        let o = d_loads(c, r, long, 0, "row 192 long literal");
        assert!(o.null);
        assert_eq!(o.code(), 15, "row 192 long literal code");
        assert_eq!(
            o.text(),
            "real number overflow",
            "row 192: no context past 20 saved bytes"
        );
    }
}

// ===========================================================================
// Rows 193, 194, 195, 341 — UTF-8 rejection in the stream
// ===========================================================================

#[test]
fn rows_193_195_and_341_utf8_rejections() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- row 193: first byte >= 0x80 that utf8_check_first rejects.
        //
        // The "no near context" half of the row applies when `saved_text` is
        // EMPTY at the moment of failure, i.e. when the bad byte is the first
        // byte of a token: `lex_scan` clears `saved_text` before reading, so
        // `error_set` takes the `else` branch, sees `STREAM_STATE_ERROR` and
        // uses the bare message. When the bad byte is inside a string literal
        // the opening `"` has already been saved, so the `near '...'` clause IS
        // added. Both forms are pinned below.
        //
        // (ERRORS.md row 193 quotes the bare form for `"[\"\xff\"]"`; the real
        // C library appends `near '"'` for that input — verified directly
        // against `c_src/build/libjansson.so` — so the bare form is asserted
        // for the token-initial position where it actually occurs.)
        let bad_first: Vec<u8> = (0x80u16..=0xBFu16)
            .chain(0xC0..=0xC1)
            .chain(0xF5..=0xFF)
            .map(|x| x as u8)
            .collect();
        for &b in &bad_first {
            // Inside a string literal: context present.
            let doc = [b'[', b'"', b, b'"', b']'];
            let o = d_loads(c, r, &doc, 0, "row 193 inside a string");
            assert!(o.null, "C: lead byte {b:#04x} must be rejected");
            assert_eq!(o.code(), 5, "row 193 code for {b:#04x}");
            assert_eq!(o.code(), JSON_ERROR_INVALID_UTF8);
            assert_eq!(
                o.text(),
                format!("unable to decode byte 0x{:x} near '\"'", b),
                "row 193 text for {b:#04x} inside a string"
            );
            // At the very start of the document: no context at all.
            let doc = [b, b'[', b']'];
            let o = d_loads(c, r, &doc, 0, "row 193 as the first byte");
            assert!(o.null);
            assert_eq!(o.code(), 5, "row 193 leading-byte code for {b:#04x}");
            assert_eq!(
                o.text(),
                format!("unable to decode byte 0x{:x}", b),
                "row 193 bare text for {b:#04x}"
            );
            // As the first byte of a token inside an array: also no context.
            let doc = [b'[', b, b']'];
            let o = d_loads(c, r, &doc, 0, "row 193 token-initial inside an array");
            assert!(o.null);
            assert_eq!(o.code(), 5);
            assert_eq!(o.text(), format!("unable to decode byte 0x{:x}", b));
        }
        // The exact examples from the row.
        let o = d_loads(c, r, b"[\"\xff\"]", 0, "row 193 documented example");
        assert_eq!(o.text(), "unable to decode byte 0xff near '\"'");
        let o = d_loads(c, r, b"[\xc0\x80]", 0, "row 193 overlong lead");
        assert_eq!(o.text(), "unable to decode byte 0xc0");

        // --- row 194: rejected by utf8_check_full (bad continuation, encoded
        // surrogate, overlong form). Inside a string, so the `near '"'` clause
        // is present exactly as for row 193.
        let cases: &[(&[u8], &str)] = &[
            (b"[\"\xc2\x41\"]", "unable to decode byte 0xc2 near '\"'"),
            (b"[\"\xed\xa0\x80\"]", "unable to decode byte 0xed near '\"'"),
            (b"[\"\xe0\x80\x80\"]", "unable to decode byte 0xe0 near '\"'"),
            (b"[\"\xf0\x80\x80\x80\"]", "unable to decode byte 0xf0 near '\"'"),
            (b"[\"\xf4\x90\x80\x80\"]", "unable to decode byte 0xf4 near '\"'"),
            (b"[\"\xc2\x00\"]", "unable to decode byte 0xc2 near '\"'"),
            (b"[\"\xe1\x80\x41\"]", "unable to decode byte 0xe1 near '\"'"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 194");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 5, "row 194 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 194 text for {:?}", show(t));
        }
        // The same sequences token-initially, where the context is absent.
        let cases: &[(&[u8], &str)] = &[
            (b"[\xc2\x41]", "unable to decode byte 0xc2"),
            (b"[\xed\xa0\x80]", "unable to decode byte 0xed"),
            (b"[\xe0\x80\x80]", "unable to decode byte 0xe0"),
            (b"[\xf0\x80\x80\x80]", "unable to decode byte 0xf0"),
            (b"[\xf4\x90\x80\x80]", "unable to decode byte 0xf4"),
        ];
        for &(t, want) in cases {
            let o = d_loads(c, r, t, 0, "row 194 token-initial");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 5, "row 194 code for {:?}", show(t));
            assert_eq!(o.text(), want, "row 194 text for {:?}", show(t));
        }

        // --- row 195: a leading UTF-8 BOM is a valid character but not a token.
        let o = d_loads(c, r, b"\xef\xbb\xbf[]", 0, "row 195");
        assert!(o.null, "C: a leading BOM must be rejected");
        assert_eq!(o.code(), 8, "row 195 code");
        assert_eq!(o.code(), JSON_ERROR_INVALID_SYNTAX);
        assert_eq!(
            o.text().as_bytes(),
            b"'[' or '{' expected near '\xef\xbb\xbf'",
            "row 195 text (the raw BOM bytes are echoed back)"
        );
        // With JSON_DECODE_ANY it becomes "invalid token" instead.
        let o = d_loads(c, r, b"\xef\xbb\xbf[]", JSON_DECODE_ANY, "row 195 DECODE_ANY");
        assert!(o.null);
        assert_eq!(o.code(), 8);
        assert_eq!(o.text().as_bytes(), b"invalid token near '\xef\xbb\xbf'");
        // A BOM in the middle, and other non-token multi-byte characters.
        for t in [&b"[\xef\xbb\xbf]"[..], b"[\xc2\xa9]", b"[\xe2\x82\xac]", b"[\xf0\x9f\x98\x80]"] {
            let o = d_loads(c, r, t, 0, "row 195 non-token multibyte");
            assert!(o.null, "C: {:?} must be rejected", show(t));
            assert_eq!(o.code(), 8, "row 195 code for {:?}", show(t));
        }

        // --- row 341: the multi-byte sequence is TRUNCATED at end of input, so
        // `stream->get` returns EOF (-1) for a continuation byte and
        // utf8_check_full sees 0xFF there.
        for lead in [0xC2u8, 0xDF, 0xE0, 0xE2, 0xEF, 0xF0, 0xF4] {
            // json_loads: the NUL terminator ends the stream. The opening `"`
            // is already in saved_text, so the `near` clause is present.
            let doc = [b'[', b'"', lead];
            let o = d_loads(c, r, &doc, 0, "row 341 via json_loads");
            assert!(o.null, "C: truncated lead {lead:#04x} must be rejected");
            assert_eq!(o.code(), 5, "row 341 code for lead {lead:#04x}");
            assert_eq!(
                o.text(),
                format!("unable to decode byte 0x{:x} near '\"'", lead),
                "row 341 text for lead {lead:#04x}"
            );
            // Token-initially there is no saved text, so no `near` clause.
            let bare = [lead];
            let o = d_loads(c, r, &bare, 0, "row 341 token-initial truncated lead");
            assert!(o.null);
            assert_eq!(o.code(), 5, "row 341 bare code for lead {lead:#04x}");
            assert_eq!(
                o.text(),
                format!("unable to decode byte 0x{:x}", lead),
                "row 341 bare text for lead {lead:#04x}"
            );
            // json_loadb: buflen ends the stream.
            let o = d_loadb(
                c,
                r,
                doc.as_ptr() as *const c_char,
                doc.len(),
                0,
                &format!("row 341 via json_loadb lead={lead:#04x}"),
            );
            assert!(o.null);
            assert_eq!(o.code(), 5, "row 341 loadb code for lead {lead:#04x}");
            // Partially truncated 3- and 4-byte sequences too. With exactly one
            // continuation byte present, a 2-byte lead is already COMPLETE, so
            // that case is an unterminated string (code 6) rather than a UTF-8
            // error (code 5) — the distinction both libraries must reproduce.
            let want_utf8_error = (c.utf8_check_first)(lead as c_char) > 2;
            let doc = [b'[', b'"', lead, 0x80];
            let o = d_loadb(
                c,
                r,
                doc.as_ptr() as *const c_char,
                doc.len(),
                0,
                &format!("row 341 one continuation lead={lead:#04x}"),
            );
            assert!(
                o.null,
                "C: lead {lead:#04x} + one continuation then EOF must fail"
            );
            assert_eq!(
                o.code(),
                if want_utf8_error { 5 } else { 6 },
                "row 341 code (1 continuation), lead {lead:#04x}"
            );
            if want_utf8_error {
                assert_eq!(
                    o.text(),
                    format!("unable to decode byte 0x{:x} near '\"'", lead),
                    "row 341 text (1 continuation), lead {lead:#04x}"
                );
            }
            // Two and three continuation bytes. Which error the C reports here
            // depends on the lead's own length: a 2-byte lead completes and the
            // SURPLUS 0x80 then fails `utf8_check_first` (code 5 via row 193), a
            // 3-byte lead completes and the string is unterminated (code 6), a
            // 4-byte lead is still truncated (code 5 via row 341). Rather than
            // re-derive that here, the requirement is that both libraries make
            // exactly the same choice — which `d_loadb`'s full-raw-image
            // comparison already enforces — plus a NULL return.
            for extra in 2..=3usize {
                let mut doc = vec![b'[', b'"', lead];
                doc.extend(std::iter::repeat(0x80u8).take(extra));
                let o = d_loadb(
                    c,
                    r,
                    doc.as_ptr() as *const c_char,
                    doc.len(),
                    0,
                    &format!("row 341 {extra} continuations, lead={lead:#04x}"),
                );
                assert!(o.null, "C: lead {lead:#04x} + {extra} continuations must fail");
                assert!(
                    o.code() == 5 || o.code() == 6,
                    "C: unexpected code {} for lead {lead:#04x} + {extra} continuations",
                    o.code()
                );
            }
        }
    }
}

// ===========================================================================
// Row 196 — json_loadb with a buflen shorter than the value
// ===========================================================================

#[test]
fn row_196_buflen_shorter_than_the_value() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let doc = b"[1]";
        let o = d_loadb(c, r, doc.as_ptr() as *const c_char, 2, 0, "row 196");
        assert!(o.null, "C: json_loadb(\"[1]\", 2) must fail");
        assert_eq!(o.code(), 6, "row 196 code");
        assert_eq!(o.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
        assert_eq!(o.text(), "']' expected near end of file", "row 196 text");
        assert_eq!(o.source(), "<buffer>", "row 196 source");

        // Every prefix length of a longer document, so every truncation point
        // in the lexer and the parser is exercised.
        let doc = b"{\"key\":[1,2.5,true,null,\"s\"]}";
        for n in 0..doc.len() {
            let o = d_loadb(
                c,
                r,
                doc.as_ptr() as *const c_char,
                n,
                0,
                &format!("row 196 prefix of length {n}"),
            );
            assert!(o.null, "C: a {n}-byte prefix must fail");
        }
        // The full length parses.
        let o = d_loadb(
            c,
            r,
            doc.as_ptr() as *const c_char,
            doc.len(),
            0,
            "row 196 full length",
        );
        assert!(!o.null, "C: the full buffer must parse");
    }
}

// ===========================================================================
// Rows 162-164, 169, 172, 175, 185, 197 — the out-of-memory paths
// ===========================================================================

/// Remaining allocations before the allocator starts failing. `-1` means
/// unlimited. Both libraries share this counter, so it is reset before each
/// individual call.
static BUDGET: AtomicIsize = AtomicIsize::new(-1);
/// Total allocation requests seen — used to size the budget sweeps.
static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn budget_malloc(n: size_t) -> *mut c_void {
    ALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
    let b = BUDGET.load(Ordering::SeqCst);
    if b == 0 {
        return std::ptr::null_mut();
    }
    if b > 0 {
        BUDGET.store(b - 1, Ordering::SeqCst);
    }
    malloc(n)
}

unsafe extern "C" fn budget_realloc(p: *mut c_void, n: size_t) -> *mut c_void {
    ALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
    let b = BUDGET.load(Ordering::SeqCst);
    if b == 0 {
        return std::ptr::null_mut();
    }
    if b > 0 {
        BUDGET.store(b - 1, Ordering::SeqCst);
    }
    realloc(p, n)
}

unsafe extern "C" fn budget_free(p: *mut c_void) {
    free(p);
}

/// Install the budgeted allocator on BOTH libraries, run `f`, and restore the
/// originals no matter how `f` ends (including a panic from a failed
/// assertion). The replacement forwards to the real libc `malloc`/`realloc`/
/// `free`, so memory allocated under it can safely be freed afterwards and
/// vice versa.
unsafe fn with_budget<F: FnOnce()>(c: &Api, r: &Api, f: F) {
    let (mut cm, mut crl, mut cf) = (None, None, None);
    let (mut rm, mut rrl, mut rf) = (None, None, None);
    (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
    (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);

    BUDGET.store(-1, Ordering::SeqCst);
    (c.json_set_alloc_funcs2)(
        Some(budget_malloc),
        Some(budget_realloc),
        Some(budget_free),
    );
    (r.json_set_alloc_funcs2)(
        Some(budget_malloc),
        Some(budget_realloc),
        Some(budget_free),
    );

    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    BUDGET.store(-1, Ordering::SeqCst);
    (c.json_set_alloc_funcs2)(cm, crl, cf);
    (r.json_set_alloc_funcs2)(rm, rrl, rf);

    // Sanity: allocation works again for the rest of the suite.
    let o = (c.json_object)();
    assert!(!o.is_null(), "C allocator was not restored");
    decref(c, o);
    let o = (r.json_object)();
    assert!(!o.is_null(), "Rust allocator was not restored");
    decref(r, o);

    if let Err(e) = res {
        std::panic::resume_unwind(e);
    }
}

/// One `json_loads` call with exactly `budget` successful allocations allowed.
unsafe fn loads_with_budget(
    c: &Api,
    r: &Api,
    buf: &[c_char],
    flags: size_t,
    budget: isize,
    ctx: &str,
) -> Obs {
    BUDGET.store(budget, Ordering::SeqCst);
    let mut ce = json_error_t::poisoned();
    let cj = (c.json_loads)(buf.as_ptr(), flags, &mut ce);
    BUDGET.store(budget, Ordering::SeqCst);
    let mut re = json_error_t::poisoned();
    let rj = (r.json_loads)(buf.as_ptr(), flags, &mut re);
    // Freeing is never budgeted, but json_delete must not be able to fail.
    BUDGET.store(-1, Ordering::SeqCst);

    let (co, ro) = (obs(cj, &ce), obs(rj, &re));
    cmp(&co, &ro, &format!("json_loads under allocation budget {budget} [{ctx}]"));
    decref(c, cj);
    decref(r, rj);
    co
}

#[test]
fn rows_162_175_185_197_out_of_memory_paths() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        with_budget(c, r, || {
            // Documents chosen so that each OOM row is on the allocation path:
            //   [1]            -> json_array (172), json_integer (163),
            //                     json_array_append_new (175)
            //   ["ab"]         -> the decoded-string jsonp_malloc (185) and
            //                     jsonp_stringn_nocheck_own (162)
            //   {"a":1}        -> json_object (164),
            //                     json_object_setn_new_nocheck (169)
            //   budget 0       -> lex_init / strbuffer_init (197)
            let docs: &[(&[u8], size_t)] = &[
                (b"[1]", 0),
                (b"[\"ab\"]", 0),
                (b"{\"a\":1}", 0),
                (b"{\"a\":[1,\"b\",true,2.5]}", 0),
                (b"[1.5]", 0),
                (b"\"ab\"", JSON_DECODE_ANY),
                (b"1", JSON_DECODE_ANY),
                (b"[]", 0),
                (b"{}", 0),
                (b"{\"a\":1,\"b\":2}", JSON_REJECT_DUPLICATES),
            ];
            for &(doc, flags) in docs {
                let buf = cs_bytes(doc);
                // How many allocations does an unrestricted parse need?
                ALLOC_CALLS.store(0, Ordering::SeqCst);
                let o = loads_with_budget(c, r, &buf, flags, -1, "unlimited");
                assert!(!o.null, "C: {:?} must parse when memory is available", show(doc));
                // ALLOC_CALLS counted BOTH libraries, so half of it is one
                // parse; sweep a little past that.
                let need = ALLOC_CALLS.load(Ordering::SeqCst) / 2 + 3;

                for budget in 0..=(need as isize) {
                    let o = loads_with_budget(
                        c,
                        r,
                        &buf,
                        flags,
                        budget,
                        &format!("input={:?} flags={flags:#x}", show(doc)),
                    );
                    if budget == 0 {
                        // Row 197: lex_init fails, so parse_json never runs and
                        // the error struct keeps whatever jsonp_error_init left
                        // (text[0] == 0, text[159] untouched). The full byte
                        // image is still compared above; only the *meaning* of
                        // the code byte is undefined, so no code assertion here.
                        assert!(
                            o.null,
                            "row 197: {:?} must fail when strbuffer_init OOMs",
                            show(doc)
                        );
                        assert_eq!(
                            o.text(),
                            "",
                            "row 197: only text[0] was cleared, so the message is empty"
                        );
                        assert_eq!(
                            (o.line(), o.column(), o.position()),
                            (-1, -1, 0),
                            "row 197: jsonp_error_init's values must survive untouched"
                        );
                    }
                }
            }

            // Row 185 in particular: the jsonp_malloc for the DECODED string
            // fails, the token stays TOKEN_INVALID, and parse_value reports
            // "invalid token" (code 8) rather than an out-of-memory error.
            // Locate that budget by sweeping and looking for the signature.
            let buf = cs_bytes(b"[\"ab\"]");
            let mut found = None;
            for budget in 1..12isize {
                let o = loads_with_budget(c, r, &buf, 0, budget, "row 185 search");
                if o.null && o.code() == 8 && o.text().starts_with("invalid token") {
                    found = Some((budget, o.text().to_string()));
                    break;
                }
            }
            let (budget, text) = found.expect(
                "row 185: no budget made the decoded-string jsonp_malloc fail with \
                 \"invalid token\" — the allocation sequence changed",
            );
            assert_eq!(
                text, "invalid token near '\"ab\"'",
                "row 185 text at budget {budget}"
            );

            // The same sweep through the other entry points, so the OOM rows are
            // not proven only for json_loads.
            for budget in 0..8isize {
                let doc = b"{\"a\":[1,2]}";
                BUDGET.store(budget, Ordering::SeqCst);
                let mut ce = json_error_t::poisoned();
                let cj = (c.json_loadb)(
                    doc.as_ptr() as *const c_char,
                    doc.len(),
                    0,
                    &mut ce,
                );
                BUDGET.store(budget, Ordering::SeqCst);
                let mut re = json_error_t::poisoned();
                let rj = (r.json_loadb)(
                    doc.as_ptr() as *const c_char,
                    doc.len(),
                    0,
                    &mut re,
                );
                BUDGET.store(-1, Ordering::SeqCst);
                let (co, ro) = (obs(cj, &ce), obs(rj, &re));
                cmp(&co, &ro, &format!("json_loadb under budget {budget}"));
                decref(c, cj);
                decref(r, rj);

                let mut cst = CbState {
                    data: doc.as_ptr(),
                    len: doc.len(),
                    pos: 0,
                    stop_after: 0,
                    mode: CB_FEED,
                    calls: 0,
                };
                let mut rst = CbState {
                    data: doc.as_ptr(),
                    len: doc.len(),
                    pos: 0,
                    stop_after: 0,
                    mode: CB_FEED,
                    calls: 0,
                };
                BUDGET.store(budget, Ordering::SeqCst);
                let mut ce = json_error_t::poisoned();
                let cj = (c.json_load_callback)(
                    Some(cb),
                    &mut cst as *mut CbState as *mut c_void,
                    0,
                    &mut ce,
                );
                BUDGET.store(budget, Ordering::SeqCst);
                let mut re = json_error_t::poisoned();
                let rj = (r.json_load_callback)(
                    Some(cb),
                    &mut rst as *mut CbState as *mut c_void,
                    0,
                    &mut re,
                );
                BUDGET.store(-1, Ordering::SeqCst);
                let (co, ro) = (obs(cj, &ce), obs(rj, &re));
                cmp(&co, &ro, &format!("json_load_callback under budget {budget}"));
                diff_eq!(cst.calls, rst.calls, "callback calls under budget {budget}");
                decref(c, cj);
                decref(r, rj);
            }
        });
    }
}

#[test]
fn row_197_lex_init_failure_on_every_entry_point() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let path = tmp_path("row197");
        std::fs::write(&path, b"[1]").expect("write temp file");
        let cpath = cs(path.to_str().unwrap());
        let mode = cs("rb");

        with_budget(c, r, || {
            // Budget 0 => the very first jsonp_malloc, which is
            // strbuffer_init's inside lex_init, returns NULL. Every entry point
            // must then return NULL leaving the error struct exactly as
            // jsonp_error_init left it: text[0] == '\0', line/column == -1,
            // position == 0, source = the entry point's own name, and
            // text[1..160] NEVER written (so the poison sentinel survives).
            BUDGET.store(0, Ordering::SeqCst);

            let expect_untouched = |o: &Obs, source: &str, what: &str| {
                assert!(o.null, "row 197: {what} must return NULL");
                assert_eq!(o.text(), "", "row 197: {what} must leave text empty");
                assert_eq!(o.source(), source, "row 197: {what} source");
                assert_eq!(
                    (o.line(), o.column(), o.position()),
                    (-1, -1, 0),
                    "row 197: {what} line/column/position"
                );
                // Everything past text[0] is still the 0x7f poison, which is
                // exactly why json_error_code() is meaningless on this path.
                let text = &o.raw[12 + JSON_ERROR_SOURCE_LENGTH..];
                assert_eq!(text[0], 0, "row 197: {what} text[0] must be NUL");
                assert!(
                    text[1..].iter().all(|&b| b == 0x7f),
                    "row 197: {what} must not write past text[0]"
                );
            };

            let buf = cs_bytes(b"[1]");
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            BUDGET.store(0, Ordering::SeqCst);
            let cj = (c.json_loads)(buf.as_ptr(), 0, &mut ce);
            BUDGET.store(0, Ordering::SeqCst);
            let rj = (r.json_loads)(buf.as_ptr(), 0, &mut re);
            let (co, ro) = (obs(cj, &ce), obs(rj, &re));
            cmp(&co, &ro, "row 197 json_loads");
            expect_untouched(&co, "<string>", "json_loads");
            BUDGET.store(-1, Ordering::SeqCst);
            decref(c, cj);
            decref(r, rj);

            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            BUDGET.store(0, Ordering::SeqCst);
            let cj = (c.json_loadb)(buf.as_ptr(), 3, 0, &mut ce);
            BUDGET.store(0, Ordering::SeqCst);
            let rj = (r.json_loadb)(buf.as_ptr(), 3, 0, &mut re);
            let (co, ro) = (obs(cj, &ce), obs(rj, &re));
            cmp(&co, &ro, "row 197 json_loadb");
            expect_untouched(&co, "<buffer>", "json_loadb");
            BUDGET.store(-1, Ordering::SeqCst);
            decref(c, cj);
            decref(r, rj);

            let cf = fopen(cpath.as_ptr(), mode.as_ptr());
            let rfp = fopen(cpath.as_ptr(), mode.as_ptr());
            assert!(!cf.is_null() && !rfp.is_null());
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            BUDGET.store(0, Ordering::SeqCst);
            let cj = (c.json_loadf)(cf, 0, &mut ce);
            BUDGET.store(0, Ordering::SeqCst);
            let rj = (r.json_loadf)(rfp, 0, &mut re);
            BUDGET.store(-1, Ordering::SeqCst);
            fclose(cf);
            fclose(rfp);
            let (co, ro) = (obs(cj, &ce), obs(rj, &re));
            cmp(&co, &ro, "row 197 json_loadf");
            expect_untouched(&co, "<stream>", "json_loadf");
            decref(c, cj);
            decref(r, rj);

            let f1 = std::fs::File::open(&path).expect("open fd");
            let f2 = std::fs::File::open(&path).expect("open fd");
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            BUDGET.store(0, Ordering::SeqCst);
            let cj = (c.json_loadfd)(f1.as_raw_fd(), 0, &mut ce);
            BUDGET.store(0, Ordering::SeqCst);
            let rj = (r.json_loadfd)(f2.as_raw_fd(), 0, &mut re);
            BUDGET.store(-1, Ordering::SeqCst);
            drop(f1);
            drop(f2);
            let (co, ro) = (obs(cj, &ce), obs(rj, &re));
            cmp(&co, &ro, "row 197 json_loadfd");
            expect_untouched(&co, "<stream>", "json_loadfd");
            decref(c, cj);
            decref(r, rj);

            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            BUDGET.store(0, Ordering::SeqCst);
            let cj = (c.json_load_file)(cpath.as_ptr(), 0, &mut ce);
            BUDGET.store(0, Ordering::SeqCst);
            let rj = (r.json_load_file)(cpath.as_ptr(), 0, &mut re);
            BUDGET.store(-1, Ordering::SeqCst);
            let (co, ro) = (obs(cj, &ce), obs(rj, &re));
            cmp(&co, &ro, "row 197 json_load_file");
            // NOTE: `json_load_file` sets `source` to the path, then delegates
            // to `json_loadf`, which calls `jsonp_error_init(error, "<stream>")`
            // a second time and OVERWRITES it. So the path only survives on the
            // `fopen` failure path (row 150); once the file opens, the source is
            // "<stream>".
            expect_untouched(&co, "<stream>", "json_load_file");
            decref(c, cj);
            decref(r, rj);

            let doc = b"[1]";
            let mut cst = CbState {
                data: doc.as_ptr(),
                len: 3,
                pos: 0,
                stop_after: 0,
                mode: CB_FEED,
                calls: 0,
            };
            let mut rst = CbState {
                data: doc.as_ptr(),
                len: 3,
                pos: 0,
                stop_after: 0,
                mode: CB_FEED,
                calls: 0,
            };
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            BUDGET.store(0, Ordering::SeqCst);
            let cj = (c.json_load_callback)(
                Some(cb),
                &mut cst as *mut CbState as *mut c_void,
                0,
                &mut ce,
            );
            BUDGET.store(0, Ordering::SeqCst);
            let rj = (r.json_load_callback)(
                Some(cb),
                &mut rst as *mut CbState as *mut c_void,
                0,
                &mut re,
            );
            BUDGET.store(-1, Ordering::SeqCst);
            let (co, ro) = (obs(cj, &ce), obs(rj, &re));
            cmp(&co, &ro, "row 197 json_load_callback");
            expect_untouched(&co, "<callback>", "json_load_callback");
            // The callback must never have been invoked: lex_init fails first.
            diff_eq!(cst.calls, rst.calls, "row 197 callback invocation count");
            assert_eq!(cst.calls, 0, "row 197: the callback must not be called");
            decref(c, cj);
            decref(r, rj);
        });
        let _ = std::fs::remove_file(&path);
    }
}

// ===========================================================================
// Rows 339, 340 — EOF from a FILE* / a file descriptor
// ===========================================================================

#[test]
fn row_339_truncated_file() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let path = tmp_path("row339");
        // The documented case: `fgetc` hits EOF mid-value.
        let o = d_load_file_bytes(c, r, b"[1,2", 0, &path, "row 339");
        assert!(o.null, "C: a truncated file must fail");
        assert_eq!(o.code(), 6, "row 339 code");
        assert_eq!(o.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
        assert_eq!(o.text(), "']' expected near end of file", "row 339 text");
        // `json_load_file` delegates to `json_loadf`, whose own
        // `jsonp_error_init(error, "<stream>")` overwrites the path it had just
        // stored in `source`. So the path is visible only when `fopen` fails
        // (row 150) — this is the observable consequence of the double init.
        assert_eq!(o.source(), "<stream>", "row 339 source");

        // The analogous '}' case, and the same through json_loadf directly.
        let o = d_load_file_bytes(c, r, b"{\"a\":1", 0, &path, "row 339 object");
        assert!(o.null);
        assert_eq!(o.code(), 6);
        assert_eq!(o.text(), "'}' expected near end of file");

        for t in [&b""[..], b"[", b"[1", b"[1,", b"{", b"{\"a\"", b"{\"a\":", b"[\"abc"] {
            let o = d_loadf_bytes(c, r, t, 0, &path, "row 339 via json_loadf");
            assert!(o.null, "C: {:?} must fail", show(t));
            assert_eq!(o.code(), 6, "row 339 code for {:?}", show(t));
            assert_eq!(o.source(), "<stream>", "row 339 json_loadf source");
            let o = d_load_file_bytes(c, r, t, 0, &path, "row 339 via json_load_file");
            assert!(o.null);
            assert_eq!(o.code(), 6, "row 339 load_file code for {:?}", show(t));
        }
        let _ = std::fs::remove_file(&path);
    }
}

#[test]
fn row_340_fd_read_returns_eof() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let path = tmp_path("row340");
        // An EMPTY file: the first read() returns 0, which fd_get_func maps to
        // EOF.
        let o = d_loadfd_bytes(c, r, b"", 0, &path, "row 340 empty fd");
        assert!(o.null, "C: an empty descriptor must fail");
        assert_eq!(o.code(), 6, "row 340 code");
        assert_eq!(o.code(), JSON_ERROR_PREMATURE_END_OF_INPUT);
        assert_eq!(
            o.text(),
            "'[' or '{' expected near end of file",
            "row 340 text"
        );
        assert_eq!(o.source(), "<stream>", "row 340 source");

        // A CLOSED descriptor: read() returns -1 (EBADF), also mapped to EOF.
        let fd = {
            let f = std::fs::File::open(&path).expect("open fd");
            let fd = f.as_raw_fd();
            drop(f);
            fd
        };
        let o = d_loadfd_raw(c, r, fd, 0, "row 340 closed fd");
        assert!(o.null, "C: a closed descriptor must fail");
        assert_eq!(o.code(), 6, "row 340 closed-fd code");
        assert_eq!(o.text(), "'[' or '{' expected near end of file");

        // A descriptor number that was never open at all.
        for fd in [9997, 9998, 9999] {
            let o = d_loadfd_raw(c, r, fd, 0, "row 340 never-open fd");
            assert!(o.null, "C: fd {fd} must fail");
            assert_eq!(o.code(), 6, "row 340 bogus-fd code for {fd}");
            assert_eq!(o.text(), "'[' or '{' expected near end of file");
        }

        // Truncated content on a real descriptor.
        for t in [&b"["[..], b"[1,2", b"{\"a\":", b"[\"abc"] {
            let o = d_loadfd_bytes(c, r, t, 0, &path, "row 340 truncated fd");
            assert!(o.null, "C: {:?} must fail", show(t));
            assert_eq!(o.code(), 6, "row 340 truncated code for {:?}", show(t));
        }
        let _ = std::fs::remove_file(&path);
    }
}

// ===========================================================================
// Rows 343, 344, 345, 347, 348 — live asserts, not testable in-process
// ===========================================================================

/// ERRORS.md rows 343, 344, 345, 347 and 348 all end in a live `assert()`
/// (`c_src` is built with `CMAKE_BUILD_TYPE` empty, so `NDEBUG` is absent and
/// the asserts are compiled in). Reaching any of them would raise `SIGABRT`,
/// which kills the whole test process — so there is no in-process differential
/// assertion to make. More importantly, **no input reaches them at all**; each
/// is an internal invariant that the surrounding code has already established.
/// Quoting `c_src/src/load.c`:
///
/// * **row 345** — `stream_get`:
///   ```c
///   count = utf8_check_first(c);
///   if (!count)
///       goto out;
///   assert(count >= 2);
///   ```
///   `utf8_check_first` returns only `0`, `1`, `2`, `3` or `4`, and returns `1`
///   exclusively for `c < 0x80` (see `c_src/src/utf.c`). This block is guarded
///   by `if (0x80 <= c && c <= 0xFF)`, and `count == 0` was just handled by the
///   `goto out`, so `count` is necessarily 2, 3 or 4. a01_utf.rs proves that
///   for all 256 byte values exhaustively.
///
/// * **row 347** — `lex_unget_unsave`:
///   ```c
///   stream_unget(&lex->stream, c);
///   d = strbuffer_pop(&lex->saved_text);
///   assert(c == d);
///   ```
///   Every `lex_unget_unsave(lex, c)` in the lexer is preceded by a
///   `lex_get_save` that returned that same `c` and pushed it onto
///   `saved_text`, so the popped byte is always the ungotten one. (The same
///   argument covers the `assert(stream->buffer_pos > 0)` and
///   `assert(stream->buffer[stream->buffer_pos] == c)` of row 346.)
///
/// * **row 343** — `lex_scan_string`, second pass:
///   ```c
///   if (utf8_encode(value, t, &length))
///       assert(0);
///   ```
///   `utf8_encode` only fails for `value < 0` or `value > 0x10FFFF` or a
///   surrogate. `decode_unicode_escape` returns at most `0xFFFF`, negative
///   values are rejected immediately above, and the surrogate range is
///   rejected by the three `invalid Unicode` branches (proven by rows 182-184
///   in `rows_176_184_string_literal_rejections`). A combined surrogate pair is
///   at most `((0xDBFF-0xD800)<<10)+(0xDFFF-0xDC00)+0x10000 == 0x10FFFF`.
///
/// * **row 344** — `lex_scan_string`, second pass, the escape `switch`:
///   ```c
///   default:
///       assert(0);
///   ```
///   The first pass already rejected every byte after a `\` that is not one of
///   `" \ / b f n r t u` with `"invalid escape"` (row 180, tested above), so the
///   second pass sees only those characters.
///
/// * **row 348** — `lex_scan_number`:
///   ```c
///   assert(end == saved_text + lex->saved_text.length);
///   ```
///   `saved_text` at that point holds exactly the digit string the lexer just
///   validated character-by-character, so `strtoll` consumes all of it.
///
/// A Rust port may therefore implement these as `unreachable!()` /
/// `debug_assert!` (or omit them, as `translation/src/load.rs` does) with no
/// observable difference, and the corresponding ERRORS.md rows are marked
/// `[-] unreachable: assert` rather than `[x]`.
///
/// What CAN be asserted is that the *guards* which make these asserts
/// unreachable really are in place, and that they behave identically in both
/// libraries — which is what this test does.
#[test]
fn rows_343_348_live_asserts_are_unreachable_by_construction() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Guard for rows 343/344: the first pass rejects every illegal escape
        // and every out-of-range/surrogate \u value BEFORE the second pass can
        // reach utf8_encode or the switch default.
        for b in 0u8..=127 {
            if b == 0 {
                continue; // ends the NUL-terminated json_loads input
            }
            let doc = [b'[', b'"', b'\\', b, b'"', b']'];
            let o = d_loads(c, r, &doc, 0, "rows 343/344 escape guard");
            let legal = matches!(b, b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't');
            if legal {
                assert!(!o.null, "C: \\{} is a legal escape", b as char);
            } else if b == b'u' {
                // \u" -> "invalid escape" (not four hex digits).
                assert!(o.null);
                assert_eq!(o.code(), 8);
            } else {
                assert!(o.null, "C: \\{:#04x} must be rejected in the FIRST pass", b);
                assert_eq!(o.code(), 8, "escape {b:#04x} code");
                assert!(
                    o.text().starts_with("invalid escape"),
                    "C: expected \"invalid escape\", got {:?}",
                    o.text()
                );
            }
        }
        // Every codepoint class \uXXXX can name: the surrogate halves must be
        // rejected (so utf8_encode is never handed one), everything else must
        // encode. Sampled across the whole 16-bit range plus every boundary.
        let mut probes: Vec<u32> = vec![
            0x0001, 0x007F, 0x0080, 0x07FF, 0x0800, 0xD7FE, 0xD7FF, 0xD800, 0xD801, 0xDBFE,
            0xDBFF, 0xDC00, 0xDC01, 0xDFFE, 0xDFFF, 0xE000, 0xE001, 0xFFFD, 0xFFFE, 0xFFFF,
        ];
        let mut rng = Rng::new(0x0343_0348);
        for _ in 0..200 {
            probes.push((rng.next_u32() & 0xFFFF).max(1));
        }
        for cp in probes {
            let doc = format!("[\"\\u{cp:04X}\"]");
            let o = d_loads(c, r, doc.as_bytes(), 0, "row 343 codepoint guard");
            let is_surrogate = (0xD800..=0xDFFF).contains(&cp);
            if is_surrogate {
                assert!(o.null, "C: \\u{cp:04X} is a lone surrogate and must fail");
                assert_eq!(o.code(), 8, "surrogate \\u{cp:04X} code");
                assert!(
                    o.text().starts_with("invalid Unicode"),
                    "C: got {:?}",
                    o.text()
                );
            } else {
                assert!(!o.null, "C: \\u{cp:04X} must encode");
            }
        }
        // Guard for row 348: every integer literal the lexer hands to strtoll
        // is a complete, validated digit string. Overflow is reported as
        // json_error_numeric_overflow (rows 190/191) instead of tripping the
        // assert, and every non-digit terminator is ungotten first.
        for t in [
            &b"[0]"[..],
            b"[-0]",
            b"[9223372036854775807]",
            b"[-9223372036854775808]",
            b"[9223372036854775808]",
            b"[-9223372036854775809]",
            b"[1,2]",
            b"[1]",
            b"[12345678901234567890123456789012345678901234567890]",
        ] {
            // The point is simply that neither library aborts.
            d_loads(c, r, t, 0, "row 348 strtoll guard");
        }
        // Guard for row 345: every byte 0x80..0xFF either has
        // utf8_check_first == 0 (handled by `goto out`) or >= 2, so the
        // `assert(count >= 2)` cannot fire. Checked directly on the exported
        // helper, for both libraries.
        for b in 0x80u16..=0xFF {
            let cn = (c.utf8_check_first)(b as u8 as c_char);
            let rn = (r.utf8_check_first)(b as u8 as c_char);
            diff_eq!(cn, rn, "utf8_check_first({b:#04x}) — row 345 guard");
            assert!(
                cn == 0 || cn >= 2,
                "row 345: utf8_check_first({b:#04x}) returned {cn}, which would trip \
                 assert(count >= 2)"
            );
        }
        // Guard for row 347: a long run of unget/save pairs through the number
        // and identifier lexers, all of which must round-trip cleanly.
        for t in [
            &b"[1 ,2]"[..],
            b"[1\t]",
            b"[1\n]",
            b"[1\r]",
            b"[true ]",
            b"[null,]",
            b"[1.5 ]",
            b"[1e5 ]",
            b"[-1 ]",
            b"[0 ]",
        ] {
            d_loads(c, r, t, 0, "row 347 unget/save round-trip");
        }
    }
}

// ===========================================================================
// Row 355 — flag words with undefined bits set
// ===========================================================================

#[test]
fn row_355_undefined_flag_bits_are_ignored() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // `flags` is a plain `size_t` with no validation anywhere in load.c, so
        // ANY 64-bit word is a legal argument. Bits outside DECODE_FLAG_MASK
        // must be folded away entirely: the result must be byte-identical to
        // the result with `flags & DECODE_FLAG_MASK`.
        //
        // Note the deliberate inclusion of encoder flags. JSON_INDENT(n) is
        // `n & 0x1F`, so an indent value ALIASES the decoder bits — the
        // comparison below is against `flags & 0x1F`, which captures that
        // aliasing correctly instead of pretending it does not happen.
        let inputs: &[&[u8]] = &[
            b"[1]",
            b"{\"a\":1}",
            b"{\"a\":1,\"a\":2}",
            b"[\"a\\u0000b\"]",
            b"[1] x",
            b"1",
            b"",
            b"[",
            b"[01]",
            b"[1e999]",
            b"[\"\xff\"]",
            b"{\"a\\u0000b\":1}",
            b"[9223372036854775808]",
        ];
        let mut flag_words: Vec<size_t> = vec![
            0x20,                 // JSON_COMPACT
            0x40,                 // JSON_ENSURE_ASCII
            0x80,                 // JSON_SORT_KEYS
            0x100,                // JSON_PRESERVE_ORDER
            0x200,                // JSON_ENCODE_ANY
            0x400,                // JSON_ESCAPE_SLASH
            0x8000,               // undefined
            0x10000,              // JSON_EMBED
            0x8000_0000,
            0x1_0000_0000,
            usize::MAX,
            usize::MAX - 1,
            usize::MAX & !DECODE_FLAG_MASK,
            !0usize ^ 0x1F,
            json_real_precision(17),
            json_indent(0),
            json_indent(4),
            json_indent(31),
            0x20 | JSON_DECODE_ANY,
            0x8000 | JSON_ALLOW_NUL,
            0x1_0000 | JSON_REJECT_DUPLICATES,
        ];
        let mut rng = Rng::new(0x0355_0355);
        for _ in 0..48 {
            flag_words.push(rng.next_u64() as size_t);
        }

        for t in inputs {
            // The reference result for the five meaningful bits.
            for &f in &flag_words {
                let masked = f & DECODE_FLAG_MASK;
                let base = d_loads(c, r, t, masked, "row 355 masked reference");
                let full = d_loads(c, r, t, f, "row 355 full flag word");
                assert_eq!(
                    base.null, full.null,
                    "row 355: undefined bits changed the outcome for {:?} \
                     flags={f:#x} (masked={masked:#x})",
                    show(t)
                );
                assert_eq!(
                    base.raw, full.raw,
                    "row 355: undefined bits changed the json_error_t for {:?} \
                     flags={f:#x} (masked={masked:#x})",
                    show(t)
                );
            }
        }

        // The same claim for every other entry point, on a smaller grid.
        let path = tmp_path("row355");
        for t in [&b"[1,2"[..], b"{\"a\":1}", b"1"] {
            for &f in &[0x20usize, 0x8000, usize::MAX & !DECODE_FLAG_MASK, 0x10000] {
                let masked = f & DECODE_FLAG_MASK;
                let a = d_loadb(c, r, t.as_ptr() as *const c_char, t.len(), masked, "row 355 loadb");
                let b = d_loadb(c, r, t.as_ptr() as *const c_char, t.len(), f, "row 355 loadb");
                assert_eq!(a.raw, b.raw, "row 355 json_loadb flags={f:#x} on {:?}", show(t));

                let a = d_loadf_bytes(c, r, t, masked, &path, "row 355 loadf");
                let b = d_loadf_bytes(c, r, t, f, &path, "row 355 loadf");
                assert_eq!(a.raw, b.raw, "row 355 json_loadf flags={f:#x} on {:?}", show(t));

                let a = d_loadfd_bytes(c, r, t, masked, &path, "row 355 loadfd");
                let b = d_loadfd_bytes(c, r, t, f, &path, "row 355 loadfd");
                assert_eq!(a.raw, b.raw, "row 355 json_loadfd flags={f:#x} on {:?}", show(t));

                let a = d_load_file_bytes(c, r, t, masked, &path, "row 355 load_file");
                let b = d_load_file_bytes(c, r, t, f, &path, "row 355 load_file");
                assert_eq!(
                    a.raw, b.raw,
                    "row 355 json_load_file flags={f:#x} on {:?}",
                    show(t)
                );

                let a = d_load_callback(c, r, Some(cb), t, CB_FEED, 0, masked, "row 355 cb");
                let b = d_load_callback(c, r, Some(cb), t, CB_FEED, 0, f, "row 355 cb");
                assert_eq!(
                    a.raw, b.raw,
                    "row 355 json_load_callback flags={f:#x} on {:?}",
                    show(t)
                );
            }
        }
        let _ = std::fs::remove_file(&path);
    }
}

// ===========================================================================
// Generic FFI boundaries for every json_load* entry point
// ===========================================================================

#[test]
fn null_error_pointer_is_tolerated_everywhere() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // `json_error_t *error` is documented as optional. NULL must be
        // tolerated on EVERY path — including the argument-guard paths, where
        // error_set() is reached with a NULL error, and the success path, where
        // parse_json writes error->position.
        let path = tmp_path("nullerr");
        std::fs::write(&path, b"[1,2").expect("write temp file");
        let cpath = cs(path.to_str().unwrap());
        let mode = cs("rb");
        let nul: *mut json_error_t = std::ptr::null_mut();

        for t in [&b"[1]"[..], b"[1,2", b"", b"[\"\xff\"]", b"1", b"[01]", b"[1e999]"] {
            let buf = cs_bytes(t);
            let cj = (c.json_loads)(buf.as_ptr(), 0, nul);
            let rj = (r.json_loads)(buf.as_ptr(), 0, nul);
            diff_eq!(cj.is_null(), rj.is_null(), "json_loads(error=NULL) on {:?}", show(t));
            decref(c, cj);
            decref(r, rj);

            let p = if t.is_empty() {
                b"".as_ptr() as *const c_char
            } else {
                t.as_ptr() as *const c_char
            };
            let cj = (c.json_loadb)(p, t.len(), 0, nul);
            let rj = (r.json_loadb)(p, t.len(), 0, nul);
            diff_eq!(cj.is_null(), rj.is_null(), "json_loadb(error=NULL) on {:?}", show(t));
            decref(c, cj);
            decref(r, rj);

            std::fs::write(&path, t).expect("write temp file");
            let cf = fopen(cpath.as_ptr(), mode.as_ptr());
            let cj = (c.json_loadf)(cf, 0, nul);
            fclose(cf);
            let rf = fopen(cpath.as_ptr(), mode.as_ptr());
            let rj = (r.json_loadf)(rf, 0, nul);
            fclose(rf);
            diff_eq!(cj.is_null(), rj.is_null(), "json_loadf(error=NULL) on {:?}", show(t));
            decref(c, cj);
            decref(r, rj);

            let f1 = std::fs::File::open(&path).expect("open fd");
            let cj = (c.json_loadfd)(f1.as_raw_fd(), 0, nul);
            drop(f1);
            let f2 = std::fs::File::open(&path).expect("open fd");
            let rj = (r.json_loadfd)(f2.as_raw_fd(), 0, nul);
            drop(f2);
            diff_eq!(cj.is_null(), rj.is_null(), "json_loadfd(error=NULL) on {:?}", show(t));
            decref(c, cj);
            decref(r, rj);

            let cj = (c.json_load_file)(cpath.as_ptr(), 0, nul);
            let rj = (r.json_load_file)(cpath.as_ptr(), 0, nul);
            diff_eq!(
                cj.is_null(),
                rj.is_null(),
                "json_load_file(error=NULL) on {:?}",
                show(t)
            );
            decref(c, cj);
            decref(r, rj);

            let mut cst = CbState {
                data: if t.is_empty() { b"".as_ptr() } else { t.as_ptr() },
                len: t.len(),
                pos: 0,
                stop_after: 0,
                mode: CB_FEED,
                calls: 0,
            };
            let mut rst = CbState {
                data: if t.is_empty() { b"".as_ptr() } else { t.as_ptr() },
                len: t.len(),
                pos: 0,
                stop_after: 0,
                mode: CB_FEED,
                calls: 0,
            };
            let cj = (c.json_load_callback)(
                Some(cb),
                &mut cst as *mut CbState as *mut c_void,
                0,
                nul,
            );
            let rj = (r.json_load_callback)(
                Some(cb),
                &mut rst as *mut CbState as *mut c_void,
                0,
                nul,
            );
            diff_eq!(
                cj.is_null(),
                rj.is_null(),
                "json_load_callback(error=NULL) on {:?}",
                show(t)
            );
            decref(c, cj);
            decref(r, rj);
        }

        // The NULL-argument guards with a NULL error pointer as well: both
        // NULLs at once must still be a clean NULL return.
        assert!((c.json_loads)(std::ptr::null(), 0, nul).is_null());
        assert!((r.json_loads)(std::ptr::null(), 0, nul).is_null());
        assert!((c.json_loadb)(std::ptr::null(), 0, 0, nul).is_null());
        assert!((r.json_loadb)(std::ptr::null(), 0, 0, nul).is_null());
        assert!((c.json_loadf)(std::ptr::null_mut(), 0, nul).is_null());
        assert!((r.json_loadf)(std::ptr::null_mut(), 0, nul).is_null());
        assert!((c.json_loadfd)(-1, 0, nul).is_null());
        assert!((r.json_loadfd)(-1, 0, nul).is_null());
        assert!((c.json_load_file)(std::ptr::null(), 0, nul).is_null());
        assert!((r.json_load_file)(std::ptr::null(), 0, nul).is_null());
        assert!((c.json_load_callback)(None, std::ptr::null_mut(), 0, nul).is_null());
        assert!((r.json_load_callback)(None, std::ptr::null_mut(), 0, nul).is_null());

        let _ = std::fs::remove_file(&path);
    }
}

#[test]
fn json_loadb_zero_and_oversized_buflen() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // --- buflen == 0. The pointer is non-NULL, so the argument guard does
        // not fire; buffer_get returns EOF on the very first call.
        for &flags in &[0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK, usize::MAX] {
            let doc = b"[1,2,3]";
            let o = d_loadb(
                c,
                r,
                doc.as_ptr() as *const c_char,
                0,
                flags,
                "zero buflen",
            );
            assert!(o.null, "C: buflen 0 must fail (flags {flags:#x})");
            assert_eq!(o.code(), 6, "zero buflen code (flags {flags:#x})");
        }
        // A zero-length buffer whose pointer is a valid but empty allocation.
        let empty: [c_char; 1] = [0];
        let o = d_loadb(c, r, empty.as_ptr(), 0, 0, "zero buflen, empty buffer");
        assert!(o.null);
        assert_eq!(o.code(), 6);

        // --- buflen == (size_t)-1 and other oversized values. `json_loadb`
        // performs NO validation, so the lexer simply keeps reading. To make
        // that observable without running off the end of an allocation, the
        // buffer is padded so the parser is guaranteed to stop within it:
        //
        //   * "[1]" + JSON_DISABLE_EOF_CHECK stops right after ']' (3 bytes),
        //   * "[1]" + NUL padding stops at the NUL during the EOF check
        //     (4 bytes), because a NUL byte is not a token.
        let mut padded = vec![0u8; 4096];
        padded[..3].copy_from_slice(b"[1]");
        for &buflen in &[usize::MAX, usize::MAX - 1, usize::MAX / 2, 1 << 40, 4096] {
            let o = d_loadb(
                c,
                r,
                padded.as_ptr() as *const c_char,
                buflen,
                JSON_DISABLE_EOF_CHECK,
                "oversized buflen, EOF check disabled",
            );
            assert!(
                !o.null,
                "C: buflen {buflen:#x} with DISABLE_EOF_CHECK must still parse \"[1]\""
            );

            let o = d_loadb(
                c,
                r,
                padded.as_ptr() as *const c_char,
                buflen,
                0,
                "oversized buflen, EOF check enabled",
            );
            assert!(
                o.null,
                "C: buflen {buflen:#x} must hit the NUL padding during the EOF check"
            );
            assert_eq!(
                o.code(),
                7,
                "oversized buflen: the NUL padding is trailing content"
            );
            assert_eq!(o.code(), JSON_ERROR_END_OF_INPUT_EXPECTED);
        }
        // buflen one byte longer than the document is the same case.
        let doc = b"[1]\0\0\0\0\0";
        let o = d_loadb(
            c,
            r,
            doc.as_ptr() as *const c_char,
            doc.len(),
            0,
            "buflen past the end of the value",
        );
        assert!(o.null);
        assert_eq!(o.code(), 7);
    }
}

#[test]
fn json_load_file_on_a_directory() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // `fopen(dir, "rb")` SUCCEEDS on Linux; the failure only shows up when
        // stdio tries to read (EISDIR), which `fgetc` reports as EOF. So this
        // is NOT the row-150 `cannot_open_file` path — it lands on premature
        // end of input, and both libraries must agree on that distinction.
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        for d in [dir.as_str(), "/", "/tmp", "/usr"] {
            let p = cs(d);
            let o = d_load_file_path(c, r, p.as_ptr(), 0, &format!("directory {d:?}"));
            assert!(o.null, "C: json_load_file({d:?}) must fail");
            // Whichever branch the platform takes, it must be one of these two
            // and BOTH libraries must take the same one (already enforced by
            // the raw-image comparison inside d_load_file_path).
            assert_eq!(
                o.code(),
                6,
                "C: json_load_file on a directory must be premature end of input, \
                 not cannot_open_file (fopen succeeds; the read fails with EISDIR) \
                 — {d:?}"
            );
            assert_eq!(
                o.text(),
                "'[' or '{' expected near end of file",
                "directory case text for {d:?}"
            );
            // `json_loadf`'s second `jsonp_error_init` replaced the path.
            assert_eq!(o.source(), "<stream>", "directory case source for {d:?}");
        }
    }
}

#[test]
fn stdin_is_named_stdin_by_loadf_and_loadfd() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // `json_loadf` picks its `source` with `input == stdin` and
        // `json_loadfd` with `input == STDIN_FILENO`. Both comparisons are
        // easy to get subtly wrong across an FFI boundary (the Rust side has to
        // resolve the libc `stdin` GLOBAL, not its own copy), and neither is
        // exercised by any other row.
        //
        // Reading from the real stdin would block, so the calls run with an
        // allocation budget of 0: `lex_init` then fails BEFORE a single byte is
        // read, while `jsonp_error_init(error, source)` has already run — which
        // is exactly the observable we want.
        with_budget(c, r, || {
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            BUDGET.store(0, Ordering::SeqCst);
            let cj = (c.json_loadf)(c_stdin_ptr(), 0, &mut ce);
            BUDGET.store(0, Ordering::SeqCst);
            let rj = (r.json_loadf)(c_stdin_ptr(), 0, &mut re);
            BUDGET.store(-1, Ordering::SeqCst);
            let (co, ro) = (obs(cj, &ce), obs(rj, &re));
            cmp(&co, &ro, "json_loadf(stdin) source naming");
            assert!(co.null);
            assert_eq!(co.source(), "<stdin>", "json_loadf(stdin) must name <stdin>");
            decref(c, cj);
            decref(r, rj);

            for (fd, want) in [(0, "<stdin>"), (1, "<stream>"), (2, "<stream>")] {
                let mut ce = json_error_t::poisoned();
                let mut re = json_error_t::poisoned();
                BUDGET.store(0, Ordering::SeqCst);
                let cj = (c.json_loadfd)(fd, 0, &mut ce);
                BUDGET.store(0, Ordering::SeqCst);
                let rj = (r.json_loadfd)(fd, 0, &mut re);
                BUDGET.store(-1, Ordering::SeqCst);
                let (co, ro) = (obs(cj, &ce), obs(rj, &re));
                cmp(&co, &ro, &format!("json_loadfd({fd}) source naming"));
                assert!(co.null);
                assert_eq!(co.source(), want, "json_loadfd({fd}) source");
                decref(c, cj);
                decref(r, rj);
            }
        });

        // A non-stdin FILE* must be named "<stream>" — the other side of the
        // same comparison, checked on the normal (non-OOM) path.
        let path = tmp_path("stdinname");
        let o = d_loadf_bytes(c, r, b"[", 0, &path, "non-stdin FILE* naming");
        assert_eq!(o.source(), "<stream>");
        let _ = std::fs::remove_file(&path);
    }
}

fn c_stdin_ptr() -> *mut FILE {
    extern "C" {
        static mut stdin: *mut FILE;
    }
    unsafe { stdin }
}

// ===========================================================================
// Randomised error-path fuzz
// ===========================================================================

/// Build a document that is malformed in one of the ways load.c has a distinct
/// error branch for, so a long randomised run keeps landing on error paths
/// rather than on the happy path.
fn gen_broken(rng: &mut Rng) -> Vec<u8> {
    let pieces: &[&[u8]] = &[
        b"[", b"]", b"{", b"}", b",", b":", b"\"", b"\\", b"1", b"01", b"-", b"1.", b"1e",
        b"1e+", b"9223372036854775808", b"-9223372036854775809", b"1e999", b"tru", b"nul",
        b"fals", b"@", b"#", b"\x80", b"\xff", b"\xc2", b"\xc0\x80", b"\xed\xa0\x80",
        b"\xef\xbb\xbf", b"\\u12", b"\\uZZZZ", b"\\uD800", b"\\uDC00", b"\\uD800\\u0041",
        b"\\u0000", b"\t", b"\n", b" ", b"\x01", b"\x1f", b"true", b"null", b"\"a\"",
        b"[1]", b"{\"a\":1}", b"1.5",
    ];
    let n = 1 + rng.below(9);
    let mut v = Vec::new();
    for _ in 0..n {
        v.extend_from_slice(rng.choice(pieces));
    }
    v
}

#[test]
fn randomised_malformed_input_error_images_agree() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Every flag combination of the five meaningful decoder bits, plus a
        // few with undefined bits mixed in, against many random broken inputs.
        let mut rng = Rng::new(0x0A11_E770);
        let path = tmp_path("fuzz");
        for i in 0..1400 {
            let doc = gen_broken(&mut rng);
            let base = (rng.next_u64() as size_t) & DECODE_FLAG_MASK;
            let flags = match rng.below(4) {
                0 => base,
                1 => base | 0x20,
                2 => base | 0x8000,
                _ => base | (rng.next_u64() as size_t & !DECODE_FLAG_MASK),
            };
            let ctx = format!("fuzz #{i}");
            // json_loads stops at the first interior NUL; json_loadb does not,
            // so both are worth running on the same bytes.
            d_loads(c, r, &doc, flags, &ctx);
            let p = if doc.is_empty() {
                b"".as_ptr() as *const c_char
            } else {
                doc.as_ptr() as *const c_char
            };
            d_loadb(c, r, p, doc.len(), flags, &ctx);
            // Every prefix length is a distinct truncation point.
            let cut = rng.below(doc.len() + 1);
            d_loadb(c, r, p, cut, flags, &format!("{ctx} truncated to {cut}"));
            if i % 7 == 0 {
                d_loadf_bytes(c, r, &doc, flags, &path, &ctx);
                d_loadfd_bytes(c, r, &doc, flags, &path, &ctx);
                d_load_file_bytes(c, r, &doc, flags, &path, &ctx);
                d_load_callback(c, r, Some(cb), &doc, CB_FEED, 0, flags, &ctx);
                d_load_callback(c, r, Some(cb), &doc, CB_TRUNCATE, cut, flags, &ctx);
            }
        }
        let _ = std::fs::remove_file(&path);
    }
}

#[test]
fn every_entry_point_agrees_on_a_table_of_malformed_documents() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // The same malformed inputs pushed through all six entry points. The
        // `source` field differs per entry point, so each comparison is
        // C-vs-Rust for that entry point — which is exactly the parity claim.
        let docs: &[&[u8]] = &[
            b"",
            b" ",
            b"[",
            b"]",
            b"{",
            b"}",
            b"[1",
            b"[1,",
            b"[1,]",
            b"[,]",
            b"{\"a\"}",
            b"{\"a\":}",
            b"{\"a\":1,}",
            b"{1:2}",
            b"[01]",
            b"[-]",
            b"[1.]",
            b"[1e]",
            b"[1e+]",
            b"[9223372036854775808]",
            b"[-9223372036854775809]",
            b"[1e999]",
            b"[tru]",
            b"[@]",
            b"[\"a",
            b"[\"a\\x\"]",
            b"[\"\\u12\"]",
            b"[\"\\uD800\"]",
            b"[\"\\uDC00\"]",
            b"[\"\\uD800\\u0041\"]",
            b"[\"a\tb\"]",
            b"[\"a\nb\"]",
            b"[\"a\\u0000b\"]",
            b"{\"a\\u0000b\":1}",
            b"[\"\xff\"]",
            b"[\"\xc2\x41\"]",
            b"[\"\xed\xa0\x80\"]",
            b"\xef\xbb\xbf[]",
            b"[1] x",
            b"1",
            b"\"x\"",
            b"true",
            b"nul",
            b"[\"\xc2",
        ];
        let path = tmp_path("matrix");
        for &flags in &[
            0usize,
            JSON_DECODE_ANY,
            JSON_DISABLE_EOF_CHECK,
            JSON_ALLOW_NUL,
            JSON_REJECT_DUPLICATES,
            JSON_DECODE_INT_AS_REAL,
            JSON_DECODE_ANY | JSON_ALLOW_NUL | JSON_DECODE_INT_AS_REAL,
        ] {
            for &t in docs {
                d_loads(c, r, t, flags, "entry-point matrix");
                let p = if t.is_empty() {
                    b"".as_ptr() as *const c_char
                } else {
                    t.as_ptr() as *const c_char
                };
                d_loadb(c, r, p, t.len(), flags, "entry-point matrix");
                d_loadf_bytes(c, r, t, flags, &path, "entry-point matrix");
                d_loadfd_bytes(c, r, t, flags, &path, "entry-point matrix");
                d_load_file_bytes(c, r, t, flags, &path, "entry-point matrix");
                d_load_callback(c, r, Some(cb), t, CB_FEED, 0, flags, "entry-point matrix");
                // One byte at a time exercises the stream refill path.
                d_load_callback(c, r, Some(cb), t, CB_TRUNCATE, 1, flags, "matrix 1 byte");
            }
        }
        let _ = std::fs::remove_file(&path);
    }
}
