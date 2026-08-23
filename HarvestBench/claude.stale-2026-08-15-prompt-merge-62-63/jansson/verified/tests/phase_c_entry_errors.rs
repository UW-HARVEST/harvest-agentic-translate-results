//! Phase C — C-vs-Rust differential tests for the LOAD and DUMP ENTRY POINTS.
//!
//! Everything here goes through the exported C ABI of BOTH `libjansson.so`
//! artifacts (see `tests/common/mod.rs`); no Rust function is ever called
//! directly. The C library is ground truth: where a row of `ERRORS.md` states an
//! absolute expectation (code / source / message) it is pinned against the C
//! with `pin_c*`, and `diff` then requires the Rust to agree exactly.
//!
//! `ERRORS.md` rows covered:
//!   * load argument validation .... 180, 183, 185, 187, 190, 192
//!   * load I/O + EOF paths ........ 182, 189, 191, 194
//!   * dump buffer / callback ...... 196, 232, 230, 240
//!   * dump I/O .................... 197, 198, 235, 237
//!   * dump root / precision ....... 238, 208
//!
//! NOTE on temp files: `diff` runs its closure ONCE PER LIBRARY, so a closure
//! that both writes and reads a fixed path would let the two runs interfere.
//! Input fixtures are therefore created *before* `diff` and only read inside it;
//! output paths are made unique per call via `unique_path`.

#![allow(dead_code)]

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

// ---------------------------------------------------------------- signatures

type FnLoadf = unsafe extern "C" fn(*mut c_void, usize, *mut json_error_t) -> *mut json_t;
type FnLoadfd = unsafe extern "C" fn(c_int, usize, *mut json_error_t) -> *mut json_t;
type FnLoadFile = unsafe extern "C" fn(*const c_char, usize, *mut json_error_t) -> *mut json_t;
type LoadCb = unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize;
type FnLoadCallback =
    unsafe extern "C" fn(Option<LoadCb>, *mut c_void, usize, *mut json_error_t) -> *mut json_t;

type FnDumpf = unsafe extern "C" fn(*const json_t, *mut c_void, usize) -> c_int;
type FnDumpfd = unsafe extern "C" fn(*const json_t, c_int, usize) -> c_int;
type FnDumpFile = unsafe extern "C" fn(*const json_t, *const c_char, usize) -> c_int;
type DumpCb = unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int;
type FnDumpCallback =
    unsafe extern "C" fn(*const json_t, Option<DumpCb>, *mut c_void, usize) -> c_int;

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------- utilities

fn tmp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("jansson_pce_{}_{}", std::process::id(), name))
}

static NEXT_PATH_ID: AtomicUsize = AtomicUsize::new(0);

/// A path that is unique per *call*. Two levels of interference have to be
/// avoided: `diff` calls its closure once for C and once for Rust, and the
/// `#[test]` fns themselves run in parallel threads — a shared output path would
/// let one run truncate/delete the file another run is about to read back.
fn unique_path(name: &str) -> PathBuf {
    let n = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    tmp_path(&format!("{}_{}", name, n))
}

unsafe fn fopen_mode(path: &Path, mode: &str) -> *mut c_void {
    let p = cs(path.to_str().unwrap());
    let m = cs(mode);
    fopen(p.as_ptr(), m.as_ptr())
}

/// `Some(errno)` if `fopen(path,"rb")` fails here too (so the library's
/// `json_load_file` will take the cannot-open-file branch with that errno),
/// `None` if the path actually opens.
unsafe fn fopen_rb_errno(path: &str) -> Option<c_int> {
    let p = cs(path);
    let m = cs("rb");
    *__errno_location() = 0;
    let f = fopen(p.as_ptr(), m.as_ptr());
    if f.is_null() {
        Some(*__errno_location())
    } else {
        fclose(f);
        None
    }
}

fn strerror_str(e: c_int) -> String {
    unsafe { cstr_to_string(strerror(e)) }
}

/// Replicates `jsonp_error_set_source()`: `< 80` bytes verbatim, otherwise
/// `"..."` followed by the last `JSON_ERROR_SOURCE_LENGTH - 4 == 76` bytes.
fn expected_source(path: &str) -> String {
    let b = path.as_bytes();
    if b.len() < JSON_ERROR_SOURCE_LENGTH {
        path.to_string()
    } else {
        let extra = b.len() - JSON_ERROR_SOURCE_LENGTH + 4;
        format!("...{}", &path[extra..])
    }
}

/// Replicates the double `vsnprintf` clamp in `error_set` + `jsonp_error_vset`:
/// the stored text is at most `JSON_ERROR_TEXT_LENGTH - 2 == 158` bytes.
fn clamp_text(s: &str) -> String {
    let mut t = s.to_string();
    if t.len() > JSON_ERROR_TEXT_LENGTH - 2 {
        t.truncate(JSON_ERROR_TEXT_LENGTH - 2);
    }
    t
}

fn expect_wrong_args(source: &str) -> ErrSnap {
    ErrSnap {
        line: -1,
        column: -1,
        position: 0,
        source: source.to_string(),
        text: "wrong arguments".to_string(),
        code: JSON_ERROR_INVALID_ARGUMENT,
    }
}

/// Pin the C library's result absolutely. C is ground truth: if this fires the
/// expectation (i.e. `ERRORS.md`) is wrong, not the library.
#[track_caller]
fn pin_c(label: &str, expected: &ErrSnap, f: impl Fn(&Library) -> ErrSnap) {
    let got = f(&libs().c);
    assert_eq!(&got, expected, "C ground truth mismatch [{}]", label);
}

/// Weaker pin for cases whose line/column depend on lexer position: only the
/// `ERRORS.md` claims (error code + stamped source) are asserted; full equality
/// between the two libraries is still enforced by `diff`.
#[track_caller]
fn pin_c_code_source(label: &str, code: i32, source: &str, f: impl Fn(&Library) -> ErrSnap) {
    let got = f(&libs().c);
    assert_eq!(
        (got.code, got.source.as_str()),
        (code, source),
        "C ground truth mismatch [{}] (full snapshot: {:?})",
        label,
        got
    );
}

unsafe fn parse_fixture(lib: &Library, text: &str) -> *mut json_t {
    let loads: Symbol<FnLoads> = sym(lib, "json_loads");
    let cp = cs(text);
    let mut err = json_error_t::new();
    let j = loads(cp.as_ptr(), 0, &mut err);
    assert!(!j.is_null(), "fixture must parse: {} ({:?})", text, err.snapshot());
    j
}

// ---------------------------------------------------------------- load probes

/// Result of an entry point called with a bad argument, both with a real
/// `json_error_t` and with `error == NULL` (which must not crash).
#[derive(PartialEq, Eq, Debug)]
struct ArgErr {
    returned_null: bool,
    null_error_returned_null: bool,
    snap: ErrSnap,
}

const LOAD_FLAG_SETS: &[usize] = &[
    0,
    JSON_DECODE_ANY,
    JSON_DISABLE_EOF_CHECK,
    JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
    JSON_REJECT_DUPLICATES | JSON_DECODE_INT_AS_REAL | JSON_ALLOW_NUL,
    usize::MAX,
];

#[test]
fn row180_json_loads_null_string() {
    for &flags in LOAD_FLAG_SETS {
        let label = format!("row180/json_loads(NULL, {:#x})", flags);
        let probe = move |lib: &Library| unsafe {
            let f: Symbol<FnLoads> = sym(lib, "json_loads");
            let mut err = json_error_t::new();
            let r = f(ptr::null(), flags, &mut err);
            let r2 = f(ptr::null(), flags, ptr::null_mut());
            ArgErr {
                returned_null: r.is_null(),
                null_error_returned_null: r2.is_null(),
                snap: err.snapshot(),
            }
        };
        diff(&label, &probe);
        pin_c(&label, &expect_wrong_args("<string>"), |lib| probe(lib).snap);
    }
}

#[test]
fn row183_json_loadb_null_buffer() {
    for &flags in LOAD_FLAG_SETS {
        for &buflen in &[0usize, 1, 5, usize::MAX] {
            let label = format!("row183/json_loadb(NULL, {}, {:#x})", buflen, flags);
            let probe = move |lib: &Library| unsafe {
                let f: Symbol<FnLoadb> = sym(lib, "json_loadb");
                let mut err = json_error_t::new();
                let r = f(ptr::null(), buflen, flags, &mut err);
                let r2 = f(ptr::null(), buflen, flags, ptr::null_mut());
                ArgErr {
                    returned_null: r.is_null(),
                    null_error_returned_null: r2.is_null(),
                    snap: err.snapshot(),
                }
            };
            diff(&label, &probe);
            pin_c(&label, &expect_wrong_args("<buffer>"), |lib| probe(lib).snap);
        }
    }
}

#[test]
fn row185_json_loadf_null_stream() {
    for &flags in LOAD_FLAG_SETS {
        let label = format!("row185/json_loadf(NULL, {:#x})", flags);
        let probe = move |lib: &Library| unsafe {
            let f: Symbol<FnLoadf> = sym(lib, "json_loadf");
            let mut err = json_error_t::new();
            let r = f(ptr::null_mut(), flags, &mut err);
            let r2 = f(ptr::null_mut(), flags, ptr::null_mut());
            ArgErr {
                returned_null: r.is_null(),
                null_error_returned_null: r2.is_null(),
                snap: err.snapshot(),
            }
        };
        diff(&label, &probe);
        // NULL != stdin, so the source is "<stream>", stamped before the check.
        pin_c(&label, &expect_wrong_args("<stream>"), |lib| probe(lib).snap);
    }
}

#[test]
fn row187_json_loadfd_negative_fd() {
    for &fd in &[-1, -2, -3, -100, c_int::MIN + 1, c_int::MIN] {
        for &flags in LOAD_FLAG_SETS {
            let label = format!("row187/json_loadfd({}, {:#x})", fd, flags);
            let probe = move |lib: &Library| unsafe {
                let f: Symbol<FnLoadfd> = sym(lib, "json_loadfd");
                let mut err = json_error_t::new();
                let r = f(fd, flags, &mut err);
                let r2 = f(fd, flags, ptr::null_mut());
                ArgErr {
                    returned_null: r.is_null(),
                    null_error_returned_null: r2.is_null(),
                    snap: err.snapshot(),
                }
            };
            diff(&label, &probe);
            pin_c(&label, &expect_wrong_args("<stream>"), |lib| probe(lib).snap);
        }
    }
}

#[test]
fn row190_json_load_file_null_path() {
    for &flags in LOAD_FLAG_SETS {
        let label = format!("row190/json_load_file(NULL, {:#x})", flags);
        let probe = move |lib: &Library| unsafe {
            let f: Symbol<FnLoadFile> = sym(lib, "json_load_file");
            let mut err = json_error_t::new();
            let r = f(ptr::null(), flags, &mut err);
            let r2 = f(ptr::null(), flags, ptr::null_mut());
            ArgErr {
                returned_null: r.is_null(),
                null_error_returned_null: r2.is_null(),
                snap: err.snapshot(),
            }
        };
        diff(&label, &probe);
        // jsonp_error_init(error, NULL) leaves source empty (source[0] = '\0').
        pin_c(&label, &expect_wrong_args(""), |lib| probe(lib).snap);
    }
}

#[test]
fn row192_json_load_callback_null_callback() {
    let mut junk: u64 = 0xdead_beef;
    let junk_ptr = &mut junk as *mut u64 as *mut c_void;
    for &flags in LOAD_FLAG_SETS {
        for &arg in &[ptr::null_mut(), junk_ptr] {
            let label = format!("row192/json_load_callback(NULL, {:?}, {:#x})", arg, flags);
            let probe = move |lib: &Library| unsafe {
                let f: Symbol<FnLoadCallback> = sym(lib, "json_load_callback");
                let mut err = json_error_t::new();
                let r = f(None, arg, flags, &mut err);
                let r2 = f(None, arg, flags, ptr::null_mut());
                ArgErr {
                    returned_null: r.is_null(),
                    null_error_returned_null: r2.is_null(),
                    snap: err.snapshot(),
                }
            };
            diff(&label, &probe);
            pin_c(&label, &expect_wrong_args("<callback>"), |lib| probe(lib).snap);
        }
    }
}

/// Every load entry point stamps its own distinctive `source` string; this pins
/// the whole set at once and asserts they really are distinct.
#[test]
fn rows180_192_argument_error_sources_are_distinct() {
    fn all(lib: &Library) -> Vec<(&'static str, ErrSnap)> {
        unsafe {
            let mut v: Vec<(&'static str, ErrSnap)> = Vec::new();
            {
                let f: Symbol<FnLoads> = sym(lib, "json_loads");
                let mut e = json_error_t::new();
                assert!(f(ptr::null(), 0, &mut e).is_null());
                v.push(("json_loads", e.snapshot()));
            }
            {
                let f: Symbol<FnLoadb> = sym(lib, "json_loadb");
                let mut e = json_error_t::new();
                assert!(f(ptr::null(), 0, 0, &mut e).is_null());
                v.push(("json_loadb", e.snapshot()));
            }
            {
                let f: Symbol<FnLoadf> = sym(lib, "json_loadf");
                let mut e = json_error_t::new();
                assert!(f(ptr::null_mut(), 0, &mut e).is_null());
                v.push(("json_loadf", e.snapshot()));
            }
            {
                let f: Symbol<FnLoadfd> = sym(lib, "json_loadfd");
                let mut e = json_error_t::new();
                assert!(f(-1, 0, &mut e).is_null());
                v.push(("json_loadfd", e.snapshot()));
            }
            {
                let f: Symbol<FnLoadFile> = sym(lib, "json_load_file");
                let mut e = json_error_t::new();
                assert!(f(ptr::null(), 0, &mut e).is_null());
                v.push(("json_load_file", e.snapshot()));
            }
            {
                let f: Symbol<FnLoadCallback> = sym(lib, "json_load_callback");
                let mut e = json_error_t::new();
                assert!(f(None, ptr::null_mut(), 0, &mut e).is_null());
                v.push(("json_load_callback", e.snapshot()));
            }
            v
        }
    }

    diff("rows180-192/all argument errors", all);

    let c = all(&libs().c);
    let expected = [
        ("json_loads", "<string>"),
        ("json_loadb", "<buffer>"),
        ("json_loadf", "<stream>"),
        ("json_loadfd", "<stream>"),
        ("json_load_file", ""),
        ("json_load_callback", "<callback>"),
    ];
    for (i, (name, source)) in expected.iter().enumerate() {
        assert_eq!(&c[i].0, name);
        assert_eq!(&c[i].1, &expect_wrong_args(source), "source for {}", name);
    }
    // json_loadf and json_loadfd deliberately share "<stream>"; the other four
    // must all be different from each other and from "<stream>".
    let uniq: std::collections::BTreeSet<&str> =
        c.iter().map(|(_, s)| s.source.as_str()).collect();
    assert_eq!(uniq.len(), 5, "distinct sources: {:?}", uniq);
}

// ------------------------------------------------- row 191: cannot open file

#[test]
fn row191_json_load_file_cannot_open() {
    let missing = tmp_path("row191_missing.json");
    let _ = std::fs::remove_file(&missing);
    let no_parent = tmp_path("row191_no_such_dir/inner/file.json");
    let noperm = tmp_path("row191_noperm.json");
    std::fs::write(&noperm, b"[1,2]").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&noperm, std::fs::Permissions::from_mode(0o000)).unwrap();
    }

    let cases: Vec<(String, String)> = vec![
        ("nonexistent file".into(), missing.to_str().unwrap().to_string()),
        ("nonexistent parent dir".into(), no_parent.to_str().unwrap().to_string()),
        ("directory".into(), "/tmp".to_string()),
        ("unreadable file".into(), noperm.to_str().unwrap().to_string()),
        ("empty path".into(), "".to_string()),
    ];

    for (what, path) in &cases {
        for &flags in &[0usize, JSON_DECODE_ANY] {
            let label = format!("row191/json_load_file({:?}) [{}] flags={:#x}", path, what, flags);
            let p = path.clone();
            let probe = move |lib: &Library| unsafe {
                let f: Symbol<FnLoadFile> = sym(lib, "json_load_file");
                let cp = cs(&p);
                let mut err = json_error_t::new();
                let j = f(cp.as_ptr(), flags, &mut err);
                let dumped =
                    if j.is_null() { None } else { dumps_to_string(lib, j, JSON_SORT_KEYS) };
                decref(lib, j);
                let j2 = f(cp.as_ptr(), flags, ptr::null_mut());
                let j2_null = j2.is_null();
                decref(lib, j2);
                (j.is_null(), j2_null, dumped, err.snapshot())
            };
            diff(&label, &probe);

            // Absolute pin, but only when fopen(path,"rb") really fails here
            // too. NOTE: on glibc, opening a DIRECTORY read-only SUCCEEDS, so
            // "/tmp" does not take the cannot-open branch at all; and if the
            // suite runs as root the unreadable file opens as well.
            if let Some(errno) = unsafe { fopen_rb_errno(path) } {
                let expected = ErrSnap {
                    line: -1,
                    column: -1,
                    position: 0,
                    source: expected_source(path),
                    text: clamp_text(&format!(
                        "unable to open {}: {}",
                        path,
                        strerror_str(errno)
                    )),
                    code: JSON_ERROR_CANNOT_OPEN_FILE,
                };
                pin_c(&label, &expected, |lib| probe(lib).3);
            } else {
                // Opened fine -> the failure comes from reading instead, and the
                // path-derived source is GONE: json_load_file hands the same
                // json_error_t to json_loadf, whose own jsonp_error_init()
                // overwrites source with "<stream>". The path only survives in
                // `source` on the cannot-open branch.
                pin_c_code_source(
                    &label,
                    JSON_ERROR_PREMATURE_END_OF_INPUT,
                    "<stream>",
                    |lib| probe(lib).3,
                );
            }
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&noperm, std::fs::Permissions::from_mode(0o644));
    }
    let _ = std::fs::remove_file(&noperm);
}

/// `JSON_ERROR_SOURCE_LENGTH` is 80: a source of length <= 79 is stored
/// verbatim, length >= 80 becomes `"..."` + the last 76 bytes (79 chars total).
#[test]
fn row191_source_truncation_boundary() {
    let lens = [40usize, 70, 77, 78, 79, 80, 81, 82, 100, 200];
    let mut measured: Vec<(usize, usize, bool)> = Vec::new();

    for &n in &lens {
        assert!(n > 5);
        let path = format!("/tmp/{}", "q".repeat(n - 5));
        assert_eq!(path.len(), n);
        assert!(!Path::new(&path).exists(), "fixture path must not exist");

        let label = format!("row191/source truncation len={}", n);
        let p = path.clone();
        let probe = move |lib: &Library| unsafe {
            let f: Symbol<FnLoadFile> = sym(lib, "json_load_file");
            let cp = cs(&p);
            let mut err = json_error_t::new();
            let j = f(cp.as_ptr(), 0, &mut err);
            assert!(j.is_null());
            err.snapshot()
        };
        diff(&label, &probe);

        let errno = unsafe { fopen_rb_errno(&path) }.expect("path must not be openable");
        let expected = ErrSnap {
            line: -1,
            column: -1,
            position: 0,
            source: expected_source(&path),
            text: clamp_text(&format!("unable to open {}: {}", path, strerror_str(errno))),
            code: JSON_ERROR_CANNOT_OPEN_FILE,
        };
        pin_c(&label, &expected, &probe);

        let src = probe(&libs().c).source;
        let truncated = src != path;
        if truncated {
            assert!(src.starts_with("..."), "truncated source must start with ...");
            assert_eq!(src.len(), 79, "truncated source length");
            assert_eq!(&src[3..], &path[path.len() - 76..], "truncated tail");
        }
        measured.push((n, src.len(), truncated));
    }

    // The boundary: verbatim up to and including 79, truncated from 80 on.
    for &(n, len, truncated) in &measured {
        if n < JSON_ERROR_SOURCE_LENGTH {
            assert!(!truncated, "len {} must not be truncated (got {})", n, len);
            assert_eq!(len, n);
        } else {
            assert!(truncated, "len {} must be truncated", n);
            assert_eq!(len, 79);
        }
    }
    eprintln!("[row191] source truncation boundary measured: {:?}", measured);
}

// ------------------------------------------------- row 182: embedded NUL

#[test]
fn row182_json_loads_embedded_nul_vs_loadb() {
    const CASES: &[&[u8]] = &[
        b"[1,2]\0[3]",
        b"[1,\0 2]",
        b"\0",
        b"\0[1]",
        b"[]\0",
        b"[1,2]\0",
        b"{\"a\":1}\0{\"b\":2}",
        b"[\"a\0b\"]",
        b"[1,2\0]",
        b"1\02",
    ];
    for &flags in &[0usize, JSON_DISABLE_EOF_CHECK, JSON_DECODE_ANY, JSON_DECODE_ANY | JSON_ALLOW_NUL] {
        for &buf in CASES {
            let label = format!("row182/{:?} flags={:#x}", buf, flags);
            let probe = move |lib: &Library| unsafe {
                // json_loads: string_get() returns EOF at the first NUL.
                let terminated = cs_bytes(buf);
                let loads: Symbol<FnLoads> = sym(lib, "json_loads");
                let mut e1 = json_error_t::new();
                let j1 = loads(terminated.as_ptr() as *const c_char, flags, &mut e1);
                let d1 = if j1.is_null() {
                    None
                } else {
                    dumps_to_string(lib, j1, JSON_ENCODE_ANY | JSON_SORT_KEYS)
                };
                decref(lib, j1);

                // json_loadb with a buflen spanning the NUL: the NUL is data.
                let loadb: Symbol<FnLoadb> = sym(lib, "json_loadb");
                let mut e2 = json_error_t::new();
                let j2 = loadb(buf.as_ptr() as *const c_char, buf.len(), flags, &mut e2);
                let d2 = if j2.is_null() {
                    None
                } else {
                    dumps_to_string(lib, j2, JSON_ENCODE_ANY | JSON_SORT_KEYS)
                };
                decref(lib, j2);

                (d1, e1.snapshot(), d2, e2.snapshot())
            };
            diff(&label, &probe);
        }
    }

    // Pin the two headline behaviours of the row on the C.
    let c = &libs().c;
    unsafe {
        let loads: Symbol<FnLoads> = sym(c, "json_loads");
        let loadb: Symbol<FnLoadb> = sym(c, "json_loadb");

        // "[1,2]\0[3]": json_loads stops at the NUL and SUCCEEDS.
        let t = cs_bytes(b"[1,2]\0[3]");
        let mut e = json_error_t::new();
        let j = loads(t.as_ptr() as *const c_char, 0, &mut e);
        assert!(!j.is_null(), "json_loads must stop at the NUL: {:?}", e.snapshot());
        assert_eq!(dumps_to_string(c, j, 0).as_deref(), Some("[1, 2]"));
        decref(c, j);

        // ...whereas json_loadb sees the NUL as trailing garbage.
        let raw = b"[1,2]\0[3]";
        let mut e = json_error_t::new();
        let j = loadb(raw.as_ptr() as *const c_char, raw.len(), 0, &mut e);
        assert!(j.is_null(), "json_loadb must reject the trailing NUL");
        assert_eq!(e.snapshot().code, JSON_ERROR_END_OF_INPUT_EXPECTED);

        // A NUL *inside* a document is a premature end for json_loads.
        let t = cs_bytes(b"[1,\0 2]");
        let mut e = json_error_t::new();
        let j = loads(t.as_ptr() as *const c_char, 0, &mut e);
        assert!(j.is_null());
        assert_eq!(e.snapshot().code, JSON_ERROR_PREMATURE_END_OF_INPUT);
        assert_eq!(e.snapshot().source, "<string>");
    }
}

// ------------------------------------------------- row 189: loadfd read fails

unsafe fn loadfd_probe(lib: &Library, fd: c_int, flags: usize) -> (bool, Option<String>, ErrSnap) {
    let f: Symbol<FnLoadfd> = sym(lib, "json_loadfd");
    let mut err = json_error_t::new();
    let j = f(fd, flags, &mut err);
    let d = if j.is_null() { None } else { dumps_to_string(lib, j, JSON_SORT_KEYS) };
    let was_null = j.is_null();
    decref(lib, j);
    (was_null, d, err.snapshot())
}

/// A private high descriptor slot per call, so two `#[test]` threads never
/// dup2() onto the same number.
static NEXT_HI_FD: AtomicI32 = AtomicI32::new(700);

/// Produce a genuinely stale (opened-then-closed) descriptor. The number is
/// moved to a high, privately reserved slot first: the kernel hands out the
/// LOWEST free descriptor, so a high number will not be recycled by the other
/// tests running in parallel threads of this same binary.
unsafe fn stale_high_fd(path: &Path) -> c_int {
    let f = std::fs::File::open(path).unwrap();
    let src = f.as_raw_fd();
    let mut chosen = src;
    for _ in 0..8 {
        let cand = NEXT_HI_FD.fetch_add(1, Ordering::Relaxed);
        if dup2(src, cand) == cand {
            chosen = cand;
            break;
        }
    }
    drop(f); // closes src
    if chosen != src {
        close(chosen);
    }
    chosen
}

#[test]
fn row189_json_loadfd_read_failures() {
    // Fixtures created BEFORE diff; the closures only read them.
    let good = tmp_path("row189_good.json");
    std::fs::write(&good, b"[1,2,{\"a\":3}]\n").unwrap();
    let wo = tmp_path("row189_writeonly.json");
    std::fs::write(&wo, b"[9]").unwrap();

    for &flags in &[0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK] {
        // (a) a high descriptor that was never opened -> read() = EBADF -> EOF
        let label = format!("row189/never-opened fd flags={:#x}", flags);
        let probe = move |lib: &Library| unsafe { loadfd_probe(lib, 4096, flags) };
        diff(&label, &probe);
        pin_c_code_source(&label, JSON_ERROR_PREMATURE_END_OF_INPUT, "<stream>", |lib| {
            probe(lib).2
        });

        // (b) a genuinely stale descriptor (opened, then closed)
        let label = format!("row189/stale closed fd flags={:#x}", flags);
        let g = good.clone();
        let probe = move |lib: &Library| unsafe {
            let fd = stale_high_fd(&g);
            loadfd_probe(lib, fd, flags)
        };
        diff(&label, &probe);
        pin_c_code_source(&label, JSON_ERROR_PREMATURE_END_OF_INPUT, "<stream>", |lib| {
            probe(lib).2
        });

        // (c) a descriptor opened WRITE-ONLY -> read() = EBADF -> EOF
        let label = format!("row189/write-only fd flags={:#x}", flags);
        let w = wo.clone();
        let probe = move |lib: &Library| unsafe {
            let f = std::fs::OpenOptions::new().write(true).create(true).open(&w).unwrap();
            let out = loadfd_probe(lib, f.as_raw_fd(), flags);
            drop(f);
            out
        };
        diff(&label, &probe);
        pin_c_code_source(&label, JSON_ERROR_PREMATURE_END_OF_INPUT, "<stream>", |lib| {
            probe(lib).2
        });

        // (d) contrast: a real readable descriptor must SUCCEED
        let label = format!("row189/valid readable fd flags={:#x}", flags);
        let g = good.clone();
        let probe = move |lib: &Library| unsafe {
            let f = std::fs::File::open(&g).unwrap();
            let out = loadfd_probe(lib, f.as_raw_fd(), flags);
            drop(f);
            out
        };
        diff(&label, &probe);
        let (was_null, dumped, _) = probe(&libs().c);
        assert!(!was_null, "valid fd must parse");
        assert_eq!(dumped.as_deref(), Some("[1, 2, {\"a\": 3}]"));

        // (e) an empty (but valid) descriptor -> premature end
        let empty = tmp_path("row189_empty.json");
        std::fs::write(&empty, b"").unwrap();
        let label = format!("row189/empty file fd flags={:#x}", flags);
        let e = empty.clone();
        let probe = move |lib: &Library| unsafe {
            let f = std::fs::File::open(&e).unwrap();
            let out = loadfd_probe(lib, f.as_raw_fd(), flags);
            drop(f);
            out
        };
        diff(&label, &probe);
        let _ = std::fs::remove_file(&empty);
    }

    let _ = std::fs::remove_file(&good);
    let _ = std::fs::remove_file(&wo);
}

// ------------------------------------------------- row 194: load callback

struct LoadState {
    doc: *const u8,
    len: usize,
    pos: usize,
    chunk: usize,
    calls: usize,
    /// call index (0-based) from which `fail_val` is returned
    fail_at: usize,
    fail_val: usize,
}

unsafe extern "C" fn load_feed(buf: *mut c_void, buflen: usize, data: *mut c_void) -> usize {
    let st = data as *mut LoadState;
    let call = (*st).calls;
    (*st).calls += 1;
    if call >= (*st).fail_at {
        return (*st).fail_val;
    }
    let remaining = (*st).len - (*st).pos;
    let mut n = (*st).chunk.min(buflen);
    if n > remaining {
        n = remaining;
    }
    if n > 0 {
        ptr::copy_nonoverlapping((*st).doc.add((*st).pos), buf as *mut u8, n);
        (*st).pos += n;
    }
    n
}

#[derive(PartialEq, Debug)]
struct CbLoad {
    was_null: bool,
    dumped: Option<String>,
    snap: ErrSnap,
    calls: usize,
    fed: usize,
}

unsafe fn load_callback_probe(
    lib: &Library,
    doc: &[u8],
    chunk: usize,
    fail_at: usize,
    fail_val: usize,
    flags: usize,
) -> CbLoad {
    let mut st = LoadState {
        doc: doc.as_ptr(),
        len: doc.len(),
        pos: 0,
        chunk,
        calls: 0,
        fail_at,
        fail_val,
    };
    let f: Symbol<FnLoadCallback> = sym(lib, "json_load_callback");
    let mut err = json_error_t::new();
    let j = f(Some(load_feed), &mut st as *mut _ as *mut c_void, flags, &mut err);
    let dumped = if j.is_null() {
        None
    } else {
        dumps_to_string(lib, j, JSON_ENCODE_ANY | JSON_SORT_KEYS)
    };
    let was_null = j.is_null();
    decref(lib, j);
    CbLoad { was_null, dumped, snap: err.snapshot(), calls: st.calls, fed: st.pos }
}

const CB_DOC: &[u8] = br#"[1,2,3,{"k":[true,false,null,"hi"],"z":-4.5}]"#;

#[test]
fn row194_json_load_callback_eof_paths() {
    // (chunk, fail_at, fail_val, what)
    let cases: &[(usize, usize, usize, &str)] = &[
        (1024, 0, 0, "returns 0 immediately"),
        (1024, 0, usize::MAX, "returns (size_t)-1 immediately"),
        (3, 1, 0, "returns 0 mid-document"),
        (3, 1, usize::MAX, "returns (size_t)-1 mid-document"),
        (5, 2, 0, "returns 0 after 2 chunks"),
        (5, 2, usize::MAX, "returns (size_t)-1 after 2 chunks"),
        (1, 10, 0, "returns 0 after 10 single bytes"),
        (1, 10, usize::MAX, "returns (size_t)-1 after 10 single bytes"),
        // whole document delivered in call 0, then a normal 0 -> SUCCESS
        (1024, usize::MAX, 0, "well-behaved single chunk"),
    ];
    for &(chunk, fail_at, fail_val, what) in cases {
        for &flags in &[0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK] {
            let label =
                format!("row194/{} chunk={} flags={:#x}", what, chunk, flags);
            let probe = move |lib: &Library| unsafe {
                load_callback_probe(lib, CB_DOC, chunk, fail_at, fail_val, flags)
            };
            diff(&label, &probe);
            if fail_at != usize::MAX {
                let got = probe(&libs().c);
                assert!(got.was_null, "[{}] must fail", label);
                pin_c_code_source(
                    &label,
                    JSON_ERROR_PREMATURE_END_OF_INPUT,
                    "<callback>",
                    |lib| probe(lib).snap,
                );
            }
        }
    }
}

#[test]
fn row194_json_load_callback_chunked_success() {
    for &chunk in &[1usize, 2, 3, 7, 16, 1024, 4096] {
        let label = format!("row194/well-behaved chunk={}", chunk);
        let probe =
            move |lib: &Library| unsafe {
                load_callback_probe(lib, CB_DOC, chunk, usize::MAX, 0, 0)
            };
        diff(&label, &probe);
        let got = probe(&libs().c);
        assert!(!got.was_null, "[{}] must succeed: {:?}", label, got.snap);
        assert_eq!(
            got.dumped.as_deref(),
            Some("[1, 2, 3, {\"k\": [true, false, null, \"hi\"], \"z\": -4.5}]"),
            "[{}]",
            label
        );
        assert_eq!(got.fed, CB_DOC.len(), "[{}] whole document consumed", label);
    }
}

#[test]
fn row194_json_load_callback_utf8_straddling_chunks() {
    // Multi-byte sequences of 2, 3 and 4 bytes, so every chunk size below
    // splits at least one of them.
    let doc = "[\"\u{e9}\u{df}\", \"\u{65e5}\u{672c}\u{8a9e}\u{20ac}\", \"\u{1d11e}\u{1f600}\"]"
        .as_bytes()
        .to_vec();
    for &chunk in &[1usize, 2, 3, 4, 5, 7, 1024] {
        for &flags in &[0usize, JSON_ENSURE_ASCII_LOAD_NOOP] {
            let label = format!("row194/utf8 straddle chunk={} flags={:#x}", chunk, flags);
            let d = doc.clone();
            let probe = move |lib: &Library| unsafe {
                load_callback_probe(lib, &d, chunk, usize::MAX, 0, flags)
            };
            diff(&label, &probe);
            let got = probe(&libs().c);
            assert!(!got.was_null, "[{}] must succeed: {:?}", label, got.snap);
        }
    }

    // Truncating a multi-byte sequence at the callback level is a hard error.
    let truncated = b"[\"\xe6\x97".to_vec(); // start of U+65E5, cut short
    for &chunk in &[1usize, 2, 1024] {
        let label = format!("row194/truncated utf8 chunk={}", chunk);
        let d = truncated.clone();
        let probe =
            move |lib: &Library| unsafe { load_callback_probe(lib, &d, chunk, usize::MAX, 0, 0) };
        diff(&label, &probe);
        assert!(probe(&libs().c).was_null);
    }
}

/// `JSON_ENSURE_ASCII` is an *encoder* flag; passing it to a loader must be
/// ignored. Used above only to prove the loader ignores unknown bits.
const JSON_ENSURE_ASCII_LOAD_NOOP: usize = JSON_ENSURE_ASCII;

// ---------------------------------------------------------------- dump side

struct DumpSink {
    out: Vec<u8>,
    calls: usize,
    /// call index (0-based) from which the callback returns non-zero
    fail_at: usize,
}

impl DumpSink {
    fn new() -> Self {
        DumpSink { out: Vec::new(), calls: 0, fail_at: usize::MAX }
    }
    fn failing_at(k: usize) -> Self {
        DumpSink { out: Vec::new(), calls: 0, fail_at: k }
    }
}

unsafe extern "C" fn dump_collect(buffer: *const c_char, size: usize, data: *mut c_void) -> c_int {
    let st = data as *mut DumpSink;
    let call = (*st).calls;
    (*st).calls += 1;
    if call >= (*st).fail_at {
        return 1;
    }
    if size > 0 {
        (*st).out.extend_from_slice(std::slice::from_raw_parts(buffer as *const u8, size));
    }
    0
}

/// Every dump entry point exercised on the same value, so each one's own
/// failure value is pinned side by side.
#[derive(PartialEq, Debug)]
struct AllDumpOut {
    rc_callback: c_int,
    cb_bytes: Vec<u8>,
    cb_calls: usize,
    dumps: Option<String>,
    dumpb_ret: usize,
    dumpb_buf: [u8; 96],
    rc_dumpf: c_int,
    dumpf_bytes: Vec<u8>,
    rc_dumpfd: c_int,
    dumpfd_bytes: Vec<u8>,
    rc_dump_file: c_int,
    dump_file_bytes: Vec<u8>,
}

unsafe fn all_dump_entry_points(
    lib: &Library,
    j: *const json_t,
    flags: usize,
) -> AllDumpOut {
    let mut sink = DumpSink::new();
    let dcb: Symbol<FnDumpCallback> = sym(lib, "json_dump_callback");
    let rc_callback = dcb(j, Some(dump_collect), &mut sink as *mut _ as *mut c_void, flags);

    let dumps = dumps_to_string(lib, j, flags);

    let dumpb: Symbol<FnDumpb> = sym(lib, "json_dumpb");
    let mut buf = [0xAAu8; 96];
    let dumpb_ret = dumpb(j, buf.as_mut_ptr() as *mut c_char, buf.len(), flags);

    // json_dumpf into a freshly created, writable FILE*
    let pf = unique_path("alldump_f");
    let _ = std::fs::remove_file(&pf);
    let fh = fopen_mode(&pf, "w");
    assert!(!fh.is_null(), "cannot create {:?}", pf);
    let dumpf: Symbol<FnDumpf> = sym(lib, "json_dumpf");
    let rc_dumpf = dumpf(j, fh, flags);
    fclose(fh);
    let dumpf_bytes = std::fs::read(&pf).unwrap_or_default();
    let _ = std::fs::remove_file(&pf);

    // json_dumpfd into a freshly created, writable fd
    let pfd = unique_path("alldump_fd");
    let _ = std::fs::remove_file(&pfd);
    let file = std::fs::File::create(&pfd).unwrap();
    let dumpfd: Symbol<FnDumpfd> = sym(lib, "json_dumpfd");
    let rc_dumpfd = dumpfd(j, file.as_raw_fd(), flags);
    drop(file);
    let dumpfd_bytes = std::fs::read(&pfd).unwrap_or_default();
    let _ = std::fs::remove_file(&pfd);

    // json_dump_file
    let pfile = unique_path("alldump_file");
    let _ = std::fs::remove_file(&pfile);
    let cp = cs(pfile.to_str().unwrap());
    let dfile: Symbol<FnDumpFile> = sym(lib, "json_dump_file");
    let rc_dump_file = dfile(j, cp.as_ptr(), flags);
    let dump_file_bytes = std::fs::read(&pfile).unwrap_or_default();
    let _ = std::fs::remove_file(&pfile);

    AllDumpOut {
        rc_callback,
        cb_bytes: sink.out,
        cb_calls: sink.calls,
        dumps,
        dumpb_ret,
        dumpb_buf: buf,
        rc_dumpf,
        dumpf_bytes,
        rc_dumpfd,
        dumpfd_bytes,
        rc_dump_file,
        dump_file_bytes,
    }
}

unsafe fn make_root(lib: &Library, kind: u32) -> *mut json_t {
    match kind {
        0 => ptr::null_mut(),
        1 => {
            let f: Symbol<FnVoidPtr> = sym(lib, "json_null");
            f()
        }
        2 => {
            let f: Symbol<FnVoidPtr> = sym(lib, "json_true");
            f()
        }
        3 => {
            let f: Symbol<FnVoidPtr> = sym(lib, "json_false");
            f()
        }
        4 => {
            let f: Symbol<FnInt> = sym(lib, "json_integer");
            f(-42)
        }
        5 => {
            let f: Symbol<FnReal> = sym(lib, "json_real");
            f(1.5)
        }
        _ => {
            let f: Symbol<FnStr> = sym(lib, "json_string");
            let s = cs("hi");
            f(s.as_ptr())
        }
    }
}

const ROOT_NAMES: &[&str] =
    &["NULL", "json_null", "json_true", "json_false", "json_integer", "json_real", "json_string"];

#[test]
fn row238_non_container_root_every_entry_point() {
    for kind in 0..7u32 {
        for &flags in &[
            0usize,
            JSON_COMPACT,
            JSON_SORT_KEYS,
            json_indent(2),
            JSON_ENCODE_ANY,
            JSON_ENCODE_ANY | JSON_COMPACT,
        ] {
            let label =
                format!("row238/root={} flags={:#x}", ROOT_NAMES[kind as usize], flags);
            let probe = |lib: &Library| unsafe {
                let j = make_root(lib, kind);
                let out = all_dump_entry_points(lib, j, flags);
                decref(lib, j);
                out
            };
            diff(&label, &probe);

            let got = probe(&libs().c);
            let must_fail = kind == 0 || (flags & JSON_ENCODE_ANY) == 0;
            if must_fail {
                // Each entry point reports failure in its OWN way.
                assert_eq!(got.rc_callback, -1, "[{}] json_dump_callback", label);
                assert_eq!(got.dumps, None, "[{}] json_dumps", label);
                assert_eq!(got.dumpb_ret, 0, "[{}] json_dumpb", label);
                assert_eq!(got.rc_dumpf, -1, "[{}] json_dumpf", label);
                assert_eq!(got.rc_dumpfd, -1, "[{}] json_dumpfd", label);
                assert_eq!(got.rc_dump_file, -1, "[{}] json_dump_file", label);
                assert!(got.cb_bytes.is_empty(), "[{}] nothing emitted", label);
                assert_eq!(got.cb_calls, 0, "[{}] callback never invoked", label);
                assert!(got.dumpb_buf.iter().all(|&b| b == 0xAA), "[{}] buffer untouched", label);
                assert!(got.dumpf_bytes.is_empty(), "[{}] file untouched", label);
                assert!(got.dumpfd_bytes.is_empty(), "[{}] fd untouched", label);
                assert!(got.dump_file_bytes.is_empty(), "[{}] dump_file untouched", label);
            } else {
                assert_eq!(got.rc_callback, 0, "[{}] must succeed", label);
                assert!(got.dumps.is_some(), "[{}] must succeed", label);
                assert_eq!(got.rc_dumpf, 0);
                assert_eq!(got.rc_dumpfd, 0);
                assert_eq!(got.rc_dump_file, 0);
            }
        }
    }
}

// --------------------------------- rows 196/232: json_dumpb short buffers

#[test]
fn rows196_232_json_dumpb_truncates_silently() {
    const DOCS: &[&str] = &[
        "[]",
        "{}",
        "[1,2,3]",
        r#"{"a":1,"b":[2,3],"c":"xyz"}"#,
        r#"[[[["deep"]]]]"#,
        r#"[1.5,true,false,null,"é"]"#,
        r#"{"long key here":"a longer value string"}"#,
    ];
    for doc in DOCS {
        for &flags in &[
            0usize,
            JSON_SORT_KEYS,
            JSON_COMPACT | JSON_SORT_KEYS,
            json_indent(4) | JSON_SORT_KEYS,
            JSON_ENSURE_ASCII | JSON_SORT_KEYS,
        ] {
            let label = format!("rows196,232/json_dumpb({}, flags={:#x})", doc, flags);
            let probe = move |lib: &Library| unsafe {
                let j = parse_fixture(lib, doc);
                let dumpb: Symbol<FnDumpb> = sym(lib, "json_dumpb");
                // size 0 with a NULL buffer is the documented "how long?" call
                let need = dumpb(j, ptr::null_mut(), 0, flags);
                let cap = need + 8;
                let mut sizes = vec![0usize, 1, 2, 3];
                if need >= 1 {
                    sizes.push(need - 1);
                }
                sizes.push(need);
                sizes.push(need + 1);
                let mut results = Vec::new();
                for &size in &sizes {
                    let mut b = vec![0xAAu8; cap];
                    let size = size.min(cap);
                    let ret = dumpb(j, b.as_mut_ptr() as *mut c_char, size, flags);
                    results.push((size, ret, b));
                }
                decref(lib, j);
                (need, results)
            };
            diff(&label, &probe);

            // Ground truth: the returned length is ALWAYS the full required
            // length, no NUL terminator is ever written, and nothing is written
            // past `size`.
            let (need, results) = probe(&libs().c);
            for (size, ret, buf) in &results {
                assert_eq!(*ret, need, "[{}] size {} must still return {}", label, size, need);
                assert!(
                    buf[*size..].iter().all(|&b| b == 0xAA),
                    "[{}] wrote past size {} (buf={:?})",
                    label,
                    size,
                    buf
                );
                if *size >= need {
                    assert_eq!(
                        buf[need], 0xAA,
                        "[{}] json_dumpb must NOT NUL-terminate",
                        label
                    );
                }
            }
        }
    }
}

// --------------------------- rows 230/232/233/234/240: failing dump callback

#[test]
fn rows230_240_dump_callback_failure_index() {
    const DOCS: &[&str] = &[
        "[1,2,3]",
        r#"{"a":1,"bb":[2,{"c":"d"}]}"#,
        r#"[[],{},"s",1,2.5,true,null]"#,
    ];
    for doc in DOCS {
        for &flags in &[0usize, JSON_SORT_KEYS, json_indent(2) | JSON_SORT_KEYS, JSON_COMPACT] {
            for &k in &[0usize, 1, 2, 3, 5, 10] {
                let label =
                    format!("rows230,240/dump_callback fail_at={} doc={} flags={:#x}", k, doc, flags);
                let probe = move |lib: &Library| unsafe {
                    let j = parse_fixture(lib, doc);
                    let mut sink = DumpSink::failing_at(k);
                    let dcb: Symbol<FnDumpCallback> = sym(lib, "json_dump_callback");
                    let rc = dcb(j, Some(dump_collect), &mut sink as *mut _ as *mut c_void, flags);
                    decref(lib, j);
                    (rc, sink.calls, sink.out)
                };
                diff(&label, &probe);

                let (rc, calls, out) = probe(&libs().c);
                // How many calls does a successful dump take?
                let total = {
                    let p2 = move |lib: &Library| unsafe {
                        let j = parse_fixture(lib, doc);
                        let mut sink = DumpSink::new();
                        let dcb: Symbol<FnDumpCallback> = sym(lib, "json_dump_callback");
                        let rc = dcb(
                            j,
                            Some(dump_collect),
                            &mut sink as *mut _ as *mut c_void,
                            flags,
                        );
                        decref(lib, j);
                        (rc, sink.calls)
                    };
                    let (rc_ok, n) = p2(&libs().c);
                    assert_eq!(rc_ok, 0, "[{}] baseline dump must succeed", label);
                    n
                };
                if k < total {
                    assert_eq!(rc, -1, "[{}] must fail (total calls {})", label, total);
                    // At least k+1 calls happened. NOT exactly k+1: `do_dump`
                    // IGNORES the return value of `dump_string()` for object
                    // KEYS (ERRORS.md row 227), so a failure landing there is
                    // swallowed and dumping continues until the next check.
                    assert!(calls >= k + 1, "[{}] calls={} k={}", label, calls, k);
                    assert!(calls <= total, "[{}] calls={} total={}", label, calls, total);
                } else {
                    assert_eq!(rc, 0, "[{}] k={} >= total={} must succeed", label, k, total);
                    assert_eq!(calls, total, "[{}]", label);
                }
                // Bytes emitted before the failure are whatever the k accepted
                // calls produced; they are compared between libraries by `diff`.
                let _ = out;
            }
        }
    }
}

/// The same failure reached from INSIDE `do_dump` (invalid UTF-8 in a string
/// created with `json_string_nocheck`), so it propagates out of every entry
/// point: `json_dumps` -> NULL, `json_dumpb` -> 0, the rest -> -1.
#[test]
fn row230_json_dumps_propagates_dump_failure() {
    for k in 0..6usize {
        for &flags in &[0usize, JSON_SORT_KEYS, json_indent(2)] {
            let label = format!("row230/invalid utf8 at index {} flags={:#x}", k, flags);
            let probe = |lib: &Library| unsafe {
                let arr: Symbol<FnVoidPtr> = sym(lib, "json_array");
                let app: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
                let int: Symbol<FnInt> = sym(lib, "json_integer");
                let snc: Symbol<FnStr> = sym(lib, "json_string_nocheck");
                let a = arr();
                for i in 0..6usize {
                    let child = if i == k {
                        let bad = cs_bytes(&[0xff, 0xfe, 0x41]);
                        snc(bad.as_ptr() as *const c_char)
                    } else {
                        int(i as json_int_t)
                    };
                    assert!(!child.is_null());
                    assert_eq!(app(a, child), 0);
                }
                let out = all_dump_entry_points(lib, a, flags);
                decref(lib, a);
                out
            };
            diff(&label, &probe);

            let got = probe(&libs().c);
            assert_eq!(got.rc_callback, -1, "[{}] json_dump_callback", label);
            assert_eq!(got.dumps, None, "[{}] json_dumps must return NULL", label);
            assert_eq!(got.dumpb_ret, 0, "[{}] json_dumpb must return 0", label);
            assert_eq!(got.rc_dumpf, -1, "[{}] json_dumpf", label);
            assert_eq!(got.rc_dumpfd, -1, "[{}] json_dumpfd", label);
            assert_eq!(got.rc_dump_file, -1, "[{}] json_dump_file", label);
            // Partial output was already flushed by the accepted calls.
            assert!(
                !got.cb_bytes.is_empty(),
                "[{}] the elements before the bad one are emitted",
                label
            );
        }
    }
}

// ------------------------- rows 197/198/235/237: dump I/O failures

const IO_DOC: &str = r#"[1,2,{"a":"b"},"ccc"]"#;

#[test]
fn rows197_198_235_237_dump_io_failures() {
    // Fixture created BEFORE diff; the closures only open it read-only.
    let existing = tmp_path("row197_readonly.txt");
    std::fs::write(&existing, b"placeholder-content").unwrap();

    for &flags in &[0usize, JSON_COMPACT | JSON_SORT_KEYS, json_indent(2)] {
        // row 197: fwrite() to a FILE* opened read-only fails.
        let label = format!("row197/json_dumpf to read-only FILE* flags={:#x}", flags);
        let ex = existing.clone();
        let probe = move |lib: &Library| unsafe {
            let j = parse_fixture(lib, IO_DOC);
            let fh = fopen_mode(&ex, "rb");
            assert!(!fh.is_null());
            let dumpf: Symbol<FnDumpf> = sym(lib, "json_dumpf");
            let rc = dumpf(j, fh, flags);
            fclose(fh);
            decref(lib, j);
            rc
        };
        diff(&label, &probe);
        assert_eq!(probe(&libs().c), -1, "[{}]", label);
        assert_eq!(
            std::fs::read(&existing).unwrap(),
            &b"placeholder-content"[..],
            "read-only FILE* must not be modified"
        );

        // contrast: a writable FILE* succeeds and writes the whole document.
        let label = format!("row197/json_dumpf to writable FILE* flags={:#x}", flags);
        let probe = |lib: &Library| unsafe {
            let j = parse_fixture(lib, IO_DOC);
            let p = unique_path("row197_ok");
            let _ = std::fs::remove_file(&p);
            let fh = fopen_mode(&p, "w");
            assert!(!fh.is_null());
            let dumpf: Symbol<FnDumpf> = sym(lib, "json_dumpf");
            let rc = dumpf(j, fh, flags);
            fclose(fh);
            let bytes = std::fs::read(&p).unwrap_or_default();
            let _ = std::fs::remove_file(&p);
            decref(lib, j);
            (rc, bytes)
        };
        diff(&label, &probe);
        assert_eq!(probe(&libs().c).0, 0, "[{}]", label);

        // row 198: write() to a read-only / closed / negative fd fails.
        let label = format!("row198/json_dumpfd to read-only fd flags={:#x}", flags);
        let ex = existing.clone();
        let probe = move |lib: &Library| unsafe {
            let j = parse_fixture(lib, IO_DOC);
            let f = std::fs::File::open(&ex).unwrap();
            let dumpfd: Symbol<FnDumpfd> = sym(lib, "json_dumpfd");
            let rc = dumpfd(j, f.as_raw_fd(), flags);
            drop(f);
            decref(lib, j);
            rc
        };
        diff(&label, &probe);
        assert_eq!(probe(&libs().c), -1, "[{}]", label);
        assert_eq!(std::fs::read(&existing).unwrap(), &b"placeholder-content"[..]);

        let label = format!("row198/json_dumpfd to stale closed fd flags={:#x}", flags);
        let ex = existing.clone();
        let probe = move |lib: &Library| unsafe {
            let j = parse_fixture(lib, IO_DOC);
            let fd = stale_high_fd(&ex);
            let dumpfd: Symbol<FnDumpfd> = sym(lib, "json_dumpfd");
            let rc = dumpfd(j, fd, flags);
            decref(lib, j);
            rc
        };
        diff(&label, &probe);
        assert_eq!(probe(&libs().c), -1, "[{}]", label);

        for &fd in &[-1, 4096] {
            let label = format!("row198/json_dumpfd({}) flags={:#x}", fd, flags);
            let probe = move |lib: &Library| unsafe {
                let j = parse_fixture(lib, IO_DOC);
                let dumpfd: Symbol<FnDumpfd> = sym(lib, "json_dumpfd");
                let rc = dumpfd(j, fd, flags);
                decref(lib, j);
                rc
            };
            diff(&label, &probe);
            assert_eq!(probe(&libs().c), -1, "[{}]", label);
        }

        // row 235: fopen(path, "w") fails.
        for path in ["/nonexistent-dir/x.json", "/tmp", "/", "/proc/self/cwd/x/y.json"] {
            let label = format!("row235/json_dump_file({:?}) flags={:#x}", path, flags);
            let probe = move |lib: &Library| unsafe {
                let j = parse_fixture(lib, IO_DOC);
                let cp = cs(path);
                let dfile: Symbol<FnDumpFile> = sym(lib, "json_dump_file");
                let rc = dfile(j, cp.as_ptr(), flags);
                decref(lib, j);
                rc
            };
            diff(&label, &probe);
            assert_eq!(probe(&libs().c), -1, "[{}]", label);
        }

        // row 237: json_dumpf's result is propagated by json_dump_file, and the
        // success path writes the same bytes json_dumps produces.
        let label = format!("row237/json_dump_file success flags={:#x}", flags);
        let probe = |lib: &Library| unsafe {
            let j = parse_fixture(lib, IO_DOC);
            let p = unique_path("row237_ok");
            let _ = std::fs::remove_file(&p);
            let cp = cs(p.to_str().unwrap());
            let dfile: Symbol<FnDumpFile> = sym(lib, "json_dump_file");
            let rc = dfile(j, cp.as_ptr(), flags);
            let bytes = std::fs::read(&p).unwrap_or_default();
            let _ = std::fs::remove_file(&p);
            let s = dumps_to_string(lib, j, flags);
            decref(lib, j);
            (rc, bytes, s)
        };
        diff(&label, &probe);
        let (rc, bytes, s) = probe(&libs().c);
        assert_eq!(rc, 0, "[{}]", label);
        assert_eq!(String::from_utf8(bytes).unwrap(), s.unwrap(), "[{}]", label);
    }

    let _ = std::fs::remove_file(&existing);
}

// ------------------------------------ row 208: JSON_REAL_PRECISION overflow

/// `(name, value, first JSON_REAL_PRECISION(n) that overflows the 25-byte
/// `jsonp_dtostr` buffer)`. The third field is the cut-over MEASURED against the
/// C library; `None` means every precision 0..=31 fits.
const PRECISION_VALUES: &[(&str, f64, Option<usize>)] = &[
    ("0.0", 0.0, None),
    ("-0.0", -0.0, None),
    ("1.0/3.0", 1.0 / 3.0, Some(22)),
    ("-1.0/3.0", -(1.0 / 3.0), Some(22)),
    ("0.1", 0.1, Some(22)),
    ("1e-4", 1e-4, Some(19)),
    ("1e300", 1e300, Some(18)),
    ("DBL_MAX", f64::MAX, Some(18)),
    ("DBL_MIN", f64::MIN_POSITIVE, Some(18)),
    ("5e-324", 5e-324, Some(18)),
    ("123456789.123456789", 123456789.123456789, Some(23)),
    ("2.0", 2.0, Some(25)),
    ("1e16", 1e16, Some(25)),
    ("1e17", 1e17, Some(25)),
];

#[test]
fn row208_real_precision_buffer_too_short() {
    let mut table: Vec<(String, Vec<usize>)> = Vec::new();

    for &(name, v, cutover) in PRECISION_VALUES {
        let mut failing = Vec::new();
        for n in 0..=31usize {
            let flags = JSON_ENCODE_ANY | json_real_precision(n);
            let label = format!("row208/json_real({}) precision={}", name, n);
            let probe = move |lib: &Library| unsafe {
                let real: Symbol<FnReal> = sym(lib, "json_real");
                let j = real(v);
                assert!(!j.is_null(), "json_real({}) must succeed", name);
                let s = dumps_to_string(lib, j, flags);
                // json_dumpb / json_dump_callback must fail identically
                let dumpb: Symbol<FnDumpb> = sym(lib, "json_dumpb");
                let mut buf = [0xAAu8; 64];
                let nb = dumpb(j, buf.as_mut_ptr() as *mut c_char, buf.len(), flags);
                let dcb: Symbol<FnDumpCallback> = sym(lib, "json_dump_callback");
                let mut sink = DumpSink::new();
                let rc = dcb(j, Some(dump_collect), &mut sink as *mut _ as *mut c_void, flags);
                decref(lib, j);
                (s, nb, rc, sink.out)
            };
            diff(&label, &probe);

            let (s, nb, rc, _) = probe(&libs().c);
            if s.is_none() {
                failing.push(n);
                assert_eq!(rc, -1, "[{}] callback must fail too", label);
                assert_eq!(nb, 0, "[{}] json_dumpb must return 0", label);
            } else {
                assert_eq!(rc, 0, "[{}]", label);
                assert_eq!(nb, s.as_ref().unwrap().len(), "[{}]", label);
            }
        }
        // Pin the exact cut-over measured against the C.
        assert_eq!(
            failing.first().copied(),
            cutover,
            "JSON_REAL_PRECISION cut-over for {} (failing={:?})",
            name,
            failing
        );
        table.push((name.to_string(), failing));
    }

    eprintln!("[row208] precisions where json_dumps returns NULL (25-byte buffer too short):");
    for (name, failing) in &table {
        eprintln!("[row208]   {:>20} -> {:?}", name, failing);
    }

    // Pin the documented example: 1.0/3.0 fails from some precision upwards and
    // the set of failing precisions is a contiguous tail (never a hole).
    let third = table.iter().find(|(n, _)| n == "1.0/3.0").unwrap();
    assert!(
        third.1.contains(&31),
        "ERRORS.md row 208: JSON_REAL_PRECISION(31) on 1.0/3.0 must fail, failing={:?}",
        third.1
    );
    for (name, failing) in &table {
        if let Some(&first) = failing.first() {
            let expected: Vec<usize> = (first..=31).collect();
            assert_eq!(
                failing, &expected,
                "failing precisions for {} must be a contiguous tail",
                name
            );
        }
    }
}

// -------------------------- randomized json_dumpb round-trip (rows 196/232)

unsafe fn build_value(lib: &Library, rng: &mut Rng, depth: u32) -> *mut json_t {
    if depth >= 3 || rng.below(100) < 55 {
        match rng.below(6) {
            0 => {
                let f: Symbol<FnVoidPtr> = sym(lib, "json_null");
                f()
            }
            1 => {
                let f: Symbol<FnVoidPtr> = sym(lib, "json_true");
                f()
            }
            2 => {
                let f: Symbol<FnVoidPtr> = sym(lib, "json_false");
                f()
            }
            3 => {
                let f: Symbol<FnInt> = sym(lib, "json_integer");
                f(rng.i64())
            }
            4 => {
                let f: Symbol<FnReal> = sym(lib, "json_real");
                f(rng.f64_finite())
            }
            _ => {
                let f: Symbol<FnStr> = sym(lib, "json_string");
                let s = cs(&rng.utf8_string(6));
                f(s.as_ptr())
            }
        }
    } else {
        build_container(lib, rng, depth)
    }
}

unsafe fn build_container(lib: &Library, rng: &mut Rng, depth: u32) -> *mut json_t {
    if rng.below(2) == 0 {
        let arr: Symbol<FnVoidPtr> = sym(lib, "json_array");
        let app: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        let a = arr();
        let n = rng.below(5);
        for _ in 0..n {
            let c = build_value(lib, rng, depth + 1);
            app(a, c);
        }
        a
    } else {
        let obj: Symbol<FnVoidPtr> = sym(lib, "json_object");
        let set: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
        let o = obj();
        let n = rng.below(5);
        for _ in 0..n {
            let k = cs(&rng.ascii_string(5));
            let c = build_value(lib, rng, depth + 1);
            set(o, k.as_ptr(), c);
        }
        o
    }
}

/// `json_indent(n)` is `n & JSON_MAX_INDENT`, i.e. the low 5 bits, spelled out
/// here because the harness helper is not a `const fn`.
const RANDOM_FLAGS: &[usize] = &[
    JSON_SORT_KEYS,
    JSON_SORT_KEYS | JSON_COMPACT,
    JSON_SORT_KEYS | JSON_ENSURE_ASCII,
    JSON_SORT_KEYS | 2,  // json_indent(2)
    JSON_SORT_KEYS | 31, // json_indent(31)
    JSON_SORT_KEYS | JSON_ESCAPE_SLASH | JSON_ENCODE_ANY,
];

#[test]
fn rows196_232_random_dumpb_buffer_sizes() {
    // Coverage counters, so the row cannot silently degenerate into 400 dumps of
    // `[]` into a big-enough buffer. Each closure runs twice per iteration (once
    // per library), hence the doubled counts.
    let too_small = AtomicUsize::new(0);
    let exact_fit = AtomicUsize::new(0);
    let roomy = AtomicUsize::new(0);
    let max_need = AtomicUsize::new(0);

    diff_n("rows196,232/random json_dumpb", 400, |lib: &Library, i: u64| unsafe {
        // Same seed per library run => structurally identical documents.
        let mut rng = Rng::new(0x9E37_79B9 ^ i.wrapping_mul(0x100_0193));
        let j = build_container(lib, &mut rng, 0);
        assert!(!j.is_null());
        let flags = RANDOM_FLAGS[rng.below(RANDOM_FLAGS.len() as u64) as usize];

        let dumpb: Symbol<FnDumpb> = sym(lib, "json_dumpb");
        let need = dumpb(j, ptr::null_mut(), 0, flags);
        let cap = need + 8;

        let size = match rng.below(7) {
            0 => 0,
            1 => 1,
            2 => need / 2,
            3 => need.saturating_sub(1),
            4 => need,
            5 => need + 1,
            _ => rng.below(need as u64 + 4) as usize,
        }
        .min(cap);

        match size.cmp(&need) {
            std::cmp::Ordering::Less => too_small.fetch_add(1, Ordering::Relaxed),
            std::cmp::Ordering::Equal => exact_fit.fetch_add(1, Ordering::Relaxed),
            std::cmp::Ordering::Greater => roomy.fetch_add(1, Ordering::Relaxed),
        };
        max_need.fetch_max(need, Ordering::Relaxed);

        let mut buf = vec![0xAAu8; cap];
        let ret = dumpb(j, buf.as_mut_ptr() as *mut c_char, size, flags);

        // A dump into a big-enough buffer must reproduce json_dumps exactly.
        let mut full = vec![0xAAu8; cap];
        let ret_full = dumpb(j, full.as_mut_ptr() as *mut c_char, cap, flags);
        let dumped = dumps_to_string(lib, j, flags);

        // Never a NUL terminator, never a byte past `size`.
        assert_eq!(ret, need);
        assert!(buf[size..].iter().all(|&b| b == 0xAA));
        assert_eq!(ret_full, need);
        assert_eq!(&full[..need], dumped.as_ref().unwrap().as_bytes());
        assert!(full[need..].iter().all(|&b| b == 0xAA));

        decref(lib, j);
        (need, size, ret, buf, ret_full, full, dumped)
    });

    let (ts, ef, rm, mn) = (
        too_small.load(Ordering::Relaxed),
        exact_fit.load(Ordering::Relaxed),
        roomy.load(Ordering::Relaxed),
        max_need.load(Ordering::Relaxed),
    );
    eprintln!(
        "[rows196,232] random coverage: too_small={} exact={} roomy={} max_len={}",
        ts, ef, rm, mn
    );
    assert!(ts > 200, "too few truncating buffer sizes: {}", ts);
    assert!(ef > 40, "too few exact-fit buffer sizes: {}", ef);
    assert!(rm > 40, "too few oversized buffer sizes: {}", rm);
    assert!(mn > 60, "generated documents are too small (max {})", mn);
}
