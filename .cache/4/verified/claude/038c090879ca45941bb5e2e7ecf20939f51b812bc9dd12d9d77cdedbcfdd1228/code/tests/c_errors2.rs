//! Phase C (continued) — `ERRORS.md` rows 22, 25, 27, 28, 30..32, 34..39,
//! 41..43, 46, 47, 49: the non-fatal rejection / edge-value rows.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

unsafe fn header_of(handle: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    ((handle as *mut u8).sub(elemsize) as *mut ArrayHeader).sub(1)
}

unsafe fn table_of(handle: *mut c_void, elemsize: usize) -> *mut HashIndex {
    (*header_of(handle, elemsize)).hash_table as *mut HashIndex
}

// ===========================================================================
// #22 — STBDS_ASSERT(table->used_count >= 0) is a tautology for size_t
// ===========================================================================

/// The C asserts `table->used_count >= 0` *after* `--table->used_count`.
/// `used_count` is a `size_t`, so the condition can never be false: with a
/// forged `used_count == 0` the decrement wraps to `SIZE_MAX` and the assert
/// still passes. The Rust must wrap identically (and must not panic).
#[test]
fn err_22_used_count_underflow_is_not_an_error() {
    let p = Pair::new();
    let elemsize = 8usize;
    p.seed(0x3141_5926);
    let mut m = MapPair::null(elemsize, 4, STBDS_HM_BINARY, KeyKind::Binary);
    for i in 1..=2u32 {
        let mut k = i.to_le_bytes();
        m.put(&p, &mut k, &(!i).to_le_bytes());
    }
    m.check("before forging used_count");
    unsafe {
        (*table_of(m.hc, elemsize)).used_count = 0;
        (*table_of(m.hr, elemsize)).used_count = 0;
    }
    // delete the LAST element so no memmove/re-index happens
    let mut k = 2u32.to_le_bytes();
    assert_eq!(m.del(&p, &mut k, 0), 1, "delete must still succeed");
    m.check("after used_count underflow");
    let s = m.snaps().0;
    assert_eq!(
        s.used_count,
        usize::MAX,
        "size_t decrement from 0 must wrap to SIZE_MAX"
    );
    // the map is still usable
    let mut k1 = 1u32.to_le_bytes();
    assert!(m.get(&p, &mut k1) >= 0);
    m.check("usable after underflow");
    m.free(&p);
}

// ===========================================================================
// #25 — STBDS_ASSERT(len <= a->remaining) is unreachable for a self-consistent
//       arena: hammer the boundary to prove it never fires on either side
// ===========================================================================

#[test]
fn err_25_stralloc_remaining_assert_unreachable() {
    let p = Pair::new();
    let mut rng = Rng::new(0xE7_25);
    for round in 0..6 {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        // walk lengths straight across every blocksize boundary
        let mut lens: Vec<usize> = Vec::new();
        for b in 0..6u32 {
            let bs = 512usize << b;
            for d in [-2i64, -1, 0, 1, 2] {
                let l = (bs as i64 + d) as usize;
                if l >= 1 {
                    lens.push(l);
                }
            }
        }
        for _ in 0..200 {
            lens.push(1 + rng.below(3000));
        }
        for (i, &len) in lens.iter().enumerate() {
            let s = vec![b'k'; len - 1]; // +1 NUL => request is exactly `len`
            let mut buf = s.clone();
            buf.push(0);
            unsafe {
                let pc = (p.c.stralloc)(&mut ac as *mut Arena, buf.as_mut_ptr() as *mut c_char);
                let pr = (p.r.stralloc)(&mut ar as *mut Arena, buf.as_mut_ptr() as *mut c_char);
                assert_eq!(cstr_bytes(pc), s, "round {round} alloc {i}: C content");
                assert_eq!(cstr_bytes(pr), s, "round {round} alloc {i}: Rust content");
                assert_eq!(
                    snap_arena(&ac),
                    snap_arena(&ar),
                    "round {round} alloc {i}: arena state diverged"
                );
            }
        }
        unsafe {
            (p.c.strreset)(&mut ac as *mut Arena);
            (p.r.strreset)(&mut ar as *mut Arena);
        }
    }
}

// ===========================================================================
// #27, #28 — the `512u << (block>>1)` shift and the MAX cap
// ===========================================================================

/// #27 — a forged `block` whose shift count is >= 64 (UB in C; on x86-64 gcc
/// emits `shl`, whose count is taken mod 64 — which is what Rust's
/// `wrapping_shl` does too).
///
/// Only `block` values whose *wrapped* `blocksize` is small are used:
/// `blocksize == 512 << ((block>>1) % 64) == 2^(9 + (block>>1)%64)`, so
/// `(block>>1)%64 >= 55` wraps the whole value to **0** and
/// `(block>>1)%64 <= 12` keeps it at most 2 MiB. Anything in between asks
/// `realloc` for 2 GiB … 8 EiB, which fails in *both* libraries and simply
/// dereferences NULL — that outcome is covered by `ERRORS.md` #50 instead.
#[test]
fn err_27_stralloc_block_ub_shift() {
    let p = Pair::new();
    // (block>>1) % 64 == 0..2  -> 512 B / 1 KiB / 2 KiB
    // (block>>1) % 64 == 55..63 -> 2^64 == 0  -> the oversized-block branch
    for block in [
        110u8, 111, 112, 118, 124, 126, 127, // shift 55..63 -> blocksize 0
        128, 129, // shift 64 -> 0 -> 512
        130, 131, // shift 65 -> 1 -> 1 KiB
        132, 133, // shift 66 -> 2 -> 2 KiB
        250, 251, 252, 253, 254, 255, // shift 125..127 -> 61..63 -> blocksize 0
    ] {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        ac.block = block;
        ar.block = block;
        let mut buf = b"payload\0".to_vec();
        unsafe {
            let pc = (p.c.stralloc)(&mut ac as *mut Arena, buf.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar as *mut Arena, buf.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(pc), b"payload".to_vec(), "block={block} C content");
            assert_eq!(cstr_bytes(pr), b"payload".to_vec(), "block={block} Rust content");
            assert_eq!(
                snap_arena(&ac),
                snap_arena(&ar),
                "block={block}: arena state diverged"
            );
            (p.c.strreset)(&mut ac as *mut Arena);
            (p.r.strreset)(&mut ar as *mut Arena);
        }
    }
}

/// #28 — `++a->block` is skipped once `512 << (block>>1) >= 1<<20`
/// (i.e. `block >= 22`).
#[test]
fn err_28_stralloc_block_saturation() {
    let p = Pair::new();
    for block in 20u8..=25 {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        ac.block = block;
        ar.block = block;
        let mut buf = b"x\0".to_vec();
        unsafe {
            (p.c.stralloc)(&mut ac as *mut Arena, buf.as_mut_ptr() as *mut c_char);
            (p.r.stralloc)(&mut ar as *mut Arena, buf.as_mut_ptr() as *mut c_char);
            let (sc, sr) = (snap_arena(&ac), snap_arena(&ar));
            assert_eq!(sc, sr, "block={block}: diverged");
            let blocksize = 512usize << (block >> 1);
            if blocksize < (1 << 20) {
                assert_eq!(sc.block, block + 1, "block={block} must advance");
            } else {
                assert_eq!(sc.block, block, "block={block} must saturate");
            }
            // repeated allocations at the saturated value keep the same blocksize
            for _ in 0..3 {
                let mut big = vec![b'B'; blocksize.min(1 << 20) + 4];
                big.push(0);
                (p.c.stralloc)(&mut ac as *mut Arena, big.as_mut_ptr() as *mut c_char);
                (p.r.stralloc)(&mut ar as *mut Arena, big.as_mut_ptr() as *mut c_char);
                assert_eq!(snap_arena(&ac), snap_arena(&ar), "block={block} repeat");
            }
            (p.c.strreset)(&mut ac as *mut Arena);
            (p.r.strreset)(&mut ar as *mut Arena);
        }
    }
}

/// #30 — `stbds_strreset` on an already-empty arena still runs the `memset`.
#[test]
fn err_30_strreset_empty() {
    let p = Pair::new();
    for &(remaining, block, mode) in &[
        (0usize, 0u8, 0u8),
        (12345, 7, 3),
        (usize::MAX, 255, 255),
    ] {
        let mut ac = Arena {
            storage: std::ptr::null_mut(),
            remaining,
            block,
            mode,
        };
        let mut ar = Arena {
            storage: std::ptr::null_mut(),
            remaining,
            block,
            mode,
        };
        unsafe {
            (p.c.strreset)(&mut ac as *mut Arena);
            (p.r.strreset)(&mut ar as *mut Arena);
            let (sc, sr) = (snap_arena(&ac), snap_arena(&ar));
            assert_eq!(sc, sr, "strreset on an empty arena diverged");
            assert_eq!(
                (sc.remaining, sc.block, sc.mode, sc.storage_is_null),
                (0, 0, 0, true),
                "strreset must fully zero the arena"
            );
        }
    }
}

// ===========================================================================
// #31, #32 — zero-length hash inputs
// ===========================================================================

/// #31 — `stbds_hash_bytes(NULL, 0, seed)`.
#[test]
fn err_31_hash_bytes_null_zero() {
    let p = Pair::new();
    let mut rng = Rng::new(0xE7_31);
    let mut seeds: Vec<usize> = vec![0, 1, 2, 0x3141_5926, usize::MAX, 1 << 63];
    for _ in 0..512 {
        seeds.push(rng.next_u64() as usize);
    }
    for s in seeds {
        let (hc, hr) = unsafe {
            (
                (p.c.hash_bytes)(std::ptr::null_mut(), 0, s),
                (p.r.hash_bytes)(std::ptr::null_mut(), 0, s),
            )
        };
        assert_eq!(hc, hr, "hash_bytes(NULL, 0, {s:#x}) diverged");
    }
}

/// #32 — `stbds_hash_string("")`.
#[test]
fn err_32_hash_string_empty() {
    let p = Pair::new();
    let mut rng = Rng::new(0xE7_32);
    let mut seeds: Vec<usize> = vec![0, 1, 2, 0x3141_5926, usize::MAX, 1 << 63];
    for _ in 0..512 {
        seeds.push(rng.next_u64() as usize);
    }
    let mut empty = [0u8; 1];
    for s in seeds {
        let (hc, hr) = unsafe {
            (
                (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, s),
                (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, s),
            )
        };
        assert_eq!(hc, hr, "hash_string(\"\", {s:#x}) diverged");
    }
}

// ===========================================================================
// #34..#37 — out-of-range enum values crossing the FFI boundary
// ===========================================================================

/// #34 — every `mode` below `STBDS_HM_STRING` must behave exactly like `0`.
#[test]
fn err_34_mode_negative() {
    let p = Pair::new();
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut traces: Vec<(c_int, Vec<Snap>)> = Vec::new();
    for &mode in &[0 as c_int, -1, -2, -1000, c_int::MIN, c_int::MIN + 1] {
        p.seed(0x1234_5678);
        let mut rng = Rng::new(0xE7_34);
        let mut m = MapPair::null(elemsize, keysize, mode, KeyKind::Binary);
        let mut trace = Vec::new();
        let mut live: Vec<u64> = Vec::new();
        for step in 0..300u64 {
            let v = if live.is_empty() || rng.next_u64() % 3 != 0 {
                let v = rng.next_u64() & 0xFFFF;
                let mut k = v.to_le_bytes().to_vec();
                m.put(&p, &mut k, &step.to_le_bytes());
                live.push(v);
                v
            } else {
                let i = rng.below(live.len());
                let v = live.swap_remove(i);
                let mut k = v.to_le_bytes().to_vec();
                m.del(&p, &mut k, 0);
                v
            };
            let mut k = v.to_le_bytes().to_vec();
            m.get(&p, &mut k);
            m.get_ts(&p, &mut k);
            m.check(&format!("mode={mode} step {step}"));
            trace.push(m.snaps().0);
        }
        traces.push((mode, trace));
        m.free(&p);
    }
    let base = &traces[0].1;
    for (mode, tr) in traces.iter().skip(1) {
        assert_eq!(base.len(), tr.len());
        for (i, (a, b)) in base.iter().zip(tr.iter()).enumerate() {
            assert_eq!(a, b, "mode={mode} step {i} differs from mode=0");
        }
    }
}

/// #35 — every `mode` at or above `STBDS_HM_STRING` takes the string path in
/// put/get; `mode == 2..INT_MAX` must behave like `mode == 1` there.
#[test]
fn err_35_mode_above_string() {
    let p = Pair::new();
    let keysize = std::mem::size_of::<*mut c_char>();
    let kind = KeyKind::StringPtr { keyoffset: 0 };
    let mut traces: Vec<(c_int, Vec<Snap>)> = Vec::new();
    for &mode in &[1 as c_int, 2, 3, 4, 255, 65536, c_int::MAX] {
        p.seed(0x9ABC_DEF0);
        let mut bufs: Vec<Vec<u8>> = Vec::new();
        let mut m = MapPair::null(16, keysize, mode, kind);
        let mut trace = Vec::new();
        for i in 0..60usize {
            let mut kb = format!("s{i}_{}", "q".repeat(i % 13)).into_bytes();
            kb.push(0);
            bufs.push(kb);
            let kp = bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char;
            m.put_str(&p, kp, &(i as u64).to_le_bytes());
            m.check(&format!("mode={mode} put {i}"));
            trace.push(m.snaps().0);
        }
        for i in 0..60usize {
            let mut kb = format!("s{i}_{}", "q".repeat(i % 13)).into_bytes();
            kb.push(0);
            bufs.push(kb);
            let kp = bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char;
            assert!(m.get_str(&p, kp) >= 0, "mode={mode} get {i}");
            m.check(&format!("mode={mode} get {i}"));
            trace.push(m.snaps().0);
        }
        traces.push((mode, trace));
        m.free(&p);
    }
    let base = &traces[0].1;
    for (mode, tr) in traces.iter().skip(1) {
        for (i, (a, b)) in base.iter().zip(tr.iter()).enumerate() {
            assert_eq!(a, b, "mode={mode} step {i} differs from mode=1");
        }
    }
}

/// #36 — `stbds_hmdel_key` with `mode == 2` on a STRDUP string map: the
/// `mode == STBDS_HM_STRING` equality test is false, so the stored copy is NOT
/// freed. Only delete-last is exercised, so the (address-dependent) binary
/// re-index branch is never entered.
#[test]
fn err_36_del_mode2_string_map() {
    let p = Pair::new();
    let keysize = std::mem::size_of::<*mut c_char>();
    let kind = KeyKind::StringPtr { keyoffset: 0 };
    for &sh_mode in &[STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for &mode in &[2 as c_int, 3, c_int::MAX] {
            for &n in &[1usize, 2, 7, 13, 30] {
                p.seed(0x2222_3333);
                let mut bufs: Vec<Vec<u8>> = Vec::new();
                let mut m = MapPair::shmode(&p, 24, keysize, mode, sh_mode, kind);
                for i in 0..n {
                    let mut kb = format!("m2_{i}_{}", "r".repeat(i % 9)).into_bytes();
                    kb.push(0);
                    bufs.push(kb);
                    let kp = bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char;
                    let mut val = (i as u64).to_le_bytes().to_vec();
                    val.extend_from_slice(&(!(i as u64)).to_le_bytes());
                    m.put_str(&p, kp, &val);
                    m.check(&format!("sh={sh_mode} mode={mode} put {i}"));
                }
                // delete last -> old_index == final_index, no re-index probe
                for i in (0..n).rev() {
                    let mut kb = format!("m2_{i}_{}", "r".repeat(i % 9)).into_bytes();
                    kb.push(0);
                    let kp = kb.as_mut_ptr() as *mut c_char;
                    assert_eq!(
                        m.del_str(&p, kp, 0),
                        1,
                        "sh={sh_mode} mode={mode} delete-last {i}"
                    );
                    m.check(&format!("sh={sh_mode} mode={mode} del {i}"));
                }
                assert_eq!(m.snaps().0.length, 1);
                m.free(&p);
            }
        }
    }
}

/// #37a — `stbds_shmode_func` with a `mode` whose `(unsigned char)` truncation
/// is **not** one of `STBDS_SH_DEFAULT/STRDUP/ARENA`: `switch (string.mode)`
/// falls to `default:` and the key is stored with a binary `memcpy`.
#[test]
fn err_37a_shmode_out_of_range_default_branch() {
    let p = Pair::new();
    let elemsize = 16usize;
    for &mode in &[
        -1 as c_int,   // 0xFF -> 255
        -2,            // 0xFE -> 254
        4,
        5,
        127,
        128,
        200,
        255,
        256,           // -> 0 (STBDS_SH_NONE)
        512,           // -> 0
        1000,          // 0x3E8 -> 232
        65536,         // -> 0
        c_int::MAX,    // 0x7FFFFFFF -> 255
        c_int::MIN,    // 0x80000000 -> 0
    ] {
        let truncated = (mode as u32 & 0xFF) as u8;
        assert!(
            !(1..=3).contains(&truncated),
            "mode={mode} truncates to {truncated}, which is a valid storage mode"
        );
        p.seed(0x4444_5555);
        let mut m = MapPair::shmode(&p, elemsize, 8, STBDS_HM_BINARY, mode, KeyKind::Binary);
        m.check(&format!("shmode({mode}) fresh"));
        let s = m.snaps().0;
        assert_eq!(
            s.arena_mode, truncated,
            "string.mode must be `(unsigned char) mode`"
        );
        for i in 0..40u64 {
            let mut k = i.to_le_bytes().to_vec();
            m.put(&p, &mut k, &(i ^ 0xFF).to_le_bytes());
            m.check(&format!("shmode({mode}) put {i}"));
        }
        for i in 0..40u64 {
            let mut k = i.to_le_bytes().to_vec();
            assert!(m.get(&p, &mut k) >= 0, "shmode({mode}) get {i}");
            m.check(&format!("shmode({mode}) get {i}"));
        }
        for i in (0..40u64).rev() {
            let mut k = i.to_le_bytes().to_vec();
            assert_eq!(m.del(&p, &mut k, 0), 1);
            m.check(&format!("shmode({mode}) del {i}"));
        }
        m.free(&p);
    }
}

/// #37b — an out-of-range `mode` whose truncation lands on 1/2/3 must be
/// indistinguishable from passing that value directly.
#[test]
fn err_37b_shmode_out_of_range_aliases_valid_modes() {
    let p = Pair::new();
    let keysize = std::mem::size_of::<*mut c_char>();
    let kind = KeyKind::StringPtr { keyoffset: 0 };

    // (out-of-range value, the valid mode it truncates onto)
    for &(mode, alias) in &[
        (257 as c_int, STBDS_SH_DEFAULT),
        (0x1_0001, STBDS_SH_DEFAULT),
        (-255, STBDS_SH_DEFAULT), // 0xFFFFFF01 -> 1
        (258, STBDS_SH_STRDUP),
        (-254, STBDS_SH_STRDUP), // 0xFFFFFF02 -> 2
        (259, STBDS_SH_ARENA),
        (-253, STBDS_SH_ARENA), // 0xFFFFFF03 -> 3
    ] {
        assert_eq!((mode as u32 & 0xFF) as c_int, alias);
        let mut traces: Vec<(c_int, Vec<Snap>)> = Vec::new();
        for &m_in in &[alias, mode] {
            p.seed(0x6666_7777);
            let mut bufs: Vec<Vec<u8>> = Vec::new();
            let mut m = MapPair::shmode(&p, 16, keysize, STBDS_HM_STRING, m_in, kind);
            let s = m.snaps().0;
            assert_eq!(s.arena_mode, alias as u8);
            let mut trace = vec![s];
            for i in 0..40usize {
                let mut kb = format!("a{i}_{}", "t".repeat(i % 7)).into_bytes();
                kb.push(0);
                bufs.push(kb);
                let kp = bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char;
                m.put_str(&p, kp, &(i as u64).to_le_bytes());
                m.check(&format!("shmode({m_in}) put {i}"));
                trace.push(m.snaps().0);
            }
            for i in 0..40usize {
                let mut kb = format!("a{i}_{}", "t".repeat(i % 7)).into_bytes();
                kb.push(0);
                let kp = kb.as_mut_ptr() as *mut c_char;
                assert!(m.get_str(&p, kp) >= 0, "shmode({m_in}) get {i}");
                m.check(&format!("shmode({m_in}) get {i}"));
                trace.push(m.snaps().0);
            }
            for i in (0..40usize).rev() {
                let mut kb = format!("a{i}_{}", "t".repeat(i % 7)).into_bytes();
                kb.push(0);
                let kp = kb.as_mut_ptr() as *mut c_char;
                assert_eq!(m.del_str(&p, kp, 0), 1, "shmode({m_in}) del {i}");
                m.check(&format!("shmode({m_in}) del {i}"));
                trace.push(m.snaps().0);
            }
            traces.push((m_in, trace));
            m.free(&p);
        }
        let base = &traces[0].1;
        let other = &traces[1].1;
        assert_eq!(base.len(), other.len());
        for (i, (a, b)) in base.iter().zip(other.iter()).enumerate() {
            assert_eq!(
                a, b,
                "shmode({mode}) step {i} differs from shmode({alias})"
            );
        }
    }
}

// ===========================================================================
// #38, #39 — degenerate sizes
// ===========================================================================

/// #38 — `keysize == 0`: all keys hash the same and `memcmp(_,_,0) == 0`, so the
/// map collapses to one entry.
#[test]
fn err_38_keysize_zero() {
    let p = Pair::new();
    let mut rng = Rng::new(0xE7_38);
    for &elemsize in &[1usize, 4, 8, 16, 24] {
        for &gseed in &[0usize, 0x3141_5926, usize::MAX] {
            p.seed(gseed);
            let mut m = MapPair::null(elemsize, 0, STBDS_HM_BINARY, KeyKind::Binary);
            for i in 0..50u64 {
                let mut k = rng.bytes(16);
                m.put(&p, &mut k, &vec![(i & 0xFF) as u8; elemsize]);
                m.check(&format!("keysize=0 put {i}"));
                assert_eq!(m.snaps().0.length, 2, "must collapse to one element");
                assert_eq!(m.snaps().0.used_count, 1);
            }
            for _ in 0..20 {
                let mut k = rng.bytes(16);
                assert_eq!(m.get(&p, &mut k), 0, "keysize=0 always hits element 0");
                m.check("keysize=0 get");
                assert_eq!(m.get_ts(&p, &mut k), 0);
                m.check("keysize=0 get_ts");
            }
            let mut k = rng.bytes(16);
            assert_eq!(m.del(&p, &mut k, 0), 1, "keysize=0 delete");
            m.check("keysize=0 del");
            assert_eq!(m.del(&p, &mut k, 0), 0, "second keysize=0 delete is a no-op");
            m.check("keysize=0 second del");
            m.free(&p);
        }
    }
}

/// #39 — `elemsize == 0` (with `keysize == 0`, the only combination that stays
/// inside the allocation: `memcpy(dst, key, keysize)` writes `keysize` bytes into
/// a zero-capacity data region, so any `keysize > 0` is an out-of-bounds write by
/// construction and its behaviour is not well-defined for either language).
#[test]
fn err_39_elemsize_zero() {
    let p = Pair::new();
    let mut rng = Rng::new(0xE7_39);
    for &gseed in &[0usize, 1, 0x3141_5926, usize::MAX] {
        p.seed(gseed);
        let mut m = MapPair::null(0, 0, STBDS_HM_BINARY, KeyKind::Binary);
        for i in 0..30 {
            let mut k = rng.bytes(8);
            let t = m.put(&p, &mut k, &[]);
            assert_eq!(t, 0, "elemsize=0 always resolves to element 0");
            m.check(&format!("elemsize=0 put {i}"));
            let s = m.snaps().0;
            assert_eq!((s.length, s.used_count), (2, 1));
        }
        for _ in 0..10 {
            let mut k = rng.bytes(8);
            assert_eq!(m.get(&p, &mut k), 0);
            m.check("elemsize=0 get");
            assert_eq!(m.get_ts(&p, &mut k), 0);
            m.check("elemsize=0 get_ts");
        }
        let mut k = rng.bytes(8);
        assert_eq!(m.del(&p, &mut k, 0), 1);
        m.check("elemsize=0 del");
        assert_eq!(m.snaps().0.length, 1);
        assert_eq!(m.del(&p, &mut k, 0), 0);
        m.check("elemsize=0 del again");
        m.free(&p);
    }
    // stbds_hmput_default / hmget_key / arrgrowf with elemsize 0
    unsafe {
        let hc = (p.c.hmput_default)(std::ptr::null_mut(), 0);
        let hr = (p.r.hmput_default)(std::ptr::null_mut(), 0);
        let mut k = [0u8; 4];
        let bc = (p.c.hmget_key)(hc, 0, k.as_mut_ptr() as *mut c_void, 0, 0);
        let br = (p.r.hmget_key)(hr, 0, k.as_mut_ptr() as *mut c_void, 0, 0);
        assert_eq!((*header_of(bc, 0)).temp, (*header_of(br, 0)).temp);
        assert_eq!((*header_of(bc, 0)).length, (*header_of(br, 0)).length);
        (p.c.hmfree_func)(bc, 0);
        (p.r.hmfree_func)(br, 0);
    }
}

// ===========================================================================
// #41 — `if (hash < 2) hash += 2;`
// ===========================================================================

/// The guard is reachable through `stbds_hash_string`: for the empty string the
/// accumulator equals `seed`, so `hash ^= seed` makes it 0 and the result is
/// `F(0) + seed`. Choosing `seed = -F(0)` (resp. `1 - F(0)`) therefore produces
/// a raw hash of exactly `STBDS_HASH_EMPTY` (resp. `STBDS_HASH_DELETED`), which
/// the C bumps to 2 so it can never be confused with an empty/deleted slot.
#[test]
fn err_41_hash_below_two_is_bumped() {
    let p = Pair::new();
    let keysize = std::mem::size_of::<*mut c_char>();
    let kind = KeyKind::StringPtr { keyoffset: 0 };

    let mut empty = [0u8; 1];
    let f0 = unsafe { (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, 0) };
    let f0r = unsafe { (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, 0) };
    assert_eq!(f0, f0r, "hash_string(\"\", 0) diverged");

    for want in [0usize, 1] {
        let seed = want.wrapping_sub(f0);
        // sanity: this seed really does make the raw hash `want`
        let raw_c = unsafe { (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) };
        let raw_r = unsafe { (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed) };
        assert_eq!(raw_c, want, "seed construction failed for want={want}");
        assert_eq!(raw_r, want, "Rust hash_string diverged for want={want}");

        p.seed(0x3141_5926);
        let mut m = MapPair::shmode(&p, 16, keysize, STBDS_HM_STRING, STBDS_SH_STRDUP, kind);
        unsafe {
            (*table_of(m.hc, 16)).seed = seed;
            (*table_of(m.hr, 16)).seed = seed;
        }
        m.check(&format!("forged seed (want={want})"));

        // insert the empty-string key: hash 0/1 must be bumped to 2
        let mut kb = [0u8; 1];
        let t = m.put_str(&p, kb.as_mut_ptr() as *mut c_char, &7u64.to_le_bytes());
        assert_eq!(t, 0);
        m.check(&format!("empty key insert (want={want})"));
        let s = m.snaps().0;
        // `if (hash < 2) hash += 2;` => 0 becomes 2 and 1 becomes 3
        let bumped = want + 2;
        assert!(
            s.buckets[0].0.contains(&bumped),
            "the bumped hash {bumped} must be stored, buckets={:?}",
            s.buckets
        );
        assert!(
            !s.buckets[0]
                .0
                .iter()
                .enumerate()
                .any(|(i, &h)| h == want && s.buckets[0].1[i] >= 0),
            "the raw hash {want} (EMPTY/DELETED marker) must never be stored \
             for a live slot: buckets={:?}",
            s.buckets
        );

        // ...and it must be findable and deletable
        assert_eq!(m.get_str(&p, kb.as_mut_ptr() as *mut c_char), 0);
        m.check(&format!("empty key get (want={want})"));
        // add more keys so the bumped slot coexists with normal ones
        let mut bufs: Vec<Vec<u8>> = Vec::new();
        for i in 0..20usize {
            let mut b = format!("n{i}").into_bytes();
            b.push(0);
            bufs.push(b);
            let kp = bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char;
            m.put_str(&p, kp, &(i as u64).to_le_bytes());
            m.check(&format!("mixed put {i} (want={want})"));
        }
        assert_eq!(m.get_str(&p, kb.as_mut_ptr() as *mut c_char), 0);
        m.check(&format!("empty key still found (want={want})"));
        assert_eq!(m.del_str(&p, kb.as_mut_ptr() as *mut c_char, 0), 1);
        m.check(&format!("empty key del (want={want})"));
        m.free(&p);
    }
}

// ===========================================================================
// #42, #43, #46, #47 — intput / strkey
// ===========================================================================

/// #42 and #43 — the `hmget(intmap, 9) == num` and `hmget(intmap, 11) == 3`
/// asserts never fire; #46 — `intput` succeeds for every `num` except 9 and 11.
#[test]
fn err_42_43_46_intput_extremes() {
    let p = Pair::new();
    let mut cases: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        3,
        8,
        10,
        12,
        -9,
        -11,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX - 1,
    ];
    let mut rng = Rng::new(0xE7_4243);
    while cases.len() < 400 {
        let v = rng.i32v();
        if v != 9 && v != 11 {
            cases.push(v);
        }
    }
    for &gseed in &[0usize, 1, 0x3141_5926, usize::MAX] {
        p.seed(gseed);
        for &num in &cases {
            unsafe {
                // must return normally on both sides (no assert, no abort)
                (p.c.intput)(num);
                (p.r.intput)(num);
            }
        }
        // both globals advanced identically
        unsafe {
            let hc = (p.c.shmode_func)(16, STBDS_SH_NONE);
            let hr = (p.r.shmode_func)(16, STBDS_SH_NONE);
            let sc = snap_map(hc, 16, KeyKind::Binary, false);
            let sr = snap_map(hr, 16, KeyKind::Binary, false);
            eq_snap("global seed after intput sweep", &sc, &sr);
            (p.c.hmfree_func)((hc as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((hr as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

/// #47 — `strkey(INT_MIN)`: `sprintf("test_%d")` must emit all 11 digits +
/// sign, NUL-terminated, inside the 256-byte static buffer.
#[test]
fn err_47_strkey_int_min() {
    let p = Pair::new();
    for n in [c_int::MIN, c_int::MIN + 1, c_int::MAX, 0, -1] {
        let (sc, sr) = unsafe { (cstr_bytes((p.c.strkey)(n)), cstr_bytes((p.r.strkey)(n))) };
        assert_eq!(sc, sr, "strkey({n}) diverged");
        assert_eq!(String::from_utf8_lossy(&sc), format!("test_{n}"));
        assert!(sc.len() < 256, "must fit the 256-byte buffer");
    }
    // the buffer is reused, so a short key after a long one must be terminated
    unsafe {
        let long = cstr_bytes((p.c.strkey)(c_int::MIN));
        assert_eq!(long.len(), 16);
        let short_c = cstr_bytes((p.c.strkey)(0));
        let short_r = cstr_bytes((p.r.strkey)(0));
        assert_eq!(short_c, b"test_0".to_vec());
        assert_eq!(short_c, short_r);
    }
}

// ===========================================================================
// #49 — size_t overflow in `elemsize * min_cap + sizeof(header)`
// ===========================================================================

/// The C multiply/add wrap; the wrapped value is what `realloc` receives.
/// Only combinations that wrap to a size >= `sizeof(stbds_array_header)` are
/// exercised, so the header write itself stays inside the allocation (anything
/// smaller is an out-of-bounds write by construction).
#[test]
fn err_49_arrgrowf_size_overflow() {
    let p = Pair::new();
    // elemsize * min_cap == 0 (mod 2^64) => realloc(32), header fits exactly
    for &(elemsize, min_cap) in &[
        (1usize << 63, 4usize),
        (1usize << 62, 8),
        (1usize << 61, 16),
        (1usize << 60, 16),
        (1usize << 32, 1usize << 32),
        (1usize << 63, 8),
        // wraps to a NON-zero small size: 1 * SIZE_MAX + 32 == 31. glibc's
        // usable size for a 31-byte request is >= 32, so the header write still
        // lands inside the allocation and the call survives on both sides.
        (1usize, usize::MAX),
        (2usize, usize::MAX),
    ] {
        let (ac, ar) = unsafe {
            (
                (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap),
                (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap),
            )
        };
        assert!(
            !ac.is_null() && !ar.is_null(),
            "wrapped size must still be allocated ({elemsize:#x} * {min_cap})"
        );
        unsafe {
            let hc = *(ac as *mut ArrayHeader).sub(1);
            let hr = *(ar as *mut ArrayHeader).sub(1);
            assert_eq!(
                (hc.length, hc.capacity, hc.temp, hc.hash_table.is_null()),
                (hr.length, hr.capacity, hr.temp, hr.hash_table.is_null()),
                "header after a wrapped-size allocation diverged \
                 ({elemsize:#x} * {min_cap})"
            );
            assert_eq!(hc.capacity, min_cap.max(4), "capacity must be the (unadjusted) min_cap");
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}
