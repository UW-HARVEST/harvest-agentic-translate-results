//! Phase C — error-path differential tests for `src/dump.c`.
//!
//! Covers ERRORS.md rows **198-220, 350, 351, 352, 353, 354 and 356** — the
//! failure surface of `json_dumps` / `json_dumpb` / `json_dumpf` /
//! `json_dumpfd` / `json_dump_file` / `json_dump_callback` — plus the generic
//! FFI boundary conditions that have no ERRORS.md row of their own: a NULL
//! `json`, a NULL callback pointer, a NULL output buffer, a zero and an
//! oversized `size`, invalid/closed/read-only file descriptors, unopenable
//! paths, and flag words with undefined bits set (`size_t flags` accepts any
//! 64-bit value, so both libraries must ignore the unknown bits identically).
//!
//! a06_dump.rs is the happy-path/observable-output complement; this file only
//! ever asks "do both libraries FAIL the same way, and leave the same bytes
//! behind when they do".
//!
//! Note the return-value zoo, because each entry point reports failure
//! differently:
//!   * `json_dumps`        -> `NULL`
//!   * `json_dumpb`        -> `0`  (**indistinguishable from an empty dump**)
//!   * `json_dumpf` / `json_dumpfd` / `json_dump_file` / `json_dump_callback`
//!                         -> `-1`

#![allow(unused_unsafe)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};

// ---------------------------------------------------------------------------
// libc bits (the test process shares its libc with both shared objects, so a
// FILE*/fd/heap pointer made here is valid in both)
// ---------------------------------------------------------------------------

extern "C" {
    fn malloc(n: size_t) -> *mut c_void;
    fn realloc(p: *mut c_void, n: size_t) -> *mut c_void;
    fn free(p: *mut c_void);
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fclose(f: *mut FILE) -> c_int;
    fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

const O_RDONLY: c_int = 0;

/// Every flag bit `dump.c` actually looks at:
/// `0x1F` indent | `0x20` compact | `0x40` ensure_ascii | `0x80` sort_keys |
/// `0x100` preserve_order | `0x200` encode_any | `0x400` escape_slash |
/// `0xF800` real precision | `0x10000` embed.
const KNOWN_FLAG_BITS: size_t = 0x1_FFFF;

// ---------------------------------------------------------------------------
// Small shared helpers
// ---------------------------------------------------------------------------

fn tmp_dir() -> PathBuf {
    PathBuf::from(std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string()))
}

fn tmp_path(name: &str) -> PathBuf {
    tmp_dir().join(format!("a12_errors_dump.{name}"))
}

/// A `Vec<u8>` that prints readably in divergence messages.
#[derive(PartialEq, Eq, Clone)]
struct Pretty(Vec<u8>);

impl std::fmt::Debug for Pretty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} ({} bytes)", String::from_utf8_lossy(&self.0), self.0.len())
    }
}

/// `json_dumps` + read the bytes + free with the matching allocator.
/// `None` means the library returned NULL (an error), which stays distinct from
/// `Some(empty)`.
unsafe fn dumps(api: &Api, j: *const json_t, flags: size_t) -> Option<Pretty> {
    let p = (api.json_dumps)(j, flags);
    let b = cbytes(p).map(Pretty);
    jfree(api, p as *mut c_void);
    b
}

/// `json_dumpb` into a 64-byte poisoned buffer: returns the count AND the full
/// byte image of the buffer, so "nothing was written" is observable.
unsafe fn dumpb64(api: &Api, j: *const json_t, size: usize, flags: size_t) -> (size_t, [u8; 64]) {
    assert!(size <= 64 || size == usize::MAX, "dumpb64 needs size <= 64");
    let mut buf = [0xAAu8; 64];
    let n = (api.json_dumpb)(j, buf.as_mut_ptr() as *mut c_char, size, flags);
    (n, buf)
}

unsafe fn pair<F>(c: &Api, r: &Api, f: F) -> (*mut json_t, *mut json_t)
where
    F: Fn(&Api) -> *mut json_t,
{
    (f(c), f(r))
}

unsafe fn oset(api: &Api, obj: *mut json_t, key: &[u8], v: *mut json_t) -> c_int {
    (api.json_object_setn_new_nocheck)(obj, key.as_ptr() as *const c_char, key.len(), v)
}

unsafe fn apush(api: &Api, arr: *mut json_t, v: *mut json_t) -> c_int {
    (api.json_array_append_new)(arr, v)
}

/// `json_array_append(array, value)` — a `static JSON_INLINE` in jansson.h, so
/// it is not exported: `append_new(array, incref(value))`. Needed to build the
/// INDIRECT cycles that `json_array_append_new` itself refuses.
unsafe fn apush_ref(api: &Api, arr: *mut json_t, v: *mut json_t) -> c_int {
    (api.json_array_append_new)(arr, incref(v))
}

unsafe fn oset_ref(api: &Api, obj: *mut json_t, key: &[u8], v: *mut json_t) -> c_int {
    (api.json_object_setn_new_nocheck)(obj, key.as_ptr() as *const c_char, key.len(), incref(v))
}

/// A document with one of everything, so that every flag bit (indent, compact,
/// ensure_ascii, sort_keys, escape_slash, real precision) is live.
unsafe fn mixed(api: &Api) -> *mut json_t {
    let root = (api.json_object)();
    oset(api, root, b"zz", (api.json_integer)(-7));
    oset(api, root, b"a", (api.json_real)(0.30000000000000004));
    oset(api, root, b"m", (api.json_string)(cs("a/b\u{00e9}\u{10348}\t").as_ptr()));
    let arr = (api.json_array)();
    apush(api, arr, (api.json_integer)(1));
    apush(api, arr, (api.json_null)());
    apush(api, arr, (api.json_true)());
    let inner = (api.json_object)();
    oset(api, inner, b"k", (api.json_real)(0.1));
    apush(api, arr, inner);
    oset(api, root, b"arr", arr);
    root
}

// ---------------------------------------------------------------------------
// A recording dump callback that can be made to fail on a chunk selected BY
// CONTENT (not by index), which is what makes the key/value asymmetry of rows
// 352/353 expressible.
// ---------------------------------------------------------------------------

struct Sink {
    chunks: Vec<Vec<u8>>,
    target: Option<Vec<u8>>,
    occurrence: usize,
    hits: usize,
    fail_ret: c_int,
}

impl Sink {
    fn quiet() -> Sink {
        Sink { chunks: Vec::new(), target: None, occurrence: 0, hits: 0, fail_ret: -1 }
    }
    fn failing_on(target: &[u8], occurrence: usize) -> Sink {
        Sink {
            chunks: Vec::new(),
            target: Some(target.to_vec()),
            occurrence,
            hits: 0,
            fail_ret: -1,
        }
    }
    fn joined(&self) -> Pretty {
        Pretty(self.chunks.iter().flatten().copied().collect())
    }
    fn pretty(&self) -> Vec<Pretty> {
        self.chunks.iter().cloned().map(Pretty).collect()
    }
}

unsafe extern "C" fn sink_cb(buf: *const c_char, size: size_t, data: *mut c_void) -> c_int {
    let s = &mut *(data as *mut Sink);
    let bytes = if size == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(buf as *const u8, size).to_vec()
    };
    let mut ret = 0;
    if let Some(t) = &s.target {
        if &bytes == t {
            if s.hits == s.occurrence {
                ret = s.fail_ret;
            }
            s.hits += 1;
        }
    }
    s.chunks.push(bytes);
    ret
}

/// (return value, chunk list, joined bytes)
unsafe fn run_cb(
    api: &Api,
    j: *const json_t,
    flags: size_t,
    mut sink: Sink,
) -> (c_int, Vec<Pretty>, Pretty) {
    let rc = (api.json_dump_callback)(
        j,
        Some(sink_cb),
        &mut sink as *mut Sink as *mut c_void,
        flags,
    );
    (rc, sink.pretty(), sink.joined())
}

// ---------------------------------------------------------------------------
// Instrumented allocator: a per-call malloc budget (to reach the OOM rows) and
// an all-or-nothing realloc switch (to reach row 354). Installed on BOTH
// libraries with the same budget, and always restored before the test returns.
// ---------------------------------------------------------------------------

/// `-1` = unlimited. Otherwise the number of `malloc` calls that succeed before
/// every later one returns NULL.
static MALLOC_BUDGET: AtomicI64 = AtomicI64::new(-1);
static MALLOC_CALLS: AtomicI64 = AtomicI64::new(0);
static REALLOC_CALLS: AtomicI64 = AtomicI64::new(0);
/// `1` = every `realloc` fails.
static REALLOC_FAILS: AtomicI64 = AtomicI64::new(0);

unsafe extern "C" fn hook_malloc(n: size_t) -> *mut c_void {
    let idx = MALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
    let budget = MALLOC_BUDGET.load(Ordering::SeqCst);
    if budget >= 0 && idx >= budget {
        return std::ptr::null_mut();
    }
    malloc(n)
}

unsafe extern "C" fn hook_realloc(p: *mut c_void, n: size_t) -> *mut c_void {
    REALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
    if REALLOC_FAILS.load(Ordering::SeqCst) == 1 {
        return std::ptr::null_mut();
    }
    realloc(p, n)
}

unsafe extern "C" fn hook_free(p: *mut c_void) {
    free(p)
}

/// Saved originals, so the rest of the suite is unaffected.
struct Allocators {
    c: (json_malloc_t, json_realloc_t, json_free_t),
    r: (json_malloc_t, json_realloc_t, json_free_t),
}

unsafe fn install_hooks(c: &Api, r: &Api) -> Allocators {
    let mut cm: json_malloc_t = None;
    let mut crl: json_realloc_t = None;
    let mut cf: json_free_t = None;
    let mut rm: json_malloc_t = None;
    let mut rrl: json_realloc_t = None;
    let mut rf: json_free_t = None;
    (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
    (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);
    MALLOC_BUDGET.store(-1, Ordering::SeqCst);
    REALLOC_FAILS.store(0, Ordering::SeqCst);
    (c.json_set_alloc_funcs2)(Some(hook_malloc), Some(hook_realloc), Some(hook_free));
    (r.json_set_alloc_funcs2)(Some(hook_malloc), Some(hook_realloc), Some(hook_free));
    Allocators { c: (cm, crl, cf), r: (rm, rrl, rf) }
}

unsafe fn restore_hooks(c: &Api, r: &Api, saved: &Allocators) {
    MALLOC_BUDGET.store(-1, Ordering::SeqCst);
    REALLOC_FAILS.store(0, Ordering::SeqCst);
    (c.json_set_alloc_funcs2)(saved.c.0, saved.c.1, saved.c.2);
    (r.json_set_alloc_funcs2)(saved.r.0, saved.r.1, saved.r.2);
    // Sanity: allocation works again, so later tests are unaffected.
    let o = (c.json_object)();
    assert!(!o.is_null(), "C allocator was not restored");
    decref(c, o);
    let o = (r.json_object)();
    assert!(!o.is_null(), "Rust allocator was not restored");
    decref(r, o);
}

/// Run `json_dumps` with a malloc budget, returning (bytes, mallocs consumed).
unsafe fn dumps_budgeted(
    api: &Api,
    j: *const json_t,
    flags: size_t,
    budget: i64,
) -> (Option<Pretty>, i64) {
    MALLOC_CALLS.store(0, Ordering::SeqCst);
    MALLOC_BUDGET.store(budget, Ordering::SeqCst);
    let p = (api.json_dumps)(j, flags);
    let used = MALLOC_CALLS.load(Ordering::SeqCst);
    MALLOC_BUDGET.store(-1, Ordering::SeqCst);
    let b = cbytes(p).map(Pretty);
    jfree(api, p as *mut c_void);
    (b, used)
}

// ===========================================================================
// Rows 200, 201, 203 — the JSON_ENCODE_ANY gate, and `!json` inside do_dump
// ===========================================================================

fn scalar_makers() -> Vec<(&'static str, fn(&Api) -> *mut json_t)> {
    vec![
        ("integer", |a: &Api| unsafe { (a.json_integer)(42) }),
        ("real", |a: &Api| unsafe { (a.json_real)(0.5) }),
        ("string", |a: &Api| unsafe { (a.json_string)(cs("x").as_ptr()) }),
        ("true", |a: &Api| unsafe { (a.json_true)() }),
        ("false", |a: &Api| unsafe { (a.json_false)() }),
        ("null", |a: &Api| unsafe { (a.json_null)() }),
    ]
}

#[test]
fn rows_200_201_203_encode_any_gate_and_null_json() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 200: `json == NULL` without JSON_ENCODE_ANY is rejected by the
        // gate in json_dump_callback, BEFORE hashtable_init and before the
        // callback is ever invoked.
        // Row 203: with JSON_ENCODE_ANY it gets as far as do_dump's `!json`.
        // Row 201: a scalar without JSON_ENCODE_ANY.
        let flag_sets: &[size_t] = &[
            0,
            JSON_COMPACT,
            json_indent(4),
            JSON_SORT_KEYS,
            JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
            JSON_EMBED,
            0x2_0000,     // an undefined bit, on its own
            KNOWN_FLAG_BITS & !JSON_ENCODE_ANY,
        ];
        for &f in flag_sets {
            for &encode_any in &[0, JSON_ENCODE_ANY] {
                let flags = f | encode_any;
                let ctx = format!("flags={flags:#x}");

                // --- NULL json through every entry point
                diff_eq!(dumps(c, std::ptr::null(), flags), dumps(r, std::ptr::null(), flags),
                    "json_dumps(NULL) [{ctx}]");
                assert!(
                    dumps(c, std::ptr::null(), flags).is_none(),
                    "C: json_dumps(NULL) must be NULL (rows 200/203) [{ctx}]"
                );

                let (cn, cbuf) = dumpb64(c, std::ptr::null(), 64, flags);
                let (rn, rbuf) = dumpb64(r, std::ptr::null(), 64, flags);
                diff_eq!((cn, cbuf), (rn, rbuf), "json_dumpb(NULL) [{ctx}]");
                assert_eq!(cn, 0, "C: json_dumpb(NULL) must be 0 [{ctx}]");
                assert_eq!(cbuf, [0xAAu8; 64], "C: json_dumpb(NULL) must not write [{ctx}]");

                diff_eq!(
                    (c.json_dumpfd)(std::ptr::null(), -1, flags),
                    (r.json_dumpfd)(std::ptr::null(), -1, flags),
                    "json_dumpfd(NULL) [{ctx}]"
                );

                // The callback must not be invoked at all on either path.
                let (crc, cch, _) = run_cb(c, std::ptr::null(), flags, Sink::quiet());
                let (rrc, rch, _) = run_cb(r, std::ptr::null(), flags, Sink::quiet());
                diff_eq!(crc, rrc, "json_dump_callback(NULL) return [{ctx}]");
                diff_eq!(cch.clone(), rch, "json_dump_callback(NULL) chunks [{ctx}]");
                assert_eq!(crc, -1, "C: json_dump_callback(NULL) must be -1 [{ctx}]");
                assert!(cch.is_empty(), "C: no chunk may be emitted for NULL json [{ctx}]");

                // --- scalars: rejected without JSON_ENCODE_ANY (row 201),
                //     accepted with it.
                for (name, mk) in scalar_makers() {
                    let (cj, rj) = pair(c, r, mk);
                    let cd = dumps(c, cj, flags);
                    let rd = dumps(r, rj, flags);
                    diff_eq!(cd.clone(), rd, "json_dumps({name}) [{ctx}]");
                    if encode_any == 0 {
                        assert!(
                            cd.is_none(),
                            "C row 201: a bare {name} must be rejected [{ctx}]"
                        );
                    } else if !(name == "real" && (flags >> 11) & 0x1F >= 22) {
                        // (a real with JSON_REAL_PRECISION >= 22 fails for the
                        // unrelated reason of rows 208/209)
                        assert!(
                            cd.is_some(),
                            "C: JSON_ENCODE_ANY must let a bare {name} through [{ctx}]"
                        );
                    }

                    let (crc, cch, _) = run_cb(c, cj, flags, Sink::quiet());
                    let (rrc, rch, _) = run_cb(r, rj, flags, Sink::quiet());
                    diff_eq!(crc, rrc, "json_dump_callback({name}) [{ctx}]");
                    diff_eq!(cch.clone(), rch, "json_dump_callback({name}) chunks [{ctx}]");
                    if encode_any == 0 {
                        assert!(cch.is_empty(), "C: gate rejects before any chunk [{ctx}]");
                    }

                    let (cn, cbuf) = dumpb64(c, cj, 64, flags);
                    let (rn, rbuf) = dumpb64(r, rj, 64, flags);
                    diff_eq!((cn, cbuf), (rn, rbuf), "json_dumpb({name}) [{ctx}]");
                    if encode_any == 0 {
                        assert_eq!(cn, 0, "C: row 201 => json_dumpb returns 0 [{ctx}]");
                        assert_eq!(cbuf, [0xAAu8; 64], "C: nothing written [{ctx}]");
                    }
                    decref(c, cj);
                    decref(r, rj);
                }
            }
        }

        // --- json_dumpf / json_dump_file with a NULL json: the file is still
        //     opened (and truncated!) before the dump fails.
        let cp = tmp_path("nulljson.c");
        let rp = tmp_path("nulljson.rust");
        std::fs::write(&cp, b"PREVIOUS CONTENTS").unwrap();
        std::fs::write(&rp, b"PREVIOUS CONTENTS").unwrap();
        let cps = cs(cp.to_str().unwrap());
        let rps = cs(rp.to_str().unwrap());
        diff_eq!(
            (c.json_dump_file)(std::ptr::null(), cps.as_ptr(), 0),
            (r.json_dump_file)(std::ptr::null(), rps.as_ptr(), 0),
            "json_dump_file(NULL json) return"
        );
        assert_eq!(
            (c.json_dump_file)(std::ptr::null(), cps.as_ptr(), 0),
            -1,
            "C: json_dump_file(NULL json) must be -1"
        );
        diff_eq!(
            Pretty(std::fs::read(&cp).unwrap()),
            Pretty(std::fs::read(&rp).unwrap()),
            "json_dump_file(NULL json) leaves the same file image"
        );
        assert!(
            std::fs::read(&cp).unwrap().is_empty(),
            "C: fopen(path,\"w\") truncates even when the dump then fails"
        );

        let cf = fopen(cps.as_ptr(), cs("w").as_ptr());
        let rf = fopen(rps.as_ptr(), cs("w").as_ptr());
        assert!(!cf.is_null() && !rf.is_null());
        diff_eq!(
            (c.json_dumpf)(std::ptr::null(), cf, 0),
            (r.json_dumpf)(std::ptr::null(), rf, 0),
            "json_dumpf(NULL json)"
        );
        assert_eq!((c.json_dumpf)(std::ptr::null(), cf, 0), -1, "C: json_dumpf(NULL) == -1");
        fclose(cf);
        fclose(rf);
        let _ = std::fs::remove_file(&cp);
        let _ = std::fs::remove_file(&rp);
    }
}

// ===========================================================================
// A NULL *callback* pointer (no ERRORS.md row — a pure FFI boundary)
// ===========================================================================

#[test]
fn null_callback_pointer_on_the_paths_that_never_dereference_it() {
    let _g = global_state_lock();
    // `json_dump_callback` takes a function POINTER, so NULL is a real input.
    // The C dereferences it only from the arms that actually emit bytes, so the
    // three paths that `return -1` first — the JSON_ENCODE_ANY gate (row
    // 200/201), do_dump's `!json` (row 203) and do_dump's `default:` (row 220)
    // — must all tolerate a NULL callback and return -1.
    //
    // Every OTHER shape (a real array/object/scalar) calls `dump(...)`
    // unconditionally and therefore segfaults in the C, so it is not asserted
    // here.
    let (c, r) = both();
    unsafe {
        // A json_t with a type outside JSON_OBJECT..JSON_NULL reaches do_dump's
        // `default:` arm. refcount = (size_t)-1 marks it as a never-freed
        // singleton, exactly like json_null(), so nothing tries to free the
        // stack slot.
        let bogus = json_t { type_: 99, refcount: usize::MAX };
        let bogus_p: *const json_t = &bogus;

        for &flags in &[
            0,
            JSON_ENCODE_ANY,
            JSON_ENCODE_ANY | JSON_COMPACT | json_indent(3),
            JSON_ENCODE_ANY | JSON_SORT_KEYS,
            usize::MAX,
        ] {
            let ctx = format!("flags={flags:#x}");
            diff_eq!(
                (c.json_dump_callback)(std::ptr::null(), None, std::ptr::null_mut(), flags),
                (r.json_dump_callback)(std::ptr::null(), None, std::ptr::null_mut(), flags),
                "json_dump_callback(NULL json, NULL callback) [{ctx}]"
            );
            assert_eq!(
                (c.json_dump_callback)(std::ptr::null(), None, std::ptr::null_mut(), flags),
                -1,
                "C: NULL json + NULL callback must be -1 [{ctx}]"
            );

            diff_eq!(
                (c.json_dump_callback)(bogus_p, None, std::ptr::null_mut(), flags),
                (r.json_dump_callback)(bogus_p, None, std::ptr::null_mut(), flags),
                "json_dump_callback(bogus type, NULL callback) [{ctx}]"
            );
            assert_eq!(
                (c.json_dump_callback)(bogus_p, None, std::ptr::null_mut(), flags),
                -1,
                "C: bogus type + NULL callback must be -1 [{ctx}]"
            );
        }

        // The gate runs before the callback is touched, so a scalar without
        // JSON_ENCODE_ANY is also safe with a NULL callback.
        for (name, mk) in scalar_makers() {
            let (cj, rj) = pair(c, r, mk);
            diff_eq!(
                (c.json_dump_callback)(cj, None, std::ptr::null_mut(), 0),
                (r.json_dump_callback)(rj, None, std::ptr::null_mut(), 0),
                "json_dump_callback({name}, NULL callback, no ENCODE_ANY)"
            );
            assert_eq!(
                (c.json_dump_callback)(cj, None, std::ptr::null_mut(), 0),
                -1,
                "C: the gate must reject {name} before touching the callback"
            );
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 198, 202, 210 — the three malloc sites inside a dump
// ===========================================================================

#[test]
fn rows_198_202_210_allocation_failure_budget_sweep() {
    let _g = global_state_lock();
    // The allocations a `json_dumps` of an object performs, in order:
    //   1. strbuffer_init            -> jsonp_malloc(16)          (row 198)
    //   2. hashtable_init            -> jsonp_malloc(8 * bucket)  (row 202)
    //   3. jsonp_loop_check          -> hashtable_set / init_pair (row 125)
    //   4. JSON_SORT_KEYS only:      -> jsonp_malloc(size * sizeof(struct key_len))
    //                                                             (row 210)
    // Sweeping a malloc budget therefore walks the failure sites one by one,
    // and the FIRST budget that succeeds must be exactly one higher with
    // JSON_SORT_KEYS than without — which is what proves row 210 was reached
    // and not merely documented.
    let (c, r) = both();
    unsafe {
        let (cj, rj) = pair(c, r, |a| {
            let o = (a.json_object)();
            oset(a, o, b"a", (a.json_integer)(1));
            oset(a, o, b"b", (a.json_integer)(2));
            oset(a, o, b"c", (a.json_integer)(3));
            o
        });
        let saved = install_hooks(c, r);

        let mut first_ok: Vec<Option<i64>> = Vec::new();
        for &flags in &[0usize, JSON_SORT_KEYS] {
            let mut ok_at: Option<i64> = None;
            for budget in 0..=8i64 {
                let (cb, cused) = dumps_budgeted(c, cj, flags, budget);
                let (rb, rused) = dumps_budgeted(r, rj, flags, budget);
                diff_eq!(
                    cb.clone(),
                    rb,
                    "json_dumps(flags={flags:#x}) with malloc budget {budget}"
                );
                diff_eq!(
                    cused,
                    rused,
                    "malloc call count for json_dumps(flags={flags:#x}) budget {budget}"
                );
                if cb.is_some() && ok_at.is_none() {
                    ok_at = Some(budget);
                }
                if budget == 0 {
                    assert!(cb.is_none(), "C row 198: strbuffer_init OOM must give NULL");
                }
                if budget == 1 {
                    assert!(cb.is_none(), "C row 202: hashtable_init OOM must give NULL");
                }
            }
            first_ok.push(ok_at);
        }
        assert_eq!(
            first_ok[1],
            first_ok[0].map(|b| b + 1),
            "C: JSON_SORT_KEYS must need exactly one more malloc (the struct key_len \
             array, row 210) than the unsorted path; unsorted first-ok={:?}, sorted={:?}",
            first_ok[0],
            first_ok[1]
        );
        assert!(first_ok[0].is_some(), "C: some budget must succeed");

        // An unlimited budget must consume the same number of mallocs in both
        // libraries for a bigger document too — allocation-site parity.
        let (cm, rm) = pair(c, r, |a| mixed(a));
        for &flags in &[0usize, JSON_SORT_KEYS, json_indent(2) | JSON_SORT_KEYS, JSON_COMPACT] {
            let (cb, cused) = dumps_budgeted(c, cm, flags, -1);
            let (rb, rused) = dumps_budgeted(r, rm, flags, -1);
            diff_eq!(cb.clone(), rb, "mixed doc json_dumps(flags={flags:#x}) under hooks");
            diff_eq!(cused, rused, "malloc call count, mixed doc, flags={flags:#x}");
            assert!(cb.is_some(), "C: the mixed doc must dump with an unlimited budget");
        }
        // ... and the same walk over the budget for the mixed document, whose
        // nested containers add one loop-check allocation each.
        for &flags in &[0usize, JSON_SORT_KEYS] {
            for budget in 0..=12i64 {
                let (cb, cused) = dumps_budgeted(c, cm, flags, budget);
                let (rb, rused) = dumps_budgeted(r, rm, flags, budget);
                diff_eq!(cb.clone(), rb, "mixed doc budget {budget} flags={flags:#x}");
                diff_eq!(cused, rused, "mixed doc malloc count budget {budget} flags={flags:#x}");
            }
        }

        // Every OTHER entry point must report the same OOM through its own
        // return convention.
        let cpath = tmp_path("oom.c");
        let rpath = tmp_path("oom.rust");
        let cps = cs(cpath.to_str().unwrap());
        let rps = cs(rpath.to_str().unwrap());
        for budget in 0..=3i64 {
            MALLOC_CALLS.store(0, Ordering::SeqCst);
            MALLOC_BUDGET.store(budget, Ordering::SeqCst);
            let (cn, cbuf) = dumpb64(c, cj, 64, JSON_SORT_KEYS);
            MALLOC_CALLS.store(0, Ordering::SeqCst);
            let (rn, rbuf) = dumpb64(r, rj, 64, JSON_SORT_KEYS);
            diff_eq!((cn, cbuf), (rn, rbuf), "json_dumpb under malloc budget {budget}");

            MALLOC_CALLS.store(0, Ordering::SeqCst);
            let crc = (c.json_dumpfd)(cj, -1, JSON_SORT_KEYS);
            MALLOC_CALLS.store(0, Ordering::SeqCst);
            let rrc = (r.json_dumpfd)(rj, -1, JSON_SORT_KEYS);
            diff_eq!(crc, rrc, "json_dumpfd under malloc budget {budget}");

            MALLOC_CALLS.store(0, Ordering::SeqCst);
            let crc = (c.json_dump_file)(cj, cps.as_ptr(), JSON_SORT_KEYS);
            MALLOC_CALLS.store(0, Ordering::SeqCst);
            let rrc = (r.json_dump_file)(rj, rps.as_ptr(), JSON_SORT_KEYS);
            diff_eq!(crc, rrc, "json_dump_file under malloc budget {budget}");
            diff_eq!(
                Pretty(std::fs::read(&cpath).unwrap()),
                Pretty(std::fs::read(&rpath).unwrap()),
                "json_dump_file image under malloc budget {budget}"
            );

            MALLOC_CALLS.store(0, Ordering::SeqCst);
            let (crc, cch, _) = run_cb(c, cj, JSON_SORT_KEYS, Sink::quiet());
            MALLOC_CALLS.store(0, Ordering::SeqCst);
            let (rrc, rch, _) = run_cb(r, rj, JSON_SORT_KEYS, Sink::quiet());
            MALLOC_BUDGET.store(-1, Ordering::SeqCst);
            diff_eq!(crc, rrc, "json_dump_callback under malloc budget {budget}");
            diff_eq!(cch, rch, "json_dump_callback chunks under malloc budget {budget}");
        }

        restore_hooks(c, r, &saved);
        decref(c, cj);
        decref(r, rj);
        decref(c, cm);
        decref(r, rm);
        let _ = std::fs::remove_file(&cpath);
        let _ = std::fs::remove_file(&rpath);
    }
}

// ===========================================================================
// Rows 199, 204, 211, 352, 353 — callback failures, and the asymmetry between
// a failure on an object KEY (swallowed) and on a VALUE (fatal)
// ===========================================================================

#[test]
fn rows_199_204_211_352_353_callback_failure_key_vs_value() {
    let _g = global_state_lock();
    // dump.c calls dump_string for object keys WITHOUT checking its result:
    //
    //     dump_string(key, key_len, dump, data, flags);
    //     if (dump(separator, separator_length, data) || do_dump(...))
    //
    // (identically in the JSON_SORT_KEYS branch). So a callback failure that
    // lands on a chunk emitted from inside dump_string FOR A KEY is silently
    // swallowed and the dump reports success, while exactly the same failure on
    // a chunk emitted for a VALUE aborts with -1. Rows 352/353.
    let (c, r) = both();
    unsafe {
        // {"k": "v"} — the chunk sequence is:
        //   0 "{"  1 "\""  2 "k"  3 "\""  4 ": "  5 "\""  6 "v"  7 "\""  8 "}"
        // so the `"` chunks 1 and 3 belong to the KEY and 5 and 7 to the VALUE.
        let (cj, rj) = pair(c, r, |a| {
            let o = (a.json_object)();
            oset(a, o, b"k", (a.json_string)(cs("v").as_ptr()));
            o
        });

        for &flags in &[0usize, JSON_SORT_KEYS, JSON_COMPACT, json_indent(2) | JSON_SORT_KEYS] {
            // Failing on the key's opening quote (occurrence 0) or closing
            // quote (occurrence 1) is SWALLOWED; on the value's quotes it is
            // fatal.
            for occurrence in 0..4 {
                let (crc, cch, cjoin) =
                    run_cb(c, cj, flags, Sink::failing_on(b"\"", occurrence));
                let (rrc, rch, rjoin) =
                    run_cb(r, rj, flags, Sink::failing_on(b"\"", occurrence));
                let ctx = format!("flags={flags:#x} fail on quote #{occurrence}");
                diff_eq!(crc, rrc, "return [{ctx}]");
                diff_eq!(cch, rch, "chunks [{ctx}]");
                diff_eq!(cjoin, rjoin, "bytes [{ctx}]");
                let expected = if occurrence < 2 { 0 } else { -1 };
                assert_eq!(
                    crc, expected,
                    "C rows 352/353: a dump_string failure on the KEY must be swallowed \
                     and on the VALUE must be fatal [{ctx}]"
                );
            }

            // Failing on the key BODY chunk vs the value BODY chunk.
            let (crc, cch, cjoin) = run_cb(c, cj, flags, Sink::failing_on(b"k", 0));
            let (rrc, rch, rjoin) = run_cb(r, rj, flags, Sink::failing_on(b"k", 0));
            diff_eq!(crc, rrc, "return, fail on key body, flags={flags:#x}");
            diff_eq!(cch, rch, "chunks, fail on key body, flags={flags:#x}");
            diff_eq!(cjoin, rjoin, "bytes, fail on key body, flags={flags:#x}");
            assert_eq!(crc, 0, "C row 352/353: key body failure is swallowed");

            let (crc, cch, cjoin) = run_cb(c, cj, flags, Sink::failing_on(b"v", 0));
            let (rrc, rch, rjoin) = run_cb(r, rj, flags, Sink::failing_on(b"v", 0));
            diff_eq!(crc, rrc, "return, fail on value body, flags={flags:#x}");
            diff_eq!(cch, rch, "chunks, fail on value body, flags={flags:#x}");
            diff_eq!(cjoin, rjoin, "bytes, fail on value body, flags={flags:#x}");
            assert_eq!(crc, -1, "C row 204: a value failure is fatal");

            // Row 204: every structural chunk IS checked.
            for target in [&b"{"[..], b": ", b":", b"}", b","] {
                let (crc, cch, cjoin) = run_cb(c, cj, flags, Sink::failing_on(target, 0));
                let (rrc, rch, rjoin) = run_cb(r, rj, flags, Sink::failing_on(target, 0));
                let ctx = format!("flags={flags:#x} fail on {:?}", String::from_utf8_lossy(target));
                diff_eq!(crc, rrc, "return [{ctx}]");
                diff_eq!(cch, rch, "chunks [{ctx}]");
                diff_eq!(cjoin, rjoin, "bytes [{ctx}]");
            }
        }
        decref(c, cj);
        decref(r, rj);

        // Row 211: dump_indent's own two chunks ("\n" and the whitespace run)
        // ARE checked, so failing on them is fatal at every nesting level.
        let (cm, rm) = pair(c, r, |a| mixed(a));
        for &flags in &[json_indent(1), json_indent(2), json_indent(31), json_indent(2) | JSON_SORT_KEYS] {
            for target in [&b"\n"[..], b" ", b"  ", b"    "] {
                for occurrence in 0..3 {
                    let (crc, cch, cjoin) = run_cb(c, cm, flags, Sink::failing_on(target, occurrence));
                    let (rrc, rch, rjoin) = run_cb(r, rm, flags, Sink::failing_on(target, occurrence));
                    let ctx = format!(
                        "row211 flags={flags:#x} target={:?} #{occurrence}",
                        String::from_utf8_lossy(target)
                    );
                    diff_eq!(crc, rrc, "return [{ctx}]");
                    diff_eq!(cch, rch, "chunks [{ctx}]");
                    diff_eq!(cjoin, rjoin, "bytes [{ctx}]");
                }
            }
        }

        // Row 199: whatever made json_dump_callback fail, json_dumps turns it
        // into NULL, json_dumpb into 0, json_dumpf/fd into -1. Compare all of
        // them for a callback-independent failure (a scalar without
        // JSON_ENCODE_ANY) to keep the conventions pinned.
        let (ci, ri) = pair(c, r, |a| (a.json_integer)(1));
        diff_eq!(dumps(c, ci, 0), dumps(r, ri, 0), "row199 json_dumps");
        assert!(dumps(c, ci, 0).is_none(), "C row 199: json_dumps must be NULL");
        decref(c, ci);
        decref(r, ri);
        decref(c, cm);
        decref(r, rm);

        // A callback returning a POSITIVE value is just as fatal as -1: the C
        // tests `if (dump(...))`, not `< 0`.
        let (cj, rj) = pair(c, r, |a| {
            let arr = (a.json_array)();
            apush(a, arr, (a.json_integer)(1));
            arr
        });
        for ret in [1i32, 7, i32::MAX, -1, i32::MIN + 1] {
            let mut cs_ = Sink::failing_on(b"[", 0);
            cs_.fail_ret = ret;
            let mut rs_ = Sink::failing_on(b"[", 0);
            rs_.fail_ret = ret;
            let (crc, cch, _) = run_cb(c, cj, 0, cs_);
            let (rrc, rch, _) = run_cb(r, rj, 0, rs_);
            diff_eq!(crc, rrc, "callback returning {ret} return value");
            diff_eq!(cch, rch, "callback returning {ret} chunks");
            assert_eq!(crc, -1, "C: any non-zero callback result must give -1 (ret={ret})");
        }
        decref(c, cj);
        decref(r, rj);
    }
}

// ===========================================================================
// Rows 205, 206 — circular references, through every entry point
// ===========================================================================

#[test]
fn rows_205_206_circular_references_every_entry_point() {
    let _g = global_state_lock();
    // The C REFUSES direct self-insertion (`json == value` in
    // json_array_set_new / json_object_setn_new_nocheck, ERRORS.md rows 11/57),
    // so a cycle has to be built INDIRECTLY. `jsonp_loop_check` then finds the
    // "%p" key of an ancestor already in the parents hashtable.
    let (c, r) = both();
    unsafe {
        // Direct self-insertion is rejected identically first.
        let (ca, ra) = pair(c, r, |a| (a.json_array)());
        diff_eq!(apush_ref(c, ca, ca), apush_ref(r, ra, ra), "row 57: json_array_append(a, a)");
        assert_eq!(apush_ref(c, ca, ca), -1, "C: direct self-insertion must be refused");
        diff_eq!(
            (c.json_array_size)(ca),
            (r.json_array_size)(ra),
            "size after refused self-insertion"
        );
        let (co, ro) = pair(c, r, |a| (a.json_object)());
        diff_eq!(oset_ref(c, co, b"k", co), oset_ref(r, ro, b"k", ro), "row 11: o[\"k\"] = o");
        assert_eq!(oset_ref(c, co, b"k", co), -1, "C: direct self-insertion must be refused");
        decref(c, ca);
        decref(r, ra);
        decref(c, co);
        decref(r, ro);

        // Each builder returns the node to dump. The cycles keep themselves
        // alive by construction, so they are deliberately leaked — decref'ing
        // one node of a reference cycle cannot free it, and calling
        // json_delete on a live cycle would double-free.
        type Mk = fn(&Api) -> *mut json_t;
        let cyclic: Vec<(&str, Mk)> = vec![
            ("2-cycle arrays a=[b] b=[a]", |a: &Api| unsafe {
                let x = (a.json_array)();
                let y = (a.json_array)();
                apush_ref(a, x, y);
                apush_ref(a, y, x);
                x
            }),
            ("3-cycle arrays", |a: &Api| unsafe {
                let x = (a.json_array)();
                let y = (a.json_array)();
                let z = (a.json_array)();
                apush_ref(a, x, y);
                apush_ref(a, y, z);
                apush_ref(a, z, x);
                x
            }),
            ("2-cycle objects a={\"b\":b} b={\"a\":a}", |a: &Api| unsafe {
                let x = (a.json_object)();
                let y = (a.json_object)();
                oset_ref(a, x, b"b", y);
                oset_ref(a, y, b"a", x);
                x
            }),
            ("array -> object -> same array", |a: &Api| unsafe {
                let x = (a.json_array)();
                let o = (a.json_object)();
                oset_ref(a, o, b"back", x);
                apush(a, x, o);
                x
            }),
            ("object -> array -> same object", |a: &Api| unsafe {
                let o = (a.json_object)();
                let x = (a.json_array)();
                apush_ref(a, x, o);
                oset(a, o, b"arr", x);
                o
            }),
            ("cycle behind a scalar sibling", |a: &Api| unsafe {
                let x = (a.json_array)();
                apush(a, x, (a.json_integer)(1));
                let y = (a.json_array)();
                apush(a, y, (a.json_string)(cs("s").as_ptr()));
                apush_ref(a, y, x);
                apush_ref(a, x, y);
                x
            }),
        ];

        let cpath = tmp_path("cycle.c");
        let rpath = tmp_path("cycle.rust");
        let cps = cs(cpath.to_str().unwrap());
        let rps = cs(rpath.to_str().unwrap());

        for (label, mk) in &cyclic {
            let (cj, rj) = pair(c, r, mk);
            for &flags in &[
                0usize,
                JSON_COMPACT,
                json_indent(2),
                JSON_SORT_KEYS,
                JSON_SORT_KEYS | json_indent(4),
                JSON_ENCODE_ANY,
                JSON_EMBED,
                KNOWN_FLAG_BITS,
            ] {
                let ctx = format!("{label} flags={flags:#x}");

                // json_dumps -> NULL
                let cd = dumps(c, cj, flags);
                let rd = dumps(r, rj, flags);
                diff_eq!(cd.clone(), rd, "json_dumps [{ctx}]");
                assert!(cd.is_none(), "C rows 205/206: a cycle must give NULL [{ctx}]");

                // json_dumpb -> 0, nothing written
                let (cn, cbuf) = dumpb64(c, cj, 64, flags);
                let (rn, rbuf) = dumpb64(r, rj, 64, flags);
                diff_eq!((cn, cbuf), (rn, rbuf), "json_dumpb [{ctx}]");
                assert_eq!(cn, 0, "C row 212: a failed dump gives 0 [{ctx}]");

                // json_dumpfd -> -1
                diff_eq!(
                    (c.json_dumpfd)(cj, -1, flags),
                    (r.json_dumpfd)(rj, -1, flags),
                    "json_dumpfd [{ctx}]"
                );

                // json_dump_file -> -1, and both leave the same file image
                let crc = (c.json_dump_file)(cj, cps.as_ptr(), flags);
                let rrc = (r.json_dump_file)(rj, rps.as_ptr(), flags);
                diff_eq!(crc, rrc, "json_dump_file [{ctx}]");
                assert_eq!(crc, -1, "C row 217: an inner failure gives -1 [{ctx}]");
                diff_eq!(
                    Pretty(std::fs::read(&cpath).unwrap()),
                    Pretty(std::fs::read(&rpath).unwrap()),
                    "json_dump_file partial image [{ctx}]"
                );

                // json_dump_callback -> -1, same partial chunk sequence
                let (crc, cch, cjoin) = run_cb(c, cj, flags, Sink::quiet());
                let (rrc, rch, rjoin) = run_cb(r, rj, flags, Sink::quiet());
                diff_eq!(crc, rrc, "json_dump_callback return [{ctx}]");
                diff_eq!(cch, rch, "json_dump_callback chunks [{ctx}]");
                diff_eq!(cjoin, rjoin, "json_dump_callback bytes [{ctx}]");
                assert_eq!(crc, -1, "C: a cycle must give -1 [{ctx}]");
            }
            // deliberately leaked; see the comment above
        }

        // A shared subtree (a DAG, not a cycle) must still SUCCEED: the loop
        // check removes each node from `parents` on the way out
        // (hashtable_del), so the same child may legitimately appear twice.
        let dags: Vec<(&str, Mk)> = vec![
            ("array holding the same child twice", |a: &Api| unsafe {
                let shared = (a.json_array)();
                apush(a, shared, (a.json_integer)(1));
                apush(a, shared, (a.json_integer)(2));
                let root = (a.json_array)();
                apush_ref(a, root, shared);
                apush_ref(a, root, shared);
                decref(a, shared);
                root
            }),
            ("object with two keys onto one child", |a: &Api| unsafe {
                let shared = (a.json_object)();
                oset(a, shared, b"x", (a.json_integer)(1));
                let root = (a.json_object)();
                oset_ref(a, root, b"p", shared);
                oset_ref(a, root, b"q", shared);
                decref(a, shared);
                root
            }),
            ("diamond: root -> [l, r] -> same leaf", |a: &Api| unsafe {
                let leaf = (a.json_array)();
                apush(a, leaf, (a.json_string)(cs("leaf").as_ptr()));
                let l = (a.json_array)();
                apush_ref(a, l, leaf);
                let rr = (a.json_array)();
                apush_ref(a, rr, leaf);
                let root = (a.json_array)();
                apush(a, root, l);
                apush(a, root, rr);
                decref(a, leaf);
                root
            }),
            ("sibling repeats at three depths", |a: &Api| unsafe {
                let shared = (a.json_object)();
                oset(a, shared, b"s", (a.json_true)());
                let mid = (a.json_array)();
                apush_ref(a, mid, shared);
                apush_ref(a, mid, shared);
                let root = (a.json_array)();
                apush_ref(a, root, shared);
                apush(a, root, mid);
                decref(a, shared);
                root
            }),
        ];
        for (label, mk) in &dags {
            let (cj, rj) = pair(c, r, mk);
            for &flags in &[0usize, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS] {
                let cd = dumps(c, cj, flags);
                let rd = dumps(r, rj, flags);
                diff_eq!(cd.clone(), rd, "DAG {label} flags={flags:#x}");
                assert!(
                    cd.is_some(),
                    "C: a shared subtree is NOT a cycle and must dump: {label}"
                );
            }
            decref(c, cj);
            decref(r, rj);
        }

        let _ = std::fs::remove_file(&cpath);
        let _ = std::fs::remove_file(&rpath);
    }
}

// ===========================================================================
// Row 207 — invalid UTF-8 inside a string makes utf8_iterate fail in
// dump_string
// ===========================================================================

fn bad_utf8() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("lone 0xff", b"\xff"),
        ("lone 0xfe", b"\xfe"),
        ("bare continuation 0x80", b"\x80"),
        ("bare continuation 0xbf", b"\xbf"),
        ("overlong lead 0xc0", b"\xc0\x80"),
        ("overlong lead 0xc1", b"\xc1\x81"),
        ("truncated 2-byte", b"\xc2"),
        ("bad continuation", b"\xc2\x41"),
        ("surrogate", b"\xed\xa0\x80"),
        ("overlong 3-byte", b"\xe0\x80\x80"),
        ("overlong 4-byte", b"\xf0\x80\x80\x80"),
        ("above U+10FFFF", b"\xf4\x90\x80\x80"),
        ("lead 0xf5", b"\xf5\x80\x80\x80"),
        ("valid then invalid", b"a\xffb"),
        ("invalid after an escape", b"\t\xff"),
        ("invalid at the very end", b"abc\xe0"),
    ]
}

#[test]
fn row_207_invalid_utf8_string_value_aborts_the_dump() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let cpath = tmp_path("badutf8.c");
        let rpath = tmp_path("badutf8.rust");
        let cps = cs(cpath.to_str().unwrap());
        let rps = cs(rpath.to_str().unwrap());

        for (label, raw) in bad_utf8() {
            // json_string() would reject these, so they can only be built with
            // the _nocheck entry point (ERRORS.md row 76 vs 71).
            let mkv = |a: &Api| -> *mut json_t {
                (a.json_stringn_nocheck)(raw.as_ptr() as *const c_char, raw.len())
            };
            let (cv, rv) = pair(c, r, mkv);
            assert!(!cv.is_null(), "C: json_stringn_nocheck must accept {label}");
            diff_eq!(cv.is_null(), rv.is_null(), "json_stringn_nocheck({label})");

            for &flags in &[
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
                JSON_ENCODE_ANY | JSON_ESCAPE_SLASH,
                JSON_ENCODE_ANY | JSON_COMPACT | json_indent(2),
            ] {
                let ctx = format!("{label} flags={flags:#x}");
                let cd = dumps(c, cv, flags);
                let rd = dumps(r, rv, flags);
                diff_eq!(cd.clone(), rd, "top-level bad string [{ctx}]");
                assert!(cd.is_none(), "C row 207: invalid UTF-8 must give NULL [{ctx}]");

                let (cn, cbuf) = dumpb64(c, cv, 64, flags);
                let (rn, rbuf) = dumpb64(r, rv, 64, flags);
                diff_eq!((cn, cbuf), (rn, rbuf), "json_dumpb bad string [{ctx}]");
                assert_eq!(cn, 0, "C row 212: json_dumpb reports 0 [{ctx}]");

                let (crc, cch, cjoin) = run_cb(c, cv, flags, Sink::quiet());
                let (rrc, rch, rjoin) = run_cb(r, rv, flags, Sink::quiet());
                diff_eq!(crc, rrc, "callback return [{ctx}]");
                diff_eq!(cch, rch, "callback chunks [{ctx}]");
                diff_eq!(cjoin, rjoin, "callback bytes [{ctx}]");
                assert_eq!(crc, -1, "C: -1 [{ctx}]");
            }
            decref(c, cv);
            decref(r, rv);

            // As an array element and as an object VALUE: still fatal.
            let mk_in_arr = move |a: &Api| -> *mut json_t {
                let arr = (a.json_array)();
                apush(a, arr, (a.json_integer)(1));
                apush(a, arr, (a.json_stringn_nocheck)(raw.as_ptr() as *const c_char, raw.len()));
                apush(a, arr, (a.json_integer)(2));
                arr
            };
            let (cj, rj) = pair(c, r, mk_in_arr);
            for &flags in &[0usize, JSON_COMPACT, json_indent(2), JSON_ENSURE_ASCII] {
                let cd = dumps(c, cj, flags);
                let rd = dumps(r, rj, flags);
                diff_eq!(cd.clone(), rd, "bad string in array [{label}] flags={flags:#x}");
                assert!(cd.is_none(), "C: fatal inside an array [{label}]");
                let crc = (c.json_dump_file)(cj, cps.as_ptr(), flags);
                let rrc = (r.json_dump_file)(rj, rps.as_ptr(), flags);
                diff_eq!(crc, rrc, "json_dump_file [{label}] flags={flags:#x}");
                assert_eq!(crc, -1, "C row 217 [{label}]");
                diff_eq!(
                    Pretty(std::fs::read(&cpath).unwrap()),
                    Pretty(std::fs::read(&rpath).unwrap()),
                    "partial file image [{label}] flags={flags:#x}"
                );
            }
            decref(c, cj);
            decref(r, rj);

            let mk_as_value = move |a: &Api| -> *mut json_t {
                let o = (a.json_object)();
                oset(a, o, b"good", (a.json_integer)(1));
                oset(
                    a,
                    o,
                    b"bad",
                    (a.json_stringn_nocheck)(raw.as_ptr() as *const c_char, raw.len()),
                );
                o
            };
            let (cj, rj) = pair(c, r, mk_as_value);
            for &flags in &[0usize, JSON_SORT_KEYS, JSON_COMPACT | JSON_SORT_KEYS] {
                let cd = dumps(c, cj, flags);
                let rd = dumps(r, rj, flags);
                diff_eq!(cd.clone(), rd, "bad string as value [{label}] flags={flags:#x}");
                assert!(cd.is_none(), "C: fatal as an object value [{label}]");
            }
            decref(c, cj);
            decref(r, rj);
        }
        let _ = std::fs::remove_file(&cpath);
        let _ = std::fs::remove_file(&rpath);
    }
}

#[test]
fn rows_352_353_invalid_utf8_object_key_is_silently_emitted() {
    let _g = global_state_lock();
    // The other half of rows 352/353: because dump_string's result is ignored
    // at BOTH object-key call sites, an invalid-UTF-8 KEY does not fail the
    // dump — it produces a truncated, malformed (and unparseable) document.
    // The same bytes must come out of both libraries.
    let (c, r) = both();
    unsafe {
        for (label, raw) in bad_utf8() {
            let mk = move |a: &Api| -> *mut json_t {
                let o = (a.json_object)();
                // json_object_setn_new (checked) would refuse this key
                // (ERRORS.md row 15); the _nocheck form accepts it.
                assert_eq!(
                    oset(a, o, raw, (a.json_integer)(1)),
                    0,
                    "setn_new_nocheck must accept a non-UTF-8 key"
                );
                oset(a, o, b"z", (a.json_integer)(2));
                o
            };
            let (cj, rj) = pair(c, r, mk);
            diff_eq!(
                (c.json_object_size)(cj),
                (r.json_object_size)(rj),
                "object size with a bad key [{label}]"
            );
            for &flags in &[
                0usize,
                JSON_SORT_KEYS,
                JSON_COMPACT,
                JSON_COMPACT | JSON_SORT_KEYS,
                json_indent(2),
                json_indent(2) | JSON_SORT_KEYS,
                JSON_ENSURE_ASCII | JSON_SORT_KEYS,
            ] {
                let ctx = format!("{label} flags={flags:#x}");
                let cd = dumps(c, cj, flags);
                let rd = dumps(r, rj, flags);
                diff_eq!(cd.clone(), rd, "bad KEY dump [{ctx}]");
                assert!(
                    cd.is_some(),
                    "C rows 352/353: dump_string's failure on a KEY is IGNORED, so the \
                     dump must SUCCEED [{ctx}]"
                );

                let (crc, cch, cjoin) = run_cb(c, cj, flags, Sink::quiet());
                let (rrc, rch, rjoin) = run_cb(r, rj, flags, Sink::quiet());
                diff_eq!(crc, rrc, "bad KEY callback return [{ctx}]");
                diff_eq!(cch, rch, "bad KEY callback chunks [{ctx}]");
                diff_eq!(cjoin, rjoin, "bad KEY callback bytes [{ctx}]");
                assert_eq!(crc, 0, "C: a bad key is not an error [{ctx}]");
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 208, 209 — jsonp_dtostr failing for a JSON_REAL
// ===========================================================================

#[test]
fn rows_208_209_real_precision_makes_dtostr_fail() {
    let _g = global_state_lock();
    // do_dump formats a real into a 25-byte stack buffer:
    //
    //     size = jsonp_dtostr(buffer, MAX_REAL_STR_LENGTH, value,
    //                         FLAGS_TO_PRECISION(flags));
    //     if (size < 0) return -1;
    //
    // and MAX_REAL_STR_LENGTH is only 25, so a high JSON_REAL_PRECISION makes
    // the formatting fail: row 208 via the `3 + digits + exp > size` length
    // check (n in 22..24 for 0.1) and row 209 via dtoa_r's `blen <= ndigits`
    // NULL (n >= 25). The failure set must match exactly, value by value and
    // precision by precision.
    let (c, r) = both();
    unsafe {
        let values: &[(&str, f64)] = &[
            ("0.1", 0.1),
            ("0.0", 0.0),
            ("-0.0", -0.0),
            ("1.0", 1.0),
            ("-1.0", -1.0),
            ("1/3", 1.0 / 3.0),
            ("0.30000000000000004", 0.30000000000000004),
            ("1e300", 1e300),
            ("1e-300", 1e-300),
            ("f64::MAX", f64::MAX),
            ("5e-324", 5e-324),
            ("f64::MIN_POSITIVE", f64::MIN_POSITIVE),
            ("2.2250738585072011e-308", 2.2250738585072011e-308),
            ("1e21", 1e21),
            ("1e-7", 1e-7),
            ("123456789012345678.0", 123456789012345678.0),
            ("-9007199254740993.0", -9007199254740993.0),
        ];
        let mut zero_one_failures: Vec<usize> = Vec::new();
        for &(label, v) in values {
            let (cv, rv) = pair(c, r, |a| (a.json_real)(v));
            assert!(!cv.is_null(), "C: json_real({label}) must exist");
            // Same real nested in a container, to prove the -1 propagates.
            let (carr, rarr) = pair(c, r, |a| {
                let arr = (a.json_array)();
                apush(a, arr, (a.json_real)(v));
                arr
            });
            let (cobj, robj) = pair(c, r, |a| {
                let o = (a.json_object)();
                oset(a, o, b"k", (a.json_real)(v));
                o
            });

            for prec in 0..=31usize {
                let flags = JSON_ENCODE_ANY | json_real_precision(prec);
                let ctx = format!("{label} precision={prec}");

                let cd = dumps(c, cv, flags);
                let rd = dumps(r, rv, flags);
                diff_eq!(cd.clone(), rd, "json_dumps(real) [{ctx}]");
                if label == "0.1" && cd.is_none() {
                    zero_one_failures.push(prec);
                }

                let ca = dumps(c, carr, flags);
                let ra = dumps(r, rarr, flags);
                diff_eq!(ca.clone(), ra, "json_dumps([real]) [{ctx}]");
                assert_eq!(
                    ca.is_none(),
                    cd.is_none(),
                    "C: the dtostr failure must propagate out of the array [{ctx}]"
                );

                let co = dumps(c, cobj, flags);
                let ro = dumps(r, robj, flags);
                diff_eq!(co.clone(), ro, "json_dumps({{real}}) [{ctx}]");
                assert_eq!(
                    co.is_none(),
                    cd.is_none(),
                    "C: the dtostr failure must propagate out of the object [{ctx}]"
                );

                // ... and through every other entry point.
                let (cn, cbuf) = dumpb64(c, cv, 64, flags);
                let (rn, rbuf) = dumpb64(r, rv, 64, flags);
                diff_eq!((cn, cbuf), (rn, rbuf), "json_dumpb(real) [{ctx}]");
                if cd.is_none() {
                    assert_eq!(cn, 0, "C row 212: 0 for a failed real [{ctx}]");
                    assert_eq!(cbuf, [0xAAu8; 64], "C: nothing written [{ctx}]");
                }
                let (crc, cch, cjoin) = run_cb(c, cv, flags, Sink::quiet());
                let (rrc, rch, rjoin) = run_cb(r, rv, flags, Sink::quiet());
                diff_eq!(crc, rrc, "callback(real) return [{ctx}]");
                diff_eq!(cch, rch, "callback(real) chunks [{ctx}]");
                diff_eq!(cjoin, rjoin, "callback(real) bytes [{ctx}]");
                assert_eq!(crc, if cd.is_none() { -1 } else { 0 }, "C consistency [{ctx}]");
                diff_eq!(
                    (c.json_dumpfd)(cv, -1, flags),
                    (r.json_dumpfd)(rv, -1, flags),
                    "json_dumpfd(real, bad fd) [{ctx}]"
                );
            }
            decref(c, cv);
            decref(r, rv);
            decref(c, carr);
            decref(r, rarr);
            decref(c, cobj);
            decref(r, robj);
        }
        // The exact window ERRORS.md records for json_real(0.1): rows 208
        // (22..24) and 209 (>= 25) together mean 22..=31 all fail and nothing
        // below 22 does.
        assert_eq!(
            zero_one_failures,
            (22..=31).collect::<Vec<usize>>(),
            "C rows 208/209: json_real(0.1) must fail for exactly precision 22..=31"
        );
    }
}

// ===========================================================================
// Rows 212, 213 — json_dumpb's return convention and its ambiguity
// ===========================================================================

#[test]
fn rows_212_213_json_dumpb_zero_is_ambiguous() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Row 213 (NOT an error): a `size` smaller than the output is fine —
        // json_dumpb returns the number of bytes the dump WOULD need and
        // copies only what fits.
        let (cj, rj) = pair(c, r, |a| {
            let arr = (a.json_array)();
            apush(a, arr, (a.json_integer)(12345));
            apush(a, arr, (a.json_string)(cs("ab").as_ptr()));
            arr
        });
        let full = dumps(c, cj, 0).unwrap();
        let need = full.0.len();
        for size in 0..=(need + 4) {
            let (cn, cbuf) = dumpb64(c, cj, size, 0);
            let (rn, rbuf) = dumpb64(r, rj, size, 0);
            diff_eq!((cn, cbuf), (rn, rbuf), "json_dumpb(size={size})");
            assert_eq!(cn, need, "C row 213: the required count is always returned");
        }
        // NULL buffer + size 0 is the documented "how big would it be?" call.
        let cn = (c.json_dumpb)(cj, std::ptr::null_mut(), 0, 0);
        let rn = (r.json_dumpb)(rj, std::ptr::null_mut(), 0, 0);
        diff_eq!(cn, rn, "json_dumpb(NULL, 0) sizing call (row 213)");
        assert_eq!(cn, need, "C row 213: sizing call must return {need}");

        // An oversized `size` is not validated at all: dump_to_buffer only ever
        // memcpy's the bytes it actually produces, so `(size_t)-1` with a
        // buffer big enough for the real output behaves like an exact fit.
        for &size in &[usize::MAX, usize::MAX - 1, usize::MAX / 2, 1 << 40] {
            let mut cbuf = [0xAAu8; 4096];
            let mut rbuf = [0xAAu8; 4096];
            let cn = (c.json_dumpb)(cj, cbuf.as_mut_ptr() as *mut c_char, size, 0);
            let rn = (r.json_dumpb)(rj, rbuf.as_mut_ptr() as *mut c_char, size, 0);
            diff_eq!(cn, rn, "json_dumpb(size={size:#x}) return");
            diff_eq!(Pretty(cbuf.to_vec()), Pretty(rbuf.to_vec()), "json_dumpb(size={size:#x}) buffer");
            assert_eq!(cn, need, "C: an oversized size is not an error");
            assert_eq!(&cbuf[..need], &full.0[..], "C: the whole dump is written");
        }
        decref(c, cj);
        decref(r, rj);

        // Row 212: the SAME return value, 0, means "failed" ...
        let (ce, re) = pair(c, r, |a| (a.json_integer)(5));
        let (cn, cbuf) = dumpb64(c, ce, 64, 0);
        let (rn, rbuf) = dumpb64(r, re, 64, 0);
        diff_eq!((cn, cbuf), (rn, rbuf), "row212 failing json_dumpb");
        assert_eq!(cn, 0, "C row 212: a failure is reported as 0");
        assert_eq!(cbuf, [0xAAu8; 64], "C: and nothing is written");
        decref(c, ce);
        decref(r, re);

        // ... and "succeeded, with an empty dump": JSON_EMBED of an empty
        // container emits nothing at all, so 0 is genuinely ambiguous.
        for (label, mk) in [
            ("empty array", (|a: &Api| unsafe { (a.json_array)() }) as fn(&Api) -> *mut json_t),
            ("empty object", |a: &Api| unsafe { (a.json_object)() }),
        ] {
            let (cj, rj) = pair(c, r, mk);
            let (cn, cbuf) = dumpb64(c, cj, 64, JSON_EMBED);
            let (rn, rbuf) = dumpb64(r, rj, 64, JSON_EMBED);
            diff_eq!((cn, cbuf), (rn, rbuf), "row212 empty EMBED dump of {label}");
            assert_eq!(cn, 0, "C row 212: an empty EMBED dump also returns 0 [{label}]");
            assert_eq!(cbuf, [0xAAu8; 64], "C: nothing written [{label}]");
            // The success/failure difference is only visible through another
            // entry point.
            assert_eq!(
                (c.json_dump_callback)(cj, Some(sink_cb), &mut Sink::quiet() as *mut Sink as *mut c_void, JSON_EMBED),
                0,
                "C: the empty EMBED dump SUCCEEDED [{label}]"
            );
            diff_eq!(
                (c.json_dump_callback)(cj, Some(sink_cb), &mut Sink::quiet() as *mut Sink as *mut c_void, JSON_EMBED),
                (r.json_dump_callback)(rj, Some(sink_cb), &mut Sink::quiet() as *mut Sink as *mut c_void, JSON_EMBED),
                "empty EMBED dump return [{label}]"
            );
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Rows 214, 215, 216, 217, 218 — the FILE*/fd/path entry points
// ===========================================================================

#[test]
fn rows_214_215_216_217_218_file_and_fd_failures() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let (cj, rj) = pair(c, r, |a| mixed(a));

        // --- Row 214: fwrite fails because the FILE* is read-only.
        let cro = tmp_path("readonly.c");
        let rro = tmp_path("readonly.rust");
        std::fs::write(&cro, b"KEEP ME").unwrap();
        std::fs::write(&rro, b"KEEP ME").unwrap();
        let cf = fopen(cs(cro.to_str().unwrap()).as_ptr(), cs("r").as_ptr());
        let rf = fopen(cs(rro.to_str().unwrap()).as_ptr(), cs("r").as_ptr());
        assert!(!cf.is_null() && !rf.is_null(), "could not open the read-only files");
        let crc = (c.json_dumpf)(cj, cf, 0);
        let rrc = (r.json_dumpf)(rj, rf, 0);
        diff_eq!(crc, rrc, "row214 json_dumpf to a read-only FILE*");
        assert_eq!(crc, -1, "C row 214: fwrite failure must give -1");
        fclose(cf);
        fclose(rf);
        diff_eq!(
            Pretty(std::fs::read(&cro).unwrap()),
            Pretty(std::fs::read(&rro).unwrap()),
            "row214 the file must be untouched"
        );
        assert_eq!(std::fs::read(&cro).unwrap(), b"KEEP ME".to_vec(), "C: untouched");

        // --- Row 215: write() fails for every kind of unusable descriptor.
        let closed = open(cs(cro.to_str().unwrap()).as_ptr(), O_RDONLY, 0);
        assert!(closed >= 0);
        close(closed);
        let rdonly = open(cs(cro.to_str().unwrap()).as_ptr(), O_RDONLY, 0);
        assert!(rdonly >= 0);
        for (label, fd) in [
            ("-1", -1),
            ("closed", closed),
            ("read-only", rdonly),
            ("never opened", 424242),
            ("INT_MIN+1", i32::MIN + 1),
            ("INT_MAX", i32::MAX),
        ] {
            for &flags in &[0usize, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS] {
                diff_eq!(
                    (c.json_dumpfd)(cj, fd, flags),
                    (r.json_dumpfd)(rj, fd, flags),
                    "row215 json_dumpfd(fd={label}) flags={flags:#x}"
                );
                assert_eq!(
                    (c.json_dumpfd)(cj, fd, flags),
                    -1,
                    "C row 215: fd={label} must give -1"
                );
            }
        }
        close(rdonly);

        // --- Row 216: fopen fails.
        for bad in [
            "/nonexistent-dir-a12/x.json",
            "",
            "/proc/self/cmdline/x",
            "/dev/null/x",
            "/",
        ] {
            let b = cs(bad);
            diff_eq!(
                (c.json_dump_file)(cj, b.as_ptr(), 0),
                (r.json_dump_file)(rj, b.as_ptr(), 0),
                "row216 json_dump_file({bad:?})"
            );
            assert_eq!(
                (c.json_dump_file)(cj, b.as_ptr(), 0),
                -1,
                "C row 216: an unopenable path must give -1"
            );
        }

        // --- Row 217: the path opens but the dump fails; the file is still
        //     created (and truncated) and then closed.
        let cp = tmp_path("innerfail.c");
        let rp = tmp_path("innerfail.rust");
        let cps = cs(cp.to_str().unwrap());
        let rps = cs(rp.to_str().unwrap());
        std::fs::write(&cp, b"OLD OLD OLD").unwrap();
        std::fs::write(&rp, b"OLD OLD OLD").unwrap();
        let (cbad, rbad) = pair(c, r, |a| {
            let arr = (a.json_array)();
            apush(a, arr, (a.json_integer)(1));
            apush(a, arr, (a.json_stringn_nocheck)(b"\xff".as_ptr() as *const c_char, 1));
            arr
        });
        for &flags in &[0usize, json_indent(2), JSON_COMPACT] {
            let crc = (c.json_dump_file)(cbad, cps.as_ptr(), flags);
            let rrc = (r.json_dump_file)(rbad, rps.as_ptr(), flags);
            diff_eq!(crc, rrc, "row217 inner failure flags={flags:#x}");
            assert_eq!(crc, -1, "C row 217: an inner failure must give -1");
            diff_eq!(
                Pretty(std::fs::read(&cp).unwrap()),
                Pretty(std::fs::read(&rp).unwrap()),
                "row217 partial file image flags={flags:#x}"
            );
        }
        // A scalar without JSON_ENCODE_ANY fails before ANY byte is produced,
        // so the file must be exactly empty.
        let (ci, ri) = pair(c, r, |a| (a.json_integer)(7));
        diff_eq!(
            (c.json_dump_file)(ci, cps.as_ptr(), 0),
            (r.json_dump_file)(ri, rps.as_ptr(), 0),
            "row217 scalar without ENCODE_ANY"
        );
        assert!(std::fs::read(&cp).unwrap().is_empty(), "C: truncated, nothing written");
        diff_eq!(
            Pretty(std::fs::read(&cp).unwrap()),
            Pretty(std::fs::read(&rp).unwrap()),
            "row217 empty file image"
        );
        decref(c, ci);
        decref(r, ri);
        decref(c, cbad);
        decref(r, rbad);

        // --- Row 218: the dump itself succeeds but fclose() reports the
        //     deferred ENOSPC. /dev/full accepts the buffered write and fails
        //     only at flush time, which is exactly the row-218 shape: -1 even
        //     though every dump callback returned 0.
        let full = cs("/dev/full");
        if std::fs::metadata("/dev/full").is_ok() {
            // json_dumpf alone does NOT fail (stdio buffers the small write) —
            // that isolates the -1 to fclose.
            let cf = fopen(full.as_ptr(), cs("w").as_ptr());
            let rf = fopen(full.as_ptr(), cs("w").as_ptr());
            assert!(!cf.is_null() && !rf.is_null(), "/dev/full exists but is not writable");
            {
                let (cs2, rs2) = pair(c, r, |a| {
                    let arr = (a.json_array)();
                    apush(a, arr, (a.json_integer)(1));
                    arr
                });
                let crc = (c.json_dumpf)(cs2, cf, 0);
                let rrc = (r.json_dumpf)(rs2, rf, 0);
                diff_eq!(crc, rrc, "row218 json_dumpf to /dev/full (buffered)");
                assert_eq!(crc, 0, "C: the buffered write succeeds");
                assert_ne!(fclose(cf), 0, "C: fclose(/dev/full) must fail");
                assert_ne!(fclose(rf), 0, "fclose(/dev/full) must fail");

                let crc = (c.json_dump_file)(cs2, full.as_ptr(), 0);
                let rrc = (r.json_dump_file)(rs2, full.as_ptr(), 0);
                diff_eq!(crc, rrc, "row218 json_dump_file(/dev/full)");
                assert_eq!(
                    crc, -1,
                    "C row 218: fclose failure must give -1 even though the dump succeeded"
                );
                decref(c, cs2);
                decref(r, rs2);
            }
        }

        decref(c, cj);
        decref(r, rj);
        for p in [&cro, &rro, &cp, &rp] {
            let _ = std::fs::remove_file(p);
        }
    }
}

// ===========================================================================
// Row 219 — the JSON_INTEGER snprintf guard is UNREACHABLE
// ===========================================================================

#[test]
fn row_219_integer_snprintf_can_never_overflow_the_buffer() {
    let _g = global_state_lock();
    // ERRORS.md row 219 is marked `[-]`: it cannot be reached in-process.
    //
    //     char buffer[MAX_INTEGER_STR_LENGTH];          /* 25 */
    //     size = snprintf(buffer, MAX_INTEGER_STR_LENGTH, "%" JSON_INTEGER_FORMAT,
    //                     json_integer_value(json));
    //     if (size < 0 || size >= MAX_INTEGER_STR_LENGTH)
    //         return -1;
    //
    // `json_int_t` is `long long` (JSON_INTEGER_IS_LONG_LONG 1), so the widest
    // possible `%lld` rendering is "-9223372036854775808" — 20 characters. 20
    // is always < 25 and glibc's snprintf cannot fail for an integer
    // conversion, so neither disjunct can ever be true. There is no argument
    // that makes this branch fire; the test below therefore proves the
    // invariant (every integer dumps, and always in < 25 bytes) instead of
    // faking the failure.
    let (c, r) = both();
    unsafe {
        let mut ints: Vec<json_int_t> = vec![
            0,
            1,
            -1,
            9,
            -9,
            i32::MAX as i64,
            i32::MIN as i64,
            i64::MAX,
            i64::MIN,
            i64::MAX - 1,
            i64::MIN + 1,
            1_000_000_000_000_000_000,
            -1_000_000_000_000_000_000,
        ];
        // Every decimal width from 1 to 19 digits, both signs.
        let mut p: i64 = 1;
        for _ in 0..18 {
            ints.push(p);
            ints.push(-p);
            ints.push(p - 1);
            ints.push(-(p - 1));
            p *= 10;
        }
        let mut rng = Rng::new(0xA12_0219);
        for _ in 0..400 {
            ints.push(rng.json_int());
        }
        for v in ints {
            let (cv, rv) = pair(c, r, |a| (a.json_integer)(v));
            let cd = dumps(c, cv, JSON_ENCODE_ANY);
            let rd = dumps(r, rv, JSON_ENCODE_ANY);
            diff_eq!(cd.clone(), rd, "json_dumps(json_integer({v}))");
            let bytes = cd.expect("C: every json_int_t must dump");
            assert!(
                bytes.0.len() < 25,
                "C: {v} rendered as {} bytes, which would trip the row-219 guard",
                bytes.0.len()
            );
            decref(c, cv);
            decref(r, rv);
        }
    }
}

// ===========================================================================
// Row 220 — do_dump's `default:` arm (a corrupted json->type)
// ===========================================================================

#[test]
fn row_220_corrupted_type_reaches_the_default_arm() {
    let _g = global_state_lock();
    // `json_typeof(json)` is just `json->type`, an `int` field, so a type
    // outside JSON_OBJECT..JSON_NULL is reachable across the FFI boundary with
    // a hand-built json_t. refcount = (size_t)-1 marks it as a singleton that
    // json_decref/json_delete never free, which is what makes it safe to embed
    // one in a real container.
    let (c, r) = both();
    unsafe {
        for t in [8, 9, 42, 99, 127, 128, 255, 256, 65536, i32::MAX, -1, -2, i32::MIN + 1] {
            let bogus = json_t { type_: t, refcount: usize::MAX };
            let p: *const json_t = &bogus;
            for &flags in &[
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_COMPACT,
                JSON_ENCODE_ANY | json_indent(2),
                JSON_ENCODE_ANY | JSON_SORT_KEYS,
                0,
                JSON_EMBED | JSON_ENCODE_ANY,
            ] {
                let ctx = format!("type={t} flags={flags:#x}");
                diff_eq!(dumps(c, p, flags), dumps(r, p, flags), "json_dumps(bogus) [{ctx}]");
                assert!(
                    dumps(c, p, flags).is_none(),
                    "C row 220: the default: arm must give NULL [{ctx}]"
                );
                let (cn, cbuf) = dumpb64(c, p, 64, flags);
                let (rn, rbuf) = dumpb64(r, p, 64, flags);
                diff_eq!((cn, cbuf), (rn, rbuf), "json_dumpb(bogus) [{ctx}]");
                assert_eq!(cn, 0, "C: 0 [{ctx}]");
                let (crc, cch, _) = run_cb(c, p, flags, Sink::quiet());
                let (rrc, rch, _) = run_cb(r, p, flags, Sink::quiet());
                diff_eq!(crc, rrc, "json_dump_callback(bogus) [{ctx}]");
                diff_eq!(cch.clone(), rch, "json_dump_callback(bogus) chunks [{ctx}]");
                assert_eq!(crc, -1, "C: -1 [{ctx}]");
                assert!(cch.is_empty(), "C: no chunk is emitted for a bogus type [{ctx}]");
                diff_eq!(
                    (c.json_dumpfd)(p, -1, flags),
                    (r.json_dumpfd)(p, -1, flags),
                    "json_dumpfd(bogus) [{ctx}]"
                );
            }

            // Nested: the parent emits "[" and then aborts, so the failure
            // propagates and the partial chunk sequence must match too.
            let mkarr = |a: &Api| -> *mut json_t {
                let arr = (a.json_array)();
                apush(a, arr, (a.json_integer)(1));
                assert_eq!(apush(a, arr, p as *mut json_t), 0);
                apush(a, arr, (a.json_integer)(2));
                arr
            };
            let (cj, rj) = pair(c, r, mkarr);
            for &flags in &[0usize, JSON_COMPACT, json_indent(2)] {
                diff_eq!(
                    dumps(c, cj, flags),
                    dumps(r, rj, flags),
                    "json_dumps([bogus]) type={t} flags={flags:#x}"
                );
                assert!(
                    dumps(c, cj, flags).is_none(),
                    "C: the failure must propagate out of the array (type={t})"
                );
                let (crc, cch, cjoin) = run_cb(c, cj, flags, Sink::quiet());
                let (rrc, rch, rjoin) = run_cb(r, rj, flags, Sink::quiet());
                diff_eq!(crc, rrc, "callback([bogus]) return type={t} flags={flags:#x}");
                diff_eq!(cch, rch, "callback([bogus]) chunks type={t} flags={flags:#x}");
                diff_eq!(cjoin, rjoin, "callback([bogus]) bytes type={t} flags={flags:#x}");
            }
            let (cobj, robj) = pair(c, r, |a| {
                let o = (a.json_object)();
                oset(a, o, b"k", p as *mut json_t);
                o
            });
            for &flags in &[0usize, JSON_SORT_KEYS] {
                diff_eq!(
                    dumps(c, cobj, flags),
                    dumps(r, robj, flags),
                    "json_dumps({{bogus}}) type={t} flags={flags:#x}"
                );
                assert!(
                    dumps(c, cobj, flags).is_none(),
                    "C: the failure must propagate out of the object (type={t})"
                );
            }
            decref(c, cj);
            decref(r, rj);
            decref(c, cobj);
            decref(r, robj);
        }
    }
}

// ===========================================================================
// Rows 350, 351 — the two asserts in the JSON_SORT_KEYS branch
// ===========================================================================

#[test]
fn rows_350_351_sort_keys_asserts_are_unreachable_invariants() {
    let _g = global_state_lock();
    // ERRORS.md rows 350 and 351 are marked `[-]`: both are live `assert()`s
    // (the C is built with an empty CMAKE_BUILD_TYPE, so NDEBUG is absent), and
    // a firing assert aborts the process with SIGABRT instead of returning a
    // value — there is nothing to compare. They also cannot be made to fire,
    // because the sorted branch fills its array straight from the object's own
    // iterator:
    //
    //     i = 0;
    //     while (iter) { keys[i].key = json_object_iter_key(iter); ... i++; }
    //     assert(i == size);                        /* row 350 */
    //     ...
    //     value = json_object_getn(json, key->key, key->len);
    //     assert(value);                            /* row 351 */
    //
    // What CAN be verified through the FFI is the invariant each assert
    // encodes, for the awkward key shapes that would break it if it were
    // breakable: json_object_size() must equal the number of keys the iterator
    // yields, and every key so harvested must be findable with
    // json_object_getn(). Both libraries must agree, key for key.
    let (c, r) = both();
    unsafe {
        let key_sets: Vec<Vec<&[u8]>> = vec![
            vec![b""],
            vec![b"", b"a"],
            vec![b"a", b"ab", b"abc", b"b"],
            vec![b"a\0b", b"a", b"a\0", b"\0"],
            vec![b"\xff", b"\xff\xfe", b"a"],
            vec![b"z", b"y", b"x", b"w", b"v", b"u", b"t", b"s", b"r", b"q", b"p"],
            vec![b"dup", b"dup2", b"dup\0hidden"],
        ];
        for (n, keys) in key_sets.iter().enumerate() {
            let mk = |a: &Api| -> *mut json_t {
                let o = (a.json_object)();
                for (i, k) in keys.iter().enumerate() {
                    oset(a, o, k, (a.json_integer)(i as i64));
                }
                o
            };
            let (cj, rj) = pair(c, r, mk);

            // Row 350's invariant: iterated count == json_object_size().
            let mut ccount = 0usize;
            let mut charvest: Vec<(Vec<u8>, bool)> = Vec::new();
            let mut it = (c.json_object_iter)(cj);
            while !it.is_null() {
                let kp = (c.json_object_iter_key)(it);
                let kl = (c.json_object_iter_key_len)(it);
                let kb = std::slice::from_raw_parts(kp as *const u8, kl).to_vec();
                // Row 351's invariant: the key is findable again.
                let found = !(c.json_object_getn)(cj, kp, kl).is_null();
                charvest.push((kb, found));
                ccount += 1;
                it = (c.json_object_iter_next)(cj, it);
            }
            let mut rcount = 0usize;
            let mut rharvest: Vec<(Vec<u8>, bool)> = Vec::new();
            let mut it = (r.json_object_iter)(rj);
            while !it.is_null() {
                let kp = (r.json_object_iter_key)(it);
                let kl = (r.json_object_iter_key_len)(it);
                let kb = std::slice::from_raw_parts(kp as *const u8, kl).to_vec();
                let found = !(r.json_object_getn)(rj, kp, kl).is_null();
                rharvest.push((kb, found));
                rcount += 1;
                it = (r.json_object_iter_next)(rj, it);
            }
            diff_eq!(ccount, rcount, "key set #{n}: iterated key count");
            diff_eq!(charvest.clone(), rharvest, "key set #{n}: harvested keys and lookups");
            assert_eq!(
                ccount,
                (c.json_object_size)(cj),
                "C row 350's invariant (i == size) must hold for key set #{n}"
            );
            assert!(
                charvest.iter().all(|(_, found)| *found),
                "C row 351's invariant (json_object_getn finds every iterated key) \
                 must hold for key set #{n}"
            );
            // And the sorted dump itself must be byte-identical.
            for &flags in &[JSON_SORT_KEYS, JSON_SORT_KEYS | JSON_COMPACT, JSON_SORT_KEYS | json_indent(2)] {
                diff_eq!(
                    dumps(c, cj, flags),
                    dumps(r, rj, flags),
                    "sorted dump of key set #{n} flags={flags:#x}"
                );
            }
            decref(c, cj);
            decref(r, rj);
        }
    }
}

// ===========================================================================
// Row 354 — the final shrink realloc failing is NOT an error
// ===========================================================================

#[test]
fn row_354_failed_shrink_realloc_returns_the_larger_buffer() {
    let _g = global_state_lock();
    // json_dumps ends with
    //
    //     result = strbuffer_steal_value(&strbuff);
    //     new_result = jsonp_realloc(result, strbuff.size, strbuff.length + 1);
    //     if (new_result) { result = new_result; }
    //
    // so a FAILING shrink must be ignored and the original, larger buffer
    // returned — json_dumps still SUCCEEDS. With every realloc failing, a dump
    // whose output fits in the initial 16-byte strbuffer never needs to grow,
    // which isolates that single trailing shrink.
    let (c, r) = both();
    unsafe {
        let saved = install_hooks(c, r);
        REALLOC_FAILS.store(1, Ordering::SeqCst);

        // Output <= 14 bytes => strbuffer_append_bytes never reallocs
        // (`size >= strbuff->size - strbuff->length` stays false).
        for text in ["[1,2]", "[]", "{}", "[1]", "[true,null]", "{\"a\":1}"] {
            let (cj, rj) = pair(c, r, |a| {
                let mut e = json_error_t::new();
                (a.json_loads)(cs(text).as_ptr(), 0, &mut e)
            });
            assert!(!cj.is_null() && !rj.is_null(), "could not parse {text:?} under the hooks");

            REALLOC_CALLS.store(0, Ordering::SeqCst);
            let cp = (c.json_dumps)(cj, JSON_COMPACT);
            let creallocs = REALLOC_CALLS.load(Ordering::SeqCst);
            let cb = cbytes(cp).map(Pretty);
            jfree(c, cp as *mut c_void);

            REALLOC_CALLS.store(0, Ordering::SeqCst);
            let rp = (r.json_dumps)(rj, JSON_COMPACT);
            let rreallocs = REALLOC_CALLS.load(Ordering::SeqCst);
            let rb = cbytes(rp).map(Pretty);
            jfree(r, rp as *mut c_void);

            diff_eq!(cb.clone(), rb, "row354 json_dumps({text:?}) with every realloc failing");
            diff_eq!(creallocs, rreallocs, "row354 realloc call count for {text:?}");
            assert!(
                cb.is_some(),
                "C row 354: a failed SHRINK must not be an error for {text:?}"
            );
            assert!(
                creallocs >= 1,
                "the trailing shrink realloc was never attempted for {text:?}"
            );
            assert_eq!(
                cb.unwrap().0.len() + 1 <= 16,
                true,
                "{text:?} must fit in the initial 16-byte strbuffer for this test to \
                 isolate the shrink"
            );
            decref(c, cj);
            decref(r, rj);
        }

        // By contrast, a GROWTH realloc failing IS fatal (strbuffer_append_bytes
        // returns -1, ERRORS.md row 292, so json_dump_callback fails, row 199).
        let (cj, rj) = pair(c, r, |a| {
            let arr = (a.json_array)();
            for i in 0..40 {
                apush(a, arr, (a.json_integer)(i));
            }
            arr
        });
        let cd = dumps(c, cj, JSON_COMPACT);
        let rd = dumps(r, rj, JSON_COMPACT);
        diff_eq!(cd.clone(), rd, "row199/292 json_dumps with a failing GROWTH realloc");
        assert!(cd.is_none(), "C: a failed strbuffer growth must give NULL");
        decref(c, cj);
        decref(r, rj);

        restore_hooks(c, r, &saved);
    }
}

// ===========================================================================
// Rows 355, 356 — undefined flag bits are ignored, and the indent field is
// only 5 bits wide
// ===========================================================================

#[test]
fn rows_355_356_undefined_flag_bits_are_ignored_identically() {
    let _g = global_state_lock();
    // `size_t flags` is never validated anywhere in dump.c, so ANY 64-bit word
    // is a legal argument. Only the bits in KNOWN_FLAG_BITS are read, which is
    // exactly row 355 ("silently ignored — there is no flag validation") and
    // row 356 (JSON_INDENT is masked with JSON_MAX_INDENT = 0x1F, so bit 5 is
    // JSON_COMPACT rather than "indent 32").
    let (c, r) = both();
    unsafe {
        let (cm, rm) = pair(c, r, |a| mixed(a));
        let (ca, ra) = pair(c, r, |a| {
            let arr = (a.json_array)();
            apush(a, arr, (a.json_integer)(1));
            apush(a, arr, (a.json_real)(0.1));
            apush(a, arr, (a.json_string)(cs("s/t").as_ptr()));
            arr
        });

        let mut flag_words: Vec<size_t> = vec![
            0,
            0x800,        // JSON_REAL_PRECISION(1) — a *known* bit, kept for contrast
            0x2_0000,     // the first undefined bit
            0x4_0000,
            0x8000_0000,
            0x1_0000_0000,
            usize::MAX,
            usize::MAX - 1,
            usize::MAX ^ JSON_ENCODE_ANY,
            !KNOWN_FLAG_BITS,
        ];
        // Every single bit from 17 upwards, on its own and combined with a
        // known-good flag set.
        for b in 17..64 {
            flag_words.push(1usize << b);
            flag_words.push((1usize << b) | JSON_COMPACT | json_indent(2) | JSON_SORT_KEYS);
        }
        // Random 64-bit words.
        let mut rng = Rng::new(0xA12_0356);
        for _ in 0..300 {
            flag_words.push(rng.next_u64() as size_t);
        }

        for (i, &f) in flag_words.iter().enumerate() {
            for (label, cj, rj) in [("mixed", cm, rm), ("array", ca, ra)] {
                let ctx = format!("#{i} {label} flags={f:#x}");
                let cd = dumps(c, cj, f);
                let rd = dumps(r, rj, f);
                diff_eq!(cd.clone(), rd.clone(), "json_dumps [{ctx}]");

                // Row 355/356: masking off every undefined bit cannot change
                // the result, in either library.
                let masked = f & KNOWN_FLAG_BITS;
                let cmd = dumps(c, cj, masked);
                let rmd = dumps(r, rj, masked);
                diff_eq!(cd.clone(), cmd, "C must ignore the undefined bits of [{ctx}]");
                diff_eq!(rd, rmd, "Rust must ignore the undefined bits of [{ctx}]");

                // The same for the callback: the chunk sequence must match too.
                let (crc, cch, _) = run_cb(c, cj, f, Sink::quiet());
                let (rrc, rch, _) = run_cb(r, rj, f, Sink::quiet());
                diff_eq!(crc, rrc, "json_dump_callback return [{ctx}]");
                diff_eq!(cch.clone(), rch, "json_dump_callback chunks [{ctx}]");
                let (cmrc, cmch, _) = run_cb(c, cj, masked, Sink::quiet());
                diff_eq!(crc, cmrc, "C masked return [{ctx}]");
                diff_eq!(cch, cmch, "C masked chunks [{ctx}]");
            }
        }

        // Row 356 spelled out: bit 5 is JSON_COMPACT, NOT "indent 32", and
        // JSON_INDENT(n) masks n with 0x1F.
        for (label, cj, rj) in [("mixed", cm, rm), ("array", ca, ra)] {
            diff_eq!(dumps(c, cj, 32), dumps(r, rj, 32), "raw flags=32 [{label}]");
            assert_eq!(
                dumps(c, cj, 32),
                dumps(c, cj, JSON_COMPACT),
                "C row 356: raw 32 is JSON_COMPACT, not indent 32 [{label}]"
            );
            for n in 0..=63usize {
                let ctx = format!("{label} JSON_INDENT({n})");
                let cd = dumps(c, cj, json_indent(n));
                let rd = dumps(r, rj, json_indent(n));
                diff_eq!(cd.clone(), rd, "json_dumps [{ctx}]");
                assert_eq!(
                    cd,
                    dumps(c, cj, json_indent(n & 0x1F)),
                    "C row 356: JSON_INDENT(n) is masked with JSON_MAX_INDENT [{ctx}]"
                );
            }
            assert_eq!(
                dumps(c, cj, json_indent(32)),
                dumps(c, cj, 0),
                "C row 356: JSON_INDENT(32) == JSON_INDENT(0) [{label}]"
            );
        }

        decref(c, cm);
        decref(r, rm);
        decref(c, ca);
        decref(r, ra);
    }
}
