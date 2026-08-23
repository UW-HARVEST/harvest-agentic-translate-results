//! Phase B — decoder (load) differential tests. CONFIGS.md rows 78-138.
//!
//! Every case drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols only (never a direct Rust call) and compares the parsed result
//! (round-tripped through `json_dumps`) together with the FULL `json_error_t`
//! snapshot — the error struct is populated even on success (`error->position`
//! is set at `load.c:877-880`), so it is part of the observable behaviour.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------- fn types

type FnLoadf = unsafe extern "C" fn(*mut c_void, usize, *mut json_error_t) -> *mut json_t;
type FnLoadfd = unsafe extern "C" fn(c_int, usize, *mut json_error_t) -> *mut json_t;
type FnLoadFile = unsafe extern "C" fn(*const c_char, usize, *mut json_error_t) -> *mut json_t;
type JsonLoadCb = unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize;
type FnLoadCallback =
    unsafe extern "C" fn(JsonLoadCb, *mut c_void, usize, *mut json_error_t) -> *mut json_t;
/// Same ABI, but the callback slot is nullable (`Option<fn>` is null-pointer
/// optimized), so the `callback == NULL` branch can be reached without forging
/// an invalid non-nullable fn pointer.
type FnLoadCallbackOpt = unsafe extern "C" fn(
    Option<JsonLoadCb>,
    *mut c_void,
    usize,
    *mut json_error_t,
) -> *mut json_t;

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

const O_RDONLY: c_int = 0;

/// Default dump flags for round-tripping: ENCODE_ANY so scalar roots produced by
/// `JSON_DECODE_ANY` can be dumped at all (`dump.c:485-488`).
const DUMP: usize = JSON_ENCODE_ANY;

/// (round-tripped document, full error snapshot)
type Probe = (Option<String>, ErrSnap);

// ---------------------------------------------------------------- probes

/// `json_loads` (NUL-terminated input; `string_get` stops at the first NUL).
unsafe fn p_loads(lib: &Library, text: &[u8], lf: usize, df: usize) -> Probe {
    let f: Symbol<FnLoads> = sym(lib, "json_loads");
    let buf = cs_bytes(text);
    let mut err = json_error_t::new();
    let j = f(buf.as_ptr() as *const c_char, lf, &mut err);
    if j.is_null() {
        return (None, err.snapshot());
    }
    let out = dumps_to_string(lib, j, df);
    decref(lib, j);
    (out, err.snapshot())
}

/// `json_loadb` over an explicit length (NUL bytes inside the range are DATA).
unsafe fn p_loadb(lib: &Library, text: &[u8], buflen: usize, lf: usize, df: usize) -> Probe {
    let f: Symbol<FnLoadb> = sym(lib, "json_loadb");
    let buf = cs_bytes(text);
    let mut err = json_error_t::new();
    let j = f(buf.as_ptr() as *const c_char, buflen, lf, &mut err);
    if j.is_null() {
        return (None, err.snapshot());
    }
    let out = dumps_to_string(lib, j, df);
    decref(lib, j);
    (out, err.snapshot())
}

/// `json_loadf` on a `FILE*` opened from `path` ("rb").
unsafe fn p_loadf(lib: &Library, path: &PathBuf, lf: usize, df: usize) -> Probe {
    let f: Symbol<FnLoadf> = sym(lib, "json_loadf");
    let cpath = cs(path.to_str().unwrap());
    let mode = cs("rb");
    let fp = fopen(cpath.as_ptr(), mode.as_ptr());
    assert!(!fp.is_null(), "fopen failed for {:?}", path);
    let mut err = json_error_t::new();
    let j = f(fp, lf, &mut err);
    fclose(fp);
    if j.is_null() {
        return (None, err.snapshot());
    }
    let out = dumps_to_string(lib, j, df);
    decref(lib, j);
    (out, err.snapshot())
}

/// `json_loadfd` on a real fd opened from `path`.
unsafe fn p_loadfd(lib: &Library, path: &PathBuf, lf: usize, df: usize) -> Probe {
    let f: Symbol<FnLoadfd> = sym(lib, "json_loadfd");
    let cpath = cs(path.to_str().unwrap());
    let fd = open(cpath.as_ptr(), O_RDONLY);
    assert!(fd >= 0, "open failed for {:?}", path);
    let mut err = json_error_t::new();
    let j = f(fd, lf, &mut err);
    close(fd);
    if j.is_null() {
        return (None, err.snapshot());
    }
    let out = dumps_to_string(lib, j, df);
    decref(lib, j);
    (out, err.snapshot())
}

/// `json_load_file`.
unsafe fn p_load_file(lib: &Library, path: &str, lf: usize, df: usize) -> Probe {
    let f: Symbol<FnLoadFile> = sym(lib, "json_load_file");
    let cpath = cs(path);
    let mut err = json_error_t::new();
    let j = f(cpath.as_ptr(), lf, &mut err);
    if j.is_null() {
        return (None, err.snapshot());
    }
    let out = dumps_to_string(lib, j, df);
    decref(lib, j);
    (out, err.snapshot())
}

// ---------------------------------------------------------------- load_callback

struct CbState {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
    calls: usize,
    /// return `(size_t)-1` once `calls` exceeds this (usize::MAX = never)
    minus1_after: usize,
}

impl CbState {
    fn new(data: &[u8], chunk: usize) -> Self {
        CbState { data: data.to_vec(), pos: 0, chunk, calls: 0, minus1_after: usize::MAX }
    }
}

unsafe extern "C" fn cb_get(buf: *mut c_void, buflen: usize, data: *mut c_void) -> usize {
    let st = &mut *(data as *mut CbState);
    st.calls += 1;
    if st.calls > st.minus1_after {
        return usize::MAX;
    }
    let remaining = st.data.len() - st.pos;
    let n = remaining.min(st.chunk).min(buflen);
    if n > 0 {
        std::ptr::copy_nonoverlapping(st.data.as_ptr().add(st.pos), buf as *mut u8, n);
        st.pos += n;
    }
    n
}

/// `json_load_callback`; also returns the number of callback invocations so the
/// refill schedule itself is compared, not just the parse result.
unsafe fn p_load_callback(
    lib: &Library,
    text: &[u8],
    chunk: usize,
    minus1_after: usize,
    lf: usize,
    df: usize,
) -> (Option<String>, ErrSnap, usize) {
    let f: Symbol<FnLoadCallback> = sym(lib, "json_load_callback");
    let mut st = CbState::new(text, chunk);
    st.minus1_after = minus1_after;
    let mut err = json_error_t::new();
    let j = f(cb_get, &mut st as *mut CbState as *mut c_void, lf, &mut err);
    if j.is_null() {
        return (None, err.snapshot(), st.calls);
    }
    let out = dumps_to_string(lib, j, df);
    decref(lib, j);
    (out, err.snapshot(), st.calls)
}

// ---------------------------------------------------------------- misc helpers

fn c_loads(text: &[u8], lf: usize) -> Probe {
    unsafe { p_loads(&libs().c, text, lf, DUMP) }
}

/// Mirrors `jsonp_error_set_source` (error.c:17-31): paths >= 80 bytes are
/// stored as `"..."` + the last 76 bytes.
fn expected_source(path: &str) -> String {
    let n = path.len();
    if n < JSON_ERROR_SOURCE_LENGTH {
        path.to_string()
    } else {
        let extra = n - JSON_ERROR_SOURCE_LENGTH + 4;
        format!("...{}", &path[extra..])
    }
}

fn tmp_path(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("phase_b_load_{}_{}.json", std::process::id(), tag));
    p
}

/// Run `f` on a thread with a large stack: the recursive-descent parser needs
/// ~2048 nested frames for rows 103-105 and the default 2 MiB test stack is not
/// guaranteed to hold them in either library.
fn in_big_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .expect("spawn big-stack thread")
        .join()
        .expect("big-stack thread panicked");
}

fn nested_arrays(depth: usize) -> Vec<u8> {
    let mut s = Vec::with_capacity(depth * 2 + 1);
    s.extend(std::iter::repeat(b'[').take(depth));
    s.push(b'1');
    s.extend(std::iter::repeat(b']').take(depth));
    s
}

fn obj_with_keys(n: usize) -> Vec<u8> {
    let mut s = String::from("{");
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("\"k{}\":{}", i, i));
    }
    s.push('}');
    s.into_bytes()
}

fn arr_with_n(n: usize) -> Vec<u8> {
    let mut s = String::from("[");
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("{}", i));
    }
    s.push(']');
    s.into_bytes()
}

// ================================================================ rows 78, 138

#[test]
fn row78_138_baseline_every_entry_point() {
    let text: &[u8] = br#"{"a":1}"#;

    // row 78 / row 138: json_loads and json_loadb. `error->position` is written
    // even though the parse succeeded, so the ErrSnap is part of the contract.
    diff("row78/loads", |lib| unsafe { p_loads(lib, text, 0, DUMP) });
    diff("row78/loadb", |lib| unsafe { p_loadb(lib, text, text.len(), 0, DUMP) });
    diff("row138/position-on-success", |lib| unsafe {
        p_loads(lib, br#"  {"a": [1, 2] }  "#, 0, DUMP).1
    });

    // File-backed entry points: create the input ONCE, outside `diff`, because
    // the closure runs once per library.
    let path = tmp_path("row78");
    std::fs::write(&path, text).unwrap();
    let p1 = path.clone();
    diff("row78/loadf", move |lib| unsafe { p_loadf(lib, &p1, 0, DUMP) });
    let p2 = path.clone();
    diff("row78/loadfd", move |lib| unsafe { p_loadfd(lib, &p2, 0, DUMP) });
    let p3 = path.to_str().unwrap().to_string();
    diff("row78/load_file", move |lib| unsafe { p_load_file(lib, &p3, 0, DUMP) });
    diff("row78/load_callback", |lib| unsafe {
        p_load_callback(lib, text, 1024, usize::MAX, 0, DUMP)
    });
    let _ = std::fs::remove_file(&path);

    // Ground truth: the C really does report position 7 on success.
    let (out, err) = c_loads(text, 0);
    assert_eq!(out.as_deref(), Some(r#"{"a": 1}"#), "C ground truth for row 78");
    assert_eq!(err.position, 7, "C ground truth: error->position on success");
    assert_eq!(err.source, "<string>");
}

// ================================================================ rows 79-82

#[test]
fn rows79_82_duplicate_keys() {
    // row 79: last value wins (hashtable.c:243-245 replaces only the value).
    diff("row79/dup-no-flag", |lib| unsafe {
        p_loads(lib, br#"{"a":1,"a":2}"#, 0, DUMP)
    });
    // row 80: rejected with json_error_duplicate_key.
    diff("row80/REJECT_DUPLICATES", |lib| unsafe {
        p_loads(lib, br#"{"a":1,"a":2}"#, JSON_REJECT_DUPLICATES, DUMP)
    });
    // row 81: distinct keys — the getn probe runs on every insert but never hits.
    diff("row81/REJECT_DUPLICATES distinct", |lib| unsafe {
        p_loads(lib, br#"{"a":1,"b":2,"c":3}"#, JSON_REJECT_DUPLICATES, DUMP)
    });
    // row 81b: 9+ distinct keys so the probe also runs across a rehash.
    diff("row81/REJECT_DUPLICATES 12 keys", |lib| unsafe {
        p_loads(lib, &obj_with_keys(12), JSON_REJECT_DUPLICATES, DUMP)
    });
    // row 82: the repeated key keeps its FIRST ordinal position.
    for df in [DUMP, DUMP | JSON_COMPACT, DUMP | JSON_SORT_KEYS] {
        diff("row82/first-ordinal-position", move |lib| unsafe {
            p_loads(lib, br#"{"a":1,"b":2,"a":3}"#, 0, df)
        });
    }
    // row 82b: same with the duplicate far from the front, spanning a rehash.
    diff("row82/first-ordinal deep", |lib| unsafe {
        p_loads(
            lib,
            br#"{"k0":0,"k1":1,"k2":2,"k3":3,"k4":4,"k5":5,"k6":6,"k7":7,"k8":8,"k1":99}"#,
            0,
            DUMP,
        )
    });
    // row 80b: duplicate only detectable after a rehash.
    diff("row80/REJECT_DUPLICATES after rehash", |lib| unsafe {
        p_loads(
            lib,
            br#"{"k0":0,"k1":1,"k2":2,"k3":3,"k4":4,"k5":5,"k6":6,"k7":7,"k8":8,"k1":99}"#,
            JSON_REJECT_DUPLICATES,
            DUMP,
        )
    });

    // Ground truth
    assert_eq!(c_loads(br#"{"a":1,"a":2}"#, 0).0.as_deref(), Some(r#"{"a": 2}"#));
    assert_eq!(
        c_loads(br#"{"a":1,"a":2}"#, JSON_REJECT_DUPLICATES).1.code,
        JSON_ERROR_DUPLICATE_KEY
    );
    assert_eq!(
        c_loads(br#"{"a":1,"b":2,"a":3}"#, 0).0.as_deref(),
        Some(r#"{"a": 3, "b": 2}"#),
        "C ground truth: repeated key keeps its FIRST ordinal slot"
    );
}

// ================================================================ rows 83-86

#[test]
fn rows83_86_eof_check() {
    // row 83: trailing garbage ignored.
    diff("row83/DISABLE_EOF_CHECK trailing garbage", |lib| unsafe {
        p_loads(lib, b"[1] trailing-garbage", JSON_DISABLE_EOF_CHECK, DUMP)
    });
    // row 84: `[1][2]` -> `[1]`, error->position == 3 (the streaming resume point).
    diff("row84/DISABLE_EOF_CHECK [1][2]", |lib| unsafe {
        p_loads(lib, b"[1][2]", JSON_DISABLE_EOF_CHECK, DUMP)
    });
    diff("row84/[1][2] no flag", |lib| unsafe { p_loads(lib, b"[1][2]", 0, DUMP) });
    // row 85: `[1] x` rejected with json_error_end_of_input_expected.
    diff("row85/[1] x no flag", |lib| unsafe { p_loads(lib, b"[1] x", 0, DUMP) });
    diff("row85/[1] x DISABLE_EOF_CHECK", |lib| unsafe {
        p_loads(lib, b"[1] x", JSON_DISABLE_EOF_CHECK, DUMP)
    });
    // trailing whitespace only, both ways
    diff("row85/[1] trailing ws", |lib| unsafe { p_loads(lib, b"[1]  \t\r\n", 0, DUMP) });
    diff("row85/[1] trailing ws EOFCHK", |lib| unsafe {
        p_loads(lib, b"[1]  \t\r\n", JSON_DISABLE_EOF_CHECK, DUMP)
    });

    // row 86: `1 2 3` as a scalar stream — resume from error->position each time.
    diff("row86/scalar stream 1 2 3", |lib| unsafe {
        scalar_stream(lib, b"1 2 3", JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY)
    });
    diff("row86/container stream", |lib| unsafe {
        scalar_stream(lib, br#"[1][2]{"a":3}"#, JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY)
    });
    diff("row86/mixed stream with ws", |lib| unsafe {
        scalar_stream(lib, b"  true\nfalse\r\nnull 42 -1.5 \"s\" ", JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY)
    });

    // Ground truth
    let (out, err) = c_loads(b"[1][2]", JSON_DISABLE_EOF_CHECK);
    assert_eq!(out.as_deref(), Some("[1]"));
    assert_eq!(err.position, 3, "C ground truth: resume position after `[1]`");
    let (out, err) = c_loads(b"[1] x", 0);
    assert!(out.is_none());
    assert_eq!(err.code, JSON_ERROR_END_OF_INPUT_EXPECTED);
    assert_eq!(err.position, 5);
    // row 86 ground truth: three scalars, resume offsets 1 / 2 / 2 (the second
    // and third include the leading space the lexer skipped).
    let st = unsafe { scalar_stream(&libs().c, b"1 2 3", JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY) };
    assert_eq!(
        st.iter().map(|x| (x.0.clone(), x.1.position)).collect::<Vec<_>>(),
        vec![
            (Some("1".to_string()), 1),
            (Some("2".to_string()), 2),
            (Some("3".to_string()), 2),
        ],
        "C ground truth for the `1 2 3` scalar stream"
    );
}

/// Repeatedly `json_loadb` from `error->position`, mimicking a streaming reader.
unsafe fn scalar_stream(lib: &Library, text: &[u8], flags: usize) -> Vec<(Option<String>, ErrSnap)> {
    let f: Symbol<FnLoadb> = sym(lib, "json_loadb");
    let mut pos = 0usize;
    let mut acc = Vec::new();
    while pos < text.len() {
        let mut err = json_error_t::new();
        let slice = &text[pos..];
        let j = f(slice.as_ptr() as *const c_char, slice.len(), flags, &mut err);
        if j.is_null() {
            acc.push((None, err.snapshot()));
            break;
        }
        let s = dumps_to_string(lib, j, DUMP);
        decref(lib, j);
        let adv = err.position as usize;
        acc.push((s, err.snapshot()));
        if adv == 0 {
            break;
        }
        pos += adv;
    }
    acc
}

// ================================================================ rows 87-88

#[test]
fn rows87_88_decode_any() {
    let scalars: &[&[u8]] = &[b"42", b"-1.5", br#""str""#, b"true", b"false", b"null"];
    for s in scalars {
        let label = format!("row87/DECODE_ANY {}", String::from_utf8_lossy(s));
        diff(&label, |lib| unsafe { p_loads(lib, s, JSON_DECODE_ANY, DUMP) });
        let label = format!("row88/no-DECODE_ANY {}", String::from_utf8_lossy(s));
        diff(&label, |lib| unsafe { p_loads(lib, s, 0, DUMP) });
    }
    // containers still work with the flag set
    diff("row87/DECODE_ANY container", |lib| unsafe {
        p_loads(lib, br#"{"a":[1,"x",null]}"#, JSON_DECODE_ANY, DUMP)
    });
    // scalar root but the dump has no ENCODE_ANY: dump fails, parse succeeded
    diff("row87/DECODE_ANY dump without ENCODE_ANY", |lib| unsafe {
        p_loads(lib, b"42", JSON_DECODE_ANY, 0)
    });
    // row 88: empty input, with and without the flag
    diff("row88/empty no flag", |lib| unsafe { p_loads(lib, b"", 0, DUMP) });
    diff("row88/empty DECODE_ANY", |lib| unsafe { p_loads(lib, b"", JSON_DECODE_ANY, DUMP) });
    diff("row88/ws only", |lib| unsafe { p_loads(lib, b" \t\r\n", JSON_DECODE_ANY, DUMP) });

    let (out, err) = c_loads(b"42", 0);
    assert!(out.is_none());
    assert!(err.text.contains("'[' or '{' expected"), "C ground truth text: {}", err.text);
}

// ================================================================ rows 89-93

#[test]
fn rows89_93_integers_and_int_as_real() {
    // row 89
    diff("row89/INT_AS_REAL [123]", |lib| unsafe {
        p_loads(lib, b"[123]", JSON_DECODE_INT_AS_REAL, DUMP)
    });
    diff("row89/plain [123]", |lib| unsafe { p_loads(lib, b"[123]", 0, DUMP) });
    // row 90: no longer an overflow with the flag
    diff("row90/INT_AS_REAL LLONG_MAX+1", |lib| unsafe {
        p_loads(lib, b"[9223372036854775808]", JSON_DECODE_INT_AS_REAL, DUMP)
    });
    diff("row90/INT_AS_REAL huge", |lib| unsafe {
        p_loads(lib, b"[99999999999999999999999999]", JSON_DECODE_INT_AS_REAL, DUMP)
    });
    // row 91: already-real literal takes the same path either way
    diff("row91/INT_AS_REAL [1.5e3]", |lib| unsafe {
        p_loads(lib, b"[1.5e3]", JSON_DECODE_INT_AS_REAL, DUMP)
    });
    diff("row91/plain [1.5e3]", |lib| unsafe { p_loads(lib, b"[1.5e3]", 0, DUMP) });
    // row 92: exact LLONG bounds
    for t in [&b"[9223372036854775807]"[..], &b"[-9223372036854775808]"[..], &b"[0]"[..], &b"[-0]"[..]] {
        let label = format!("row92/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
        let label = format!("row92/INT_AS_REAL {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, JSON_DECODE_INT_AS_REAL, DUMP) });
    }
    // row 93: one past each bound (rejected)
    for t in [&b"[9223372036854775808]"[..], &b"[-9223372036854775809]"[..]] {
        let label = format!("row93/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
    // INT_AS_REAL also on the LLONG bounds and on a real needing the exponent path
    diff("row89/INT_AS_REAL bounds", |lib| unsafe {
        p_loads(
            lib,
            b"[9223372036854775807,-9223372036854775808,0,-0,1,-1,10]",
            JSON_DECODE_INT_AS_REAL,
            DUMP,
        )
    });
    // INT_AS_REAL with a precision-limited dump (17 digits)
    diff("row89/INT_AS_REAL precision17", |lib| unsafe {
        p_loads(lib, b"[123,9223372036854775807]", JSON_DECODE_INT_AS_REAL, DUMP | json_real_precision(17))
    });

    let (_, err) = c_loads(b"[9223372036854775808]", 0);
    assert_eq!(err.code, JSON_ERROR_NUMERIC_OVERFLOW);
    assert!(err.text.contains("too big integer"), "C text: {}", err.text);
    let (_, err) = c_loads(b"[-9223372036854775809]", 0);
    assert!(err.text.contains("too big negative integer"), "C text: {}", err.text);
}

// ================================================================ rows 94-96

#[test]
fn rows94_96_allow_nul() {
    // row 94: NUL inside a string VALUE is accepted, decoded length is 3.
    diff("row94/ALLOW_NUL value", |lib| unsafe {
        p_loads(lib, br#"["a\u0000b"]"#, JSON_ALLOW_NUL, DUMP)
    });
    diff("row94/ALLOW_NUL value length", |lib| unsafe {
        let f: Symbol<FnLoads> = sym(lib, "json_loads");
        let buf = cs_bytes(br#"["a\u0000b"]"#);
        let mut err = json_error_t::new();
        let j = f(buf.as_ptr() as *const c_char, JSON_ALLOW_NUL, &mut err);
        if j.is_null() {
            return (None, 0usize, err.snapshot());
        }
        let get: Symbol<FnArrGet> = sym(lib, "json_array_get");
        let slen: Symbol<FnSize> = sym(lib, "json_string_length");
        let sval: Symbol<FnStrVal> = sym(lib, "json_string_value");
        let e0 = get(j, 0);
        let len = slen(e0);
        let raw = sval(e0);
        // read exactly `len` bytes so the interior NUL is observable
        let bytes: Vec<u8> = (0..len).map(|i| *(raw as *const u8).add(i)).collect();
        let snap = err.snapshot();
        decref(lib, j);
        (Some(bytes), len, snap)
    });
    // a NUL-only string, and a NUL at the very start/end
    for t in [
        &br#"["\u0000"]"#[..],
        &br#"["\u0000ab"]"#[..],
        &br#"["ab\u0000"]"#[..],
        &br#"{"k":"a\u0000b"}"#[..],
    ] {
        let label = format!("row94/ALLOW_NUL {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, JSON_ALLOW_NUL, DUMP) });
        let label = format!("row95/no-flag {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
    // row 95
    diff("row95/no ALLOW_NUL", |lib| unsafe {
        p_loads(lib, br#"["a\u0000b"]"#, 0, DUMP)
    });
    // row 96: keys are rejected regardless of the flag (load.c:684-689)
    diff("row96/ALLOW_NUL key", |lib| unsafe {
        p_loads(lib, br#"{"a\u0000b":1}"#, JSON_ALLOW_NUL, DUMP)
    });
    diff("row96/no-flag key", |lib| unsafe { p_loads(lib, br#"{"a\u0000b":1}"#, 0, DUMP) });
    diff("row96/ALLOW_NUL key only NUL", |lib| unsafe {
        p_loads(lib, br#"{"\u0000":1}"#, JSON_ALLOW_NUL, DUMP)
    });

    let (_, err) = c_loads(br#"["a\u0000b"]"#, 0);
    assert_eq!(err.code, JSON_ERROR_NULL_CHARACTER);
    let (_, err) = c_loads(br#"{"a\u0000b":1}"#, JSON_ALLOW_NUL);
    assert_eq!(err.code, JSON_ERROR_NULL_BYTE_IN_KEY, "keys reject NUL flag-independently");
}

// ================================================================ rows 97-98

#[test]
fn rows97_98_flag_combinations() {
    // A corpus that reacts to every one of the five decoder flags.
    let corpus: &[&[u8]] = &[
        br#"{"a":1}"#,
        br#"{"a":1,"a":2}"#,
        br#"{"a":1,"b":2,"a":3}"#,
        b"[1][2]",
        b"[1] x",
        b"42",
        b"-1.5",
        b"null",
        b"[123]",
        b"[9223372036854775808]",
        br#"["a\u0000b"]"#,
        br#"{"a\u0000b":1}"#,
        br#"[1,{"a":[true,false,null,"\u00e9"]},1e2]"#,
    ];

    // row 97 + all 32 subsets of the five decoder flag bits.
    for bits in 0..32usize {
        for t in corpus {
            let label = format!("row97/flags=0x{:02x} {}", bits, String::from_utf8_lossy(t));
            diff(&label, |lib| unsafe { p_loads(lib, t, bits, DUMP) });
        }
    }

    // row 98: encoder-only bits handed to a decode function. JSON_INDENT(n)'s low
    // 5 bits ALIAS the decoder flags, so json_indent(31) == all five flags.
    let enc_flags: &[(&str, usize)] = &[
        ("COMPACT", JSON_COMPACT),
        ("ENSURE_ASCII", JSON_ENSURE_ASCII),
        ("SORT_KEYS", JSON_SORT_KEYS),
        ("PRESERVE_ORDER", JSON_PRESERVE_ORDER),
        ("ENCODE_ANY", JSON_ENCODE_ANY),
        ("ESCAPE_SLASH", JSON_ESCAPE_SLASH),
        ("EMBED", JSON_EMBED),
        ("REAL_PRECISION(17)", json_real_precision(17)),
        ("INDENT(0)", json_indent(0)),
        ("INDENT(1)", json_indent(1)),
        ("INDENT(2)", json_indent(2)),
        ("INDENT(4)", json_indent(4)),
        ("INDENT(8)", json_indent(8)),
        ("INDENT(31)", json_indent(31)),
        ("high-bits", 0xFFFF_0000_0000_0000usize),
        ("all-encoder", JSON_COMPACT | JSON_ENSURE_ASCII | JSON_SORT_KEYS | JSON_ENCODE_ANY),
    ];
    for (name, f) in enc_flags {
        for t in corpus {
            let label = format!("row98/{} {}", name, String::from_utf8_lossy(t));
            diff(&label, |lib| unsafe { p_loads(lib, t, *f, DUMP) });
        }
    }

    // json_indent(31) == 0x1F really is all five decoder flags at once, so `42`
    // now parses (DECODE_ANY) *and* comes back as a real (DECODE_INT_AS_REAL).
    assert_eq!(
        c_loads(b"42", json_indent(31)).0.as_deref(),
        Some("42.0"),
        "C ground truth: JSON_INDENT(31) aliases DECODE_ANY|DECODE_INT_AS_REAL|..."
    );
    assert_eq!(
        c_loads(b"42", json_indent(4)).0.as_deref(),
        Some("42"),
        "C ground truth: JSON_INDENT(4) aliases exactly JSON_DECODE_ANY"
    );
    assert!(
        c_loads(b"42", json_indent(3)).0.is_none(),
        "C ground truth: JSON_INDENT(3) has no DECODE_ANY bit"
    );
    assert_eq!(
        c_loads(br#"{"a":1,"a":2}"#, json_indent(1)).1.code,
        JSON_ERROR_DUPLICATE_KEY,
        "C ground truth: JSON_INDENT(1) aliases JSON_REJECT_DUPLICATES"
    );
}

// ================================================================ rows 99-102

#[test]
fn rows99_102_container_shapes() {
    // row 99: empty containers (early returns load.c:668/745)
    for t in [&b"{}"[..], &b"[]"[..], &b"{ }"[..], &b"[ ]"[..], &b"[\n\t]"[..]] {
        let label = format!("row99/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
    // row 100: single element / single key
    for t in [&b"[1]"[..], &br#"{"a":1}"#[..], &br#"[[]]"#[..], &br#"{"a":{}}"#[..]] {
        let label = format!("row100/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
    // row 101: 8 vs 9 (array grow at the 9th append, object rehash at the 9th key)
    for n in [1usize, 7, 8, 9, 10, 16, 17] {
        let a = arr_with_n(n);
        let label = format!("row101/array n={}", n);
        diff(&label, move |lib| unsafe { p_loads(lib, &a, 0, DUMP) });
        let o = obj_with_keys(n);
        let label = format!("row101/object n={}", n);
        diff(&label, move |lib| unsafe { p_loads(lib, &o, 0, DUMP) });
        let o2 = obj_with_keys(n);
        let label = format!("row101/object n={} SORT_KEYS", n);
        diff(&label, move |lib| unsafe { p_loads(lib, &o2, 0, DUMP | JSON_SORT_KEYS) });
    }
    // row 102: 40 keys (two rehashes, insertion order preserved on dump)
    let o40 = obj_with_keys(40);
    diff("row102/40 keys", move |lib| unsafe { p_loads(lib, &o40, 0, DUMP) });
    let o40b = obj_with_keys(40);
    diff("row102/40 keys SORT_KEYS", move |lib| unsafe {
        p_loads(lib, &o40b, 0, DUMP | JSON_SORT_KEYS)
    });
    let o40c = obj_with_keys(40);
    diff("row102/40 keys REJECT_DUPLICATES", move |lib| unsafe {
        p_loads(lib, &o40c, JSON_REJECT_DUPLICATES, DUMP)
    });
    // trailing comma / missing value shapes, to pin the loop-exit error paths
    for t in [
        &b"[1,]"[..],
        &b"[,1]"[..],
        &b"[1 2]"[..],
        &br#"{"a":1,}"#[..],
        &br#"{,}"#[..],
        &br#"{"a"}"#[..],
        &br#"{"a":}"#[..],
        &br#"{1:2}"#[..],
        &b"["[..],
        &b"{"[..],
        &b"]"[..],
        &b"}"[..],
    ] {
        let label = format!("row99/malformed {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
}

// ================================================================ rows 103-105

#[test]
fn rows103_105_nesting_depth() {
    in_big_stack(|| {
        // row 103: 2047 arrays + inner scalar == 2048 values == the legal maximum.
        let ok = nested_arrays(2047);
        diff("row103/depth 2047 arrays + scalar", move |lib| unsafe {
            // Only compare the shape summary; the dumped string is ~4 KiB.
            let (out, err) = p_loads(lib, &ok, 0, DUMP);
            (out.map(|s| (s.len(), s.as_bytes()[0], *s.as_bytes().last().unwrap())), err)
        });
        // row 104: 2048 arrays + scalar == 2049 values -> stack overflow.
        let bad = nested_arrays(2048);
        diff("row104/depth 2048 arrays + scalar", move |lib| unsafe {
            p_loads(lib, &bad, 0, DUMP)
        });
        // The boundary from the other side: 2048 arrays with an EMPTY innermost
        // array is 2048 values as well (the empty array is the 2048th value).
        let edge = {
            let mut s = Vec::new();
            s.extend(std::iter::repeat(b'[').take(2048));
            s.extend(std::iter::repeat(b']').take(2048));
            s
        };
        diff("row103/depth 2048 empty innermost", move |lib| unsafe {
            let (out, err) = p_loads(lib, &edge, 0, DUMP);
            (out.map(|s| s.len()), err)
        });
        let edge2 = {
            let mut s = Vec::new();
            s.extend(std::iter::repeat(b'[').take(2049));
            s.extend(std::iter::repeat(b']').take(2049));
            s
        };
        diff("row104/depth 2049 empty innermost", move |lib| unsafe {
            let (out, err) = p_loads(lib, &edge2, 0, DUMP);
            (out.map(|s| s.len()), err)
        });

        // row 105: mixed object/array nesting at depth ~1000 (500 pairs + scalar).
        let mixed = {
            let mut s = Vec::new();
            for _ in 0..500 {
                s.extend_from_slice(br#"[{"k":"#);
            }
            s.extend_from_slice(b"1");
            for _ in 0..500 {
                s.extend_from_slice(b"}]");
            }
            s
        };
        diff("row105/mixed nesting depth ~1001", move |lib| unsafe {
            let (out, err) = p_loads(lib, &mixed, 0, DUMP);
            (out.map(|s| (s.len(), s.as_bytes()[0])), err)
        });
        // and the same shape with an indented dump, to cross-check the tree itself
        let mixed2 = {
            let mut s = Vec::new();
            for _ in 0..40 {
                s.extend_from_slice(br#"[{"k":"#);
            }
            s.extend_from_slice(br#""x""#);
            for _ in 0..40 {
                s.extend_from_slice(b"}]");
            }
            s
        };
        diff("row105/mixed nesting depth 81 full dump", move |lib| unsafe {
            p_loads(lib, &mixed2, 0, DUMP | json_indent(1))
        });

        // Ground truth for the depth cap.
        let (out, err) = c_loads(&nested_arrays(2047), 0);
        assert!(out.is_some(), "C ground truth: 2047 arrays + scalar is legal");
        assert_eq!(err.code, JSON_ERROR_UNKNOWN);
        let (out, err) = c_loads(&nested_arrays(2048), 0);
        assert!(out.is_none(), "C ground truth: 2048 arrays + scalar is rejected");
        assert_eq!(err.code, JSON_ERROR_STACK_OVERFLOW);
    });
}

// ================================================================ rows 106-107

#[test]
fn rows106_107_whitespace_and_line_tracking() {
    // row 106: every whitespace form, leading / between / trailing
    let cases: &[&[u8]] = &[
        b" [1,2] ",
        b"\t[1,2]\t",
        b"\n[1,2]\n",
        b"\r[1,2]\r",
        b" \t\r\n[ 1 , 2 ]\n\r\t ",
        b"[\n\t1,\r\n\t2\n]",
        br#"{ "a" : 1 , "b" : [ ] }"#,
        b"{\n\t\"a\"\r:\n1\t,\n\"b\"\r\n:\r\n2\n}",
        b"\r\n\r\n{}\r\n\r\n",
        b"\t\t\t{}",
    ];
    for t in cases {
        let label = format!("row106/{:?}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
    // whitespace-only, and whitespace before a syntax error
    diff("row106/ws then error", |lib| unsafe { p_loads(lib, b" \t\r\n x", JSON_DECODE_ANY, DUMP) });

    // row 107: LF advances line/column; CR does NOT (stream_get load.c:191-199)
    let line_cases: &[&[u8]] = &[
        b"[\n1,\n2,\nx\n]",
        b"{\n  \"a\": 1,\n  \"b\": \n}",
        b"[1,\n2,\n3,\n4,\n5,\n@]",
        b"\n\n\n\n[1] x",
        b"[\r1,\r2,\rx]",
        b"[\r\n1,\r\n@]",
        b"[\n\"unterminated",
        b"[\n\"line\nbreak\"]",
        b"{\"a\":\n\n\n1 1}",
    ];
    for t in line_cases {
        let label = format!("row107/{:?}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }

    // Ground truth: the error really is reported on line 4.
    let (out, err) = c_loads(b"[\n1,\n2,\nx\n]", 0);
    assert!(out.is_none());
    assert_eq!(err.line, 4, "C ground truth: LF-tracked error line");
    assert_eq!(err.column, 1, "C ground truth: column after the LF");
    // CR does not start a new line
    let (_, err) = c_loads(b"[\r1,\r2,\rx]", 0);
    assert_eq!(err.line, 1, "C ground truth: CR does not advance the line counter");
}

// ================================================================ rows 108-110

#[test]
fn rows108_110_number_grammar() {
    let numbers: &[&str] = &[
        "0", "-0", "1", "-1", "10", "1.0", "-1.5", "1e2", "1E2", "1e+2", "1e-2", "1.5e308",
        "0.0001", "0.0", "-0.0", "100000000000000000000.0", "1e0", "1E0", "1e-308", "-1e-308",
        "2.2250738585072014e-308", "5e-324", "3.141592653589793", "1234567890123456789",
        "0e0", "0.5", "-0.5", "1.7976931348623157e308",
    ];
    for n in numbers {
        let inner = format!("[{}]", n).into_bytes();
        let label = format!("row108/[{}]", n);
        diff(&label, move |lib| unsafe { p_loads(lib, &inner, 0, DUMP) });
        let bare = n.to_string().into_bytes();
        let label = format!("row108/bare {} DECODE_ANY", n);
        diff(&label, move |lib| unsafe { p_loads(lib, &bare, JSON_DECODE_ANY, DUMP) });
        let inner2 = format!("[{}]", n).into_bytes();
        let label = format!("row108/[{}] INT_AS_REAL", n);
        diff(&label, move |lib| unsafe {
            p_loads(lib, &inner2, JSON_DECODE_INT_AS_REAL, DUMP)
        });
    }
    // all of them at once
    let all = format!("[{}]", numbers.join(",")).into_bytes();
    diff("row108/all numbers", move |lib| unsafe { p_loads(lib, &all, 0, DUMP) });

    // grammar violations the lexer must reject
    let bad: &[&[u8]] = &[
        b"[01]", b"[-01]", b"[+1]", b"[1.]", b"[.1]", b"[1e]", b"[1e+]", b"[1e-]", b"[--1]",
        b"[1.2.3]", b"[-]", b"[1.e2]", b"[0x10]", b"[1_000]", b"[Infinity]", b"[NaN]",
        b"[00]", b"[1e2e3]",
    ];
    for t in bad {
        let label = format!("row108/reject {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
        let label = format!("row108/reject INT_AS_REAL {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, JSON_DECODE_INT_AS_REAL, DUMP) });
    }

    // row 109: real overflow via HUGE_VAL/ERANGE
    for t in [&b"[1e999]"[..], &b"[-1e999]"[..], &b"[1e400]"[..], &b"[1.5e310]"[..]] {
        let label = format!("row109/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
    // underflow is NOT an error in jansson's jsonp_strtod (no ERANGE check for it)
    for t in [&b"[1e-999]"[..], &b"[-1e-999]"[..]] {
        let label = format!("row109/underflow {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }

    // row 110: -0.0 round-trips with the sign
    diff("row110/[-0.0]", |lib| unsafe { p_loads(lib, b"[-0.0]", 0, DUMP) });
    diff("row110/[-0.0] precision17", |lib| unsafe {
        p_loads(lib, b"[-0.0]", 0, DUMP | json_real_precision(17))
    });
    diff("row110/[-0] INT_AS_REAL", |lib| unsafe {
        p_loads(lib, b"[-0]", JSON_DECODE_INT_AS_REAL, DUMP)
    });

    let (_, err) = c_loads(b"[1e999]", 0);
    assert_eq!(err.code, JSON_ERROR_NUMERIC_OVERFLOW);
    assert!(err.text.contains("real number overflow"), "C text: {}", err.text);
    assert_eq!(
        c_loads(b"[-0.0]", 0).0.as_deref(),
        Some("[-0.0]"),
        "C ground truth: -0.0 keeps its sign"
    );
}

// ================================================================ rows 111-115

#[test]
fn rows111_115_string_escapes_and_surrogates() {
    // row 111: every mandatory short escape
    let esc: &[&[u8]] = &[
        br#"["\""]"#,
        br#"["\\"]"#,
        br#"["\/"]"#,
        br#"["\b"]"#,
        br#"["\f"]"#,
        br#"["\n"]"#,
        br#"["\r"]"#,
        br#"["\t"]"#,
        br#"["\"\\\/\b\f\n\r\t"]"#,
        br#"{"\t":"\n"}"#,
    ];
    for t in esc {
        let label = format!("row111/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
        let label = format!("row111/ENSURE_ASCII {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP | JSON_ENSURE_ASCII) });
    }
    // invalid escapes
    for t in [
        &br#"["\q"]"#[..],
        &br#"["\ "]"#[..],
        &br#"["\"#[..],
        &br#"["\x41"]"#[..],
        &br#"["\U0041"]"#[..],
        &br#"["\0"]"#[..],
    ] {
        let label = format!("row111/reject {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }

    // row 112: \uXXXX with lower AND upper case hex (both decode_unicode_escape arms)
    let uni: &[&[u8]] = &[
        br#"["\u0041"]"#,
        br#"["\u00e9"]"#,
        br#"["\u00E9"]"#,
        br#"["\u20ac"]"#,
        br#"["\u20AC"]"#,
        br#"["\uabcd"]"#,
        br#"["\uABCD"]"#,
        br#"["\uAbCd"]"#,
        br#"["\u007f"]"#,
        br#"["\u0080"]"#,
        br#"["\u07ff"]"#,
        br#"["\u0800"]"#,
        br#"["\uffff"]"#,
        br#"["\uFFFF"]"#,
        br#"["\u0022\u005c\u002f"]"#,
        br#"{"\u0041":"\u0042"}"#,
    ];
    for t in uni {
        let label = format!("row112/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
        let label = format!("row112/ENSURE_ASCII {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP | JSON_ENSURE_ASCII) });
    }
    // bad hex digits
    for t in [&br#"["\uZZZZ"]"#[..], &br#"["\u00g0"]"#[..], &br#"["\u12"]"#[..], &br#"["\u"]"#[..]] {
        let label = format!("row112/reject {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }

    // row 113: valid surrogate pairs
    for t in [
        &br#"["\uD834\uDD1E"]"#[..],
        &br#"["\ud834\udd1e"]"#[..],
        &br#"["\uD800\uDC00"]"#[..], // U+10000, the low boundary
        &br#"["\uDBFF\uDFFF"]"#[..], // U+10FFFF, the high boundary
        &br#"["a\uD83D\uDE00b"]"#[..],
    ] {
        let label = format!("row113/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
        let label = format!("row113/ENSURE_ASCII {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP | JSON_ENSURE_ASCII) });
    }

    // row 114: lone high surrogate / bad second half
    for t in [
        &br#"["\uD834"]"#[..],
        &br#"["\uD834x"]"#[..],
        &br#"["\uD834\u0041"]"#[..],
        &br#"["\uD834\uD834"]"#[..],
        &br#"["\uD834\\"]"#[..],
        &br#"["\uDBFF"]"#[..],
        &br#"["\uD800\uD800"]"#[..],
    ] {
        let label = format!("row114/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
    // row 115: lone low surrogate
    for t in [
        &br#"["\uDC00"]"#[..],
        &br#"["\uDFFF"]"#[..],
        &br#"["\uDC00\uD834"]"#[..],
        &br#"["a\uDD1Eb"]"#[..],
    ] {
        let label = format!("row115/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }

    // Ground truth
    assert_eq!(
        c_loads(br#"["\uD834\uDD1E"]"#, 0).0.as_deref(),
        Some("[\"\u{1D11E}\"]"),
        "C ground truth: surrogate pair -> U+1D11E"
    );
    let (out, err) = c_loads(br#"["\uD834"]"#, 0);
    assert!(out.is_none(), "C ground truth: lone high surrogate rejected");
    assert_eq!(err.code, JSON_ERROR_INVALID_SYNTAX);
    let (out, err) = c_loads(br#"["\uDC00"]"#, 0);
    assert!(out.is_none(), "C ground truth: lone low surrogate rejected");
    assert_eq!(err.code, JSON_ERROR_INVALID_SYNTAX);
}

// ================================================================ rows 116-120

#[test]
fn rows116_120_utf8_literals_and_string_lengths() {
    // row 116: raw 1/2/3/4-byte UTF-8 inside a string
    let raws: &[&[u8]] = &[
        "[\"a\"]".as_bytes(),
        "[\"\u{e9}\"]".as_bytes(),
        "[\"\u{20ac}\"]".as_bytes(),
        "[\"\u{1D11E}\"]".as_bytes(),
        "[\"a\u{e9}\u{20ac}\u{1D11E}z\"]".as_bytes(),
        "[\"\u{7f}\u{80}\u{7ff}\u{800}\u{ffff}\u{10000}\u{10ffff}\"]".as_bytes(),
        "{\"\u{4e2d}\u{6587}\":\"\u{1F600}\"}".as_bytes(),
    ];
    for t in raws {
        let label = format!("row116/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
        let label = format!("row116/ENSURE_ASCII {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP | JSON_ENSURE_ASCII) });
    }
    // invalid raw UTF-8 (stream_get -> STREAM_STATE_ERROR, no "near" context)
    let bad_utf8: &[&[u8]] = &[
        b"[\"\xc3\"]",       // truncated 2-byte
        b"[\"\xe2\x82\"]",   // truncated 3-byte
        b"[\"\xff\"]",       // never a lead byte
        b"[\"\x80\"]",       // bare continuation
        b"[\"\xc0\x80\"]",   // overlong NUL
        b"[\"\xed\xa0\x80\"]", // encoded surrogate
        b"[\"\xf5\x80\x80\x80\"]", // > U+10FFFF
    ];
    for t in bad_utf8 {
        let label = format!("row116/invalid-utf8 {:?}", t);
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }

    // row 117: multi-byte UTF-8 as the FIRST byte of a token outside a string
    // (lex_save_cached load.c:623 saves the rest of the sequence for the message)
    let outside: &[&[u8]] = &[
        "[\u{e9}]".as_bytes(),
        "[\u{20ac}]".as_bytes(),
        "[\u{1D11E}]".as_bytes(),
        "\u{e9}".as_bytes(),
        "{\u{e9}:1}".as_bytes(),
        "[1,\u{4e2d}]".as_bytes(),
    ];
    for t in outside {
        let label = format!("row117/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, JSON_DECODE_ANY, DUMP) });
        let label = format!("row117/no-any {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
    // ASCII garbage bytes outside a string, for the same else-branch
    for t in [&b"[@]"[..], &b"[#]"[..], &b"[\x01]"[..], &b"[\x7f]"[..]] {
        let label = format!("row117/ascii-garbage {:?}", t);
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }

    // row 118: true / false / null and their near-misses
    for t in [
        &b"[true,false,null]"[..],
        &b"[true]"[..],
        &b"[false]"[..],
        &b"[null]"[..],
        &b"[tru]"[..],
        &b"[truex]"[..],
        &b"[TRUE]"[..],
        &b"[True]"[..],
        &b"[nulll]"[..],
        &b"[nul]"[..],
        &b"[undefined]"[..],
        &b"[falsey]"[..],
        &b"[t]"[..],
    ] {
        let label = format!("row118/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }

    // row 119: key/value string lengths 0, 1, 12, 13, > 1024
    for len in [0usize, 1, 11, 12, 13, 24, 25, 1023, 1024, 1025, 4096] {
        let filler: String = "a".repeat(len);
        let doc = format!("{{\"{}\":\"{}\"}}", filler, filler).into_bytes();
        let label = format!("row119/key+value len={}", len);
        diff(&label, move |lib| unsafe {
            let (out, err) = p_loads(lib, &doc, 0, DUMP);
            (out.map(|s| (s.len(), s)), err)
        });
    }
    // the empty key and the empty string value on their own
    for t in [&br#"{"":1}"#[..], &br#"[""]"#[..], &br#"{"":""}"#[..]] {
        let label = format!("row119/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }
    // unterminated / control characters in strings
    for t in [
        &br#"["abc"#[..],
        &b"[\"a\nb\"]"[..],
        &b"[\"a\tb\"]"[..],
        &b"[\"a\x01b\"]"[..],
        &b"[\"a\x1fb\"]"[..],
        &b"[\"a\rb\"]"[..],
    ] {
        let label = format!("row119/reject {:?}", t);
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
    }

    // row 120: escapes make the decoded value SHORTER than the source
    // (the t/p walk at load.c:358-451 over-allocates on purpose)
    let shorter: &[&[u8]] = &[
        br#"["\n\t\r\b\f"]"#,
        br#"["\u0041\u0042\u0043"]"#,
        br#"["\uD834\uDD1E\uD834\uDD1E"]"#,
        br#"["\\\\\\\\"]"#,
        br#"["a\u0041b\u0042c\nd\te"]"#,
        br#"["\u00e9\u00e9\u00e9\u00e9\u00e9\u00e9\u00e9\u00e9"]"#,
    ];
    for t in shorter {
        let label = format!("row120/{}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe {
            let (out, err) = p_loads(lib, t, 0, DUMP);
            (out.map(|s| (s.len(), s)), err)
        });
        let label = format!("row120/ENSURE_ASCII {}", String::from_utf8_lossy(t));
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP | JSON_ENSURE_ASCII) });
    }
    // a 2 KiB string built entirely from \uXXXX escapes (source 6x the value)
    let big_esc = {
        let mut s = String::from("[\"");
        for _ in 0..400 {
            s.push_str("\\u00e9");
        }
        s.push_str("\"]");
        s.into_bytes()
    };
    diff("row120/400 x \\u00e9", move |lib| unsafe {
        let (out, err) = p_loads(lib, &big_esc, 0, DUMP);
        (out.map(|s| s.len()), err)
    });
}

// ================================================================ rows 121-124

#[test]
fn rows121_124_loads_vs_loadb() {
    // row 121: json_loads stops at the embedded NUL (string_get returns EOF).
    let with_nul: &[u8] = b"[1]\0[2]";
    diff("row121/loads stops at NUL", |lib| unsafe {
        p_loads(lib, with_nul, 0, DUMP)
    });
    // row 122: json_loadb with a buflen SPANNING the NUL returns it as data.
    diff("row122/loadb spans NUL", |lib| unsafe {
        p_loadb(lib, with_nul, with_nul.len(), 0, DUMP)
    });
    diff("row122/loadb spans NUL EOFCHK", |lib| unsafe {
        p_loadb(lib, with_nul, with_nul.len(), JSON_DISABLE_EOF_CHECK, DUMP)
    });
    // NUL in the middle of a token / inside a string / as the whole input
    for t in [&b"[1,\0 2]"[..], &b"[\"a\0b\"]"[..], &b"\0"[..], &b"\0[1]"[..], &b"[1]\0"[..]] {
        let label = format!("row121/loads {:?}", t);
        diff(&label, |lib| unsafe { p_loads(lib, t, 0, DUMP) });
        let label = format!("row122/loadb {:?}", t);
        let n = t.len();
        diff(&label, move |lib| unsafe { p_loadb(lib, t, n, 0, DUMP) });
        let label = format!("row122/loadb ALLOW_NUL {:?}", t);
        diff(&label, move |lib| unsafe { p_loadb(lib, t, n, JSON_ALLOW_NUL, DUMP) });
    }

    // row 123: buflen shorter than strlen
    let long: &[u8] = b"[1,2,3]";
    for n in 0..=long.len() {
        let label = format!("row123/loadb buflen={}", n);
        diff(&label, move |lib| unsafe { p_loadb(lib, long, n, 0, DUMP) });
        let label = format!("row123/loadb buflen={} EOFCHK", n);
        diff(&label, move |lib| unsafe {
            p_loadb(lib, long, n, JSON_DISABLE_EOF_CHECK, DUMP)
        });
    }
    // truncation that still yields a complete document
    diff("row123/loadb [1]xxxx buflen=3", |lib| unsafe {
        p_loadb(lib, b"[1]xxxx", 3, 0, DUMP)
    });
    // row 124: buflen == 0
    diff("row124/loadb buflen=0", |lib| unsafe { p_loadb(lib, b"[1]", 0, 0, DUMP) });
    diff("row124/loadb buflen=0 DECODE_ANY", |lib| unsafe {
        p_loadb(lib, b"[1]", 0, JSON_DECODE_ANY, DUMP)
    });

    // NULL buffer arguments (invalid argument, lex never initialised)
    diff("row121/loads NULL string", |lib| unsafe {
        let f: Symbol<FnLoads> = sym(lib, "json_loads");
        let mut err = json_error_t::new();
        let j = f(std::ptr::null(), 0, &mut err);
        (j.is_null(), err.snapshot())
    });
    diff("row122/loadb NULL buffer", |lib| unsafe {
        let f: Symbol<FnLoadb> = sym(lib, "json_loadb");
        let mut err = json_error_t::new();
        let j = f(std::ptr::null(), 4, 0, &mut err);
        (j.is_null(), err.snapshot())
    });

    // Ground truth: loads vs loadb really do differ on the same bytes.
    assert_eq!(
        c_loads(b"[1]\0[2]", 0).0.as_deref(),
        Some("[1]"),
        "C ground truth: json_loads treats NUL as EOF"
    );
    let cb = unsafe { p_loadb(&libs().c, b"[1]\0[2]", 7, 0, DUMP) };
    assert!(cb.0.is_none(), "C ground truth: json_loadb sees the NUL as data");
    assert_eq!(cb.1.source, "<buffer>");
}

// ================================================================ rows 125-129

#[test]
fn rows125_129_loadf_and_loadfd() {
    // Create every input file ONCE, outside the closures.
    let path = tmp_path("row125");
    std::fs::write(&path, br#"{"a":[1,2,3],"b":"\u00e9","c":null}"#).unwrap();
    let empty = tmp_path("row125_empty");
    std::fs::write(&empty, b"").unwrap();
    let broken = tmp_path("row125_broken");
    std::fs::write(&broken, b"[1,2,").unwrap();
    let trailing = tmp_path("row125_trailing");
    std::fs::write(&trailing, b"[1] x").unwrap();
    let scalar = tmp_path("row125_scalar");
    std::fs::write(&scalar, b"42").unwrap();
    let multiline = tmp_path("row125_multiline");
    std::fs::write(&multiline, b"[\n1,\n2,\nx\n]").unwrap();
    // > 1024 bytes so the fgetc / read loops iterate many times
    let big = tmp_path("row125_big");
    std::fs::write(&big, arr_with_n(600)).unwrap();

    // row 125: json_loadf on a regular file -> source "<stream>"
    for (tag, p, lf) in [
        ("ok", &path, 0usize),
        ("empty", &empty, 0),
        ("broken", &broken, 0),
        ("trailing", &trailing, 0),
        ("trailing-EOFCHK", &trailing, JSON_DISABLE_EOF_CHECK),
        ("scalar-any", &scalar, JSON_DECODE_ANY),
        ("scalar-noany", &scalar, 0),
        ("multiline", &multiline, 0),
        ("big", &big, 0),
    ] {
        let pc = (*p).clone();
        let label = format!("row125/loadf {}", tag);
        diff(&label, move |lib| unsafe { p_loadf(lib, &pc, lf, DUMP) });
        let pc = (*p).clone();
        let label = format!("row127/loadfd {}", tag);
        diff(&label, move |lib| unsafe { p_loadfd(lib, &pc, lf, DUMP) });
    }

    // row 126 note: `json_loadf(stdin)` / row 128 `json_loadfd(STDIN_FILENO)`
    // would consume the harness's stdin (and can block), so the `<stdin>` source
    // branch is exercised only indirectly here. What IS asserted is the
    // NULL-input branch (which shares `jsonp_error_init(source)`) and the
    // regular-file `<stream>` branch above.
    diff("row126/loadf NULL input", |lib| unsafe {
        let f: Symbol<FnLoadf> = sym(lib, "json_loadf");
        let mut err = json_error_t::new();
        let j = f(std::ptr::null_mut(), 0, &mut err);
        (j.is_null(), err.snapshot())
    });

    // row 129: fd < 0 (rejected before lex_init)
    for fd in [-1i32, -2, i32::MIN] {
        let label = format!("row129/loadfd fd={}", fd);
        diff(&label, move |lib| unsafe {
            let f: Symbol<FnLoadfd> = sym(lib, "json_loadfd");
            let mut err = json_error_t::new();
            let j = f(fd, 0, &mut err);
            (j.is_null(), err.snapshot())
        });
    }

    // Ground truth
    let cp = unsafe { p_loadf(&libs().c, &path, 0, DUMP) };
    assert_eq!(cp.1.source, "<stream>", "C ground truth: loadf source on a regular file");
    let cf = unsafe {
        let f: Symbol<FnLoadfd> = sym(&libs().c, "json_loadfd");
        let mut err = json_error_t::new();
        let j = f(-1, 0, &mut err);
        assert!(j.is_null());
        err.snapshot()
    };
    assert_eq!(cf.code, JSON_ERROR_INVALID_ARGUMENT);
    assert_eq!(cf.source, "<stream>");

    for p in [&path, &empty, &broken, &trailing, &scalar, &multiline, &big] {
        let _ = std::fs::remove_file(p);
    }
}

// ================================================================ rows 130-132

#[test]
fn rows130_132_load_file() {
    let path = tmp_path("row130");
    std::fs::write(&path, br#"{"a":1,"b":[2,3]}"#).unwrap();
    let ps = path.to_str().unwrap().to_string();

    // row 130: existing file — on SUCCESS the source is overwritten to "<stream>"
    // by json_loadf's jsonp_error_init (load.c:978).
    let p = ps.clone();
    diff("row130/load_file ok", move |lib| unsafe { p_load_file(lib, &p, 0, DUMP) });
    // a parse error keeps "<stream>" too (the error is set inside json_loadf)
    let bad_path = tmp_path("row130_bad");
    std::fs::write(&bad_path, b"[1,").unwrap();
    let bp = bad_path.to_str().unwrap().to_string();
    let p = bp.clone();
    diff("row130/load_file parse error", move |lib| unsafe {
        p_load_file(lib, &p, 0, DUMP)
    });

    // row 131: nonexistent path -> cannot-open-file, source = the path
    let missing = tmp_path("row131_does_not_exist");
    let _ = std::fs::remove_file(&missing);
    let ms = missing.to_str().unwrap().to_string();
    let p = ms.clone();
    diff("row131/load_file missing", move |lib| unsafe { p_load_file(lib, &p, 0, DUMP) });
    // a directory is openable-as-a-path but unreadable on Linux: fopen("rb")
    // succeeds on some systems, so just compare whatever both libraries do.
    let dir = std::env::temp_dir().to_str().unwrap().to_string();
    diff("row131/load_file directory", move |lib| unsafe {
        p_load_file(lib, &dir, 0, DUMP)
    });
    // NULL path
    diff("row131/load_file NULL", |lib| unsafe {
        let f: Symbol<FnLoadFile> = sym(lib, "json_load_file");
        let mut err = json_error_t::new();
        let j = f(std::ptr::null(), 0, &mut err);
        (j.is_null(), err.snapshot())
    });

    // row 132: path >= JSON_ERROR_SOURCE_LENGTH (80) -> "..." + last 76 chars.
    // The truncation is only OBSERVABLE on the fopen-failure path, because
    // success overwrites source with "<stream>".
    let mut long = std::env::temp_dir();
    long.push(format!("phase_b_load_long_{}_{}.json", std::process::id(), "y".repeat(140)));
    let long_s = long.to_str().unwrap().to_string();
    assert!(long_s.len() >= JSON_ERROR_SOURCE_LENGTH, "need a path >= 80 chars");
    let p = long_s.clone();
    diff("row132/load_file long missing path", move |lib| unsafe {
        p_load_file(lib, &p, 0, DUMP)
    });
    // and the same length of path, but existing (source becomes "<stream>")
    std::fs::write(&long, br#"[1,2]"#).unwrap();
    let p = long_s.clone();
    diff("row132/load_file long existing path", move |lib| unsafe {
        p_load_file(lib, &p, 0, DUMP)
    });
    // exactly 79 and exactly 80 characters, to pin the `<` in error.c:24
    for target in [JSON_ERROR_SOURCE_LENGTH - 1, JSON_ERROR_SOURCE_LENGTH, JSON_ERROR_SOURCE_LENGTH + 1] {
        let dirpart = std::env::temp_dir().to_str().unwrap().to_string();
        let prefix = format!("{}/pbl_", dirpart.trim_end_matches('/'));
        if prefix.len() + 1 > target {
            continue;
        }
        let cand = format!("{}{}", prefix, "z".repeat(target - prefix.len()));
        assert_eq!(cand.len(), target);
        let label = format!("row132/load_file path len={}", target);
        diff(&label, move |lib| unsafe { p_load_file(lib, &cand, 0, DUMP) });
    }

    // Ground truth
    let c = unsafe { p_load_file(&libs().c, &ps, 0, DUMP) };
    assert_eq!(c.0.as_deref(), Some(r#"{"a": 1, "b": [2, 3]}"#));
    assert_eq!(c.1.source, "<stream>", "C: source overwritten on success");
    let c = unsafe { p_load_file(&libs().c, &ms, 0, DUMP) };
    assert!(c.0.is_none());
    assert_eq!(c.1.code, JSON_ERROR_CANNOT_OPEN_FILE);
    assert_eq!(
        c.1.source,
        expected_source(&ms),
        "C: fopen failure keeps the path as source (truncated at 80 bytes)"
    );
    // The >= 80 truncation really is what happens, and it starts with "..."
    let long_missing = format!("{}.nope", long_s);
    let c = unsafe { p_load_file(&libs().c, &long_missing, 0, DUMP) };
    assert!(c.0.is_none());
    assert!(c.1.source.starts_with("..."), "C: long path truncated, got {:?}", c.1.source);
    assert_eq!(c.1.source.len(), JSON_ERROR_SOURCE_LENGTH - 1);
    assert_eq!(c.1.source, expected_source(&long_missing));
    // ... whereas SUCCESS discards the path entirely
    let c = unsafe { p_load_file(&libs().c, &long_s, 0, DUMP) };
    assert_eq!(c.1.source, "<stream>");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&bad_path);
    let _ = std::fs::remove_file(&long);
}

// ================================================================ rows 133-136

#[test]
fn rows133_136_load_callback() {
    // A document with 1-, 2-, 3- and 4-byte UTF-8 so small chunk sizes straddle
    // multi-byte sequences (stream_t.buffer[5] is refilled byte-by-byte).
    let utf8_doc = "{\"k\u{e9}\":[\"\u{20ac}\u{1D11E}\",\"a\u{4e2d}b\",1,2.5,true,null]}"
        .as_bytes()
        .to_vec();

    // rows 133 / 134: 1-, 2- and 3-byte chunks
    for chunk in [1usize, 2, 3, 4, 5, 7] {
        let d = utf8_doc.clone();
        let label = format!("row133/callback chunk={}", chunk);
        diff(&label, move |lib| unsafe {
            p_load_callback(lib, &d, chunk, usize::MAX, 0, DUMP)
        });
        let d = utf8_doc.clone();
        let label = format!("row134/callback chunk={} ENSURE_ASCII", chunk);
        diff(&label, move |lib| unsafe {
            p_load_callback(lib, &d, chunk, usize::MAX, 0, DUMP | JSON_ENSURE_ASCII)
        });
    }
    // a 4-byte sequence deliberately split 2+2 and 3+1
    for (tag, doc) in [
        ("4byte-split", "[\"\u{1D11E}\"]".as_bytes().to_vec()),
        ("3byte-split", "[\"\u{20ac}\"]".as_bytes().to_vec()),
    ] {
        for chunk in [2usize, 3] {
            let d = doc.clone();
            let label = format!("row134/{} chunk={}", tag, chunk);
            diff(&label, move |lib| unsafe {
                p_load_callback(lib, &d, chunk, usize::MAX, 0, DUMP)
            });
        }
    }
    // truncated multi-byte sequence delivered in tiny chunks
    diff("row134/truncated utf8 chunk=1", |lib| unsafe {
        p_load_callback(lib, b"[\"\xe2\x82\"]", 1, usize::MAX, 0, DUMP)
    });

    // row 135: exactly MAX_BUF_LEN (1024) then 0
    let exactly_1024 = {
        let filler = "a".repeat(1024 - 10);
        let s = format!("{{\"key\":\"{}\"}}", filler);
        assert_eq!(s.len(), 1024);
        s.into_bytes()
    };
    let d = exactly_1024.clone();
    diff("row135/callback exactly 1024 then 0", move |lib| unsafe {
        let (out, err, calls) = p_load_callback(lib, &d, 1024, usize::MAX, 0, DUMP);
        (out.map(|s| (s.len(), s.as_bytes()[0])), err, calls)
    });
    // 1025 bytes: the second refill delivers the tail
    let over_1024 = {
        let filler = "a".repeat(1025 - 10);
        let s = format!("{{\"key\":\"{}\"}}", filler);
        assert_eq!(s.len(), 1025);
        s.into_bytes()
    };
    let d = over_1024.clone();
    diff("row135/callback 1025 bytes", move |lib| unsafe {
        let (out, err, calls) = p_load_callback(lib, &d, 1024, usize::MAX, 0, DUMP);
        (out.map(|s| s.len()), err, calls)
    });
    // a chunk size LARGER than MAX_BUF_LEN is clamped by buflen inside the callback
    let d = over_1024.clone();
    diff("row135/callback chunk=4096 clamped", move |lib| unsafe {
        let (out, err, calls) = p_load_callback(lib, &d, 4096, usize::MAX, 0, DUMP);
        (out.map(|s| s.len()), err, calls)
    });

    // row 136: (size_t)-1 is treated as EOF (load.c:1072)
    diff("row136/callback -1 immediately", |lib| unsafe {
        p_load_callback(lib, br#"{"a":1}"#, 1024, 0, 0, DUMP)
    });
    diff("row136/callback -1 after 1 chunk", |lib| unsafe {
        p_load_callback(lib, br#"{"a":1}"#, 1024, 1, 0, DUMP)
    });
    diff("row136/callback -1 mid document", |lib| unsafe {
        p_load_callback(lib, br#"{"a":1,"b":2}"#, 4, 1, 0, DUMP)
    });
    diff("row136/callback -1 after complete doc", |lib| unsafe {
        p_load_callback(lib, br#"{"a":1}"#, 7, 1, 0, DUMP)
    });
    // empty input via a callback that returns 0 straight away
    diff("row136/callback empty", |lib| unsafe {
        p_load_callback(lib, b"", 1024, usize::MAX, 0, DUMP)
    });

    // NULL callback
    diff("row136/callback NULL", |lib| unsafe {
        let f: Symbol<FnLoadCallbackOpt> = sym(lib, "json_load_callback");
        let mut err = json_error_t::new();
        let j = f(None, std::ptr::null_mut(), 0, &mut err);
        (j.is_null(), err.snapshot())
    });

    // Ground truth
    let (out, err, calls) =
        unsafe { p_load_callback(&libs().c, br#"{"a":1}"#, 1, usize::MAX, 0, DUMP) };
    assert_eq!(out.as_deref(), Some(r#"{"a": 1}"#));
    assert_eq!(err.source, "<callback>");
    // 7 bytes at 1 byte/call, plus the refill that reports EOF.
    assert_eq!(calls, 8, "C: one callback per byte plus the final EOF probe");
    let (out, _, calls) =
        unsafe { p_load_callback(&libs().c, &exactly_1024, 1024, usize::MAX, 0, DUMP) };
    assert!(out.is_some());
    assert_eq!(calls, 2, "C: exactly MAX_BUF_LEN then the 0-length EOF refill");
    let (out, err, _) = unsafe { p_load_callback(&libs().c, b"", 1024, 0, 0, DUMP) };
    assert!(out.is_none(), "C: (size_t)-1 on the first call is EOF");
    assert_eq!(err.code, JSON_ERROR_PREMATURE_END_OF_INPUT);
}

// ================================================================ row 137

#[test]
fn row137_error_null_every_entry_point() {
    let inputs: &[&[u8]] = &[
        br#"{"a":1}"#,
        b"[1,2,3]",
        b"[1,",
        b"42",
        b"",
        br#"["a\u0000b"]"#,
        br#"{"a":1,"a":2}"#,
        b"[1] x",
        "[\"\u{1D11E}\"]".as_bytes(),
        b"[\"\xff\"]",
    ];
    let flags: &[usize] = &[0, JSON_DECODE_ANY, JSON_REJECT_DUPLICATES, 0x1F];

    for t in inputs {
        for lf in flags {
            let label = format!("row137/loads err=NULL {:?} flags=0x{:x}", t, lf);
            diff(&label, move |lib| unsafe {
                let f: Symbol<FnLoads> = sym(lib, "json_loads");
                let buf = cs_bytes(t);
                let j = f(buf.as_ptr() as *const c_char, *lf, std::ptr::null_mut());
                if j.is_null() {
                    return None;
                }
                let out = dumps_to_string(lib, j, DUMP);
                decref(lib, j);
                out
            });
            let label = format!("row137/loadb err=NULL {:?} flags=0x{:x}", t, lf);
            diff(&label, move |lib| unsafe {
                let f: Symbol<FnLoadb> = sym(lib, "json_loadb");
                let j = f(t.as_ptr() as *const c_char, t.len(), *lf, std::ptr::null_mut());
                if j.is_null() {
                    return None;
                }
                let out = dumps_to_string(lib, j, DUMP);
                decref(lib, j);
                out
            });
            let label = format!("row137/load_callback err=NULL {:?} flags=0x{:x}", t, lf);
            diff(&label, move |lib| unsafe {
                let f: Symbol<FnLoadCallback> = sym(lib, "json_load_callback");
                let mut st = CbState::new(t, 3);
                let j = f(
                    cb_get,
                    &mut st as *mut CbState as *mut c_void,
                    *lf,
                    std::ptr::null_mut(),
                );
                if j.is_null() {
                    return None;
                }
                let out = dumps_to_string(lib, j, DUMP);
                decref(lib, j);
                out
            });
        }
    }

    // NULL-argument paths with error == NULL must not crash either.
    diff("row137/loads NULL string + NULL error", |lib| unsafe {
        let f: Symbol<FnLoads> = sym(lib, "json_loads");
        f(std::ptr::null(), 0, std::ptr::null_mut()).is_null()
    });
    diff("row137/loadb NULL buffer + NULL error", |lib| unsafe {
        let f: Symbol<FnLoadb> = sym(lib, "json_loadb");
        f(std::ptr::null(), 3, 0, std::ptr::null_mut()).is_null()
    });
    diff("row137/loadf NULL input + NULL error", |lib| unsafe {
        let f: Symbol<FnLoadf> = sym(lib, "json_loadf");
        f(std::ptr::null_mut(), 0, std::ptr::null_mut()).is_null()
    });
    diff("row137/loadfd fd=-1 + NULL error", |lib| unsafe {
        let f: Symbol<FnLoadfd> = sym(lib, "json_loadfd");
        f(-1, 0, std::ptr::null_mut()).is_null()
    });
    diff("row137/load_file NULL path + NULL error", |lib| unsafe {
        let f: Symbol<FnLoadFile> = sym(lib, "json_load_file");
        f(std::ptr::null(), 0, std::ptr::null_mut()).is_null()
    });

    // file-backed entry points with error == NULL
    let path = tmp_path("row137");
    std::fs::write(&path, br#"{"a":[1,2],"b":"\u00e9"}"#).unwrap();
    let missing = tmp_path("row137_missing");
    let _ = std::fs::remove_file(&missing);

    let p = path.clone();
    diff("row137/loadf err=NULL", move |lib| unsafe {
        let f: Symbol<FnLoadf> = sym(lib, "json_loadf");
        let cpath = cs(p.to_str().unwrap());
        let mode = cs("rb");
        let fp = fopen(cpath.as_ptr(), mode.as_ptr());
        assert!(!fp.is_null());
        let j = f(fp, 0, std::ptr::null_mut());
        fclose(fp);
        if j.is_null() {
            return None;
        }
        let out = dumps_to_string(lib, j, DUMP);
        decref(lib, j);
        out
    });
    let p = path.clone();
    diff("row137/loadfd err=NULL", move |lib| unsafe {
        let f: Symbol<FnLoadfd> = sym(lib, "json_loadfd");
        let cpath = cs(p.to_str().unwrap());
        let fd = open(cpath.as_ptr(), O_RDONLY);
        assert!(fd >= 0);
        let j = f(fd, 0, std::ptr::null_mut());
        close(fd);
        if j.is_null() {
            return None;
        }
        let out = dumps_to_string(lib, j, DUMP);
        decref(lib, j);
        out
    });
    for (tag, target) in
        [("ok", path.clone()), ("missing", missing.clone())]
    {
        let s = target.to_str().unwrap().to_string();
        let label = format!("row137/load_file err=NULL {}", tag);
        diff(&label, move |lib| unsafe {
            let f: Symbol<FnLoadFile> = sym(lib, "json_load_file");
            let cpath = cs(&s);
            let j = f(cpath.as_ptr(), 0, std::ptr::null_mut());
            if j.is_null() {
                return None;
            }
            let out = dumps_to_string(lib, j, DUMP);
            decref(lib, j);
            out
        });
    }
    let _ = std::fs::remove_file(&path);
}

// ================================================================ randomized

const INTS: &[&str] = &[
    "0", "-0", "1", "-1", "10", "-10", "42", "1000000", "123456789", "9223372036854775807",
    "-9223372036854775808", "999999999999",
];
const INTS_EDGE: &[&str] =
    &["9223372036854775808", "-9223372036854775809", "99999999999999999999999999"];
const REALS: &[&str] = &[
    "0.0", "-0.0", "1.0", "-1.5", "1e2", "1E2", "1e+2", "1e-2", "0.0001", "1.5e308",
    "2.2250738585072014e-308", "5e-324", "3.141592653589793", "-0.5e-3", "1e15", "1e16",
    "1e-4", "1e-5",
];
const REALS_EDGE: &[&str] = &["1e999", "-1e999", "1e400"];

/// Emit random JSON whitespace (safe between tokens only).
fn ws(rng: &mut Rng, out: &mut String) {
    for _ in 0..rng.below(3) {
        out.push(match rng.below(4) {
            0 => ' ',
            1 => '\t',
            2 => '\n',
            _ => '\r',
        });
    }
}

fn push_escaped(rng: &mut Rng, ch: char, out: &mut String) {
    let cp = ch as u32;
    let choice = rng.below(6);
    match ch {
        '"' => out.push_str(if choice < 4 { "\\\"" } else { "\\u0022" }),
        '\\' => out.push_str(if choice < 4 { "\\\\" } else { "\\u005C" }),
        '/' => out.push_str(if choice < 3 { "/" } else { "\\/" }),
        '\u{8}' => out.push_str(if choice < 4 { "\\b" } else { "\\u0008" }),
        '\u{c}' => out.push_str(if choice < 4 { "\\f" } else { "\\u000c" }),
        '\n' => out.push_str(if choice < 4 { "\\n" } else { "\\u000A" }),
        '\r' => out.push_str(if choice < 4 { "\\r" } else { "\\u000d" }),
        '\t' => out.push_str(if choice < 4 { "\\t" } else { "\\u0009" }),
        _ if cp < 0x20 => {
            if choice % 2 == 0 {
                out.push_str(&format!("\\u{:04x}", cp));
            } else {
                out.push_str(&format!("\\u{:04X}", cp));
            }
        }
        _ if choice < 3 => out.push(ch),
        _ if cp <= 0xFFFF => {
            if choice == 3 {
                out.push_str(&format!("\\u{:04x}", cp));
            } else {
                out.push_str(&format!("\\u{:04X}", cp));
            }
        }
        _ => {
            // non-BMP -> explicit UTF-16 surrogate pair, mixed hex case
            let v = cp - 0x10000;
            out.push_str(&format!("\\u{:04X}\\u{:04x}", 0xD800 + (v >> 10), 0xDC00 + (v & 0x3FF)));
        }
    }
}

fn gen_string(rng: &mut Rng, allow_nul: bool, maxlen: usize) -> String {
    let raw = rng.utf8_string(maxlen);
    let mut out = String::from("\"");
    for ch in raw.chars() {
        if allow_nul && rng.below(24) == 0 {
            out.push_str("\\u0000");
        }
        push_escaped(rng, ch, &mut out);
    }
    out.push('"');
    out
}

fn gen_number(rng: &mut Rng) -> String {
    match rng.below(32) {
        0 => INTS_EDGE[rng.below(INTS_EDGE.len() as u64) as usize].to_string(),
        1 => REALS_EDGE[rng.below(REALS_EDGE.len() as u64) as usize].to_string(),
        n if n % 2 == 0 => INTS[rng.below(INTS.len() as u64) as usize].to_string(),
        _ => REALS[rng.below(REALS.len() as u64) as usize].to_string(),
    }
}

fn gen_value(rng: &mut Rng, depth: u32, out: &mut String) {
    // beyond depth 4 emit leaves only, so documents stay small
    let pick = if depth >= 4 { 2 + rng.below(6) } else { rng.below(8) };
    match pick {
        0 => {
            out.push('{');
            let n = rng.below(4);
            let dup = n >= 2 && rng.below(4) == 0;
            for k in 0..n {
                if k > 0 {
                    ws(rng, out);
                    out.push(',');
                }
                ws(rng, out);
                if dup && k < 2 {
                    out.push_str("\"dup\u{e9}key\"");
                } else {
                    out.push_str(&gen_string(rng, false, 6));
                }
                ws(rng, out);
                out.push(':');
                ws(rng, out);
                gen_value(rng, depth + 1, out);
            }
            ws(rng, out);
            out.push('}');
        }
        1 => {
            out.push('[');
            let n = rng.below(5);
            for k in 0..n {
                if k > 0 {
                    ws(rng, out);
                    out.push(',');
                }
                ws(rng, out);
                gen_value(rng, depth + 1, out);
            }
            ws(rng, out);
            out.push(']');
        }
        2 | 3 => out.push_str(&gen_string(rng, true, 10)),
        4 | 5 => out.push_str(&gen_number(rng)),
        6 => out.push_str(if rng.below(2) == 0 { "true" } else { "false" }),
        _ => out.push_str("null"),
    }
}

/// A whole document: usually a container root, occasionally a bare scalar (which
/// only parses with `JSON_DECODE_ANY`), occasionally deeply nested.
fn gen_document(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(0xC0FFEE_1234_5678 ^ seed.wrapping_mul(0x9E37_79B9));
    let mut body = String::new();
    if rng.below(8) == 0 {
        // bare scalar root
        let leaf = 2 + rng.below(6);
        match leaf {
            2 | 3 => body.push_str(&gen_string(&mut rng, true, 10)),
            4 | 5 => body.push_str(&gen_number(&mut rng)),
            6 => body.push_str("true"),
            _ => body.push_str("null"),
        }
    } else {
        // force a container root
        let mut inner = String::new();
        loop {
            inner.clear();
            gen_value(&mut rng, 0, &mut inner);
            if inner.starts_with('{') || inner.starts_with('[') {
                break;
            }
        }
        body = inner;
    }
    // occasionally wrap in deep nesting
    if rng.below(10) == 0 {
        let d = 1 + rng.below(90) as usize;
        let mut s = String::new();
        for _ in 0..d {
            s.push('[');
        }
        s.push_str(&body);
        for _ in 0..d {
            s.push(']');
        }
        body = s;
    }
    let mut doc = String::new();
    ws(&mut rng, &mut doc);
    doc.push_str(&body);
    ws(&mut rng, &mut doc);
    doc.into_bytes()
}

const RAND_LOAD_FLAGS: &[usize] = &[
    0,
    JSON_DECODE_ANY,
    JSON_DECODE_ANY | JSON_ALLOW_NUL,
    JSON_DECODE_ANY | JSON_REJECT_DUPLICATES,
    JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
    JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
    JSON_REJECT_DUPLICATES | JSON_ALLOW_NUL,
    0x1F,
];

fn rand_dump_flags() -> Vec<usize> {
    vec![
        JSON_ENCODE_ANY,
        JSON_ENCODE_ANY | JSON_COMPACT,
        JSON_ENCODE_ANY | JSON_SORT_KEYS,
        JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
        JSON_ENCODE_ANY | json_indent(2),
        JSON_ENCODE_ANY | JSON_ENSURE_ASCII | JSON_SORT_KEYS | JSON_ESCAPE_SLASH | JSON_COMPACT,
        JSON_ENCODE_ANY | json_real_precision(17),
    ]
}

#[test]
fn randomized_documents_flag_matrix() {
    // 448 iterations = every (load flag, dump flag) pair 8 times, with a fresh
    // document each time. Both libraries see byte-identical input because the
    // generator is a pure function of `i`.
    // Guard the generator itself: a differential test over 448 documents that
    // all failed to parse would be worthless, and so would one that never
    // produced escapes / multi-byte UTF-8 / duplicate keys / deep nesting.
    {
        let (mut ok, mut esc, mut sur, mut dup, mut nul, mut mb, mut deep) =
            (0, 0, 0, 0, 0, 0, 0);
        for i in 0..448u64 {
            let d = gen_document(i);
            let s = String::from_utf8_lossy(&d);
            if s.contains("\\u") {
                esc += 1;
            }
            if s.contains("\\uD8") || s.contains("\\uDB") {
                sur += 1;
            }
            if s.contains("dup") {
                dup += 1;
            }
            if s.contains("\\u0000") {
                nul += 1;
            }
            if d.iter().any(|b| *b >= 0x80) {
                mb += 1;
            }
            if s.contains("[[[[[[[[[[") {
                deep += 1;
            }
            if c_loads(&d, JSON_DECODE_ANY | JSON_ALLOW_NUL).0.is_some() {
                ok += 1;
            }
        }
        assert!(ok >= 350, "generator degenerated: only {}/448 documents parse", ok);
        assert!(esc >= 100, "too few \\uXXXX escapes: {}", esc);
        assert!(sur >= 20, "too few surrogate pairs: {}", sur);
        assert!(dup >= 10, "too few duplicate keys: {}", dup);
        assert!(nul >= 10, "too few \\u0000 escapes: {}", nul);
        assert!(mb >= 50, "too little raw multi-byte UTF-8: {}", mb);
        assert!(deep >= 10, "too little deep nesting: {}", deep);
    }

    let dfs = rand_dump_flags();
    diff_n("rows78-138/randomized", 448, |lib, i| unsafe {
        let doc = gen_document(i);
        let lf = RAND_LOAD_FLAGS[(i as usize) % RAND_LOAD_FLAGS.len()];
        let df = dfs[((i as usize) / RAND_LOAD_FLAGS.len()) % dfs.len()];
        let via_loads = p_loads(lib, &doc, lf, df);
        let via_loadb = p_loadb(lib, &doc, doc.len(), lf, df);
        let via_helper = load_then_dump(lib, &doc, lf, df);
        (doc.len(), via_loads, via_loadb, via_helper)
    });
}

#[test]
fn randomized_documents_via_callback_and_files() {
    // Same generator, but through the low-level entry points: chunked callback
    // refills and a real file. 224 iterations.
    let dfs = rand_dump_flags();
    diff_n("rows121-136/randomized entry points", 224, |lib, i| unsafe {
        let doc = gen_document(i ^ 0xABCD_EF01);
        let lf = RAND_LOAD_FLAGS[(i as usize) % RAND_LOAD_FLAGS.len()];
        let df = dfs[((i as usize) / RAND_LOAD_FLAGS.len()) % dfs.len()];
        let chunk = [1usize, 2, 3, 5, 17, 1024][(i as usize) % 6];
        let cb = p_load_callback(lib, &doc, chunk, usize::MAX, lf, df);
        // truncated buffer views, exercising premature-EOF paths deterministically
        let cut = if doc.is_empty() { 0 } else { doc.len() / 2 };
        let trunc = p_loadb(lib, &doc, cut, lf, df);
        (doc.len(), cb, trunc)
    });

    // File-backed: write each document once, then parse it with both libraries.
    let path = tmp_path("rand_files");
    for i in 0..220u64 {
        let doc = gen_document(i ^ 0x5A5A_1357);
        std::fs::write(&path, &doc).unwrap();
        let lf = RAND_LOAD_FLAGS[(i as usize) % RAND_LOAD_FLAGS.len()];
        let df = dfs[((i as usize) / RAND_LOAD_FLAGS.len()) % dfs.len()];
        let p1 = path.clone();
        let label = format!("rows125-130/randomized file i={}", i);
        diff(&label, move |lib| unsafe {
            let a = p_loadf(lib, &p1, lf, df);
            let b = p_loadfd(lib, &p1, lf, df);
            let c = p_load_file(lib, p1.to_str().unwrap(), lf, df);
            (a, b, c)
        });
    }
    let _ = std::fs::remove_file(&path);
}

