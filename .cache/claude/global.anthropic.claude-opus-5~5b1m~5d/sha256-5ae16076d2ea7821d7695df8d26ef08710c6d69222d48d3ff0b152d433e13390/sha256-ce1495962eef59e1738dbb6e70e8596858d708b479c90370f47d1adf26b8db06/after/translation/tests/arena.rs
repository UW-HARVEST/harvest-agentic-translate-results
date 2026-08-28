//! Phase B, Group 10 of `CONFIGS.md`: `stbds_stralloc` / `stbds_strreset`
//! called directly on a caller-owned `stbds_string_arena`.

mod common;

use common::*;
use core::ffi::c_char;

#[derive(Debug, PartialEq, Eq, Clone)]
struct AllocRes {
    arena: ArenaSnap,
    text: Vec<u8>,
    /// `p == a->storage->storage + a->remaining` — true on the normal path,
    /// false on the oversized-dedicated-block path.
    in_current_block: bool,
}

unsafe fn one(lib: &Lib, a: *mut CStringArena, s: &[u8]) -> AllocRes {
    assert_eq!(*s.last().unwrap(), 0, "arena strings must be NUL terminated");
    let p = (lib.stralloc)(a, s.as_ptr() as *mut c_char);
    let snap = snap_arena(a);
    let in_current_block = {
        let st = (*a).storage;
        if st.is_null() {
            false
        } else {
            let base = (&raw const (*st).storage) as *const u8;
            p as *const u8 == base.add((*a).remaining)
        }
    };
    AllocRes {
        arena: snap,
        text: read_cstr(p),
        in_current_block,
    }
}

fn duo(p: &Pair, strings: &[Vec<u8>], label: &str) {
    let mut ca = CStringArena::zeroed();
    let mut ra = CStringArena::zeroed();
    for (i, s) in strings.iter().enumerate() {
        let c = unsafe { one(&p.c, &mut ca, s) };
        let r = unsafe { one(&p.r, &mut ra, s) };
        diff_eq!(c, r, "{label} alloc #{i} (len={})", s.len() - 1);
    }
    // strreset must produce identical (zeroed) arenas
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    let c = unsafe { snap_arena(&ca) };
    let r = unsafe { snap_arena(&ra) };
    diff_eq!(c.clone(), r, "{label} after strreset");
    assert_eq!(
        c,
        ArenaSnap {
            has_storage: false,
            remaining: 0,
            block: 0,
            mode: 0,
            chain_len: 0
        },
        "{label}: strreset must zero the arena"
    );
}

fn s(body: &[u8]) -> Vec<u8> {
    let mut v = body.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// C71 — first allocation into a zeroed arena
// ---------------------------------------------------------------------------
#[test]
fn cfg_c71_first_alloc() {
    let p = libs();
    duo(&p, &[s(b"hello")], "C71 hello");
    duo(&p, &[s(b"")], "C71 empty");
    duo(&p, &[s(b"x")], "C71 one");
    duo(&p, &[s(&vec![b'q'; 511])], "C71 511");
    duo(&p, &[s(&vec![b'q'; 510])], "C71 510");
    duo(&p, &[s(&vec![b'q'; 512])], "C71 512(oversized: len 513 > 512)");
}

// ---------------------------------------------------------------------------
// C72 — fill exactly to remaining == 0 then one more
// ---------------------------------------------------------------------------
#[test]
fn cfg_c72_exact_fill() {
    let p = libs();
    // 512-byte block; 8 x 64-byte strings (len incl. NUL = 64) exactly fills it
    let mut v: Vec<Vec<u8>> = (0..8).map(|_| s(&vec![b'a'; 63])).collect();
    v.push(s(b"one-more"));
    duo(&p, &v, "C72 exact 8x64");

    // 511 + 1 == 512
    duo(&p, &[s(&vec![b'b'; 510]), s(b""), s(b"after")], "C72 510+1+");
    // 1-byte strings until the block is exactly empty, then one more
    let mut w: Vec<Vec<u8>> = (0..512).map(|_| s(b"")).collect();
    w.push(s(b"z"));
    duo(&p, &w, "C72 512x1");
}

// ---------------------------------------------------------------------------
// C73 — 400 random strings, block chain growth
// ---------------------------------------------------------------------------
#[test]
fn cfg_c73_random_strings() {
    let p = libs();
    let mut rng = Rng::new(73);
    for (maxlen, label) in [(20usize, "short"), (60, "mid"), (200, "long")] {
        let v: Vec<Vec<u8>> = (0..400)
            .map(|_| {
                let n = rng.below(maxlen + 1);
                s(&rng.cstr_body(n, false))
            })
            .collect();
        duo(&p, &v, &format!("C73 {label}"));
    }
    // full byte range (incl. >= 0x80)
    let v: Vec<Vec<u8>> = (0..400)
        .map(|_| {
            let n = rng.range(1, 50);
            s(&rng.cstr_body(n, true))
        })
        .collect();
    duo(&p, &v, "C73 highbytes");
}

// ---------------------------------------------------------------------------
// C74 — string longer than the current blocksize
// ---------------------------------------------------------------------------
#[test]
fn cfg_c74_oversized() {
    let p = libs();
    // start a block, then request something bigger than blocksize
    duo(
        &p,
        &[s(b"seed"), s(&vec![b'X'; 5000]), s(b"after"), s(&vec![b'Y'; 600])],
        "C74 mixed",
    );
    // exactly blocksize and blocksize-1 boundaries at each block level
    let mut v = Vec::new();
    v.push(s(b"seed")); // block -> 1, blocksize 512, remaining 512-5
    v.push(s(&vec![b'a'; 511])); // len 512 > remaining 507 -> new block: blocksize=512<<0=512; len 512 <= 512 -> normal
    v.push(s(&vec![b'b'; 512])); // len 513 -> blocksize 512<<1=1024 wait block is now 2 -> 512<<1=1024
    v.push(s(&vec![b'c'; 1024]));
    v.push(s(&vec![b'd'; 1023]));
    duo(&p, &v, "C74 boundaries");
}

// ---------------------------------------------------------------------------
// C75 — oversized as the very first allocation (storage == NULL)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c75_oversized_first() {
    let p = libs();
    for n in [512usize, 513, 1000, 100_000] {
        duo(&p, &[s(&vec![b'Z'; n])], &format!("C75 first oversized {n}"));
        // then keep using the arena (remaining was forced to 0)
        duo(
            &p,
            &[s(&vec![b'Z'; n]), s(b"next"), s(b"and-another"), s(&vec![b'w'; 700])],
            &format!("C75 first oversized {n} + more"),
        );
    }
}

// ---------------------------------------------------------------------------
// C76 — drive a->block from 0 to its saturation value 22
// ---------------------------------------------------------------------------
#[test]
fn cfg_c76_block_saturation() {
    let p = libs();
    // Each string is bigger than the current blocksize would leave, forcing a
    // fresh block every time.  Sizes grow so blocksize keeps doubling
    // (512 << (block>>1)) until it hits 1<<20 and `block` saturates at 22.
    let mut v = Vec::new();
    let mut want = 400usize;
    for _ in 0..30 {
        v.push(s(&vec![b'p'; want]));
        want = (want * 3 / 2) + 1;
        if want > 3_000_000 {
            want = 3_000_000;
        }
    }
    let mut ca = CStringArena::zeroed();
    let mut ra = CStringArena::zeroed();
    let mut max_block_c = 0u8;
    for (i, ss) in v.iter().enumerate() {
        let c = unsafe { one(&p.c, &mut ca, ss) };
        let r = unsafe { one(&p.r, &mut ra, ss) };
        diff_eq!(c.clone(), r, "C76 alloc #{i} len={}", ss.len() - 1);
        max_block_c = max_block_c.max(c.arena.block);
    }
    assert!(
        max_block_c >= 22,
        "block should saturate at 22, only reached {max_block_c}"
    );
    // keep going past saturation
    for i in 0..10 {
        let ss = s(&vec![b'q'; 2_000_000]);
        let c = unsafe { one(&p.c, &mut ca, &ss) };
        let r = unsafe { one(&p.r, &mut ra, &ss) };
        diff_eq!(c.clone(), r, "C76 post-saturation #{i}");
        assert_eq!(c.arena.block, 22, "block must stay saturated");
    }
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    diff_eq!(
        unsafe { snap_arena(&ca) },
        unsafe { snap_arena(&ra) },
        "C76 after strreset"
    );
}

// ---------------------------------------------------------------------------
// C77 — 600 empty strings
// ---------------------------------------------------------------------------
#[test]
fn cfg_c77_empty_strings() {
    let p = libs();
    let v: Vec<Vec<u8>> = (0..600).map(|_| s(b"")).collect();
    duo(&p, &v, "C77 600 empties");
}

// ---------------------------------------------------------------------------
// C78 — strreset in every arena shape
// ---------------------------------------------------------------------------
#[test]
fn cfg_c78_strreset_shapes() {
    let p = libs();
    // empty arena
    let mut ca = CStringArena::zeroed();
    let mut ra = CStringArena::zeroed();
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    diff_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) }, "C78 empty");
    // reset twice
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    diff_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) }, "C78 double");

    // 1 block, long chain, and after oversized blocks -- `duo` already resets
    duo(&p, &[s(b"only")], "C78 one block");
    let v: Vec<Vec<u8>> = (0..300).map(|i| s(&vec![b'a'; (i % 40) + 1])).collect();
    duo(&p, &v, "C78 long chain");
    duo(
        &p,
        &[s(&vec![b'A'; 9000]), s(b"x"), s(&vec![b'B'; 4000]), s(b"y")],
        "C78 with oversized",
    );

    // reset, reuse, reset again
    let mut ca = CStringArena::zeroed();
    let mut ra = CStringArena::zeroed();
    for round in 0..5 {
        for i in 0..80 {
            let ss = s(&vec![b'r'; (i % 30) + 1]);
            let c = unsafe { one(&p.c, &mut ca, &ss) };
            let r = unsafe { one(&p.r, &mut ra, &ss) };
            diff_eq!(c, r, "C78 round {round} alloc {i}");
        }
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
        diff_eq!(
            unsafe { snap_arena(&ca) },
            unsafe { snap_arena(&ra) },
            "C78 round {round} reset"
        );
    }
}

// ---------------------------------------------------------------------------
// The arena's `mode` byte is caller-owned data stralloc never touches.
// ---------------------------------------------------------------------------
#[test]
fn cfg_c71b_mode_byte_untouched() {
    let p = libs();
    for mode in [0u8, 1, 2, 3, 200, 255] {
        let mut ca = CStringArena::zeroed();
        let mut ra = CStringArena::zeroed();
        ca.mode = mode;
        ra.mode = mode;
        for i in 0..40 {
            let ss = s(&vec![b'm'; (i % 25) + 1]);
            let c = unsafe { one(&p.c, &mut ca, &ss) };
            let r = unsafe { one(&p.r, &mut ra, &ss) };
            diff_eq!(c.clone(), r, "mode={mode} alloc {i}");
            assert_eq!(c.arena.mode, mode, "stralloc must not touch `mode`");
        }
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
        diff_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) }, "mode={mode} reset");
        assert_eq!(ca.mode, 0, "strreset memsets the whole arena");
    }
}
