//! Phase B rows C50..C57 -- `stbds_stralloc` / `stbds_strreset`.

mod common;
use common::*;
use std::ffi::c_char;

const BLOCKSIZE_MIN: usize = 512;
const BLOCKSIZE_MAX: usize = 1 << 20;

/// The C computes `(size_t) 512u << (a->block >> 1)`.  For a caller-supplied
/// `block` the shift count can reach 127; x86-64 masks it to 6 bits, which is
/// what `wrapping_shl` reproduces.
fn blocksize_for(block: u8) -> usize {
    BLOCKSIZE_MIN.wrapping_shl((block >> 1) as u32)
}

#[derive(Debug, PartialEq, Eq)]
enum Path {
    /// `len <= remaining` -- carve out of the current block
    InPlace,
    /// `len > remaining`, `len <= blocksize` -- take a fresh head block
    NewBlock,
    /// `len > remaining`, `len > blocksize` -- dedicated oversized block
    Oversized,
}

fn ac_before(a: &StringArena) -> StringArena {
    *a
}

fn path_for(arena: &StringArena, len: usize) -> Path {
    if len <= arena.remaining {
        Path::InPlace
    } else if len > blocksize_for(arena.block) {
        Path::Oversized
    } else {
        Path::NewBlock
    }
}

/// One differential `stbds_stralloc` step, with full state + placement checks.
#[track_caller]
fn step(
    c: &Api,
    rs: &Api,
    ac: &mut StringArena,
    ar: &mut StringArena,
    s: &[u8],
    tag: &str,
) -> (*mut c_char, *mut c_char) {
    unsafe {
        assert_same(
            &format!("{tag}: arena state before"),
            &snap_arena(ac),
            &snap_arena(ar),
        );
        let len = strlen(s.as_ptr() as *const c_char) + 1;
        let before_c = *ac;
        let before_r = *ar;
        let path = path_for(&before_c, len);
        let bs = blocksize_for(before_c.block);

        let pc = (c.stralloc)(ac as *mut StringArena, s.as_ptr() as *mut c_char);
        let pr = (rs.stralloc)(ar as *mut StringArena, s.as_ptr() as *mut c_char);

        assert_same(
            &format!("{tag}: arena state after ({path:?}, len={len}, bs={bs})"),
            &snap_arena(ac),
            &snap_arena(ar),
        );

        // content must be a byte-exact copy
        assert_eq!(strcmp(pc, s.as_ptr() as *const c_char), 0, "{tag}: C copy");
        assert_eq!(strcmp(pr, s.as_ptr() as *const c_char), 0, "{tag}: RUST copy");
        assert_eq!(
            std::slice::from_raw_parts(pc as *const u8, len),
            std::slice::from_raw_parts(pr as *const u8, len),
            "{tag}: copied bytes differ"
        );

        // address-independent placement invariants, checked per library
        for (name, p, a, before) in [("C", pc, &*ac, &before_c), ("RUST", pr, &*ar, &before_r)] {
            let head = a.storage as usize;
            match path {
                Path::InPlace | Path::NewBlock => {
                    assert_eq!(
                        p as usize - (head + 8),
                        a.remaining,
                        "{tag}/{name}: p must sit at storage+8+remaining"
                    );
                    if path == Path::InPlace {
                        assert_eq!(a.remaining, before.remaining - len, "{tag}/{name}");
                        assert_eq!(a.block, before.block, "{tag}/{name}: block must not move");
                    } else {
                        assert_eq!(a.remaining, bs - len, "{tag}/{name}");
                    }
                }
                Path::Oversized => {
                    if before.storage.is_null() {
                        assert_eq!(p as usize, head + 8, "{tag}/{name}: oversized head");
                        assert_eq!(a.remaining, 0, "{tag}/{name}");
                    } else {
                        // spliced in as storage->next, remaining untouched
                        let second = *(head as *const usize);
                        assert_eq!(p as usize, second + 8, "{tag}/{name}: oversized splice");
                        assert_eq!(
                            a.remaining, before.remaining,
                            "{tag}/{name}: remaining must be untouched"
                        );
                        assert_eq!(head, before.storage as usize, "{tag}/{name}: head moved");
                    }
                }
            }
            // `block` is bumped iff the *old* blocksize was below the 1 MiB cap
            if path != Path::InPlace {
                let want = if bs < BLOCKSIZE_MAX {
                    before.block.wrapping_add(1)
                } else {
                    before.block
                };
                assert_eq!(a.block, want, "{tag}/{name}: block progression");
            }
        }
        (pc, pr)
    }
}

fn reset(c: &Api, rs: &Api, ac: &mut StringArena, ar: &mut StringArena, tag: &str) {
    unsafe {
        (c.strreset)(ac as *mut StringArena);
        (rs.strreset)(ar as *mut StringArena);
        assert_same(&format!("{tag}: after strreset"), &snap_arena(ac), &snap_arena(ar));
        assert_eq!(*ac, StringArena::new(), "{tag}: C arena not zeroed");
        assert_eq!(*ar, StringArena::new(), "{tag}: RUST arena not zeroed");
    }
}

// --- C50 --------------------------------------------------------------------
#[test]
fn cfg_c50_stralloc_fresh() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(50);
        for _ in 0..200 {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let n = rng.below(60);
            let s = rng.cstring(1 + n);
            step(c, rs, &mut ac, &mut ar, &s, "c50");
            reset(c, rs, &mut ac, &mut ar, "c50");
        }
        // deterministic boundary lengths
        for n in [0usize, 1, 2, 7, 8, 63, 64, 255, 256, 510, 511] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let mut s = vec![b'z'; n];
            s.push(0);
            step(c, rs, &mut ac, &mut ar, &s, &format!("c50 n={n}"));
            reset(c, rs, &mut ac, &mut ar, "c50");
        }
    });
}

// --- C51 --------------------------------------------------------------------
#[test]
fn cfg_c51_stralloc_block_refill() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(51);
        for strlen_ in [1usize, 6, 7, 15, 100, 170, 511] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let mut prev: Option<(usize, usize)> = None;
            for i in 0..400 {
                let mut s = vec![b'a' + (i % 26) as u8; strlen_];
                s.push(0);
                // `step` already asserts the exact placement invariant
                // (`p == storage+8+remaining`) for the in-block path; comparing
                // raw addresses across a *block change* would only compare
                // allocator choices, which are not part of the contract.
                let path = path_for(&ac_before(&ac), strlen_ + 1);
                let (pc, pr) = step(c, rs, &mut ac, &mut ar, &s, &format!("c51 L{strlen_} #{i}"));
                if path == Path::InPlace {
                    if let Some((lc, lr)) = prev {
                        assert_eq!(lc - pc as usize, strlen_ + 1, "c51 C delta #{i}");
                        assert_eq!(lr - pr as usize, strlen_ + 1, "c51 RUST delta #{i}");
                    }
                }
                prev = Some((pc as usize, pr as usize));
            }
            assert!(ac.block > 1, "several blocks must have been taken");
            reset(c, rs, &mut ac, &mut ar, "c51");
        }
        for _ in 0..20 {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            for i in 0..300 {
                let n = rng.below(200);
                let s = rng.cstring(1 + n);
                step(c, rs, &mut ac, &mut ar, &s, &format!("c51 rnd#{i}"));
            }
            reset(c, rs, &mut ac, &mut ar, "c51 rnd");
        }
    });
}

// --- C52 --------------------------------------------------------------------
#[test]
fn cfg_c52_stralloc_block_sweep() {
    // caller-supplied `block` 0..=22: 512 << (block>>1) up to the 1 MiB cap
    with_libs(0x31415926, |c, rs| {
        for block in 0u8..=22 {
            let bs = blocksize_for(block);
            assert!(bs <= BLOCKSIZE_MAX);
            for &n in &[0usize, 1, 100, 511] {
                let mut ac = StringArena::new();
                let mut ar = StringArena::new();
                ac.block = block;
                ar.block = block;
                let mut s = vec![b'q'; n];
                s.push(0);
                step(c, rs, &mut ac, &mut ar, &s, &format!("c52 block={block} n={n}"));
                // a second allocation exercises the incremented block value
                let mut s2 = vec![b'w'; n + 3];
                s2.push(0);
                step(c, rs, &mut ac, &mut ar, &s2, &format!("c52b block={block}"));
                reset(c, rs, &mut ac, &mut ar, "c52");
            }
        }
        // the 1 MiB saturation point: block 22 must stay at 22
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        ac.block = 22;
        ar.block = 22;
        let mut s = vec![b'x'; 10];
        s.push(0);
        step(c, rs, &mut ac, &mut ar, &s, "c52 sat");
        assert_eq!(ac.block, 22, "block must saturate at 22 (1 MiB)");
        assert_eq!(ar.block, 22);
        reset(c, rs, &mut ac, &mut ar, "c52 sat");
    });
}

// --- C53 --------------------------------------------------------------------
#[test]
fn cfg_c53_stralloc_oversized_both() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(53);
        // (a) storage == NULL
        for n in [512usize, 513, 1000, 4096, 100_000] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let mut s = vec![b'o'; n];
            s.push(0);
            step(c, rs, &mut ac, &mut ar, &s, &format!("c53a n={n}"));
            assert_eq!(ac.remaining, 0, "oversized-into-empty leaves remaining 0");
            reset(c, rs, &mut ac, &mut ar, "c53a");
        }
        // (b) storage != NULL -> spliced behind the head block
        for n in [512usize, 600, 5000, 200_000] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let mut small = vec![b's'; 10];
            small.push(0);
            step(c, rs, &mut ac, &mut ar, &small, "c53b seed");
            let rem_before = ac.remaining;
            let mut s = vec![b'O'; n];
            s.push(0);
            step(c, rs, &mut ac, &mut ar, &s, &format!("c53b n={n}"));
            assert_eq!(ac.remaining, rem_before, "splice must not touch remaining");
            // the head block must still be usable afterwards
            step(c, rs, &mut ac, &mut ar, &small, "c53b after");
            reset(c, rs, &mut ac, &mut ar, "c53b");
        }
        // (c) randomized interleaving of tiny and oversized strings
        for _ in 0..30 {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            for i in 0..120 {
                let big = rng.below(4) == 0;
                let n = if big { 600 + rng.below(3000) } else { 1 + rng.below(40) };
                let s = rng.cstring(n);
                step(c, rs, &mut ac, &mut ar, &s, &format!("c53c #{i}"));
            }
            reset(c, rs, &mut ac, &mut ar, "c53c");
        }
    });
}

// --- C54 --------------------------------------------------------------------
#[test]
fn cfg_c54_stralloc_remaining_boundary() {
    with_libs(0x31415926, |c, rs| {
        for extra in [0usize, 1] {
            for base in [1usize, 10, 100] {
                let mut ac = StringArena::new();
                let mut ar = StringArena::new();
                let mut seed_s = vec![b'a'; base];
                seed_s.push(0);
                step(c, rs, &mut ac, &mut ar, &seed_s, "c54 seed");
                let rem = ac.remaining;
                // len == remaining  (no new block) / len == remaining+1 (new block)
                let n = rem - 1 + extra;
                let mut s = vec![b'b'; n];
                s.push(0);
                let expect = if extra == 0 { Path::InPlace } else { Path::NewBlock };
                assert_eq!(path_for(&ac, n + 1), expect);
                step(c, rs, &mut ac, &mut ar, &s, &format!("c54 extra={extra} base={base}"));
                if extra == 0 {
                    assert_eq!(ac.remaining, 0, "block must be exactly exhausted");
                }
                reset(c, rs, &mut ac, &mut ar, "c54");
            }
        }
    });
}

// --- C55 --------------------------------------------------------------------
#[test]
fn cfg_c55_stralloc_empty_strings() {
    with_libs(0x31415926, |c, rs| {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        let s = [0u8; 1];
        for i in 0..1000 {
            step(c, rs, &mut ac, &mut ar, &s, &format!("c55 #{i}"));
        }
        assert!(ac.block >= 2, "1000 x 1 byte must span several blocks");
        reset(c, rs, &mut ac, &mut ar, "c55");
    });
}

// --- C56 --------------------------------------------------------------------
#[test]
fn cfg_c56_stralloc_random_then_reset() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(56);
        for round in 0..25 {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            for i in 0..500 {
                let which = rng.below(10);
                let len = match which {
                    0 => 1 + rng.below(2000),
                    1 => 500 + rng.below(30),
                    _ => 1 + rng.below(80),
                };
                let s = rng.cstring(len);
                step(c, rs, &mut ac, &mut ar, &s, &format!("c56 r{round} #{i}"));
            }
            reset(c, rs, &mut ac, &mut ar, &format!("c56 r{round}"));
            // reusable after reset
            let s = rng.cstring(20);
            step(c, rs, &mut ac, &mut ar, &s, "c56 reuse");
            reset(c, rs, &mut ac, &mut ar, "c56 reuse");
        }
    });
}

// --- C57 --------------------------------------------------------------------
#[test]
fn cfg_c57_strreset_states() {
    with_libs(0x31415926, |c, rs| {
        // (a) fresh / all-zero arena
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        reset(c, rs, &mut ac, &mut ar, "c57 fresh");
        reset(c, rs, &mut ac, &mut ar, "c57 fresh twice");
        // (b) single block
        let mut s = vec![b'k'; 7];
        s.push(0);
        step(c, rs, &mut ac, &mut ar, &s, "c57 one");
        assert_eq!(unsafe { snap_arena(&ac as *const _) }.chain_len, 1);
        reset(c, rs, &mut ac, &mut ar, "c57 one");
        // (c) many blocks (incl. oversized ones spliced behind the head)
        for i in 0..200 {
            let n = if i % 7 == 0 { 900 } else { 30 };
            let mut s = vec![b'm'; n];
            s.push(0);
            step(c, rs, &mut ac, &mut ar, &s, &format!("c57 many #{i}"));
        }
        let chain = unsafe { snap_arena(&ac as *const _) }.chain_len;
        assert!(chain > 5, "expected a long block chain, got {chain}");
        assert_eq!(chain, unsafe { snap_arena(&ar as *const _) }.chain_len);
        reset(c, rs, &mut ac, &mut ar, "c57 many");
        reset(c, rs, &mut ac, &mut ar, "c57 many twice");
    });
}
