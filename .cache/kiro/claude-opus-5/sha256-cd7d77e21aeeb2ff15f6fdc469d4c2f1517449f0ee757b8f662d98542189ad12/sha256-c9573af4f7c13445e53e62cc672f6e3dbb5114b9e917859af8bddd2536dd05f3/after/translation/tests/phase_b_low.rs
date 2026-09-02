//! Phase B — valid-path differential tests for the lowest-level entry points.
//!
//! CONFIGS.md rows 1-13: `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed`, `stbds_arrgrowf`, `stbds_stralloc`, `stbds_strreset`.
//!
//! Every call goes through `libloading` into the two `.so` files; nothing calls
//! the Rust crate directly.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

/// The lengths that make `stbds_siphash_bytes` take every distinct path:
/// every `len % 8` tail case, zero, and several full sip-loop iterations.
const LENS: &[usize] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 23, 24, 25, 31, 32, 33, 39, 63,
    64, 65, 127, 128, 129, 255, 256,
];

const SEEDS: &[usize] = &[
    DEFAULT_SEED,
    0,
    1,
    2,
    usize::MAX,
    usize::MAX - 1,
    0x8000_0000_0000_0000,
    0xdead_beef_cafe_babe,
];

// ---------------------------------------------------------------------------
// Row 1 / 2 / 3 — stbds_hash_bytes
// ---------------------------------------------------------------------------

#[test]
fn row01_hash_bytes_lengths_default_seed() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x1000);
    for &len in LENS {
        for _ in 0..64 {
            let mut buf = rng.bytes(len);
            let (a, b) = unsafe {
                let pa = buf.as_mut_ptr() as *mut c_void;
                (
                    (p.c.hash_bytes)(pa, len, DEFAULT_SEED),
                    (p.rs.hash_bytes)(pa, len, DEFAULT_SEED),
                )
            };
            assert_eq!(a, b, "hash_bytes len={len} buf={buf:02x?}");
        }
    }
}

#[test]
fn row02_hash_bytes_lengths_x_seeds() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x2000);
    for &len in LENS {
        for &seed in SEEDS {
            for _ in 0..24 {
                let mut buf = rng.bytes(len);
                let (a, b) = unsafe {
                    let pa = buf.as_mut_ptr() as *mut c_void;
                    (
                        (p.c.hash_bytes)(pa, len, seed),
                        (p.rs.hash_bytes)(pa, len, seed),
                    )
                };
                assert_eq!(a, b, "hash_bytes len={len} seed={seed:#x} buf={buf:02x?}");
            }
        }
    }
}

#[test]
fn row03_hash_bytes_high_bit_bytes() {
    // Exercises the C sign-extension quirk: `d[3] << 24` and `d[7] << 24` are
    // `int` expressions that overflow into negative values before being widened
    // to `size_t`.
    let (p, _g) = libs();
    let mut rng = Rng::new(0x3000);
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for &len in LENS {
        cases.push(vec![0xFF; len]);
        cases.push(vec![0x80; len]);
        cases.push((0..len).map(|i| 0x80 | (i as u8 & 0x7f)).collect());
        for _ in 0..16 {
            cases.push((0..len).map(|_| 0x80 | (rng.next_u32() as u8 >> 1)).collect());
        }
        // one high byte at each position
        for pos in 0..len.min(16) {
            let mut v = vec![0u8; len];
            v[pos] = 0xFF;
            cases.push(v);
        }
    }
    for mut buf in cases {
        let len = buf.len();
        for &seed in SEEDS {
            let (a, b) = unsafe {
                let pa = buf.as_mut_ptr() as *mut c_void;
                (
                    (p.c.hash_bytes)(pa, len, seed),
                    (p.rs.hash_bytes)(pa, len, seed),
                )
            };
            assert_eq!(a, b, "hash_bytes hi-bit len={len} seed={seed:#x} {buf:02x?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — stbds_hash_string
// ---------------------------------------------------------------------------

#[test]
fn row04_hash_string() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x4000);

    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"ab".to_vec(),
        b"abcdefg".to_vec(),
        b"abcdefgh".to_vec(),
        b"abcdefghi".to_vec(),
        b"test_0".to_vec(),
        b"test_-2147483648".to_vec(),
        vec![b'x'; 200],
        vec![0xFF; 1],
        vec![0xFF; 7],
        vec![0xFF; 8],
        vec![0xFF; 64],
        vec![0x80; 33],
    ];
    // randomized, including non-ASCII high bytes
    for len in 0..40usize {
        for _ in 0..8 {
            let mut v: Vec<u8> = (0..len).map(|_| 1 + (rng.next_u32() as u8 % 255)).collect();
            v.retain(|&c| c != 0);
            cases.push(v);
        }
    }

    for c in cases {
        let mut buf = c.clone();
        buf.push(0);
        for &seed in SEEDS {
            let (a, b) = unsafe {
                let pa = buf.as_mut_ptr() as *mut c_char;
                (
                    (p.c.hash_string)(pa, seed),
                    (p.rs.hash_string)(pa, seed),
                )
            };
            assert_eq!(a, b, "hash_string seed={seed:#x} s={c:02x?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — stbds_rand_seed + per-table seed advance
// ---------------------------------------------------------------------------

#[test]
fn row05_rand_seed_and_table_seed_chain() {
    let (p, _g) = libs();
    // For each starting seed, create a chain of fresh tables and check both the
    // per-table `seed` and the global seed advance `seed = seed*a + b`.
    let mut extra: Vec<usize> = SEEDS.to_vec();
    let mut rng = Rng::new(0x5000);
    for _ in 0..8 {
        extra.push(rng.next_u64() as usize);
    }
    for &seed in &extra {
        reseed(p, seed);
        for step in 0..8 {
            let tc = unsafe { (p.c.shmode_func)(16, SH_ARENA) };
            let tr = unsafe { (p.rs.shmode_func)(16, SH_ARENA) };
            let sc = unsafe { snap_hm(tc, 16, KeyKind::Bytes) };
            let sr = unsafe { snap_hm(tr, 16, KeyKind::Bytes) };
            assert_eq!(sc, sr, "table seed chain seed={seed:#x} step={step}");
            unsafe {
                (p.c.hmfree_func)(hash_to_arr(tc, 16), 16);
                (p.rs.hmfree_func)(hash_to_arr(tr, 16), 16);
            }
        }
    }
    reseed(p, DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// Rows 6-9 — stbds_arrgrowf
// ---------------------------------------------------------------------------

unsafe fn grow_snap(lib: &Lib, elemsize: usize, addlen: usize, min_cap: usize) -> ArrSnap {
    let a = (lib.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
    if a.is_null() {
        return snap_arr(std::ptr::null_mut(), elemsize, KeyKind::Bytes);
    }
    // do not read elements: the freshly-realloc'd payload is uninitialised
    let h = header(a);
    let s = ArrSnap {
        is_null: false,
        length: (*h).length,
        capacity: (*h).capacity,
        temp: (*h).temp,
        idx: None,
        elems: Vec::new(),
    };
    assert!((*h).hash_table.is_null(), "fresh array must have no index");
    (lib.arrfreef)(a);
    s
}

#[test]
fn row06_arrgrowf_noop_from_null() {
    let (p, _g) = libs();
    // min_len = 0, min_cap = 0 => `min_cap <= arrcap(NULL)` => returns `a` (NULL)
    for &elemsize in &[0usize, 1, 4, 8, 16, 20, 64] {
        let a = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        let b = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        assert!(a.is_null(), "C arrgrowf(NULL,{elemsize},0,0) should be NULL");
        assert!(b.is_null(), "RS arrgrowf(NULL,{elemsize},0,0) should be NULL");
    }
}

#[test]
fn row07_arrgrowf_from_null_grid() {
    let (p, _g) = libs();
    for &elemsize in &[1usize, 2, 4, 8, 16, 20, 64] {
        for &addlen in &[0usize, 1, 2, 3, 4, 5, 7, 8, 63, 64, 65] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 8, 63, 64, 65, 1024] {
                if addlen == 0 && min_cap == 0 {
                    continue; // row 6
                }
                let sc = unsafe { grow_snap(&p.c, elemsize, addlen, min_cap) };
                let sr = unsafe { grow_snap(&p.rs, elemsize, addlen, min_cap) };
                assert_eq!(sc, sr, "arrgrowf(NULL,{elemsize},{addlen},{min_cap})");
            }
        }
    }
}

/// Grow the same array in lockstep in both libraries, writing a deterministic
/// payload after each step so that the element bytes are comparable.
#[test]
fn row08_arrgrowf_growth_chain() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x8000);
    for &elemsize in &[1usize, 4, 8, 16, 20] {
        for trial in 0..24 {
            let mut ac: *mut c_void = std::ptr::null_mut();
            let mut ar: *mut c_void = std::ptr::null_mut();
            let mut len: usize = 0;
            for step in 0..24 {
                let addlen = rng.below(6);
                let min_cap = if rng.bool() { 0 } else { rng.below(40) };
                unsafe {
                    ac = (p.c.arrgrowf)(ac, elemsize, addlen, min_cap);
                    ar = (p.rs.arrgrowf)(ar, elemsize, addlen, min_cap);
                    assert_eq!(ac.is_null(), ar.is_null());
                    if ac.is_null() {
                        continue;
                    }
                    // emulate stbds_arraddn: bump length, then fill new elements
                    let cap = (*header(ac)).capacity;
                    let newlen = (len + addlen).min(cap);
                    (*header(ac)).length = newlen;
                    (*header(ar)).length = newlen;
                    for i in len..newlen {
                        let v = ((i * 7 + step * 13 + trial) % 251) as u8;
                        std::ptr::write_bytes((ac as *mut u8).add(i * elemsize), v, elemsize);
                        std::ptr::write_bytes((ar as *mut u8).add(i * elemsize), v, elemsize);
                    }
                    len = newlen;
                    let sc = snap_arr(ac, elemsize, KeyKind::Bytes);
                    let sr = snap_arr(ar, elemsize, KeyKind::Bytes);
                    assert_eq!(
                        sc, sr,
                        "arrgrowf chain elemsize={elemsize} trial={trial} step={step} \
                         addlen={addlen} min_cap={min_cap}"
                    );
                }
            }
            unsafe {
                if !ac.is_null() {
                    (p.c.arrfreef)(ac);
                }
                if !ar.is_null() {
                    (p.rs.arrfreef)(ar);
                }
            }
        }
    }
}

#[test]
fn row09_arrgrowf_branch_boundaries() {
    let (p, _g) = libs();
    // exercise `min_cap < 2*cap`, `min_cap == 2*cap`, `min_cap > 2*cap`,
    // and the `min_cap < 4` clamp, on top of an existing capacity.
    for &elemsize in &[0usize, 1, 8, 20] {
        for &start_cap in &[1usize, 2, 3, 4, 5, 8, 16] {
            let mut ac = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, start_cap) };
            let mut ar = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, start_cap) };
            let cap = unsafe { (*header(ac)).capacity };
            for delta in -2i64..=2 {
                for &base in &[cap, 2 * cap, 4] {
                    let target = (base as i64 + delta).max(0) as usize;
                    unsafe {
                        let cap_before = (*header(ac)).capacity;
                        let len_before = (*header(ac)).length;
                        // the exact C no-op condition:
                        //   min_cap = max(min_cap, arrlen(a)+addlen); min_cap <= arrcap(a)
                        let noop = target.max(len_before) <= cap_before;
                        let bc = (p.c.arrgrowf)(ac, elemsize, 0, target);
                        let br = (p.rs.arrgrowf)(ar, elemsize, 0, target);
                        let sc = ArrSnap {
                            is_null: bc.is_null(),
                            length: (*header(bc)).length,
                            capacity: (*header(bc)).capacity,
                            temp: (*header(bc)).temp,
                            idx: None,
                            elems: Vec::new(),
                        };
                        let sr = ArrSnap {
                            is_null: br.is_null(),
                            length: (*header(br)).length,
                            capacity: (*header(br)).capacity,
                            temp: (*header(br)).temp,
                            idx: None,
                            elems: Vec::new(),
                        };
                        assert_eq!(
                            sc, sr,
                            "arrgrowf boundary elemsize={elemsize} cap={cap} target={target}"
                        );
                        // On the no-op branch the C returns the *input* pointer
                        // without touching the allocator; both must do that.
                        // (In the growing branch `realloc` may or may not move
                        // the block, which is an allocator artifact, so pointer
                        // identity carries no information there.)
                        if noop {
                            assert!(
                                std::ptr::eq(bc as *const u8, ac as *const u8),
                                "C arrgrowf should have been a no-op \
                                 (elemsize={elemsize} cap={cap_before} target={target})"
                            );
                            assert!(
                                std::ptr::eq(br as *const u8, ar as *const u8),
                                "RS arrgrowf should have been a no-op \
                                 (elemsize={elemsize} cap={cap_before} target={target})"
                            );
                        } else {
                            assert!(
                                sc.capacity > cap_before,
                                "arrgrowf should have grown \
                                 (elemsize={elemsize} cap={cap_before} target={target})"
                            );
                        }
                        ac = bc;
                        ar = br;
                    }
                }
            }
            unsafe {
                (p.c.arrfreef)(ac);
                (p.rs.arrfreef)(ar);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 10-13 — stbds_stralloc / stbds_strreset
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    storage_null: bool,
    chain_len: usize,
    ret: Vec<u8>,
    /// Where the returned pointer sits, expressed structurally (and therefore
    /// independently of the allocator's addresses):
    ///
    /// * `RetLoc::Head` — `ret == a->storage->storage + a->remaining`, the
    ///   normal bump-allocation result (also the dedicated-block case that
    ///   installs a new head and sets `remaining = 0`);
    /// * `RetLoc::Chain(n)` — `ret == chain[n]->storage`, the dedicated block
    ///   spliced in after an existing head;
    /// * `RetLoc::Unknown` — outside every block in the chain.
    ret_loc: RetLoc,
}

#[derive(PartialEq, Eq, Debug)]
enum RetLoc {
    Head,
    Chain(usize),
    Unknown,
}

unsafe fn chain_blocks(a: *const CArena) -> Vec<*const u8> {
    // stbds_string_block { struct stbds_string_block *next; char storage[8]; }
    let mut v = Vec::new();
    let mut x = (*a).storage as *const u8;
    while !x.is_null() {
        v.push(x);
        assert!(v.len() < 10_000, "arena chain looks cyclic");
        x = *(x as *const *const u8);
    }
    v
}

unsafe fn arena_snap(a: *const CArena, ret: *const c_char) -> ArenaSnap {
    let chain = chain_blocks(a);
    let retp = ret as *const u8;
    let mut loc = RetLoc::Unknown;
    if let Some(&head) = chain.first() {
        if retp == head.add(8).add((*a).remaining) {
            loc = RetLoc::Head;
        }
    }
    if loc == RetLoc::Unknown {
        for (i, &blk) in chain.iter().enumerate() {
            if retp == blk.add(8) {
                loc = RetLoc::Chain(i);
                break;
            }
        }
    }
    ArenaSnap {
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        storage_null: (*a).storage.is_null(),
        chain_len: chain.len(),
        ret: {
            let mut v = Vec::new();
            let mut q = retp;
            while *q != 0 {
                v.push(*q);
                q = q.add(1);
            }
            v
        },
        ret_loc: loc,
    }
}

/// Push a sequence of strings through a fresh arena in each library and compare
/// the arena state (and the returned bytes) after every single call.
fn arena_sequence(p: &Pair, strings: &[String], ctx: &str) {
    let mut ac = CArena::zeroed();
    let mut ar = CArena::zeroed();
    for (n, s) in strings.iter().enumerate() {
        let mut buf: Vec<u8> = s.as_bytes().to_vec();
        buf.push(0);
        unsafe {
            let rc = (p.c.stralloc)(&mut ac, buf.as_mut_ptr() as *mut c_char);
            let rr = (p.rs.stralloc)(&mut ar, buf.as_mut_ptr() as *mut c_char);
            let sc = arena_snap(&ac, rc);
            let sr = arena_snap(&ar, rr);
            assert_eq!(sc, sr, "[{ctx}] stralloc #{n} len={}", s.len());
            assert_eq!(sc.ret, s.as_bytes(), "[{ctx}] stralloc #{n} content");
        }
    }
    unsafe {
        (p.c.strreset)(&mut ac);
        (p.rs.strreset)(&mut ar);
        let sc = arena_snap(&ac, b"\0".as_ptr() as *const c_char);
        let sr = arena_snap(&ar, b"\0".as_ptr() as *const c_char);
        assert_eq!(sc, sr, "[{ctx}] strreset");
        assert!(sc.storage_null && sc.remaining == 0 && sc.block == 0 && sc.mode == 0);
    }
}

#[test]
fn row10_stralloc_short_string_sequence() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xA000);
    for trial in 0..12 {
        let n = 400;
        let strings: Vec<String> = (0..n)
            .map(|i| {
                let l = rng.below(40);
                format!("{}{}", "k".repeat(l), i)
            })
            .collect();
        arena_sequence(p, &strings, &format!("short/trial{trial}"));
    }
    // fixed shapes: empty string, exactly-fits, one-past-fits
    arena_sequence(p, &vec!["".to_string(); 8], "empty");
    arena_sequence(
        p,
        &[
            "x".repeat(511),
            "y".repeat(1),
            "z".repeat(1),
            "w".repeat(510),
        ],
        "exact-fit",
    );
}

#[test]
fn row11_stralloc_oversize_on_empty_arena() {
    let (p, _g) = libs();
    // first call, arena empty (block=0 => blocksize=512): len > 512 takes the
    // dedicated-block path with `a->storage == NULL`.
    for &len in &[512usize, 511, 513, 1024, 4096, 100_000] {
        arena_sequence(p, &["q".repeat(len - 1)], &format!("oversize-empty/{len}"));
    }
}

#[test]
fn row12_stralloc_oversize_on_nonempty_arena() {
    let (p, _g) = libs();
    // seed the arena with a small string first so `a->storage != NULL`, then
    // request one bigger than the current blocksize (spliced after the head,
    // `remaining` preserved).
    for &len in &[600usize, 1025, 5000, 70_000] {
        arena_sequence(
            p,
            &[
                "small".to_string(),
                "b".repeat(len),
                "after".to_string(),
                "c".repeat(len * 2),
                "tail".to_string(),
            ],
            &format!("oversize-nonempty/{len}"),
        );
    }
}

#[test]
fn row13_stralloc_block_saturation() {
    let (p, _g) = libs();
    // Force `a->block` all the way to its saturation point (blocksize == 1<<20)
    // by always asking for slightly more than the current block can hold.
    let mut strings: Vec<String> = Vec::new();
    let mut want = 400usize;
    for _ in 0..64 {
        strings.push("s".repeat(want));
        want = (want * 2).min(900_000);
    }
    // then many small ones at saturation
    for i in 0..200 {
        strings.push(format!("small{i}"));
    }
    arena_sequence(p, &strings, "saturation");
}
