//! Level 5: the public API declared in `include/lib.h` (`arr_del`), the array
//! macro sequence it drives, and exported-symbol parity between the two shared
//! objects.

mod common;

use common::*;
use std::ffi::c_void;
use std::process::Command;

/// `arr_del` returns nothing and frees everything it allocates, so the only
/// direct observation is that both libraries survive the same inputs.
#[test]
fn arr_del_runs_identically() {
    let (c, r) = both();
    let inputs: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        3,
        4,
        7,
        42,
        -42,
        1000,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
    ];
    for n in inputs {
        unsafe {
            (c.arr_del)(n);
            (r.arr_del)(n);
        }
    }
    // repeated invocations must not accumulate state
    for _ in 0..50 {
        unsafe {
            (c.arr_del)(5);
            (r.arr_del)(5);
        }
    }
}

/// Re-implementation of the exact macro sequence inside `arr_del`, driven
/// through each library's exported `stbds_arrgrowf`, so the resulting buffer
/// contents and headers can be compared step by step.
///
/// ```c
/// for (i=0; i < 4; ++i) {
///   arrpush(arr,num); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
///   arrdel(arr,i);      arrfree(arr);
///   arrpush(arr,num); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
///   arrdelswap(arr,i);  arrfree(arr);
/// }
/// ```
#[test]
fn arr_del_macro_sequence_matches() {
    let (c, r) = both();
    const ES: usize = 4; // sizeof(int)

    unsafe fn maybegrow(lib: &Lib, a: *mut u8, n: usize) -> *mut u8 {
        if a.is_null() || (*header(a)).length + n > (*header(a)).capacity {
            (lib.arrgrowf)(a as *mut c_void, ES, n, 0) as *mut u8
        } else {
            a
        }
    }
    unsafe fn push(lib: &Lib, a: *mut u8, v: i32) -> *mut u8 {
        let a = maybegrow(lib, a, 1);
        let len = (*header(a)).length;
        std::ptr::copy_nonoverlapping(v.to_ne_bytes().as_ptr(), a.add(len * ES), ES);
        (*header(a)).length = len + 1;
        a
    }
    unsafe fn snap(a: *mut u8) -> (bool, usize, usize, isize, Vec<u8>) {
        if a.is_null() {
            return (true, 0, 0, 0, Vec::new());
        }
        let h = *header(a);
        (
            false,
            h.length,
            h.capacity,
            h.temp,
            std::slice::from_raw_parts(a, h.length * ES).to_vec(),
        )
    }

    for num in [0i32, 1, -1, 4, 42, i32::MAX, i32::MIN] {
        unsafe {
            let mut ca: *mut u8 = std::ptr::null_mut();
            let mut ra: *mut u8 = std::ptr::null_mut();

            for i in 0..4i32 {
                for v in [num, 2, 3, 4] {
                    ca = push(&c, ca, v);
                    ra = push(&r, ra, v);
                    assert_eq!(snap(ca), snap(ra), "arrpush mismatch num={num} i={i} v={v}");
                }

                // arrdel(arr, i) == arrdeln(arr, i, 1)
                for a in [ca, ra] {
                    let h = header(a);
                    let n = 1usize;
                    let count = (*h).length - n - (i as usize);
                    std::ptr::copy(
                        a.add((i as usize + n) * ES),
                        a.add(i as usize * ES),
                        count * ES,
                    );
                    (*h).length -= n;
                }
                assert_eq!(snap(ca), snap(ra), "arrdel mismatch num={num} i={i}");

                // arrfree(arr)
                (c.arrfreef)(ca as *mut c_void);
                (r.arrfreef)(ra as *mut c_void);
                ca = std::ptr::null_mut();
                ra = std::ptr::null_mut();

                for v in [num, 2, 3, 4] {
                    ca = push(&c, ca, v);
                    ra = push(&r, ra, v);
                    assert_eq!(snap(ca), snap(ra), "arrpush(2) mismatch num={num} i={i} v={v}");
                }

                // arrdelswap(arr, i)
                for a in [ca, ra] {
                    let h = header(a);
                    let last = a.add(((*h).length - 1) * ES);
                    std::ptr::copy(last, a.add(i as usize * ES), ES);
                    (*h).length -= 1;
                }
                assert_eq!(snap(ca), snap(ra), "arrdelswap mismatch num={num} i={i}");

                (c.arrfreef)(ca as *mut c_void);
                (r.arrfreef)(ra as *mut c_void);
                ca = std::ptr::null_mut();
                ra = std::ptr::null_mut();
            }
        }
    }
}

fn dynamic_symbols(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            match it.next() {
                // "<addr> <type> <name>"
                Some(name) => Some((b.to_string(), name.to_string())),
                // "         <type> <name>" (undefined / absolute)
                None => Some((a.to_string(), b.to_string())),
            }
        })
        .filter(|(kind, _)| kind != "U" && kind != "w" && kind != "v")
        .map(|(_, name)| name)
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// Every symbol the C shared object exports must also be exported by the Rust
/// shared object, under exactly the same name.
#[test]
fn exported_symbols_cover_the_c_library() {
    let c_path = c_so_path();
    let r_path = rust_so_path();
    let c_syms = dynamic_symbols(&c_path);
    let r_syms = dynamic_symbols(&r_path);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C   ({}): {c_syms:?}\n\
         Rust({}): {r_syms:?}",
        c_path.display(),
        r_path.display()
    );

    // sanity: the C library really did export the whole stb_ds surface
    for expected in [
        "arr_del",
        "strkey",
        "stbds_arrgrowf",
        "stbds_arrfreef",
        "stbds_rand_seed",
        "stbds_hash_bytes",
        "stbds_hash_string",
        "stbds_hmfree_func",
        "stbds_hmget_key",
        "stbds_hmget_key_ts",
        "stbds_hmput_default",
        "stbds_hmput_key",
        "stbds_hmdel_key",
        "stbds_shmode_func",
        "stbds_stralloc",
        "stbds_strreset",
    ] {
        assert!(
            c_syms.iter().any(|s| s == expected),
            "test assumption broken: C .so does not export {expected}"
        );
        assert!(
            r_syms.iter().any(|s| s == expected),
            "Rust .so does not export {expected}"
        );
    }
}
