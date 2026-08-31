//! Differential tests for src/strbuffer.c, src/memory.c and src/error.c.
//!
//! These are the lowest layer above libc, so they are driven directly through
//! their exported symbols rather than through any convenience wrapper.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// strbuffer.c
// ===========================================================================

/// Snapshot everything observable about a strbuffer: the struct fields plus
/// the bytes actually held. `size` is included because the growth policy
/// (STRBUFFER_MIN_SIZE 16, STRBUFFER_FACTOR 2, and the
/// `max(size*2, length+size+1)` rule) is part of the behaviour.
unsafe fn sb_snapshot(api: &Api, sb: &strbuffer_t) -> (size_t, size_t, bool, Option<Vec<u8>>) {
    let val = (api.strbuffer_value)(sb);
    (sb.length, sb.size, sb.value.is_null(), cbytes(val))
}

#[test]
fn strbuffer_init_close_clear() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        diff_eq!(
            (c.strbuffer_init)(&mut csb),
            (r.strbuffer_init)(&mut rsb),
            "strbuffer_init return"
        );
        // Fresh buffer: length 0, size STRBUFFER_MIN_SIZE, value == ""
        diff_eq!(sb_snapshot(c, &csb), sb_snapshot(r, &rsb), "after init");

        (c.strbuffer_append_bytes)(&mut csb, cs("hello").as_ptr(), 5);
        (r.strbuffer_append_bytes)(&mut rsb, cs("hello").as_ptr(), 5);
        diff_eq!(sb_snapshot(c, &csb), sb_snapshot(r, &rsb), "after append");

        (c.strbuffer_clear)(&mut csb);
        (r.strbuffer_clear)(&mut rsb);
        diff_eq!(sb_snapshot(c, &csb), sb_snapshot(r, &rsb), "after clear");

        (c.strbuffer_close)(&mut csb);
        (r.strbuffer_close)(&mut rsb);
        // close() must zero size/length and NULL the pointer.
        diff_eq!(
            (csb.length, csb.size, csb.value.is_null()),
            (rsb.length, rsb.size, rsb.value.is_null()),
            "after close"
        );
    }
}

#[test]
fn strbuffer_growth_boundary_exact() {
    let _g = global_state_lock();
    // STRBUFFER_MIN_SIZE is 16 and the grow test is `size >= strbuff->size -
    // strbuff->length`, so appending 15 bytes must NOT grow while 16 must.
    let (c, r) = both();
    for n in 0usize..=64 {
        unsafe {
            let mut csb = strbuffer_t::zeroed();
            let mut rsb = strbuffer_t::zeroed();
            (c.strbuffer_init)(&mut csb);
            (r.strbuffer_init)(&mut rsb);
            let data: Vec<c_char> = (0..n).map(|i| b'a'.wrapping_add(i as u8) as c_char).collect();
            let cret = (c.strbuffer_append_bytes)(&mut csb, data.as_ptr(), n);
            let rret = (r.strbuffer_append_bytes)(&mut rsb, data.as_ptr(), n);
            diff_eq!(cret, rret, "append_bytes({n}) return");
            diff_eq!(sb_snapshot(c, &csb), sb_snapshot(r, &rsb), "append_bytes({n})");
            (c.strbuffer_close)(&mut csb);
            (r.strbuffer_close)(&mut rsb);
        }
    }
}

#[test]
fn strbuffer_append_zero_bytes() {
    let _g = global_state_lock();
    // size 0 still takes the grow test (`0 >= 16 - 0` is false) and still
    // writes the terminator.
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        (c.strbuffer_init)(&mut csb);
        (r.strbuffer_init)(&mut rsb);
        for _ in 0..5 {
            diff_eq!(
                (c.strbuffer_append_bytes)(&mut csb, cs("").as_ptr(), 0),
                (r.strbuffer_append_bytes)(&mut rsb, cs("").as_ptr(), 0),
                "append 0 bytes"
            );
            diff_eq!(sb_snapshot(c, &csb), sb_snapshot(r, &rsb), "append 0 bytes state");
        }
        (c.strbuffer_close)(&mut csb);
        (r.strbuffer_close)(&mut rsb);
    }
}

#[test]
fn strbuffer_large_append_uses_length_plus_size_rule() {
    let _g = global_state_lock();
    // A single append much larger than size*2 must select the
    // `length + size + 1` branch of max(), giving an exact-fit allocation.
    let (c, r) = both();
    for n in [17usize, 31, 32, 33, 100, 1000, 4096, 65537] {
        unsafe {
            let mut csb = strbuffer_t::zeroed();
            let mut rsb = strbuffer_t::zeroed();
            (c.strbuffer_init)(&mut csb);
            (r.strbuffer_init)(&mut rsb);
            let data = vec![b'z' as c_char; n];
            diff_eq!(
                (c.strbuffer_append_bytes)(&mut csb, data.as_ptr(), n),
                (r.strbuffer_append_bytes)(&mut rsb, data.as_ptr(), n),
                "large append({n}) return"
            );
            diff_eq!(sb_snapshot(c, &csb), sb_snapshot(r, &rsb), "large append({n})");
            (c.strbuffer_close)(&mut csb);
            (r.strbuffer_close)(&mut rsb);
        }
    }
}

#[test]
fn strbuffer_randomised_operation_sequences() {
    let _g = global_state_lock();
    // Property-style: drive both buffers through the SAME random sequence of
    // append_byte / append_bytes / pop / clear and compare the full state after
    // every single step, so a divergence is caught at the operation that
    // caused it rather than at the end.
    let (c, r) = both();
    let mut rng = Rng::new(0x5B_0001);

    for trial in 0..400 {
        unsafe {
            let mut csb = strbuffer_t::zeroed();
            let mut rsb = strbuffer_t::zeroed();
            (c.strbuffer_init)(&mut csb);
            (r.strbuffer_init)(&mut rsb);

            for step in 0..60 {
                match rng.below(10) {
                    0..=3 => {
                        let b = rng.next_u32() as u8 as c_char;
                        diff_eq!(
                            (c.strbuffer_append_byte)(&mut csb, b),
                            (r.strbuffer_append_byte)(&mut rsb, b),
                            "trial {trial} step {step}: append_byte return"
                        );
                    }
                    4..=6 => {
                        let n = rng.below(40);
                        let data: Vec<c_char> =
                            (0..n).map(|_| rng.next_u32() as u8 as c_char).collect();
                        diff_eq!(
                            (c.strbuffer_append_bytes)(&mut csb, data.as_ptr(), n),
                            (r.strbuffer_append_bytes)(&mut rsb, data.as_ptr(), n),
                            "trial {trial} step {step}: append_bytes({n}) return"
                        );
                    }
                    7 | 8 => {
                        diff_eq!(
                            (c.strbuffer_pop)(&mut csb),
                            (r.strbuffer_pop)(&mut rsb),
                            "trial {trial} step {step}: pop return"
                        );
                    }
                    _ => {
                        (c.strbuffer_clear)(&mut csb);
                        (r.strbuffer_clear)(&mut rsb);
                    }
                }
                diff_eq!(
                    sb_snapshot(c, &csb),
                    sb_snapshot(r, &rsb),
                    "trial {trial} step {step}: state"
                );
            }

            (c.strbuffer_close)(&mut csb);
            (r.strbuffer_close)(&mut rsb);
        }
    }
}

#[test]
fn strbuffer_pop_from_empty_returns_nul() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        (c.strbuffer_init)(&mut csb);
        (r.strbuffer_init)(&mut rsb);
        // Popping an empty buffer must yield '\0' and leave length at 0
        // (in particular it must NOT underflow to SIZE_MAX).
        for _ in 0..4 {
            diff_eq!(
                (c.strbuffer_pop)(&mut csb),
                (r.strbuffer_pop)(&mut rsb),
                "pop from empty"
            );
            diff_eq!(sb_snapshot(c, &csb), sb_snapshot(r, &rsb), "pop from empty state");
        }
        // Push one byte, pop it, pop again past empty.
        (c.strbuffer_append_byte)(&mut csb, b'Q' as c_char);
        (r.strbuffer_append_byte)(&mut rsb, b'Q' as c_char);
        for _ in 0..3 {
            diff_eq!(
                (c.strbuffer_pop)(&mut csb),
                (r.strbuffer_pop)(&mut rsb),
                "pop to empty"
            );
        }
        diff_eq!(sb_snapshot(c, &csb), sb_snapshot(r, &rsb), "pop to empty state");
        (c.strbuffer_close)(&mut csb);
        (r.strbuffer_close)(&mut rsb);
    }
}

#[test]
fn strbuffer_pop_preserves_embedded_nul_bytes() {
    let _g = global_state_lock();
    // pop() returns the raw byte, so a stored NUL must come back as NUL and be
    // distinguishable from the empty-buffer case only via `length`.
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        (c.strbuffer_init)(&mut csb);
        (r.strbuffer_init)(&mut rsb);
        let data: Vec<c_char> = vec![b'a' as c_char, 0, b'b' as c_char, 0];
        (c.strbuffer_append_bytes)(&mut csb, data.as_ptr(), 4);
        (r.strbuffer_append_bytes)(&mut rsb, data.as_ptr(), 4);
        for i in 0..4 {
            diff_eq!(
                (c.strbuffer_pop)(&mut csb) as u8,
                (r.strbuffer_pop)(&mut rsb) as u8,
                "pop #{i} of NUL-containing buffer"
            );
            diff_eq!(csb.length, rsb.length, "length after pop #{i}");
        }
        (c.strbuffer_close)(&mut csb);
        (r.strbuffer_close)(&mut rsb);
    }
}

#[test]
fn strbuffer_steal_value_then_reinit() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        (c.strbuffer_init)(&mut csb);
        (r.strbuffer_init)(&mut rsb);
        (c.strbuffer_append_bytes)(&mut csb, cs("stolen goods").as_ptr(), 12);
        (r.strbuffer_append_bytes)(&mut rsb, cs("stolen goods").as_ptr(), 12);

        let cv = (c.strbuffer_steal_value)(&mut csb);
        let rv = (r.strbuffer_steal_value)(&mut rsb);
        diff_eq!(cbytes(cv), cbytes(rv), "strbuffer_steal_value contents");
        // steal NULLs the pointer but deliberately leaves size/length alone.
        diff_eq!(
            (csb.length, csb.size, csb.value.is_null()),
            (rsb.length, rsb.size, rsb.value.is_null()),
            "state after steal"
        );
        jfree(c, cv as *mut c_void);
        jfree(r, rv as *mut c_void);

        // The stolen buffer is re-usable via a fresh init.
        (c.strbuffer_init)(&mut csb);
        (r.strbuffer_init)(&mut rsb);
        diff_eq!(sb_snapshot(c, &csb), sb_snapshot(r, &rsb), "after re-init");
        (c.strbuffer_close)(&mut csb);
        (r.strbuffer_close)(&mut rsb);
    }
}

#[test]
fn strbuffer_close_on_stolen_buffer_is_safe() {
    let _g = global_state_lock();
    // close() checks `if (strbuff->value)`, so closing after a steal must be a
    // no-op rather than a double free.
    let (c, r) = both();
    unsafe {
        let mut csb = strbuffer_t::zeroed();
        let mut rsb = strbuffer_t::zeroed();
        (c.strbuffer_init)(&mut csb);
        (r.strbuffer_init)(&mut rsb);
        let cv = (c.strbuffer_steal_value)(&mut csb);
        let rv = (r.strbuffer_steal_value)(&mut rsb);
        (c.strbuffer_close)(&mut csb);
        (r.strbuffer_close)(&mut rsb);
        diff_eq!(
            (csb.length, csb.size, csb.value.is_null()),
            (rsb.length, rsb.size, rsb.value.is_null()),
            "close after steal"
        );
        jfree(c, cv as *mut c_void);
        jfree(r, rv as *mut c_void);
    }
}

// ===========================================================================
// memory.c
// ===========================================================================

#[test]
fn jsonp_malloc_zero_returns_null() {
    let _g = global_state_lock();
    // `if (!size) return NULL;` — a real, observable branch.
    let (c, r) = both();
    unsafe {
        let cp = (c.jsonp_malloc)(0);
        let rp = (r.jsonp_malloc)(0);
        diff_eq!(cp.is_null(), rp.is_null(), "jsonp_malloc(0) returns NULL");
    }
}

#[test]
fn jsonp_free_null_is_noop() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        (c.jsonp_free)(std::ptr::null_mut());
        (r.jsonp_free)(std::ptr::null_mut());
    }
}

#[test]
fn jsonp_malloc_and_free_round_trip() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x11_0002);
    unsafe {
        for _ in 0..500 {
            let n = 1 + rng.below(4096);
            let cp = (c.jsonp_malloc)(n);
            let rp = (r.jsonp_malloc)(n);
            diff_eq!(cp.is_null(), rp.is_null(), "jsonp_malloc({n}) null-ness");
            // Both must return usable memory of at least n bytes.
            if !cp.is_null() {
                std::ptr::write_bytes(cp as *mut u8, 0x5A, n);
                std::ptr::write_bytes(rp as *mut u8, 0x5A, n);
            }
            (c.jsonp_free)(cp);
            (r.jsonp_free)(rp);
        }
    }
}

#[test]
fn jsonp_realloc_grow_shrink_and_zero() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for (orig, new) in [
            (16usize, 32usize),
            (32, 16),
            (16, 16),
            (1, 4096),
            (4096, 1),
            (16, 0),
            (0, 16),
        ] {
            let cp = if orig > 0 { (c.jsonp_malloc)(orig) } else { std::ptr::null_mut() };
            let rp = if orig > 0 { (r.jsonp_malloc)(orig) } else { std::ptr::null_mut() };
            if orig > 0 {
                // Fill with a known pattern so a copying realloc emulation
                // would be observable.
                for i in 0..orig {
                    *(cp as *mut u8).add(i) = i as u8;
                    *(rp as *mut u8).add(i) = i as u8;
                }
            }
            let cq = (c.jsonp_realloc)(cp, orig, new);
            let rq = (r.jsonp_realloc)(rp, orig, new);
            diff_eq!(cq.is_null(), rq.is_null(), "jsonp_realloc({orig},{new}) null-ness");
            if !cq.is_null() {
                let keep = orig.min(new);
                let cbytes_: Vec<u8> =
                    (0..keep).map(|i| *(cq as *const u8).add(i)).collect();
                let rbytes_: Vec<u8> =
                    (0..keep).map(|i| *(rq as *const u8).add(i)).collect();
                diff_eq!(cbytes_, rbytes_, "jsonp_realloc({orig},{new}) preserved bytes");
            }
            (c.jsonp_free)(cq);
            (r.jsonp_free)(rq);
        }
    }
}

#[test]
fn jsonp_strndup_lengths() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x11_0003);
    unsafe {
        for _ in 0..600 {
            let s = rng.ascii_string(24);
            let cstring = cs_bytes(s.as_bytes());
            // len shorter than, equal to, and (deliberately) not longer than
            // the string — reading past the end would be UB in the C too.
            let len = rng.below(s.len() + 1);
            let cp = (c.jsonp_strndup)(cstring.as_ptr(), len);
            let rp = (r.jsonp_strndup)(cstring.as_ptr(), len);
            diff_eq!(cbytes(cp), cbytes(rp), "jsonp_strndup({s:?}, {len})");
            jfree(c, cp as *mut c_void);
            jfree(r, rp as *mut c_void);
        }
        // len 0 must still allocate a 1-byte "" (malloc(0+1)), not return NULL.
        let e = cs("anything");
        let cp = (c.jsonp_strndup)(e.as_ptr(), 0);
        let rp = (r.jsonp_strndup)(e.as_ptr(), 0);
        diff_eq!(cbytes(cp), cbytes(rp), "jsonp_strndup(_, 0)");
        diff_eq!(cp.is_null(), rp.is_null(), "jsonp_strndup(_, 0) null-ness");
        jfree(c, cp as *mut c_void);
        jfree(r, rp as *mut c_void);
    }
}

#[test]
fn jsonp_strndup_preserves_embedded_nuls() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let raw = b"ab\0cd";
        let buf = cs_bytes(raw);
        let cp = (c.jsonp_strndup)(buf.as_ptr(), 5);
        let rp = (r.jsonp_strndup)(buf.as_ptr(), 5);
        // Compare all 6 bytes (5 + terminator), not just up to the first NUL.
        let cv: Vec<u8> = (0..6).map(|i| *(cp as *const u8).add(i)).collect();
        let rv: Vec<u8> = (0..6).map(|i| *(rp as *const u8).add(i)).collect();
        diff_eq!(cv, rv, "jsonp_strndup over embedded NUL");
        jfree(c, cp as *mut c_void);
        jfree(r, rp as *mut c_void);
    }
}

// Custom allocators, used to prove the function-pointer slots behave the same.
// Counters are per-library so each side's traffic is measured independently.
static mut C_MALLOC_CALLS: usize = 0;
static mut C_FREE_CALLS: usize = 0;
static mut C_REALLOC_CALLS: usize = 0;
static mut R_MALLOC_CALLS: usize = 0;
static mut R_FREE_CALLS: usize = 0;
static mut R_REALLOC_CALLS: usize = 0;

extern "C" {
    fn malloc(n: size_t) -> *mut c_void;
    fn realloc(p: *mut c_void, n: size_t) -> *mut c_void;
    fn free(p: *mut c_void);
}

unsafe extern "C" fn c_my_malloc(n: size_t) -> *mut c_void {
    C_MALLOC_CALLS += 1;
    malloc(n)
}
unsafe extern "C" fn c_my_free(p: *mut c_void) {
    C_FREE_CALLS += 1;
    free(p)
}
unsafe extern "C" fn c_my_realloc(p: *mut c_void, n: size_t) -> *mut c_void {
    C_REALLOC_CALLS += 1;
    realloc(p, n)
}
unsafe extern "C" fn r_my_malloc(n: size_t) -> *mut c_void {
    R_MALLOC_CALLS += 1;
    malloc(n)
}
unsafe extern "C" fn r_my_free(p: *mut c_void) {
    R_FREE_CALLS += 1;
    free(p)
}
unsafe extern "C" fn r_my_realloc(p: *mut c_void, n: size_t) -> *mut c_void {
    R_REALLOC_CALLS += 1;
    realloc(p, n)
}

/// This test mutates process-global allocator state in both libraries, so it
/// must not run next to anything else that allocates. `--test-threads` is not
/// enough on its own, hence it lives in its own test binary section guarded by
/// restoring the defaults at the end.
#[test]
fn alloc_funcs_get_set_and_realloc_slot_semantics() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // ---- defaults: realloc slot is non-NULL (it starts as libc realloc)
        let mut cm: json_malloc_t = None;
        let mut crl: json_realloc_t = None;
        let mut cf: json_free_t = None;
        let mut rm: json_malloc_t = None;
        let mut rrl: json_realloc_t = None;
        let mut rf: json_free_t = None;
        (c.json_get_alloc_funcs2)(&mut cm, &mut crl, &mut cf);
        (r.json_get_alloc_funcs2)(&mut rm, &mut rrl, &mut rf);
        diff_eq!(
            (cm.is_some(), crl.is_some(), cf.is_some()),
            (rm.is_some(), rrl.is_some(), rf.is_some()),
            "default alloc func slots populated"
        );
        let default_c = (cm, crl, cf);
        let default_r = (rm, rrl, rf);

        // ---- set_alloc_funcs2 installs all three
        (c.json_set_alloc_funcs2)(Some(c_my_malloc), Some(c_my_realloc), Some(c_my_free));
        (r.json_set_alloc_funcs2)(Some(r_my_malloc), Some(r_my_realloc), Some(r_my_free));
        let (mut cm2, mut crl2, mut cf2) = (None, None, None);
        let (mut rm2, mut rrl2, mut rf2) = (None, None, None);
        (c.json_get_alloc_funcs2)(&mut cm2, &mut crl2, &mut cf2);
        (r.json_get_alloc_funcs2)(&mut rm2, &mut rrl2, &mut rf2);
        diff_eq!(
            (
                cm2 == Some(c_my_malloc as _),
                crl2 == Some(c_my_realloc as _),
                cf2 == Some(c_my_free as _)
            ),
            (
                rm2 == Some(r_my_malloc as _),
                rrl2 == Some(r_my_realloc as _),
                rf2 == Some(r_my_free as _)
            ),
            "set_alloc_funcs2 round trip"
        );

        // Both must actually route through the custom hooks.
        C_MALLOC_CALLS = 0;
        R_MALLOC_CALLS = 0;
        C_REALLOC_CALLS = 0;
        R_REALLOC_CALLS = 0;
        let cp = (c.jsonp_malloc)(64);
        let rp = (r.jsonp_malloc)(64);
        let cq = (c.jsonp_realloc)(cp, 64, 128);
        let rq = (r.jsonp_realloc)(rp, 64, 128);
        (c.jsonp_free)(cq);
        (r.jsonp_free)(rq);
        diff_eq!(
            (C_MALLOC_CALLS, C_REALLOC_CALLS),
            (R_MALLOC_CALLS, R_REALLOC_CALLS),
            "custom malloc/realloc hook call counts"
        );

        // ---- set_alloc_funcs (2-arg) must CLEAR the realloc slot to NULL,
        //      which switches jsonp_realloc to its malloc+memcpy emulation.
        (c.json_set_alloc_funcs)(Some(c_my_malloc), Some(c_my_free));
        (r.json_set_alloc_funcs)(Some(r_my_malloc), Some(r_my_free));
        let (mut cm3, mut crl3, mut cf3) = (None, None, None);
        let (mut rm3, mut rrl3, mut rf3) = (None, None, None);
        (c.json_get_alloc_funcs2)(&mut cm3, &mut crl3, &mut cf3);
        (r.json_get_alloc_funcs2)(&mut rm3, &mut rrl3, &mut rf3);
        diff_eq!(
            crl3.is_none(),
            rrl3.is_none(),
            "set_alloc_funcs must NULL the realloc slot"
        );
        assert!(crl3.is_none(), "C: realloc slot should be NULL after set_alloc_funcs");

        // With realloc NULL, jsonp_realloc emulates: malloc, copy min(orig,new), free.
        C_MALLOC_CALLS = 0;
        C_FREE_CALLS = 0;
        R_MALLOC_CALLS = 0;
        R_FREE_CALLS = 0;
        let cp = (c.jsonp_malloc)(8);
        let rp = (r.jsonp_malloc)(8);
        for i in 0..8usize {
            *(cp as *mut u8).add(i) = 0xC0 | i as u8;
            *(rp as *mut u8).add(i) = 0xC0 | i as u8;
        }
        let cq = (c.jsonp_realloc)(cp, 8, 24);
        let rq = (r.jsonp_realloc)(rp, 8, 24);
        let cv: Vec<u8> = (0..8).map(|i| *(cq as *const u8).add(i)).collect();
        let rv: Vec<u8> = (0..8).map(|i| *(rq as *const u8).add(i)).collect();
        diff_eq!(cv, rv, "emulated realloc copies the original bytes");
        diff_eq!(
            (C_MALLOC_CALLS, C_FREE_CALLS),
            (R_MALLOC_CALLS, R_FREE_CALLS),
            "emulated realloc malloc/free call counts"
        );
        (c.jsonp_free)(cq);
        (r.jsonp_free)(rq);

        // Emulated realloc with newSize == 0 must free and return NULL.
        let cp = (c.jsonp_malloc)(8);
        let rp = (r.jsonp_malloc)(8);
        let cq = (c.jsonp_realloc)(cp, 8, 0);
        let rq = (r.jsonp_realloc)(rp, 8, 0);
        diff_eq!(cq.is_null(), rq.is_null(), "emulated realloc to 0 returns NULL");

        // Emulated realloc of a NULL pointer must just malloc.
        let cq = (c.jsonp_realloc)(std::ptr::null_mut(), 0, 32);
        let rq = (r.jsonp_realloc)(std::ptr::null_mut(), 0, 32);
        diff_eq!(cq.is_null(), rq.is_null(), "emulated realloc(NULL, 0, 32)");
        (c.jsonp_free)(cq);
        (r.jsonp_free)(rq);

        // Emulated realloc(NULL, 0, 0) must return NULL without allocating.
        let cq = (c.jsonp_realloc)(std::ptr::null_mut(), 0, 0);
        let rq = (r.jsonp_realloc)(std::ptr::null_mut(), 0, 0);
        diff_eq!(cq.is_null(), rq.is_null(), "emulated realloc(NULL, 0, 0)");

        // ---- get_alloc_funcs (2-arg) reads back malloc+free only
        let (mut cm4, mut cf4) = (None, None);
        let (mut rm4, mut rf4) = (None, None);
        (c.json_get_alloc_funcs)(&mut cm4, &mut cf4);
        (r.json_get_alloc_funcs)(&mut rm4, &mut rf4);
        diff_eq!(
            (cm4 == Some(c_my_malloc as _), cf4 == Some(c_my_free as _)),
            (rm4 == Some(r_my_malloc as _), rf4 == Some(r_my_free as _)),
            "get_alloc_funcs round trip"
        );

        // ---- NULL out-params must be tolerated by both getters
        (c.json_get_alloc_funcs)(std::ptr::null_mut(), std::ptr::null_mut());
        (r.json_get_alloc_funcs)(std::ptr::null_mut(), std::ptr::null_mut());
        (c.json_get_alloc_funcs2)(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        (r.json_get_alloc_funcs2)(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        // Partial: only some out-params supplied.
        let mut only_free: json_free_t = None;
        let mut only_free_r: json_free_t = None;
        (c.json_get_alloc_funcs)(std::ptr::null_mut(), &mut only_free);
        (r.json_get_alloc_funcs)(std::ptr::null_mut(), &mut only_free_r);
        diff_eq!(
            only_free == Some(c_my_free as _),
            only_free_r == Some(r_my_free as _),
            "get_alloc_funcs with NULL malloc out-param"
        );

        // ---- restore the defaults so later tests use the normal allocator
        (c.json_set_alloc_funcs2)(default_c.0, default_c.1, default_c.2);
        (r.json_set_alloc_funcs2)(default_r.0, default_r.1, default_r.2);
    }
}

// ===========================================================================
// error.c
// ===========================================================================

#[test]
fn jsonp_error_init_null_and_normal() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // NULL error pointer must be a no-op, not a crash.
        (c.jsonp_error_init)(std::ptr::null_mut(), cs("src").as_ptr());
        (r.jsonp_error_init)(std::ptr::null_mut(), cs("src").as_ptr());

        // Start from a poisoned struct so exactly which fields get written is
        // part of the comparison.
        for src in [None, Some(""), Some("x"), Some("some/path/file.json")] {
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            let owned = src.map(cs);
            let p = owned.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
            (c.jsonp_error_init)(&mut ce, p);
            (r.jsonp_error_init)(&mut re, p);
            diff_eq!(ce.raw(), re.raw(), "jsonp_error_init(src={src:?}) raw image");
        }
    }
}

#[test]
fn jsonp_error_set_source_truncation_boundary() {
    let _g = global_state_lock();
    let (c, r) = both();
    // JSON_ERROR_SOURCE_LENGTH is 80. length < 80 copies verbatim; length >= 80
    // takes the "..." + tail branch with extra = length - 80 + 4.
    for len in 0usize..200 {
        let src: String = (0..len).map(|i| (b'a' + (i % 26) as u8) as char).collect();
        let cstr = cs(&src);
        unsafe {
            let mut ce = json_error_t::poisoned();
            let mut re = json_error_t::poisoned();
            (c.jsonp_error_set_source)(&mut ce, cstr.as_ptr());
            (r.jsonp_error_set_source)(&mut re, cstr.as_ptr());
            diff_eq!(
                ce.source.iter().map(|&x| x as u8).collect::<Vec<u8>>(),
                re.source.iter().map(|&x| x as u8).collect::<Vec<u8>>(),
                "jsonp_error_set_source(len={len}) full source buffer"
            );
        }
    }
}

#[test]
fn jsonp_error_set_source_null_args() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Both `!error` and `!source` return early without writing.
        (c.jsonp_error_set_source)(std::ptr::null_mut(), cs("s").as_ptr());
        (r.jsonp_error_set_source)(std::ptr::null_mut(), cs("s").as_ptr());

        let mut ce = json_error_t::poisoned();
        let mut re = json_error_t::poisoned();
        (c.jsonp_error_set_source)(&mut ce, std::ptr::null());
        (r.jsonp_error_set_source)(&mut re, std::ptr::null());
        diff_eq!(ce.raw(), re.raw(), "set_source(NULL source) leaves struct untouched");
    }
}

#[test]
fn jsonp_error_set_formats_and_stores_code() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for code in 0..20 {
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            (c.jsonp_error_init)(&mut ce, std::ptr::null());
            (r.jsonp_error_init)(&mut re, std::ptr::null());
            (c.jsonp_error_set)(
                &mut ce,
                7,
                13,
                42,
                code,
                cs("msg %s num %d").as_ptr(),
                cs("abc").as_ptr(),
                99 as c_int,
            );
            (r.jsonp_error_set)(
                &mut re,
                7,
                13,
                42,
                code,
                cs("msg %s num %d").as_ptr(),
                cs("abc").as_ptr(),
                99 as c_int,
            );
            diff_eq!(ce.raw(), re.raw(), "jsonp_error_set(code={code}) raw image");
            diff_eq!(ce.code(), re.code(), "jsonp_error_set(code={code}) code byte");
        }
    }
}

#[test]
fn jsonp_error_set_is_sticky() {
    let _g = global_state_lock();
    // `if (error->text[0] != '\0') return;` — the FIRST error wins and later
    // calls must not overwrite it.
    let (c, r) = both();
    unsafe {
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        (c.jsonp_error_init)(&mut ce, cs("s").as_ptr());
        (r.jsonp_error_init)(&mut re, cs("s").as_ptr());
        (c.jsonp_error_set)(&mut ce, 1, 2, 3, JSON_ERROR_INVALID_SYNTAX, cs("first").as_ptr());
        (r.jsonp_error_set)(&mut re, 1, 2, 3, JSON_ERROR_INVALID_SYNTAX, cs("first").as_ptr());
        (c.jsonp_error_set)(&mut ce, 9, 9, 9, JSON_ERROR_WRONG_TYPE, cs("second").as_ptr());
        (r.jsonp_error_set)(&mut re, 9, 9, 9, JSON_ERROR_WRONG_TYPE, cs("second").as_ptr());
        diff_eq!(ce.raw(), re.raw(), "second jsonp_error_set must be ignored");
        assert_eq!(ce.text_str(), "first", "C: first error must win");
    }
}

#[test]
fn jsonp_error_set_long_message_truncation() {
    let _g = global_state_lock();
    // vsnprintf is given JSON_ERROR_TEXT_LENGTH - 1 (159), then text[158] is
    // forced to '\0' and text[159] holds the code. A message longer than that
    // must truncate identically.
    let (c, r) = both();
    unsafe {
        for len in [0usize, 1, 100, 155, 156, 157, 158, 159, 160, 161, 200, 500] {
            let msg: String = (0..len).map(|i| (b'A' + (i % 26) as u8) as char).collect();
            let m = cs(&msg);
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            (c.jsonp_error_init)(&mut ce, std::ptr::null());
            (r.jsonp_error_init)(&mut re, std::ptr::null());
            // Pass the message as a %s argument so no format chars are
            // interpreted, regardless of its content.
            (c.jsonp_error_set)(&mut ce, 1, 1, 1, JSON_ERROR_UNKNOWN, cs("%s").as_ptr(), m.as_ptr());
            (r.jsonp_error_set)(&mut re, 1, 1, 1, JSON_ERROR_UNKNOWN, cs("%s").as_ptr(), m.as_ptr());
            diff_eq!(ce.raw(), re.raw(), "jsonp_error_set long msg len={len}");
        }
    }
}

#[test]
fn jsonp_error_set_null_error_is_noop() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        (c.jsonp_error_set)(std::ptr::null_mut(), 1, 2, 3, 4, cs("x").as_ptr());
        (r.jsonp_error_set)(std::ptr::null_mut(), 1, 2, 3, 4, cs("x").as_ptr());
    }
}

#[test]
fn jsonp_error_vset_via_shim() {
    let _g = global_state_lock();
    // Exercises the exported `jsonp_error_vset` symbol itself (not just the
    // variadic wrapper) by handing a real va_list through the C shim.
    let (c, r) = both();
    let sh = vashim();
    let cfn = sym_addr("C", b"jsonp_error_vset");
    let rfn = sym_addr("Rust", b"jsonp_error_vset");
    unsafe {
        for code in [0, 1, 5, 8, 17, 200, 255] {
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            (c.jsonp_error_init)(&mut ce, cs("via-shim").as_ptr());
            (r.jsonp_error_init)(&mut re, cs("via-shim").as_ptr());
            (sh.error_vset)(
                cfn,
                &mut ce,
                11,
                22,
                33,
                code,
                cs("v %s / %d / %g").as_ptr(),
                cs("str").as_ptr(),
                -7 as c_int,
                2.5f64,
            );
            (sh.error_vset)(
                rfn,
                &mut re,
                11,
                22,
                33,
                code,
                cs("v %s / %d / %g").as_ptr(),
                cs("str").as_ptr(),
                -7 as c_int,
                2.5f64,
            );
            diff_eq!(ce.raw(), re.raw(), "jsonp_error_vset(code={code})");
        }

        // NULL error pointer through the v-path too.
        (sh.error_vset)(cfn, std::ptr::null_mut(), 1, 1, 1, 0, cs("x").as_ptr());
        (sh.error_vset)(rfn, std::ptr::null_mut(), 1, 1, 1, 0, cs("x").as_ptr());
    }
}

#[test]
fn jsonp_error_set_position_is_truncated_to_int() {
    let _g = global_state_lock();
    // `error->position = (int)position;` — a size_t larger than INT_MAX must
    // wrap identically in both implementations.
    let (c, r) = both();
    unsafe {
        for pos in [
            0usize,
            1,
            i32::MAX as usize,
            i32::MAX as usize + 1,
            u32::MAX as usize,
            u32::MAX as usize + 1,
            usize::MAX,
        ] {
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            (c.jsonp_error_init)(&mut ce, std::ptr::null());
            (r.jsonp_error_init)(&mut re, std::ptr::null());
            (c.jsonp_error_set)(&mut ce, 0, 0, pos, JSON_ERROR_UNKNOWN, cs("p").as_ptr());
            (r.jsonp_error_set)(&mut re, 0, 0, pos, JSON_ERROR_UNKNOWN, cs("p").as_ptr());
            diff_eq!(ce.position, re.position, "position cast for {pos}");
        }
    }
}
