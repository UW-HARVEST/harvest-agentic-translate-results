//! Phase B rows 65–70: `stbds_stralloc` driven directly through the public
//! `stbds_string_arena *` signature.

mod common;
use common::*;
use std::ffi::c_char;

/// Structural fingerprint of an arena + the result of one allocation.
/// Absolute pointers differ between the two libraries, so we record the
/// *structure*: which block the result landed in, its offset from that block's
/// storage, and the arena bookkeeping.
#[derive(Debug, PartialEq, Eq)]
struct Probe {
    /// (length, FNV-1a hash) of the returned string — keeps failure output small
    content: (usize, u64),
    remaining: usize,
    block: u8,
    mode: u8,
    nblocks: usize,
    /// index of the block whose `storage` field the result points at, if any
    block_index: Option<usize>,
    /// offset of the result from the head block's `storage`. Only recorded when
    /// the result is a *carve* out of the head block; when the result is a whole
    /// block's `storage` the distance between two independent `malloc`s is not a
    /// behavioural property and must not be compared.
    head_offset: Option<isize>,
    is_null: bool,
}

fn fnv(b: &[u8]) -> (usize, u64) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    (b.len(), h)
}

fn blocks(a: &StringArena) -> Vec<*mut StringBlock> {
    let mut v = Vec::new();
    let mut b = a.storage;
    while !b.is_null() && v.len() < 100_000 {
        v.push(b);
        b = unsafe { (*b).next };
    }
    v
}

fn probe(a: &StringArena, p: *mut c_char) -> Probe {
    let bl = blocks(a);
    let mut block_index = None;
    for (i, &b) in bl.iter().enumerate() {
        let st = unsafe { (&raw mut (*b).storage) as *mut c_char };
        if st == p {
            block_index = Some(i);
            break;
        }
    }
    let head_offset = bl.first().map(|&b| unsafe {
        let st = (&raw mut (*b).storage) as *mut c_char;
        (p as isize) - (st as isize)
    });
    Probe {
        content: if p.is_null() { (0, 0) } else { fnv(unsafe { std::ffi::CStr::from_ptr(p).to_bytes() }) },
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        nblocks: bl.len(),
        block_index,
        head_offset: if block_index.is_some() { None } else { head_offset },
        is_null: p.is_null(),
    }
}

/// Run the same sequence of `stralloc` calls on both libraries, comparing the
/// probe after every call.
fn run_seq(ctx: &str, init: StringArena, strings: &mut [Vec<u8>]) {
    let (c, r) = libs();
    let mut ac = init;
    let mut ar = init;
    unsafe {
        for (i, s) in strings.iter_mut().enumerate() {
            let p = s.as_mut_ptr() as *mut c_char;
            let pc = (c.stralloc)(&mut ac, p);
            let pr = (r.stralloc)(&mut ar, p);
            let qc = probe(&ac, pc);
            let qr = probe(&ar, pr);
            assert_eq!(qc, qr, "{ctx}: divergence at alloc #{i} (len={})", s.len());
            assert_eq!(qc.content, fnv(&s[..s.len() - 1]), "{ctx}: content at #{i}");
        }
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }
}

// --- row 65 ---------------------------------------------------------------

#[test]
fn row65_fresh_arena_lengths_0_40() {
    let mut rng = Rng::new(0x6501);
    for len in 0usize..=40 {
        let mut v = vec![rng.cstring(len, ASCII)];
        run_seq(&format!("row65 len={len}"), StringArena::zeroed(), &mut v);
    }
    // and empty strings repeatedly
    let mut v: Vec<Vec<u8>> = (0..40).map(|_| vec![0u8]).collect();
    run_seq("row65 empty x40", StringArena::zeroed(), &mut v);
}

// --- row 66 ---------------------------------------------------------------

#[test]
fn row66_sequential_allocs_exhaust_blocks() {
    let mut rng = Rng::new(0x6601);
    for &n in &[10usize, 60, 300] {
        let mut v: Vec<Vec<u8>> = (0..n)
            .map(|_| {
                let l = 1 + rng.below(60);
                rng.cstring(l, ASCII)
            })
            .collect();
        run_seq(&format!("row66 n={n}"), StringArena::zeroed(), &mut v);
    }
    // exactly-fitting allocations: 512-byte block, 511-byte payload + NUL
    let mut v: Vec<Vec<u8>> = (0..6).map(|_| rng.cstring(511, ASCII)).collect();
    run_seq("row66 exact-fit 512", StringArena::zeroed(), &mut v);
}

// --- row 67 ---------------------------------------------------------------

#[test]
fn row67_oversized_first_alloc() {
    let mut rng = Rng::new(0x6701);
    for &len in &[512usize, 513, 600, 1024, 4096, 100_000] {
        let mut v = vec![rng.cstring(len, ASCII)];
        run_seq(&format!("row67 len={len}"), StringArena::zeroed(), &mut v);
    }
}

// --- row 68 ---------------------------------------------------------------

#[test]
fn row68_oversized_after_normal_block() {
    let mut rng = Rng::new(0x6801);
    // small alloc first (creates the 512-byte head block), then oversized ones
    // which must be spliced in *after* the head
    let mut v = vec![
        rng.cstring(10, ASCII),
        rng.cstring(5000, ASCII),
        rng.cstring(12, ASCII),
        rng.cstring(9000, ASCII),
        rng.cstring(3, ASCII),
        rng.cstring(600, ASCII),
    ];
    run_seq("row68", StringArena::zeroed(), &mut v);
}

// --- row 69: pre-set `block` counter, incl. shifts >= 64 -----------------

#[test]
fn row69_preset_block_counter() {
    // blocksize = (size_t)512 << (block >> 1)
    //   block <= 21 -> blocksize < 1<<20, so `block` is incremented
    //   block >= 22 -> blocksize >= 1<<20, so `block` saturates
    //   block == 110/111 -> shift 55 -> 2^64 wraps to 0 (and the x86 shift is
    //                       masked mod 64 for block >= 128)
    let presets: [u8; 20] = [0, 1, 2, 3, 4, 5, 10, 11, 20, 21, 22, 23, 24, 25, 30, 110, 111, 112, 254, 255];
    let mut rng = Rng::new(0x6901);
    for &blk in &presets {
        for &len in &[1usize, 7, 40, 600] {
            let init = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: blk, mode: 0 };
            let mut v = vec![rng.cstring(len, ASCII)];
            run_seq(&format!("row69 block={blk} len={len}"), init, &mut v);
        }
    }
}

#[test]
fn row69b_block_saturation_boundary() {
    let (c, r) = libs();
    let mut rng = Rng::new(0x6902);
    for blk in 0u8..=30 {
        unsafe {
            let mut ac = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: blk, mode: 0 };
            let mut ar = ac;
            let mut s = rng.cstring(4, ASCII);
            let p = s.as_mut_ptr() as *mut c_char;
            (c.stralloc)(&mut ac, p);
            (r.stralloc)(&mut ar, p);
            let ctx = format!("row69b block={blk}");
            assert_eq!(ac.block, ar.block, "{ctx}: block counter");
            assert_eq!(ac.remaining, ar.remaining, "{ctx}: remaining");
            let expect_inc = blk <= 21;
            assert_eq!(
                ac.block,
                if expect_inc { blk + 1 } else { blk },
                "{ctx}: expected saturation above 21"
            );
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    }
}

// --- row 70: long run driving `block` toward saturation -------------------

#[test]
fn row70_long_run_to_saturation() {
    let (c, r) = libs();
    let mut rng = Rng::new(0x7001);
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        let mut maxblk = 0u8;
        for i in 0..30_000usize {
            let l = 1 + rng.below(120);
            let mut s = rng.cstring(l, ASCII);
            let p = s.as_mut_ptr() as *mut c_char;
            let pc = (c.stralloc)(&mut ac, p);
            let pr = (r.stralloc)(&mut ar, p);
            assert_eq!(ac.block, ar.block, "row70: block counter at #{i}");
            assert_eq!(ac.remaining, ar.remaining, "row70: remaining at #{i}");
            assert_eq!(blocks(&ac).len(), blocks(&ar).len(), "row70: block count at #{i}");
            let cc = fnv(std::ffi::CStr::from_ptr(pc).to_bytes());
            let cr = fnv(std::ffi::CStr::from_ptr(pr).to_bytes());
            assert_eq!(cc, cr, "row70: content at #{i}");
            assert_eq!(cc, fnv(&s[..s.len() - 1]), "row70: content matches input at #{i}");
            maxblk = maxblk.max(ac.block);
        }
        assert!(maxblk >= 12, "row70: expected the block counter to climb, got {maxblk}");
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }
}
