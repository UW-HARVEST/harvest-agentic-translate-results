use matrix_multiplication::utils;

#[test]
fn test_print_float_array_returns_zero() {
    let m: Vec<Vec<f32>> = vec![
        vec![1.0, 2.0, 3.0],
        vec![4.0, 5.0, 6.0],
    ];
    let rc = utils::print_float_array(&m);
    assert_eq!(rc, 0);
}

#[test]
fn test_print_float_array_empty() {
    let m: Vec<Vec<f32>> = vec![];
    let rc = utils::print_float_array(&m);
    assert_eq!(rc, 0);
}

#[test]
fn test_print_array_float_returns_zero() {
    let m: Vec<Vec<f32>> = vec![
        vec![0.0, 1.0],
        vec![2.0, 3.0],
    ];
    let rc = utils::print_array(&m, "matrix", "float");
    assert_eq!(rc, 0);
}

#[test]
fn test_print_array_unsupported_type_returns_zero() {
    // The C code prints a warning but still returns 0 for unsupported types.
    let m: Vec<Vec<f32>> = vec![vec![1.0, 2.0]];
    let rc = utils::print_array(&m, "test", "int");
    assert_eq!(rc, 0);
}

#[test]
fn test_print_array_empty_name() {
    let m: Vec<Vec<f32>> = vec![vec![0.0]];
    let rc = utils::print_array(&m, "", "float");
    assert_eq!(rc, 0);
}

fn main() {}
