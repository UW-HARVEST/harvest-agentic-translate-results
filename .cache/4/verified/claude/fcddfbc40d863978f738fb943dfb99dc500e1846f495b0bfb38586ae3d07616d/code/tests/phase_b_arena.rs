//! Phase B — valid-path differential tests, CONFIGS.md rows 37..44.
//! `stbds_stralloc`, `stbds_strreset`, `strkey`, `str_put`.
mod common;

use common::*;
use std::ffi::{c_char, CStr};

// ---------------------------------------------------------------------------
// Observable, address-independent state of a string arena
// ---------------------------------------------------------------------------

unsafe fn chain_len(a: &Arena) -> usize {
    unsafe {
        let mut n = 0usize;
        let mut b = a.storage as *const StringBlock;
        while !b.is_null() {
            n += 1;
            b = (*b).next as *const StringBlock;
            assert!(n < 100_000, "arena chain loop");
        }
        n
    }
}

/// Everything about an arena that must match between the two libraries.
unsafe fn arena_state(a: &Arena, p: *mut c_char) -> String {
    unsafe {
        // Is the returned pointer the freshly-carved tail of the current block?
        let in_current = !a.storage.is_null()
            && (p as usize) == (a.storage as usize) + 8 + a.remaining;
        format!(
            "remaining={} block={} mode={} storage={} chain={} in_current={} str={:?}",
            a.remaining,
            a.block,
            a.mode,
            if a.storage.is_null() { "no" } else { "yes" },
            chain_len(a),
            in_current,
            if p.is_null() {
                Vec::new()
            } else {
                CStr::from_ptr(p).to_bytes().to_vec()
            }
        )
    }
}

/// Drive `stbds_stralloc` on both libraries with the same string list and
/// compare the arena state after every call. Every previously returned string
/// is re-verified so that block reuse cannot silently clobber older data.
fn arena_run(label: &str, strings: &[Vec<u8>], start_block: u8, reset_at_end: bool) {
    let l = libs();
    let mut ca = Arena::zeroed();
    let mut ra = Arena::zeroed();
    ca.block = start_block;
    ra.block = start_block;
    let mut cptrs: Vec<(*mut c_char, Vec<u8>)> = Vec::new();
    let mut rptrs: Vec<(*mut c_char, Vec<u8>)> = Vec::new();
    unsafe {
        for (i, sb) in strings.iter().enumerate() {
            let mut owned = sb.clone();
            owned.push(0);
            let cp = (l.c.stralloc)(&mut ca, owned.as_ptr() as *mut c_char);
            let rp = (l.r.stralloc)(&mut ra, owned.as_ptr() as *mut c_char);
            let cs = arena_state(&ca, cp);
            let rs = arena_state(&ra, rp);
            assert_eq!(
                cs, rs,
                "{label}: divergence at stralloc #{i} (len={})\nC   : {cs}\nRUST: {rs}",
                sb.len()
            );
            cptrs.push((cp, sb.clone()));
            rptrs.push((rp, sb.clone()));
            // all earlier strings must still read back correctly in both
            for (p, want) in &cptrs {
                assert_eq!(
                    CStr::from_ptr(*p).to_bytes(),
                    &want[..],
                    "{label}: C clobbered an earlier arena string (after #{i})"
                );
            }
            for (p, want) in &rptrs {
                assert_eq!(
                    CStr::from_ptr(*p).to_bytes(),
                    &want[..],
                    "{label}: RUST clobbered an earlier arena string (after #{i})"
                );
            }
        }
        if reset_at_end {
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
            let cs = arena_state(&ca, std::ptr::null_mut());
            let rs = arena_state(&ra, std::ptr::null_mut());
            assert_eq!(cs, rs, "{label}: strreset divergence\nC: {cs}\nRUST: {rs}");
            assert_eq!(ca.remaining, 0);
            assert_eq!(ca.block, 0);
            assert_eq!(ca.mode, 0);
            assert!(ca.storage.is_null());
            assert_eq!(ra.remaining, 0);
            assert_eq!(ra.block, 0);
            assert_eq!(ra.mode, 0);
            assert!(ra.storage.is_null());
        }
    }
}

// ---------------------------------------------------------------------------
// row 37 — fresh arena, many short strings: carve from the current block and
//          walk the `block` counter / `remaining` bookkeeping
// ---------------------------------------------------------------------------
#[test]
fn cfg_37_arena_short_strings() {
    let mut rng = Rng::new(0xB0_0037);
    for &count in &[0usize, 1, 2, 3, 63, 64, 65, 200, 2000] {
        let strings: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let n = 1 + rng.below(40);
                rng.cstr_bytes(n, false)
            })
            .collect();
        arena_run(&format!("short count={count}"), &strings, 0, true);
    }
    // strings that exactly fill / just overflow the 512-byte first block
    for &len in &[1usize, 7, 8, 63, 255, 256, 510, 511, 512] {
        let strings: Vec<Vec<u8>> = (0..40).map(|_| vec![b'x'; len]).collect();
        arena_run(&format!("uniform len={len}"), &strings, 0, true);
    }
}

// ---------------------------------------------------------------------------
// row 38 — first call with len > blocksize (a->storage == NULL)
// ---------------------------------------------------------------------------
#[test]
fn cfg_38_arena_oversize_first() {
    for &len in &[512usize, 513, 1000, 4096, 100_000] {
        // len+1 > 512 on the very first call => dedicated block, remaining = 0
        let strings = vec![vec![b'A'; len]];
        arena_run(&format!("oversize-first len={len}"), &strings, 0, true);
    }
    // then keep allocating after the oversized first block
    for &len in &[600usize, 5000] {
        let mut strings = vec![vec![b'A'; len]];
        for i in 0..30 {
            strings.push(vec![b'b' + (i % 20) as u8; 1 + i as usize]);
        }
        arena_run(&format!("oversize-first-then-small len={len}"), &strings, 0, true);
    }
}

// ---------------------------------------------------------------------------
// row 39 — len > blocksize with a->storage != NULL (spliced block)
// ---------------------------------------------------------------------------
#[test]
fn cfg_39_arena_oversize_later() {
    let mut rng = Rng::new(0xB0_0039);
    for &big in &[520usize, 1024, 8000, 70_000] {
        let mut strings: Vec<Vec<u8>> = Vec::new();
        strings.push(b"seed-the-first-block".to_vec());
        for round in 0..6 {
            strings.push(vec![b'Z'; big + round]);
            for _ in 0..5 {
                let n = 1 + rng.below(30);
                strings.push(rng.cstr_bytes(n, false));
            }
        }
        arena_run(&format!("oversize-later big={big}"), &strings, 0, true);
    }
    // interleave many oversized allocations so the splice runs repeatedly
    let mut strings: Vec<Vec<u8>> = vec![b"x".to_vec()];
    for i in 0..40usize {
        strings.push(vec![b'Q'; 600 + i * 37]);
    }
    arena_run("oversize-interleaved", &strings, 0, true);
}

// ---------------------------------------------------------------------------
// row 40 — pre-set `a->block`: `512 << (block>>1)` shift masking / wrap-to-0
//          and the `< 1<<20` saturation
// ---------------------------------------------------------------------------
#[test]
fn cfg_40_arena_block_counter() {
    // `blocksize = (size_t)512 << (block>>1)`:
    //   * reaches the 1<<20 MAX at block>>1 == 11, i.e. block 22/23, after
    //     which `++a->block` stops;
    //   * `block>>1 >= 55` makes `512 << n` overflow to 0, so every string
    //     takes the dedicated-block path;
    //   * `block >= 128` gives `block>>1 >= 64`, which is C UB — x86-64 `shlq`
    //     masks the count to 6 bits, so block 128 behaves like block 0
    //     (blocksize 512 again). Testing 128..=145 therefore *proves* the
    //     masking, and 238..=255 covers the masked-to-wrap-to-0 region.
    //
    // Starts whose blocksize is huge but non-zero (block>>1 in ~19..54, and
    // 166..=237 after masking) make the C `realloc` fail and both libraries
    // then dereference NULL — identical behaviour, but it kills the test
    // process, so those live in the Phase C subprocess tests instead.
    let mut starts: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 20, 21, 22, 23, 24, 25, 26, 27, 28,
                                  29, 30, 31];
    starts.extend(110u8..=145);
    starts.extend(238u8..=255);
    for start in starts {
        let strings: Vec<Vec<u8>> = vec![
            b"short".to_vec(),
            vec![b'm'; 100],
            vec![b'n'; 600],
            b"tiny".to_vec(),
            vec![b'o'; 3000],
            b"z".to_vec(),
        ];
        arena_run(&format!("block-start={start}"), &strings, start, true);
    }
    // a long run from each interesting start so the counter keeps walking
    for start in [0u8, 20, 22, 30, 126, 128, 254] {
        let mut rng = Rng::new(0xB0_0040 ^ start as u64);
        let strings: Vec<Vec<u8>> = (0..400)
            .map(|_| {
                let n = 1 + rng.below(80);
                rng.cstr_bytes(n, false)
            })
            .collect();
        arena_run(&format!("block-walk start={start}"), &strings, start, true);
    }
}

// ---------------------------------------------------------------------------
// row 41 — strreset over empty / 1-block / many-block / mixed arenas,
//          then reuse the same arena struct
// ---------------------------------------------------------------------------
#[test]
fn cfg_41_strreset() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0041);

    // empty arena: strreset must be a no-op that zeroes the struct
    unsafe {
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        ca.block = 7;
        ca.remaining = 0;
        ra.block = 7;
        ra.remaining = 0;
        (l.c.strreset)(&mut ca);
        (l.r.strreset)(&mut ra);
        assert_eq!(
            arena_state(&ca, std::ptr::null_mut()),
            arena_state(&ra, std::ptr::null_mut())
        );
        assert_eq!(ca.block, 0);
        assert_eq!(ra.block, 0);
    }

    // reuse the same arena struct across several fill/reset cycles
    unsafe {
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        for cycle in 0..8usize {
            let count = 1 + rng.below(60);
            for i in 0..count {
                let n = if (cycle + i) % 11 == 0 {
                    600 + rng.below(2000)
                } else {
                    1 + rng.below(50)
                };
                let mut s = rng.cstr_bytes(n, false);
                s.push(0);
                let cp = (l.c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
                let rp = (l.r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
                assert_eq!(
                    arena_state(&ca, cp),
                    arena_state(&ra, rp),
                    "reuse cycle {cycle} alloc {i}"
                );
            }
            (l.c.strreset)(&mut ca);
            (l.r.strreset)(&mut ra);
            assert_eq!(
                arena_state(&ca, std::ptr::null_mut()),
                arena_state(&ra, std::ptr::null_mut()),
                "reuse cycle {cycle} reset"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 42 — strkey over the whole int range (incl. INT_MIN / INT_MAX)
// ---------------------------------------------------------------------------
#[test]
fn cfg_42_strkey() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0042);
    let mut ns: Vec<i32> = vec![
        0,
        1,
        2,
        9,
        10,
        11,
        99,
        100,
        999,
        1000,
        -1,
        -9,
        -10,
        -99,
        -100,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    for _ in 0..2000 {
        ns.push(rng.next_u32() as i32);
    }
    unsafe {
        for n in ns {
            let cp = (l.c.strkey)(n);
            let rp = (l.r.strkey)(n);
            let cs = CStr::from_ptr(cp).to_bytes().to_vec();
            let rs = CStr::from_ptr(rp).to_bytes().to_vec();
            assert_eq!(
                cs,
                rs,
                "strkey({n}): C={:?} RUST={:?}",
                String::from_utf8_lossy(&cs),
                String::from_utf8_lossy(&rs)
            );
            assert_eq!(cs, format!("test_{n}").into_bytes());
            // the pointer must be the same static buffer on every call
            assert_eq!(cp, (l.c.strkey)(n));
            assert_eq!(rp, (l.r.strkey)(n));
        }
    }
}

// ---------------------------------------------------------------------------
// row 43 — str_put: stdout compared byte for byte
// ---------------------------------------------------------------------------
#[test]
fn cfg_43_str_put_stdout() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0043);
    let mut nums: Vec<i32> = vec![0, 1, 2, 3, 4, 7, 8, 9, 16, 63, 64, 100, 1000, 5000, -1, -2,
                                 -100, -1000, i32::MIN];
    for _ in 0..40 {
        nums.push((rng.next_u32() % 4000) as i32);
    }
    for num in nums {
        for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
            let cout = capture_stdout(|| unsafe {
                (l.c.rand_seed)(seed);
                (l.c.str_put)(num);
            });
            let rout = capture_stdout(|| unsafe {
                (l.r.rand_seed)(seed);
                (l.r.str_put)(num);
            });
            assert_eq!(
                cout,
                rout,
                "str_put({num}) seed={seed:#x}\nC   : {:?}\nRUST: {:?}",
                String::from_utf8_lossy(&cout),
                String::from_utf8_lossy(&rout)
            );
            assert_eq!(cout, format!("a {num}\n").into_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// row 44 — str_put called repeatedly in one process: the static `buffer` is
//          reused and the global hash seed keeps advancing
// ---------------------------------------------------------------------------
#[test]
fn cfg_44_str_put_repeated() {
    let l = libs();
    let mut rng = Rng::new(0xB0_0044);
    for trial in 0..6 {
        let nums: Vec<i32> = (0..20).map(|i| (rng.next_u32() % 300) as i32 + i).collect();
        let seed = if trial == 0 {
            0x3141_5926
        } else {
            rng.next_u64() as usize
        };
        let cout = capture_stdout(|| unsafe {
            (l.c.rand_seed)(seed);
            for &n in &nums {
                (l.c.str_put)(n);
            }
        });
        let rout = capture_stdout(|| unsafe {
            (l.r.rand_seed)(seed);
            for &n in &nums {
                (l.r.str_put)(n);
            }
        });
        assert_eq!(
            cout,
            rout,
            "20-call str_put sequence (trial {trial}) diverged\nC   : {:?}\nRUST: {:?}",
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout)
        );
        let expect: String = nums.iter().map(|n| format!("a {n}\n")).collect();
        assert_eq!(cout, expect.into_bytes());
    }
}
