use libloading::{Library, Symbol};
use merge_sort_lib::spritebatch_sprite_t;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libmerge_sort_lib.so")
}

type MergeSortFn = unsafe extern "C" fn(*mut spritebatch_sprite_t, *mut spritebatch_sprite_t, i32);

fn zeroed_buf(n: usize) -> Vec<spritebatch_sprite_t> {
    (0..n).map(|_| unsafe { std::mem::zeroed() }).collect()
}

fn copy_input(input: &[spritebatch_sprite_t]) -> Vec<spritebatch_sprite_t> {
    let mut v = zeroed_buf(input.len());
    unsafe { std::ptr::copy_nonoverlapping(input.as_ptr(), v.as_mut_ptr(), input.len()) };
    v
}

fn call_c_merge_sort(input: &[spritebatch_sprite_t]) -> Vec<spritebatch_sprite_t> {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let func: Symbol<MergeSortFn> = unsafe { lib.get(b"merge_sort").expect("merge_sort not found") };
    let mut a = copy_input(input);
    let mut b = zeroed_buf(a.len());
    unsafe { func(a.as_mut_ptr(), b.as_mut_ptr(), a.len() as i32) };
    a
}

fn call_rust_merge_sort(input: &[spritebatch_sprite_t]) -> Vec<spritebatch_sprite_t> {
    let mut a = copy_input(input);
    let mut b = zeroed_buf(a.len());
    unsafe { merge_sort_lib::merge_sort(a.as_mut_ptr(), b.as_mut_ptr(), a.len() as i32) };
    a
}

fn assert_results_match(label: &str, c_result: &[spritebatch_sprite_t], rust_result: &[spritebatch_sprite_t]) {
    assert_eq!(c_result.len(), rust_result.len(), "{label}: length mismatch");
    for (i, (c, r)) in c_result.iter().zip(rust_result.iter()).enumerate() {
        let c_bytes: [u8; std::mem::size_of::<spritebatch_sprite_t>()] = unsafe { std::mem::transmute_copy(c) };
        let r_bytes: [u8; std::mem::size_of::<spritebatch_sprite_t>()] = unsafe { std::mem::transmute_copy(r) };
        assert_eq!(c_bytes, r_bytes, "{label}: mismatch at index {i}: C=({},{}) Rust=({},{})",
            c.texture_id, c.sort_bits, r.texture_id, r.sort_bits);
    }
}

fn make_sprites(data: &[(u64, i32)]) -> Vec<spritebatch_sprite_t> {
    data.iter().map(|&(t, s)| {
        let mut sprite: spritebatch_sprite_t = unsafe { std::mem::zeroed() };
        sprite.texture_id = t;
        sprite.sort_bits = s;
        sprite
    }).collect()
}

#[test]
fn test_empty() {
    let input = make_sprites(&[]);
    let c = call_c_merge_sort(&input);
    let r = call_rust_merge_sort(&input);
    assert_results_match("empty", &c, &r);
}

#[test]
fn test_single() {
    let input = make_sprites(&[(42, 7)]);
    let c = call_c_merge_sort(&input);
    let r = call_rust_merge_sort(&input);
    assert_results_match("single", &c, &r);
}

#[test]
fn test_already_sorted() {
    let input = make_sprites(&[(1, 1), (2, 2), (3, 3), (4, 4)]);
    let c = call_c_merge_sort(&input);
    let r = call_rust_merge_sort(&input);
    assert_results_match("already_sorted", &c, &r);
}

#[test]
fn test_reverse_sorted() {
    let input = make_sprites(&[(4, 4), (3, 3), (2, 2), (1, 1)]);
    let c = call_c_merge_sort(&input);
    let r = call_rust_merge_sort(&input);
    assert_results_match("reverse_sorted", &c, &r);
}

#[test]
fn test_same_sort_bits_different_texture() {
    let input = make_sprites(&[(100, 5), (50, 5), (200, 5), (10, 5)]);
    let c = call_c_merge_sort(&input);
    let r = call_rust_merge_sort(&input);
    assert_results_match("same_sort_bits", &c, &r);
}

#[test]
fn test_mixed() {
    let input = make_sprites(&[
        (10, 3), (20, 1), (30, 2), (40, 1), (50, 3), (60, 2), (70, 1), (80, 2),
    ]);
    let c = call_c_merge_sort(&input);
    let r = call_rust_merge_sort(&input);
    assert_results_match("mixed", &c, &r);
}

#[test]
fn test_duplicates() {
    let input = make_sprites(&[(5, 5), (5, 5), (3, 3), (3, 3)]);
    let c = call_c_merge_sort(&input);
    let r = call_rust_merge_sort(&input);
    assert_results_match("duplicates", &c, &r);
}

#[test]
fn test_large() {
    let input: Vec<spritebatch_sprite_t> = (0..100)
        .map(|i| spritebatch_sprite_t { texture_id: (100 - i) as u64, sort_bits: ((i * 7) % 13) as i32 })
        .collect();
    let c = call_c_merge_sort(&input);
    let r = call_rust_merge_sort(&input);
    assert_results_match("large", &c, &r);
}
