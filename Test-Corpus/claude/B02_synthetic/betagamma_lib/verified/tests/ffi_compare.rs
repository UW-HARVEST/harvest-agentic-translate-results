// Integration tests that compare C and Rust shared libraries via libloading.
//
// To avoid any heap-state cross-contamination between the two libraries (the
// C function `compute_hash` performs pointer-order comparisons that depend on
// the heap allocator's history), each library is exercised in a fresh forked
// child process. The child writes its result to a pipe, and the parent reads
// and compares.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy)]
struct DataBlock {
    id: c_int,
    name: [u8; 32],
    flags: u8,
}

#[repr(C)]
struct MemoryBlock {
    data: *mut c_int,
    size: usize,
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    let release = p.join("release").join("libbetagamma_lib.so");
    let debug = p.join("debug").join("libbetagamma_lib.so");
    if release.exists() {
        release
    } else {
        debug
    }
}

/// Compare bytes up to and including the first null byte (string semantics).
fn strncmp_up_to_null(a: &[u8], b: &[u8]) -> bool {
    let len = a.len().min(b.len());
    for i in 0..len {
        if a[i] != b[i] {
            return false;
        }
        if a[i] == 0 {
            return true;
        }
    }
    true
}

fn runner_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    let release = p.join("release").join("betagamma_runner");
    let debug = p.join("debug").join("betagamma_runner");
    if release.exists() {
        release
    } else {
        debug
    }
}

/// Spawn the betagamma_runner binary in a fresh process to call betagamma()
/// against the given .so. This avoids any heap-state cross-contamination
/// between the C and Rust libraries (which would otherwise affect the
/// pointer-based hash in compute_hash).
fn run_betagamma(so: &PathBuf, a: c_int, b: c_int, cc: c_int, d: c_int) -> c_int {
    let out = Command::new(runner_path())
        .arg(so)
        .arg(a.to_string())
        .arg(b.to_string())
        .arg(cc.to_string())
        .arg(d.to_string())
        .output()
        .expect("spawn runner");
    assert!(
        out.status.success(),
        "runner failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim().parse::<c_int>().expect("parse runner output")
}

#[test]
fn test_betagamma_matches() {
    let c = c_so_path();
    let r = rust_so_path();

    let inputs: &[(c_int, c_int, c_int, c_int)] = &[
        (1, 2, 3, 4),
        (0, 0, 0, 0),
        (10, 20, 30, 40),
        (-1, -2, -3, -4),
        (100, 200, 300, 400),
        (5, 7, 11, 13),
        (1000, -1000, 999, -999),
        (123456, 654321, 1, 2),
        (7, 0, 0, 0),
        (9, 8, 7, 6),
    ];

    for &(a, b, cc, d) in inputs {
        let cv = run_betagamma(&c, a, b, cc, d);
        let rv = run_betagamma(&r, a, b, cc, d);
        assert_eq!(
            cv, rv,
            "betagamma({}, {}, {}, {}) C={} R={}",
            a, b, cc, d, cv, rv
        );
    }
}

#[test]
fn test_create_block_matches() {
    unsafe {
        let c = Library::new(c_so_path()).unwrap();
        let r = Library::new(rust_so_path()).unwrap();
        let cfn: Symbol<unsafe extern "C" fn(c_int, *const c_char, u8) -> DataBlock> =
            c.get(b"create_block").unwrap();
        let rfn: Symbol<unsafe extern "C" fn(c_int, *const c_char, u8) -> DataBlock> =
            r.get(b"create_block").unwrap();

        let cases: &[(c_int, &[u8], u8)] = &[
            (1, b"Hello\0", 0xAA),
            (0, b"\0", 0x00),
            (42, b"ABC\0", 0xFF),
            (-7, b"Test_String_Foo\0", 0x55),
            (1000, b"Block_Alpha\0", 0b10101010),
        ];

        for (id, name, flags) in cases {
            let cv = cfn(*id, name.as_ptr() as *const c_char, *flags);
            let rv = rfn(*id, name.as_ptr() as *const c_char, *flags);
            assert_eq!(cv.id, rv.id, "id mismatch for {:?}", name);
            assert_eq!(cv.flags, rv.flags, "flags mismatch for {:?}", name);
            // Compare only bytes up to and including the null terminator.
            // Bytes after the null are uninitialized stack memory in C.
            assert!(
                strncmp_up_to_null(&cv.name[..], &rv.name[..]),
                "name C-string mismatch for {:?} C={:?} R={:?}",
                name,
                &cv.name[..],
                &rv.name[..]
            );
        }
    }
}

#[test]
fn test_allocate_and_free_block() {
    unsafe {
        let c = Library::new(c_so_path()).unwrap();
        let r = Library::new(rust_so_path()).unwrap();
        let c_alloc: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock> =
            c.get(b"allocate_block").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut MemoryBlock)> = c.get(b"free_block").unwrap();
        let r_alloc: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock> =
            r.get(b"allocate_block").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut MemoryBlock)> = r.get(b"free_block").unwrap();

        let cases: &[(usize, c_int)] = &[
            (5, 0),
            (10, 100),
            (1, -1),
            (8, -50),
            (16, 1234),
            (3, i32::MAX - 1),
            (4, i32::MIN + 1),
        ];

        for &(count, init) in cases {
            let cb = c_alloc(count, init);
            let rb = r_alloc(count, init);
            assert!(!cb.is_null(), "C allocate_block null");
            assert!(!rb.is_null(), "Rust allocate_block null");

            assert_eq!((*cb).size, count);
            assert_eq!((*rb).size, count);

            for i in 0..count {
                let cv = *((*cb).data.add(i));
                let rv = *((*rb).data.add(i));
                assert_eq!(cv, rv, "data[{}] differs for count={} init={}", i, count, init);
            }

            c_free(cb);
            r_free(rb);
        }
    }
}

#[test]
fn test_compute_hash_self() {
    unsafe {
        let c = Library::new(c_so_path()).unwrap();
        let r = Library::new(rust_so_path()).unwrap();
        let c_alloc: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock> =
            c.get(b"allocate_block").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut MemoryBlock)> = c.get(b"free_block").unwrap();
        let c_hash: Symbol<unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int> =
            c.get(b"compute_hash").unwrap();
        let r_alloc: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock> =
            r.get(b"allocate_block").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut MemoryBlock)> = r.get(b"free_block").unwrap();
        let r_hash: Symbol<unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int> =
            r.get(b"compute_hash").unwrap();

        let cb = c_alloc(5, 10);
        let rb = r_alloc(5, 10);

        let ch = c_hash(cb, cb);
        let rh = r_hash(rb, rb);
        assert_eq!(ch, 0);
        assert_eq!(rh, 0);

        c_free(cb);
        r_free(rb);
    }
}

#[test]
fn test_compute_hash_two_blocks() {
    unsafe {
        let c = Library::new(c_so_path()).unwrap();
        let r = Library::new(rust_so_path()).unwrap();
        let c_alloc: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock> =
            c.get(b"allocate_block").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut MemoryBlock)> = c.get(b"free_block").unwrap();
        let c_hash: Symbol<unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int> =
            c.get(b"compute_hash").unwrap();
        let r_alloc: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut MemoryBlock> =
            r.get(b"allocate_block").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut MemoryBlock)> = r.get(b"free_block").unwrap();
        let r_hash: Symbol<unsafe extern "C" fn(*mut MemoryBlock, *mut MemoryBlock) -> c_int> =
            r.get(b"compute_hash").unwrap();

        let c1 = c_alloc(5, 10);
        let c2 = c_alloc(5, 20);
        let r1 = r_alloc(5, 10);
        let r2 = r_alloc(5, 20);

        let ch12 = c_hash(c1, c2);
        let ch21 = c_hash(c2, c1);
        let rh12 = r_hash(r1, r2);
        let rh21 = r_hash(r2, r1);

        // Property: hash(a,b) + hash(b,a) == 330 when a != b.
        assert_eq!(ch12 + ch21, 330);
        assert_eq!(rh12 + rh21, 330);

        c_free(c1);
        c_free(c2);
        r_free(r1);
        r_free(r2);
    }
}

#[test]
fn test_exported_symbols_match() {
    // Ensure every "T" symbol exported by the C .so is also exported by the
    // Rust .so. We invoke `nm -D` and parse its output.
    use std::process::Command;
    let c_out = Command::new("nm")
        .args(["-D", c_so_path().to_str().unwrap()])
        .output()
        .expect("nm c");
    let r_out = Command::new("nm")
        .args(["-D", rust_so_path().to_str().unwrap()])
        .output()
        .expect("nm r");

    fn t_syms(out: &[u8]) -> Vec<String> {
        let s = String::from_utf8_lossy(out);
        s.lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 3 && parts[1] == "T" {
                    Some(parts[2].to_string())
                } else {
                    None
                }
            })
            .collect()
    }
    let c_syms = t_syms(&c_out.stdout);
    let r_syms = t_syms(&r_out.stdout);
    // Every C T-symbol (excluding _init / _fini, which are autogenerated)
    // must appear in Rust.
    for s in &c_syms {
        if s == "_init" || s == "_fini" {
            continue;
        }
        assert!(
            r_syms.iter().any(|x| x == s),
            "Rust .so missing exported symbol: {}",
            s
        );
    }
}
