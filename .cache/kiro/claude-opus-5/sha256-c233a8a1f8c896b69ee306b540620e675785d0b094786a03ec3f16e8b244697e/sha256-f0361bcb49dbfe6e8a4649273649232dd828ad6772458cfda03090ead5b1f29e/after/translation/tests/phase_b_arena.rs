//! Phase B — CONFIGS.md rows 51-57: `stbds_stralloc`, `stbds_strreset`,
//! `strkey`.

mod common;
use common::*;
use std::ffi::c_void;

/// `stbds_string_arena` is 24 bytes: `{ void *storage; size_t remaining;
/// unsigned char block; unsigned char mode; }`
const ARENA_SIZE: usize = 24;

struct Arena {
    buf: CBuf,
}

impl Arena {
    fn new() -> Arena {
        Arena {
            buf: CBuf::new(&[0u8; ARENA_SIZE]),
        }
    }
    fn ptr(&self) -> *mut c_void {
        self.buf.ptr()
    }
    fn view(&self) -> &StringArena {
        unsafe { &*(self.buf.0 as *const StringArena) }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    has_storage: bool,
    block_count: usize,
}

fn arena_snap(a: &Arena) -> ArenaSnap {
    let v = a.view();
    let mut n = 0usize;
    unsafe {
        let mut x = v.storage as *const *const c_void;
        while !x.is_null() {
            n += 1;
            assert!(n < 100_000, "block chain cycle");
            x = *x as *const *const c_void;
        }
    }
    ArenaSnap {
        remaining: v.remaining,
        block: v.block,
        mode: v.mode,
        has_storage: !v.storage.is_null(),
        block_count: n,
    }
}

/// Exact location of a returned string pointer relative to the arena's block
/// chain.  Pointer *values* differ between the two libraries, but the
/// structural location must be identical.
#[derive(Debug, PartialEq, Eq)]
enum Loc {
    /// carved out of the head block's tail: `head->storage + remaining`
    HeadTail(usize),
    /// start of the dedicated block at chain index `i` (`sb->storage`)
    BlockStart(usize),
    Unknown,
}

fn chain(a: &Arena) -> Vec<*const u8> {
    let mut v = Vec::new();
    unsafe {
        let mut x = a.view().storage as *const u8;
        while !x.is_null() {
            v.push(x);
            assert!(v.len() < 100_000, "block chain cycle");
            x = *(x as *const *const u8);
        }
    }
    v
}

fn locate(a: &Arena, p: *const u8) -> Loc {
    let blocks = chain(a);
    if blocks.is_empty() {
        return Loc::Unknown;
    }
    unsafe {
        let head_storage = blocks[0].add(8);
        if p == head_storage.add(a.view().remaining) {
            return Loc::HeadTail(a.view().remaining);
        }
        for (i, blk) in blocks.iter().enumerate() {
            if p == blk.add(8) {
                return Loc::BlockStart(i);
            }
        }
    }
    Loc::Unknown
}

fn alloc_both(b: &Both, ca: &Arena, ra: &Arena, s: &[u8], tag: &str) {
    let key = CBuf::new(s);
    unsafe {
        let cp = (b.c.stralloc)(ca.ptr(), key.cptr());
        let rp = (b.r.stralloc)(ra.ptr(), key.cptr());
        assert!(!cp.is_null() && !rp.is_null(), "{tag}: null return");
        assert_eq!(cstr(cp), cstr(rp), "{tag}: contents");
        assert_eq!(&cstr(cp)[..], &s[..s.len() - 1], "{tag}: round-trip");
        assert_eq!(arena_snap(ca), arena_snap(ra), "{tag}: arena state");
        let cl = locate(ca, cp as *const u8);
        let rl = locate(ra, rp as *const u8);
        assert_eq!(cl, rl, "{tag}: returned pointer location");
        assert_ne!(cl, Loc::Unknown, "{tag}: pointer outside the block chain");
    }
}

/// rows 51-55 and ERRORS.md #35-#39
#[test]
fn row_51_to_55_stralloc() {
    let (_g, b) = both();
    let mut rng = Rng::new(0x510);

    // row 51: fresh arena, many short strings (fast path + new-block path)
    for trial in 0..6u64 {
        let ca = Arena::new();
        let ra = Arena::new();
        let mut rng2 = Rng::new(0x511 ^ trial);
        for i in 0..300 {
            let l = 1 + rng2.below(60);
            let s = rng2.cstring(l);
            alloc_both(b, &ca, &ra, &s, &format!("row51 trial={trial} i={i} len={l}"));
        }
        unsafe {
            (b.c.strreset)(ca.ptr());
            (b.r.strreset)(ra.ptr());
        }
        assert_eq!(arena_snap(&ca), arena_snap(&ra), "row51 after reset");
    }

    // row 55 / ERRORS.md #39: empty strings only
    {
        let ca = Arena::new();
        let ra = Arena::new();
        for i in 0..600 {
            alloc_both(b, &ca, &ra, b"\0", &format!("row55 i={i}"));
        }
        unsafe {
            (b.c.strreset)(ca.ptr());
            (b.r.strreset)(ra.ptr());
        }
        assert_eq!(arena_snap(&ca), arena_snap(&ra));
    }

    // row 52 / ERRORS.md #36: first string longer than the 512-byte block
    for len in [511usize, 512, 513, 1000, 4096, 100_000] {
        let ca = Arena::new();
        let ra = Arena::new();
        let s: Vec<u8> = (0..len).map(|_| 1 + (rng.byte() % 250)).chain([0]).collect();
        alloc_both(b, &ca, &ra, &s, &format!("row52 first-big len={len}"));
        // then a few small ones on top
        for i in 0..20 {
            let l = 1 + rng.below(40);
            let t = rng.cstring(l);
            alloc_both(b, &ca, &ra, &t, &format!("row52 follow len={len} i={i}"));
        }
        unsafe {
            (b.c.strreset)(ca.ptr());
            (b.r.strreset)(ra.ptr());
        }
        assert_eq!(arena_snap(&ca), arena_snap(&ra), "row52 after reset len={len}");
    }

    // row 53 / ERRORS.md #37: existing block, then an oversized string
    for big in [600usize, 2000, 40_000] {
        let ca = Arena::new();
        let ra = Arena::new();
        alloc_both(b, &ca, &ra, b"seed\0", "row53 seed");
        let s: Vec<u8> = (0..big).map(|_| 1 + (rng.byte() % 250)).chain([0]).collect();
        alloc_both(b, &ca, &ra, &s, &format!("row53 big={big}"));
        for i in 0..10 {
            let l = 1 + rng.below(30);
            let t = rng.cstring(l);
            alloc_both(b, &ca, &ra, &t, &format!("row53 follow big={big} i={i}"));
        }
        unsafe {
            (b.c.strreset)(ca.ptr());
            (b.r.strreset)(ra.ptr());
        }
        assert_eq!(arena_snap(&ca), arena_snap(&ra));
    }
}

/// row 54 / ERRORS.md #38 — drive `a->block` all the way to the `1<<20`
/// saturation point.  Each iteration requests exactly the current block size
/// so `remaining` lands on 0 and the next call allocates a bigger block.
#[test]
fn row_54_block_growth_to_saturation() {
    let (_g, b) = both();
    let ca = Arena::new();
    let ra = Arena::new();
    for i in 0..30usize {
        let block = ca.view().block as usize;
        assert_eq!(block, ra.view().block as usize, "row54 block desync at i={i}");
        let blocksize = 512usize << (block >> 1);
        // len == blocksize  =>  strlen == blocksize - 1
        let len = blocksize - 1;
        let s: Vec<u8> = std::iter::repeat(b'x').take(len).chain([0]).collect();
        alloc_both(
            b,
            &ca,
            &ra,
            &s,
            &format!("row54 i={i} block={block} blocksize={blocksize}"),
        );
    }
    // saturation: 512 << (22>>1) == 1<<20, which is not < 1<<20, so `block`
    // stops at 22 in both libraries.
    assert_eq!(ca.view().block, ra.view().block);
    assert_eq!(ca.view().block, 22, "expected saturation at block==22");
    unsafe {
        (b.c.strreset)(ca.ptr());
        (b.r.strreset)(ra.ptr());
    }
    assert_eq!(arena_snap(&ca), arena_snap(&ra));

    // and an oversized request while already saturated
    let ca = Arena::new();
    let ra = Arena::new();
    for i in 0..24usize {
        let block = ca.view().block as usize;
        let blocksize = 512usize << (block >> 1);
        let s: Vec<u8> = std::iter::repeat(b'y').take(blocksize + 5).chain([0]).collect();
        alloc_both(b, &ca, &ra, &s, &format!("row54b i={i} block={block}"));
    }
    assert_eq!(arena_snap(&ca), arena_snap(&ra));
    unsafe {
        (b.c.strreset)(ca.ptr());
        (b.r.strreset)(ra.ptr());
    }
    assert_eq!(arena_snap(&ca), arena_snap(&ra));
}

/// row 56 / ERRORS.md #40 — `stbds_strreset`
#[test]
fn row_56_strreset() {
    let (_g, b) = both();
    // already-empty arena
    let ca = Arena::new();
    let ra = Arena::new();
    for _ in 0..3 {
        unsafe {
            (b.c.strreset)(ca.ptr());
            (b.r.strreset)(ra.ptr());
        }
        assert_eq!(arena_snap(&ca), arena_snap(&ra), "strreset on empty arena");
        assert_eq!(arena_snap(&ca).block_count, 0);
    }

    // 1 block, then many blocks
    for n in [1usize, 2, 5, 40, 200] {
        let ca = Arena::new();
        let ra = Arena::new();
        let mut rng = Rng::new(0x560 ^ n as u64);
        for i in 0..n {
            // alternate small and oversized so both block kinds end up chained
            let l = if i % 3 == 0 { 700 + rng.below(200) } else { 1 + rng.below(50) };
            let s = rng.cstring(l);
            alloc_both(b, &ca, &ra, &s, &format!("row56 n={n} i={i}"));
        }
        let before = arena_snap(&ca);
        assert_eq!(before, arena_snap(&ra));
        assert!(before.block_count > 0);
        unsafe {
            (b.c.strreset)(ca.ptr());
            (b.r.strreset)(ra.ptr());
        }
        let after = arena_snap(&ca);
        assert_eq!(after, arena_snap(&ra));
        assert_eq!(
            after,
            ArenaSnap {
                remaining: 0,
                block: 0,
                mode: 0,
                has_storage: false,
                block_count: 0
            },
            "strreset must zero the arena"
        );
    }
}

/// row 57 — `strkey`
#[test]
fn row_57_strkey() {
    let (_g, b) = both();
    let mut vals: Vec<i32> = vec![
        0,
        1,
        9,
        10,
        99,
        100,
        999,
        12345,
        -1,
        -9,
        -12345,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    let mut rng = Rng::new(0x570);
    for _ in 0..200 {
        vals.push(rng.next_u64() as i32);
    }
    for n in vals {
        unsafe {
            let cp = (b.c.strkey)(n);
            let rp = (b.r.strkey)(n);
            let cs = cstr(cp);
            let rs = cstr(rp);
            assert_eq!(cs, rs, "strkey({n})");
            assert_eq!(cs, format!("test_{n}").into_bytes(), "strkey({n}) value");
        }
    }
    // the returned pointer must be the same static buffer on repeat calls
    unsafe {
        let a = (b.c.strkey)(1);
        let a2 = (b.c.strkey)(2);
        let r = (b.r.strkey)(1);
        let r2 = (b.r.strkey)(2);
        assert_eq!(a, a2, "C strkey must reuse its static buffer");
        assert_eq!(r, r2, "Rust strkey must reuse its static buffer");
    }
}
