//! Differential tests for `src/dump.c` — CONFIGS.md section A, rows 1-88.
//!
//! Every test builds the SAME json tree twice, once with each library's own
//! constructors (or by parsing the same text with each library's `json_loads`),
//! then dumps it through the SAME entry point with the SAME flags in both
//! libraries and compares the produced bytes byte-for-byte.
//!
//! Object iteration order — and therefore the byte layout of every object dump
//! — depends on the hashtable seed, which `both()` pins to `FIXED_SEED` in both
//! libraries before anything else runs.

// The `unsafe` blocks inside the `fn(&Api) -> *mut json_t` maker closures are
// required when those closures are coerced to plain fn pointers in safe
// context, and redundant when the same literal appears inside an `unsafe`
// block; keep them uniform rather than half-and-half.
#![allow(unused_unsafe)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// libc bits the file/fd entry points need (the test process shares libc with
// both shared objects, so a FILE*/fd created here is valid in both).
// ---------------------------------------------------------------------------

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fclose(f: *mut FILE) -> c_int;
    fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: size_t) -> ssize_t;
}

const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const O_RDONLY: c_int = 0;
const F_SETFL: c_int = 4;
const O_NONBLOCK: c_int = 0o4000;

// ---------------------------------------------------------------------------
// Small shared helpers
// ---------------------------------------------------------------------------

fn tmp_dir() -> PathBuf {
    PathBuf::from(std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string()))
}

fn tmp_path(name: &str) -> PathBuf {
    tmp_dir().join(format!("a06_dump.{name}"))
}

/// `json_dumps` + read the bytes + free with the matching allocator.
unsafe fn dumps(api: &Api, j: *const json_t, flags: size_t) -> Option<Vec<u8>> {
    let p = (api.json_dumps)(j, flags);
    let b = cbytes(p);
    jfree(api, p as *mut c_void);
    b
}

/// Dump the two structurally identical trees with the same flags through
/// `json_dumps` in both libraries and compare the raw bytes.
unsafe fn cmp_dump(
    c: &Api,
    r: &Api,
    cj: *const json_t,
    rj: *const json_t,
    flags: size_t,
    ctx: &str,
) -> Option<Vec<u8>> {
    let cb = dumps(c, cj, flags);
    let rb = dumps(r, rj, flags);
    diff_eq!(
        cb.clone().map(Pretty),
        rb.clone().map(Pretty),
        "json_dumps(flags={flags:#x}) [{ctx}]"
    );
    cb
}

/// A `Vec<u8>` that prints as a readable string in divergence messages.
#[derive(PartialEq, Eq, Clone)]
struct Pretty(Vec<u8>);

impl std::fmt::Debug for Pretty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} ({} bytes)", String::from_utf8_lossy(&self.0), self.0.len())
    }
}

/// Build the same value in both libraries by running the same constructor
/// closure against each `Api`.
unsafe fn pair<F>(c: &Api, r: &Api, f: F) -> (*mut json_t, *mut json_t)
where
    F: Fn(&Api) -> *mut json_t,
{
    (f(c), f(r))
}

/// Parse the same text with both libraries. `JSON_DECODE_ANY` so scalars work.
unsafe fn load2(c: &Api, r: &Api, text: &str) -> (*mut json_t, *mut json_t) {
    let t = cs(text);
    let mut ce = json_error_t::new();
    let mut re = json_error_t::new();
    let cj = (c.json_loads)(t.as_ptr(), JSON_DECODE_ANY, &mut ce);
    let rj = (r.json_loads)(t.as_ptr(), JSON_DECODE_ANY, &mut re);
    assert!(!cj.is_null(), "C could not parse {text:?}: {}", ce.text_str());
    diff_eq!(cj.is_null(), rj.is_null(), "json_loads({text:?}) null-ness");
    (cj, rj)
}

unsafe fn jstr(api: &Api, s: &[u8]) -> *mut json_t {
    (api.json_stringn)(s.as_ptr() as *const c_char, s.len())
}

unsafe fn oset(api: &Api, obj: *mut json_t, key: &[u8], v: *mut json_t) {
    let rc = (api.json_object_setn_new_nocheck)(obj, key.as_ptr() as *const c_char, key.len(), v);
    assert_eq!(rc, 0, "{}: json_object_setn_new_nocheck failed", api.which);
}

unsafe fn apush(api: &Api, arr: *mut json_t, v: *mut json_t) {
    let rc = (api.json_array_append_new)(arr, v);
    assert_eq!(rc, 0, "{}: json_array_append_new failed", api.which);
}

// ---------------------------------------------------------------------------
// Randomised document builder
//
// The builder makes every decision from the `Rng` alone and never inspects a
// return value, so running it twice from the same seed — once per library —
// produces two structurally identical trees.
// ---------------------------------------------------------------------------

fn pick_count(rng: &mut Rng, budget: usize) -> usize {
    // Empty / 1 / 2 / a few / many, with "many" only where it cannot blow the
    // total node count up exponentially.
    match rng.below(10) {
        0 => 0,
        1 | 2 => 1,
        3 | 4 | 5 => 2,
        6 | 7 => 3 + rng.below(2),
        _ => {
            if budget <= 1 {
                12 + rng.below(6)
            } else {
                3 + rng.below(2)
            }
        }
    }
}

unsafe fn rand_string_bytes(rng: &mut Rng) -> Vec<u8> {
    match rng.below(6) {
        0 => rng.ascii_string(12).into_bytes(),
        1 | 2 => rng.utf8_string(10).into_bytes(),
        3 => {
            // embedded NUL — dump_string is driven by json_string_length
            let mut b = rng.ascii_string(6).into_bytes();
            let at = rng.below(b.len() + 1);
            b.insert(at, 0);
            b
        }
        4 => (*rng.choice(&[
            "", "/", "\\", "\"", "a/b", "http://x/y", "\u{8}\u{c}\n\r\t", "\u{e9}", "\u{ff}",
            "\u{100}", "\u{7ff}", "\u{20ac}", "\u{ffff}", "\u{d7ff}", "\u{10000}", "\u{1f600}",
            "\u{10ffff}", "a\u{e9}b\u{20ac}c\u{1f600}d",
        ]))
        .as_bytes()
        .to_vec(),
        _ => {
            // every control character, which all take the escape paths
            let mut v: Vec<u8> = (1u8..=0x1f).collect();
            v.push(b'x');
            v
        }
    }
}

fn rand_key(rng: &mut Rng) -> Vec<u8> {
    match rng.below(7) {
        0 => rng.ascii_string(6).into_bytes(),
        1 => rng.utf8_string(5).into_bytes(),
        2 => Vec::new(), // the empty key
        3 => {
            let mut b = vec![b'a', 0u8];
            b.extend_from_slice(rng.ascii_string(3).as_bytes());
            b
        }
        4 => (*rng.choice(&[
            "a", "A", "aa", "aaa", "ab", "b", "B", "z", "Z", "M", "m", "~", "e", "\u{e9}",
            "\u{20ac}", "\u{1f600}",
        ]))
        .as_bytes()
        .to_vec(),
        5 => format!("k{}", rng.below(20)).into_bytes(),
        _ => format!("key{:03}", rng.below(40)).into_bytes(),
    }
}

/// A random json document of bounded depth, covering all 8 json types.
unsafe fn build(api: &Api, rng: &mut Rng, budget: usize) -> *mut json_t {
    // 0=object 1=array 2=string 3=integer 4=real 5=true 6=false 7=null
    let t = if budget == 0 { 2 + rng.below(6) } else { rng.below(8) };
    match t {
        0 => {
            let o = (api.json_object)();
            let n = pick_count(rng, budget);
            for _ in 0..n {
                let k = rand_key(rng);
                let v = build(api, rng, budget - 1);
                oset(api, o, &k, v);
            }
            o
        }
        1 => {
            let a = (api.json_array)();
            let n = pick_count(rng, budget);
            for _ in 0..n {
                let v = build(api, rng, budget - 1);
                apush(api, a, v);
            }
            a
        }
        2 => {
            let s = rand_string_bytes(rng);
            let j = jstr(api, &s);
            assert!(!j.is_null(), "{}: json_stringn rejected {s:?}", api.which);
            j
        }
        3 => (api.json_integer)(rng.json_int()),
        4 => {
            let v = rng.real();
            let j = (api.json_real)(v);
            assert!(!j.is_null(), "{}: json_real rejected {v}", api.which);
            j
        }
        5 => (api.json_true)(),
        6 => (api.json_false)(),
        _ => (api.json_null)(),
    }
}

/// A random document that is always a container, so `flags` without
/// `JSON_ENCODE_ANY` is accepted.
unsafe fn build_container(api: &Api, rng: &mut Rng, budget: usize) -> *mut json_t {
    if rng.bool() {
        let a = (api.json_array)();
        let n = pick_count(rng, budget);
        for _ in 0..n {
            let v = build(api, rng, budget.saturating_sub(1));
            apush(api, a, v);
        }
        a
    } else {
        let o = (api.json_object)();
        let n = pick_count(rng, budget);
        for _ in 0..n {
            let k = rand_key(rng);
            let v = build(api, rng, budget.saturating_sub(1));
            oset(api, o, &k, v);
        }
        o
    }
}

/// Two structurally identical random documents, one per library.
unsafe fn build_pair(c: &Api, r: &Api, seed: u64, budget: usize) -> (*mut json_t, *mut json_t) {
    let mut ra = Rng::new(seed);
    let cj = build(c, &mut ra, budget);
    let mut rb = Rng::new(seed);
    let rj = build(r, &mut rb, budget);
    (cj, rj)
}

unsafe fn build_container_pair(
    c: &Api,
    r: &Api,
    seed: u64,
    budget: usize,
) -> (*mut json_t, *mut json_t) {
    let mut ra = Rng::new(seed);
    let cj = build_container(c, &mut ra, budget);
    let mut rb = Rng::new(seed);
    let rj = build_container(r, &mut rb, budget);
    (cj, rj)
}

/// One fixed nested mixed document, built through the given library, used by the
/// flag cross-product rows. Contains all 8 types, empty/1/many containers,
/// nesting, `/`, mandatory escapes, control chars, 2-/3-/4-byte UTF-8, reals and
/// integers at their bounds.
unsafe fn mixed_doc(api: &Api) -> *mut json_t {
    let root = (api.json_object)();

    oset(api, root, b"z", (api.json_integer)(i64::MIN));
    oset(api, root, b"\xc3\xa9", (api.json_real)(1.0 / 3.0));

    let arr = (api.json_array)();
    apush(api, arr, (api.json_null)());
    apush(api, arr, (api.json_true)());
    apush(api, arr, (api.json_false)());
    apush(api, arr, (api.json_integer)(0));
    apush(api, arr, (api.json_real)(-1.5));
    apush(api, arr, jstr(api, b"str/with\\slash\"and\ttab"));
    apush(api, arr, (api.json_array)()); // empty array
    apush(api, arr, (api.json_object)()); // empty object
    let inner = (api.json_array)();
    apush(api, inner, jstr(api, b"a\xc3\xa9b\xe2\x82\xacc\xf0\x9f\x98\x8ad"));
    apush(api, inner, (api.json_integer)(i64::MAX));
    apush(api, arr, inner);
    oset(api, root, b"a", arr);

    let nested = (api.json_object)();
    oset(api, nested, b"A", jstr(api, b"\x01\x02\x1f"));
    oset(api, nested, b"", jstr(api, b"/"));
    oset(api, nested, b"aa", (api.json_real)(1e308));
    let deep = (api.json_array)();
    apush(api, deep, {
        let d2 = (api.json_object)();
        oset(api, d2, b"d", (api.json_array)());
        d2
    });
    oset(api, nested, b"deep", deep);
    oset(api, root, b"M", nested);

    oset(api, root, b"m", jstr(api, b"x"));
    oset(api, root, b"aa", (api.json_real)(0.0));
    // A real whose shortest representation needs all 17 digits, so that EVERY
    // bit of the 5-bit JSON_REAL_PRECISION field is observable (row 67).
    oset(api, root, b"p", (api.json_real)(0.30000000000000004));
    root
}

// ---------------------------------------------------------------------------
// Recording / failing dump callback
// ---------------------------------------------------------------------------

const NO_FAIL: usize = usize::MAX;

struct Rec {
    chunks: Vec<Vec<u8>>,
    fail_at: usize,
    fail_ret: c_int,
}

impl Rec {
    fn new() -> Rec {
        Rec { chunks: Vec::new(), fail_at: NO_FAIL, fail_ret: -1 }
    }
    fn failing(at: usize, ret: c_int) -> Rec {
        Rec { chunks: Vec::new(), fail_at: at, fail_ret: ret }
    }
    fn joined(&self) -> Vec<u8> {
        self.chunks.iter().flatten().copied().collect()
    }
    fn pretty(&self) -> Vec<Pretty> {
        self.chunks.iter().cloned().map(Pretty).collect()
    }
}

unsafe extern "C" fn rec_cb(buf: *const c_char, size: size_t, data: *mut c_void) -> c_int {
    let rec = &mut *(data as *mut Rec);
    let bytes = if size == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(buf as *const u8, size).to_vec()
    };
    let idx = rec.chunks.len();
    rec.chunks.push(bytes);
    if idx == rec.fail_at {
        rec.fail_ret
    } else {
        0
    }
}

/// Run `json_dump_callback` with a recording callback and return
/// (return value, chunk list).
unsafe fn record(api: &Api, j: *const json_t, flags: size_t, rec: &mut Rec) -> c_int {
    (api.json_dump_callback)(j, Some(rec_cb), rec as *mut Rec as *mut c_void, flags)
}

// ===========================================================================
// Rows 1-2 — flags = 0 and JSON_INDENT(0)
// ===========================================================================

const SEED_R01: u64 = 0xA06_0001;

#[test]
fn r01_r02_no_indent_no_compact_and_indent0_are_the_same_bits() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 1: the documented expectation for [1,2,3].
        let (cj, rj) = load2(c, r, "[1,2,3]");
        let out = cmp_dump(c, r, cj, rj, 0, "row1 [1,2,3]");
        assert_eq!(out.as_deref(), Some(&b"[1, 2, 3]"[..]), "C: row 1 expectation");

        // Row 2: JSON_INDENT(0) sets no bits at all.
        assert_eq!(json_indent(0), 0, "JSON_INDENT(0) must be 0");
        let out0 = cmp_dump(c, r, cj, rj, json_indent(0), "row2 [1,2,3]");
        diff_eq!(out.map(Pretty), out0.map(Pretty), "row2: INDENT(0) == flags 0");
        decref(c, cj);
        decref(r, rj);

        // Randomised: many documents, both spellings, always byte-identical.
        let mut seeds = Rng::new(SEED_R01);
        for i in 0..250 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            let a = cmp_dump(c, r, cj, rj, 0, &format!("row1 rand #{i} seed {s:#x}"));
            let b = cmp_dump(c, r, cj, rj, json_indent(0), &format!("row2 rand #{i}"));
            diff_eq!(a.map(Pretty), b.map(Pretty), "row2: INDENT(0)==0 on rand #{i}");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 3-9 — every JSON_INDENT(n) value and the 32-space chunking loop
// ===========================================================================

const SEED_R03: u64 = 0xA06_0003;

#[test]
fn r03_r09_indent_values_and_whitespace_chunking() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // (row, indent, input)
        let cases: &[(u32, size_t, &str)] = &[
            (3, 1, "[1,[2,3],{\"a\":4}]"),
            (4, 2, "{\"a\":[1,2],\"b\":{\"c\":null}}"),
            (5, 4, "[[[[1]]]]"),
            (6, 31, "[1]"),
            (7, 31, "[[1]]"),
            (8, 31, "[[[1]]]"),
            (9, 5, "[[[1]]]"),
            (9, 5, "[[[[[[[1]]]]]]]"),
            // depth 11 at indent 31 => 341 spaces, 11 chunks
            (8, 31, "[[[[[[[[[[[1]]]]]]]]]]]"),
        ];
        for &(row, ind, text) in cases {
            let (cj, rj) = load2(c, r, text);
            let out = cmp_dump(
                c,
                r,
                cj,
                rj,
                json_indent(ind),
                &format!("row{row} INDENT({ind}) {text}"),
            )
            .expect("C returned NULL");
            // Every newline must be followed by a multiple of `ind` spaces.
            for seg in out.split(|&b| b == b'\n').skip(1) {
                let sp = seg.iter().take_while(|&&b| b == b' ').count();
                assert_eq!(
                    sp % ind,
                    0,
                    "C: indent {ind} produced {sp} spaces after a newline in {:?}",
                    String::from_utf8_lossy(&out)
                );
            }
            decref(c, cj);
            decref(r, rj);
        }

        // Row 6 boundary spelled out: 31 spaces exactly, one chunk.
        let (cj, rj) = load2(c, r, "[1]");
        let out = cmp_dump(c, r, cj, rj, json_indent(31), "row6 [1] INDENT(31)").unwrap();
        assert_eq!(out, format!("[\n{}1\n]", " ".repeat(31)).into_bytes(), "C: row 6");
        decref(c, cj);
        decref(r, rj);

        // Row 7: 62 spaces => the chunk loop runs twice.
        let (cj, rj) = load2(c, r, "[[1]]");
        let out = cmp_dump(c, r, cj, rj, json_indent(31), "row7 [[1]] INDENT(31)").unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains(&" ".repeat(62)), "C: row 7 wants a 62-space run: {s:?}");
        decref(c, cj);
        decref(r, rj);

        // Row 8: 93 spaces => three chunks.
        let (cj, rj) = load2(c, r, "[[[1]]]");
        let out = cmp_dump(c, r, cj, rj, json_indent(31), "row8 [[[1]]] INDENT(31)").unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains(&" ".repeat(93)), "C: row 8 wants a 93-space run");
        decref(c, cj);
        decref(r, rj);

        // Randomised sweep over every indent value the field can hold.
        let mut seeds = Rng::new(SEED_R03);
        for i in 0..220 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            for ind in 0..=31usize {
                cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    json_indent(ind),
                    &format!("rows3-9 rand #{i} INDENT({ind}) seed {s:#x}"),
                );
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Row 10 — indent x empty containers (the early returns skip dump_indent)
// ===========================================================================

const SEED_R10: u64 = 0xA06_0010;

#[test]
fn r10_empty_containers_have_no_interior_whitespace_at_any_indent() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for text in ["[]", "{}", "[[],{}]", "{\"a\":[],\"b\":{}}", "[[[]]]", "[{},{}]"] {
            let (cj, rj) = load2(c, r, text);
            for ind in [0usize, 1, 2, 4, 5, 31] {
                let out = cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    json_indent(ind),
                    &format!("row10 {text} INDENT({ind})"),
                )
                .unwrap();
                let s = String::from_utf8(out).unwrap();
                assert!(!s.contains("[\n]") && !s.contains("{\n}"), "C: row 10 {s:?}");
                assert!(s.contains("[]") || s.contains("{}"), "C: row 10 {s:?}");
            }
            decref(c, cj);
            decref(r, rj);
        }

        // Randomised: documents deliberately rich in empty containers.
        let mut rng = Rng::new(SEED_R10);
        for i in 0..200 {
            let seed = rng.next_u64();
            let mk = |api: &Api| {
                let mut g = Rng::new(seed);
                let root = (api.json_array)();
                for _ in 0..(1 + g.below(6)) {
                    match g.below(4) {
                        0 => apush(api, root, (api.json_array)()),
                        1 => apush(api, root, (api.json_object)()),
                        2 => {
                            let a = (api.json_array)();
                            apush(api, a, (api.json_object)());
                            apush(api, a, (api.json_array)());
                            apush(api, root, a);
                        }
                        _ => {
                            let o = (api.json_object)();
                            oset(api, o, b"e", (api.json_array)());
                            oset(api, o, b"f", (api.json_object)());
                            apush(api, root, o);
                        }
                    }
                }
                root
            };
            let (cj, rj) = pair(c, r, mk);
            for ind in [0usize, 1, 2, 3, 31] {
                cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    json_indent(ind),
                    &format!("row10 rand #{i} INDENT({ind})"),
                );
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 11-16 — JSON_COMPACT alone and crossed with every indent
// ===========================================================================

const SEED_R11: u64 = 0xA06_0011;

#[test]
fn r11_r16_compact_separators_and_compact_times_indent() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Rows 11/12: the separator and the post-comma space.
        let (cj, rj) = load2(c, r, "{\"a\":1,\"b\":[2,3]}");
        let comp = cmp_dump(c, r, cj, rj, JSON_COMPACT, "row11 COMPACT").unwrap();
        assert_eq!(comp, b"{\"a\":1,\"b\":[2,3]}".to_vec(), "C: row 11");
        let plain = cmp_dump(c, r, cj, rj, 0, "row12 flags=0").unwrap();
        assert_eq!(plain, b"{\"a\": 1, \"b\": [2, 3]}".to_vec(), "C: row 12");
        // Row 13: COMPACT|INDENT(0) is the same bits as COMPACT.
        let c13 = cmp_dump(
            c,
            r,
            cj,
            rj,
            JSON_COMPACT | json_indent(0),
            "row13 COMPACT|INDENT(0)",
        )
        .unwrap();
        assert_eq!(c13, comp, "C: row 13 must equal row 11");
        decref(c, cj);
        decref(r, rj);

        // Row 14: indent wins in dump_indent but the separator stays ":".
        let (cj, rj) = load2(c, r, "{\"a\":1,\"b\":2}");
        let out = cmp_dump(
            c,
            r,
            cj,
            rj,
            JSON_COMPACT | json_indent(1),
            "row14 COMPACT|INDENT(1)",
        )
        .unwrap();
        assert_eq!(out, b"{\n \"a\":1,\n \"b\":2\n}".to_vec(), "C: row 14");
        decref(c, cj);
        decref(r, rj);

        // Row 15: more depths, and the ": " -> ":" difference is the ONLY one.
        let (cj, rj) = load2(c, r, "{\"a\":[1,{\"b\":2}]}");
        for ind in [2usize, 4] {
            let comp = cmp_dump(
                c,
                r,
                cj,
                rj,
                JSON_COMPACT | json_indent(ind),
                &format!("row15 COMPACT|INDENT({ind})"),
            )
            .unwrap();
            let plain = cmp_dump(
                c,
                r,
                cj,
                rj,
                json_indent(ind),
                &format!("row15 INDENT({ind})"),
            )
            .unwrap();
            assert_eq!(
                plain.iter().filter(|&&b| b != b' ').count(),
                comp.iter().filter(|&&b| b != b' ').count(),
                "C: row 15 only whitespace may differ"
            );
            assert_eq!(
                plain.len(),
                comp.len() + plain.windows(2).filter(|w| w == b": ").count(),
                "C: row 15 exactly one space per separator differs"
            );
        }
        decref(c, cj);
        decref(r, rj);

        // Row 16: COMPACT | INDENT(31).
        let (cj, rj) = load2(c, r, "[1,2]");
        let out = cmp_dump(
            c,
            r,
            cj,
            rj,
            JSON_COMPACT | json_indent(31),
            "row16 COMPACT|INDENT(31)",
        )
        .unwrap();
        let sp = " ".repeat(31);
        assert_eq!(out, format!("[\n{sp}1,\n{sp}2\n]").into_bytes(), "C: row 16");
        decref(c, cj);
        decref(r, rj);

        // Randomised COMPACT x every indent.
        let mut seeds = Rng::new(SEED_R11);
        for i in 0..220 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            for ind in [0usize, 1, 2, 4, 5, 31] {
                for compact in [0, JSON_COMPACT] {
                    cmp_dump(
                        c,
                        r,
                        cj,
                        rj,
                        compact | json_indent(ind),
                        &format!("rows11-16 rand #{i} COMPACT={compact:#x} INDENT({ind})"),
                    );
                }
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 17-23 — JSON_ENSURE_ASCII over every UTF-8 sequence length
// ===========================================================================

const SEED_R17: u64 = 0xA06_0017;

#[test]
fn r17_r23_ensure_ascii_over_all_utf8_lengths() {
    let _g = global_state_lock();
    let (c, r) = both();
    let any = JSON_ENCODE_ANY;
    let asc = JSON_ENSURE_ASCII | JSON_ENCODE_ANY;
    unsafe {
        // Row 17: pure ASCII printables, no escapes at all.
        let s17 = b"abc XYZ 019 !#$%&'()*+,-.:;<=>?@[]^_`{|}~";
        let (cj, rj) = pair(c, r, |a| jstr(a, s17));
        let out = cmp_dump(c, r, cj, rj, asc, "row17 ascii printables").unwrap();
        let mut want = vec![b'"'];
        want.extend_from_slice(s17);
        want.push(b'"');
        assert_eq!(out, want, "C: row 17 must be a single verbatim run");
        decref(c, cj);
        decref(r, rj);

        // Rows 18/19/20: 2-, 3- and 4-byte sequences, and row 21 without ASCII.
        let cases: &[(u32, &str, &str)] = &[
            (18, "\u{e9}", "\\u00E9"),
            (18, "\u{ff}", "\\u00FF"),
            (18, "\u{100}", "\\u0100"),
            (18, "\u{7ff}", "\\u07FF"),
            (19, "\u{20ac}", "\\u20AC"),
            (19, "\u{ffff}", "\\uFFFF"),
            (19, "\u{d7ff}", "\\uD7FF"),
            (19, "\u{800}", "\\u0800"),
            (20, "\u{10000}", "\\uD800\\uDC00"),
            (20, "\u{1f600}", "\\uD83D\\uDE00"),
            (20, "\u{10ffff}", "\\uDBFF\\uDFFF"),
        ];
        for &(row, text, esc) in cases {
            let bytes = text.as_bytes().to_vec();
            let (cj, rj) = pair(c, r, |a| jstr(a, &bytes));
            let out =
                cmp_dump(c, r, cj, rj, asc, &format!("row{row} ENSURE_ASCII {text:?}")).unwrap();
            assert_eq!(
                out,
                format!("\"{esc}\"").into_bytes(),
                "C: row {row} escape of {text:?}"
            );
            // Row 21: without ENSURE_ASCII the raw UTF-8 bytes pass through.
            let raw = cmp_dump(c, r, cj, rj, any, &format!("row21 raw {text:?}")).unwrap();
            let mut want = vec![b'"'];
            want.extend_from_slice(text.as_bytes());
            want.push(b'"');
            assert_eq!(raw, want, "C: row 21 verbatim {text:?}");
            decref(c, cj);
            decref(r, rj);
        }

        // Row 22: alternating ASCII / non-ASCII runs.
        let s22 = "a\u{e9}b\u{20ac}c\u{1f600}d".as_bytes().to_vec();
        let (cj, rj) = pair(c, r, |a| jstr(a, &s22));
        let out = cmp_dump(c, r, cj, rj, asc, "row22 alternating").unwrap();
        assert_eq!(out, b"\"a\\u00E9b\\u20ACc\\uD83D\\uDE00d\"".to_vec(), "C: row 22");
        cmp_dump(c, r, cj, rj, any, "row22 alternating raw");
        decref(c, cj);
        decref(r, rj);

        // Row 23: leading and trailing non-ASCII.
        for (text, want) in [
            ("\u{e9}abc", "\"\\u00E9abc\""),
            ("abc\u{e9}", "\"abc\\u00E9\""),
            ("\u{e9}", "\"\\u00E9\""),
            ("\u{e9}\u{e9}", "\"\\u00E9\\u00E9\""),
        ] {
            let bytes = text.as_bytes().to_vec();
            let (cj, rj) = pair(c, r, |a| jstr(a, &bytes));
            let out = cmp_dump(c, r, cj, rj, asc, &format!("row23 {text:?}")).unwrap();
            assert_eq!(out, want.as_bytes().to_vec(), "C: row 23 {text:?}");
            decref(c, cj);
            decref(r, rj);
        }

        // Randomised: many UTF-8 strings, with and without ENSURE_ASCII.
        let mut rng = Rng::new(SEED_R17);
        for i in 0..300 {
            let s = rng.utf8_string(24).into_bytes();
            let (cj, rj) = pair(c, r, |a| jstr(a, &s));
            assert!(!cj.is_null(), "C: json_stringn rejected {s:?}");
            for f in [any, asc, any | JSON_ESCAPE_SLASH, asc | JSON_ESCAPE_SLASH] {
                let out = cmp_dump(c, r, cj, rj, f, &format!("rows17-23 rand #{i}")).unwrap();
                if f & JSON_ENSURE_ASCII != 0 {
                    assert!(out.iter().all(|&b| b < 0x80), "C: ENSURE_ASCII leaked a byte >= 0x80");
                }
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 24-25 — JSON_ESCAPE_SLASH off / on
// ===========================================================================

const SEED_R24: u64 = 0xA06_0024;

#[test]
fn r24_r25_escape_slash_off_and_on() {
    let _g = global_state_lock();
    let (c, r) = both();
    let any = JSON_ENCODE_ANY;
    unsafe {
        let inputs: &[(&[u8], &str, &str)] = &[
            (b"/", "\"/\"", "\"\\/\""),
            (b"a/b", "\"a/b\"", "\"a\\/b\""),
            (b"//", "\"//\"", "\"\\/\\/\""),
            (b"http://x/y", "\"http://x/y\"", "\"http:\\/\\/x\\/y\""),
            (b"\\/", "\"\\\\/\"", "\"\\\\\\/\""),
            (b"/a", "\"/a\"", "\"\\/a\""),
            (b"a/", "\"a/\"", "\"a\\/\""),
        ];
        for &(input, want_off, want_on) in inputs {
            let (cj, rj) = pair(c, r, |a| jstr(a, input));
            let off = cmp_dump(c, r, cj, rj, any, &format!("row24 {input:?}")).unwrap();
            assert_eq!(off, want_off.as_bytes().to_vec(), "C: row 24 {input:?}");
            let on = cmp_dump(
                c,
                r,
                cj,
                rj,
                any | JSON_ESCAPE_SLASH,
                &format!("row25 {input:?}"),
            )
            .unwrap();
            assert_eq!(on, want_on.as_bytes().to_vec(), "C: row 25 {input:?}");
            decref(c, cj);
            decref(r, rj);
        }

        // Randomised strings full of slashes.
        let mut rng = Rng::new(SEED_R24);
        for i in 0..250 {
            let n = rng.below(20);
            let s: Vec<u8> = (0..n).map(|_| *rng.choice(b"/ab\\\"/")).collect();
            let (cj, rj) = pair(c, r, |a| jstr(a, &s));
            for f in [any, any | JSON_ESCAPE_SLASH] {
                cmp_dump(c, r, cj, rj, f, &format!("rows24-25 rand #{i}"));
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 26-28 — mandatory escapes, all control chars, embedded NULs
// ===========================================================================

const SEED_R26: u64 = 0xA06_0026;

#[test]
fn r26_r28_mandatory_escapes_control_chars_and_embedded_nul() {
    let _g = global_state_lock();
    let (c, r) = both();
    let any = JSON_ENCODE_ANY;
    let asc = JSON_ENCODE_ANY | JSON_ENSURE_ASCII;
    unsafe {
        // Row 26: the two mandatory escapes plus the five named control escapes.
        let s26: &[u8] = b"\"\\\x08\x0c\n\r\t";
        let (cj, rj) = pair(c, r, |a| jstr(a, s26));
        let out = cmp_dump(c, r, cj, rj, any, "row26 named escapes").unwrap();
        assert_eq!(out, b"\"\\\"\\\\\\b\\f\\n\\r\\t\"".to_vec(), "C: row 26");
        // Escaping is unconditional: identical with every string flag combo.
        for f in [asc, any | JSON_ESCAPE_SLASH, asc | JSON_ESCAPE_SLASH] {
            let o = cmp_dump(c, r, cj, rj, f, "row26 other flags").unwrap();
            assert_eq!(o, out, "C: row 26 escapes are not flag-gated");
        }
        decref(c, cj);
        decref(r, rj);

        // Row 27: every control char 0x01..0x1F, identical with/without ASCII.
        let s27: Vec<u8> = (1u8..=0x1f).collect();
        let (cj, rj) = pair(c, r, |a| jstr(a, &s27));
        let plain = cmp_dump(c, r, cj, rj, any, "row27 controls").unwrap();
        let ascii = cmp_dump(c, r, cj, rj, asc, "row27 controls ENSURE_ASCII").unwrap();
        assert_eq!(plain, ascii, "C: row 27 ENSURE_ASCII changes nothing for controls");
        let mut want = String::from("\"");
        for b in 1u8..=0x1f {
            match b {
                0x08 => want.push_str("\\b"),
                0x09 => want.push_str("\\t"),
                0x0a => want.push_str("\\n"),
                0x0c => want.push_str("\\f"),
                0x0d => want.push_str("\\r"),
                _ => want.push_str(&format!("\\u{:04X}", b)),
            }
        }
        want.push('"');
        assert_eq!(plain, want.into_bytes(), "C: row 27");
        decref(c, cj);
        decref(r, rj);

        // Row 28: embedded NULs via json_stringn.
        for (input, want) in [
            (&b"a\0b"[..], "\"a\\u0000b\""),
            (b"\0ab", "\"\\u0000ab\""),
            (b"ab\0", "\"ab\\u0000\""),
            (b"\0\0", "\"\\u0000\\u0000\""),
            (b"\0", "\"\\u0000\""),
        ] {
            let (cj, rj) = pair(c, r, |a| jstr(a, input));
            assert!(!cj.is_null(), "C: json_stringn rejected {input:?}");
            diff_eq!(
                (c.json_string_length)(cj),
                (r.json_string_length)(rj),
                "row28 length of {input:?}"
            );
            let out = cmp_dump(c, r, cj, rj, any, &format!("row28 {input:?}")).unwrap();
            assert_eq!(out, want.as_bytes().to_vec(), "C: row 28 {input:?}");
            decref(c, cj);
            decref(r, rj);
        }

        // Randomised byte soup out of exactly the branchy bytes.
        let mut rng = Rng::new(SEED_R26);
        for i in 0..300 {
            let n = rng.below(24);
            let s: Vec<u8> = (0..n)
                .map(|_| match rng.below(4) {
                    0 => rng.below(0x20) as u8,
                    1 => *rng.choice(b"\"\\/\x08\x0c\n\r\t"),
                    _ => 0x20 + rng.below(0x5f) as u8,
                })
                .collect();
            let (cj, rj) = pair(c, r, |a| jstr(a, &s));
            assert!(!cj.is_null(), "C: json_stringn rejected {s:?}");
            for f in [any, asc, any | JSON_ESCAPE_SLASH, asc | JSON_ESCAPE_SLASH] {
                cmp_dump(c, r, cj, rj, f, &format!("rows26-28 rand #{i}"));
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 29-36 — key order: unsorted branch, PRESERVE_ORDER, SORT_KEYS
// ===========================================================================

const SEED_R29: u64 = 0xA06_0029;

#[test]
fn r29_r36_object_key_order_and_sort_keys() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Rows 29/30: unsorted branch, and PRESERVE_ORDER is never tested.
        let mk29 = |api: &Api| {
            let o = (api.json_object)();
            for (i, k) in [&b"z"[..], b"a", b"M", b"m", b"aa"].iter().enumerate() {
                oset(api, o, k, (api.json_integer)(i as i64));
            }
            o
        };
        let (cj, rj) = pair(c, r, mk29);
        let unsorted = cmp_dump(c, r, cj, rj, 0, "row29 unsorted").unwrap();
        let preserve = cmp_dump(c, r, cj, rj, JSON_PRESERVE_ORDER, "row30 PRESERVE_ORDER").unwrap();
        assert_eq!(preserve, unsorted, "C: row 30 PRESERVE_ORDER is a no-op");
        let sorted = cmp_dump(c, r, cj, rj, JSON_SORT_KEYS, "row30 SORT_KEYS").unwrap();
        let both_flags = cmp_dump(
            c,
            r,
            cj,
            rj,
            JSON_SORT_KEYS | JSON_PRESERVE_ORDER,
            "row30 SORT|PRESERVE",
        )
        .unwrap();
        assert_eq!(both_flags, sorted, "C: row 30 sort wins, PRESERVE ignored");
        assert_eq!(
            sorted,
            b"{\"M\": 2, \"a\": 1, \"aa\": 4, \"m\": 3, \"z\": 0}".to_vec(),
            "C: rows 29-31 byte-wise sort"
        );
        decref(c, cj);
        decref(r, rj);

        // Row 31: case differences sort by raw byte value.
        let mk31 = |api: &Api| {
            let o = (api.json_object)();
            for (i, k) in [&b"a"[..], b"A", b"b", b"B", b"Z", b"z"].iter().enumerate() {
                oset(api, o, k, (api.json_integer)(i as i64));
            }
            o
        };
        let (cj, rj) = pair(c, r, mk31);
        let out = cmp_dump(c, r, cj, rj, JSON_SORT_KEYS | JSON_COMPACT, "row31").unwrap();
        assert_eq!(
            out,
            b"{\"A\":1,\"B\":3,\"Z\":4,\"a\":0,\"b\":2,\"z\":5}".to_vec(),
            "C: row 31"
        );
        decref(c, cj);
        decref(r, rj);

        // Row 32: prefixes and the empty key — the `res == 0` length tiebreak.
        let mk32 = |api: &Api| {
            let o = (api.json_object)();
            for (i, k) in [&b"a"[..], b"aa", b"aaa", b"ab", b""].iter().enumerate() {
                oset(api, o, k, (api.json_integer)(i as i64));
            }
            o
        };
        let (cj, rj) = pair(c, r, mk32);
        let out = cmp_dump(c, r, cj, rj, JSON_SORT_KEYS | JSON_COMPACT, "row32").unwrap();
        assert_eq!(
            out,
            b"{\"\":4,\"a\":0,\"aa\":1,\"aaa\":2,\"ab\":3}".to_vec(),
            "C: row 32"
        );
        decref(c, cj);
        decref(r, rj);

        // Row 33: UTF-8 keys sort by raw bytes, not by codepoint collation.
        let mk33 = |api: &Api| {
            let o = (api.json_object)();
            let keys: [&[u8]; 6] = [
                "\u{e9}".as_bytes(),
                b"e",
                b"z",
                "\u{20ac}".as_bytes(),
                "\u{1f600}".as_bytes(),
                b"~",
            ];
            for (i, k) in keys.iter().enumerate() {
                oset(api, o, k, (api.json_integer)(i as i64));
            }
            o
        };
        let (cj, rj) = pair(c, r, mk33);
        let out = cmp_dump(c, r, cj, rj, JSON_SORT_KEYS | JSON_COMPACT, "row33").unwrap();
        assert_eq!(
            out,
            "{\"e\":1,\"z\":2,\"~\":5,\"\u{e9}\":0,\"\u{20ac}\":3,\"\u{1f600}\":4}"
                .as_bytes()
                .to_vec(),
            "C: row 33 byte-wise UTF-8 key order"
        );
        decref(c, cj);
        decref(r, rj);

        // Row 34: keys with embedded NULs, sorted and looked up with getn.
        let mk34 = |api: &Api| {
            let o = (api.json_object)();
            for (i, k) in [&b"a\0b"[..], b"a\0c", b"a", b"a\0", b"b"].iter().enumerate() {
                oset(api, o, k, (api.json_integer)(i as i64));
            }
            o
        };
        let (cj, rj) = pair(c, r, mk34);
        diff_eq!(
            (c.json_object_size)(cj),
            (r.json_object_size)(rj),
            "row34 object size"
        );
        assert_eq!((c.json_object_size)(cj), 5, "C: NUL keys are distinct");
        let out = cmp_dump(c, r, cj, rj, JSON_SORT_KEYS | JSON_COMPACT, "row34").unwrap();
        assert_eq!(
            out,
            b"{\"a\":2,\"a\\u0000\":3,\"a\\u0000b\":0,\"a\\u0000c\":1,\"b\":4}".to_vec(),
            "C: row 34"
        );
        cmp_dump(c, r, cj, rj, JSON_COMPACT, "row34 unsorted");
        decref(c, cj);
        decref(r, rj);

        // Row 35: SORT_KEYS on a 1-key and an empty object.
        for (text, want) in [("{\"only\":1}", "{\"only\":1}"), ("{}", "{}")] {
            let (cj, rj) = load2(c, r, text);
            for ind in [0usize, 2, 31] {
                let out = cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    JSON_SORT_KEYS | JSON_COMPACT | json_indent(ind),
                    &format!("row35 {text} INDENT({ind})"),
                )
                .unwrap();
                if ind == 0 {
                    assert_eq!(out, want.as_bytes().to_vec(), "C: row 35 {text}");
                }
            }
            decref(c, cj);
            decref(r, rj);
        }

        // Row 36: SORT_KEYS | INDENT(2) on a many-key object; the sorted and the
        // unsorted branch must produce the same entries in a different order.
        let mk36 = |api: &Api| {
            let o = (api.json_object)();
            for i in 0..12 {
                let k = format!("k{i:02}");
                oset(api, o, k.as_bytes(), (api.json_integer)(i));
            }
            o
        };
        let (cj, rj) = pair(c, r, mk36);
        let s = cmp_dump(
            c,
            r,
            cj,
            rj,
            JSON_SORT_KEYS | json_indent(2),
            "row36 SORT|INDENT(2)",
        )
        .unwrap();
        let u = cmp_dump(c, r, cj, rj, json_indent(2), "row36 unsorted INDENT(2)").unwrap();
        assert_eq!(s.len(), u.len(), "C: row 36 same shape");
        let lines = |v: &[u8]| {
            let mut l: Vec<String> = String::from_utf8(v.to_vec())
                .unwrap()
                .lines()
                .map(|x| x.trim_end_matches(',').trim().to_string())
                .collect();
            l.sort();
            l
        };
        assert_eq!(lines(&s), lines(&u), "C: row 36 same entries");
        // and the sorted output really is sorted
        let mut prev = String::new();
        for line in String::from_utf8(s.clone()).unwrap().lines().skip(1) {
            let t = line.trim();
            if t == "}" {
                break;
            }
            assert!(t > prev.as_str(), "C: row 36 not sorted: {t} after {prev}");
            prev = t.to_string();
        }
        decref(c, cj);
        decref(r, rj);

        // Randomised: sorted vs unsorted vs preserve-order on random objects.
        let mut seeds = Rng::new(SEED_R29);
        for i in 0..250 {
            let seed = seeds.next_u64();
            let mk = |api: &Api| {
                let mut g = Rng::new(seed);
                let o = (api.json_object)();
                let n = pick_count(&mut g, 1);
                for _ in 0..n {
                    let k = rand_key(&mut g);
                    let v = build(api, &mut g, 1);
                    oset(api, o, &k, v);
                }
                o
            };
            let (cj, rj) = pair(c, r, mk);
            for f in [
                0,
                JSON_SORT_KEYS,
                JSON_PRESERVE_ORDER,
                JSON_SORT_KEYS | JSON_PRESERVE_ORDER,
                JSON_SORT_KEYS | json_indent(2),
                JSON_SORT_KEYS | JSON_COMPACT | JSON_ENSURE_ASCII,
            ] {
                cmp_dump(c, r, cj, rj, f, &format!("rows29-36 rand #{i} flags {f:#x}"));
            }
            // PRESERVE_ORDER never changes anything.
            let a = dumps(c, cj, 0);
            let b = dumps(c, cj, JSON_PRESERVE_ORDER);
            assert_eq!(a, b, "C: PRESERVE_ORDER changed the output on rand #{i}");
            let a = dumps(c, cj, JSON_SORT_KEYS);
            let b = dumps(c, cj, JSON_SORT_KEYS | JSON_PRESERVE_ORDER);
            assert_eq!(a, b, "C: PRESERVE_ORDER changed sorted output on rand #{i}");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 37-44 — the JSON_ENCODE_ANY gate and every top-level type
// ===========================================================================

fn scalar_makers() -> Vec<(&'static str, fn(&Api) -> *mut json_t)> {
    vec![
        ("null", |a| unsafe { (a.json_null)() }),
        ("true", |a| unsafe { (a.json_true)() }),
        ("false", |a| unsafe { (a.json_false)() }),
        ("integer", |a| unsafe { (a.json_integer)(-42) }),
        ("real", |a| unsafe { (a.json_real)(1.5) }),
        ("string", |a| unsafe {
            (a.json_string)(b"x\0".as_ptr() as *const c_char)
        }),
    ]
}

#[test]
fn r37_r44_encode_any_gate_and_scalar_top_levels() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 37: without ENCODE_ANY all six non-container types are rejected.
        for (name, mk) in scalar_makers() {
            let (cj, rj) = pair(c, r, mk);
            for f in [0, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS, JSON_EMBED] {
                let cb = dumps(c, cj, f);
                let rb = dumps(r, rj, f);
                diff_eq!(cb.clone(), rb.clone(), "row37 {name} flags {f:#x}");
                assert!(cb.is_none(), "C: row 37 {name} must be rejected at flags {f:#x}");
            }
            decref(c, cj);
            decref(r, rj);
        }

        // Rows 38/39/40/41/42: each scalar with ENCODE_ANY.
        let expect: &[(&str, fn(&Api) -> *mut json_t, &str)] = &[
            ("null", |a| unsafe { (a.json_null)() }, "null"),
            ("true", |a| unsafe { (a.json_true)() }, "true"),
            ("false", |a| unsafe { (a.json_false)() }, "false"),
            ("int 0", |a| unsafe { (a.json_integer)(0) }, "0"),
            ("int -7", |a| unsafe { (a.json_integer)(-7) }, "-7"),
            ("string x", |a| unsafe { jstr(a, b"x") }, "\"x\""),
            ("empty string", |a| unsafe { jstr(a, b"") }, "\"\""),
        ];
        for &(name, mk, want) in expect {
            let (cj, rj) = pair(c, r, mk);
            let out = cmp_dump(c, r, cj, rj, JSON_ENCODE_ANY, &format!("rows38-42 {name}")).unwrap();
            assert_eq!(out, want.as_bytes().to_vec(), "C: {name}");
            decref(c, cj);
            decref(r, rj);
        }

        // Row 41: a bare real always carries a '.' or an 'e'.
        for v in [1.5f64, 1.0, -0.0, 1e308, 1e-308, 1.0 / 3.0, 100.0] {
            let (cj, rj) = pair(c, r, |a| (a.json_real)(v));
            let out = cmp_dump(c, r, cj, rj, JSON_ENCODE_ANY, &format!("row41 real {v}")).unwrap();
            let s = String::from_utf8(out).unwrap();
            assert!(
                s.contains('.') || s.contains('e'),
                "C: row 41 real {v} dumped as {s:?} — would re-decode as an integer"
            );
            decref(c, cj);
            decref(r, rj);
        }

        // Row 43: the gate is a no-op for containers.
        for text in ["[]", "{}", "[1,2]", "{\"a\":1}", "[[1],{\"b\":2}]"] {
            let (cj, rj) = load2(c, r, text);
            let a = cmp_dump(c, r, cj, rj, 0, &format!("row43 {text} flags=0")).unwrap();
            let b = cmp_dump(
                c,
                r,
                cj,
                rj,
                JSON_ENCODE_ANY,
                &format!("row43 {text} ENCODE_ANY"),
            )
            .unwrap();
            assert_eq!(a, b, "C: row 43 {text}");
            decref(c, cj);
            decref(r, rj);
        }

        // Row 44: a NULL json_t*.
        for f in [0, JSON_ENCODE_ANY, JSON_ENCODE_ANY | json_indent(2), JSON_EMBED] {
            let cb = dumps(c, std::ptr::null(), f);
            let rb = dumps(r, std::ptr::null(), f);
            diff_eq!(cb.clone(), rb.clone(), "row44 json_dumps(NULL, {f:#x})");
            assert!(cb.is_none(), "C: row 44 must return NULL at flags {f:#x}");

            diff_eq!(
                (c.json_dumpb)(std::ptr::null(), std::ptr::null_mut(), 0, f),
                (r.json_dumpb)(std::ptr::null(), std::ptr::null_mut(), 0, f),
                "row44 json_dumpb(NULL)"
            );
            let mut crec = Rec::new();
            let mut rrec = Rec::new();
            diff_eq!(
                record(c, std::ptr::null(), f, &mut crec),
                record(r, std::ptr::null(), f, &mut rrec),
                "row44 json_dump_callback(NULL, {f:#x})"
            );
            diff_eq!(crec.pretty(), rrec.pretty(), "row44 chunks for NULL");
            assert!(crec.chunks.is_empty(), "C: row 44 emits nothing");
        }
    }
}

// ===========================================================================
// Rows 45-52 — JSON_REAL_PRECISION and jsonp_dtostr
// ===========================================================================

const SEED_R45: u64 = 0xA06_0045;

#[test]
fn r45_r52_real_precision_sweep() {
    let _g = global_state_lock();
    let (c, r) = both();
    let any = JSON_ENCODE_ANY;
    unsafe {
        // Rows 45-49: the named precisions with their named inputs.
        let cases: &[(u32, size_t, &[f64])] = &[
            (45, 0, &[0.1, 1.0 / 3.0, 3.141592653589793]),
            (46, 1, &[0.1, 123.456, 9.99, 1e308]),
            (47, 6, &[1.0 / 3.0, 2.0 / 7.0, 1234567.0, 0.000123456789]),
            (48, 17, &[0.1, 0.2, 0.30000000000000004, f64::EPSILON]),
            (49, 31, &[0.1, 1e-308, 1e308]),
        ];
        let mut overflowed = Vec::new();
        for &(row, prec, vals) in cases {
            for &v in vals {
                let (cj, rj) = pair(c, r, |a| (a.json_real)(v));
                let out = cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    any | json_real_precision(prec),
                    &format!("row{row} precision {prec} value {v:e}"),
                );
                match &out {
                    None => overflowed.push((prec, v)),
                    Some(b) => {
                        let s = String::from_utf8(b.clone()).unwrap();
                        assert!(
                            s.contains('.') || s.contains('e'),
                            "C: row{row} {v:e} at precision {prec} => {s:?}"
                        );
                        if prec == 17 {
                            // 17 digits must round-trip bit-exactly
                            let back: f64 = s.parse().unwrap();
                            assert_eq!(
                                back.to_bits(),
                                v.to_bits(),
                                "C: row 48 {s} does not round-trip"
                            );
                        }
                    }
                }
                decref(c, cj);
                decref(r, rj);
            }
        }
        // Row 49 records which inputs overflow MAX_REAL_STR_LENGTH.
        assert!(
            !overflowed.is_empty(),
            "C: row 49 — expected precision 31 to overflow the 25-byte buffer for some input, \
             got none (overflow list {overflowed:?})"
        );

        // Row 50: {0,1,6,17,31} x the hard doubles = 35 configurations.
        let hard: &[(&str, f64)] = &[
            ("0.0", 0.0),
            ("-0.0", -0.0),
            ("min subnormal", 5e-324),
            ("min normal", 2.2250738585072014e-308),
            ("max", 1.7976931348623157e308),
            ("1e308", 1e308),
            ("1e-308", 1e-308),
        ];
        for &prec in &[0usize, 1, 6, 17, 31] {
            for &(name, v) in hard {
                let (cj, rj) = pair(c, r, |a| (a.json_real)(v));
                let out = cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    any | json_real_precision(prec),
                    &format!("row50 precision {prec} {name}"),
                );
                if let Some(b) = out {
                    let s = String::from_utf8(b).unwrap();
                    if v == 0.0 && v.is_sign_negative() {
                        assert!(s.starts_with('-'), "C: row 50 -0.0 lost its sign: {s:?}");
                    }
                }
                decref(c, cj);
                decref(r, rj);
            }
        }

        // Row 51: integral reals must not lose their ".0".
        for v in [1.0f64, -2.0, 1e16, 1e17, 100.0, 0.0, -0.0, 1e15] {
            let (cj, rj) = pair(c, r, |a| (a.json_real)(v));
            let out = cmp_dump(c, r, cj, rj, any, &format!("row51 integral real {v:e}")).unwrap();
            let s = String::from_utf8(out).unwrap();
            assert!(
                s.contains('.') || s.contains('e'),
                "C: row 51 {v:e} dumped as {s:?}"
            );
            decref(c, cj);
            decref(r, rj);
        }

        // Row 52: every precision 1..=17, one value needing exactly that many
        // significant digits.
        for n in 1..=17usize {
            let mut txt = String::from("1.");
            for _ in 0..(n.saturating_sub(2)) {
                txt.push('0');
            }
            if n >= 2 {
                txt.push('2');
            } else {
                txt = "1.0".to_string();
            }
            let v: f64 = txt.parse().unwrap();
            for p in 1..=17usize {
                let (cj, rj) = pair(c, r, |a| (a.json_real)(v));
                cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    any | json_real_precision(p),
                    &format!("row52 value {txt} at precision {p}"),
                );
                decref(c, cj);
                decref(r, rj);
            }
        }

        // Randomised: every precision 0..=31 x many doubles.
        let mut rng = Rng::new(SEED_R45);
        for i in 0..260 {
            let v = rng.real();
            let (cj, rj) = pair(c, r, |a| (a.json_real)(v));
            assert!(!cj.is_null(), "C: json_real rejected {v:e}");
            for p in 0..=31usize {
                cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    any | json_real_precision(p),
                    &format!("rows45-52 rand #{i} value {v:e} bits {:#x} precision {p}", v.to_bits()),
                );
            }
            decref(c, cj);
            decref(r, rj);
        }

        // ... and inside containers, where the JSON_REAL arm runs at depth > 0.
        let mut rng = Rng::new(SEED_R45 ^ 0x5555);
        for i in 0..120 {
            let seed = rng.next_u64();
            let mk = |api: &Api| {
                let mut g = Rng::new(seed);
                let a = (api.json_array)();
                for _ in 0..(1 + g.below(8)) {
                    let v = g.real();
                    apush(api, a, (api.json_real)(v));
                }
                a
            };
            let (cj, rj) = pair(c, r, mk);
            for p in [0usize, 1, 6, 17, 31] {
                cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    json_real_precision(p),
                    &format!("rows45-52 nested rand #{i} precision {p}"),
                );
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Row 53 — integers at their bounds
// ===========================================================================

const SEED_R53: u64 = 0xA06_0053;

#[test]
fn r53_integer_bounds() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let vals: &[i64] = &[
            0,
            1,
            -1,
            2147483647,
            -2147483648,
            2147483648,
            -2147483649,
            i64::MAX,
            i64::MIN,
        ];
        for &v in vals {
            let (cj, rj) = pair(c, r, |a| (a.json_integer)(v));
            let out = cmp_dump(
                c,
                r,
                cj,
                rj,
                JSON_ENCODE_ANY,
                &format!("row53 integer {v}"),
            )
            .expect("C: row 53 — no valid json_int_t may overflow the 25-byte buffer");
            assert_eq!(out, v.to_string().into_bytes(), "C: row 53 {v}");
            assert!(out.len() <= 20, "C: row 53 {v} produced {} bytes", out.len());
            decref(c, cj);
            decref(r, rj);
        }

        // Randomised integers, bare and nested.
        let mut rng = Rng::new(SEED_R53);
        for i in 0..300 {
            let v = rng.json_int();
            let (cj, rj) = pair(c, r, |a| (a.json_integer)(v));
            let out = cmp_dump(c, r, cj, rj, JSON_ENCODE_ANY, &format!("row53 rand #{i} {v}"))
                .expect("C: row 53 integer must always fit");
            assert_eq!(out, v.to_string().into_bytes(), "C: row 53 rand {v}");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 54-60 — JSON_EMBED
// ===========================================================================

const SEED_R54: u64 = 0xA06_0054;

#[test]
fn r54_r60_embed() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 54 / 55 / 57.
        let cases: &[(u32, &str, &str)] = &[
            (54, "[1,2,3]", "1, 2, 3"),
            (55, "{\"a\":1,\"b\":2}", "\"a\": 1, \"b\": 2"),
            (57, "[[1],{\"a\":2}]", "[1], {\"a\": 2}"),
        ];
        for &(row, text, want) in cases {
            let (cj, rj) = load2(c, r, text);
            let out = cmp_dump(c, r, cj, rj, JSON_EMBED, &format!("row{row} EMBED {text}")).unwrap();
            assert_eq!(out, want.as_bytes().to_vec(), "C: row {row} {text}");
            decref(c, cj);
            decref(r, rj);
        }

        // Row 56: EMBED on the empty containers yields the empty string, not NULL.
        for text in ["[]", "{}"] {
            let (cj, rj) = load2(c, r, text);
            let out = cmp_dump(c, r, cj, rj, JSON_EMBED, &format!("row56 EMBED {text}"));
            assert_eq!(out, Some(Vec::new()), "C: row 56 {text} must be \"\"");
            // json_dumpb also returns 0 here (indistinguishable from failure).
            let mut buf = [0x55u8; 16];
            diff_eq!(
                (c.json_dumpb)(cj, buf.as_mut_ptr() as *mut c_char, 16, JSON_EMBED),
                (r.json_dumpb)(rj, buf.as_mut_ptr() as *mut c_char, 16, JSON_EMBED),
                "row56 json_dumpb EMBED {text}"
            );
            assert_eq!(
                (c.json_dumpb)(cj, buf.as_mut_ptr() as *mut c_char, 16, JSON_EMBED),
                0,
                "C: row 56 json_dumpb returns 0"
            );
            decref(c, cj);
            decref(r, rj);
        }

        // Row 58: EMBED is cleared and never consulted by the scalar arms.
        for (name, mk) in scalar_makers() {
            let (cj, rj) = pair(c, r, mk);
            let a = cmp_dump(c, r, cj, rj, JSON_ENCODE_ANY, &format!("row58 {name} plain")).unwrap();
            let b = cmp_dump(
                c,
                r,
                cj,
                rj,
                JSON_ENCODE_ANY | JSON_EMBED,
                &format!("row58 {name} EMBED"),
            )
            .unwrap();
            assert_eq!(a, b, "C: row 58 {name}");
            decref(c, cj);
            decref(r, rj);
        }

        // Row 59: EMBED | INDENT(n) — leading "\n<indent>" and trailing "\n".
        for text in ["[1,2]", "{\"a\":1,\"b\":2}"] {
            for ind in [2usize, 4, 31] {
                let (cj, rj) = load2(c, r, text);
                let out = cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    JSON_EMBED | json_indent(ind),
                    &format!("row59 EMBED|INDENT({ind}) {text}"),
                )
                .unwrap();
                let s = String::from_utf8(out).unwrap();
                assert!(
                    s.starts_with(&format!("\n{}", " ".repeat(ind))),
                    "C: row 59 {text} INDENT({ind}) => {s:?}"
                );
                assert!(s.ends_with('\n'), "C: row 59 must end with the depth-0 indent");
                decref(c, cj);
                decref(r, rj);
            }
        }

        // Row 60: EMBED | COMPACT.
        let (cj, rj) = load2(c, r, "{\"a\":1,\"b\":2}");
        let out = cmp_dump(c, r, cj, rj, JSON_EMBED | JSON_COMPACT, "row60").unwrap();
        assert_eq!(out, b"\"a\":1,\"b\":2".to_vec(), "C: row 60");
        decref(c, cj);
        decref(r, rj);

        // Randomised: EMBED crossed with the other encoding flags. Also assert
        // the EMBED output is exactly the non-EMBED output minus its first and
        // last byte, proving `flags &= ~JSON_EMBED` stops the clear propagating.
        let mut seeds = Rng::new(SEED_R54);
        for i in 0..220 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            for extra in [
                0,
                JSON_COMPACT,
                json_indent(2),
                json_indent(31),
                JSON_SORT_KEYS,
                JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
                JSON_COMPACT | json_indent(4) | JSON_SORT_KEYS,
            ] {
                let e = cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    JSON_EMBED | extra,
                    &format!("rows54-60 rand #{i} EMBED|{extra:#x}"),
                )
                .unwrap();
                let p = dumps(c, cj, extra).unwrap();
                assert_eq!(
                    e,
                    p[1..p.len() - 1].to_vec(),
                    "C: EMBED must only remove the outermost delimiters (rand #{i}, extra {extra:#x})"
                );
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 61-65 — feature combinations on torture documents
// ===========================================================================

const SEED_R61: u64 = 0xA06_0061;

#[test]
fn r61_r65_flag_combinations() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 61: INDENT(2) | ENSURE_ASCII | SORT_KEYS on a nested mixed doc.
        let mk61 = |api: &Api| {
            let o = (api.json_object)();
            let arr = (api.json_array)();
            apush(api, arr, (api.json_null)());
            apush(api, arr, (api.json_true)());
            apush(api, arr, (api.json_false)());
            apush(api, arr, (api.json_integer)(0));
            apush(api, arr, (api.json_real)(-1.5));
            apush(api, arr, jstr(api, b"str"));
            oset(api, o, b"z", arr);
            let inner = (api.json_object)();
            oset(api, inner, b"i", (api.json_integer)(1));
            oset(api, o, "\u{e9}".as_bytes(), inner);
            oset(api, o, b"a", (api.json_array)());
            oset(
                api,
                o,
                b"A",
                jstr(api, b"\"\\\n\xc3\xa9\xf0\x9f\x98\x80"),
            );
            o
        };
        let (cj, rj) = pair(c, r, mk61);
        let out = cmp_dump(
            c,
            r,
            cj,
            rj,
            json_indent(2) | JSON_ENSURE_ASCII | JSON_SORT_KEYS,
            "row61",
        )
        .unwrap();
        assert!(out.iter().all(|&b| b < 0x80), "C: row 61 must be pure ASCII");
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("\\u00E9"), "C: row 61 key escape missing: {s}");
        assert!(s.contains("\\uD83D\\uDE00"), "C: row 61 surrogate pair missing");
        // key order: "A" < "a" < "z" < "\xc3\xa9" byte-wise
        let ai = s.find("\"A\"").unwrap();
        let bi = s.find("\"a\"").unwrap();
        let zi = s.find("\"z\"").unwrap();
        let ei = s.find("\"\\u00E9\"").unwrap();
        assert!(ai < bi && bi < zi && zi < ei, "C: row 61 key order wrong: {s}");
        decref(c, cj);
        decref(r, rj);

        // Row 62: COMPACT | ESCAPE_SLASH.
        let mk62 = |api: &Api| {
            let o = (api.json_object)();
            oset(api, o, b"url", jstr(api, b"http://a/b"));
            oset(api, o, b"re", jstr(api, b"a\\/b"));
            o
        };
        let (cj, rj) = pair(c, r, mk62);
        let out = cmp_dump(c, r, cj, rj, JSON_COMPACT | JSON_ESCAPE_SLASH, "row62").unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("http:\\/\\/a\\/b"), "C: row 62 {s}");
        assert!(!s.contains(": "), "C: row 62 must be compact");
        decref(c, cj);
        decref(r, rj);

        // Row 63: bare top-level real at several precisions.
        for v in [1.0f64 / 3.0, -0.0, 1e308] {
            for p in [4usize, 0, 17] {
                let (cj, rj) = pair(c, r, |a| (a.json_real)(v));
                cmp_dump(
                    c,
                    r,
                    cj,
                    rj,
                    JSON_ENCODE_ANY | json_real_precision(p),
                    &format!("row63 bare real {v:e} precision {p}"),
                );
                decref(c, cj);
                decref(r, rj);
            }
        }

        // Row 64: ESCAPE_SLASH break is tested before the ENSURE_ASCII break.
        let s64 = "a/\u{1f600}/b\u{1f600}".as_bytes().to_vec();
        let (cj, rj) = pair(c, r, |a| jstr(a, &s64));
        let out = cmp_dump(
            c,
            r,
            cj,
            rj,
            JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH | JSON_ENCODE_ANY,
            "row64",
        )
        .unwrap();
        assert_eq!(
            out,
            b"\"a\\/\\uD83D\\uDE00\\/b\\uD83D\\uDE00\"".to_vec(),
            "C: row 64"
        );
        decref(c, cj);
        decref(r, rj);

        // Row 65: the 4 ENSURE_ASCII x ESCAPE_SLASH combinations on one torture
        // string containing ", \, /, 0x00-0x1F and 2-/3-/4-byte UTF-8.
        let mut t = Vec::new();
        t.extend_from_slice(b"\"\\a/");
        t.extend(0u8..=0x1f);
        t.extend_from_slice("\u{e9}\u{20ac}\u{1f600}\u{7f}\u{80}\u{7ff}\u{800}\u{ffff}\u{10000}\u{10ffff}".as_bytes());
        t.extend_from_slice(b"tail/\"");
        let n_slash = t.iter().filter(|&&b| b == b'/').count();
        assert_eq!(n_slash, 2, "the row-65 torture string has two slashes");
        let (cj, rj) = pair(c, r, |a| jstr(a, &t));
        assert!(!cj.is_null(), "C: json_stringn rejected the row-65 torture string");
        for asc in [0, JSON_ENSURE_ASCII] {
            let mut lens = Vec::new();
            for esc in [0, JSON_ESCAPE_SLASH] {
                let f = JSON_ENCODE_ANY | asc | esc;
                let out = cmp_dump(c, r, cj, rj, f, "row65 torture").unwrap();
                if asc != 0 {
                    assert!(out.iter().all(|&b| b < 0x80), "C: row 65 ASCII leak");
                }
                assert_eq!(
                    out.iter().filter(|&&b| b == b'/').count(),
                    n_slash,
                    "C: row 65 must keep exactly {n_slash} slashes"
                );
                lens.push(out.len());
            }
            // Every `/` costs exactly one extra byte under ESCAPE_SLASH.
            assert_eq!(
                lens[1],
                lens[0] + n_slash,
                "C: row 65 ESCAPE_SLASH must add one byte per slash (ascii={asc:#x})"
            );
        }
        decref(c, cj);
        decref(r, rj);

        // Randomised: the same four combinations over random documents.
        let mut seeds = Rng::new(SEED_R61);
        for i in 0..220 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            for asc in [0, JSON_ENSURE_ASCII] {
                for esc in [0, JSON_ESCAPE_SLASH] {
                    for extra in [0, json_indent(2) | JSON_SORT_KEYS, JSON_COMPACT] {
                        cmp_dump(
                            c,
                            r,
                            cj,
                            rj,
                            asc | esc | extra,
                            &format!("rows61-65 rand #{i} {:#x}", asc | esc | extra),
                        );
                    }
                }
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Row 66 — the systematic indent x compact x ascii x sort x slash sweep
// ===========================================================================

const SEED_R66: u64 = 0xA06_0066;

#[test]
fn r66_full_flag_cross_product() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let indents = [0usize, 1, 2, 4, 5, 31];

        // First on the one fixed nested mixed document, all 6*2*2*2*2 = 96 sets.
        let (cj, rj) = pair(c, r, |a| mixed_doc(a));
        let mut n_cfg = 0;
        for &ind in &indents {
            for compact in [0, JSON_COMPACT] {
                for ascii in [0, JSON_ENSURE_ASCII] {
                    for sort in [0, JSON_SORT_KEYS] {
                        for slash in [0, JSON_ESCAPE_SLASH] {
                            let f = json_indent(ind) | compact | ascii | sort | slash;
                            cmp_dump(c, r, cj, rj, f, "row66 mixed_doc");
                            n_cfg += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(n_cfg, 96, "row 66 must sweep 96 configurations");
        decref(c, cj);
        decref(r, rj);

        // Then the same 96 configurations over 200 random documents.
        let mut seeds = Rng::new(SEED_R66);
        let mut total_bytes = 0usize;
        let mut biggest = 0usize;
        for i in 0..200 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            for &ind in &indents {
                for compact in [0, JSON_COMPACT] {
                    for ascii in [0, JSON_ENSURE_ASCII] {
                        for sort in [0, JSON_SORT_KEYS] {
                            for slash in [0, JSON_ESCAPE_SLASH] {
                                let f = json_indent(ind) | compact | ascii | sort | slash;
                                let out = cmp_dump(
                                    c,
                                    r,
                                    cj,
                                    rj,
                                    f,
                                    &format!("row66 rand #{i} seed {s:#x}"),
                                );
                                let n = out.map(|v| v.len()).unwrap_or(0);
                                total_bytes += n;
                                biggest = biggest.max(n);
                            }
                        }
                    }
                }
            }
            decref(c, cj);
            decref(r, rj);
        }
        // Guard against the generator silently degenerating into empty
        // documents, which would make the whole sweep vacuous.
        assert!(
            total_bytes > 1_000_000 && biggest > 2_000,
            "row 66 random documents are too small: {total_bytes} bytes total, biggest {biggest}"
        );
    }
}

// ===========================================================================
// Row 67 — which flag bits are live
// ===========================================================================

const SEED_R67: u64 = 0xA06_0067;

#[test]
fn r67_flag_bit_liveness_and_unknown_bits() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (cj, rj) = pair(c, r, |a| mixed_doc(a));

        // The documented named cases.
        for f in [0x800usize, 0x20000, usize::MAX, 0x10000_0000, 0x1_0000_0000_0000] {
            cmp_dump(c, r, cj, rj, f, &format!("row67 flags {f:#x}"));
        }
        // 0x800 is the low bit of REAL_PRECISION, so it MUST change real output.
        let base = dumps(c, cj, 0).unwrap();
        let p1 = dumps(c, cj, 0x800).unwrap();
        assert_ne!(base, p1, "C: row 67 — 0x800 is REAL_PRECISION(1) and must be live");

        // Full single-bit liveness map, compared between the two libraries.
        let mut c_live = Vec::new();
        let mut r_live = Vec::new();
        for bit in 0..25u32 {
            let f = 1usize << bit;
            let out = cmp_dump(c, r, cj, rj, f, &format!("row67 single bit {bit}"));
            c_live.push(dumps(c, cj, f) != Some(base.clone()));
            r_live.push(dumps(r, rj, f) != Some(base.clone()));
            let _ = out;
        }
        diff_eq!(c_live.clone(), r_live.clone(), "row67 flag-bit liveness map");

        // The bits dump.c documents as dead for a container input.
        for bit in [8u32, 9, 17, 18, 19, 20, 21, 22, 23, 24] {
            assert!(
                !c_live[bit as usize],
                "C: row 67 — bit {bit} ({:#x}) should be inert for a container",
                1usize << bit
            );
        }
        // ... and the ones that must be live.
        for bit in [0u32, 1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15, 16] {
            assert!(
                c_live[bit as usize],
                "C: row 67 — bit {bit} ({:#x}) should be live",
                1usize << bit
            );
        }
        decref(c, cj);
        decref(r, rj);

        // Randomised whole-word flags: any bit pattern at all must agree.
        let mut rng = Rng::new(SEED_R67);
        for i in 0..250 {
            let s = rng.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 2);
            for _ in 0..8 {
                let f = rng.next_u64() as usize;
                cmp_dump(c, r, cj, rj, f, &format!("row67 rand #{i} flags {f:#x}"));
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 68-72 — container arity, deep nesting, all 8 types at depth 0 and 1
// ===========================================================================

const SEED_R68: u64 = 0xA06_0068;

#[test]
fn r68_r72_arity_nesting_and_every_type_at_every_depth() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 68/69: arrays of 1, 2 and many elements.
        for n in [1usize, 2, 3, 16, 17, 40] {
            let mk = |api: &Api| {
                let a = (api.json_array)();
                for i in 0..n {
                    apush(api, a, (api.json_integer)(i as i64));
                }
                a
            };
            let (cj, rj) = pair(c, r, mk);
            for f in [0, json_indent(2), JSON_COMPACT, json_indent(2) | JSON_COMPACT] {
                let out = cmp_dump(c, r, cj, rj, f, &format!("rows68-69 n={n} flags {f:#x}")).unwrap();
                assert_eq!(
                    out.iter().filter(|&&b| b == b',').count(),
                    n - 1,
                    "C: rows 68-69 comma count for n={n}"
                );
                if f == json_indent(2) {
                    // comma comes immediately before the newline of the indent
                    assert!(
                        n == 1 || out.windows(2).any(|w| w == b",\n"),
                        "C: row 69 comma+indent order"
                    );
                }
            }
            decref(c, cj);
            decref(r, rj);
        }

        // Row 70: objects with 1, 2 and many keys.
        for n in [1usize, 2, 3, 16, 17, 40] {
            let mk = |api: &Api| {
                let o = (api.json_object)();
                for i in 0..n {
                    let k = format!("k{i:03}");
                    oset(api, o, k.as_bytes(), (api.json_integer)(i as i64));
                }
                o
            };
            let (cj, rj) = pair(c, r, mk);
            for f in [0, json_indent(2), JSON_COMPACT, JSON_SORT_KEYS, JSON_SORT_KEYS | json_indent(2)] {
                let out = cmp_dump(c, r, cj, rj, f, &format!("row70 n={n} flags {f:#x}")).unwrap();
                assert_eq!(
                    out.iter().filter(|&&b| b == b',').count(),
                    n - 1,
                    "C: row 70 comma count for n={n}"
                );
            }
            decref(c, cj);
            decref(r, rj);
        }

        // Row 71: 5+ levels of mixed nesting with an empty innermost object.
        let (cj, rj) = load2(c, r, "{\"a\":[{\"b\":[{\"c\":[1,[2,{\"d\":{}}]]}]}]}");
        for ind in [0usize, 2, 4, 31] {
            let out = cmp_dump(
                c,
                r,
                cj,
                rj,
                json_indent(ind),
                &format!("row71 INDENT({ind})"),
            )
            .unwrap();
            let s = String::from_utf8(out).unwrap();
            assert!(s.contains("{}"), "C: row 71 innermost {{}} must stay bare: {s}");
        }
        decref(c, cj);
        decref(r, rj);

        // Row 72: all 8 types bare, as the sole array element and as the sole
        // object value.
        let makers: Vec<(&str, fn(&Api) -> *mut json_t)> = vec![
            ("object", |a| unsafe { (a.json_object)() }),
            ("array", |a| unsafe { (a.json_array)() }),
            ("string", |a| unsafe { jstr(a, b"s") }),
            ("integer", |a| unsafe { (a.json_integer)(7) }),
            ("real", |a| unsafe { (a.json_real)(2.5) }),
            ("true", |a| unsafe { (a.json_true)() }),
            ("false", |a| unsafe { (a.json_false)() }),
            ("null", |a| unsafe { (a.json_null)() }),
        ];
        for (name, mk) in &makers {
            // depth 0
            let (cj, rj) = pair(c, r, mk);
            cmp_dump(c, r, cj, rj, JSON_ENCODE_ANY, &format!("row72 bare {name}"));
            decref(c, cj);
            decref(r, rj);
            // depth 1, sole array element
            let (cj, rj) = pair(c, r, |a| {
                let arr = (a.json_array)();
                apush(a, arr, mk(a));
                arr
            });
            for f in [0, json_indent(2), JSON_COMPACT] {
                cmp_dump(c, r, cj, rj, f, &format!("row72 [{name}] flags {f:#x}"));
            }
            decref(c, cj);
            decref(r, rj);
            // depth 1, sole object value
            let (cj, rj) = pair(c, r, |a| {
                let o = (a.json_object)();
                oset(a, o, b"k", mk(a));
                o
            });
            for f in [0, json_indent(2), JSON_COMPACT, JSON_SORT_KEYS] {
                cmp_dump(c, r, cj, rj, f, &format!("row72 {{k:{name}}} flags {f:#x}"));
            }
            decref(c, cj);
            decref(r, rj);
        }

        // Randomised values of ANY type at top level (rows 37-44 + 72): with
        // JSON_ENCODE_ANY everything dumps, without it the six scalar types are
        // rejected and the two container types are not.
        let mut seeds = Rng::new(SEED_R68 ^ 0x77);
        for i in 0..220 {
            let s = seeds.next_u64();
            let (cj, rj) = build_pair(c, r, s, 3);
            for f in [
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | json_indent(2),
                JSON_ENCODE_ANY | JSON_COMPACT | JSON_SORT_KEYS,
                JSON_ENCODE_ANY | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH | json_indent(5),
            ] {
                cmp_dump(c, r, cj, rj, f, &format!("row72 any-type rand #{i} flags {f:#x}"));
            }
            let cb = dumps(c, cj, 0);
            let rb = dumps(r, rj, 0);
            diff_eq!(cb.clone(), rb.clone(), "row37 any-type rand #{i} without ENCODE_ANY");
            let is_container = matches!(typeof_(cj), JSON_ARRAY | JSON_OBJECT);
            assert_eq!(
                cb.is_some(),
                is_container,
                "C: the ENCODE_ANY gate must accept exactly the containers (rand #{i})"
            );
            decref(c, cj);
            decref(r, rj);
        }

        // Randomised deep documents.
        let mut seeds = Rng::new(SEED_R68);
        for i in 0..200 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 5);
            for f in [
                0,
                json_indent(2),
                JSON_COMPACT,
                json_indent(31),
                JSON_SORT_KEYS | json_indent(3),
                JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH | JSON_COMPACT,
            ] {
                cmp_dump(c, r, cj, rj, f, &format!("rows68-72 deep rand #{i} flags {f:#x}"));
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 73-77 — json_dumpb
// ===========================================================================

const SEED_R73: u64 = 0xA06_0073;

/// Dump into a sentinel-filled buffer of `size` bytes; returns (ret, buffer).
unsafe fn dumpb(api: &Api, j: *const json_t, size: usize, flags: size_t) -> (size_t, Vec<u8>) {
    let mut buf = vec![0xAAu8; size + 8];
    let ret = (api.json_dumpb)(j, buf.as_mut_ptr() as *mut c_char, size, flags);
    (ret, buf)
}

#[test]
fn r73_r77_json_dumpb_every_buffer_size() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 73/74: bigger than needed, then exactly the needed size.
        let (cj, rj) = load2(c, r, "[1,2,3]");
        let need = (c.json_dumpb)(cj, std::ptr::null_mut(), 0, 0);
        diff_eq!(
            need,
            (r.json_dumpb)(rj, std::ptr::null_mut(), 0, 0),
            "row76 measuring call"
        );
        assert_eq!(need, 9, "C: row 73 [1, 2, 3] is 9 bytes");

        let (cret, cbuf) = dumpb(c, cj, 64, 0);
        let (rret, rbuf) = dumpb(r, rj, 64, 0);
        diff_eq!(cret, rret, "row73 json_dumpb(size=64) return");
        diff_eq!(Pretty(cbuf.clone()), Pretty(rbuf.clone()), "row73 buffer image");
        assert_eq!(cret, 9, "C: row 73 return");
        assert_eq!(&cbuf[..9], b"[1, 2, 3]", "C: row 73 content");
        assert!(cbuf[9..].iter().all(|&b| b == 0xAA), "C: row 73 tail untouched");

        let (cret, cbuf) = dumpb(c, cj, 9, 0);
        let (rret, rbuf) = dumpb(r, rj, 9, 0);
        diff_eq!(cret, rret, "row74 json_dumpb(size=exact) return");
        diff_eq!(Pretty(cbuf.clone()), Pretty(rbuf.clone()), "row74 buffer image");
        assert_eq!(cret, 9, "C: row 74 return");
        assert_eq!(&cbuf[..9], b"[1, 2, 3]", "C: row 74 content");
        assert_eq!(cbuf[9], 0xAA, "C: row 74 writes no NUL terminator");

        // Row 75/76: every smaller size, including 0, plus the NULL buffer.
        for size in 0..=12usize {
            let (cret, cbuf) = dumpb(c, cj, size, 0);
            let (rret, rbuf) = dumpb(r, rj, size, 0);
            diff_eq!(cret, rret, "row75 json_dumpb(size={size}) return");
            diff_eq!(
                Pretty(cbuf.clone()),
                Pretty(rbuf.clone()),
                "row75 buffer image at size={size}"
            );
            assert_eq!(cret, 9, "C: row 75 always reports the required size");
        }
        diff_eq!(
            (c.json_dumpb)(cj, std::ptr::null_mut(), 0, 0),
            (r.json_dumpb)(rj, std::ptr::null_mut(), 0, 0),
            "row76 NULL buffer, size 0"
        );
        decref(c, cj);
        decref(r, rj);

        // Row 75 non-prefix writes: a chunk that does not fit is skipped but a
        // later, smaller chunk may still be copied. `["aaaaaaaa",1,2]` has a
        // long string chunk followed by short ones.
        let (cj, rj) = load2(c, r, "[\"aaaaaaaa\",1,2]");
        let need = (c.json_dumpb)(cj, std::ptr::null_mut(), 0, 0);
        for size in 0..=(need + 2) {
            let (cret, cbuf) = dumpb(c, cj, size, 0);
            let (rret, rbuf) = dumpb(r, rj, size, 0);
            diff_eq!(cret, rret, "row75 nonprefix size={size} return");
            diff_eq!(
                Pretty(cbuf.clone()),
                Pretty(rbuf.clone()),
                "row75 nonprefix buffer at size={size}"
            );
        }
        // Row 75, the partial-copy semantics spelled out: because `buf->used`
        // keeps growing even for a skipped chunk, once ONE chunk has been
        // skipped `used > size` holds forever and every later chunk is skipped
        // too. So the buffer always ends up holding a plain truncated PREFIX —
        // a "non-prefix write" is in fact unreachable. Verify exactly that, for
        // every size, and also that the cut always falls on a chunk boundary.
        let full = dumps(c, cj, 0).unwrap();
        let mut base = Rec::new();
        record(c, cj, 0, &mut base);
        let mut boundaries = vec![0usize];
        for ch in &base.chunks {
            boundaries.push(boundaries.last().unwrap() + ch.len());
        }
        let mut saw_partial = false;
        for size in 0..=need {
            let (_, buf) = dumpb(c, cj, size, 0);
            let written = buf[..size]
                .iter()
                .zip(full.iter())
                .take_while(|(a, b)| a == b)
                .count();
            // everything after the prefix is still the sentinel
            assert!(
                buf[written..].iter().all(|&b| b == 0xAA),
                "C: row 75 — json_dumpb wrote outside the copied prefix at size={size}"
            );
            assert!(
                boundaries.contains(&written),
                "C: row 75 — the truncation point {written} is not a chunk boundary (size={size})"
            );
            if written > 0 && written < need {
                saw_partial = true;
            }
        }
        assert!(saw_partial, "C: row 75 — expected some size to copy a partial prefix");
        decref(c, cj);
        decref(r, rj);

        // Row 77: ENCODE_ANY scalar, then the gate failure, then EMBED.
        let (cj, rj) = pair(c, r, |a| (a.json_null)());
        let (cret, cbuf) = dumpb(c, cj, 16, JSON_ENCODE_ANY);
        let (rret, rbuf) = dumpb(r, rj, 16, JSON_ENCODE_ANY);
        diff_eq!(cret, rret, "row77 json_dumpb(null, ENCODE_ANY)");
        diff_eq!(Pretty(cbuf.clone()), Pretty(rbuf.clone()), "row77 buffer");
        assert_eq!(cret, 4, "C: row 77 writes 4 bytes");
        assert_eq!(&cbuf[..4], b"null", "C: row 77 content");
        let (cret, _) = dumpb(c, cj, 16, 0);
        let (rret, _) = dumpb(r, rj, 16, 0);
        diff_eq!(cret, rret, "row77 gate failure returns 0");
        assert_eq!(cret, 0, "C: row 77 gate failure is indistinguishable from empty");
        decref(c, cj);
        decref(r, rj);

        // Randomised: every size from 0 to needed+2 on random documents, with
        // the full buffer image compared each time.
        let mut seeds = Rng::new(SEED_R73);
        for i in 0..200 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 2);
            for f in [0, json_indent(2), JSON_COMPACT | JSON_SORT_KEYS, JSON_ENSURE_ASCII] {
                let need = (c.json_dumpb)(cj, std::ptr::null_mut(), 0, f);
                diff_eq!(
                    need,
                    (r.json_dumpb)(rj, std::ptr::null_mut(), 0, f),
                    "rows73-77 rand #{i} measure flags {f:#x}"
                );
                // A handful of interesting sizes rather than all of them, plus
                // the exact boundary.
                let mut sizes = vec![0usize, 1, 2, need.saturating_sub(1), need, need + 1, need + 8];
                if need > 4 {
                    sizes.push(need / 2);
                    sizes.push(need / 3);
                }
                for size in sizes {
                    let (cret, cbuf) = dumpb(c, cj, size, f);
                    let (rret, rbuf) = dumpb(r, rj, size, f);
                    diff_eq!(
                        cret,
                        rret,
                        "rows73-77 rand #{i} flags {f:#x} size {size} return"
                    );
                    diff_eq!(
                        Pretty(cbuf),
                        Pretty(rbuf),
                        "rows73-77 rand #{i} flags {f:#x} size {size} buffer"
                    );
                    assert_eq!(cret, need, "C: json_dumpb must always report the required size");
                }
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 78-79 — json_dumpf
// ===========================================================================

const SEED_R78: u64 = 0xA06_0078;

unsafe fn dumpf_to(api: &Api, j: *const json_t, flags: size_t, path: &PathBuf) -> (c_int, Vec<u8>) {
    let p = cs(path.to_str().unwrap());
    let mode = cs("w");
    let f = fopen(p.as_ptr(), mode.as_ptr());
    assert!(!f.is_null(), "fopen({path:?}, \"w\") failed");
    let rc = (api.json_dumpf)(j, f, flags);
    assert_eq!(fclose(f), 0, "fclose failed");
    (rc, std::fs::read(path).unwrap_or_default())
}

#[test]
fn r78_r79_json_dumpf_success_and_write_failure() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let cp = tmp_path("dumpf.c");
        let rp = tmp_path("dumpf.rust");

        // Row 78: contents must equal json_dumps, and no zero-length chunk is
        // ever emitted (fwrite(_,0,1,_) would be reported as a failure).
        let (cj, rj) = pair(c, r, |a| mixed_doc(a));
        for f in [0, json_indent(2), JSON_COMPACT, JSON_SORT_KEYS | json_indent(4)] {
            let (crc, cbytes_) = dumpf_to(c, cj, f, &cp);
            let (rrc, rbytes_) = dumpf_to(r, rj, f, &rp);
            diff_eq!(crc, rrc, "row78 json_dumpf return flags {f:#x}");
            diff_eq!(
                Pretty(cbytes_.clone()),
                Pretty(rbytes_.clone()),
                "row78 file contents flags {f:#x}"
            );
            assert_eq!(crc, 0, "C: row 78 must succeed");
            assert_eq!(
                cbytes_,
                dumps(c, cj, f).unwrap(),
                "C: row 78 file must equal json_dumps"
            );
            // no zero-length chunk
            let mut rec = Rec::new();
            record(c, cj, f, &mut rec);
            assert!(
                rec.chunks.iter().all(|ch| !ch.is_empty()),
                "C: row 78 — dump.c must never emit a zero-length chunk"
            );
        }
        decref(c, cj);
        decref(r, rj);

        // Row 79a: failure on the very first chunk — a stream opened read-only.
        let (cj, rj) = load2(c, r, "[1,2,3]");
        {
            let p = cs(cp.to_str().unwrap());
            std::fs::write(&cp, b"seed").unwrap();
            std::fs::write(&rp, b"seed").unwrap();
            let mode = cs("r");
            let cf = fopen(p.as_ptr(), mode.as_ptr());
            let p2 = cs(rp.to_str().unwrap());
            let rf = fopen(p2.as_ptr(), mode.as_ptr());
            assert!(!cf.is_null() && !rf.is_null());
            diff_eq!(
                (c.json_dumpf)(cj, cf, 0),
                (r.json_dumpf)(rj, rf, 0),
                "row79 first-chunk failure (read-only stream)"
            );
            assert_eq!((c.json_dumpf)(cj, cf, 0), -1, "C: row 79 must return -1");
            fclose(cf);
            fclose(rf);
        }
        decref(c, cj);
        decref(r, rj);

        // Row 79b: failure on a LATER chunk — /dev/full with the default 4 KiB
        // buffering, so the first chunks are absorbed and a later flush fails.
        let mk_big = |api: &Api| {
            let a = (api.json_array)();
            for i in 0..4000 {
                apush(api, a, (api.json_integer)(i));
            }
            a
        };
        let (cj, rj) = pair(c, r, mk_big);
        {
            let devfull = cs("/dev/full");
            let mode = cs("w");
            let cf = fopen(devfull.as_ptr(), mode.as_ptr());
            let rf = fopen(devfull.as_ptr(), mode.as_ptr());
            if cf.is_null() || rf.is_null() {
                // /dev/full unavailable — the read-only case above still covers
                // the propagation path.
                assert!(cf.is_null() && rf.is_null());
            } else {
                let crc = (c.json_dumpf)(cj, cf, 0);
                let rrc = (r.json_dumpf)(rj, rf, 0);
                diff_eq!(crc, rrc, "row79 later-chunk failure (/dev/full)");
                assert_eq!(crc, -1, "C: row 79 later-chunk failure must return -1");
                fclose(cf);
                fclose(rf);
            }
        }
        decref(c, cj);
        decref(r, rj);

        // Randomised: many documents and flags through a real FILE*.
        let mut seeds = Rng::new(SEED_R78);
        for i in 0..200 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            let f = *[0usize, json_indent(2), JSON_COMPACT, JSON_SORT_KEYS]
                .get(i % 4)
                .unwrap();
            let (crc, cb) = dumpf_to(c, cj, f, &cp);
            let (rrc, rb) = dumpf_to(r, rj, f, &rp);
            diff_eq!(crc, rrc, "row78 rand #{i} return");
            diff_eq!(Pretty(cb.clone()), Pretty(rb), "row78 rand #{i} contents");
            assert_eq!(cb, dumps(c, cj, f).unwrap(), "C: row 78 rand #{i}");
            decref(c, cj);
            decref(r, rj);
        }
        let _ = std::fs::remove_file(&cp);
        let _ = std::fs::remove_file(&rp);
    }
}

// ===========================================================================
// Row 80 — json_dumpfd
// ===========================================================================

const SEED_R80: u64 = 0xA06_0080;

unsafe fn dumpfd_to(api: &Api, j: *const json_t, flags: size_t, path: &PathBuf) -> (c_int, Vec<u8>) {
    let p = cs(path.to_str().unwrap());
    let fd = open(p.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644);
    assert!(fd >= 0, "open({path:?}) failed");
    let rc = (api.json_dumpfd)(j, fd, flags);
    close(fd);
    (rc, std::fs::read(path).unwrap_or_default())
}

#[test]
fn r80_json_dumpfd() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let cp = tmp_path("dumpfd.c");
        let rp = tmp_path("dumpfd.rust");

        let (cj, rj) = pair(c, r, |a| mixed_doc(a));
        for f in [0, json_indent(2)] {
            let (crc, cb) = dumpfd_to(c, cj, f, &cp);
            let (rrc, rb) = dumpfd_to(r, rj, f, &rp);
            diff_eq!(crc, rrc, "row80 json_dumpfd return flags {f:#x}");
            diff_eq!(Pretty(cb.clone()), Pretty(rb), "row80 fd contents flags {f:#x}");
            assert_eq!(crc, 0, "C: row 80 must succeed");
            assert_eq!(cb, dumps(c, cj, f).unwrap(), "C: row 80 == json_dumps");
        }

        // An invalid fd and a closed fd both fail on the first write.
        diff_eq!(
            (c.json_dumpfd)(cj, -1, 0),
            (r.json_dumpfd)(rj, -1, 0),
            "row80 fd = -1"
        );
        assert_eq!((c.json_dumpfd)(cj, -1, 0), -1, "C: row 80 invalid fd");

        let p = cs(cp.to_str().unwrap());
        let fd = open(p.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644);
        assert!(fd >= 0);
        close(fd);
        diff_eq!(
            (c.json_dumpfd)(cj, fd, 0),
            (r.json_dumpfd)(rj, fd, 0),
            "row80 closed fd"
        );
        assert_eq!((c.json_dumpfd)(cj, fd, 0), -1, "C: row 80 closed fd");

        // A read-only fd: write() fails with EBADF.
        let fd = open(p.as_ptr(), O_RDONLY, 0);
        assert!(fd >= 0);
        diff_eq!(
            (c.json_dumpfd)(cj, fd, 0),
            (r.json_dumpfd)(rj, fd, 0),
            "row80 read-only fd"
        );
        assert_eq!((c.json_dumpfd)(cj, fd, 0), -1, "C: row 80 read-only fd");
        close(fd);
        decref(c, cj);
        decref(r, rj);

        // A short/failing write partway through: a non-blocking pipe that fills
        // up. Both libraries emit the same chunks, so both must stop at the
        // same byte count.
        let mk_big = |api: &Api| {
            let a = (api.json_array)();
            for i in 0..60000 {
                apush(api, a, (api.json_integer)(i));
            }
            a
        };
        let (cj, rj) = pair(c, r, mk_big);
        let mut cres = (0, 0usize);
        let mut rres = (0, 0usize);
        for (api, j, slot) in [
            (c, cj as *const json_t, 0usize),
            (r, rj as *const json_t, 1usize),
        ] {
            let mut fds = [0 as c_int; 2];
            assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
            fcntl(fds[0], F_SETFL, O_NONBLOCK);
            fcntl(fds[1], F_SETFL, O_NONBLOCK);
            let rc = (api.json_dumpfd)(j, fds[1], 0);
            close(fds[1]);
            let mut total = 0usize;
            let mut buf = vec![0u8; 65536];
            loop {
                let n = read(fds[0], buf.as_mut_ptr() as *mut c_void, buf.len());
                if n <= 0 {
                    break;
                }
                total += n as usize;
            }
            close(fds[0]);
            if slot == 0 {
                cres = (rc, total);
            } else {
                rres = (rc, total);
            }
        }
        diff_eq!(cres, rres, "row80 non-blocking pipe fills up");
        assert_eq!(cres.0, -1, "C: row 80 a short write must return -1");
        assert!(cres.1 > 0, "C: row 80 some bytes must have made it through");
        decref(c, cj);
        decref(r, rj);

        // Randomised.
        let mut seeds = Rng::new(SEED_R80);
        for i in 0..200 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            let f = if i % 2 == 0 { 0 } else { json_indent(2) };
            let (crc, cb) = dumpfd_to(c, cj, f, &cp);
            let (rrc, rb) = dumpfd_to(r, rj, f, &rp);
            diff_eq!(crc, rrc, "row80 rand #{i} return");
            diff_eq!(Pretty(cb.clone()), Pretty(rb), "row80 rand #{i} contents");
            assert_eq!(cb, dumps(c, cj, f).unwrap(), "C: row 80 rand #{i}");
            decref(c, cj);
            decref(r, rj);
        }
        let _ = std::fs::remove_file(&cp);
        let _ = std::fs::remove_file(&rp);
    }
}

// ===========================================================================
// Rows 81-82 — json_dump_file
// ===========================================================================

const SEED_R81: u64 = 0xA06_0081;

#[test]
fn r81_r82_json_dump_file() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let cp = tmp_path("dumpfile.c");
        let rp = tmp_path("dumpfile.rust");
        let cps = cs(cp.to_str().unwrap());
        let rps = cs(rp.to_str().unwrap());

        // Row 81: a valid writable path.
        let (cj, rj) = pair(c, r, |a| mixed_doc(a));
        for f in [0, json_indent(2), JSON_SORT_KEYS] {
            let crc = (c.json_dump_file)(cj, cps.as_ptr(), f);
            let rrc = (r.json_dump_file)(rj, rps.as_ptr(), f);
            diff_eq!(crc, rrc, "row81 json_dump_file return flags {f:#x}");
            let cb = std::fs::read(&cp).unwrap();
            let rb = std::fs::read(&rp).unwrap();
            diff_eq!(Pretty(cb.clone()), Pretty(rb), "row81 file contents flags {f:#x}");
            assert_eq!(crc, 0, "C: row 81 must succeed");
            assert_eq!(cb, dumps(c, cj, f).unwrap(), "C: row 81 == json_dumps");
        }

        // Row 81: an unopenable path.
        for bad in ["/nonexistent-dir-a06/x.json", "/proc/self/cmdline/x"] {
            let b = cs(bad);
            diff_eq!(
                (c.json_dump_file)(cj, b.as_ptr(), 0),
                (r.json_dump_file)(rj, b.as_ptr(), 0),
                "row81 unopenable {bad}"
            );
            assert_eq!(
                (c.json_dump_file)(cj, b.as_ptr(), 0),
                -1,
                "C: row 81 fopen failure must return -1"
            );
        }

        // Row 82: "w" truncates an existing longer file.
        std::fs::write(&cp, vec![b'X'; 10_000]).unwrap();
        std::fs::write(&rp, vec![b'X'; 10_000]).unwrap();
        let (scj, srj) = load2(c, r, "[1]");
        diff_eq!(
            (c.json_dump_file)(scj, cps.as_ptr(), 0),
            (r.json_dump_file)(srj, rps.as_ptr(), 0),
            "row82 truncating overwrite"
        );
        let cb = std::fs::read(&cp).unwrap();
        let rb = std::fs::read(&rp).unwrap();
        diff_eq!(Pretty(cb.clone()), Pretty(rb), "row82 truncated contents");
        assert_eq!(cb, b"[1]".to_vec(), "C: row 82 the file must be truncated");
        decref(c, scj);
        decref(r, srj);

        // Row 82: the write succeeds but fclose fails (/dev/full).
        let full = cs("/dev/full");
        let crc = (c.json_dump_file)(cj, full.as_ptr(), 0);
        let rrc = (r.json_dump_file)(rj, full.as_ptr(), 0);
        diff_eq!(crc, rrc, "row82 fclose failure on /dev/full");
        assert_eq!(crc, -1, "C: row 82 fclose failure must return -1");
        decref(c, cj);
        decref(r, rj);

        // Randomised.
        let mut seeds = Rng::new(SEED_R81);
        for i in 0..200 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            let f = *[0usize, json_indent(2), JSON_SORT_KEYS, JSON_COMPACT]
                .get(i % 4)
                .unwrap();
            let crc = (c.json_dump_file)(cj, cps.as_ptr(), f);
            let rrc = (r.json_dump_file)(rj, rps.as_ptr(), f);
            diff_eq!(crc, rrc, "row81 rand #{i} return");
            let cb = std::fs::read(&cp).unwrap();
            let rb = std::fs::read(&rp).unwrap();
            diff_eq!(Pretty(cb.clone()), Pretty(rb), "row81 rand #{i} contents");
            assert_eq!(cb, dumps(c, cj, f).unwrap(), "C: row 81 rand #{i}");
            decref(c, cj);
            decref(r, rj);
        }
        let _ = std::fs::remove_file(&cp);
        let _ = std::fs::remove_file(&rp);
    }
}

// ===========================================================================
// Rows 83-87 — json_dump_callback: the exact chunk sequence and every
// error-propagation site
// ===========================================================================

const SEED_R83: u64 = 0xA06_0083;

#[test]
fn r83_json_dump_callback_chunk_sequence() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 83: establish the exact chunk sequence for a nested document.
        let (cj, rj) = pair(c, r, |a| mixed_doc(a));
        for f in [
            0,
            json_indent(2),
            json_indent(31),
            JSON_COMPACT,
            JSON_SORT_KEYS,
            JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
            JSON_SORT_KEYS | json_indent(4) | JSON_ENSURE_ASCII,
        ] {
            let mut crec = Rec::new();
            let mut rrec = Rec::new();
            let crc = record(c, cj, f, &mut crec);
            let rrc = record(r, rj, f, &mut rrec);
            diff_eq!(crc, rrc, "row83 return flags {f:#x}");
            diff_eq!(crec.pretty(), rrec.pretty(), "row83 chunk sequence flags {f:#x}");
            assert_eq!(crc, 0, "C: row 83 must succeed");
            assert_eq!(
                crec.joined(),
                dumps(c, cj, f).unwrap(),
                "C: row 83 chunks must concatenate to json_dumps"
            );
            assert!(
                crec.chunks.iter().all(|ch| !ch.is_empty()),
                "C: row 83 no zero-length chunk"
            );
        }
        decref(c, cj);
        decref(r, rj);

        // Randomised chunk sequences — the strongest available comparison,
        // since it pins the exact call pattern and not just the bytes.
        let mut seeds = Rng::new(SEED_R83);
        for i in 0..250 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            for f in [
                0,
                json_indent(2),
                JSON_COMPACT | JSON_ENSURE_ASCII,
                JSON_SORT_KEYS | json_indent(3) | JSON_ESCAPE_SLASH,
            ] {
                let mut crec = Rec::new();
                let mut rrec = Rec::new();
                let crc = record(c, cj, f, &mut crec);
                let rrc = record(r, rj, f, &mut rrec);
                diff_eq!(crc, rrc, "row83 rand #{i} return flags {f:#x}");
                diff_eq!(
                    crec.pretty(),
                    rrec.pretty(),
                    "row83 rand #{i} chunks flags {f:#x}"
                );
                assert!(
                    crec.chunks.iter().all(|ch| !ch.is_empty()),
                    "C: row 83 rand #{i} zero-length chunk"
                );
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

const SEED_R84: u64 = 0xA06_0084;

#[test]
fn r84_r87_callback_failure_at_every_chunk_index() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Rows 84/85/86: fail at chunk 0, at every middle index, and at the
        // last index — for several flag sets so every `return -1` site in
        // dump.c (dump_indent, dump_string, comma, separator, delimiters) and
        // both object branches are reached.
        let docs: Vec<(&str, fn(&Api) -> *mut json_t)> = vec![
            ("mixed", |a| unsafe { mixed_doc(a) }),
            ("array", |a| unsafe {
                let arr = (a.json_array)();
                apush(a, arr, (a.json_integer)(1));
                apush(a, arr, jstr(a, b"a\"b/c\n\xc3\xa9"));
                apush(a, arr, (a.json_real)(1.5));
                let inner = (a.json_object)();
                oset(a, inner, b"k", (a.json_array)());
                apush(a, arr, inner);
                arr
            }),
            ("scalar", |a| unsafe { jstr(a, b"only") }),
        ];
        for (name, mk) in &docs {
            let (cj, rj) = pair(c, r, mk);
            for f in [
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | json_indent(2),
                JSON_ENCODE_ANY | JSON_COMPACT | JSON_ENSURE_ASCII,
                JSON_ENCODE_ANY | JSON_SORT_KEYS | json_indent(31),
                JSON_ENCODE_ANY | JSON_SORT_KEYS | JSON_ESCAPE_SLASH,
                JSON_ENCODE_ANY | JSON_EMBED,
            ] {
                // ground truth chunk count
                let mut base = Rec::new();
                assert_eq!(record(c, cj, f, &mut base), 0, "C: baseline must succeed");
                let n = base.chunks.len();
                let embed = (f & JSON_EMBED) != 0;
                let mut tolerated = Vec::new();
                for k in 0..n {
                    for ret in [-1 as c_int, 1, 7] {
                        let mut crec = Rec::failing(k, ret);
                        let mut rrec = Rec::failing(k, ret);
                        let crc = record(c, cj, f, &mut crec);
                        let rrc = record(r, rj, f, &mut rrec);
                        diff_eq!(
                            crc,
                            rrc,
                            "rows84-86 {name} flags {f:#x} fail at chunk {k}/{n} ret {ret}"
                        );
                        diff_eq!(
                            crec.pretty(),
                            rrec.pretty(),
                            "rows84-86 {name} flags {f:#x} chunks up to failure {k}"
                        );
                        if crc == 0 {
                            // Rows 85/87: the ONLY tolerated failures are the
                            // ones inside a key's `dump_string`, whose return
                            // value dump.c deliberately ignores. `dump_string`
                            // itself still aborts (so the rest of that key is
                            // never emitted), but the dump as a whole carries on
                            // and reports success.
                            tolerated.push(k);
                            assert!(
                                crec.chunks.len() > k + 1,
                                "C: a tolerated failure at {k} must not stop the dump \
                                 (got {} of {n} chunks)",
                                crec.chunks.len()
                            );
                        } else {
                            // Row 84: nothing is emitted after the failure.
                            assert_eq!(
                                crec.chunks.len(),
                                k + 1,
                                "C: flags {f:#x}: dump did not stop at the failing chunk {k}"
                            );
                        }
                        // Row 86: the value returned by the LAST chunk is
                        // handed back verbatim, not normalised to -1, because
                        // do_dump ends in `return embed ? 0 : dump("]", 1, data)`.
                        if k == n - 1 && !embed && crc != 0 {
                            assert_eq!(
                                crc, ret,
                                "C: row 86 — the last chunk's return must not be normalised"
                            );
                        }
                    }
                }
                // Row 84: failing on chunk 0 aborts immediately whenever chunk 0
                // is a checked dump (i.e. not a key string in an embedded object).
                if !embed {
                    let mut rec0 = Rec::failing(0, -1);
                    assert_ne!(
                        record(c, cj, f, &mut rec0),
                        0,
                        "C: row 84 — failure on the first chunk must abort ({name}, {f:#x})"
                    );
                }
                // Row 87's note: with SORT_KEYS a failure that lands on a key
                // string really is silently tolerated.
                if (f & JSON_SORT_KEYS) != 0 && *name == "mixed" {
                    assert!(
                        !tolerated.is_empty(),
                        "C: row 87 — expected dump_string's ignored return to tolerate \
                         some chunk failure ({f:#x})"
                    );
                }
            }
            decref(c, cj);
            decref(r, rj);
        }

        // Row 87: the SORT_KEYS branch's three failure sites, on a many-key
        // object, swept over every chunk index.
        let mk = |api: &Api| {
            let o = (api.json_object)();
            for i in 0..10 {
                let k = format!("k{i:02}");
                let v = if i % 3 == 0 {
                    jstr(api, b"v/\"\n")
                } else if i % 3 == 1 {
                    (api.json_integer)(i)
                } else {
                    let a = (api.json_array)();
                    apush(api, a, (api.json_integer)(i));
                    a
                };
                oset(api, o, k.as_bytes(), v);
            }
            o
        };
        let (cj, rj) = pair(c, r, mk);
        for f in [
            JSON_SORT_KEYS,
            JSON_SORT_KEYS | json_indent(2),
            JSON_SORT_KEYS | JSON_COMPACT | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
        ] {
            let mut base = Rec::new();
            assert_eq!(record(c, cj, f, &mut base), 0);
            let n = base.chunks.len();
            for k in 0..n {
                let mut crec = Rec::failing(k, -1);
                let mut rrec = Rec::failing(k, -1);
                diff_eq!(
                    record(c, cj, f, &mut crec),
                    record(r, rj, f, &mut rrec),
                    "row87 SORT_KEYS flags {f:#x} fail at chunk {k}/{n}"
                );
                diff_eq!(
                    crec.pretty(),
                    rrec.pretty(),
                    "row87 SORT_KEYS flags {f:#x} chunks up to {k}"
                );
            }
        }
        decref(c, cj);
        decref(r, rj);

        // Randomised: fail at a random chunk index in a random document.
        let mut seeds = Rng::new(SEED_R84);
        for i in 0..250 {
            let s = seeds.next_u64();
            let (cj, rj) = build_container_pair(c, r, s, 3);
            let f = *[
                0usize,
                json_indent(2),
                JSON_COMPACT,
                JSON_SORT_KEYS,
                JSON_SORT_KEYS | json_indent(4) | JSON_ENSURE_ASCII,
                JSON_EMBED,
            ]
            .get(i % 6)
            .unwrap();
            let mut base = Rec::new();
            assert_eq!(record(c, cj, f, &mut base), 0, "C: baseline rand #{i}");
            let n = base.chunks.len();
            if n == 0 {
                decref(c, cj);
                decref(r, rj);
                continue;
            }
            for _ in 0..6 {
                let k = seeds.below(n);
                let ret = *seeds.choice(&[-1 as c_int, 1, 42]);
                let mut crec = Rec::failing(k, ret);
                let mut rrec = Rec::failing(k, ret);
                diff_eq!(
                    record(c, cj, f, &mut crec),
                    record(r, rj, f, &mut rrec),
                    "rows84-87 rand #{i} flags {f:#x} fail at {k}/{n} ret {ret}"
                );
                diff_eq!(
                    crec.pretty(),
                    rrec.pretty(),
                    "rows84-87 rand #{i} flags {f:#x} chunks up to {k}"
                );
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Row 88 — circular-reference detection and shared (DAG) children
// ===========================================================================

const SEED_R88: u64 = 0xA06_0088;

#[test]
fn r88_circular_references_and_shared_children() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let flagsets = [0usize, json_indent(2), JSON_EMBED, JSON_SORT_KEYS, JSON_COMPACT];

        // An array that (indirectly) contains itself. A *direct* self-reference
        // is impossible to build: `json_array_append_new` refuses `json ==
        // value`, so the shortest cycle a caller can create is length 2.
        {
            let (cj, rj) = pair(c, r, |api| {
                let a = (api.json_array)();
                assert_eq!(
                    (api.json_array_append_new)(a, incref(a)),
                    -1,
                    "{}: a direct self-append must be refused",
                    api.which
                );
                // the refused call already released the reference we handed it
                a
            });
            decref(c, cj);
            decref(r, rj);
        }
        for &f in &flagsets {
            let mk = |api: &Api| {
                let outer = (api.json_array)();
                let inner = (api.json_array)();
                apush(api, inner, incref(outer));
                apush(api, outer, inner);
                outer
            };
            let (cj, rj) = pair(c, r, mk);
            let cb = dumps(c, cj, f);
            let rb = dumps(r, rj, f);
            diff_eq!(cb.clone(), rb.clone(), "row88 self-array flags {f:#x}");
            assert!(cb.is_none(), "C: row 88 self-array must fail at flags {f:#x}");
            // break the cycle before releasing
            (c.json_array_clear)(cj);
            (r.json_array_clear)(rj);
            decref(c, cj);
            decref(r, rj);
        }

        // An object that (indirectly) contains itself.
        for &f in &flagsets {
            let mk = |api: &Api| {
                let o = (api.json_object)();
                assert_eq!(
                    (api.json_object_setn_new_nocheck)(o, b"x\0".as_ptr() as *const c_char, 1, incref(o)),
                    -1,
                    "{}: a direct self-set must be refused",
                    api.which
                );
                let inner = (api.json_object)();
                oset(api, inner, b"back", incref(o));
                oset(api, o, b"self", inner);
                o
            };
            let (cj, rj) = pair(c, r, mk);
            let cb = dumps(c, cj, f);
            let rb = dumps(r, rj, f);
            diff_eq!(cb.clone(), rb.clone(), "row88 self-object flags {f:#x}");
            assert!(cb.is_none(), "C: row 88 self-object must fail at flags {f:#x}");
            (c.json_object_clear)(cj);
            (r.json_object_clear)(rj);
            decref(c, cj);
            decref(r, rj);
        }

        // A 3-cycle a -> b -> c -> a.
        for &f in &flagsets {
            let mk = |api: &Api| {
                let a = (api.json_array)();
                let b = (api.json_array)();
                let cc = (api.json_array)();
                apush(api, cc, incref(a));
                apush(api, b, cc);
                apush(api, a, b);
                a
            };
            let (cj, rj) = pair(c, r, mk);
            let cb = dumps(c, cj, f);
            let rb = dumps(r, rj, f);
            diff_eq!(cb.clone(), rb.clone(), "row88 3-cycle flags {f:#x}");
            assert!(cb.is_none(), "C: row 88 3-cycle must fail at flags {f:#x}");
            (c.json_array_clear)(cj);
            (r.json_array_clear)(rj);
            decref(c, cj);
            decref(r, rj);
        }

        // A DAG: the SAME child appears twice. hashtable_del must have removed
        // it from the parents set after the first visit, so this succeeds.
        for &f in &flagsets {
            let mk = |api: &Api| {
                let shared = (api.json_array)();
                apush(api, shared, (api.json_integer)(1));
                let inner = (api.json_object)();
                oset(api, inner, b"s", incref(shared));
                let root = (api.json_array)();
                apush(api, root, incref(shared));
                apush(api, root, inner);
                apush(api, root, incref(shared));
                decref(api, shared);
                root
            };
            let (cj, rj) = pair(c, r, mk);
            let out = cmp_dump(c, r, cj, rj, f, &format!("row88 shared DAG flags {f:#x}"));
            assert!(out.is_some(), "C: row 88 a DAG is not a cycle and must succeed");
            decref(c, cj);
            decref(r, rj);
        }

        // A cycle buried deep inside an otherwise valid document, at random
        // positions, so the failure unwinds through many nesting levels.
        let mut seeds = Rng::new(SEED_R88);
        for i in 0..200 {
            let s = seeds.next_u64();
            let mk = |api: &Api| {
                let mut g = Rng::new(s);
                let root = build_container(api, &mut g, 3);
                // graft a 2-cycle at the end of the root
                let cyc = (api.json_array)();
                let back = (api.json_array)();
                apush(api, back, incref(cyc));
                apush(api, cyc, back);
                if (*root).type_ == JSON_ARRAY {
                    apush(api, root, cyc);
                } else {
                    oset(api, root, b"__cycle__", cyc);
                }
                root
            };
            let (cj, rj) = pair(c, r, mk);
            for &f in &flagsets {
                let cb = dumps(c, cj, f);
                let rb = dumps(r, rj, f);
                diff_eq!(cb.clone(), rb.clone(), "row88 rand #{i} flags {f:#x}");
                assert!(cb.is_none(), "C: row 88 rand #{i} must fail");
            }
            // The cycle keeps the graft alive; that is acceptable in a test.
            decref(c, cj);
            decref(r, rj);
        }
    }
}
