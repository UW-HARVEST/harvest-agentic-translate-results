use libloading::{Library, Symbol};
use std::ffi::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sprite {
    texture_id: u64,
    sort_bits: c_int,
}

type MergeSortFn = unsafe extern "C" fn(*mut Sprite, *mut Sprite, c_int);

fn load_libs() -> (Library, Library) {
    unsafe {
        let c_lib = Library::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("c_src/build/libtranslated_rust.so"),
        )
        .expect("load C .so");
        let rust_lib = Library::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/libmerge_sort_lib.so"),
        )
        .expect("load Rust .so");
        (c_lib, rust_lib)
    }
}

fn run_both(c_lib: &Library, rust_lib: &Library, input: &[Sprite]) -> (Vec<Sprite>, Vec<Sprite>) {
    unsafe {
        let c_fn: Symbol<MergeSortFn> = c_lib.get(b"merge_sort").unwrap();
        let r_fn: Symbol<MergeSortFn> = rust_lib.get(b"merge_sort").unwrap();

        let mut c_a = input.to_vec();
        let mut c_b = vec![Sprite { texture_id: 0, sort_bits: 0 }; input.len()];
        c_fn(c_a.as_mut_ptr(), c_b.as_mut_ptr(), input.len() as c_int);

        let mut r_a = input.to_vec();
        let mut r_b = vec![Sprite { texture_id: 0, sort_bits: 0 }; input.len()];
        r_fn(r_a.as_mut_ptr(), r_b.as_mut_ptr(), input.len() as c_int);

        (c_a, r_a)
    }
}

fn assert_match(label: &str, input: &[Sprite], c_out: &[Sprite], r_out: &[Sprite]) {
    assert_eq!(
        c_out, r_out,
        "{label}: mismatch for input {input:?}\nC:    {c_out:?}\nRust: {r_out:?}"
    );
}

#[test]
fn test_empty() {
    let (c_lib, r_lib) = load_libs();
    let (c, r) = run_both(&c_lib, &r_lib, &[]);
    assert_match("empty", &[], &c, &r);
}

#[test]
fn test_single() {
    let (c_lib, r_lib) = load_libs();
    let input = [Sprite { texture_id: 42, sort_bits: 7 }];
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("single", &input, &c, &r);
}

#[test]
fn test_already_sorted() {
    let (c_lib, r_lib) = load_libs();
    let input: Vec<Sprite> = (0..5)
        .map(|i| Sprite { texture_id: i as u64, sort_bits: i })
        .collect();
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("sorted", &input, &c, &r);
}

#[test]
fn test_reverse_sorted() {
    let (c_lib, r_lib) = load_libs();
    let input: Vec<Sprite> = (0..5)
        .rev()
        .map(|i| Sprite { texture_id: i as u64, sort_bits: i })
        .collect();
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("reverse", &input, &c, &r);
}

#[test]
fn test_same_sort_bits_different_texture() {
    let (c_lib, r_lib) = load_libs();
    let input = vec![
        Sprite { texture_id: 100, sort_bits: 1 },
        Sprite { texture_id: 50, sort_bits: 1 },
        Sprite { texture_id: 200, sort_bits: 1 },
        Sprite { texture_id: 1, sort_bits: 1 },
    ];
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("same_sort_bits", &input, &c, &r);
}

#[test]
fn test_mixed() {
    let (c_lib, r_lib) = load_libs();
    let input = vec![
        Sprite { texture_id: 3, sort_bits: 2 },
        Sprite { texture_id: 1, sort_bits: 1 },
        Sprite { texture_id: 2, sort_bits: 3 },
        Sprite { texture_id: 5, sort_bits: 1 },
        Sprite { texture_id: 4, sort_bits: 2 },
    ];
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("mixed", &input, &c, &r);
}

#[test]
fn test_negative_sort_bits() {
    let (c_lib, r_lib) = load_libs();
    let input = vec![
        Sprite { texture_id: 1, sort_bits: -5 },
        Sprite { texture_id: 2, sort_bits: 3 },
        Sprite { texture_id: 3, sort_bits: -1 },
        Sprite { texture_id: 4, sort_bits: 0 },
    ];
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("negative", &input, &c, &r);
}

#[test]
fn test_large_texture_ids() {
    let (c_lib, r_lib) = load_libs();
    let input = vec![
        Sprite { texture_id: u64::MAX, sort_bits: 1 },
        Sprite { texture_id: 0, sort_bits: 1 },
        Sprite { texture_id: u64::MAX / 2, sort_bits: 1 },
    ];
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("large_tex", &input, &c, &r);
}

#[test]
fn test_duplicates() {
    let (c_lib, r_lib) = load_libs();
    let input = vec![
        Sprite { texture_id: 1, sort_bits: 1 },
        Sprite { texture_id: 1, sort_bits: 1 },
        Sprite { texture_id: 1, sort_bits: 1 },
    ];
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("duplicates", &input, &c, &r);
}

#[test]
fn test_two_elements() {
    let (c_lib, r_lib) = load_libs();
    let input = vec![
        Sprite { texture_id: 10, sort_bits: 5 },
        Sprite { texture_id: 20, sort_bits: 2 },
    ];
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("two_elem", &input, &c, &r);
}

#[test]
fn test_power_of_two_size() {
    let (c_lib, r_lib) = load_libs();
    let input: Vec<Sprite> = (0..8)
        .rev()
        .map(|i| Sprite { texture_id: (i * 7 % 11) as u64, sort_bits: i })
        .collect();
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("pow2", &input, &c, &r);
}

#[test]
fn test_odd_size() {
    let (c_lib, r_lib) = load_libs();
    let input: Vec<Sprite> = (0..7)
        .rev()
        .map(|i| Sprite { texture_id: (i * 3 % 5) as u64, sort_bits: i % 3 })
        .collect();
    let (c, r) = run_both(&c_lib, &r_lib, &input);
    assert_match("odd_size", &input, &c, &r);
}
