//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through the exported `extern "C"` symbols of BOTH shared
//! libraries (loaded with `libloading`), never through the Rust crate directly.

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// §3 rows 1-8 — stbds_hash_bytes
// ===========================================================================

/// Compare `stbds_hash_bytes(buf, len, seed)` between both libraries.
fn hb(buf: &mut [u8], len: usize, seed: usize) -> usize {
    let [c, r] = both();
    let p = buf.as_mut_ptr() as *mut c_void;
    let a = unsafe { (c.hash_bytes)(p, len, seed) };
    let b = unsafe { (r.hash_bytes)(p, len, seed) };
    assert_eq!(
        a, b,
        "hash_bytes(len={len}, seed={seed:#x}, bytes={:02x?}) C={a:#x} Rust={b:#x}",
        &buf[..len.min(buf.len())]
    );
    a
}

const SEEDS: [usize; 5] = [0, 1, DEFAULT_SEED, usize::MAX, 0xdead_beef_cafe_babe];

#[test]
fn cfg_hash_bytes_len0() {
    // Row 1: len == 0 and p == NULL — `p` is never dereferenced.
    let [c, r] = both();
    let mut rng = Rng::new(0x1001);
    let mut seeds: Vec<usize> = SEEDS.to_vec();
    for _ in 0..64 {
        seeds.push(rng.next_usize());
    }
    for &s in &seeds {
        let a = unsafe { (c.hash_bytes)(std::ptr::null_mut(), 0, s) };
        let b = unsafe { (r.hash_bytes)(std::ptr::null_mut(), 0, s) };
        assert_eq!(a, b, "hash_bytes(NULL,0,{s:#x})");
    }
    // ... and with a real (untouched) buffer.
    let mut buf = vec![0xAAu8; 64];
    for &s in &seeds {
        hb(&mut buf, 0, s);
    }
}

#[test]
fn cfg_hash_bytes_tail_only() {
    // Row 2: len 1..=7 — the `switch (len - i)` fall-through cases.
    let mut rng = Rng::new(0x1002);
    for len in 1..=7usize {
        for _ in 0..64 {
            let mut buf = rng.bytes(64);
            for &s in &SEEDS {
                hb(&mut buf, len, s);
            }
            hb(&mut buf, len, rng.next_usize());
        }
    }
}

#[test]
fn cfg_hash_bytes_one_block() {
    // Row 3: len == 8 — one full block, empty tail (`case 0`).
    let mut rng = Rng::new(0x1003);
    for _ in 0..64 {
        let mut buf = rng.bytes(64);
        for &s in &SEEDS {
            hb(&mut buf, 8, s);
        }
    }
}

#[test]
fn cfg_hash_bytes_block_plus_tail() {
    // Row 4: len 9..=15 — one block plus a 1..7 byte tail.
    let mut rng = Rng::new(0x1004);
    for len in 9..=15usize {
        for _ in 0..64 {
            let mut buf = rng.bytes(64);
            for &s in &SEEDS {
                hb(&mut buf, len, s);
            }
        }
    }
}

#[test]
fn cfg_hash_bytes_multiblock() {
    // Row 5: len 16..=64 — several blocks.
    let mut rng = Rng::new(0x1005);
    for len in 16..=64usize {
        for _ in 0..8 {
            let mut buf = rng.bytes(80);
            for &s in &SEEDS {
                hb(&mut buf, len, s);
            }
        }
    }
}

#[test]
fn cfg_hash_bytes_len_gt_255() {
    // Row 6: `data = len << (64-8)` keeps only `len & 0xff`.
    let mut rng = Rng::new(0x1006);
    for &len in &[255usize, 256, 257, 511, 512, 1024, 4096, 65536] {
        for _ in 0..16 {
            let mut buf = rng.bytes(len + 8);
            for &s in &SEEDS {
                hb(&mut buf, len, s);
            }
        }
    }
}

#[test]
fn cfg_hash_bytes_sign_extension() {
    // Row 7: `d[3] << 24` / `d[7] << 24` are `int` shifts that sign-extend into
    // the upper 32 bits of the `size_t`.  Force the high bit both ways.
    let mut rng = Rng::new(0x1007);
    for pattern in 0..3 {
        for _ in 0..32 {
            let mut buf: Vec<u8> = match pattern {
                0 => vec![0x00; 64],
                1 => vec![0xFF; 64],
                _ => rng.bytes(64),
            };
            if pattern == 2 {
                // guarantee the sign bit in every 4th byte
                for i in 0..buf.len() {
                    if i % 4 == 3 {
                        buf[i] |= 0x80;
                    }
                }
            }
            for len in 0..=32usize {
                for &s in &SEEDS {
                    hb(&mut buf, len, s);
                }
            }
        }
    }
}

#[test]
fn cfg_hash_bytes_seed_sweep() {
    // Row 8: seed sweep with random lengths and contents.
    let mut rng = Rng::new(0x1008);
    for _ in 0..2000 {
        let len = rng.below(72);
        let mut buf = rng.bytes(80);
        let seed = match rng.below(6) {
            0 => 0,
            1 => 1,
            2 => DEFAULT_SEED,
            3 => usize::MAX,
            4 => usize::MAX - 1,
            _ => rng.next_usize(),
        };
        hb(&mut buf, len, seed);
    }
}

// ===========================================================================
// §3 rows 9-12 — stbds_hash_string
// ===========================================================================

fn hs(bytes: &[u8], seed: usize) -> usize {
    let [c, r] = both();
    let mut v = bytes.to_vec();
    v.push(0);
    let p = v.as_mut_ptr() as *mut c_char;
    let a = unsafe { (c.hash_string)(p, seed) };
    let b = unsafe { (r.hash_string)(p, seed) };
    assert_eq!(
        a, b,
        "hash_string(len={}, seed={seed:#x}) C={a:#x} Rust={b:#x}",
        bytes.len()
    );
    a
}

#[test]
fn cfg_hash_string_empty() {
    // Row 9: "" — the accumulation loop never runs.
    let mut rng = Rng::new(0x1009);
    for &s in &SEEDS {
        hs(b"", s);
    }
    for _ in 0..64 {
        hs(b"", rng.next_usize());
    }
}

#[test]
fn cfg_hash_string_short() {
    // Row 10: ASCII bodies of length 1..=32.
    let mut rng = Rng::new(0x100A);
    for len in 1..=32usize {
        for _ in 0..32 {
            let body = rng.ascii(len);
            for &s in &SEEDS {
                hs(&body, s);
            }
        }
    }
}

#[test]
fn cfg_hash_string_long() {
    // Row 11: long bodies — many ROTATE_LEFT(hash,9) rounds.
    let mut rng = Rng::new(0x100B);
    for &len in &[100usize, 1000, 4096] {
        for _ in 0..16 {
            let body = rng.ascii(len);
            for &s in &SEEDS {
                hs(&body, s);
            }
        }
    }
}

#[test]
fn cfg_hash_string_high_bytes() {
    // Row 12: bytes >= 0x80 exercise the `(unsigned char) *str` cast.
    let mut rng = Rng::new(0x100C);
    for _ in 0..64 {
        let len = rng.range(1, 40);
        let body: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u64() as u8 & 0x7f)).collect();
        for &s in &SEEDS {
            hs(&body, s);
        }
    }
    // every single high byte on its own
    for b in 0x80u8..=0xFF {
        for &s in &SEEDS {
            hs(&[b], s);
            hs(&[b, b], s);
        }
    }
}

// ===========================================================================
// §3 rows 13-16 — stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

#[derive(Debug, PartialEq, Eq)]
struct ArrObs {
    is_null: bool,
    length: usize,
    capacity: usize,
    hash_table_null: bool,
    temp: isize,
    data: Vec<u8>,
}

/// NOTE: whether `realloc` happens to return the *same* block is an allocator
/// artifact and is deliberately not part of `ArrObs`.  The one case where the C
/// guarantees pointer identity (`min_cap <= arrcap`, L286-287) is asserted
/// explicitly by the caller.
unsafe fn arr_obs(a: *mut c_void, elemsize: usize, data_len: usize) -> ArrObs {
    if a.is_null() {
        return ArrObs {
            is_null: true,
            length: 0,
            capacity: 0,
            hash_table_null: true,
            temp: 0,
            data: Vec::new(),
        };
    }
    let h = header(a);
    ArrObs {
        is_null: false,
        length: (*h).length,
        capacity: (*h).capacity,
        hash_table_null: (*h).hash_table.is_null(),
        temp: (*h).temp,
        data: std::slice::from_raw_parts(a as *const u8, data_len.min(elemsize * (*h).capacity))
            .to_vec(),
    }
}

#[test]
fn cfg_arrgrowf_fresh_matrix() {
    // Row 13: a == NULL, cross product of elemsize x addlen x min_cap.
    let [c, r] = both();
    for &elemsize in &[1usize, 4, 7, 8, 12, 16, 24, 32] {
        for &addlen in &[0usize, 1, 2, 5] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 7, 8, 100] {
                let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let oc = unsafe { arr_obs(ac, elemsize, 0) };
                let or_ = unsafe { arr_obs(ar, elemsize, 0) };
                assert_eq!(
                    oc, or_,
                    "arrgrowf(NULL, elemsize={elemsize}, addlen={addlen}, min_cap={min_cap})"
                );
                if addlen == 0 && min_cap == 0 {
                    // C L280-287: min_len == min_cap == 0 and arrcap(NULL) == 0,
                    // so `min_cap <= arrcap` hits and NULL is returned verbatim
                    // *without* allocating.  Freeing it would be UB.
                    assert!(ac.is_null(), "C arrgrowf(NULL,_,0,0) must return NULL");
                    assert!(ar.is_null(), "Rust arrgrowf(NULL,_,0,0) must return NULL");
                    continue;
                }
                assert!(!ac.is_null() && !ar.is_null());
                unsafe { (c.arrfreef)(ac) };
                unsafe { (r.arrfreef)(ar) };
            }
        }
    }
}

#[test]
fn cfg_arrgrowf_repeated_doubling() {
    // Row 14: keep growing an existing array; the header (length / hash_table /
    // temp) and the payload bytes must survive every realloc identically.
    let [c, r] = both();
    let mut rng = Rng::new(0x2001);
    for &elemsize in &[1usize, 4, 7, 8, 12, 16, 24, 32] {
        let mut ac: *mut c_void = std::ptr::null_mut();
        let mut ar: *mut c_void = std::ptr::null_mut();
        let mut len = 0usize;
        let mut payload: Vec<u8> = Vec::new();
        for step in 0..20 {
            // avoid (addlen,min_cap) == (0,0) on an empty array: that returns
            // NULL verbatim (covered by ERRORS.md row 3) and there would be
            // nothing to grow.
            let addlen = rng.range(1, 4);
            let min_cap = rng.below(40);
            ac = unsafe { (c.arrgrowf)(ac, elemsize, addlen, min_cap) };
            ar = unsafe { (r.arrgrowf)(ar, elemsize, addlen, min_cap) };
            let oc = unsafe { arr_obs(ac, elemsize, payload.len()) };
            let or_ = unsafe { arr_obs(ar, elemsize, payload.len()) };
            assert_eq!(
                oc, or_,
                "elemsize={elemsize} step={step} addlen={addlen} min_cap={min_cap}"
            );
            // Simulate a consumer that appends `addlen` elements and fills them.
            let cap = unsafe { (*header(ac)).capacity };
            len = (len + addlen).min(cap);
            payload = rng.bytes(len * elemsize);
            unsafe {
                (*header(ac)).length = len;
                (*header(ar)).length = len;
                std::ptr::copy_nonoverlapping(payload.as_ptr(), ac as *mut u8, payload.len());
                std::ptr::copy_nonoverlapping(payload.as_ptr(), ar as *mut u8, payload.len());
            }
        }
        unsafe { (c.arrfreef)(ac) };
        unsafe { (r.arrfreef)(ar) };
    }
}

#[test]
fn cfg_arrgrowf_boundaries() {
    // Row 15: min_cap == cap-1 / cap / cap+1 around an existing array.
    let [c, r] = both();
    for &elemsize in &[1usize, 8, 16, 24, 32] {
        for &delta in &[-1i64, 0, 1] {
            let mut ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10) };
            let mut ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10) };
            let cap = unsafe { (*header(ac)).capacity };
            assert_eq!(cap, unsafe { (*header(ar)).capacity });
            let target = (cap as i64 + delta).max(0) as usize;
            let bc = ac;
            let br = ar;
            ac = unsafe { (c.arrgrowf)(ac, elemsize, 0, target) };
            ar = unsafe { (r.arrgrowf)(ar, elemsize, 0, target) };
            let oc = unsafe { arr_obs(ac, elemsize, 0) };
            let or_ = unsafe { arr_obs(ar, elemsize, 0) };
            assert_eq!(oc, or_, "elemsize={elemsize} cap={cap} target={target}");
            // identity return when nothing has to grow (C lib.c L286-287)
            if target <= cap {
                assert_eq!(ac, bc, "C must return the same pointer for min_cap<=cap");
                assert_eq!(ar, br, "Rust must return the same pointer for min_cap<=cap");
            }
            unsafe { (c.arrfreef)(ac) };
            unsafe { (r.arrfreef)(ar) };
        }
    }
}

#[test]
fn cfg_arrgrow_then_free() {
    // Row 16: full grow-then-free sequences (checks the alloc/free pairing).
    let [c, r] = both();
    let mut rng = Rng::new(0x2002);
    for _ in 0..32 {
        let elemsize = rng.range(1, 40);
        let mut ac: *mut c_void = std::ptr::null_mut();
        let mut ar: *mut c_void = std::ptr::null_mut();
        for _ in 0..rng.range(1, 8) {
            let addlen = rng.range(1, 8);
            let min_cap = rng.below(64);
            ac = unsafe { (c.arrgrowf)(ac, elemsize, addlen, min_cap) };
            ar = unsafe { (r.arrgrowf)(ar, elemsize, addlen, min_cap) };
            let oc = unsafe { arr_obs(ac, elemsize, 0) };
            let or_ = unsafe { arr_obs(ar, elemsize, 0) };
            assert_eq!(oc, or_, "elemsize={elemsize}");
        }
        unsafe { (c.arrfreef)(ac) };
        unsafe { (r.arrfreef)(ar) };
    }
}

// ===========================================================================
// §3 rows 17-24 — stbds_stralloc / stbds_strreset
// ===========================================================================

#[derive(Debug, PartialEq, Eq)]
struct ArenaObs {
    remaining: usize,
    block: u8,
    mode: u8,
    block_count: usize,
    ret_null: bool,
    content: Vec<u8>,
    /// `p == &head->storage`  (the oversized-empty-arena path)
    at_head_start: bool,
    /// `p == &head->storage + remaining_after`  (the normal bump-allocate path)
    at_head_bump: bool,
    /// `p == &head->next->storage`  (the oversized-non-empty-arena path)
    at_second_start: bool,
}

unsafe fn arena_obs(a: &StringArena, p: *mut c_char) -> ArenaObs {
    let head_storage = if a.storage.is_null() {
        std::ptr::null_mut()
    } else {
        std::ptr::addr_of_mut!((*a.storage).storage) as *mut c_char
    };
    let second_storage = if a.storage.is_null() || (*a.storage).next.is_null() {
        std::ptr::null_mut()
    } else {
        std::ptr::addr_of_mut!((*(*a.storage).next).storage) as *mut c_char
    };
    ArenaObs {
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        block_count: count_blocks(a.storage),
        ret_null: p.is_null(),
        content: if p.is_null() { Vec::new() } else { cstr_bytes(p) },
        at_head_start: !p.is_null() && p == head_storage,
        at_head_bump: !p.is_null()
            && !head_storage.is_null()
            && p == head_storage.wrapping_add(a.remaining),
        at_second_start: !p.is_null() && p == second_storage,
    }
}

/// Run the identical `stralloc` on both libraries' own arenas and compare.
struct ArenaPair {
    a: [StringArena; 2],
}

impl ArenaPair {
    fn new() -> Self {
        ArenaPair {
            a: [StringArena::zeroed(), StringArena::zeroed()],
        }
    }
    fn alloc(&mut self, body: &[u8], ctx: &str) {
        let [c, r] = both();
        let mut buf = body.to_vec();
        buf.push(0);
        let p = buf.as_mut_ptr() as *mut c_char;
        let pc = unsafe { (c.stralloc)(&mut self.a[0], p) };
        let pr = unsafe { (r.stralloc)(&mut self.a[1], p) };
        let oc = unsafe { arena_obs(&self.a[0], pc) };
        let or_ = unsafe { arena_obs(&self.a[1], pr) };
        assert_eq!(oc, or_, "stralloc len={} {ctx}", body.len() + 1);
        assert_eq!(oc.content, body, "stralloc content {ctx}");
    }
    fn reset(&mut self, ctx: &str) {
        let [c, r] = both();
        unsafe { (c.strreset)(&mut self.a[0]) };
        unsafe { (r.strreset)(&mut self.a[1]) };
        let oc = unsafe { arena_obs(&self.a[0], std::ptr::null_mut()) };
        let or_ = unsafe { arena_obs(&self.a[1], std::ptr::null_mut()) };
        assert_eq!(oc, or_, "strreset {ctx}");
        assert_eq!(oc.remaining, 0);
        assert_eq!(oc.block, 0);
        assert_eq!(oc.mode, 0);
        assert_eq!(oc.block_count, 0);
    }
}

impl Drop for ArenaPair {
    fn drop(&mut self) {
        let [c, r] = both();
        unsafe { (c.strreset)(&mut self.a[0]) };
        unsafe { (r.strreset)(&mut self.a[1]) };
    }
}

#[test]
fn cfg_stralloc_fresh_short() {
    // Row 17: fresh arena, a single string of length 0..=32.
    let mut rng = Rng::new(0x3001);
    for len in 0..=32usize {
        let mut ap = ArenaPair::new();
        let body = rng.ascii(len);
        ap.alloc(&body, &format!("fresh len={len}"));
    }
}

#[test]
fn cfg_stralloc_fresh_blocksize_boundary() {
    // Row 18: crosses the initial blocksize (512).
    let mut rng = Rng::new(0x3002);
    for &len in &[509usize, 510, 511, 512, 513, 1022, 1023, 1024, 1025] {
        let mut ap = ArenaPair::new();
        let body = rng.ascii(len);
        ap.alloc(&body, &format!("boundary len={len}"));
    }
}

#[test]
fn cfg_stralloc_sequence_random() {
    // Row 19: long sequences of short strings — drives `remaining` down and
    // `block` up through several block allocations.
    for seed in 0..8u64 {
        let mut rng = Rng::new(0x3100 + seed);
        let mut ap = ArenaPair::new();
        for i in 0..200 {
            let len = rng.range(0, 60);
            let body = rng.ascii(len);
            ap.alloc(&body, &format!("seq seed={seed} i={i} len={len}"));
        }
    }
}

#[test]
fn cfg_stralloc_oversize_on_empty() {
    // Row 20: len > blocksize with storage == NULL -> the dedicated block
    // becomes the head and `remaining` is forced to 0.
    let mut rng = Rng::new(0x3003);
    for &len in &[513usize, 600, 1024, 4096, 100_000] {
        let mut ap = ArenaPair::new();
        let body = rng.ascii(len);
        ap.alloc(&body, &format!("oversize-empty len={len}"));
        assert!(
            unsafe { arena_obs(&ap.a[0], std::ptr::null_mut()) }.remaining == 0,
            "remaining must be forced to 0"
        );
        // a following small alloc must take the new-block path again
        let b2 = rng.ascii(8);
        ap.alloc(&b2, &format!("after oversize-empty len={len}"));
    }
}

#[test]
fn cfg_stralloc_oversize_on_nonempty() {
    // Row 21: len > blocksize with storage != NULL -> spliced in as
    // storage->next and `remaining` is preserved.
    let mut rng = Rng::new(0x3004);
    for &len in &[513usize, 600, 2048, 100_000] {
        let mut ap = ArenaPair::new();
        ap.alloc(&rng.ascii(4), "seed the arena");
        ap.alloc(&rng.ascii(len), &format!("oversize-nonempty len={len}"));
        ap.alloc(&rng.ascii(4), "after oversize-nonempty");
    }
}

#[test]
fn cfg_stralloc_block_presets() {
    // Row 22: pre-set a->block so `512 << (block>>1)` covers the whole range
    // including the 1<<20 saturation point and the shift-count wrap.
    // NOTE: `block` values whose `512 << (block>>1)` lands between 4 MiB and
    // 2^63 make the C `realloc` fail and then dereference NULL — that is a
    // crash row (`err_stralloc_block_realloc_fail_segv`, ERRORS.md #67b) and is
    // tested in a child process instead.  `block == 255` is safe: the shift
    // count wraps to 63, `512 << 63` is 0, so the dedicated-block path is taken.
    let mut rng = Rng::new(0x3005);
    for &blk in &[0u8, 1, 2, 3, 4, 5, 6, 7, 10, 20, 21, 22, 23, 24, 25, 255] {
        for &len in &[1usize, 700, 3000] {
            let mut ap = ArenaPair::new();
            ap.a[0].block = blk;
            ap.a[1].block = blk;
            let body = rng.ascii(len);
            ap.alloc(&body, &format!("block={blk} len={len}"));
            ap.alloc(&rng.ascii(16), &format!("second block={blk} len={len}"));
        }
    }
}

#[test]
fn cfg_stralloc_saturate_block() {
    // Row 23: enough allocations to walk `block` all the way to saturation.
    let mut rng = Rng::new(0x3006);
    let mut ap = ArenaPair::new();
    for i in 0..3000 {
        // mostly short, occasionally huge so new blocks keep being needed
        let len = if i % 7 == 0 { rng.range(400, 900) } else { rng.range(1, 40) };
        let body = rng.ascii(len);
        ap.alloc(&body, &format!("saturate i={i} len={len}"));
    }
    let obs = unsafe { arena_obs(&ap.a[0], std::ptr::null_mut()) };
    assert!(obs.block >= 4, "block should have grown, got {}", obs.block);
}

#[test]
fn cfg_strreset_chain() {
    // Row 24: strreset over chains of 0/1/2/5/50 blocks, then reuse the arena.
    let mut rng = Rng::new(0x3007);
    for &nblocks in &[0usize, 1, 2, 5, 50] {
        let mut ap = ArenaPair::new();
        for i in 0..nblocks {
            // alternate normal and oversized blocks
            let len = if i % 3 == 2 { 2000 } else { 600 };
            ap.alloc(&rng.ascii(len), &format!("chain n={nblocks} i={i}"));
        }
        ap.reset(&format!("chain n={nblocks}"));
        // reusable afterwards
        ap.alloc(&rng.ascii(10), &format!("reuse after n={nblocks}"));
        ap.reset(&format!("second reset n={nblocks}"));
    }
}

// ===========================================================================
// §3 rows 73-77 — strkey / str_dups
// ===========================================================================

#[test]
fn cfg_strkey_values() {
    // Row 73
    let [c, r] = both();
    let mut rng = Rng::new(0x4001);
    let mut vals: Vec<c_int> = vec![0, 1, -1, 9, 10, 99, 100, i32::MAX, i32::MIN, -99, 12345];
    for _ in 0..64 {
        vals.push(rng.next_u32() as i32);
    }
    for n in vals {
        let a = unsafe { cstr_bytes((c.strkey)(n)) };
        let b = unsafe { cstr_bytes((r.strkey)(n)) };
        assert_eq!(
            a,
            b,
            "strkey({n}) C={:?} Rust={:?}",
            String::from_utf8_lossy(&a),
            String::from_utf8_lossy(&b)
        );
        assert_eq!(a, format!("test_{n}").into_bytes());
    }
}

#[test]
fn cfg_strkey_static_buffer() {
    // Row 74: the returned pointer is the same static buffer every time, so the
    // second call overwrites the first result.
    let [c, r] = both();
    let mut rng = Rng::new(0x4002);
    for _ in 0..16 {
        let n1 = rng.next_u32() as i32;
        let n2 = rng.next_u32() as i32;
        let pc1 = unsafe { (c.strkey)(n1) };
        let pr1 = unsafe { (r.strkey)(n1) };
        let pc2 = unsafe { (c.strkey)(n2) };
        let pr2 = unsafe { (r.strkey)(n2) };
        assert_eq!(pc1, pc2, "C strkey must reuse one static buffer");
        assert_eq!(pr1, pr2, "Rust strkey must reuse one static buffer");
        let a = unsafe { cstr_bytes(pc1) };
        let b = unsafe { cstr_bytes(pr1) };
        assert_eq!(a, b);
        assert_eq!(a, format!("test_{n2}").into_bytes());
    }
}

fn str_dups_stdout(num: c_int) {
    let [c, r] = both();
    // Keep the two libraries' global `stbds_hash_seed` in lockstep.
    seed_both(DEFAULT_SEED);
    let oc = capture_stdout(&format!("c{num}"), || unsafe { (c.str_dups)(num) });
    seed_both(DEFAULT_SEED);
    let or_ = capture_stdout(&format!("r{num}"), || unsafe { (r.str_dups)(num) });
    assert_eq!(
        oc,
        or_,
        "str_dups({num}) stdout differs:\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(&oc),
        String::from_utf8_lossy(&or_)
    );
    assert_eq!(
        oc,
        format!("a {num}\n").into_bytes(),
        "str_dups({num}) unexpected output {:?}",
        String::from_utf8_lossy(&oc)
    );
}

#[test]
fn cfg_str_dups_stdout() {
    // Row 75
    for &num in &[0i32, 1, 2, 3, 10, 29, 30, 31, 32, 64, 100, 512, 1000] {
        str_dups_stdout(num);
    }
}

#[test]
fn cfg_str_dups_non_positive() {
    // Row 76: the arena loop is skipped entirely.
    for &num in &[-1i32, -2, -1000, i32::MIN] {
        str_dups_stdout(num);
    }
}

#[test]
fn cfg_str_dups_repeated() {
    // Row 77: called repeatedly without re-seeding — the global hash seed
    // advances once per call and both libraries must stay in step.
    let [c, r] = both();
    seed_both(DEFAULT_SEED);
    let mut co = Vec::new();
    for i in 0..10 {
        co.push(capture_stdout(&format!("rep-c{i}"), || unsafe { (c.str_dups)(i + 1) }));
    }
    seed_both(DEFAULT_SEED);
    let mut ro = Vec::new();
    for i in 0..10 {
        ro.push(capture_stdout(&format!("rep-r{i}"), || unsafe { (r.str_dups)(i + 1) }));
    }
    assert_eq!(co, ro, "repeated str_dups output diverges");
}
