//! `stbds_stralloc` / `stbds_strreset` / `strkey`.

mod common;

use common::*;
use std::ffi::c_char;

unsafe fn chain_len(a: &StringArena) -> usize {
    let mut n = 0usize;
    let mut x = a.storage;
    while !x.is_null() {
        n += 1;
        x = (*x).next;
        assert!(n < 100_000, "string arena chain looks cyclic");
    }
    n
}

fn describe(a: &StringArena, chain: usize) -> String {
    format!(
        "remaining={} block={} mode={} storage={} chain={}",
        a.remaining,
        a.block,
        a.mode,
        if a.storage.is_null() { "nil" } else { "set" },
        chain
    )
}

/// True when `p` sits exactly at `a->storage->storage + a->remaining`, i.e. the
/// bump-down allocation came out of the current head block. False for the
/// oversized-block path when a head block already existed.
unsafe fn from_head_block(a: &StringArena, p: *mut c_char) -> bool {
    if a.storage.is_null() {
        return false;
    }
    let base = (*a.storage).storage.as_ptr() as usize;
    (p as usize) == base + a.remaining
}

/// Runs the same sequence of `stralloc` calls against both libraries and
/// compares every observable: returned contents, arena bookkeeping, chain
/// length, where the returned pointer sits inside the head block, and the
/// integrity of all earlier strings.
fn run_stralloc_sequence(label: &str, strings: &[Vec<u8>]) {
    let p = load_pair();
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();

        let mut cout: Vec<*mut c_char> = Vec::new();
        let mut rout: Vec<*mut c_char> = Vec::new();

        for (i, s) in strings.iter().enumerate() {
            let mut buf = cbuf_bytes(s);
            let cp = (p.c.stralloc)(&mut ca, buf.as_mut_ptr());
            let rp = (p.r.stralloc)(&mut ra, buf.as_mut_ptr());

            assert_eq!(cstr(cp), cstr(rp), "{label}: alloc #{i} contents");
            assert_eq!(
                cstr(cp),
                String::from_utf8_lossy(s),
                "{label}: alloc #{i} payload"
            );

            let cch = chain_len(&ca);
            let rch = chain_len(&ra);
            assert_eq!(
                describe(&ca, cch),
                describe(&ra, rch),
                "{label}: arena state after alloc #{i}"
            );

            // Exact placement inside the head block (`p == storage + remaining`).
            assert_eq!(
                from_head_block(&ca, cp),
                from_head_block(&ra, rp),
                "{label}: head-block placement after alloc #{i}"
            );

            cout.push(cp);
            rout.push(rp);

            // every previously returned string must still be intact
            for (j, s2) in strings[..=i].iter().enumerate() {
                let want = String::from_utf8_lossy(s2).into_owned();
                assert_eq!(cstr(cout[j]), want, "{label}: C clobbered string #{j}");
                assert_eq!(cstr(rout[j]), want, "{label}: Rust clobbered string #{j}");
            }
        }

        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
        assert_eq!(
            describe(&ca, chain_len(&ca)),
            describe(&ra, chain_len(&ra)),
            "{label}: arena state after strreset"
        );
        assert!(ca.storage.is_null() && ca.remaining == 0 && ca.block == 0 && ca.mode == 0);
        assert!(ra.storage.is_null() && ra.remaining == 0 && ra.block == 0 && ra.mode == 0);
    }
}

#[test]
fn stralloc_small_strings() {
    let strings: Vec<Vec<u8>> = (0..400).map(|i| format!("test_{i}").into_bytes()).collect();
    run_stralloc_sequence("small", &strings);
}

#[test]
fn stralloc_empty_and_tiny() {
    let mut strings: Vec<Vec<u8>> = Vec::new();
    for i in 0..200 {
        strings.push(vec![b'x'; i % 5]);
    }
    run_stralloc_sequence("tiny", &strings);
}

#[test]
fn stralloc_block_boundaries() {
    // Sizes that straddle the 512-byte minimum block and force the
    // `len > blocksize` oversized-block path in both orders.
    let mut strings: Vec<Vec<u8>> = Vec::new();
    for &n in &[
        1usize, 510, 511, 512, 513, 1023, 1024, 1025, 2047, 2048, 5, 4096, 7, 100_000, 3, 1,
    ] {
        strings.push(vec![b'q'; n]);
    }
    run_stralloc_sequence("boundaries", &strings);
}

#[test]
fn stralloc_growing_blocks() {
    // Keep allocating until `a->block` saturates so the `blocksize` shift and the
    // BLOCKSIZE_MAX clamp are both exercised.
    let mut strings: Vec<Vec<u8>> = Vec::new();
    let mut n = 1usize;
    for _ in 0..48 {
        strings.push(vec![b'z'; n]);
        n = (n * 2).min(1 << 21);
    }
    run_stralloc_sequence("growing", &strings);
}

#[test]
fn stralloc_random_sizes() {
    let mut rng = Rng::new(0xABCD_1234);
    let strings: Vec<Vec<u8>> = (0..600)
        .map(|_| {
            let n = (rng.next_u32() % 700) as usize;
            let c = b'A' + (rng.next_u32() % 26) as u8;
            vec![c; n]
        })
        .collect();
    run_stralloc_sequence("random", &strings);
}

#[test]
fn strreset_on_zeroed_arena() {
    let p = load_pair();
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
        assert_eq!(describe(&ca, 0), describe(&ra, 0));
    }
}

/// `strkey` writes into a file-static buffer, so every test that touches it must
/// be serialised against the others in this (single-process) test binary.
static STRKEY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn strkey_matches() {
    let _guard = STRKEY_LOCK.lock().unwrap();
    let p = load_pair();
    let mut cases: Vec<i32> = vec![
        0,
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
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    let mut rng = Rng::new(0x55AA_55AA);
    for _ in 0..2000 {
        cases.push(rng.next_u32() as i32);
    }

    for n in cases {
        unsafe {
            let cp = (p.c.strkey)(n);
            let rp = (p.r.strkey)(n);
            assert_eq!(cstr(cp), format!("test_{n}"), "C strkey({n})");
            assert_eq!(cstr(rp), cstr(cp), "strkey({n})");
        }
    }
}

#[test]
fn strkey_reuses_static_buffer() {
    let _guard = STRKEY_LOCK.lock().unwrap();
    // Both implementations must return the *same* pointer every call (a file
    // static buffer), so an earlier result is overwritten by a later call.
    let p = load_pair();
    unsafe {
        let a = (p.c.strkey)(1);
        let b = (p.c.strkey)(22222);
        assert_eq!(a, b, "C strkey buffer moved");
        let a = (p.r.strkey)(1);
        let b = (p.r.strkey)(22222);
        assert_eq!(a, b, "Rust strkey buffer moved");
        assert_eq!(cstr(a), "test_22222");
    }
}

#[test]
fn stralloc_with_strkey_like_str_put() {
    let _guard = STRKEY_LOCK.lock().unwrap();
    // Mirrors the loop inside `str_put`.
    let p = load_pair();
    for num in [0i32, 1, 2, 5, 50, 500] {
        unsafe {
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            for i in 0..num {
                let cp = (p.c.stralloc)(&mut ca, (p.c.strkey)(i));
                let rp = (p.r.stralloc)(&mut ra, (p.r.strkey)(i));
                assert_eq!(cstr(cp), cstr(rp), "num={num} i={i}");
            }
            assert_eq!(
                describe(&ca, chain_len(&ca)),
                describe(&ra, chain_len(&ra)),
                "num={num} arena state"
            );
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
    }
}
