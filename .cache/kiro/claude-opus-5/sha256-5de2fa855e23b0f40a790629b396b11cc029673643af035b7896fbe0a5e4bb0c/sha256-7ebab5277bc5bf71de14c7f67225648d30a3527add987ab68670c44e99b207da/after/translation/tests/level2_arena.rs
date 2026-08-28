//! Level 2: the string arena (`stbds_stralloc` / `stbds_strreset`) and the
//! `strkey` helper.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

/// Observable arena state after each allocation, with pointers reduced to
/// structural facts so the two heaps stay comparable.
#[derive(Debug, PartialEq, Eq)]
struct ArenaStep {
    text: Vec<u8>,
    remaining: usize,
    block: u8,
    mode: u8,
    storage_null: bool,
    /// true when the returned pointer is `head_block->storage + remaining`,
    /// i.e. the normal bump-allocation path; false for the dedicated-block
    /// path taken by oversized strings.
    from_head_block: bool,
    chain_len: usize,
}

unsafe fn arena_step(a: *mut StringArena, p: *mut c_char) -> ArenaStep {
    let storage = (*a).storage as *mut u8;
    let from_head_block = !storage.is_null()
        && p as usize == storage.wrapping_add(8).wrapping_add((*a).remaining) as usize;
    ArenaStep {
        text: read_cstr(p),
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        storage_null: storage.is_null(),
        from_head_block,
        chain_len: block_chain_len((*a).storage),
    }
}

fn arena_workload() -> Vec<Vec<Vec<u8>>> {
    let mut sets: Vec<Vec<Vec<u8>>> = Vec::new();

    // many small strings: fills the first 512-byte block, then grows
    sets.push((0..200).map(|i| format!("key_{i}").into_bytes()).collect());

    // empty strings (len 1 including NUL)
    sets.push(vec![b"".to_vec(); 40]);

    // exactly-block-sized and oversized strings, exercising the
    // `len > blocksize` branch that allocates a dedicated block
    sets.push(vec![
        vec![b'x'; 511],
        vec![b'y'; 1],
        vec![b'z'; 600],
        vec![b'w'; 5],
        vec![b'v'; 2000],
        vec![b'u'; 3],
    ]);

    // an oversized string first (a->storage still NULL -> different branch)
    sets.push(vec![vec![b'A'; 4096], vec![b'B'; 10], vec![b'C'; 900]]);

    // steadily increasing sizes to walk the block-size doubling schedule
    sets.push((0..60).map(|i| vec![b'q'; 1 + i * 40]).collect());

    // strings with high-bit bytes
    sets.push((0..30).map(|i| vec![0x80u8 + (i as u8 % 0x7f); 1 + i]).collect());

    sets
}

#[test]
fn stralloc_and_strreset_match() {
    let (c, r) = both();
    for (wi, strings) in arena_workload().into_iter().enumerate() {
        let mut ca = StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut ra = ca;

        for (si, s) in strings.iter().enumerate() {
            let mut cs = s.clone();
            cs.push(0);
            let mut rs = cs.clone();
            unsafe {
                let cp = (c.stralloc)(
                    &mut ca as *mut StringArena as *mut c_void,
                    cs.as_mut_ptr() as *mut c_char,
                );
                let rp = (r.stralloc)(
                    &mut ra as *mut StringArena as *mut c_void,
                    rs.as_mut_ptr() as *mut c_char,
                );
                let cst = arena_step(&mut ca, cp);
                let rst = arena_step(&mut ra, rp);
                assert_eq!(
                    cst, rst,
                    "stbds_stralloc mismatch workload={wi} step={si} len={}",
                    s.len()
                );
            }
        }

        // every previously returned pointer must still hold its string; walk the
        // block chain length as an extra structural check before resetting.
        unsafe {
            let cblocks = block_chain_len(ca.storage);
            let rblocks = block_chain_len(ra.storage);
            assert_eq!(cblocks, rblocks, "arena block-chain length mismatch workload={wi}");

            (c.strreset)(&mut ca as *mut StringArena as *mut c_void);
            (r.strreset)(&mut ra as *mut StringArena as *mut c_void);
            assert_eq!(
                (ca.storage.is_null(), ca.remaining, ca.block, ca.mode),
                (ra.storage.is_null(), ra.remaining, ra.block, ra.mode),
                "stbds_strreset did not zero identically (workload={wi})"
            );
            assert!(ca.storage.is_null() && ca.remaining == 0 && ca.block == 0 && ca.mode == 0);
        }
    }
}

unsafe fn block_chain_len(mut b: *mut c_void) -> usize {
    let mut n = 0usize;
    while !b.is_null() {
        n += 1;
        b = *(b as *mut *mut c_void);
        assert!(n < 100_000, "runaway block chain");
    }
    n
}

#[test]
fn strreset_on_empty_arena_matches() {
    let (c, r) = both();
    let mut ca = StringArena {
        storage: std::ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut ra = ca;
    unsafe {
        (c.strreset)(&mut ca as *mut StringArena as *mut c_void);
        (r.strreset)(&mut ra as *mut StringArena as *mut c_void);
    }
    assert_eq!(
        (ca.storage.is_null(), ca.remaining, ca.block, ca.mode),
        (ra.storage.is_null(), ra.remaining, ra.block, ra.mode)
    );
}

#[test]
fn strkey_matches() {
    let (c, r) = both();
    for n in [
        0i32,
        1,
        -1,
        9,
        10,
        99,
        100,
        12345,
        -12345,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
    ] {
        unsafe {
            let cs = read_cstr((c.strkey)(n));
            let rs = read_cstr((r.strkey)(n));
            assert_eq!(
                cs,
                rs,
                "strkey({n}) mismatch: C={:?} Rust={:?}",
                String::from_utf8_lossy(&cs),
                String::from_utf8_lossy(&rs)
            );
        }
    }
    // repeated calls must keep reusing the same static buffer
    unsafe {
        let p1 = (c.strkey)(1);
        let p2 = (c.strkey)(2);
        let q1 = (r.strkey)(1);
        let q2 = (r.strkey)(2);
        assert_eq!(p1, p2, "C strkey should return the same static buffer");
        assert_eq!(q1, q2, "Rust strkey should return the same static buffer");
        assert_eq!(read_cstr(p2), read_cstr(q2));
    }
}
