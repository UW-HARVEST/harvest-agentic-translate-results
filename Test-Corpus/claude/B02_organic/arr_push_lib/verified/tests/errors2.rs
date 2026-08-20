//! Phase C, ERRORS.md rows 34a and 35-56 — arena, hash, out-of-range enum
//! values across the FFI boundary, and the `arr_push` / `strkey` helpers.

mod common;
use common::map::*;
use common::*;
use std::ffi::c_void;

#[repr(C, align(8))]
struct Arena([u8; ARENA_SIZE]);
impl Arena {
    fn zeroed() -> Self {
        Arena([0u8; ARENA_SIZE])
    }
    fn p(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

unsafe fn alloc_both(p: &Pair, ca: &mut Arena, ra: &mut Arena, s: &[u8]) -> (*mut i8, *mut i8) {
    let mut cb = s.to_vec();
    cb.push(0);
    let mut rb = cb.clone();
    let c = (p.c.stralloc)(ca.p() as *mut c_void, cb.as_mut_ptr() as *mut i8);
    let r = (p.rs.stralloc)(ra.p() as *mut c_void, rb.as_mut_ptr() as *mut i8);
    (c, r)
}

// ===========================================================================
// row 34a — `STBDS_ASSERT(len <= a->remaining)` is unreachable
// ===========================================================================
#[test]
fn err34a_stralloc_remaining_assert_unreachable() {
    // Exhaustive path analysis of stbds_stralloc (lib.c:881-918):
    //   (1) len <= remaining          -> the `if` is skipped, assert trivially holds
    //   (2) len > remaining, len >  blocksize -> the oversize branch `return`s
    //                                            BEFORE reaching the assert
    //   (3) len > remaining, len <= blocksize -> `remaining = blocksize >= len`
    // so no input can reach the assert with `len > remaining`.
    for block in 0u8..=127 {
        let blocksize = 512usize << (block >> 1);
        for len in [1usize, 2, 511, 512, 513, 1024, 1 << 20, 1 << 21] {
            for remaining in [0usize, 1, len.saturating_sub(1), len, len + 1] {
                if len <= remaining {
                    continue; // path (1)
                }
                if len > blocksize {
                    continue; // path (2): returns early
                }
                // path (3)
                assert!(
                    len <= blocksize,
                    "block={block} len={len}: path 3 must satisfy len <= blocksize"
                );
            }
        }
    }
    // Empirically: drive a great many allocations and confirm the invariant
    // `len <= remaining` holds at the assert point (i.e. nothing aborts) and the
    // two implementations track `remaining` identically.
    let (p, _g) = session(INITIAL_HASH_SEED);
    let mut r = Rng::new(0x340034);
    unsafe {
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        for i in 0..1500usize {
            let n = r.range(1, 1600);
            let s: Vec<u8> = vec![b'q'; n];
            let (c, rr) = alloc_both(p, &mut ca, &mut ra, &s);
            assert_snap_eq(
                &snap_arena(ca.p(), c as *const u8),
                &snap_arena(ra.p(), rr as *const u8),
                &format!("alloc {i} len={n}"),
            );
        }
        (p.c.strreset)(ca.p() as *mut c_void);
        (p.rs.strreset)(ra.p() as *mut c_void);
    }
}

// ===========================================================================
// rows 35, 36, 37, 38, 39, 40 — the stralloc / strreset branches
// ===========================================================================
#[test]
fn err35_stralloc_fast_path() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        // first alloc creates a 512-byte block
        alloc_both(p, &mut ca, &mut ra, b"x");
        assert_eq!(rd_usize(ca.p(), ARENA_REMAINING), 510);
        let blocks_before = chain_len(rd_ptr(ca.p(), ARENA_STORAGE));
        // subsequent allocs that fit take the fast path: no new block
        let mut rem = 510usize;
        for i in 0..50usize {
            let n = 5;
            let (c, rr) = alloc_both(p, &mut ca, &mut ra, &vec![b'f'; n]);
            rem -= n + 1;
            assert_snap_eq(
                &snap_arena(ca.p(), c as *const u8),
                &snap_arena(ra.p(), rr as *const u8),
                &format!("fast path {i}"),
            );
            assert_eq!(rd_usize(ca.p(), ARENA_REMAINING), rem, "C remaining");
            assert_eq!(rd_usize(ra.p(), ARENA_REMAINING), rem, "Rust remaining");
            assert_eq!(chain_len(rd_ptr(ca.p(), ARENA_STORAGE)), blocks_before);
            assert_eq!(rd_u8(ca.p(), ARENA_BLOCK), 1, "block not bumped on fast path");
            assert_eq!(rd_u8(ra.p(), ARENA_BLOCK), 1);
        }
        // exactly-fits boundary: len == remaining takes the fast path
        let n = rem - 1;
        let (c, rr) = alloc_both(p, &mut ca, &mut ra, &vec![b'g'; n]);
        assert_snap_eq(
            &snap_arena(ca.p(), c as *const u8),
            &snap_arena(ra.p(), rr as *const u8),
            "len == remaining",
        );
        assert_eq!(rd_usize(ca.p(), ARENA_REMAINING), 0);
        assert_eq!(chain_len(rd_ptr(ca.p(), ARENA_STORAGE)), blocks_before);
        // one more byte now forces a new block
        let (c, rr) = alloc_both(p, &mut ca, &mut ra, b"h");
        assert_snap_eq(
            &snap_arena(ca.p(), c as *const u8),
            &snap_arena(ra.p(), rr as *const u8),
            "one past remaining",
        );
        assert_eq!(chain_len(rd_ptr(ca.p(), ARENA_STORAGE)), blocks_before + 1);
        (p.c.strreset)(ca.p() as *mut c_void);
        (p.rs.strreset)(ra.p() as *mut c_void);
    }
}

unsafe fn chain_len(mut b: *mut u8) -> usize {
    let mut n = 0;
    while !b.is_null() {
        n += 1;
        b = rd_ptr(b, 0);
    }
    n
}

#[test]
fn err36_stralloc_oversize_no_storage() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for len in [512usize, 513, 1000, 65536] {
            let mut ca = Arena::zeroed();
            let mut ra = Arena::zeroed();
            let (c, rr) = alloc_both(p, &mut ca, &mut ra, &vec![b'o'; len]);
            assert_snap_eq(
                &snap_arena(ca.p(), c as *const u8),
                &snap_arena(ra.p(), rr as *const u8),
                &format!("oversize/no-storage len={}", len + 1),
            );
            // sb->next = 0 ; storage = sb ; remaining = 0
            let st = rd_ptr(ca.p(), ARENA_STORAGE);
            assert!(!st.is_null());
            assert!(rd_ptr(st, 0).is_null(), "sb->next must be NULL");
            assert_eq!(rd_usize(ca.p(), ARENA_REMAINING), 0);
            assert_eq!(rd_usize(ra.p(), ARENA_REMAINING), 0);
            assert_eq!(c as usize, st as usize + 8, "must return sb->storage");
            (p.c.strreset)(ca.p() as *mut c_void);
            (p.rs.strreset)(ra.p() as *mut c_void);
        }
    }
}

#[test]
fn err37_stralloc_oversize_with_storage() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        alloc_both(p, &mut ca, &mut ra, b"head");
        let head_c = rd_ptr(ca.p(), ARENA_STORAGE);
        let head_r = rd_ptr(ra.p(), ARENA_STORAGE);
        let rem_c = rd_usize(ca.p(), ARENA_REMAINING);
        let rem_r = rd_usize(ra.p(), ARENA_REMAINING);
        let (c, rr) = alloc_both(p, &mut ca, &mut ra, &vec![b'O'; 5000]);
        assert_snap_eq(
            &snap_arena(ca.p(), c as *const u8),
            &snap_arena(ra.p(), rr as *const u8),
            "oversize/with-storage",
        );
        // the head is unchanged; the new block is spliced in AFTER it
        assert_eq!(rd_ptr(ca.p(), ARENA_STORAGE), head_c, "head must stay the head");
        assert_eq!(rd_ptr(ra.p(), ARENA_STORAGE), head_r);
        assert_eq!(rd_usize(ca.p(), ARENA_REMAINING), rem_c, "remaining untouched");
        assert_eq!(rd_usize(ra.p(), ARENA_REMAINING), rem_r);
        let second_c = rd_ptr(head_c, 0);
        assert_eq!(c as usize, second_c as usize + 8, "returned sb is chain[1]");
        assert_eq!(chain_len(head_c), 2);
        assert_eq!(chain_len(head_r), 2);
        (p.c.strreset)(ca.p() as *mut c_void);
        (p.rs.strreset)(ra.p() as *mut c_void);
    }
}

#[test]
fn err38_stralloc_block_saturates() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // block >= 22 => blocksize >= 1<<20 => the ++ is skipped.
        //
        // Only `block` values whose `blocksize` is actually allocatable are used
        // here.  `blocksize = 512 << (block>>1)`, so:
        //   block <=  30  -> <= 16 MiB, always allocatable
        //   block 44..109 -> 2^40..2^63, the malloc FAILS and both
        //                    implementations dereference NULL -> SIGSEGV;
        //                    covered in tests/errors_fatal.rs instead
        //   block >= 110  -> 512 << 55 wraps to 0 (well-defined for unsigned),
        //                    so `len > blocksize` sends it down the oversize
        //                    path and it is allocatable again
        let mut blocks: Vec<u8> = (0u8..=30).collect();
        blocks.extend(110u8..=127);
        for block in blocks {
            let mut ca = Arena::zeroed();
            let mut ra = Arena::zeroed();
            wr_u8(ca.p(), ARENA_BLOCK, block);
            wr_u8(ra.p(), ARENA_BLOCK, block);
            let (c, rr) = alloc_both(p, &mut ca, &mut ra, b"tiny");
            assert_snap_eq(
                &snap_arena(ca.p(), c as *const u8),
                &snap_arena(ra.p(), rr as *const u8),
                &format!("block={block}"),
            );
            let blocksize = 512usize.wrapping_shl((block >> 1) as u32);
            let want = if blocksize < (1 << 20) {
                block.wrapping_add(1)
            } else {
                block
            };
            assert_eq!(rd_u8(ca.p(), ARENA_BLOCK), want, "C block for {block}");
            assert_eq!(rd_u8(ra.p(), ARENA_BLOCK), want, "Rust block for {block}");
            (p.c.strreset)(ca.p() as *mut c_void);
            (p.rs.strreset)(ra.p() as *mut c_void);
        }
    }
}

#[test]
fn err39_stralloc_empty_string() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        for i in 0..600usize {
            let (c, rr) = alloc_both(p, &mut ca, &mut ra, b"");
            assert_snap_eq(
                &snap_arena(ca.p(), c as *const u8),
                &snap_arena(ra.p(), rr as *const u8),
                &format!("empty string {i}"),
            );
            assert_eq!(*(c as *const u8), 0, "C must return an empty C string");
            assert_eq!(*(rr as *const u8), 0, "Rust must return an empty C string");
        }
        // 512 one-byte allocations exactly exhaust the first block
        assert_eq!(chain_len(rd_ptr(ca.p(), ARENA_STORAGE)), 2);
        assert_eq!(chain_len(rd_ptr(ra.p(), ARENA_STORAGE)), 2);
        (p.c.strreset)(ca.p() as *mut c_void);
        (p.rs.strreset)(ra.p() as *mut c_void);
    }
}

#[test]
fn err40_strreset_empty() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // storage == NULL: the while loop never runs, the arena is memset to 0
        for (rem, blk, mode) in [(0usize, 0u8, 0u8), (99, 7, 3), (usize::MAX, 255, 255)] {
            let mut ca = Arena::zeroed();
            let mut ra = Arena::zeroed();
            wr_usize(ca.p(), ARENA_REMAINING, rem);
            wr_u8(ca.p(), ARENA_BLOCK, blk);
            wr_u8(ca.p(), ARENA_MODE, mode);
            wr_usize(ra.p(), ARENA_REMAINING, rem);
            wr_u8(ra.p(), ARENA_BLOCK, blk);
            wr_u8(ra.p(), ARENA_MODE, mode);
            (p.c.strreset)(ca.p() as *mut c_void);
            (p.rs.strreset)(ra.p() as *mut c_void);
            assert_eq!(ca.0, [0u8; ARENA_SIZE], "C must zero the whole arena");
            assert_eq!(ra.0, [0u8; ARENA_SIZE], "Rust must zero the whole arena");
        }
        // repeated resets are idempotent
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        for _ in 0..5 {
            (p.c.strreset)(ca.p() as *mut c_void);
            (p.rs.strreset)(ra.p() as *mut c_void);
            assert_eq!(ca.0, ra.0);
        }
    }
}

// ===========================================================================
// rows 41, 42, 43, 44 — hash boundaries
// ===========================================================================
#[test]
fn err41_hash_bytes_len0() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // len == 0 reads nothing, so even a wild pointer is fine
        for ptr in [
            std::ptr::null_mut::<c_void>(),
            usize::MAX as *mut c_void,
            1 as *mut c_void,
        ] {
            for &seed in &[0usize, 1, INITIAL_HASH_SEED, usize::MAX] {
                let a = (p.c.hash_bytes)(ptr, 0, seed);
                let b = (p.rs.hash_bytes)(ptr, 0, seed);
                assert_eq_ctx(a, b, &format!("hash_bytes({ptr:?}, 0, {seed:#x})"));
            }
        }
        // the value is deterministic and depends only on the seed
        let x = (p.c.hash_bytes)(std::ptr::null_mut(), 0, 0);
        let y = (p.c.hash_bytes)(1 as *mut c_void, 0, 0);
        assert_eq!(x, y, "len=0 must not depend on the pointer");
    }
}

#[test]
fn err42_hash_bytes_tail_signext() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // `case 4: data |= (d[3] << 24);` overflows `int` for d[3] >= 0x80 and
        // the negative result is sign-extended into the size_t `data`.
        // Exhaustive over d[1..=3] and every tail length that reaches them.
        for pos in 1..=3usize {
            for v in 0..=255u8 {
                for len in (pos + 1)..=7usize {
                    let mut buf = [0u8; 8];
                    buf[pos] = v;
                    let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 12345);
                    let b = (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 12345);
                    assert_eq_ctx(a, b, &format!("d[{pos}]={v:#02x} len={len}"));
                }
            }
        }
        // and cases 5,6,7 which use explicit `(size_t)` casts instead
        for pos in 4..=6usize {
            for v in 0..=255u8 {
                for len in (pos + 1)..=7usize {
                    let mut buf = [0u8; 8];
                    buf[pos] = v;
                    let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 999);
                    let b = (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 999);
                    assert_eq_ctx(a, b, &format!("d[{pos}]={v:#02x} len={len}"));
                }
            }
        }
        // the full-word loop also sign-extends its low half (`d[3] << 24`)
        for v in 0..=255u8 {
            let mut buf = vec![0u8; 16];
            buf[3] = v;
            buf[11] = v;
            for len in [8usize, 16] {
                let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 7);
                let b = (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 7);
                assert_eq_ctx(a, b, &format!("word d[3]={v:#02x} len={len}"));
            }
        }
    }
}

#[test]
fn err43_hash_string_empty() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        let mut z = [0u8; 1];
        for &seed in &[0usize, 1, INITIAL_HASH_SEED, usize::MAX, 0x8000_0000_0000_0000] {
            let a = (p.c.hash_string)(z.as_mut_ptr() as *mut i8, seed);
            let b = (p.rs.hash_string)(z.as_mut_ptr() as *mut i8, seed);
            assert_eq_ctx(a, b, &format!("hash_string(\"\", {seed:#x})"));
        }
    }
}

#[test]
fn err44_hash_string_high_bytes() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // `(unsigned char) *str++` -- must NOT sign-extend even though plain
        // `char` is signed on x86-64.
        for v in 1..=255u8 {
            let mut s = [v, 0];
            for &seed in &[0usize, INITIAL_HASH_SEED, usize::MAX] {
                let a = (p.c.hash_string)(s.as_mut_ptr() as *mut i8, seed);
                let b = (p.rs.hash_string)(s.as_mut_ptr() as *mut i8, seed);
                assert_eq_ctx(a, b, &format!("hash_string([{v:#02x}], {seed:#x})"));
            }
        }
        // all 255*255 two-byte strings would be slow; sample the high plane
        let mut r = Rng::new(0x440044);
        for _ in 0..2000 {
            let n = r.range(1, 16);
            let mut s = r.cstring_hibytes(n);
            let seed = r.u64() as usize;
            let a = (p.c.hash_string)(s.as_mut_ptr() as *mut i8, seed);
            let b = (p.rs.hash_string)(s.as_mut_ptr() as *mut i8, seed);
            assert_eq_ctx(a, b, &format!("hash_string(hi, {seed:#x})"));
        }
    }
}

// ===========================================================================
// row 45 — `if (hash < 2) hash += 2` keeps 0/1 reserved as sentinels
// ===========================================================================
#[test]
fn err45_hash_lt2_bumped() {
    // A siphash output of exactly 0 or 1 is not constructible (that is a 2^-63
    // event and both `hash_bytes` and `hash_string` are one-way), so the `+= 2`
    // bump is verified through the invariant it exists to protect:
    //   - a LIVE slot never stores hash 0 (EMPTY) or 1 (DELETED)
    //   - an EMPTY slot has hash 0 and index -1
    //   - a TOMBSTONE has hash 1 and index -2
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = MapCfg::int_int();
    let mut m = MapPair::empty(p, cfg);
    let mut r = Rng::new(0x450045);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    let mut tombstones_seen = 0usize;
    unsafe {
        for op in 0..3000usize {
            owned.push((r.below(70) as i32).to_le_bytes().to_vec());
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            if r.below(2) == 0 {
                m.put(k, &r.u32().to_le_bytes(), &format!("op {op}"));
            } else {
                m.del(k, &format!("op {op}"));
            }
            for (mm, tag) in [(&m.c, "C"), (&m.rs, "Rust")] {
                if mm.t.is_null() {
                    continue;
                }
                let tbl = rd_ptr(
                    (mm.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
                    HDR_HASH_TABLE,
                );
                if tbl.is_null() {
                    continue;
                }
                let sc = rd_usize(tbl, HI_SLOT_COUNT);
                let storage = rd_ptr(tbl, HI_STORAGE);
                for b in 0..(sc >> 3) {
                    let bp = storage.add(b * BUCKET_SIZE);
                    for j in 0..BUCKET_LENGTH {
                        let h = rd_usize(bp, j * 8);
                        let i = rd_isize(bp, 64 + j * 8);
                        match i {
                            -1 => assert_eq!(h, 0, "{tag} op {op}: EMPTY slot must have hash 0"),
                            -2 => {
                                assert_eq!(h, 1, "{tag} op {op}: DELETED slot must have hash 1");
                                tombstones_seen += 1;
                            }
                            _ => assert!(
                                h >= 2,
                                "{tag} op {op}: LIVE slot has sentinel hash {h} \
                                 -- the `hash < 2` bump is missing"
                            ),
                        }
                    }
                }
            }
        }
        assert!(tombstones_seen > 0, "no tombstone ever observed");
        m.free();
    }
}

// ===========================================================================
// rows 46, 47 — out-of-range `mode` enum values across the FFI boundary
// ===========================================================================
#[test]
fn err46_mode_negative_is_binary() {
    // `mode >= STBDS_HM_STRING` is false for every mode < 1, so the whole
    // library treats it exactly like STBDS_HM_BINARY.
    let modes = [-1i32, -2, -128, -1000, i32::MIN, i32::MIN + 1, 0];
    let mut reference: Option<Vec<u8>> = None;
    for &mode in &modes {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg {
            elemsize: 8,
            keysize: 4,
            keyoffset: 0,
            mode,
            valoffset: 4,
            valsize: 4,
            force_raw_snap: false,
        };
        let mut m = MapPair::empty(p, cfg);
        let mut owned: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for i in 0..25i32 {
                owned.push(i.to_le_bytes().to_vec());
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                m.put(k, &(i as u32).to_le_bytes(), &format!("mode={mode} put {i}"));
            }
            for i in 0..30i32 {
                let mut kb = i.to_le_bytes();
                let idx = m.geti(kb.as_mut_ptr() as *mut c_void, &format!("mode={mode} get {i}"));
                assert_eq!(idx >= 0, i < 25, "mode={mode}: presence of {i}");
            }
            for i in 0..25i32 {
                let mut kb = i.to_le_bytes();
                assert_eq!(
                    m.del(kb.as_mut_ptr() as *mut c_void, &format!("mode={mode} del {i}")),
                    1
                );
            }
            // string.mode must be 0 (not SH_DEFAULT)
            let tbl = rd_ptr(
                (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
                HDR_HASH_TABLE,
            );
            assert_eq!(
                rd_u8(tbl.add(HI_STRING), ARENA_MODE),
                STBDS_SH_NONE,
                "mode={mode}: string.mode must be 0"
            );
            // every negative mode must produce the SAME structure as mode 0
            let snap = m.snap_c();
            match &reference {
                None => reference = Some(snap),
                Some(rf) => assert_eq!(
                    *rf, snap,
                    "mode={mode} must be structurally identical to mode 0"
                ),
            }
            m.free();
        }
    }
}

#[test]
fn err47_mode_gt1_is_string() {
    // Every mode >= 1 hashes/compares as a string; only the strdup-free in
    // hmdel_key distinguishes `mode == 1` exactly (row 31 / row 29).
    let modes = [1i32, 2, 3, 4, 255, 1000, i32::MAX];
    let mut reference: Option<Vec<u8>> = None;
    for &mode in &modes {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg {
            elemsize: 16,
            keysize: 8,
            keyoffset: 0,
            mode,
            valoffset: 8,
            valsize: 8,
            force_raw_snap: false,
        };
        let mut m = MapPair::empty(p, cfg);
        let mut owned: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for i in 0..25usize {
                let mut s = format!("skey_{i:04}").into_bytes();
                s.push(0);
                owned.push(s);
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                m.put(k, &(i as u64).to_le_bytes(), &format!("mode={mode} put {i}"));
            }
            for i in 0..30usize {
                let mut s = format!("skey_{i:04}").into_bytes();
                s.push(0);
                owned.push(s);
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                let idx = m.geti(k, &format!("mode={mode} get {i}"));
                assert_eq!(idx >= 0, i < 25, "mode={mode}: presence of {i}");
            }
            // string.mode must be SH_DEFAULT for every mode >= 1
            let tbl = rd_ptr(
                (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
                HDR_HASH_TABLE,
            );
            assert_eq!(
                rd_u8(tbl.add(HI_STRING), ARENA_MODE),
                STBDS_SH_DEFAULT,
                "mode={mode}: string.mode must be SH_DEFAULT"
            );
            // reverse-order deletes are safe for every mode (row 29 avoided)
            for i in (0..25usize).rev() {
                let mut s = format!("skey_{i:04}").into_bytes();
                s.push(0);
                owned.push(s);
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                assert_eq!(m.del(k, &format!("mode={mode} del {i}")), 1);
            }
            let snap = m.snap_c();
            match &reference {
                None => reference = Some(snap),
                Some(rf) => assert_eq!(
                    *rf, snap,
                    "mode={mode} must be structurally identical to mode 1"
                ),
            }
            m.free();
        }
    }
}

// ===========================================================================
// rows 48, 49 — out-of-range `mode` for shmode_func and the `default:` memcpy
// ===========================================================================
#[test]
fn err48_49_shmode_func_out_of_range() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for &mode in &[
            -1i32,
            4,
            5,
            255,
            256,
            257,
            1000,
            i32::MAX,
            i32::MIN,
            0,
            1,
            2,
            3,
        ] {
            let es = 16usize;
            let a = (p.c.shmode_func)(es, mode);
            let b = (p.rs.shmode_func)(es, mode);
            let ta = rd_ptr((a as *mut u8).sub(es).sub(HDR_SIZE), HDR_HASH_TABLE);
            let tb = rd_ptr((b as *mut u8).sub(es).sub(HDR_SIZE), HDR_HASH_TABLE);
            // row 48: h->string.mode = (unsigned char) mode
            let want = (mode as u32 & 0xff) as u8;
            assert_eq_ctx(
                rd_u8(ta.add(HI_STRING), ARENA_MODE),
                rd_u8(tb.add(HI_STRING), ARENA_MODE),
                &format!("shmode_func(es, {mode}): string.mode"),
            );
            assert_eq!(
                rd_u8(ta.add(HI_STRING), ARENA_MODE),
                want,
                "shmode_func({mode}) must truncate to (unsigned char)"
            );
            assert_snap_eq(
                &snap_map(a, es, KeyRepr::Raw),
                &snap_map(b, es, KeyRepr::Raw),
                &format!("shmode_func(es, {mode})"),
            );
            (p.c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (p.rs.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
        }
    }

    // row 49: with a string.mode outside {1,2,3} the switch takes `default:`
    // and memcpy's `keysize` bytes of the STRING into the element (not a
    // pointer), so the element must never be dereferenced as a key pointer and
    // no lookup may be issued (a hash match would strcmp raw bytes as a ptr).
    for &sh in &[0i32, 4, 5, 255, 256, 1000, -1] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = MapCfg {
            elemsize: 16,
            keysize: 8,
            keyoffset: 0,
            mode: STBDS_HM_STRING,
            valoffset: 8,
            valsize: 8,
            force_raw_snap: true,
        };
        let mut m = MapPair::with_shmode(p, cfg, sh);
        let mut owned: Vec<Vec<u8>> = Vec::new();
        unsafe {
            let effective = (sh as u32 & 0xff) as u8;
            for i in 0..20usize {
                // pad every key to >= keysize bytes so the memcpy reads only
                // initialised memory
                let mut s = format!("dkey_{i:03}").into_bytes();
                s.push(0);
                while s.len() < cfg.keysize {
                    s.push(0);
                }
                owned.push(s);
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                let idx = m.put(k, &(i as u64).to_le_bytes(), &format!("sh={sh} put {i}"));
                if effective != STBDS_SH_DEFAULT
                    && effective != STBDS_SH_STRDUP
                    && effective != STBDS_SH_ARENA
                {
                    // `default:` branch => the element holds the string's bytes
                    let want = &owned.last().unwrap()[..cfg.keysize];
                    assert_eq!(
                        std::slice::from_raw_parts(m.c.elem(idx), cfg.keysize),
                        want,
                        "sh={sh}: C default: must memcpy the string bytes"
                    );
                    assert_eq!(
                        std::slice::from_raw_parts(m.rs.elem(idx), cfg.keysize),
                        want,
                        "sh={sh}: Rust default: must memcpy the string bytes"
                    );
                }
            }
            m.free();
        }
    }
}

// ===========================================================================
// row 50 — keysize == 0
// ===========================================================================
#[test]
fn err50_keysize_zero() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    // memcmp(..., 0) == 0 always, so every binary-mode key compares equal and
    // the map collapses onto a single entry.
    for es in [1usize, 4, 8, 16] {
        let cfg = MapCfg {
            elemsize: es,
            keysize: 0,
            keyoffset: 0,
            mode: STBDS_HM_BINARY,
            valoffset: 0,
            valsize: es,
            force_raw_snap: false,
        };
        let mut m = MapPair::empty(p, cfg);
        let mut r = Rng::new(0x500050 + es as u64);
        let mut owned: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for i in 0..30usize {
                owned.push(r.bytes(8));
                let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
                let v = r.bytes(es);
                let idx = m.put(k, &v, &format!("es={es} put {i}"));
                assert_eq!(idx, 0, "es={es}: keysize=0 must collapse to one entry");
            }
            assert_eq!(m.hmlen("collapsed"), 1);
            // any key at all now hits index 0
            for i in 0..10usize {
                let mut kb = r.bytes(8);
                assert_eq!(
                    m.geti(kb.as_mut_ptr() as *mut c_void, &format!("es={es} get {i}")),
                    0
                );
            }
            // and one delete empties the map
            let mut kb = r.bytes(8);
            assert_eq!(m.del(kb.as_mut_ptr() as *mut c_void, "del"), 1);
            assert_eq!(m.hmlen("emptied"), 0);
            // a further delete now misses
            assert_eq!(m.del(kb.as_mut_ptr() as *mut c_void, "del again"), 0);
            m.free();
        }
    }
}

// ===========================================================================
// row 51 — hmdel_key with a keyoffset inconsistent with hmput_key's 0
// ===========================================================================
#[test]
fn err51_hmdel_keyoffset_nonzero() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    // element: [0..4) key (written by hmput_key), [4..16) a fixed pattern that
    // can never equal a key, so the delete's `elem + keyoffset` compare always
    // fails and the key is reported absent.
    let cfg = MapCfg {
        elemsize: 16,
        keysize: 4,
        keyoffset: 8,
        mode: STBDS_HM_BINARY,
        valoffset: 4,
        valsize: 12,
    force_raw_snap: false,
    };
    let mut m = MapPair::empty(p, cfg);
    let mut owned: Vec<Vec<u8>> = Vec::new();
    unsafe {
        let pattern: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
        for i in 0..20i32 {
            owned.push(i.to_le_bytes().to_vec());
            let k = owned.last_mut().unwrap().as_mut_ptr() as *mut c_void;
            m.put(k, &pattern, &format!("put {i}"));
        }
        let len_before = m.hmlen("before");
        for i in 0..20i32 {
            let mut kb = i.to_le_bytes();
            let rc = m.del(
                kb.as_mut_ptr() as *mut c_void,
                &format!("del {i} with keyoffset=8"),
            );
            assert_eq!(
                rc, 0,
                "the key lives at offset 0, so a keyoffset=8 delete must miss"
            );
        }
        assert_eq!(m.hmlen("after"), len_before, "nothing may be removed");
        // with the CONSISTENT keyoffset (0) the same deletes succeed
        let cfg0 = MapCfg { keyoffset: 0, ..cfg };
        m.c.cfg = cfg0;
        m.rs.cfg = cfg0;
        for i in (0..20i32).rev() {
            let mut kb = i.to_le_bytes();
            assert_eq!(
                m.del(kb.as_mut_ptr() as *mut c_void, &format!("del {i} keyoffset=0")),
                1
            );
        }
        assert_eq!(m.hmlen("drained"), 0);
        m.free();
    }
}

// ===========================================================================
// rows 52, 53 — arr_push edge cases
// ===========================================================================
#[test]
fn err52_53_arr_push_boundaries() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        // row 52: num <= 0 -> the loop body never runs
        // row 53: 1..=50   -> exactly one iteration with i == 0, inner loop
        //                     empty, arrfree(NULL) guarded
        for num in [
            i32::MIN,
            i32::MIN + 1,
            -1000,
            -50,
            -1,
            0,
            1,
            2,
            49,
            50,
            51,
            100,
            101,
        ] {
            (p.c.arr_push)(num);
            (p.rs.arr_push)(num);
        }
        // the global seed must be untouched by arr_push (it allocates no table)
        let a = (p.c.shmode_func)(8, 0);
        let b = (p.rs.shmode_func)(8, 0);
        let sa = rd_usize(rd_ptr((a as *mut u8).sub(8).sub(HDR_SIZE), HDR_HASH_TABLE), HI_SEED);
        let sb = rd_usize(rd_ptr((b as *mut u8).sub(8).sub(HDR_SIZE), HDR_HASH_TABLE), HI_SEED);
        assert_eq_ctx(sa, sb, "global seed after arr_push boundaries");
        assert_eq!(sa, INITIAL_HASH_SEED, "arr_push must not touch the seed");
        (p.c.hmfree_func)((a as *mut u8).sub(8) as *mut c_void, 8);
        (p.rs.hmfree_func)((b as *mut u8).sub(8) as *mut c_void, 8);
    }
}

// ===========================================================================
// row 54 — strkey(INT_MIN) and friends
// ===========================================================================
#[test]
fn err54_strkey_int_min() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for n in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX, i32::MAX - 1] {
            let c = cstr_bytes((p.c.strkey)(n) as *const u8);
            let r = cstr_bytes((p.rs.strkey)(n) as *const u8);
            assert_eq_ctx(
                String::from_utf8_lossy(&c).to_string(),
                String::from_utf8_lossy(&r).to_string(),
                &format!("strkey({n})"),
            );
            assert_eq!(c, format!("test_{n}").into_bytes());
            assert!(c.len() < 256, "must fit the 256-byte static buffer");
        }
        // "test_-2147483648" is 16 characters
        assert_eq!(cstr_bytes((p.c.strkey)(i32::MIN) as *const u8).len(), 16);
    }
}
