//! Phase B, Group 11 of `CONFIGS.md`: the two top-level helpers `strkey` and
//! `arr_push` from the bottom of `c_src/src/lib.c`.

mod common;

use common::*;

/// The whole 256-byte static buffer, so trailing bytes left over from a
/// previous `sprintf` are compared too.
unsafe fn full_buffer(p: *const core::ffi::c_char) -> Vec<u8> {
    core::slice::from_raw_parts(p as *const u8, 256).to_vec()
}

// ---------------------------------------------------------------------------
// C79 — strkey over the whole interesting int domain
// ---------------------------------------------------------------------------
#[test]
fn cfg_c79_strkey() {
    let p = libs();
    let mut vals: Vec<i32> = vec![
        0,
        1,
        -1,
        7,
        9,
        10,
        11,
        99,
        100,
        101,
        999,
        1000,
        12345,
        -99,
        -100,
        -12345,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        -2147483647,
        1_000_000_000,
        -1_000_000_000,
    ];
    let mut rng = Rng::new(79);
    for _ in 0..200 {
        vals.push(rng.next_u32() as i32);
    }
    for _ in 0..100 {
        vals.push((rng.next_u32() % 1000) as i32);
    }

    for (i, &n) in vals.iter().enumerate() {
        unsafe {
            let cp = (p.c.strkey)(n);
            let rp = (p.r.strkey)(n);
            // the returned pointer must be the same static buffer every time
            let cb = full_buffer(cp);
            let rb = full_buffer(rp);
            diff_eq!(cb, rb, "strkey({n}) #{i} full 256-byte buffer");
            let text = read_cstr(cp);
            let expect = format!("test_{n}");
            assert_eq!(
                text,
                expect.as_bytes(),
                "strkey({n}) should render as `{expect}`"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C79b — the static buffer aliases across calls (E59)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c79b_strkey_aliases() {
    let p = libs();
    unsafe {
        let c1 = (p.c.strkey)(12345);
        let c2 = (p.c.strkey)(1);
        assert_eq!(c1, c2, "C strkey must reuse one static buffer");
        let r1 = (p.r.strkey)(12345);
        let r2 = (p.r.strkey)(1);
        assert_eq!(r1, r2, "Rust strkey must reuse one static buffer");
        // and the leftover tail of the longer previous render must match
        diff_eq!(full_buffer(c2), full_buffer(r2), "strkey leftover tail");
        assert_eq!(read_cstr(c2), b"test_1");
    }
}

// ---------------------------------------------------------------------------
// C80 / C81 — arr_push
// ---------------------------------------------------------------------------
#[test]
fn cfg_c80_c81_arr_push() {
    let p = libs();
    for num in [
        0i32, 1, 2, 3, 49, 50, 51, 52, 99, 100, 101, 149, 150, 151, 500, 1000, 5000,
    ] {
        unsafe {
            (p.c.arr_push)(num);
            (p.r.arr_push)(num);
        }
    }
    // negative / boundary values: the outer `for (i=0; i < num; i += 50)` never runs
    for num in [-1i32, -50, -51, -1000, i32::MIN, i32::MIN + 1] {
        unsafe {
            (p.c.arr_push)(num);
            (p.r.arr_push)(num);
        }
    }
    // repeat to be sure nothing is left behind between calls
    for _ in 0..20 {
        unsafe {
            (p.c.arr_push)(200);
            (p.r.arr_push)(200);
        }
    }
}

// ---------------------------------------------------------------------------
// Symbol parity is re-verified from inside the test process: every symbol in
// `EXPORTS` must resolve in BOTH libraries (Phase D, cross-checked in shell too).
// ---------------------------------------------------------------------------
#[test]
fn symbol_parity_all_exports_resolve() {
    let p = libs();
    // `Lib::load` already panics on a missing symbol, so simply constructing the
    // pair proves all 16 resolve in both .so files.  Assert the list length and
    // that the two libraries are genuinely different files.
    assert_eq!(EXPORTS.len(), 16);
    assert_ne!(p.c.path, p.r.path);
    println!("C   .so: {}", p.c.path.display());
    println!("RUST.so: {}", p.r.path.display());
}

// ---------------------------------------------------------------------------
// Layout parity, checked against the *C* library's actual behaviour: writing a
// header through our mirror struct must be seen identically by both libs.
// ---------------------------------------------------------------------------
#[test]
fn layout_parity() {
    let p = libs();
    assert_eq!(core::mem::size_of::<CArrayHeader>(), 32);
    assert_eq!(core::mem::size_of::<CHashIndex>(), 104);
    assert_eq!(core::mem::size_of::<CHashBucket>(), 128);
    assert_eq!(core::mem::size_of::<CStringArena>(), 24);
    assert_eq!(core::mem::size_of::<CStringBlock>(), 16);

    // `stbds_arrgrowf` writes length/capacity/hash_table/temp through the C's own
    // struct definition; if our mirror (and the Rust translation's) disagreed we
    // would read different values here.
    for elemsize in [1usize, 4, 8, 20] {
        let mut ca = Arr::new(&p.c, elemsize);
        let mut ra = Arr::new(&p.r, elemsize);
        ca.grow(0, 37);
        ra.grow(0, 37);
        let (ch, _) = ca.snap();
        let (rh, _) = ra.snap();
        assert_eq!(ch.capacity, 37);
        assert_eq!(ch.length, 0);
        assert_eq!(ch.temp, 0);
        assert!(!ch.has_table);
        diff_eq!(ch, rh, "layout probe e={elemsize}");
        ca.free();
        ra.free();
    }

    // `stbds_shmode_func` writes every `stbds_hash_index` field; read them all.
    reset_seed(&p, 0xFEED_FACE);
    let spec = Spec::bytes(16, 8);
    let mut cm = Map::new_shmode(&p.c, spec, STBDS_HM_STRING, STBDS_SH_ARENA);
    let mut rm = Map::new_shmode(&p.r, spec, STBDS_HM_STRING, STBDS_SH_ARENA);
    let cs = cm.snap();
    let rs = rm.snap();
    assert_eq!(cs.idx.slot_count, 8);
    assert_eq!(cs.idx.slot_count_log2, 3);
    assert_eq!(cs.idx.used_count_threshold, 6);
    assert_eq!(cs.idx.tombstone_count_threshold, 1);
    assert_eq!(cs.idx.used_count_shrink_threshold, 0);
    assert_eq!(cs.idx.seed, 0xFEED_FACE);
    assert_eq!(cs.idx.arena.mode, 3);
    assert!(cs.idx.storage_aligned_64, "storage must be 64-byte aligned");
    assert!(cs.idx.storage_in_alloc, "storage must sit inside the allocation");
    assert_eq!(cs.idx.slots.len(), 8);
    assert!(cs.idx.slots.iter().all(|&(h, i)| h == 0 && i == -1));
    diff_eq!(cs, rs, "hash_index layout probe");
    cm.hmfree();
    rm.hmfree();
    reset_seed(&p, DEFAULT_SEED);
}
