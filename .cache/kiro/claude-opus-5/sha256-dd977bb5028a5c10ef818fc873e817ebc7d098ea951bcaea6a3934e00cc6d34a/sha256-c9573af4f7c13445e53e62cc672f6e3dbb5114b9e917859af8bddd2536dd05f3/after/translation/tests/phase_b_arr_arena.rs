//! Phase B — CONFIGS.md rows 8..15: `stbds_arrgrowf` / `stbds_arrfreef` and the
//! string arena (`stbds_stralloc` / `stbds_strreset`).

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;

/// Canonical, address-independent view of a raw array produced by `arrgrowf`.
unsafe fn grow_dump(a: *mut c_void, elemsize: usize, written: usize) -> String {
    if a.is_null() {
        return "NULL".into();
    }
    let h = &*header(a);
    let mut s = format!(
        "len={} cap={} temp={} table_null={}",
        h.length,
        h.capacity,
        h.temp,
        h.hash_table.is_null()
    );
    for i in 0..written {
        let base = (a as *const u8).add(elemsize * i);
        s.push_str(&format!(
            " e{i}={:?}",
            std::slice::from_raw_parts(base, elemsize)
        ));
    }
    s
}

/// Row 8 — `arrgrowf(NULL, elemsize, addlen, min_cap)` across the full
/// cross-product of shapes.
#[test]
fn cfg_08_arrgrowf_from_null() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 8);
    let elemsizes = [1usize, 2, 4, 8, 12, 13, 16, 24, 40];
    let addlens = [0usize, 1, 2, 3, 5, 100];
    let mincaps = [0usize, 1, 2, 3, 4, 5, 7, 8, 64, 1000];
    for &e in &elemsizes {
        for &addlen in &addlens {
            for &min_cap in &mincaps {
                unsafe {
                    let ca = (p.c.arrgrowf)(std::ptr::null_mut(), e, addlen, min_cap);
                    let ra = (p.rs.arrgrowf)(std::ptr::null_mut(), e, addlen, min_cap);
                    assert_eq!(
                        ca.is_null(),
                        ra.is_null(),
                        "arrgrowf(NULL,{e},{addlen},{min_cap}) NULL-ness differs"
                    );
                    if ca.is_null() {
                        // addlen == 0 && min_cap == 0 => min_cap <= arrcap(NULL) => `return a`
                        assert_eq!((addlen, min_cap), (0, 0));
                        continue;
                    }
                    // Fill deterministically and re-compare so element storage
                    // (not just the header) is exercised.
                    let cap = (*header(ca)).capacity;
                    let fill: Vec<u8> = rng.bytes(e * cap.min(8));
                    std::ptr::copy_nonoverlapping(fill.as_ptr(), ca as *mut u8, fill.len());
                    std::ptr::copy_nonoverlapping(fill.as_ptr(), ra as *mut u8, fill.len());
                    let n = cap.min(8);
                    assert_eq!(
                        grow_dump(ca, e, n),
                        grow_dump(ra, e, n),
                        "arrgrowf(NULL,{e},{addlen},{min_cap})"
                    );
                    (p.c.arrfreef)(ca);
                    (p.rs.arrfreef)(ra);
                }
            }
        }
    }
}

/// Row 9 — `arrgrowf` on an existing array: no-op, sub-doubling and
/// super-doubling requests.
#[test]
fn cfg_09_arrgrowf_existing() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 9);
    let elemsizes = [1usize, 4, 8, 13, 16, 40];
    for &e in &elemsizes {
        for &start_cap in &[4usize, 5, 8, 16, 33, 100] {
            for &len in &[0usize, 1, 2, 3] {
                for &(addlen, min_cap) in &[
                    (0usize, 0usize),
                    (0, 1),
                    (0, start_cap),
                    (1, 0),
                    (1, start_cap),
                    (1, start_cap + 1),
                    (2, start_cap * 2),
                    (2, start_cap * 2 + 1),
                    (start_cap * 4, 0),
                    (0, start_cap * 4),
                    // NB: a request so large that realloc() fails is ERRORS.md
                    // row 44 (both implementations dereference NULL) and is
                    // deliberately not exercised here.
                ] {
                    unsafe {
                        let mut ca = (p.c.arrgrowf)(std::ptr::null_mut(), e, 0, start_cap);
                        let mut ra = (p.rs.arrgrowf)(std::ptr::null_mut(), e, 0, start_cap);
                        (*header(ca)).length = len;
                        (*header(ra)).length = len;
                        let payload: Vec<u8> = rng.bytes(e * start_cap.min(4));
                        std::ptr::copy_nonoverlapping(
                            payload.as_ptr(),
                            ca as *mut u8,
                            payload.len(),
                        );
                        std::ptr::copy_nonoverlapping(
                            payload.as_ptr(),
                            ra as *mut u8,
                            payload.len(),
                        );
                        // Sentinel in temp / hash_table to prove they survive.
                        (*header(ca)).temp = 0x1234;
                        (*header(ra)).temp = 0x1234;

                        let cb = (p.c.arrgrowf)(ca, e, addlen, min_cap);
                        let rb = (p.rs.arrgrowf)(ra, e, addlen, min_cap);
                        let n = start_cap.min(4);
                        assert_eq!(
                            grow_dump(cb, e, n),
                            grow_dump(rb, e, n),
                            "e={e} start_cap={start_cap} len={len} addlen={addlen} min_cap={min_cap}"
                        );
                        // realloc() may or may not move a block, so pointer
                        // identity is only *semantic* when the C code takes the
                        // `min_cap <= stbds_arrcap(a) => return a` early exit.
                        let effective = min_cap.max(len + addlen);
                        if effective <= start_cap {
                            assert_eq!(cb, ca, "C must return `a` unchanged (no growth needed)");
                            assert_eq!(rb, ra, "Rust must return `a` unchanged (no growth needed)");
                        }
                        ca = cb;
                        ra = rb;
                        (p.c.arrfreef)(ca);
                        (p.rs.arrfreef)(ra);
                    }
                }
            }
        }
    }
}

/// Row 10 — repeated doubling growth chain, then free.
#[test]
fn cfg_10_arrgrowf_growth_chain() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 10);
    for &e in &[1usize, 8, 13, 16, 40] {
        unsafe {
            let mut ca: *mut c_void = std::ptr::null_mut();
            let mut ra: *mut c_void = std::ptr::null_mut();
            for step in 0..300usize {
                let addlen = 1 + rng.below(3);
                let need = header_or_null(ca).0 + addlen;
                if need > cap_of(ca) {
                    ca = (p.c.arrgrowf)(ca, e, addlen, 0);
                    ra = (p.rs.arrgrowf)(ra, e, addlen, 0);
                }
                let base_len = (*header(ca)).length;
                (*header(ca)).length = base_len + addlen;
                (*header(ra)).length = base_len + addlen;
                for i in base_len..base_len + addlen {
                    let bytes = rng.bytes(e);
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        (ca as *mut u8).add(e * i),
                        e,
                    );
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        (ra as *mut u8).add(e * i),
                        e,
                    );
                }
                let n = (*header(ca)).length;
                assert_eq!(
                    grow_dump(ca, e, n),
                    grow_dump(ra, e, n),
                    "e={e} step={step}"
                );
            }
            (p.c.arrfreef)(ca);
            (p.rs.arrfreef)(ra);
        }
    }
}

unsafe fn header_or_null(a: *mut c_void) -> (usize, usize) {
    if a.is_null() {
        (0, 0)
    } else {
        let h = &*header(a);
        (h.length, h.capacity)
    }
}

unsafe fn cap_of(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

struct ArenaRun {
    arena: StringArena,
    /// every pointer handed back, with the string it must still contain
    live: Vec<(*mut c_char, Vec<u8>)>,
}

impl ArenaRun {
    fn new() -> ArenaRun {
        ArenaRun { arena: StringArena::zeroed(), live: Vec::new() }
    }
    unsafe fn alloc(&mut self, api: &Api, s: &mut [u8]) -> String {
        let p = (api.stralloc)(&mut self.arena as *mut StringArena, s.as_mut_ptr() as *mut c_char);
        assert!(!p.is_null());
        let content = std::ffi::CStr::from_ptr(p).to_bytes().to_vec();
        assert_eq!(
            content,
            &s[..s.len() - 1],
            "{}: stralloc returned wrong content",
            api.name
        );
        self.live.push((p, content));
        dump_arena(&self.arena)
    }
    /// Every previously returned pointer must still hold its string.
    unsafe fn verify_live(&self, api: &Api) {
        for (p, want) in &self.live {
            let got = std::ffi::CStr::from_ptr(*p).to_bytes();
            assert_eq!(got, &want[..], "{}: arena string corrupted", api.name);
        }
    }
    unsafe fn reset(&mut self, api: &Api) -> String {
        (api.strreset)(&mut self.arena as *mut StringArena);
        self.live.clear();
        dump_arena(&self.arena)
    }
}

fn cstr(n: usize, fill: u8) -> Vec<u8> {
    let mut v = vec![fill; n];
    v.push(0);
    v
}

/// Row 11 — fast path: many short strings into a fresh arena.
#[test]
fn cfg_11_stralloc_fast_path() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 11);
    let mut cr = ArenaRun::new();
    let mut rr = ArenaRun::new();
    for step in 0..400usize {
        let n = 1 + rng.below(40);
        let mut s = rng.cstring(n);
        let mut s2 = s.clone();
        unsafe {
            let cd = cr.alloc(&p.c, &mut s);
            let rd = rr.alloc(&p.rs, &mut s2);
            assert_eq!(cd, rd, "step={step} len={}", s.len());
            cr.verify_live(&p.c);
            rr.verify_live(&p.rs);
        }
    }
    unsafe {
        assert_eq!(cr.reset(&p.c), rr.reset(&p.rs));
    }
}

/// Row 12 — new-block path and the `block` progression up to saturation.
#[test]
fn cfg_12_stralloc_block_progression() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut cr = ArenaRun::new();
    let mut rr = ArenaRun::new();
    // Each iteration asks for exactly `blocksize` bytes, which empties the
    // current block and forces a new one on the next call.
    for step in 0..30usize {
        let blocksize: usize = 512usize << (cr.arena.block >> 1);
        assert_eq!(cr.arena.block, rr.arena.block, "block desync at step {step}");
        let mut s = cstr(blocksize - 1, b'x');
        let mut s2 = s.clone();
        unsafe {
            let cd = cr.alloc(&p.c, &mut s);
            let rd = rr.alloc(&p.rs, &mut s2);
            assert_eq!(cd, rd, "step={step} blocksize={blocksize}");
        }
    }
    assert!(
        cr.arena.block >= 22,
        "expected block to saturate, got {}",
        cr.arena.block
    );
    unsafe {
        cr.verify_live(&p.c);
        rr.verify_live(&p.rs);
        assert_eq!(cr.reset(&p.c), rr.reset(&p.rs));
    }
}

/// Row 13 — oversize string as the very first allocation (`storage == NULL`).
#[test]
fn cfg_13_stralloc_oversize_first() {
    let (p, _g) = begin(DEFAULT_SEED);
    for len in [512usize, 513, 600, 1024, 5000, 100_000] {
        let mut cr = ArenaRun::new();
        let mut rr = ArenaRun::new();
        let mut s = cstr(len, b'Z');
        let mut s2 = s.clone();
        unsafe {
            let cd = cr.alloc(&p.c, &mut s);
            let rd = rr.alloc(&p.rs, &mut s2);
            assert_eq!(cd, rd, "oversize-first len={len}");
            assert_eq!(cr.arena.remaining, 0, "remaining must be 0 (len={len})");
            // A follow-up short string must allocate a normal block.
            let mut t = cstr(4, b'q');
            let mut t2 = t.clone();
            assert_eq!(cr.alloc(&p.c, &mut t), rr.alloc(&p.rs, &mut t2), "after oversize");
            cr.verify_live(&p.c);
            rr.verify_live(&p.rs);
            assert_eq!(cr.reset(&p.c), rr.reset(&p.rs));
        }
    }
}

/// Row 14 — oversize string when the arena already has a block
/// (`storage != NULL`: the new block is spliced in *after* the head).
#[test]
fn cfg_14_stralloc_oversize_after() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 14);
    for big in [512usize, 513, 700, 2048, 70_000] {
        let mut cr = ArenaRun::new();
        let mut rr = ArenaRun::new();
        unsafe {
            for _ in 0..3 {
                let n = 1 + rng.below(20);
                let mut s = rng.cstring(n);
                let mut s2 = s.clone();
                assert_eq!(cr.alloc(&p.c, &mut s), rr.alloc(&p.rs, &mut s2));
            }
            let mut b = cstr(big, b'B');
            let mut b2 = b.clone();
            assert_eq!(cr.alloc(&p.c, &mut b), rr.alloc(&p.rs, &mut b2), "big={big}");
            for _ in 0..5 {
                let n = 1 + rng.below(20);
                let mut s = rng.cstring(n);
                let mut s2 = s.clone();
                assert_eq!(cr.alloc(&p.c, &mut s), rr.alloc(&p.rs, &mut s2), "big={big}");
            }
            cr.verify_live(&p.c);
            rr.verify_live(&p.rs);
            assert_eq!(cr.reset(&p.c), rr.reset(&p.rs));
        }
    }
}

/// Row 15 — randomized mixed-length sequences, reset, and reuse.
#[test]
fn cfg_15_stralloc_random_mixed() {
    let (p, _g) = begin(DEFAULT_SEED);
    let mut rng = Rng::new(SEED ^ 15);
    for round in 0..6usize {
        let mut cr = ArenaRun::new();
        let mut rr = ArenaRun::new();
        for step in 0..250usize {
            // Heavy tail so the oversize path is hit regularly.
            let n = match rng.below(10) {
                0 => 500 + rng.below(3000),
                1 => 500 + rng.below(40),
                _ => rng.below(64),
            };
            let mut s = rng.cstring(n);
            let mut s2 = s.clone();
            unsafe {
                assert_eq!(
                    cr.alloc(&p.c, &mut s),
                    rr.alloc(&p.rs, &mut s2),
                    "round={round} step={step} n={n}"
                );
            }
        }
        unsafe {
            cr.verify_live(&p.c);
            rr.verify_live(&p.rs);
            assert_eq!(cr.reset(&p.c), rr.reset(&p.rs));
            // reuse the same (now zeroed) arena
            let mut s = rng.cstring(30);
            let mut s2 = s.clone();
            assert_eq!(cr.alloc(&p.c, &mut s), rr.alloc(&p.rs, &mut s2));
            assert_eq!(cr.reset(&p.c), rr.reset(&p.rs));
        }
    }
}
