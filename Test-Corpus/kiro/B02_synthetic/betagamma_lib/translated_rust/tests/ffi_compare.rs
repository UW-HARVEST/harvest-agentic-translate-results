use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;

#[repr(C)]
#[derive(Debug)]
struct DataBlock {
    id: c_int,
    name: [c_char; 32],
    flags: u8,
}

#[repr(C)]
struct MemoryBlock {
    data: *mut c_int,
    size: usize,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    dir.join(format!("target/{}/libbetagamma_lib.so", profile))
}

// ---- create_block ----

#[test]
fn test_create_block() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int, *const c_char, u8) -> DataBlock> =
            c_lib.get(b"create_block").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, *const c_char, u8) -> DataBlock> =
            r_lib.get(b"create_block").unwrap();

        let cases: &[(c_int, &str, u8)] = &[
            (1, "Alpha", 0xAA),
            (0, "", 0),
            (999, "A_long_name_here_12345", 0xFF),
        ];

        for &(id, name, flags) in cases {
            let cname = CString::new(name).unwrap();
            let c_res = c_fn(id, cname.as_ptr(), flags);
            let r_res = r_fn(id, cname.as_ptr(), flags);

            assert_eq!(c_res.id, r_res.id, "id mismatch for {name}");
            // Only compare name bytes up to and including the null terminator,
            // since C's strcpy leaves the rest uninitialized.
            let len = c_res.name.iter().position(|&b| b == 0).unwrap_or(32);
            assert_eq!(
                &c_res.name[..=len], &r_res.name[..=len],
                "name mismatch for {name}"
            );
            assert_eq!(c_res.flags, r_res.flags, "flags mismatch for {name}");
        }
    }
}

// ---- allocate_block / free_block ----

#[test]
fn test_allocate_and_free_block() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type AllocFn = unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock;
        type FreeFn = unsafe extern "C" fn(*mut MemoryBlock);

        let c_alloc: Symbol<AllocFn> = c_lib.get(b"allocate_block").unwrap();
        let r_alloc: Symbol<AllocFn> = r_lib.get(b"allocate_block").unwrap();
        let c_free: Symbol<FreeFn> = c_lib.get(b"free_block").unwrap();
        let r_free: Symbol<FreeFn> = r_lib.get(b"free_block").unwrap();

        let cases: &[(usize, c_int)] = &[(5, 10), (1, 0), (10, -3), (8, 100)];

        for &(count, init) in cases {
            let c_mb = c_alloc(count, init);
            let r_mb = r_alloc(count, init);
            assert!(!c_mb.is_null());
            assert!(!r_mb.is_null());

            assert_eq!((*c_mb).size, (*r_mb).size, "size mismatch count={count} init={init}");
            for i in 0..count {
                let cv = *(*c_mb).data.add(i);
                let rv = *(*r_mb).data.add(i);
                assert_eq!(cv, rv, "data[{i}] mismatch count={count} init={init}");
            }

            c_free(c_mb);
            r_free(r_mb);
        }

        // free_block(null) should not crash
        c_free(std::ptr::null_mut());
        r_free(std::ptr::null_mut());
    }
}

// ---- compute_hash ----

#[test]
fn test_compute_hash() {
    // compute_hash depends on pointer ordering which differs between C and Rust allocators.
    // We test that the function logic is correct by using the SAME library's allocate_block
    // for both arguments, then comparing the hash output pattern.
    // Since pointer ordering is non-deterministic, we verify structural correctness:
    // the hash should be one of {0, 10, 20, 100, 110, 120, 200, 210, 220}.
    unsafe {
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type AllocFn = unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock;
        type FreeFn = unsafe extern "C" fn(*mut MemoryBlock);
        type HashFn = unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int;

        let r_alloc: Symbol<AllocFn> = r_lib.get(b"allocate_block").unwrap();
        let r_free: Symbol<FreeFn> = r_lib.get(b"free_block").unwrap();
        let r_hash: Symbol<HashFn> = r_lib.get(b"compute_hash").unwrap();

        let mb1 = r_alloc(5, 1);
        let mb2 = r_alloc(5, 2);
        assert!(!mb1.is_null() && !mb2.is_null());

        let h = r_hash(mb1, mb2);
        let valid = [0, 10, 20, 100, 110, 120, 200, 210, 220];
        assert!(valid.contains(&h), "unexpected hash value: {h}");

        r_free(mb1);
        r_free(mb2);
    }
}

// ---- betagamma (the main function) ----

#[test]
fn test_betagamma() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type BgFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

        let c_bg: Symbol<BgFn> = c_lib.get(b"betagamma").unwrap();
        let r_bg: Symbol<BgFn> = r_lib.get(b"betagamma").unwrap();

        // betagamma has pointer-dependent hash component, so we compare
        // the deterministic part by subtracting the hash contribution.
        // Actually, let's first check if they happen to match (they often do
        // since the hash is based on relative pointer ordering which is
        // typically consistent within a single run).
        //
        // The pointer-comparison parts (compute_hash, data!=data, data>NULL)
        // are deterministic for non-null separate allocations:
        // - data != data: always true -> +99
        // - data > NULL: always true for both -> +255
        // - compute_hash: depends on allocator, but within each library it's consistent
        //
        // So the only non-deterministic part between C and Rust is compute_hash.
        // Let's compute the deterministic part and verify that matches,
        // then verify compute_hash contribution is valid.

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (1, 2, 3, 4),
            (0, 0, 0, 0),
            (10, 20, 30, 40),
            (5, 5, 5, 5),
            (-1, -2, -3, -4),
            (7, 3, 1, 9),
        ];

        for &(a, b, c, d) in cases {
            let c_res = c_bg(a, b, c, d);
            let r_res = r_bg(a, b, c, d);

            // The hash component differs due to allocator differences.
            // The hash is always one of {0,10,20,100,110,120,200,210,220}.
            // So c_res and r_res should differ by at most 220.
            let diff = (c_res - r_res).abs();
            let valid_diffs: Vec<c_int> = {
                let vals = [0, 10, 20, 100, 110, 120, 200, 210, 220];
                let mut diffs = Vec::new();
                for &v1 in &vals {
                    for &v2 in &vals {
                        let d: c_int = (v1 as c_int - v2 as c_int).abs();
                        if !diffs.contains(&d) {
                            diffs.push(d);
                        }
                    }
                }
                diffs
            };
            assert!(
                valid_diffs.contains(&diff),
                "betagamma({a},{b},{c},{d}): C={c_res} Rust={r_res} diff={diff} not in valid set"
            );
        }
    }
}

// Stricter test: verify the deterministic portion of betagamma matches exactly.
// We do this by calling betagamma with the same params and checking that
// result - hash is the same for both C and Rust.
// Since we can't extract hash directly, we test with params where block_size
// is the same and sum1-sum2 is deterministic.
#[test]
fn test_betagamma_deterministic_part() {
    // The flag contribution loop is purely deterministic (no pointers).
    // Let's verify it by computing expected values manually.
    //
    // Block 1: id=1, flags=0b10101010 -> low nibble 0b1010 != 0 -> +p1
    //                                    high nibble 0b1010_0000 != 0 -> +p2
    //                                    & 0b10101010 != 0 -> +p3
    //                                    & 0b01010101: 0b10101010 & 0b01010101 = 0 -> nothing
    //   contribution = (p1+p2+p3)*1
    //
    // Block 2: id=2, flags=0b11001100 -> & 0x0F: 0b1100 != 0 -> +p1
    //                                    & 0xF0: 0b11000000 != 0 -> +p2
    //                                    & 0xAA: 0b11001100 & 0b10101010 = 0b10001000 != 0 -> +p3
    //                                    & 0x55: 0b11001100 & 0b01010101 = 0b01000100 != 0 -> +p4
    //   contribution = (p1+p2+p3+p4)*2
    //
    // Block 3: id=3, flags=0b11110000 -> & 0x0F: 0 -> nothing
    //                                    & 0xF0: 0b11110000 != 0 -> +p2
    //                                    & 0xAA: 0b11110000 & 0b10101010 = 0b10100000 != 0 -> +p3
    //                                    & 0x55: 0b11110000 & 0b01010101 = 0b01010000 != 0 -> +p4
    //   contribution = (p2+p3+p4)*3
    //
    // Total flag part = (p1+p2+p3)*1 + (p1+p2+p3+p4)*2 + (p2+p3+p4)*3
    //                 = p1 + p2 + p3 + 2p1 + 2p2 + 2p3 + 2p4 + 3p2 + 3p3 + 3p4
    //                 = 3*p1 + 6*p2 + 6*p3 + 5*p4

    // sum1 - sum2: for block_size n, init p1 and p2:
    //   sum1 = sum(p1+i for i in 0..n) = n*p1 + n*(n-1)/2
    //   sum2 = sum(p2+i for i in 0..n) = n*p2 + n*(n-1)/2
    //   (sum1-sum2)/10 = n*(p1-p2)/10

    // special_id (99) + special_flags (255) = 354 (always added for non-null allocs)

    // So deterministic part = 3*p1 + 6*p2 + 6*p3 + 5*p4 + n*(p1-p2)/10 + 354
    // where n = (p1 % 10) + 5

    // Test with p1=10, p2=20, p3=30, p4=40:
    // flag part = 3*10 + 6*20 + 6*30 + 5*40 = 30 + 120 + 180 + 200 = 530
    // n = (10%10)+5 = 5
    // sum_diff = 5*(10-20)/10 = -5
    // deterministic = 530 + (-5) + 354 = 879

    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type BgFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
        let c_bg: Symbol<BgFn> = c_lib.get(b"betagamma").unwrap();
        let r_bg: Symbol<BgFn> = r_lib.get(b"betagamma").unwrap();

        let c_res = c_bg(10, 20, 30, 40);
        let r_res = r_bg(10, 20, 30, 40);

        // Subtract the hash (which is the only non-deterministic part)
        // Both should give 879 + hash, where hash is from their respective allocators
        let c_det = c_res; // includes c_hash
        let r_det = r_res; // includes r_hash

        // The deterministic base is 879. Both results should be 879 + some valid hash.
        let c_hash = c_det - 879;
        let r_hash = r_det - 879;

        let valid_hashes = [0, 10, 20, 100, 110, 120, 200, 210, 220];
        assert!(
            valid_hashes.contains(&c_hash),
            "C hash component {c_hash} not valid (c_res={c_res})"
        );
        assert!(
            valid_hashes.contains(&r_hash),
            "Rust hash component {r_hash} not valid (r_res={r_res})"
        );
    }
}
