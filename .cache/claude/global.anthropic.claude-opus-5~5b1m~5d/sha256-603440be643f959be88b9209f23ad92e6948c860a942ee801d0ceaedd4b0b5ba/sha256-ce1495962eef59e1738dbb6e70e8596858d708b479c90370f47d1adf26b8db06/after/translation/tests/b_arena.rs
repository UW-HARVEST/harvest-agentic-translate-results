//! Phase B — CONFIGS.md rows C24..C34: `stbds_stralloc` / `stbds_strreset`.
//!
//! Heap addresses legitimately differ between the two libraries, so besides
//! comparing every scalar arena field we independently verify, *inside each
//! library*, the exact placement identity the C code implies:
//!
//! ```c
//! p = a->storage->storage + a->remaining - len;   a->remaining -= len;
//! ```

mod common;
use common::*;
use std::ffi::c_char;

const BS_MIN: usize = 512;
const BS_MAX: usize = 1 << 20;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Case {
    InBlock,
    NewBlock,
    OversizeFresh,
    OversizeSplice,
}

/// `blocksize = (size_t) 512u << (a->block >> 1)` — on x86-64 the shift count is
/// taken modulo 64, which is what `block >= 128` relies on (R27).
fn blocksize_of(block: u8) -> usize {
    BS_MIN.wrapping_shl((block >> 1) as u32)
}

fn classify(before: &StringArena, len: usize) -> Case {
    if len <= before.remaining {
        return Case::InBlock;
    }
    let bs = blocksize_of(before.block);
    if len > bs {
        if before.storage.is_null() {
            Case::OversizeFresh
        } else {
            Case::OversizeSplice
        }
    } else {
        Case::NewBlock
    }
}

unsafe fn block_count(a: &StringArena) -> usize {
    let mut n = 0usize;
    let mut x = a.storage as *mut StringBlock;
    unsafe {
        while !x.is_null() {
            n += 1;
            x = (*x).next;
            assert!(n < 1_000_000, "cycle in arena block chain");
        }
    }
    n
}

unsafe fn snap(a: &StringArena) -> String {
    unsafe { format!("{} blocks={}", snap_arena(a), block_count(a)) }
}

/// Run one `stralloc` on one library and assert every structural invariant the
/// C code guarantees, then return a library-independent snapshot.
unsafe fn one(
    l: &Lib,
    arena: &mut StringArena,
    s: &[u8], // NUL-terminated
    ctx: &str,
) -> String {
    unsafe {
        let len = s.len(); // strlen+1
        let before = *arena;
        let case = classify(&before, len);
        let p = (l.stralloc)(arena as *mut StringArena, s.as_ptr() as *mut c_char);
        let after = *arena;

        // the string itself must round-trip
        assert_eq!(
            cstr_opt(p),
            cstr_opt(s.as_ptr() as *const c_char),
            "[{}] {ctx}: content",
            l.name
        );

        let bs = blocksize_of(before.block);
        let want_block = if bs < BS_MAX {
            before.block.wrapping_add(1)
        } else {
            before.block
        };

        match case {
            Case::InBlock => {
                assert_eq!(after.storage, before.storage, "[{}] {ctx}: storage", l.name);
                assert_eq!(after.block, before.block, "[{}] {ctx}: block", l.name);
                assert_eq!(
                    after.remaining,
                    before.remaining - len,
                    "[{}] {ctx}: remaining",
                    l.name
                );
                let base = (before.storage as *mut u8).add(8);
                assert_eq!(
                    p as *mut u8,
                    base.add(before.remaining - len),
                    "[{}] {ctx}: placement",
                    l.name
                );
            }
            Case::NewBlock => {
                assert!(!after.storage.is_null());
                assert_ne!(after.storage, before.storage, "[{}] {ctx}: new head", l.name);
                assert_eq!(after.block, want_block, "[{}] {ctx}: block", l.name);
                assert_eq!(after.remaining, bs - len, "[{}] {ctx}: remaining", l.name);
                assert_eq!(
                    (*(after.storage as *mut StringBlock)).next as *mut std::ffi::c_void,
                    before.storage,
                    "[{}] {ctx}: chain",
                    l.name
                );
                let base = (after.storage as *mut u8).add(8);
                assert_eq!(
                    p as *mut u8,
                    base.add(bs - len),
                    "[{}] {ctx}: placement",
                    l.name
                );
            }
            Case::OversizeFresh => {
                assert!(!after.storage.is_null());
                assert_eq!(after.remaining, 0, "[{}] {ctx}: remaining", l.name);
                assert_eq!(after.block, want_block, "[{}] {ctx}: block", l.name);
                assert!(
                    (*(after.storage as *mut StringBlock)).next.is_null(),
                    "[{}] {ctx}: next must be NULL",
                    l.name
                );
                assert_eq!(
                    p as *mut u8,
                    (after.storage as *mut u8).add(8),
                    "[{}] {ctx}: placement",
                    l.name
                );
            }
            Case::OversizeSplice => {
                // the head block and `remaining` are left completely alone
                assert_eq!(after.storage, before.storage, "[{}] {ctx}: storage", l.name);
                assert_eq!(
                    after.remaining, before.remaining,
                    "[{}] {ctx}: remaining untouched",
                    l.name
                );
                assert_eq!(after.block, want_block, "[{}] {ctx}: block", l.name);
                let sb = (*(after.storage as *mut StringBlock)).next;
                assert!(!sb.is_null(), "[{}] {ctx}: spliced block", l.name);
                assert_eq!(
                    p as *mut u8,
                    (sb as *mut u8).add(8),
                    "[{}] {ctx}: placement",
                    l.name
                );
            }
        }
        format!("case={case:?} {}", snap(&after))
    }
}

/// Drive an identical `stralloc` sequence through both libraries.
fn seq(ctx: &str, strings: &[Vec<u8>], start: StringArena) {
    let (c, r) = both();
    let mut ca = start;
    let mut ra = start;
    unsafe {
        for (i, s) in strings.iter().enumerate() {
            let cs = one(c, &mut ca, s, &format!("{ctx} #{i} len={}", s.len()));
            let rs = one(r, &mut ra, s, &format!("{ctx} #{i} len={}", s.len()));
            eqs(&format!("{ctx} #{i} len={}", s.len()), &cs, &rs);
        }
        // teardown
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        eqs(&format!("{ctx} after strreset"), &snap(&ca), &snap(&ra));
        assert_eq!(ca.remaining, 0);
        assert_eq!(ca.block, 0);
        assert_eq!(ca.mode, 0);
        assert!(ca.storage.is_null());
    }
}

fn s_of(n: usize) -> Vec<u8> {
    // n non-NUL bytes + NUL  => len == n+1
    let mut v: Vec<u8> = (0..n).map(|i| b'A' + (i % 26) as u8).collect();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// C24/C33 — fresh arena, one short string; empty string
// ---------------------------------------------------------------------------
#[test]
fn c24_c33_fresh_arena_single_string() {
    let _g = lock();
    for n in [0usize, 1, 2, 7, 8, 15, 16, 100, 510, 511] {
        seq(&format!("fresh n={n}"), &[s_of(n)], StringArena::zeroed());
    }
}

// ---------------------------------------------------------------------------
// C25 — the block ladder driven by many short strings
// ---------------------------------------------------------------------------
#[test]
fn c25_block_ladder_many_short_strings() {
    let _g = lock();
    let mut rng = Rng::new(0xAE25_0001);
    for &n in &[1usize, 4, 8, 16, 40] {
        let strings: Vec<Vec<u8>> = (0..3000).map(|_| rng.cstring(n)).collect();
        seq(&format!("ladder n={n}"), &strings, StringArena::zeroed());
    }
    // mixed lengths
    for trial in 0..20u64 {
        let mut rng = Rng::new(0xAE25_2000 + trial);
        let strings: Vec<Vec<u8>> = (0..600)
            .map(|_| {
                let n = rng.below(600) as usize;
                rng.cstring(n)
            })
            .collect();
        seq(&format!("mixed trial={trial}"), &strings, StringArena::zeroed());
    }
}

// ---------------------------------------------------------------------------
// C26/C27/C28/C29/C30 — the len-vs-remaining and len-vs-blocksize boundaries
// ---------------------------------------------------------------------------
#[test]
fn c26_c30_boundaries() {
    let _g = lock();
    // C26: len == remaining exactly.  C27: len == remaining + 1.
    // Prime the arena with one small string, then hit the boundary.
    for delta in [-2i64, -1, 0, 1, 2] {
        // after allocating `prime` bytes from the 512 block, remaining = 512 - prime
        let prime = 100usize;
        let rem = 512 - prime;
        let target = (rem as i64 + delta) as usize;
        if target == 0 {
            continue;
        }
        seq(
            &format!("rem-boundary delta={delta}"),
            &[s_of(prime - 1), s_of(target - 1)],
            StringArena::zeroed(),
        );
    }

    // C28: len == blocksize exactly (fresh arena, blocksize 512)
    seq("len==blocksize", &[s_of(511)], StringArena::zeroed());
    // C29: len == blocksize + 1 on a fresh arena  =>  OversizeFresh
    seq("len==blocksize+1 fresh", &[s_of(512)], StringArena::zeroed());
    seq("len==blocksize+huge fresh", &[s_of(10_000)], StringArena::zeroed());
    // C30: len > blocksize on a NON-empty arena  =>  OversizeSplice
    seq(
        "oversize splice",
        &[s_of(10), s_of(2000), s_of(11), s_of(5000), s_of(12)],
        StringArena::zeroed(),
    );
    // every boundary of the first few ladder steps
    for step in 0..8u8 {
        let bs = blocksize_of(step);
        let mut arena = StringArena::zeroed();
        arena.block = step;
        for d in [-1i64, 0, 1] {
            let n = (bs as i64 + d - 1) as usize;
            seq(&format!("ladder-boundary block={step} bs={bs} d={d}"), &[s_of(n)], arena);
        }
    }
}

// ---------------------------------------------------------------------------
// C31 — pre-set `block` across the whole ladder incl. the 1 MiB ceiling (R28)
// C32 — `block` >= 128 so that `512 << (block>>1)` shifts by >= 64 (R27)
// ---------------------------------------------------------------------------
#[test]
fn c31_c32_preset_block_field() {
    let _g = lock();

    // `block` values whose `blocksize` is either <= 16 MiB or exactly 0.
    //
    // Values in between (e.g. `block = 64` -> `512 << 32` == 2 TiB) make the C
    // code call `realloc` for a multi-terabyte block, get NULL back and then
    // dereference it (`sb->next = a->storage`).  That is undefined behaviour in
    // the C original and it crashes *both* libraries identically, so it is not a
    // divergence and is deliberately not exercised (see ERRORS.md R27).
    let mut blocks: Vec<u8> = Vec::new();
    for b in 0u16..=255 {
        let b = b as u8;
        let bs = blocksize_of(b);
        if bs == 0 || bs <= 16 * 1024 * 1024 {
            blocks.push(b);
        }
    }
    assert!(blocks.contains(&255) && blocks.contains(&127) && blocks.contains(&22));

    for block in blocks {
        let mut arena = StringArena::zeroed();
        arena.block = block;
        // small string, then a couple more so the follow-on state is compared too
        seq(
            &format!("preset block={block}"),
            &[s_of(3), s_of(7), s_of(3)],
            arena,
        );
    }
    // the documented ceiling: block stops incrementing once blocksize == 1 MiB
    assert_eq!(blocksize_of(22), BS_MAX);
    assert!(blocksize_of(21) < BS_MAX);
    assert_eq!(blocksize_of(255), 0, "512 << 63 wraps to 0");
}

// ---------------------------------------------------------------------------
// C34 — strreset on 0 / 1 / many blocks, incl. a spliced oversize block, then
//       re-use of the same arena (R29, R30)
// ---------------------------------------------------------------------------
#[test]
fn c34_strreset_shapes() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        // empty arena, reset twice
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        for i in 0..3 {
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            eqs(&format!("empty reset {i}"), &snap(&ca), &snap(&ra));
        }

        // build -> reset -> rebuild -> reset, several shapes
        for shape in 0..4 {
            let strings: Vec<Vec<u8>> = match shape {
                0 => vec![s_of(3)],
                1 => (0..200).map(|i| s_of(i % 40)).collect(),
                2 => vec![s_of(10), s_of(3000), s_of(10)],
                _ => (0..50).map(|i| s_of(600 + i * 13)).collect(),
            };
            for round in 0..2 {
                for (i, s) in strings.iter().enumerate() {
                    one(c, &mut ca, s, &format!("shape{shape} r{round} #{i}"));
                    one(r, &mut ra, s, &format!("shape{shape} r{round} #{i}"));
                    eqs(
                        &format!("shape{shape} round{round} #{i}"),
                        &snap(&ca),
                        &snap(&ra),
                    );
                }
                (c.strreset)(&mut ca);
                (r.strreset)(&mut ra);
                eqs(&format!("shape{shape} round{round} reset"), &snap(&ca), &snap(&ra));
                assert!(ca.storage.is_null() && ra.storage.is_null());
                assert_eq!(ca.block, 0);
                assert_eq!(ra.block, 0);
            }
        }

        // mode must be zeroed by strreset too
        ca.mode = 3;
        ra.mode = 3;
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        assert_eq!(ca.mode, 0);
        assert_eq!(ra.mode, 0);
    }
}

/// Long randomized arena walks (property style, fixed seeds).
#[test]
fn c24_c34_randomized_arena_walks() {
    let _g = lock();
    for trial in 0..60u64 {
        let mut rng = Rng::new(0xAE99_0000 + trial);
        let mut arena = StringArena::zeroed();
        arena.block = (rng.below(26)) as u8;
        let strings: Vec<Vec<u8>> = (0..250)
            .map(|_| {
                let n = match rng.below(10) {
                    0 => rng.below(3) as usize,
                    1..=6 => rng.below(64) as usize,
                    7 | 8 => rng.below(1200) as usize,
                    _ => rng.below(9000) as usize,
                };
                rng.cstring(n)
            })
            .collect();
        seq(&format!("walk trial={trial} block={}", arena.block), &strings, arena);
    }
}
