//! Phase B rows 46–52 — the string arena (`stbds_stralloc` / `stbds_strreset`).
//!
//! `stbds_string_arena` is caller-owned, so both libraries can be driven with
//! byte-identical starting states and their post-call arena fields compared
//! directly, along with the contents of every string handed back.

mod common;
use common::*;
use std::ffi::c_char;

unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut out = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        out.push(*q);
        q = q.add(1);
    }
    out
}

/// Drive both arenas with the same string list and compare arena state plus the
/// content of every returned pointer after each step.
unsafe fn drive(c: &Impl, r: &Impl, strings: &[Vec<u8>], ctx: &str) {
    let mut ac = StringArena::new();
    let mut ar = StringArena::new();
    let mut pc: Vec<*mut c_char> = Vec::new();
    let mut pr: Vec<*mut c_char> = Vec::new();

    for (i, s) in strings.iter().enumerate() {
        let qc = (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
        let qr = (r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
        assert!(!qc.is_null() && !qr.is_null());
        assert_eq!(
            read_cstr(qc),
            s[..s.len() - 1].to_vec(),
            "{}: C returned the wrong content at #{}",
            ctx,
            i
        );
        assert_eq!(
            read_cstr(qr),
            read_cstr(qc),
            "{}: content mismatch at #{}",
            ctx,
            i
        );
        assert_eq!(
            snap_arena(&ac),
            snap_arena(&ar),
            "{}: arena state diverged after #{} (len={})",
            ctx,
            i,
            s.len()
        );
        pc.push(qc);
        pr.push(qr);
        // every previously returned string must still be intact and identical
        for (j, (a, b)) in pc.iter().zip(pr.iter()).enumerate() {
            assert_eq!(
                read_cstr(*a),
                read_cstr(*b),
                "{}: earlier string #{} corrupted after #{}",
                ctx,
                j,
                i
            );
        }
    }

    (c.strreset)(&mut ac);
    (r.strreset)(&mut ar);
    assert_eq!(snap_arena(&ac), snap_arena(&ar), "{}: after strreset", ctx);
    assert_eq!(snap_arena(&ac), snap_arena(&StringArena::new()));
}

/// CONFIGS row 46 — fresh arena, one short string.
#[test]
fn cfg_46_stralloc_first_short() {
    run(1, |c, r| unsafe {
        for len in [0usize, 1, 2, 7, 8, 31, 100, 400, 510, 511, 512] {
            let mut rng = Rng::new(0x5eed_b_0000 + len as u64);
            drive(c, r, &[rng.ascii(len)], &format!("row46 len={}", len));
        }
    });
}

/// CONFIGS row 47 — many short strings, overflowing the 512-byte block repeatedly.
#[test]
fn cfg_47_stralloc_block_overflow() {
    run(1, |c, r| unsafe {
        for trial in 0..6u64 {
            let mut rng = Rng::new(0x5eed_c_0000 + trial);
            let strings: Vec<Vec<u8>> = (0..300)
                .map(|_| {
                    let n = 1 + rng.below(40);
                    rng.ascii(n)
                })
                .collect();
            drive(c, r, &strings, &format!("row47 trial={}", trial));
        }
    });
}

/// CONFIGS row 48 — oversized string on an empty arena.
#[test]
fn cfg_48_stralloc_oversized_first() {
    run(1, |c, r| unsafe {
        for len in [512usize, 513, 600, 1023, 1024, 1025, 4096, 100_000] {
            let mut rng = Rng::new(0x5eed_d_0000 + len as u64);
            drive(c, r, &[rng.ascii(len)], &format!("row48 len={}", len));
        }
    });
}

/// CONFIGS row 49 — oversized string on a non-empty arena
/// (spliced in after the head block; `remaining` deliberately untouched).
#[test]
fn cfg_49_stralloc_oversized_after() {
    run(1, |c, r| unsafe {
        for big in [513usize, 1025, 2048, 9000] {
            let mut rng = Rng::new(0x5eed_e_0000 + big as u64);
            let mut strings = vec![rng.ascii(10), rng.ascii(20)];
            strings.push(rng.ascii(big));
            strings.push(rng.ascii(15));
            strings.push(rng.ascii(big + 7));
            strings.push(rng.ascii(30));
            drive(c, r, &strings, &format!("row49 big={}", big));
        }
    });
}

/// CONFIGS row 50 — interleaved small/oversized, 200 randomized lengths 0..4096.
#[test]
fn cfg_50_stralloc_interleaved_random() {
    run(1, |c, r| unsafe {
        for trial in 0..6u64 {
            let mut rng = Rng::new(0x5eed_f_0000 + trial);
            let strings: Vec<Vec<u8>> = (0..200)
                .map(|_| {
                    let len = match rng.below(4) {
                        0 => rng.below(16),
                        1 => rng.below(600),
                        2 => rng.below(1500),
                        _ => rng.below(4096),
                    };
                    rng.ascii(len)
                })
                .collect();
            drive(c, r, &strings, &format!("row50 trial={}", trial));
        }
    });
}

/// CONFIGS row 51 — drive `a->block` to saturation (`512 << (block>>1) >= 1 MiB`).
///
/// Every allocation is sized to just overflow the current block, forcing a new
/// block each time; `block` must stop incrementing at the same value in both.
#[test]
fn cfg_51_stralloc_block_saturation() {
    run(1, |c, r| unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        let mut rng = Rng::new(0x5eed_1_1111);
        for step in 0..70usize {
            // current blocksize per the C formula
            let blocksize = 512usize << (ac.block as usize >> 1);
            // a string that fits the *new* block but not the remaining space
            let len = (blocksize / 2).max(1);
            let s = rng.ascii(len);
            let qc = (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            let qr = (r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_eq!(read_cstr(qc), read_cstr(qr), "row51 content step {}", step);
            assert_eq!(
                snap_arena(&ac),
                snap_arena(&ar),
                "row51 arena state step {} (blocksize={})",
                step,
                blocksize
            );
        }
        assert_eq!(ac.block, ar.block);
        assert_eq!(
            512usize << (ac.block as usize >> 1) >= (1 << 20),
            true,
            "expected block to reach saturation, got block={}",
            ac.block
        );
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_eq!(snap_arena(&ac), snap_arena(&ar));
    });
}

/// CONFIGS row 52 — reset a multi-block arena and reuse it.
#[test]
fn cfg_52_strreset_and_reuse() {
    run(1, |c, r| unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        let mut rng = Rng::new(0x5eed_1_2222);
        for cycle in 0..5usize {
            for i in 0..120usize {
                let n = 1 + rng.below(if i % 17 == 0 { 2000 } else { 50 });
                let s = rng.ascii(n);
                let qc = (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
                let qr = (r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
                assert_eq!(read_cstr(qc), read_cstr(qr));
                assert_eq!(
                    snap_arena(&ac),
                    snap_arena(&ar),
                    "row52 cycle {} step {}",
                    cycle,
                    i
                );
            }
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
            assert_eq!(snap_arena(&ac), snap_arena(&ar), "row52 reset cycle {}", cycle);
            assert_eq!(snap_arena(&ac), snap_arena(&StringArena::new()));
        }
    });
}
