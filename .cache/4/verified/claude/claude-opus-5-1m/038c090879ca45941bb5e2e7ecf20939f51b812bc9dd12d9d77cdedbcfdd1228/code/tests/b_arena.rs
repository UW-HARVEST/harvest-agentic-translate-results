//! Phase B — CONFIGS.md rows 43..47: `stbds_stralloc` / `stbds_strreset` driven
//! directly on a caller-owned `stbds_string_arena` (the lowest-level string
//! entry points; `STBDS_SH_ARENA` maps go through the very same code).

mod common;

use common::*;
use std::ffi::{c_char, c_void};

/// Where inside the arena the returned pointer landed — this pins down exactly
/// which branch of `stbds_stralloc` ran, without comparing raw addresses.
#[derive(Debug, PartialEq, Eq)]
struct AllocLoc {
    /// `p == a->storage->storage + a->remaining` — the bump-allocation path.
    eq_head_bump: bool,
    /// `p == a->storage->storage` — the oversized block became the head.
    eq_head_base: bool,
    /// `p == a->storage->next->storage` — the oversized block was spliced in
    /// right after the head.
    eq_second_base: bool,
    content: Vec<u8>,
}

unsafe fn alloc_loc(a: *const Arena, p: *const c_char) -> AllocLoc {
    let head = (*a).storage;
    let head_base = if head.is_null() {
        std::ptr::null()
    } else {
        std::ptr::addr_of!((*head).storage) as *const c_char
    };
    let second = if head.is_null() {
        std::ptr::null_mut()
    } else {
        (*head).next
    };
    let second_base = if second.is_null() {
        std::ptr::null()
    } else {
        std::ptr::addr_of!((*second).storage) as *const c_char
    };
    AllocLoc {
        eq_head_bump: !head_base.is_null() && p == head_base.add((*a).remaining),
        eq_head_base: !head_base.is_null() && p == head_base,
        eq_second_base: !second_base.is_null() && p == second_base,
        content: cstr_bytes(p),
    }
}

/// Run one `stbds_stralloc` on both sides and compare everything observable.
#[track_caller]
fn stralloc_both(p: &Pair, ac: &mut Arena, ar: &mut Arena, s: &[u8], what: &str) {
    let mut buf = s.to_vec();
    buf.push(0);
    unsafe {
        let pc = (p.c.stralloc)(ac as *mut Arena, buf.as_mut_ptr() as *mut c_char);
        let pr = (p.r.stralloc)(ar as *mut Arena, buf.as_mut_ptr() as *mut c_char);
        let (lc, lr) = (
            alloc_loc(ac as *const Arena, pc),
            alloc_loc(ar as *const Arena, pr),
        );
        assert_eq!(lc, lr, "{what}: stralloc result diverged");
        assert_eq!(lc.content, s, "{what}: stored string is wrong");
        // absolute invariant of the bump path (C L914):
        //   p == a->storage->storage + a->remaining   (after `remaining -= len`)
        // The oversized branch returns the new block's `storage` instead.
        assert!(
            lc.eq_head_bump || lc.eq_head_base || lc.eq_second_base,
            "{what}: the returned pointer matches none of stbds_stralloc's \
             three documented positions: {lc:?}"
        );
        let (sc, sr) = (snap_arena(ac), snap_arena(ar));
        assert_eq!(sc, sr, "{what}: arena state diverged");
    }
}

fn reset_both(p: &Pair, ac: &mut Arena, ar: &mut Arena, what: &str) {
    unsafe {
        (p.c.strreset)(ac as *mut Arena);
        (p.r.strreset)(ar as *mut Arena);
        let (sc, sr) = (snap_arena(ac), snap_arena(ar));
        assert_eq!(sc, sr, "{what}: arena state after strreset diverged");
        assert_eq!(
            sc,
            ArenaSnap {
                remaining: 0,
                block: 0,
                mode: 0,
                block_count: 0,
                storage_is_null: true,
            },
            "{what}: strreset must zero the arena"
        );
    }
}

/// Row 43 — many allocations from a fresh arena: the 512-byte first block is
/// filled, then `512 << (block>>1)` block growth kicks in.
#[test]
fn cfg_43_stralloc_progressive() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_43);
    for round in 0..8u64 {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        // start with the empty string (len == 1, the smallest possible request)
        stralloc_both(&p, &mut ac, &mut ar, b"", "empty string");
        for i in 0..400usize {
            let len = 1 + (rng.below(40) + (i % 7)) % 40;
            let s: Vec<u8> = (0..len).map(|_| 0x41 + (rng.next_u64() % 26) as u8).collect();
            stralloc_both(&p, &mut ac, &mut ar, &s, &format!("round {round} alloc {i}"));
        }
        let sc = unsafe { snap_arena(&ac) };
        assert!(sc.block >= 1, "arena block must have advanced");
        assert!(sc.block_count > 1, "expected multiple arena blocks");
        reset_both(&p, &mut ac, &mut ar, "progressive");
    }
}

/// Row 44 — a string longer than `blocksize` on a *fresh* arena
/// (`storage == NULL`): the dedicated block becomes the head, `remaining = 0`.
#[test]
fn cfg_44_stralloc_oversized_fresh() {
    let p = Pair::new();
    for &len in &[512usize, 513, 1000, 5000, 100_000] {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        let s = vec![b'Q'; len];
        stralloc_both(&p, &mut ac, &mut ar, &s, &format!("oversized-fresh len={len}"));
        let sc = unsafe { snap_arena(&ac) };
        if len + 1 > 512 {
            assert!(!sc.storage_is_null);
            assert_eq!(sc.remaining, 0, "oversized-on-fresh sets remaining = 0");
            assert_eq!(sc.block_count, 1);
            // absolute check: the dedicated block became the head and the string
            // sits at its `storage` field (block + 8)
            unsafe {
                for (name, a) in [("C", &ac), ("Rust", &ar)] {
                    let base = a.storage as usize;
                    let sp = std::ptr::addr_of!((*a.storage).storage) as usize;
                    assert_eq!(sp - base, 8, "{name}: storage field must be at +8");
                }
            }
        }
        // a follow-up allocation must still work (remaining == 0 forces a grow)
        stralloc_both(&p, &mut ac, &mut ar, b"after", "after oversized-fresh");
        reset_both(&p, &mut ac, &mut ar, "oversized-fresh");
    }
}

/// Row 45 — a string longer than `blocksize` on a *used* arena
/// (`storage != NULL`): the dedicated block is spliced in *after* the head and
/// `remaining` is preserved.
#[test]
fn cfg_45_stralloc_oversized_used() {
    let p = Pair::new();
    for &len in &[1000usize, 5000, 200_000] {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        stralloc_both(&p, &mut ac, &mut ar, b"seed-the-arena", "seed alloc");
        let before = unsafe { snap_arena(&ac) };
        assert!(!before.storage_is_null);
        let s = vec![b'Z'; len];
        stralloc_both(&p, &mut ac, &mut ar, &s, &format!("oversized-used len={len}"));
        let after = unsafe { snap_arena(&ac) };
        assert_eq!(
            after.remaining, before.remaining,
            "oversized-on-used must preserve `remaining`"
        );
        assert_eq!(after.block_count, before.block_count + 1);
        // absolute check: the oversized block was spliced in *after* the head
        // (`sb->next = a->storage->next; a->storage->next = sb;`), so the head is
        // unchanged and the new block is the second element of the list.
        unsafe {
            for (name, a) in [("C", &ac), ("Rust", &ar)] {
                let head = a.storage;
                assert!(!head.is_null(), "{name}: head must survive");
                let second = (*head).next;
                assert!(!second.is_null(), "{name}: the oversized block must be #2");
            }
        }
        // interleave more small allocations with more oversized ones
        for i in 0..8usize {
            stralloc_both(&p, &mut ac, &mut ar, b"small", &format!("small {i}"));
            let big = vec![b'Y'; len + i];
            stralloc_both(&p, &mut ac, &mut ar, &big, &format!("big {i}"));
        }
        reset_both(&p, &mut ac, &mut ar, "oversized-used");
    }
}

/// Row 46 — the `block` counter across its whole range, including saturation
/// (`512 << (block>>1) >= 1<<20` ⇒ `++a->block` is skipped) and the shift-count
/// wrap-around for absurd `block` values.
#[test]
fn cfg_46_stralloc_block_range() {
    let p = Pair::new();
    for block in 0u8..=44 {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        ac.block = block;
        ar.block = block;
        // remaining == 0 forces the grow branch, so `block` is consulted
        stralloc_both(&p, &mut ac, &mut ar, b"hello", &format!("block={block}"));
        let sc = unsafe { snap_arena(&ac) };
        let blocksize = 512usize.wrapping_shl((block >> 1) as u32);
        if blocksize < (1 << 20) {
            assert_eq!(sc.block, block + 1, "block must advance while < MAX");
        } else {
            assert_eq!(sc.block, block, "block must saturate at >= MAX");
        }
        // a second allocation with the (possibly saturated) block value
        stralloc_both(&p, &mut ac, &mut ar, b"world", &format!("block={block} #2"));
        reset_both(&p, &mut ac, &mut ar, &format!("block={block}"));
    }
}

/// Row 47 — allocate N blocks then `stbds_strreset`; repeated resets are no-ops.
#[test]
fn cfg_47_strreset() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_47);

    // (a) empty arena
    let mut ac = Arena::zeroed();
    let mut ar = Arena::zeroed();
    reset_both(&p, &mut ac, &mut ar, "empty arena");
    reset_both(&p, &mut ac, &mut ar, "empty arena twice");

    // (b) arena with many blocks (small, oversized, and mixed)
    for round in 0..6 {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        for i in 0..200usize {
            let len = if i % 17 == 0 {
                1000 + rng.below(2000)
            } else {
                1 + rng.below(60)
            };
            let s = vec![b'a' + (i % 26) as u8; len];
            stralloc_both(&p, &mut ac, &mut ar, &s, &format!("round {round} alloc {i}"));
        }
        let sc = unsafe { snap_arena(&ac) };
        assert!(sc.block_count > 2, "expected many blocks, got {}", sc.block_count);
        reset_both(&p, &mut ac, &mut ar, &format!("round {round}"));
        // the arena is reusable after a reset
        stralloc_both(&p, &mut ac, &mut ar, b"reused", "after reset");
        reset_both(&p, &mut ac, &mut ar, "after reuse");
    }

    // (c) non-zero mode/block must also be cleared by strreset
    let mut ac = Arena::zeroed();
    let mut ar = Arena::zeroed();
    ac.mode = 3;
    ar.mode = 3;
    ac.block = 5;
    ar.block = 5;
    stralloc_both(&p, &mut ac, &mut ar, b"x", "mode/block preset");
    reset_both(&p, &mut ac, &mut ar, "mode/block preset");
}

/// The `STBDS_SH_ARENA` table uses the very same arena code; verify the arena
/// inside the table survives a grow/shrink/rebuild (`stbds_make_hash_index`
/// copies `ot->string` verbatim).
#[test]
fn cfg_43b_table_arena_survives_rehash() {
    let p = Pair::new();
    let keysize = std::mem::size_of::<*mut c_char>();
    let kind = KeyKind::StringPtr { keyoffset: 0 };
    for &gseed in &[0usize, 0x3141_5926] {
        p.seed(gseed);
        let mut m = MapPair::shmode(&p, 16, keysize, STBDS_HM_STRING, STBDS_SH_ARENA, kind);
        let mut bufs: Vec<Vec<u8>> = Vec::new();
        let mut live: Vec<usize> = Vec::new();
        let mut rng = Rng::new(0xC0FFEE_43B ^ gseed as u64);
        for step in 0..500usize {
            let id = step;
            let mut kb = format!("arena_key_{id}_{}", "p".repeat(id % 60)).into_bytes();
            kb.push(0);
            bufs.push(kb);
            let kp = bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char;
            if live.len() > 5 && rng.next_u64() % 3 == 0 {
                let i = rng.below(live.len());
                let victim = live.swap_remove(i);
                let mut vb = format!("arena_key_{victim}_{}", "p".repeat(victim % 60)).into_bytes();
                vb.push(0);
                m.del_str(&p, vb.as_mut_ptr() as *mut c_char, 0);
            } else {
                m.put_str(&p, kp, &(step as u64).to_le_bytes());
                live.push(id);
            }
            m.check(&format!("table arena step {step}"));
        }
        m.free(&p);
    }
    let _ = unsafe { (p.c.hash_bytes)(std::ptr::null_mut(), 0, 0) } as *const c_void;
}
