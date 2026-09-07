//! Phase B — `CONFIGS.md` rows 16..32: `stbds_arrgrowf`, `stbds_arrfreef`,
//! `stbds_stralloc`, `stbds_strreset`.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

// ---------------------------------------------------------------------------
// Arrays
// ---------------------------------------------------------------------------

/// Calls `stbds_arrgrowf` on both libs and asserts the *header* result matches.
/// (The freshly `realloc`ed payload is uninitialised in C, so only bytes the
/// test itself wrote are compared.)
fn grow_pair(
    a: (*mut c_void, *mut c_void),
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> (*mut c_void, *mut c_void) {
    let (lc, lr) = libs();
    unsafe {
        // Decide, *before* the call, whether the C code takes its early-out
        // branch (`min_cap <= stbds_arrcap(a)` after the `min_len` clamp).
        let before = arr_snap(a.0, 0);
        let effective_min_cap = {
            let min_len = before.length + addlen;
            if min_len > min_cap {
                min_len
            } else {
                min_cap
            }
        };
        let expect_early_out = effective_min_cap <= before.capacity;

        let c = (lc.arrgrowf)(a.0, elemsize, addlen, min_cap);
        let r = (lr.arrgrowf)(a.1, elemsize, addlen, min_cap);
        let sc = arr_snap(c, 0);
        let sr = arr_snap(r, 0);
        assert_eq!(
            sc, sr,
            "arrgrowf(elemsize={elemsize}, addlen={addlen}, min_cap={min_cap}) header diverged"
        );
        // On the early-out both sides must hand back the *input* pointer.  When
        // the function really grows, the returned pointer is `realloc`'s choice
        // and may coincidentally equal the old one, so identity is only
        // asserted for the early-out.
        if expect_early_out {
            assert_eq!(
                c, a.0,
                "C must early-out (elemsize={elemsize}, addlen={addlen}, min_cap={min_cap})"
            );
            assert_eq!(
                r, a.1,
                "Rust must early-out (elemsize={elemsize}, addlen={addlen}, min_cap={min_cap})"
            );
        }
        (c, r)
    }
}

unsafe fn free_pair(a: (*mut c_void, *mut c_void)) {
    let (lc, lr) = libs();
    if !a.0.is_null() {
        (lc.arrfreef)(a.0);
    }
    if !a.1.is_null() {
        (lr.arrfreef)(a.1);
    }
}

/// row 16 — `a=NULL, addlen=0, min_cap=0` → returns NULL, allocates nothing
#[test]
fn row16_arrgrowf_null_nogrow() {
    for elemsize in [1usize, 4, 8, 16, 24, 32, 64] {
        let (c, r) = grow_pair((std::ptr::null_mut(), std::ptr::null_mut()), elemsize, 0, 0);
        assert!(c.is_null() && r.is_null(), "expected NULL for elemsize={elemsize}");
    }
}

/// row 17 — `a=NULL, addlen=0, min_cap=1` → cap 4
#[test]
fn row17_arrgrowf_null_mincap1() {
    for elemsize in [1usize, 4, 8, 16, 24, 32, 64] {
        let p = grow_pair((std::ptr::null_mut(), std::ptr::null_mut()), elemsize, 0, 1);
        unsafe {
            assert_eq!(arr_snap(p.0, 0).capacity, 4);
            free_pair(p);
        }
    }
}

/// row 18 — `a=NULL, addlen=1..3, min_cap=0` → cap 4
#[test]
fn row18_arrgrowf_null_small_addlen() {
    for elemsize in [1usize, 8, 16, 32] {
        for addlen in 1..4usize {
            let p = grow_pair((std::ptr::null_mut(), std::ptr::null_mut()), elemsize, addlen, 0);
            unsafe {
                assert_eq!(arr_snap(p.0, 0).capacity, 4, "addlen={addlen}");
                free_pair(p);
            }
        }
    }
}

/// row 19 — `a=NULL`, `addlen` >= 4 → cap == addlen
#[test]
fn row19_arrgrowf_null_addlen_ge4() {
    let mut rng = Rng::new(0xB2_0019);
    for _ in 0..300 {
        let elemsize = 1 + rng.below(64);
        let addlen = 4 + rng.below(1000);
        let p = grow_pair((std::ptr::null_mut(), std::ptr::null_mut()), elemsize, addlen, 0);
        unsafe {
            assert_eq!(arr_snap(p.0, 0).capacity, addlen);
            free_pair(p);
        }
    }
}

/// row 20 — `a=NULL`, `min_cap` dominates
#[test]
fn row20_arrgrowf_null_mincap_dominates() {
    let mut rng = Rng::new(0xB2_0020);
    for _ in 0..300 {
        let elemsize = 1 + rng.below(32);
        let min_cap = 8 + rng.below(2000);
        let addlen = rng.below(min_cap);
        let p = grow_pair((std::ptr::null_mut(), std::ptr::null_mut()), elemsize, addlen, min_cap);
        unsafe {
            assert_eq!(arr_snap(p.0, 0).capacity, min_cap.max(addlen));
            free_pair(p);
        }
    }
}

/// row 21 — existing array, `min_cap <= cap` → same pointer, nothing changes
#[test]
fn row21_arrgrowf_nogrow_existing() {
    let elemsize = 16usize;
    let mut p = grow_pair((std::ptr::null_mut(), std::ptr::null_mut()), elemsize, 0, 100);
    unsafe {
        // give the header a distinctive state
        for q in [p.0, p.1] {
            let h = (q as *mut ArrayHeader).wrapping_sub(1);
            (*h).length = 37;
            (*h).temp = -7;
        }
        for min_cap in [0usize, 1, 4, 37, 50, 99, 100] {
            for addlen in [0usize, 1, 10, 63] {
                if 37 + addlen > min_cap.max(37 + addlen) {
                    // unreachable, keeps the intent explicit
                }
                let before = (arr_snap(p.0, 0), arr_snap(p.1, 0));
                let q = grow_pair(p, elemsize, addlen, min_cap);
                if 37 + addlen <= 100 && min_cap <= 100 {
                    assert_eq!(q.0, p.0, "C reallocated when it should not have");
                    assert_eq!(q.1, p.1, "Rust reallocated when it should not have");
                    assert_eq!(before.0, arr_snap(q.0, 0));
                    assert_eq!(before.1, arr_snap(q.1, 0));
                }
                p = q;
            }
        }
        free_pair(p);
    }
}

/// row 22 — existing array, `cap < min_cap < 2*cap` → cap doubles
#[test]
fn row22_arrgrowf_double() {
    let elemsize = 8usize;
    for cap in [4usize, 8, 16, 100, 1000] {
        let p = grow_pair((std::ptr::null_mut(), std::ptr::null_mut()), elemsize, 0, cap);
        unsafe {
            assert_eq!(arr_snap(p.0, 0).capacity, cap.max(4));
            let c0 = arr_snap(p.0, 0).capacity;
            let q = grow_pair(p, elemsize, 0, c0 + 1);
            assert_eq!(arr_snap(q.0, 0).capacity, 2 * c0, "cap={cap}");
            free_pair(q);
        }
    }
}

/// row 23 — existing array, `min_cap > 2*cap` → cap == min_cap
#[test]
fn row23_arrgrowf_mincap_beats_double() {
    let elemsize = 8usize;
    for cap in [4usize, 16, 128] {
        let p = grow_pair((std::ptr::null_mut(), std::ptr::null_mut()), elemsize, 0, cap);
        unsafe {
            let c0 = arr_snap(p.0, 0).capacity;
            let want = 5 * c0 + 3;
            let q = grow_pair(p, elemsize, 0, want);
            assert_eq!(arr_snap(q.0, 0).capacity, want);
            free_pair(q);
        }
    }
}

/// row 24 — repeated `arrput`-style growth chain, payload compared byte-for-byte
#[test]
fn row24_arrgrowf_push_chain() {
    let mut rng = Rng::new(0xB2_0024);
    for elemsize in [1usize, 2, 4, 8, 16, 24, 32, 64] {
        let mut p: (*mut c_void, *mut c_void) = (std::ptr::null_mut(), std::ptr::null_mut());
        let mut model: Vec<u8> = Vec::new();
        unsafe {
            for n in 0..500usize {
                // `stbds_arrmaybegrow(a,1)`
                let need = {
                    let s = arr_snap(p.0, 0);
                    p.0.is_null() || s.length + 1 > s.capacity
                };
                if need {
                    p = grow_pair(p, elemsize, 1, 0);
                }
                let bytes = rng.bytes(elemsize);
                for q in [p.0, p.1] {
                    let h = (q as *mut ArrayHeader).wrapping_sub(1);
                    let idx = (*h).length;
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        (q as *mut u8).add(elemsize * idx),
                        elemsize,
                    );
                    (*h).length += 1;
                }
                model.extend_from_slice(&bytes);
                let used = (n + 1) * elemsize;
                assert_eq!(
                    arr_snap(p.0, used),
                    arr_snap(p.1, used),
                    "push chain diverged at n={n}, elemsize={elemsize}"
                );
                assert_eq!(arr_snap(p.0, used).payload, model, "C payload/model mismatch");
            }
            free_pair(p);
        }
    }
}

/// row 25 — element-size sweep with randomized grow requests
#[test]
fn row25_arrgrowf_elemsize_sweep() {
    let mut rng = Rng::new(0xB2_0025);
    for elemsize in [1usize, 2, 3, 4, 5, 8, 12, 16, 24, 32, 48, 64, 100] {
        let mut p: (*mut c_void, *mut c_void) = (std::ptr::null_mut(), std::ptr::null_mut());
        for _ in 0..60 {
            let addlen = rng.below(20);
            let min_cap = rng.below(200);
            p = grow_pair(p, elemsize, addlen, min_cap);
            if !p.0.is_null() {
                unsafe {
                    let h = (p.0 as *mut ArrayHeader).wrapping_sub(1);
                    let h2 = (p.1 as *mut ArrayHeader).wrapping_sub(1);
                    let l = (*h).capacity.min(3);
                    (*h).length = l;
                    (*h2).length = l;
                }
            }
        }
        unsafe { free_pair(p) };
    }
}

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

struct ArenaPair {
    c: StringArena,
    r: StringArena,
    /// (`c` pointer, `r` pointer, expected contents)
    handed_out: Vec<(*mut c_char, *mut c_char, Vec<u8>)>,
}

impl ArenaPair {
    fn new() -> Self {
        ArenaPair {
            c: StringArena::zeroed(),
            r: StringArena::zeroed(),
            handed_out: Vec::new(),
        }
    }
    fn alloc(&mut self, body: &[u8]) {
        let (lc, lr) = libs();
        let mut s = cstr(body);
        unsafe {
            let pc = (lc.stralloc)(&mut self.c, s.as_mut_ptr() as *mut c_char);
            let pr = (lr.stralloc)(&mut self.r, s.as_mut_ptr() as *mut c_char);
            assert_eq!(
                std::ffi::CStr::from_ptr(pc).to_bytes(),
                body,
                "C stralloc returned wrong contents"
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(pr).to_bytes(),
                body,
                "Rust stralloc returned wrong contents"
            );
            assert_eq!(
                arena_snap(&self.c),
                arena_snap(&self.r),
                "arena state diverged after stralloc(len={})",
                body.len()
            );
            self.handed_out.push((pc, pr, body.to_vec()));
        }
    }
    /// Every pointer handed out earlier must still hold its string.
    fn verify_all(&self) {
        for (pc, pr, want) in &self.handed_out {
            unsafe {
                assert_eq!(
                    std::ffi::CStr::from_ptr(*pc).to_bytes(),
                    &want[..],
                    "C arena corrupted an earlier allocation"
                );
                assert_eq!(
                    std::ffi::CStr::from_ptr(*pr).to_bytes(),
                    &want[..],
                    "Rust arena corrupted an earlier allocation"
                );
            }
        }
    }
    fn reset(&mut self) {
        let (lc, lr) = libs();
        unsafe {
            (lc.strreset)(&mut self.c);
            (lr.strreset)(&mut self.r);
        }
        self.handed_out.clear();
        unsafe { assert_eq!(arena_snap(&self.c), arena_snap(&self.r)) };
        assert_eq!(
            unsafe { arena_snap(&self.c) },
            ArenaSnap {
                remaining: 0,
                block: 0,
                mode: 0,
                storage_null: true,
                block_count: 0
            }
        );
    }
}

/// row 26 — fresh arena, short string → first 512-byte block
#[test]
fn row26_stralloc_fresh_short() {
    let mut a = ArenaPair::new();
    a.alloc(b"hello");
    unsafe {
        let s = arena_snap(&a.c);
        assert_eq!(s.remaining, 512 - 6);
        assert_eq!(s.block, 1);
        assert_eq!(s.block_count, 1);
    }
    a.verify_all();
    a.reset();
}

/// row 27 — many short strings: block progression 0 → N
#[test]
fn row27_stralloc_block_progression() {
    let mut rng = Rng::new(0xB2_0027);
    let mut a = ArenaPair::new();
    for _ in 0..4000 {
        let n = rng.below(40);
        a.alloc(&rng.ascii(n));
    }
    unsafe {
        assert!(arena_snap(&a.c).block >= 4, "block never advanced");
    }
    a.verify_all();
    a.reset();
}

/// row 28 — `len > blocksize` on an empty arena → dedicated block, remaining 0
#[test]
fn row28_stralloc_oversize_empty_arena() {
    for n in [512usize, 513, 1000, 4096, 100_000] {
        let mut a = ArenaPair::new();
        let body = vec![b'x'; n];
        a.alloc(&body);
        unsafe {
            let s = arena_snap(&a.c);
            assert_eq!(s.remaining, 0, "n={n}");
            assert_eq!(s.block_count, 1);
            assert_eq!(s.block, 1);
        }
        a.verify_all();
        a.reset();
    }
}

/// row 29 — `len > blocksize` on a non-empty arena → spliced block,
/// `remaining` deliberately left untouched
#[test]
fn row29_stralloc_oversize_nonempty_arena() {
    let mut a = ArenaPair::new();
    a.alloc(b"short");
    let before = unsafe { arena_snap(&a.c) };
    a.alloc(&vec![b'y'; 5000]);
    unsafe {
        let s = arena_snap(&a.c);
        assert_eq!(s.remaining, before.remaining, "remaining must be unchanged");
        assert_eq!(s.block_count, 2);
    }
    a.verify_all();
    // keep going: the arena must still serve short strings out of the old block
    a.alloc(b"tail");
    a.verify_all();
    a.reset();
}

/// row 30 — randomized mixed short / over-sized sequence
#[test]
fn row30_stralloc_mixed_random() {
    let mut rng = Rng::new(0xB2_0030);
    for trial in 0..12 {
        let mut a = ArenaPair::new();
        for _ in 0..600 {
            let n = match rng.below(10) {
                0 => 1000 + rng.below(3000),
                1 => 400 + rng.below(300),
                _ => rng.below(60),
            };
            a.alloc(&rng.nonzero(n));
        }
        a.verify_all();
        a.reset();
        let _ = trial;
    }
}

/// row 31 — reset a multi-block arena and reuse it
#[test]
fn row31_strreset_and_reuse() {
    let mut rng = Rng::new(0xB2_0031);
    let mut a = ArenaPair::new();
    for round in 0..6 {
        for _ in 0..300 {
            let n = rng.below(80);
            a.alloc(&rng.ascii(n));
        }
        a.alloc(&vec![b'z'; 4000]);
        a.verify_all();
        a.reset();
        let _ = round;
    }
}

/// row 32 — `strreset` on a never-used arena
#[test]
fn row32_strreset_empty() {
    let mut a = ArenaPair::new();
    a.reset();
    a.reset();
    // and on an arena whose `block`/`mode` were poked but storage stays NULL
    let (lc, lr) = libs();
    let mut c = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 7, mode: 3 };
    let mut r = c;
    unsafe {
        (lc.strreset)(&mut c);
        (lr.strreset)(&mut r);
        assert_eq!(arena_snap(&c), arena_snap(&r));
        assert_eq!(arena_snap(&c).block, 0);
        assert_eq!(arena_snap(&c).mode, 0);
    }
}
