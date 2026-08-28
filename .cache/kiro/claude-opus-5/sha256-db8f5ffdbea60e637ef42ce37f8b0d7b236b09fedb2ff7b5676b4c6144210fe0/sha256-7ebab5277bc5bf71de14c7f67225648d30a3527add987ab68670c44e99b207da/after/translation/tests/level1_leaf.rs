//! Level 1: leaf functions with no dependencies on the rest of the library.
//!
//! `stbds_hash_string`, `stbds_hash_bytes`, `stbds_rand_seed`,
//! `stbds_arrgrowf`, `stbds_arrfreef`, `strkey`.

mod harness;

use harness::*;
use std::ffi::{c_char, c_void};

/// Small deterministic PRNG so both sides see identical inputs.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn byte(&mut self) -> u8 {
        (self.next() >> 24) as u8
    }
}

// ---------------------------------------------------------------------------
// stbds_hash_string
// ---------------------------------------------------------------------------

#[test]
fn hash_string_matches() {
    let p = pair();

    let mut inputs: Vec<Vec<u8>> = vec![
        cstring(""),
        cstring("a"),
        cstring("ab"),
        cstring("abc"),
        cstring("test_0"),
        cstring("test_123456"),
        cstring("The quick brown fox jumps over the lazy dog"),
        cstring("\u{7f}\u{1}\u{2}"),
    ];
    // high-bit bytes: `(unsigned char) *str` in C
    inputs.push(vec![0x80, 0xff, 0x7f, 0x01, 0x00]);
    inputs.push(vec![0xff; 65]);
    // every single byte value
    for b in 1u8..=255 {
        inputs.push(vec![b, 0]);
    }
    // pseudo-random strings of many lengths
    let mut rng = Rng(0xDEADBEEF);
    for len in 0..80usize {
        let mut v: Vec<u8> = (0..len).map(|_| rng.byte().max(1)).collect();
        v.push(0);
        inputs.push(v);
    }

    let seeds: [usize; 10] = [
        0,
        1,
        2,
        0x31415926,
        0xffff_ffff,
        0xffff_ffff_ffff_ffff,
        0x8000_0000_0000_0000,
        0x0123_4567_89ab_cdef,
        usize::MAX - 1,
        0xdead_beef_cafe_babe,
    ];

    for s in &mut inputs {
        for &seed in &seeds {
            let ptr = s.as_mut_ptr() as *mut c_char;
            let (a, b) = unsafe { (p.c.hash_string(ptr, seed), p.rs.hash_string(ptr, seed)) };
            assert_eq!(
                a, b,
                "hash_string({:?}, {:#x}) C={:#x} Rust={:#x}",
                &s[..s.len() - 1],
                seed,
                a,
                b
            );
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hash_bytes
// ---------------------------------------------------------------------------

#[test]
fn hash_bytes_matches() {
    let p = pair();

    let seeds: [usize; 8] = [
        0,
        1,
        0x31415926,
        0xffff_ffff,
        usize::MAX,
        0x8000_0000_0000_0000,
        0x0123_4567_89ab_cdef,
        0xdead_beef_cafe_babe,
    ];

    // Exhaustive lengths across the siphash tail switch (0..=8) plus longer
    // multi-block inputs.
    let mut rng = Rng(0x1234_5678);
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for len in 0..=40usize {
        cases.push((0..len).map(|_| rng.byte()).collect());
        // all high bits set: exercises the `int` sign-extension in the tail
        cases.push(vec![0xffu8; len]);
        cases.push(vec![0x80u8; len]);
        cases.push(vec![0x00u8; len]);
        // only the top byte of each 32-bit half set
        let mut v = vec![0u8; len];
        for (i, b) in v.iter_mut().enumerate() {
            *b = if i % 4 == 3 { 0x80 } else { 0 };
        }
        cases.push(v);
    }
    for len in [64usize, 65, 127, 128, 255, 256, 1000] {
        cases.push((0..len).map(|_| rng.byte()).collect());
    }

    for c in &mut cases {
        for &seed in &seeds {
            let ptr = c.as_mut_ptr() as *mut c_void;
            let len = c.len();
            let (a, b) = unsafe { (p.c.hash_bytes(ptr, len, seed), p.rs.hash_bytes(ptr, len, seed)) };
            assert_eq!(
                a, b,
                "hash_bytes(len={}, seed={:#x}) first bytes {:?} C={:#x} Rust={:#x}",
                len,
                seed,
                &c[..len.min(8)],
                a,
                b
            );
        }
    }
}

/// `stbds_hash_bytes` is also reachable through the maps with the *exact* key
/// sizes stb_ds uses (`sizeof (t)->key`), so pin those down separately.
#[test]
fn hash_bytes_typical_key_sizes() {
    let p = pair();
    let mut rng = Rng(0xABCD_1234);
    for _ in 0..200 {
        let mut buf: Vec<u8> = (0..16).map(|_| rng.byte()).collect();
        for &len in &[1usize, 2, 4, 8, 12, 16] {
            for &seed in &[0usize, 0x31415926, usize::MAX] {
                let ptr = buf.as_mut_ptr() as *mut c_void;
                let (a, b) =
                    unsafe { (p.c.hash_bytes(ptr, len, seed), p.rs.hash_bytes(ptr, len, seed)) };
                assert_eq!(a, b, "hash_bytes len={} seed={:#x}", len, seed);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_rand_seed: observable through the seed stored in a fresh hash index
// and through the seed LCG advance.
// ---------------------------------------------------------------------------

#[test]
fn rand_seed_and_seed_sequence_match() {
    let _g = shared_lock(); // mutates the library-global stbds_hash_seed
    let p = pair();
    let elemsize = 8usize;

    for &seed in &[
        0usize,
        1,
        0x31415926,
        usize::MAX,
        0x8000_0000_0000_0000,
        0xdead_beef,
    ] {
        unsafe {
            p.c.rand_seed(seed);
            p.rs.rand_seed(seed);
        }
        // Each stbds_make_hash_index() with ot == NULL consumes the global seed
        // and advances it; 6 consecutive tables pin the whole LCG down.
        let mut c_seeds = Vec::new();
        let mut rs_seeds = Vec::new();
        let mut c_maps = Vec::new();
        let mut rs_maps = Vec::new();
        for _ in 0..6 {
            unsafe {
                let t = p.c.shmode_func(elemsize, SH_NONE);
                c_seeds.push(snapshot_binary(t, elemsize, &[]).seed);
                c_maps.push(t);
                let t = p.rs.shmode_func(elemsize, SH_NONE);
                rs_seeds.push(snapshot_binary(t, elemsize, &[]).seed);
                rs_maps.push(t);
            }
        }
        assert_eq!(c_seeds, rs_seeds, "seed sequence after rand_seed({:#x})", seed);
        unsafe {
            for t in c_maps {
                p.c.hmfree_func((t as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
            for t in rs_maps {
                p.rs.hmfree_func((t as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_arrgrowf / stbds_arrfreef
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct ArrState {
    length: usize,
    capacity: usize,
    hash_table_null: bool,
    temp: isize,
    payload: Vec<u8>,
}

unsafe fn arr_state(a: *mut c_void, elemsize: usize, nbytes: usize) -> ArrState {
    let h = header(a);
    let mut payload = Vec::new();
    let readable = nbytes.min(h.capacity * elemsize);
    for i in 0..readable {
        payload.push(*(a as *const u8).add(i));
    }
    ArrState {
        length: h.length,
        capacity: h.capacity,
        hash_table_null: h.hash_table.is_null(),
        temp: h.temp,
        payload,
    }
}

#[test]
fn arrgrowf_fresh_allocation_matches() {
    let p = pair();
    for &elemsize in &[1usize, 2, 4, 8, 12, 16, 20, 64] {
        for &addlen in &[0usize, 1, 2, 3, 4, 5, 7, 8, 100, 1000] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 8, 9, 64, 1000] {
                unsafe {
                    let ca = p.c.arrgrowf(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ra = p.rs.arrgrowf(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    // addlen == 0 && min_cap == 0 leaves min_cap <= arrcap(NULL)
                    // == 0, so the C code returns the incoming NULL untouched.
                    assert_eq!(
                        ca.is_null(),
                        ra.is_null(),
                        "arrgrowf(NULL, {}, {}, {}) nullness differs (C null={}, Rust null={})",
                        elemsize,
                        addlen,
                        min_cap,
                        ca.is_null(),
                        ra.is_null()
                    );
                    if ca.is_null() {
                        assert!(addlen == 0 && min_cap == 0);
                        continue;
                    }
                    let cs = arr_state(ca, elemsize, 0);
                    let rs = arr_state(ra, elemsize, 0);
                    assert_eq!(
                        cs, rs,
                        "arrgrowf(NULL, {}, {}, {})",
                        elemsize, addlen, min_cap
                    );
                    p.c.arrfreef(ca);
                    p.rs.arrfreef(ra);
                }
            }
        }
    }
}

#[test]
fn arrgrowf_regrow_matches() {
    let p = pair();
    let mut rng = Rng(0x5EED);

    for &elemsize in &[1usize, 4, 8, 16, 20] {
        // Replay the same sequence of (set_length, grow) operations on both.
        let mut ca = unsafe { p.c.arrgrowf(std::ptr::null_mut(), elemsize, 0, 1) };
        let mut ra = unsafe { p.rs.arrgrowf(std::ptr::null_mut(), elemsize, 0, 1) };

        for step in 0..60 {
            let len = unsafe { header(ca).length };
            assert_eq!(len, unsafe { header(ra).length });

            // fill the live prefix with deterministic bytes
            let nbytes = len * elemsize;
            let fill: Vec<u8> = (0..nbytes).map(|_| rng.byte()).collect();
            unsafe {
                std::ptr::copy_nonoverlapping(fill.as_ptr(), ca as *mut u8, nbytes);
                std::ptr::copy_nonoverlapping(fill.as_ptr(), ra as *mut u8, nbytes);
            }

            let addlen = (rng.next() % 6) as usize;
            let min_cap = (rng.next() % 40) as usize;
            unsafe {
                ca = p.c.arrgrowf(ca, elemsize, addlen, min_cap);
                ra = p.rs.arrgrowf(ra, elemsize, addlen, min_cap);
                let cs = arr_state(ca, elemsize, nbytes);
                let rs = arr_state(ra, elemsize, nbytes);
                assert_eq!(
                    cs, rs,
                    "elemsize={} step={} grow(addlen={}, min_cap={})",
                    elemsize, step, addlen, min_cap
                );
                // emulate stbds_arraddn: length += addlen
                let nl = header(ca).length + addlen;
                set_length(ca, nl);
                set_length(ra, nl);
            }
        }
        unsafe {
            p.c.arrfreef(ca);
            p.rs.arrfreef(ra);
        }
    }
}

#[test]
fn arrgrowf_noop_when_capacity_suffices() {
    let p = pair();
    let elemsize = 8usize;
    unsafe {
        let ca = p.c.arrgrowf(std::ptr::null_mut(), elemsize, 0, 10);
        let ra = p.rs.arrgrowf(std::ptr::null_mut(), elemsize, 0, 10);
        let cap = header(ca).capacity;
        assert_eq!(cap, header(ra).capacity);
        // min_cap <= cap => the same pointer must come back
        let ca2 = p.c.arrgrowf(ca, elemsize, 0, cap);
        let ra2 = p.rs.arrgrowf(ra, elemsize, 0, cap);
        assert_eq!(ca2, ca, "C returned a new pointer");
        assert_eq!(ra2, ra, "Rust returned a new pointer");
        assert_eq!(arr_state(ca2, elemsize, 0), arr_state(ra2, elemsize, 0));
        p.c.arrfreef(ca2);
        p.rs.arrfreef(ra2);
    }
}

// ---------------------------------------------------------------------------
// strkey
// ---------------------------------------------------------------------------

#[test]
fn strkey_matches() {
    let _g = shared_lock(); // strkey writes a library-global static buffer
    let p = pair();
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
        i32::MIN + 1,
    ];
    let mut rng = Rng(0x9999);
    for _ in 0..200 {
        cases.push(rng.next() as i32);
    }
    for n in cases {
        unsafe {
            let cs = read_cstr(p.c.strkey(n));
            let rs = read_cstr(p.rs.strkey(n));
            assert_eq!(
                String::from_utf8_lossy(&cs),
                String::from_utf8_lossy(&rs),
                "strkey({})",
                n
            );
        }
    }
}

/// `strkey` returns a pointer into a single static buffer; repeated calls must
/// keep returning the same address and overwrite in place.
#[test]
fn strkey_uses_stable_static_buffer() {
    let _g = shared_lock(); // strkey writes a library-global static buffer
    let p = pair();
    unsafe {
        let c1 = p.c.strkey(1);
        let c2 = p.c.strkey(22);
        assert_eq!(c1, c2, "C strkey buffer moved");
        let r1 = p.rs.strkey(1);
        let r2 = p.rs.strkey(22);
        assert_eq!(r1, r2, "Rust strkey buffer moved");
        assert_eq!(read_cstr(c2), read_cstr(r2));
        // long value then short value: the NUL must be re-written
        p.c.strkey(i32::MIN);
        p.rs.strkey(i32::MIN);
        assert_eq!(read_cstr(p.c.strkey(7)), read_cstr(p.rs.strkey(7)));
    }
}
