//! Differential tests for `memory.c`, `version.c`, `hashtable_seed.c` and
//! `error.c`.
//!
//! Covers CONFIGS.md rows 129-138 and ERRORS.md rows 200-206, 244-251, 257-258.
//!
//! Everything is driven through `dlsym`'d function pointers on the two shared
//! objects, so the Rust side is exercised exactly like an external consumer.
//!
//! ## Serialisation
//!
//! `do_malloc` / `do_realloc` / `do_free` are process-wide, per-library mutable
//! globals. Cargo runs `#[test]` functions on several threads inside ONE
//! process, so a swapped allocator in one test would be visible to every other
//! test. Therefore:
//!
//!   * every test in this file takes the same process-wide `LOCK`;
//!   * ALL allocator-mutating work lives in a single `#[test]`
//!     (`t130_131_202_203_204_custom_allocators`), which captures the original
//!     hooks with `json_get_alloc_funcs2` first and restores them through
//!     `catch_unwind` so even a failing assertion cannot leak a custom
//!     allocator into another test;
//!   * every custom allocator ultimately delegates to libc `malloc`/`realloc`/
//!     `free`, so a block obtained under one hook can always be released under
//!     another.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_void};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// process-wide lock
// ---------------------------------------------------------------------------

fn lock() -> std::sync::MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::new(());
    M.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// libc, for the "defaults are the C library's allocator" check (row 129)
// ---------------------------------------------------------------------------

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// `[malloc, realloc, free]` as this process sees them. Taking the address of
/// an undefined function in the test executable creates a canonical PLT entry,
/// which is exactly the definition both `.so`s resolve their static
/// initialisers against, so the three values are directly comparable.
fn libc_alloc_addrs() -> [*mut c_void; 3] {
    let m: unsafe extern "C" fn(usize) -> *mut c_void = malloc;
    let r: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void = realloc;
    let f: unsafe extern "C" fn(*mut c_void) = free;
    [m as *mut c_void, r as *mut c_void, f as *mut c_void]
}

// ---------------------------------------------------------------------------
// recording allocators (one independent recorder per library)
// ---------------------------------------------------------------------------

/// `(op, size)`. `size` is 0 for `free` (the size is not observable there).
type Ev = (u8, usize);

const OP_MALLOC: u8 = 0;
/// `realloc` with a non-NULL `ptr`.
const OP_REALLOC: u8 = 1;
/// `realloc` with `ptr == NULL`.
const OP_REALLOC0: u8 = 2;
const OP_FREE: u8 = 3;

/// index 0 == the C library, index 1 == the Rust library.
static REC: [Mutex<Vec<Ev>>; 2] = [Mutex::new(Vec::new()), Mutex::new(Vec::new())];

fn rec(i: usize, e: Ev) {
    REC[i].lock().unwrap_or_else(|x| x.into_inner()).push(e);
}

fn take(i: usize) -> Vec<Ev> {
    std::mem::take(&mut *REC[i].lock().unwrap_or_else(|x| x.into_inner()))
}

fn clear_rec() {
    let _ = take(0);
    let _ = take(1);
}

unsafe extern "C" fn c_malloc(n: usize) -> *mut c_void {
    rec(0, (OP_MALLOC, n));
    malloc(n)
}
unsafe extern "C" fn c_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    rec(0, (if p.is_null() { OP_REALLOC0 } else { OP_REALLOC }, n));
    realloc(p, n)
}
unsafe extern "C" fn c_free(p: *mut c_void) {
    rec(0, (OP_FREE, 0));
    free(p)
}

unsafe extern "C" fn r_malloc(n: usize) -> *mut c_void {
    rec(1, (OP_MALLOC, n));
    malloc(n)
}
unsafe extern "C" fn r_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    rec(1, (if p.is_null() { OP_REALLOC0 } else { OP_REALLOC }, n));
    realloc(p, n)
}
unsafe extern "C" fn r_free(p: *mut c_void) {
    rec(1, (OP_FREE, 0));
    free(p)
}

/// An allocator that always fails (ERRORS.md row 204).
unsafe extern "C" fn fail_malloc(_n: usize) -> *mut c_void {
    std::ptr::null_mut()
}
unsafe extern "C" fn fail_realloc(_p: *mut c_void, _n: usize) -> *mut c_void {
    std::ptr::null_mut()
}
unsafe extern "C" fn noop_free(_p: *mut c_void) {}

fn ev_str(e: &Ev) -> String {
    let n = match e.0 {
        OP_MALLOC => "malloc",
        OP_REALLOC => "realloc",
        OP_REALLOC0 => "realloc(NULL)",
        _ => "free",
    };
    if e.0 == OP_FREE {
        "free".to_string()
    } else {
        format!("{}({})", n, e.1)
    }
}

fn seq_str(s: &[Ev]) -> String {
    s.iter().map(ev_str).collect::<Vec<_>>().join(" ")
}

#[track_caller]
fn eq_seq(what: &str, c: &[Ev], r: &[Ev]) {
    if c == r {
        return;
    }
    let n = c.len().min(r.len());
    let mut first = n;
    for i in 0..n {
        if c[i] != r[i] {
            first = i;
            break;
        }
    }
    let lo = first.saturating_sub(4);
    panic!(
        "C vs RUST ALLOCATION-SEQUENCE divergence in {}\n  \
         C   : {} events\n  RUST: {} events\n  first difference at index {}\n  \
         C   [{}..]: {}\n  RUST[{}..]: {}",
        what,
        c.len(),
        r.len(),
        first,
        lo,
        seq_str(&c[lo..c.len().min(first + 8)]),
        lo,
        seq_str(&r[lo..r.len().min(first + 8)]),
    );
}

// ---------------------------------------------------------------------------
// misc helpers
// ---------------------------------------------------------------------------

fn libn(d: &'static Duo, i: usize) -> &'static Lib {
    if i == 0 {
        &d.c
    } else {
        &d.rs
    }
}

unsafe fn get3(l: &Lib) -> [*mut c_void; 3] {
    let mut a: [*mut c_void; 3] = [std::ptr::null_mut(); 3];
    (l.json_get_alloc_funcs2)(&mut a[0], &mut a[1], &mut a[2]);
    a
}

unsafe fn get2(l: &Lib) -> [*mut c_void; 2] {
    let mut a: [*mut c_void; 2] = [std::ptr::null_mut(); 2];
    (l.json_get_alloc_funcs)(&mut a[0], &mut a[1]);
    a
}

unsafe fn set3(l: &Lib, a: [*mut c_void; 3]) {
    (l.json_set_alloc_funcs2)(
        std::mem::transmute::<*mut c_void, Option<unsafe extern "C" fn(usize) -> *mut c_void>>(a[0]),
        std::mem::transmute::<
            *mut c_void,
            Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
        >(a[1]),
        std::mem::transmute::<*mut c_void, Option<unsafe extern "C" fn(*mut c_void)>>(a[2]),
    );
}

fn addrs(a: &[*mut c_void]) -> Vec<usize> {
    a.iter().map(|p| *p as usize).collect()
}

/// A `json_error_t` whose every one of the 252 bytes is `b`, so that any byte
/// the library does *not* write stays distinguishable.
fn filled(b: u8) -> json_error_t {
    let mut e = json_error_t::new();
    unsafe {
        std::ptr::write_bytes(
            &mut e as *mut json_error_t as *mut u8,
            b,
            std::mem::size_of::<json_error_t>(),
        );
    }
    e
}

/// `n` bytes of `A B C ... Z A B ...` — no `%`, so it is also a safe printf
/// format string.
fn pat(n: usize) -> Vec<u8> {
    (0..n).map(|i| b'A' + (i % 26) as u8).collect()
}

fn raw_bytes(p: *const c_char, n: usize) -> Vec<u8> {
    unsafe { std::slice::from_raw_parts(p as *const u8, n) }.to_vec()
}

// ===========================================================================
// CONFIGS row 129 / ERRORS rows 205, 206
// ===========================================================================

#[test]
fn t129_205_206_get_alloc_funcs_defaults_and_null_outparams() {
    let _g = lock();
    let d = duo();
    unsafe {
        let c3 = get3(&d.c);
        let r3 = get3(&d.rs);

        // -- row 129: all three defaults are non-NULL -----------------------
        assert!(
            !c3[0].is_null() && !c3[1].is_null() && !c3[2].is_null(),
            "C json_get_alloc_funcs2 default has a NULL member: {:?}",
            c3
        );
        assert!(
            !r3[0].is_null() && !r3[1].is_null() && !r3[2].is_null(),
            "RUST json_get_alloc_funcs2 default has a NULL member: {:?}",
            r3
        );

        // the two libraries must report the same three functions
        eq(
            "json_get_alloc_funcs2 defaults",
            addrs(&c3),
            addrs(&r3),
        );

        // ... and those must be libc's malloc/realloc/free
        let l = libc_alloc_addrs();
        eq("C default do_malloc == libc malloc", c3[0] as usize, l[0] as usize);
        eq("C default do_realloc == libc realloc", c3[1] as usize, l[1] as usize);
        eq("C default do_free == libc free", c3[2] as usize, l[2] as usize);
        eq("RUST default do_malloc == libc malloc", r3[0] as usize, l[0] as usize);
        eq("RUST default do_realloc == libc realloc", r3[1] as usize, l[1] as usize);
        eq("RUST default do_free == libc free", r3[2] as usize, l[2] as usize);

        // the 2-arg getter must report the same malloc/free
        let c2 = get2(&d.c);
        let r2 = get2(&d.rs);
        eq("json_get_alloc_funcs defaults", addrs(&c2), addrs(&r2));
        eq("C get_alloc_funcs vs _2 malloc", c2[0] as usize, c3[0] as usize);
        eq("C get_alloc_funcs vs _2 free", c2[1] as usize, c3[2] as usize);
        eq("RUST get_alloc_funcs vs _2 malloc", r2[0] as usize, r3[0] as usize);
        eq("RUST get_alloc_funcs vs _2 free", r2[1] as usize, r3[2] as usize);

        // -- ERRORS rows 205 / 206: NULL out-params are no-ops --------------
        let nul = std::ptr::null_mut();
        for l in d.both() {
            (l.json_get_alloc_funcs)(nul, nul);
            (l.json_get_alloc_funcs2)(nul, nul, nul);
        }
        // partial NULLs: each out-param individually
        for i in 0..2usize {
            let lb = libn(d, i);
            let mut m: *mut c_void = 1usize as *mut c_void;
            (lb.json_get_alloc_funcs)(&mut m, nul);
            eq(
                &format!("{} get_alloc_funcs(&m, NULL)", lb.which),
                m as usize,
                if i == 0 { c3[0] } else { r3[0] } as usize,
            );
            let mut f: *mut c_void = 1usize as *mut c_void;
            (lb.json_get_alloc_funcs)(nul, &mut f);
            eq(
                &format!("{} get_alloc_funcs(NULL, &f)", lb.which),
                f as usize,
                if i == 0 { c3[2] } else { r3[2] } as usize,
            );
            let mut rr: *mut c_void = 1usize as *mut c_void;
            (lb.json_get_alloc_funcs2)(nul, &mut rr, nul);
            eq(
                &format!("{} get_alloc_funcs2(NULL, &r, NULL)", lb.which),
                rr as usize,
                if i == 0 { c3[1] } else { r3[1] } as usize,
            );
        }

        // nothing was disturbed
        eq("C hooks after NULL getters", addrs(&get3(&d.c)), addrs(&c3));
        eq("RUST hooks after NULL getters", addrs(&get3(&d.rs)), addrs(&r3));
    }
}

// ===========================================================================
// CONFIGS row 132 / ERRORS rows 200, 201, 204 (the `len + 1 == 0` half)
// ===========================================================================

#[test]
fn t132_200_201_direct_alloc_api() {
    let _g = lock();
    let d = duo();
    unsafe {
        // -- jsonp_malloc ---------------------------------------------------
        // row 200: size 0 -> NULL
        for l in d.both() {
            let p = (l.jsonp_malloc)(0);
            assert!(p.is_null(), "{}: jsonp_malloc(0) must be NULL", l.which);
        }
        for size in [1usize, 16, 4096, 65536] {
            let mut got: [bool; 2] = [false; 2];
            for i in 0..2usize {
                let l = libn(d, i);
                let p = (l.jsonp_malloc)(size) as *mut u8;
                got[i] = !p.is_null();
                assert!(got[i], "{}: jsonp_malloc({}) returned NULL", l.which, size);
                // the whole block must be writable
                for k in 0..size {
                    *p.add(k) = (k & 0xFF) as u8;
                }
                for k in 0..size {
                    assert_eq!(*p.add(k), (k & 0xFF) as u8);
                }
                (l.jsonp_free)(p as *mut c_void);
            }
            eq(&format!("jsonp_malloc({}) non-NULL", size), got[0], got[1]);
        }

        // -- row 201: jsonp_free(NULL) is a no-op ---------------------------
        for l in d.both() {
            (l.jsonp_free)(std::ptr::null_mut());
            (l.jsonp_free)(std::ptr::null_mut());
        }

        // -- jsonp_realloc with the DEFAULT allocator (do_realloc != NULL) --
        for size in [1usize, 16, 4096, 65536] {
            let mut grown: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            let mut shrunk: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            for i in 0..2usize {
                let l = libn(d, i);
                let p = (l.jsonp_malloc)(size) as *mut u8;
                assert!(!p.is_null());
                for k in 0..size {
                    *p.add(k) = ((k as u8) ^ 0x5A).wrapping_add(1);
                }
                // grow
                let big = size * 4 + 7;
                let q = (l.jsonp_realloc)(p as *mut c_void, size, big) as *mut u8;
                assert!(!q.is_null(), "{}: realloc grow returned NULL", l.which);
                grown[i] = raw_bytes(q as *const c_char, size);
                // shrink
                let small = (size / 2).max(1);
                let s = (l.jsonp_realloc)(q as *mut c_void, big, small) as *mut u8;
                assert!(!s.is_null(), "{}: realloc shrink returned NULL", l.which);
                shrunk[i] = raw_bytes(s as *const c_char, small);
                (l.jsonp_free)(s as *mut c_void);
            }
            eq_bytes(&format!("jsonp_realloc grow from {}", size), &grown[0], &grown[1]);
            eq_bytes(&format!("jsonp_realloc shrink from {}", size), &shrunk[0], &shrunk[1]);
            let want: Vec<u8> = (0..size).map(|k| ((k as u8) ^ 0x5A).wrapping_add(1)).collect();
            eq_bytes(
                &format!("jsonp_realloc grow preserved {} bytes", size),
                &want,
                &grown[0],
            );
            eq_bytes(
                &format!("jsonp_realloc shrink preserved bytes (size {})", size),
                &want[..(size / 2).max(1)],
                &shrunk[0],
            );
        }
        // `ptr == NULL` through the *real* realloc
        {
            let mut nn = [false; 2];
            for i in 0..2usize {
                let l = libn(d, i);
                let p = (l.jsonp_realloc)(std::ptr::null_mut(), 0, 64);
                nn[i] = !p.is_null();
                if !p.is_null() {
                    (l.jsonp_free)(p);
                }
                // newSize == 0 through the real realloc: whatever libc does,
                // both libraries must agree.
                let q = (l.jsonp_malloc)(32);
                let z = (l.jsonp_realloc)(q, 32, 0);
                if !z.is_null() {
                    (l.jsonp_free)(z);
                }
            }
            eq("jsonp_realloc(NULL,0,64) non-NULL", nn[0], nn[1]);
        }

        // -- jsonp_strndup --------------------------------------------------
        // (src bytes, len)
        let long100 = pat(100);
        let cases: Vec<(Vec<u8>, usize)> = vec![
            (cbuf(b""), 0),
            (cbuf(b"a"), 0),
            (cbuf(b"a"), 1),
            (cbuf(b"abcdef"), 3),          // len < strlen
            (cbuf(b"abcdef"), 6),
            (cbuf(b"ab\0cd"), 5),          // embedded NUL, len == full buffer
            (cbuf(b"ab\0cd"), 2),
            (cbuf(b"\0\0\0\0"), 4),        // all NULs
            (cbuf(&long100), 100),
            (cbuf(&long100), 99),
            (cbuf(&long100), 1),
        ];
        for (buf, len) in &cases {
            let mut out: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            let mut nn = [false; 2];
            for i in 0..2usize {
                let l = libn(d, i);
                let p = (l.jsonp_strndup)(buf.as_ptr() as *const c_char, *len);
                nn[i] = !p.is_null();
                assert!(nn[i], "{}: jsonp_strndup(.., {}) returned NULL", l.which, len);
                // len bytes of payload plus the NUL the C writes at [len]
                out[i] = raw_bytes(p, *len + 1);
                (l.jsonp_free)(p as *mut c_void);
            }
            let what = format!("jsonp_strndup({:?}, {})", String::from_utf8_lossy(buf), len);
            eq(&format!("{} non-NULL", what), nn[0], nn[1]);
            eq_bytes(&what, &out[0], &out[1]);
            let mut want = buf[..*len].to_vec();
            want.push(0);
            eq_bytes(&format!("{} vs expected", what), &want, &out[0]);
        }

        // row 204 (second half): `len == SIZE_MAX` makes `len + 1` wrap to 0,
        // so `jsonp_malloc(0)` returns NULL and jsonp_strndup bails out before
        // the memcpy. Verified against `memory.c`: `jsonp_malloc(len + 1)` is
        // the very first statement and its NULL result short-circuits.
        {
            let s = cbuf(b"abc");
            let mut nul = [false; 2];
            for i in 0..2usize {
                let l = libn(d, i);
                let p = (l.jsonp_strndup)(s.as_ptr() as *const c_char, usize::MAX);
                nul[i] = p.is_null();
                if !p.is_null() {
                    (l.jsonp_free)(p as *mut c_void);
                }
            }
            eq("jsonp_strndup(.., SIZE_MAX) is NULL", nul[0], nul[1]);
            assert!(nul[0], "C jsonp_strndup(.., SIZE_MAX) must return NULL");
        }
    }
}

// ===========================================================================
// CONFIGS row 133
// ===========================================================================

#[test]
fn t133_version_str() {
    let _g = lock();
    let d = duo();
    unsafe {
        let cp = (d.c.jansson_version_str)();
        let rp = (d.rs.jansson_version_str)();
        assert!(!cp.is_null() && !rp.is_null());
        let cb = cstr_bytes(cp);
        let rb = cstr_bytes(rp);
        eq_bytes("jansson_version_str", &cb, &rb);
        eq_bytes("jansson_version_str == \"2.15.0\"", b"2.15.0", &cb);
        // stable across calls, and the same pointer each time
        eq(
            "jansson_version_str pointer stability (C)",
            cp as usize,
            (d.c.jansson_version_str)() as usize,
        );
        eq(
            "jansson_version_str pointer stability (RUST)",
            rp as usize,
            (d.rs.jansson_version_str)() as usize,
        );
    }
}

// ===========================================================================
// CONFIGS row 134 / ERRORS row 257
// ===========================================================================

#[test]
fn t134_257_version_cmp() {
    let _g = lock();
    let d = duo();
    unsafe {
        let mut fixed: Vec<(c_int, c_int, c_int)> = Vec::new();
        // every sign combination around (2, 15, 0)
        for &ma in &[1, 2, 3] {
            for &mi in &[14, 15, 16] {
                for &mc in &[-1, 0, 1] {
                    fixed.push((ma, mi, mc));
                }
            }
        }
        // exact match, negatives and the int extremes
        fixed.extend_from_slice(&[
            (2, 15, 0),
            (0, 0, 0),
            (-1, -1, -1),
            (-2, -15, 0),
            (i32::MIN, 0, 0),
            (0, i32::MIN, 0),
            (0, 0, i32::MIN),
            (2, i32::MIN, 0),
            (2, 15, i32::MIN),
            (i32::MAX, 0, 0),
            (0, i32::MAX, 0),
            (0, 0, i32::MAX),
            (2, i32::MAX, 0),
            (2, 15, i32::MAX),
            (i32::MIN, i32::MIN, i32::MIN),
            (i32::MAX, i32::MAX, i32::MAX),
            (2, 15, 1),
            (2, 14, 99),
            (2, 16, -99),
            (3, 0, 0),
            (1, 99, 99),
        ]);
        for (a, b, c) in fixed {
            eq(
                &format!("jansson_version_cmp({}, {}, {})", a, b, c),
                (d.c.jansson_version_cmp)(a, b, c),
                (d.rs.jansson_version_cmp)(a, b, c),
            );
        }

        // 4000 seeded random triples: a mix of near-version values and the
        // full int32 range, so the raw (possibly wrapping) difference matters.
        let mut rng = Rng::new(0x0913_4C4D_5001_2345);
        for _ in 0..4000 {
            let pick = |rng: &mut Rng| -> c_int {
                match rng.below(4) {
                    0 => rng.range_i64(-3, 20) as c_int,
                    1 => rng.range_i64(-1000, 1000) as c_int,
                    2 => rng.next_u32() as i32,
                    _ => {
                        let v = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
                        v[rng.below(v.len())]
                    }
                }
            };
            let a = pick(&mut rng);
            let b = pick(&mut rng);
            let c = pick(&mut rng);
            eq(
                &format!("jansson_version_cmp({}, {}, {}) [random]", a, b, c),
                (d.c.jansson_version_cmp)(a, b, c),
                (d.rs.jansson_version_cmp)(a, b, c),
            );
        }
    }
}

// ===========================================================================
// CONFIGS rows 135, 136 / ERRORS row 258
// ===========================================================================

#[test]
fn t135_136_258_object_seed_is_a_noop_once_set() {
    let _g = lock();
    let d = duo();
    unsafe {
        // `duo()` already called json_object_seed(TEST_SEED) on both libraries,
        // so `hashtable_seed != 0` and every further call must do nothing.
        let c0: u32 = d.c.data("hashtable_seed");
        let r0: u32 = d.rs.data("hashtable_seed");
        eq("hashtable_seed (row 136)", c0, r0);
        assert_ne!(c0, 0, "hashtable_seed must already be non-zero");
        eq("hashtable_seed == TEST_SEED", c0, TEST_SEED as u32);

        // reading through the exported data pointer must agree
        let cp: *mut u32 = d.c.data_ptr("hashtable_seed");
        let rp: *mut u32 = d.rs.data_ptr("hashtable_seed");
        assert!(!cp.is_null() && !rp.is_null());
        eq("hashtable_seed via data_ptr", *cp, *rp);
        eq("hashtable_seed via data_ptr == data", *cp, c0);

        // row 135 / 258: further calls are no-ops (seed already set).
        for s in [
            12345usize,
            1,
            0xFFFF_FFFF,
            TEST_SEED,
            usize::MAX,
            0x1_0000_0000,
        ] {
            (d.c.json_object_seed)(s);
            (d.rs.json_object_seed)(s);
            let c1: u32 = d.c.data("hashtable_seed");
            let r1: u32 = d.rs.data("hashtable_seed");
            eq(&format!("hashtable_seed after json_object_seed({})", s), c1, r1);
            eq(
                &format!("hashtable_seed UNCHANGED after json_object_seed({})", s),
                c0,
                c1,
            );
        }

        // A json_object() must not reseed either.
        let co = (d.c.json_object)();
        let ro = (d.rs.json_object)();
        decref(&d.c, co);
        decref(&d.rs, ro);
        let c2: u32 = d.c.data("hashtable_seed");
        let r2: u32 = d.rs.data("hashtable_seed");
        eq("hashtable_seed after json_object()", c2, r2);
        eq("hashtable_seed UNCHANGED after json_object()", c0, c2);
    }
}

// ===========================================================================
// CONFIGS rows 137, 138 / ERRORS row 244 — jsonp_error_init
// ===========================================================================

#[test]
fn t137_138_244_error_init() {
    let _g = lock();
    let d = duo();
    unsafe {
        let sources: Vec<Option<Vec<u8>>> = vec![
            None,
            Some(cbuf(b"")),
            Some(cbuf(b"x")),
            Some(cbuf(b"<string>")),
            Some(cbuf(&pat(79))),
            Some(cbuf(&pat(80))),
            Some(cbuf(&pat(200))),
        ];
        for src in &sources {
            for fill in [0x00u8, 0xAA, 0xFF] {
                let mut ce = filled(fill);
                let mut re = filled(fill);
                let sp: *const c_char = match src {
                    None => std::ptr::null(),
                    Some(v) => v.as_ptr() as *const c_char,
                };
                (d.c.jsonp_error_init)(&mut ce, sp);
                (d.rs.jsonp_error_init)(&mut re, sp);
                let what = format!(
                    "jsonp_error_init(prefill={:#02x}, source={})",
                    fill,
                    match src {
                        None => "NULL".to_string(),
                        Some(v) => format!("{} bytes", v.len() - 1),
                    }
                );
                eq_err(&what, &ce, &re);
                // documented post-conditions
                eq(&format!("{} line", what), ce.line, -1);
                eq(&format!("{} column", what), ce.column, -1);
                eq(&format!("{} position", what), ce.position, 0);
                eq(&format!("{} text[0]", what), ce.text[0] as i32, 0);
                if src.is_none() {
                    eq(&format!("{} source[0]", what), ce.source[0] as i32, 0);
                }
            }
        }

        // ERRORS row 244: error == NULL is a no-op (must not crash)
        let s = cbuf(b"whatever");
        for l in d.both() {
            (l.jsonp_error_init)(std::ptr::null_mut(), s.as_ptr() as *const c_char);
            (l.jsonp_error_init)(std::ptr::null_mut(), std::ptr::null());
        }
    }
}

// ===========================================================================
// CONFIGS rows 137, 138 / ERRORS rows 245, 246, 247 — jsonp_error_set_source
// ===========================================================================

/// The exact 80-byte `source` array `error.c` leaves behind, given a `source`
/// array pre-filled with `fill`.
fn expect_source(src: &[u8], fill: u8) -> Vec<u8> {
    let mut out = vec![fill; JSON_ERROR_SOURCE_LENGTH];
    let length = src.len();
    if length < JSON_ERROR_SOURCE_LENGTH {
        // strncpy(source, src, length + 1) copies exactly length + 1 bytes
        out[..length].copy_from_slice(src);
        out[length] = 0;
    } else {
        let extra = length - JSON_ERROR_SOURCE_LENGTH + 4;
        out[0] = b'.';
        out[1] = b'.';
        out[2] = b'.';
        // strncpy(source + 3, src + extra, length - extra + 1)
        let tail = &src[extra..];
        out[3..3 + tail.len()].copy_from_slice(tail);
        out[3 + tail.len()] = 0;
    }
    out
}

#[test]
fn t137_138_245_246_247_error_set_source() {
    let _g = lock();
    let d = duo();
    unsafe {
        for len in [0usize, 1, 2, 40, 78, 79, 80, 81, 82, 83, 100, 159, 200, 500] {
            let src = pat(len);
            let buf = cbuf(&src);
            for fill in [0x00u8, 0xAA] {
                let mut ce = filled(fill);
                let mut re = filled(fill);
                (d.c.jsonp_error_set_source)(&mut ce, buf.as_ptr() as *const c_char);
                (d.rs.jsonp_error_set_source)(&mut re, buf.as_ptr() as *const c_char);
                let what = format!("jsonp_error_set_source(len={}, prefill={:#02x})", len, fill);
                eq_err(&what, &ce, &re);
                // exact bytes, including the "..." truncation prefix
                let got: Vec<u8> = ce.source.iter().map(|c| *c as u8).collect();
                eq_bytes(&format!("{} source[] bytes", what), &expect_source(&src, fill), &got);
                if len >= JSON_ERROR_SOURCE_LENGTH {
                    assert_eq!(
                        &got[..3],
                        b"...",
                        "{}: expected the \"...\" truncation prefix",
                        what
                    );
                    eq(&format!("{} source_str len", what), ce.source_str().len(), 79);
                }
                // nothing outside source[] may be touched
                eq(&format!("{} line untouched", what), ce.line, filled(fill).line);
                eq(&format!("{} text untouched", what), ce.text[0], filled(fill).text[0]);
            }
        }

        // overwriting an already-populated source
        {
            let a = cbuf(&pat(200));
            let b = cbuf(b"short");
            let mut ce = filled(0xAA);
            let mut re = filled(0xAA);
            for (l, e) in [
                (&d.c, &mut ce as *mut json_error_t),
                (&d.rs, &mut re as *mut json_error_t),
            ] {
                (l.jsonp_error_set_source)(e, a.as_ptr() as *const c_char);
                (l.jsonp_error_set_source)(e, b.as_ptr() as *const c_char);
            }
            eq_err("jsonp_error_set_source overwrite long->short", &ce, &re);
            let got: Vec<u8> = ce.source.iter().map(|c| *c as u8).collect();
            let mut want = expect_source(&pat(200), 0xAA);
            let over = expect_source(b"short", 0xAA);
            want[..6].copy_from_slice(&over[..6]);
            eq_bytes("overwrite long->short source[] bytes", &want, &got);
        }

        // ERRORS rows 245 / 246: error == NULL and source == NULL are no-ops
        {
            let s = cbuf(b"src");
            for l in d.both() {
                (l.jsonp_error_set_source)(std::ptr::null_mut(), s.as_ptr() as *const c_char);
                (l.jsonp_error_set_source)(std::ptr::null_mut(), std::ptr::null());
            }
            let mut ce = filled(0xAA);
            let mut re = filled(0xAA);
            (d.c.jsonp_error_set_source)(&mut ce, std::ptr::null());
            (d.rs.jsonp_error_set_source)(&mut re, std::ptr::null());
            eq_err("jsonp_error_set_source(source = NULL)", &ce, &re);
            eq_bytes(
                "jsonp_error_set_source(source = NULL) leaves the struct alone",
                &filled(0xAA).raw(),
                &ce.raw(),
            );
        }
    }
}

// ===========================================================================
// CONFIGS rows 137, 138 / ERRORS rows 248, 249, 250, 251 — jsonp_error_vset
// ===========================================================================

/// Runs `jsonp_error_vset` on both libraries with freshly built (identical)
/// `va_list`s and byte-compares the two 252-byte structs.
unsafe fn vset_both(
    d: &'static Duo,
    what: &str,
    prefill: u8,
    clear_text: bool,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    fmt: &[u8],
    build: &dyn Fn() -> VaArgs,
) -> json_error_t {
    let mut ce = filled(prefill);
    let mut re = filled(prefill);
    if clear_text {
        ce.text[0] = 0;
        re.text[0] = 0;
    }
    {
        let mut va = build();
        let ap = va.build();
        (d.c.jsonp_error_vset)(
            &mut ce,
            line,
            column,
            position,
            code,
            fmt.as_ptr() as *const c_char,
            ap,
        );
    }
    {
        let mut va = build();
        let ap = va.build();
        (d.rs.jsonp_error_vset)(
            &mut re,
            line,
            column,
            position,
            code,
            fmt.as_ptr() as *const c_char,
            ap,
        );
    }
    eq_err(what, &ce, &re);
    ce
}

#[test]
fn t137_138_248_249_250_251_error_vset() {
    let _g = lock();
    let d = duo();
    let fmt_s = cbuf(b"%s");
    unsafe {
        // ---- message lengths x every code, valid and out of range --------
        let codes: Vec<c_int> = (0..=17)
            .chain([18, 99, 127, 128, 200, 255, 256, 257, -1, -128, -129].into_iter())
            .chain([i32::MIN, i32::MAX].into_iter())
            .collect();
        for &len in &[0usize, 1, 100, 157, 158, 159, 160, 300] {
            let msg = cbuf(&pat(len));
            for &code in &codes {
                let what = format!("jsonp_error_vset(\"%s\", msglen={}, code={})", len, code);
                let ce = vset_both(
                    d,
                    &what,
                    0xAA,
                    true,
                    11,
                    22,
                    33,
                    code,
                    &fmt_s,
                    &|| VaArgs::new().ptr(msg.as_ptr()),
                );
                // documented post-conditions of the C
                eq(&format!("{} line", what), ce.line, 11);
                eq(&format!("{} column", what), ce.column, 22);
                eq(&format!("{} position", what), ce.position, 33);
                // row 250: truncation at JSON_ERROR_TEXT_LENGTH - 2 == 158
                let want_len = len.min(JSON_ERROR_TEXT_LENGTH - 2);
                eq(&format!("{} text len", what), ce.text_str().len(), want_len);
                eq_bytes(
                    &format!("{} text bytes", what),
                    &pat(len)[..want_len],
                    ce.text_str().as_bytes(),
                );
                eq(
                    &format!("{} text[158] == 0", what),
                    ce.text[JSON_ERROR_TEXT_LENGTH - 2] as i32,
                    0,
                );
                // row 251: text[159] holds the raw low byte of `code`
                eq(
                    &format!("{} text[159] raw byte", what),
                    ce.text[JSON_ERROR_TEXT_LENGTH - 1] as u8,
                    code as u32 as u8,
                );
                eq(
                    &format!("{} json_error_code round trip", what),
                    ce.code(),
                    code as u32 as u8 as i8 as c_int,
                );
            }
        }

        // ---- line / column / position edge values -------------------------
        let msg = cbuf(b"boom");
        for &(line, column, position) in &[
            (0i32, 0i32, 0usize),
            (-1, -1, 0),
            (i32::MIN, i32::MAX, 1),
            (1, 2, 0x7FFF_FFFF),
            (1, 2, 0x8000_0000),
            (1, 2, 0xFFFF_FFFF),
            (1, 2, usize::MAX),
            (1, 2, 0x1_2345_6789),
        ] {
            let what = format!(
                "jsonp_error_vset(line={}, column={}, position={:#x})",
                line, column, position
            );
            let ce = vset_both(
                d,
                &what,
                0xAA,
                true,
                line,
                column,
                position,
                json_error_invalid_syntax,
                &fmt_s,
                &|| VaArgs::new().ptr(msg.as_ptr()),
            );
            eq(
                &format!("{} position truncated to int", what),
                ce.position,
                position as u32 as i32,
            );
        }

        // ---- richer formats through the va_list ---------------------------
        {
            let s1 = cbuf(b"alpha");
            let s2 = cbuf(b"beta");
            let f = cbuf(b"a=%s b=%d c=%c d=%x e=%% f=%s g=%ld");
            let what = "jsonp_error_vset(mixed format)";
            let ce = vset_both(d, what, 0xAA, true, 1, 2, 3, 5, &f, &|| {
                VaArgs::new()
                    .ptr(s1.as_ptr())
                    .int(-4242)
                    .int('Z' as c_int)
                    .int(0x1234_ABCD_u32 as c_int)
                    .ptr(s2.as_ptr())
                    .i64(-1234567890123i64)
            });
            eq(
                what,
                ce.text_str(),
                "a=alpha b=-4242 c=Z d=1234abcd e=% f=beta g=-1234567890123".to_string(),
            );
        }
        {
            let f = cbuf(b"%f|%g|%e|%d");
            let what = "jsonp_error_vset(floating point format)";
            let ce = vset_both(d, what, 0xAA, true, 1, 2, 3, 6, &f, &|| {
                VaArgs::new().f64(1.5).f64(0.125).f64(-2.5e-7).int(9)
            });
            eq(
                what,
                ce.text_str(),
                "1.500000|0.125|-2.500000e-07|9".to_string(),
            );
        }
        {
            // no conversions at all: `ap` must not be touched
            let f = cbuf(&pat(50));
            let what = "jsonp_error_vset(no conversions)";
            let ce = vset_both(d, what, 0xAA, true, 1, 2, 3, 7, &f, &VaArgs::new);
            eq_bytes(what, &pat(50), ce.text_str().as_bytes());
        }

        // ---- ERRORS row 249: already set -> no overwrite ------------------
        for prefill in [0xAAu8, 0x01, 0xFF] {
            let what = format!("jsonp_error_vset already set (prefill={:#02x})", prefill);
            let ce = vset_both(
                d,
                &what,
                prefill,
                false, // leave text[0] != 0
                77,
                88,
                99,
                json_error_wrong_type,
                &fmt_s,
                &|| VaArgs::new().ptr(msg.as_ptr()),
            );
            eq_bytes(
                &format!("{} struct untouched", what),
                &filled(prefill).raw(),
                &ce.raw(),
            );
        }
        // and the realistic case: an error struct populated by a first vset
        {
            let first = cbuf(b"first message");
            let second = cbuf(b"second message");
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            for (l, e) in [
                (&d.c, &mut ce as *mut json_error_t),
                (&d.rs, &mut re as *mut json_error_t),
            ] {
                let mut va = VaArgs::new().ptr(first.as_ptr());
                (l.jsonp_error_vset)(e, 1, 2, 3, 4, fmt_s.as_ptr() as *const c_char, va.build());
                let mut va = VaArgs::new().ptr(second.as_ptr());
                (l.jsonp_error_vset)(
                    e,
                    9,
                    9,
                    9,
                    json_error_null_value,
                    fmt_s.as_ptr() as *const c_char,
                    va.build(),
                );
            }
            eq_err("jsonp_error_vset second call does not overwrite", &ce, &re);
            eq("second call kept text", ce.text_str(), "first message".to_string());
            eq("second call kept line", ce.line, 1);
            eq("second call kept code", ce.code(), 4);
        }
        // a zero-length message leaves text[0] == '\0', so the *next* vset is
        // not considered "already set"
        {
            let empty = cbuf(b"");
            let after = cbuf(b"now set");
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            for (l, e) in [
                (&d.c, &mut ce as *mut json_error_t),
                (&d.rs, &mut re as *mut json_error_t),
            ] {
                let mut va = VaArgs::new().ptr(empty.as_ptr());
                (l.jsonp_error_vset)(e, 1, 2, 3, 4, fmt_s.as_ptr() as *const c_char, va.build());
                let mut va = VaArgs::new().ptr(after.as_ptr());
                (l.jsonp_error_vset)(e, 5, 6, 7, 8, fmt_s.as_ptr() as *const c_char, va.build());
            }
            eq_err("jsonp_error_vset after an empty message", &ce, &re);
            eq("empty then set: text", ce.text_str(), "now set".to_string());
            eq("empty then set: code", ce.code(), 8);
        }

        // ---- ERRORS row 248: error == NULL -------------------------------
        for l in d.both() {
            let mut va = VaArgs::new().ptr(msg.as_ptr());
            (l.jsonp_error_vset)(
                std::ptr::null_mut(),
                1,
                2,
                3,
                4,
                fmt_s.as_ptr() as *const c_char,
                va.build(),
            );
        }
    }
}

// ===========================================================================
// CONFIGS rows 137, 138 / ERRORS row 248 — jsonp_error_set (variadic)
//
// The Rust side of this one is hand-written naked assembly (`src/va.rs`), so it
// can only be exercised through the `.so`.
// ===========================================================================

macro_rules! set_case {
    ($d:expr, $what:expr, $code:expr, $fmt:expr $(, $arg:expr)* ) => {{
        let mut ce = filled(0xAA);
        let mut re = filled(0xAA);
        ce.text[0] = 0;
        re.text[0] = 0;
        (($d).c.jsonp_error_set)(
            &mut ce, 13, 26, 39, $code, $fmt.as_ptr() as *const c_char $(, $arg)*);
        (($d).rs.jsonp_error_set)(
            &mut re, 13, 26, 39, $code, $fmt.as_ptr() as *const c_char $(, $arg)*);
        eq_err($what, &ce, &re);
        eq(concat!("line of ", $what), ce.line, 13);
        eq(concat!("column of ", $what), ce.column, 26);
        eq(concat!("position of ", $what), ce.position, 39);
        eq(concat!("code of ", $what), ce.code(), $code as u32 as u8 as i8 as c_int);
        ce
    }};
}

#[test]
fn t137_138_248_error_set_variadic() {
    let _g = lock();
    let d = duo();
    unsafe {
        let s = cbuf(b"hello");
        let t = cbuf(b"world");

        let f = cbuf(b"%s");
        let e = set_case!(d, "jsonp_error_set(\"%s\")", 8, f, s.as_ptr());
        eq("text of %s", e.text_str(), "hello".to_string());

        let f = cbuf(b"%d");
        let e = set_case!(d, "jsonp_error_set(\"%d\")", 9, f, -12345 as c_int);
        eq("text of %d", e.text_str(), "-12345".to_string());

        let f = cbuf(b"100%% done");
        let e = set_case!(d, "jsonp_error_set(\"%%\")", 10, f);
        eq("text of %%", e.text_str(), "100% done".to_string());

        let f = cbuf(b"[%c]");
        let e = set_case!(d, "jsonp_error_set(\"%c\")", 11, f, 'Q' as c_int);
        eq("text of %c", e.text_str(), "[Q]".to_string());

        let f = cbuf(b"%x/%X/%#x");
        let e = set_case!(
            d,
            "jsonp_error_set(\"%x\")",
            12,
            f,
            0xdead_beefu32 as c_int,
            0xdead_beefu32 as c_int,
            0xcafeu32 as c_int
        );
        eq("text of %x", e.text_str(), "deadbeef/DEADBEEF/0xcafe".to_string());

        // several args of mixed kinds; all six named params already fill
        // rdi..r9, so every variadic argument lives in the overflow area
        let f = cbuf(b"a=%s b=%d c=%c d=%x e=%% f=%s g=%u h=%ld");
        let e = set_case!(
            d,
            "jsonp_error_set(mixed)",
            13,
            f,
            s.as_ptr(),
            -7 as c_int,
            '!' as c_int,
            0x1234 as c_int,
            t.as_ptr(),
            4294967295u32 as c_int,
            -9007199254740993i64
        );
        eq(
            "text of mixed",
            e.text_str(),
            "a=hello b=-7 c=! d=1234 e=% f=world g=4294967295 h=-9007199254740993".to_string(),
        );

        // eight pointer arguments, deep into the overflow area
        let f = cbuf(b"%s%s%s%s%s%s%s%s");
        let e = set_case!(
            d,
            "jsonp_error_set(8 strings)",
            14,
            f,
            s.as_ptr(),
            t.as_ptr(),
            s.as_ptr(),
            t.as_ptr(),
            s.as_ptr(),
            t.as_ptr(),
            s.as_ptr(),
            t.as_ptr()
        );
        eq(
            "text of 8 strings",
            e.text_str(),
            "helloworldhelloworldhelloworldhelloworld".to_string(),
        );

        // floating point arguments (al != 0 -> the xmm save area is used)
        let f = cbuf(b"%f|%g|%e|%d|%s");
        let e = set_case!(
            d,
            "jsonp_error_set(floats)",
            15,
            f,
            1.5f64,
            0.125f64,
            -2.5e-7f64,
            42 as c_int,
            s.as_ptr()
        );
        eq(
            "text of floats",
            e.text_str(),
            "1.500000|0.125|-2.500000e-07|42|hello".to_string(),
        );

        // truncation through the variadic path (3 x 60 chars = 180 > 158)
        let big = cbuf(&pat(60));
        let f = cbuf(b"%s%s%s");
        let e = set_case!(
            d,
            "jsonp_error_set(truncated)",
            16,
            f,
            big.as_ptr(),
            big.as_ptr(),
            big.as_ptr()
        );
        eq("truncated text len", e.text_str().len(), 158);
        let mut want = pat(60);
        want.extend_from_slice(&pat(60));
        want.extend_from_slice(&pat(60));
        eq_bytes("truncated text bytes", &want[..158], e.text_str().as_bytes());

        // out-of-range codes through the variadic wrapper
        for code in [-1i32, 18, 200, 255, i32::MIN, i32::MAX] {
            let f = cbuf(b"code %d");
            let mut ce = filled(0xAA);
            let mut re = filled(0xAA);
            ce.text[0] = 0;
            re.text[0] = 0;
            (d.c.jsonp_error_set)(&mut ce, 1, 2, 3, code, f.as_ptr() as *const c_char, code);
            (d.rs.jsonp_error_set)(&mut re, 1, 2, 3, code, f.as_ptr() as *const c_char, code);
            eq_err(&format!("jsonp_error_set(code={})", code), &ce, &re);
            eq(
                &format!("jsonp_error_set(code={}) raw byte", code),
                ce.text[JSON_ERROR_TEXT_LENGTH - 1] as u8,
                code as u32 as u8,
            );
        }

        // already set -> no overwrite, through the variadic wrapper
        {
            let f = cbuf(b"%s");
            let mut ce = filled(0xAA);
            let mut re = filled(0xAA);
            for (l, p) in [
                (&d.c, &mut ce as *mut json_error_t),
                (&d.rs, &mut re as *mut json_error_t),
            ] {
                (*p).text[0] = 0;
                (l.jsonp_error_set)(p, 1, 2, 3, 4, f.as_ptr() as *const c_char, s.as_ptr());
                (l.jsonp_error_set)(p, 5, 6, 7, 8, f.as_ptr() as *const c_char, t.as_ptr());
            }
            eq_err("jsonp_error_set already set", &ce, &re);
            eq("jsonp_error_set kept text", ce.text_str(), "hello".to_string());
            eq("jsonp_error_set kept line", ce.line, 1);
            eq("jsonp_error_set kept code", ce.code(), 4);
        }

        // ERRORS row 248: error == NULL through the variadic wrapper
        {
            let f = cbuf(b"%s %d");
            for l in d.both() {
                (l.jsonp_error_set)(
                    std::ptr::null_mut(),
                    1,
                    2,
                    3,
                    4,
                    f.as_ptr() as *const c_char,
                    s.as_ptr(),
                    7 as c_int,
                );
            }
        }
    }
}

// ===========================================================================
// CONFIGS rows 130, 131 / ERRORS rows 202, 203, 204
//
// THE ONLY test in this file that mutates the allocator hooks.
// ===========================================================================

const DOC: &str = concat!(
    r#"{"name":"jansson","version":[2,15,0],"#,
    r#""nested":{"a":true,"b":false,"c":null,"d":"xéy"},"#,
    r#""nums":[0,-1,3.25,1e10,-2.5e-5,1234567890123456789],"#,
    r#""long":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","#,
    r#""deep":[[[[1,2],[3,4]],[[5,6]]],{"z":{"y":{"x":[]}}}]}"#,
);

/// `json_loads` -> `json_dumps` -> `json_deep_copy` -> `json_dumps(SORT_KEYS)`.
unsafe fn wl_load_dump_copy(l: &Lib) -> String {
    let src = cs(DOC);
    let j = (l.json_loads)(src.as_ptr(), 0, std::ptr::null_mut());
    assert!(!j.is_null(), "{}: json_loads(DOC) returned NULL", l.which);
    let s1 = (l.json_dumps)(j, 0);
    assert!(!s1.is_null(), "{}: json_dumps returned NULL", l.which);
    let cp = (l.json_deep_copy)(j);
    assert!(!cp.is_null(), "{}: json_deep_copy returned NULL", l.which);
    let s2 = (l.json_dumps)(cp, JSON_SORT_KEYS | json_indent(2) | JSON_ENSURE_ASCII);
    assert!(!s2.is_null(), "{}: json_dumps(SORT_KEYS) returned NULL", l.which);
    let out = format!(
        "{}\n---\n{}\n---\n{}",
        String::from_utf8_lossy(&cstr_bytes(s1)),
        String::from_utf8_lossy(&cstr_bytes(s2)),
        describe(l, cp),
    );
    (l.jsonp_free)(s1 as *mut c_void);
    (l.jsonp_free)(s2 as *mut c_void);
    decref(l, cp);
    decref(l, j);
    out
}

/// 100-key object (forces several hashtable rehashes) + dump + reload.
unsafe fn wl_object_100(l: &Lib) -> String {
    let o = (l.json_object)();
    assert!(!o.is_null());
    for i in 0..100usize {
        let k = cs(&format!("key{:04}", i * 7 % 100));
        let v = (l.json_integer)(i as i64);
        assert_eq!((l.json_object_set_new)(o, k.as_ptr(), v), 0);
    }
    let s = (l.json_dumps)(o, JSON_SORT_KEYS);
    assert!(!s.is_null());
    let bytes = cstr_bytes(s);
    let back = (l.json_loads)(s as *const c_char, 0, std::ptr::null_mut());
    assert!(!back.is_null());
    let out = format!(
        "{}\n---\n{}\n---\n{}",
        String::from_utf8_lossy(&bytes),
        describe(l, o),
        describe(l, back),
    );
    (l.jsonp_free)(s as *mut c_void);
    decref(l, back);
    decref(l, o);
    out
}

/// 1000-element array (forces repeated `json_array` regrowth).
unsafe fn wl_array_1000(l: &Lib) -> String {
    let a = (l.json_array)();
    assert!(!a.is_null());
    for i in 0..1000usize {
        let v = (l.json_integer)(i as i64 - 500);
        assert_eq!((l.json_array_append_new)(a, v), 0);
    }
    let s = (l.json_dumps)(a, JSON_COMPACT);
    assert!(!s.is_null());
    let n = cstr_bytes(s).len();
    (l.jsonp_free)(s as *mut c_void);
    // shrink it back down again
    for _ in 0..500usize {
        assert_eq!((l.json_array_remove)(a, 0), 0);
    }
    let out = format!("{}|{}", n, (l.json_array_size)(a));
    decref(l, a);
    out
}

/// `json_dumps` with `JSON_SORT_KEYS` on a freshly loaded document.
unsafe fn wl_sort_keys(l: &Lib) -> String {
    let src = cs(DOC);
    let j = (l.json_loads)(src.as_ptr(), 0, std::ptr::null_mut());
    assert!(!j.is_null());
    let mut out = String::new();
    for flags in [
        JSON_SORT_KEYS,
        JSON_SORT_KEYS | JSON_COMPACT,
        JSON_SORT_KEYS | json_indent(4),
        JSON_SORT_KEYS | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
    ] {
        let s = (l.json_dumps)(j, flags);
        assert!(!s.is_null());
        out.push_str(&String::from_utf8_lossy(&cstr_bytes(s)));
        out.push('\n');
        (l.jsonp_free)(s as *mut c_void);
    }
    decref(l, j);
    out
}

/// Runs `f` on both libraries with the recorders cleared, then compares both
/// the observable result and the recorded allocation sequence.
unsafe fn compare_workload(d: &'static Duo, what: &str, f: unsafe fn(&Lib) -> String) {
    clear_rec();
    let mut res: [String; 2] = [String::new(), String::new()];
    let mut seq: [Vec<Ev>; 2] = [Vec::new(), Vec::new()];
    for i in 0..2usize {
        res[i] = f(libn(d, i));
        seq[i] = take(i);
    }
    eq(&format!("{} result", what), res[0].clone(), res[1].clone());
    assert!(
        seq[0].len() > 10,
        "{}: only {} allocation events recorded — the hook was not installed?",
        what,
        seq[0].len()
    );
    eq_seq(&format!("{} allocation sequence", what), &seq[0], &seq[1]);
}

#[test]
fn t130_131_202_203_204_custom_allocators() {
    let _g = lock();
    let d = duo();
    let saved_c = unsafe { get3(&d.c) };
    let saved_r = unsafe { get3(&d.rs) };
    assert!(
        !saved_c[0].is_null() && !saved_r[0].is_null(),
        "refusing to run: the captured default allocators look wrong"
    );

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        allocator_body(d)
    }));

    // ALWAYS put the defaults back, whatever happened.
    unsafe {
        set3(&d.c, saved_c);
        set3(&d.rs, saved_r);
    }
    unsafe {
        eq("restored C allocators", addrs(&get3(&d.c)), addrs(&saved_c));
        eq("restored RUST allocators", addrs(&get3(&d.rs)), addrs(&saved_r));
    }
    clear_rec();

    if let Err(p) = outcome {
        std::panic::resume_unwind(p);
    }
}

unsafe fn allocator_body(d: &'static Duo) {
    // =====================================================================
    // CONFIGS row 130: json_set_alloc_funcs() nulls do_realloc
    // =====================================================================
    (d.c.json_set_alloc_funcs)(Some(c_malloc), Some(c_free));
    (d.rs.json_set_alloc_funcs)(Some(r_malloc), Some(r_free));

    let c3 = get3(&d.c);
    let r3 = get3(&d.rs);
    assert!(
        c3[1].is_null(),
        "C: json_set_alloc_funcs must leave do_realloc NULL, got {:?}",
        c3[1]
    );
    assert!(
        r3[1].is_null(),
        "RUST: json_set_alloc_funcs must leave do_realloc NULL, got {:?}",
        r3[1]
    );
    eq(
        "do_realloc NULL-ness after json_set_alloc_funcs",
        c3[1].is_null(),
        r3[1].is_null(),
    );
    eq(
        "C reports the installed malloc",
        c3[0] as usize,
        c_malloc as unsafe extern "C" fn(usize) -> *mut c_void as usize,
    );
    eq(
        "C reports the installed free",
        c3[2] as usize,
        c_free as unsafe extern "C" fn(*mut c_void) as usize,
    );
    eq(
        "RUST reports the installed malloc",
        r3[0] as usize,
        r_malloc as unsafe extern "C" fn(usize) -> *mut c_void as usize,
    );
    eq(
        "RUST reports the installed free",
        r3[2] as usize,
        r_free as unsafe extern "C" fn(*mut c_void) as usize,
    );
    // the 2-arg getter agrees
    let c2 = get2(&d.c);
    let r2 = get2(&d.rs);
    eq("C get_alloc_funcs malloc", c2[0] as usize, c3[0] as usize);
    eq("C get_alloc_funcs free", c2[1] as usize, c3[2] as usize);
    eq("RUST get_alloc_funcs malloc", r2[0] as usize, r3[0] as usize);
    eq("RUST get_alloc_funcs free", r2[1] as usize, r3[2] as usize);

    // ---- the realloc EMULATION path (rows 130, 202, 203) -----------------
    let mut seq: [Vec<Ev>; 2] = [Vec::new(), Vec::new()];
    let mut grown: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    let mut shrunk: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    let mut fresh: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    clear_rec();
    for i in 0..2usize {
        let l = libn(d, i);
        let _ = take(i);

        // 1. malloc 16 and fill with a recognisable pattern
        let p = (l.jsonp_malloc)(16) as *mut u8;
        assert!(!p.is_null());
        for k in 0..16usize {
            *p.add(k) = 0x30 + k as u8;
        }
        // 2. grow: malloc(48) + memcpy(16) + free
        let q = (l.jsonp_realloc)(p as *mut c_void, 16, 48) as *mut u8;
        assert!(!q.is_null(), "{}: emulated realloc grow returned NULL", l.which);
        grown[i] = raw_bytes(q as *const c_char, 16);
        // 3. shrink: malloc(8) + memcpy(min(48,8) == 8) + free
        let s = (l.jsonp_realloc)(q as *mut c_void, 48, 8) as *mut u8;
        assert!(!s.is_null(), "{}: emulated realloc shrink returned NULL", l.which);
        shrunk[i] = raw_bytes(s as *const c_char, 8);
        // 4. row 202: newSize == 0 with a non-NULL ptr -> free, return NULL
        let z = (l.jsonp_realloc)(s as *mut c_void, 8, 0);
        assert!(
            z.is_null(),
            "{}: emulated realloc to size 0 must return NULL",
            l.which
        );
        // 5. row 203: ptr == NULL, newSize > 0 -> a fresh block, no copy/free
        let n = (l.jsonp_realloc)(std::ptr::null_mut(), 12345, 64) as *mut u8;
        assert!(!n.is_null(), "{}: emulated realloc(NULL, .., 64) NULL", l.which);
        for k in 0..64usize {
            *n.add(k) = 0x77;
        }
        fresh[i] = raw_bytes(n as *const c_char, 64);
        (l.jsonp_free)(n as *mut c_void);
        // 6. ptr == NULL and newSize == 0 -> NULL, no allocator call at all
        let z2 = (l.jsonp_realloc)(std::ptr::null_mut(), 0, 0);
        assert!(z2.is_null());

        seq[i] = take(i);
    }
    eq_bytes("emulated realloc grow contents", &grown[0], &grown[1]);
    eq_bytes("emulated realloc shrink contents", &shrunk[0], &shrunk[1]);
    eq_bytes("emulated realloc fresh block", &fresh[0], &fresh[1]);
    let want_bytes: Vec<u8> = (0..16u8).map(|k| 0x30 + k).collect();
    eq_bytes("emulated realloc grow preserved 16 bytes", &want_bytes, &grown[0]);
    eq_bytes(
        "emulated realloc shrink preserved min(old,new) bytes",
        &want_bytes[..8],
        &shrunk[0],
    );
    eq_seq("realloc emulation", &seq[0], &seq[1]);
    let want_seq: Vec<Ev> = vec![
        (OP_MALLOC, 16),
        (OP_MALLOC, 48),
        (OP_FREE, 0),
        (OP_MALLOC, 8),
        (OP_FREE, 0),
        (OP_FREE, 0),
        (OP_MALLOC, 64),
        (OP_FREE, 0),
    ];
    eq(
        "realloc emulation call sequence (C, vs memory.c)",
        seq_str(&want_seq),
        seq_str(&seq[0]),
    );

    // ---- a real workload through the emulation path (row 130) ------------
    compare_workload(d, "emulation: 100-key object", wl_object_100);
    compare_workload(d, "emulation: load/dump/deep_copy", wl_load_dump_copy);

    // =====================================================================
    // CONFIGS row 131: counting allocator via json_set_alloc_funcs2
    // =====================================================================
    (d.c.json_set_alloc_funcs2)(Some(c_malloc), Some(c_realloc), Some(c_free));
    (d.rs.json_set_alloc_funcs2)(Some(r_malloc), Some(r_realloc), Some(r_free));
    let c3 = get3(&d.c);
    let r3 = get3(&d.rs);
    assert!(!c3[1].is_null() && !r3[1].is_null(), "do_realloc must be set now");
    eq(
        "C reports the installed realloc",
        c3[1] as usize,
        c_realloc as unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void as usize,
    );
    eq(
        "RUST reports the installed realloc",
        r3[1] as usize,
        r_realloc as unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void as usize,
    );

    compare_workload(d, "counting: load/dump/deep_copy", wl_load_dump_copy);
    compare_workload(d, "counting: 100-key object", wl_object_100);
    compare_workload(d, "counting: 1000-element array", wl_array_1000);
    compare_workload(d, "counting: dumps with JSON_SORT_KEYS", wl_sort_keys);

    // =====================================================================
    // ERRORS row 204: the allocator fails
    // =====================================================================
    (d.c.json_set_alloc_funcs2)(Some(fail_malloc), Some(fail_realloc), Some(noop_free));
    (d.rs.json_set_alloc_funcs2)(Some(fail_malloc), Some(fail_realloc), Some(noop_free));
    {
        let s = cbuf(b"abcdef");
        let mut got: [Vec<bool>; 2] = [Vec::new(), Vec::new()];
        for i in 0..2usize {
            let l = libn(d, i);
            got[i].push((l.jsonp_malloc)(16).is_null());
            got[i].push((l.jsonp_malloc)(0).is_null());
            got[i].push((l.jsonp_strndup)(s.as_ptr() as *const c_char, 0).is_null());
            got[i].push((l.jsonp_strndup)(s.as_ptr() as *const c_char, 3).is_null());
            got[i].push((l.jsonp_strndup)(s.as_ptr() as *const c_char, 6).is_null());
            got[i].push((l.jsonp_strndup)(s.as_ptr() as *const c_char, usize::MAX).is_null());
            got[i].push((l.jsonp_realloc)(std::ptr::null_mut(), 0, 16).is_null());
            (l.jsonp_free)(std::ptr::null_mut());
        }
        eq("failing allocator: NULL-ness vector", got[0].clone(), got[1].clone());
        assert!(
            got[0].iter().all(|b| *b),
            "C with a failing allocator returned a non-NULL somewhere: {:?}",
            got[0]
        );
    }

    // and once more with do_realloc == NULL, so the emulation path has to cope
    // with do_malloc failing
    (d.c.json_set_alloc_funcs)(Some(fail_malloc), Some(noop_free));
    (d.rs.json_set_alloc_funcs)(Some(fail_malloc), Some(noop_free));
    {
        let mut got: [Vec<bool>; 2] = [Vec::new(), Vec::new()];
        for i in 0..2usize {
            let l = libn(d, i);
            got[i].push(get3(l)[1].is_null());
            got[i].push((l.jsonp_realloc)(std::ptr::null_mut(), 0, 16).is_null());
            got[i].push((l.jsonp_realloc)(std::ptr::null_mut(), 0, 0).is_null());
            got[i].push((l.jsonp_malloc)(64).is_null());
        }
        eq(
            "failing allocator + emulation: NULL-ness vector",
            got[0].clone(),
            got[1].clone(),
        );
        assert!(got[0].iter().all(|b| *b), "C: {:?}", got[0]);
    }
}
