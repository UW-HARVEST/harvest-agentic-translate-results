use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_ulonglong};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpritebatchSprite {
    pub texture_id: c_ulonglong,
    pub sort_bits: c_int,
}

type MergeSortFn = unsafe extern "C" fn(
    a: *mut SpritebatchSprite,
    b: *mut SpritebatchSprite,
    size: c_int,
);

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    // Search for the .so in target/debug or target/release
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in &["debug", "release"] {
        let p = manifest.join("target").join(profile).join("libmerge_sort_lib.so");
        if p.exists() {
            return p;
        }
    }
    // Fallback: cargo test sets OUT_DIR for build script. Use CARGO_TARGET_TMPDIR if set.
    manifest.join("target").join("debug").join("libmerge_sort_lib.so")
}

fn run_c(input: &[SpritebatchSprite]) -> Vec<SpritebatchSprite> {
    unsafe {
        let lib = Library::new(c_so_path()).expect("load C .so");
        let f: Symbol<MergeSortFn> = lib.get(b"merge_sort").expect("find merge_sort");
        let mut a = input.to_vec();
        let mut b = vec![
            SpritebatchSprite {
                texture_id: 0,
                sort_bits: 0
            };
            input.len()
        ];
        if !a.is_empty() {
            f(a.as_mut_ptr(), b.as_mut_ptr(), input.len() as c_int);
        } else {
            f(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        }
        a
    }
}

fn run_rust(input: &[SpritebatchSprite]) -> Vec<SpritebatchSprite> {
    unsafe {
        let lib = Library::new(rust_so_path()).expect("load Rust .so");
        let f: Symbol<MergeSortFn> = lib.get(b"merge_sort").expect("find merge_sort");
        let mut a = input.to_vec();
        let mut b = vec![
            SpritebatchSprite {
                texture_id: 0,
                sort_bits: 0
            };
            input.len()
        ];
        if !a.is_empty() {
            f(a.as_mut_ptr(), b.as_mut_ptr(), input.len() as c_int);
        } else {
            f(std::ptr::null_mut(), std::ptr::null_mut(), 0);
        }
        a
    }
}

fn assert_match(input: &[SpritebatchSprite]) {
    let c_out = run_c(input);
    let r_out = run_rust(input);
    assert_eq!(c_out.len(), r_out.len(), "length mismatch");
    for i in 0..c_out.len() {
        assert_eq!(
            c_out[i], r_out[i],
            "mismatch at index {}: C={:?} Rust={:?}",
            i, c_out[i], r_out[i]
        );
    }
}

fn s(texture_id: u64, sort_bits: i32) -> SpritebatchSprite {
    SpritebatchSprite {
        texture_id,
        sort_bits,
    }
}

#[test]
fn merge_sort_empty() {
    assert_match(&[]);
}

#[test]
fn merge_sort_single() {
    assert_match(&[s(42, 7)]);
}

#[test]
fn merge_sort_two_sorted() {
    assert_match(&[s(1, 1), s(2, 2)]);
}

#[test]
fn merge_sort_two_reversed() {
    assert_match(&[s(2, 2), s(1, 1)]);
}

#[test]
fn merge_sort_already_sorted() {
    let v: Vec<_> = (0..10).map(|i| s(i as u64, i)).collect();
    assert_match(&v);
}

#[test]
fn merge_sort_reverse_sorted() {
    let v: Vec<_> = (0..10).rev().map(|i| s(i as u64, i)).collect();
    assert_match(&v);
}

#[test]
fn merge_sort_all_same_sort_bits() {
    // Same sort_bits, varying texture_ids in random order
    let v = vec![
        s(5, 3),
        s(1, 3),
        s(9, 3),
        s(2, 3),
        s(7, 3),
        s(0, 3),
        s(3, 3),
    ];
    assert_match(&v);
}

#[test]
fn merge_sort_all_same_texture_id() {
    // Same texture_id, varying sort_bits in random order
    let v = vec![
        s(100, 5),
        s(100, -3),
        s(100, 9),
        s(100, 2),
        s(100, 0),
        s(100, -10),
    ];
    assert_match(&v);
}

#[test]
fn merge_sort_negative_sort_bits() {
    let v = vec![
        s(1, -5),
        s(2, 3),
        s(3, -10),
        s(4, 0),
        s(5, i32::MIN),
        s(6, i32::MAX),
    ];
    assert_match(&v);
}

#[test]
fn merge_sort_with_duplicates() {
    let v = vec![
        s(1, 1),
        s(1, 1),
        s(2, 2),
        s(2, 2),
        s(1, 1),
        s(3, 1),
        s(3, 2),
    ];
    assert_match(&v);
}

#[test]
fn merge_sort_large_random() {
    // Deterministic pseudo-random
    let mut v = Vec::new();
    let mut state: u64 = 0xDEADBEEFCAFEBABE;
    for _ in 0..1000 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let texture_id = state >> 32;
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let sort_bits = (state >> 32) as i32 % 100; // small range to get many ties
        v.push(s(texture_id, sort_bits));
    }
    assert_match(&v);
}

#[test]
fn merge_sort_max_values() {
    let v = vec![
        s(u64::MAX, i32::MAX),
        s(0, i32::MIN),
        s(u64::MAX, i32::MIN),
        s(0, i32::MAX),
        s(u64::MAX / 2, 0),
    ];
    assert_match(&v);
}

#[test]
fn merge_sort_three_elements_all_perms() {
    let perms = [
        [s(1, 1), s(2, 2), s(3, 3)],
        [s(1, 1), s(3, 3), s(2, 2)],
        [s(2, 2), s(1, 1), s(3, 3)],
        [s(2, 2), s(3, 3), s(1, 1)],
        [s(3, 3), s(1, 1), s(2, 2)],
        [s(3, 3), s(2, 2), s(1, 1)],
    ];
    for p in &perms {
        assert_match(p);
    }
}

#[test]
fn merge_sort_size_one() {
    // size=1 special case
    assert_match(&[s(0xFFFFFFFFFFFFFFFF, -1)]);
}
