// The `defs` module exports the `countof!` macro via `#[macro_export]`.
// In C, COUNTOF(x) computes sizeof(x)/sizeof(x[0]).
use Graph_recogniser::countof;

#[test]
fn test_countof_array_literal() {
    assert_eq!(countof!([1, 2, 3]), 3);
    assert_eq!(countof!([0; 10]), 10);
    let empty: [i32; 0] = [];
    assert_eq!(countof!(empty), 0);
}

#[test]
fn test_countof_array_variable() {
    let arr = [10u32, 20, 30, 40, 50];
    assert_eq!(countof!(arr), 5);
}

#[test]
fn test_countof_string_array() {
    // Mirrors the `test_strs` array used in c_src/tests
    let test_strs: [[&str; 2]; 12] = [
        ["stefan", "manov"],
        ["hristo", "tenchev"],
        ["dimitar", "kajabachev"],
        ["georgi", "popov"],
        ["stanislav", "ivanov"],
        ["nikola", "yolov"],
        ["andrei", "radev"],
        ["iulen", "dobrev"],
        ["iasen", "bantchev"],
        ["samuele", "carli"],
        ["henning", "weiler"],
        ["javier", "martin"],
    ];
    assert_eq!(countof!(test_strs), 12);
}

#[test]
fn test_countof_permutation_array() {
    let permut: [u32; 21] = [
        10, 0, 4, 3, 5, 3, 7, 11, 4, 11, 6, 0, 1, 8, 5, 1, 10, 3, 5, 2, 9,
    ];
    assert_eq!(countof!(permut), 21);
}

#[test]
fn test_countof_slice() {
    let v: Vec<i32> = vec![1, 2, 3, 4];
    let slice: &[i32] = &v;
    assert_eq!(countof!(slice), 4);
}

fn main() {}
