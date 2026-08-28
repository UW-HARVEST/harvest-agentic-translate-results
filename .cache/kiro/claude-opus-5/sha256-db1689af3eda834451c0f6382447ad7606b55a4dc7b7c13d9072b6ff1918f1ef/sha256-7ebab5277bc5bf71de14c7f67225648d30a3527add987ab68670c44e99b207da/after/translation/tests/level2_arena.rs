//! Level 2: the string arena (`stbds_stralloc` / `stbds_strreset`) and the
//! global seed setter (`stbds_rand_seed`).
mod harness;

use harness::*;
use std::os::raw::c_char;

/// Allocation-independent description of one `stbds_stralloc` result.
#[derive(PartialEq, Eq, Debug)]
struct AllocSnap {
    /// the string that was actually stored (length + checksum, to keep
    /// assertion messages readable)
    content_len: usize,
    content_sum: u64,
    /// arena bookkeeping after the call
    remaining: usize,
    block: u8,
    mode: u8,
    blocks: usize,
    /// true when the result sits at `storage->storage + remaining` of the
    /// arena's *current* head block (the normal path); false for the
    /// oversized-string path that returns a freshly chained block.
    in_head_block: bool,
    /// byte offset of the result inside the head block, but only for the
    /// in-head-block path — for the oversized path the result lives in a
    /// different malloc'd block, so the delta is allocator noise.
    offset_in_head: Option<isize>,
}

fn fnv(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

unsafe fn snap(a: *mut StringArena, ret: *mut c_char) -> AllocSnap {
    unsafe {
        let ar = *a;
        let head_storage = if ar.storage.is_null() {
            std::ptr::null_mut()
        } else {
            (&raw mut (*ar.storage).storage) as *mut c_char
        };
        let expected = if head_storage.is_null() {
            std::ptr::null_mut()
        } else {
            head_storage.add(ar.remaining)
        };
        let in_head_block = ret == expected;
        let content = cstr(ret).unwrap();
        AllocSnap {
            content_len: content.len(),
            content_sum: fnv(&content),
            remaining: ar.remaining,
            block: ar.block,
            mode: ar.mode,
            blocks: arena_block_count(a),
            in_head_block,
            offset_in_head: if in_head_block {
                Some((ret as isize) - (head_storage as isize))
            } else {
                None
            },
        }
    }
}

fn cbuf(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

#[test]
fn stralloc_sequences_match() {
    let p = pair();

    // Sequences chosen to exercise: fresh arena, block-size doubling
    // (`a->block >> 1` shift), the `len > blocksize` oversized path with and
    // without an existing head block, and exact-fit boundaries.
    let mut cases: Vec<Vec<Vec<u8>>> = Vec::new();

    // simple ramp
    cases.push((0..40).map(|i| cbuf(format!("test_{i}").as_bytes())).collect());

    // strings that exactly fill / overflow the 512-byte first block
    cases.push(vec![
        cbuf(&vec![b'x'; 511]),
        cbuf(&vec![b'y'; 1]),
        cbuf(&vec![b'z'; 1]),
    ]);
    cases.push(vec![cbuf(&vec![b'x'; 512]), cbuf(&vec![b'y'; 1])]);
    // oversized first allocation (len > 512) on a *fresh* arena
    cases.push(vec![cbuf(&vec![b'q'; 600]), cbuf(b"tail")]);
    // small alloc first (creates head block), then oversized
    cases.push(vec![
        cbuf(b"small"),
        cbuf(&vec![b'Q'; 5000]),
        cbuf(b"after"),
        cbuf(&vec![b'R'; 100000]),
        cbuf(b"end"),
    ]);
    // many mid-size strings to walk `block` up several notches
    cases.push(
        (0..64)
            .map(|i| cbuf(&vec![b'a' + (i % 26) as u8; 200 + (i * 37) % 900]))
            .collect(),
    );
    // empty strings
    cases.push(vec![cbuf(b""); 20]);
    // alternate tiny / huge
    cases.push(vec![
        cbuf(b"a"),
        cbuf(&vec![b'H'; 1 << 21]),
        cbuf(b"b"),
        cbuf(&vec![b'I'; 1 << 21]),
        cbuf(b"c"),
    ]);

    for (ci, case) in cases.iter().enumerate() {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        for (si, s) in case.iter().enumerate() {
            let mut cs = s.clone();
            let mut rs = s.clone();
            let cr = unsafe { (p.c.stralloc)(&raw mut ca, cs.as_mut_ptr() as *mut c_char) };
            let rr = unsafe { (p.rs.stralloc)(&raw mut ra, rs.as_mut_ptr() as *mut c_char) };
            let csn = unsafe { snap(&raw mut ca, cr) };
            let rsn = unsafe { snap(&raw mut ra, rr) };
            assert_eq!(csn, rsn, "case {ci} step {si}: stralloc mismatch");
        }
        // strreset must free the chain and zero the struct identically
        unsafe {
            (p.c.strreset)(&raw mut ca);
            (p.rs.strreset)(&raw mut ra);
        }
        assert_eq!(
            (ca.storage.is_null(), ca.remaining, ca.block, ca.mode),
            (ra.storage.is_null(), ra.remaining, ra.block, ra.mode),
            "case {ci}: strreset mismatch"
        );
        assert!(ca.storage.is_null() && ca.remaining == 0 && ca.block == 0 && ca.mode == 0);
    }
}

/// Previously-stored strings must stay readable and unchanged as the arena
/// grows — this catches wrong pointer arithmetic inside a block.
#[test]
fn stralloc_previous_strings_stay_intact() {
    let p = pair();
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    let mut cps: Vec<*mut c_char> = Vec::new();
    let mut rps: Vec<*mut c_char> = Vec::new();
    let mut expect: Vec<Vec<u8>> = Vec::new();

    for i in 0..300 {
        let s = format!("string-{i}-{}", "p".repeat(i % 71));
        let mut b1 = cbuf(s.as_bytes());
        let mut b2 = cbuf(s.as_bytes());
        cps.push(unsafe { (p.c.stralloc)(&raw mut ca, b1.as_mut_ptr() as *mut c_char) });
        rps.push(unsafe { (p.rs.stralloc)(&raw mut ra, b2.as_mut_ptr() as *mut c_char) });
        expect.push(s.into_bytes());
    }
    for i in 0..expect.len() {
        let cv = unsafe { cstr(cps[i]) }.unwrap();
        let rv = unsafe { cstr(rps[i]) }.unwrap();
        assert_eq!(cv, expect[i], "C arena corrupted at {i}");
        assert_eq!(rv, expect[i], "Rust arena corrupted at {i}");
    }
    unsafe {
        (p.c.strreset)(&raw mut ca);
        (p.rs.strreset)(&raw mut ra);
    }
}

/// `stbds_rand_seed` is observable through the seed baked into a freshly
/// created hash index, and through the LCG that advances it.
#[test]
fn rand_seed_propagation_matches() {
    let _g = global_lock();
    let p = pair();
    let elemsize = std::mem::size_of::<IntEntry>();
    for seed in [
        0usize,
        1,
        0x31415926,
        0xffff_ffff_ffff_ffff,
        0xdead_beef,
        1 << 63,
    ] {
        unsafe {
            (p.c.rand_seed)(seed);
            (p.rs.rand_seed)(seed);
        }
        // Each shmode_func creates a fresh index (ot == NULL) which both
        // records the current seed and advances the global one.
        let mut cseeds = Vec::new();
        let mut rseeds = Vec::new();
        let mut cmaps = Vec::new();
        let mut rmaps = Vec::new();
        for _ in 0..8 {
            let cm = unsafe { (p.c.shmode_func)(elemsize, SH_ARENA) };
            let rm = unsafe { (p.rs.shmode_func)(elemsize, SH_ARENA) };
            let craw = unsafe { (cm as *mut u8).sub(elemsize) } as *mut std::ffi::c_void;
            let rraw = unsafe { (rm as *mut u8).sub(elemsize) } as *mut std::ffi::c_void;
            cseeds.push(unsafe { (*(header(craw).hash_table as *mut HashIndex)).seed });
            rseeds.push(unsafe { (*(header(rraw).hash_table as *mut HashIndex)).seed });
            cmaps.push(cm);
            rmaps.push(rm);
        }
        assert_eq!(cseeds, rseeds, "seed chain mismatch for rand_seed({seed:#x})");
        for i in 0..cmaps.len() {
            unsafe {
                (p.c.hmfree_func)((cmaps[i] as *mut u8).sub(elemsize) as *mut std::ffi::c_void, elemsize);
                (p.rs.hmfree_func)((rmaps[i] as *mut u8).sub(elemsize) as *mut std::ffi::c_void, elemsize);
            }
        }
    }
}
