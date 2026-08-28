//! Level 2: the string arena - `stbds_stralloc` and `stbds_strreset`.
//!
//! Both are exported and are the only consumers of `stbds_string_block`, so
//! they can be driven directly. Pointers differ between the two libraries, so
//! the returned pointer is compared via a *classification* relative to the
//! arena's block chain, which pins down the exact offset arithmetic.

mod harness;

use harness::*;
use std::ffi::c_char;

#[derive(Debug, PartialEq, Eq)]
enum Placement {
    /// `p == head->storage + a->remaining` (the normal bump-down path).
    InHeadAtRemaining,
    /// `p == head->storage` and the head block is a dedicated oversized block.
    HeadStart,
    /// `p == head->next->storage`: oversized block spliced in behind the head.
    SecondBlockStart,
    Unclassified,
}

unsafe fn classify(a: *const StringArena, p: *const c_char) -> Placement {
    let head = (*a).storage;
    if head.is_null() {
        return Placement::Unclassified;
    }
    let head_data = (*head).storage.as_ptr();
    if p == head_data.add((*a).remaining) {
        return Placement::InHeadAtRemaining;
    }
    if p == head_data {
        return Placement::HeadStart;
    }
    let second = (*head).next;
    if !second.is_null() && p == (*second).storage.as_ptr() {
        return Placement::SecondBlockStart;
    }
    Placement::Unclassified
}

/// Replays one sequence of `stralloc` calls on both libraries, comparing the
/// arena state, the placement classification and the contents of *every*
/// string handed out so far after each step.
fn replay(label: &str, strings: &[Vec<u8>]) {
    let p = pair();
    let mut ca = StringArena::new();
    let mut ra = StringArena::new();
    let mut c_ptrs: Vec<*mut c_char> = Vec::new();
    let mut r_ptrs: Vec<*mut c_char> = Vec::new();

    for (step, s) in strings.iter().enumerate() {
        let expect = &s[..s.len() - 1];
        let sp = s.as_ptr() as *mut c_char;
        unsafe {
            let cp = p.c.stralloc(&mut ca, sp);
            let rp = p.rs.stralloc(&mut ra, sp);

            assert_eq!(
                snapshot_arena(&ca),
                snapshot_arena(&ra),
                "{}: arena state after step {} (len {})",
                label,
                step,
                expect.len()
            );
            assert_eq!(
                classify(&ca, cp),
                classify(&ra, rp),
                "{}: placement of step {} (len {})",
                label,
                step,
                expect.len()
            );
            assert_eq!(
                read_cstr(cp),
                expect,
                "{}: C returned wrong contents at step {}",
                label,
                step
            );
            assert_eq!(
                read_cstr(rp),
                expect,
                "{}: Rust returned wrong contents at step {}",
                label,
                step
            );

            c_ptrs.push(cp);
            r_ptrs.push(rp);
            // every earlier allocation must still be intact and identical
            for (i, (&cq, &rq)) in c_ptrs.iter().zip(r_ptrs.iter()).enumerate() {
                let want = &strings[i][..strings[i].len() - 1];
                assert_eq!(read_cstr(cq), want, "{}: C clobbered string {}", label, i);
                assert_eq!(read_cstr(rq), want, "{}: Rust clobbered string {}", label, i);
            }
        }
    }

    unsafe {
        p.c.strreset(&mut ca);
        p.rs.strreset(&mut ra);
    }
    assert_eq!(
        unsafe { snapshot_arena(&ca) },
        unsafe { snapshot_arena(&ra) },
        "{}: arena state after strreset",
        label
    );
    let zero = ArenaSnapshot {
        has_storage: false,
        remaining: 0,
        block: 0,
        mode: 0,
        block_chain_len: 0,
    };
    assert_eq!(unsafe { snapshot_arena(&ca) }, zero, "{}: C not reset", label);
    assert_eq!(unsafe { snapshot_arena(&ra) }, zero, "{}: Rust not reset", label);
}

fn s(n: usize, fill: u8) -> Vec<u8> {
    let mut v = vec![fill; n];
    v.push(0);
    v
}

#[test]
fn stralloc_short_strings() {
    // exactly what sh_puts does: strkey(0..n)
    let strings: Vec<Vec<u8>> = (0..300).map(|i| cstring(&format!("test_{}", i))).collect();
    replay("strkey-like", &strings);
}

#[test]
fn stralloc_empty_and_tiny() {
    let mut strings = Vec::new();
    for i in 0..200 {
        strings.push(s(i % 5, b'x' + (i % 3) as u8));
    }
    replay("tiny", &strings);
}

#[test]
fn stralloc_block_boundaries() {
    // 512 is STBDS_STRING_ARENA_BLOCKSIZE_MIN; walk right across it.
    let mut strings = Vec::new();
    for n in [
        1usize, 510, 511, 512, 513, 1, 1023, 1024, 1025, 2047, 2048, 2049, 1, 2, 3,
    ] {
        strings.push(s(n, b'A'));
    }
    replay("boundaries", &strings);
}

#[test]
fn stralloc_forces_repeated_oversized_blocks() {
    // len 600 alternates between the oversized path (blocksize 512) and the
    // normal path once blocksize has doubled past it.
    let strings: Vec<Vec<u8>> = (0..40).map(|i| s(600 + i, b'Z')).collect();
    replay("oversized-600", &strings);
}

#[test]
fn stralloc_exact_remaining_fit() {
    // Drive `len == a->remaining` exactly, the boundary of `len > a->remaining`.
    let p = pair();
    let mut ca = StringArena::new();
    let mut ra = StringArena::new();
    unsafe {
        let first = s(10, b'q');
        p.c.stralloc(&mut ca, first.as_ptr() as *mut c_char);
        p.rs.stralloc(&mut ra, first.as_ptr() as *mut c_char);
        assert_eq!(snapshot_arena(&ca), snapshot_arena(&ra));

        for _ in 0..4 {
            let rem = ca.remaining;
            assert_eq!(rem, ra.remaining);
            assert!(rem > 0);
            let exact = s(rem - 1, b'e'); // len == rem
            let cp = p.c.stralloc(&mut ca, exact.as_ptr() as *mut c_char);
            let rp = p.rs.stralloc(&mut ra, exact.as_ptr() as *mut c_char);
            assert_eq!(snapshot_arena(&ca), snapshot_arena(&ra), "exact fit");
            assert_eq!(classify(&ca, cp), classify(&ra, rp), "exact fit placement");
            assert_eq!(read_cstr(cp), read_cstr(rp));
            assert_eq!(ca.remaining, 0);

            // one more byte than fits: forces a fresh block
            let over = s(1, b'o');
            let cp = p.c.stralloc(&mut ca, over.as_ptr() as *mut c_char);
            let rp = p.rs.stralloc(&mut ra, over.as_ptr() as *mut c_char);
            assert_eq!(snapshot_arena(&ca), snapshot_arena(&ra), "after overflow");
            assert_eq!(classify(&ca, cp), classify(&ra, rp), "overflow placement");
            assert_eq!(read_cstr(cp), read_cstr(rp));
        }
        p.c.strreset(&mut ca);
        p.rs.strreset(&mut ra);
    }
}

#[test]
fn stralloc_saturates_block_counter() {
    // Each oversized allocation bumps `a->block` until blocksize reaches
    // STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20), i.e. block == 22.
    let big = 1_100_000usize; // > 1<<20, so always the oversized path
    let p = pair();
    let mut ca = StringArena::new();
    let mut ra = StringArena::new();
    let payload = s(big, b'M');
    unsafe {
        for step in 0..26 {
            let cp = p.c.stralloc(&mut ca, payload.as_ptr() as *mut c_char);
            let rp = p.rs.stralloc(&mut ra, payload.as_ptr() as *mut c_char);
            assert_eq!(
                snapshot_arena(&ca),
                snapshot_arena(&ra),
                "block counter step {}",
                step
            );
            assert_eq!(classify(&ca, cp), classify(&ra, rp), "step {}", step);
            assert_eq!(read_cstr(cp).len(), big);
            assert_eq!(read_cstr(rp).len(), big);
        }
        assert_eq!(ca.block, 22, "block counter should saturate at 22");
        assert_eq!(ra.block, ca.block);
        p.c.strreset(&mut ca);
        p.rs.strreset(&mut ra);
        assert_eq!(snapshot_arena(&ca), snapshot_arena(&ra));
    }
}

#[test]
fn strreset_on_pristine_arena() {
    let p = pair();
    let mut ca = StringArena::new();
    let mut ra = StringArena::new();
    // non-zero mode/block must also be cleared
    ca.mode = 3;
    ca.block = 7;
    ra.mode = 3;
    ra.block = 7;
    unsafe {
        p.c.strreset(&mut ca);
        p.rs.strreset(&mut ra);
        assert_eq!(snapshot_arena(&ca), snapshot_arena(&ra));
        assert_eq!(ca.mode, 0);
        assert_eq!(ra.mode, 0);
        assert_eq!(ca.block, 0);
        assert_eq!(ra.block, 0);
    }
}
