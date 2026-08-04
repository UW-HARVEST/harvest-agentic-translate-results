use matrix_multiplication::utils;

#[test]
fn test_print_float_array_returns_zero() {
    let m = vec![vec![1.0_f32, 2.0], vec![3.0, 4.0]];
    let rc = utils::print_float_array(&m);
    assert_eq!(rc, 0);
}

#[test]
fn test_print_float_array_empty_returns_zero() {
    let m: Vec<Vec<f32>> = Vec::new();
    let rc = utils::print_float_array(&m);
    assert_eq!(rc, 0);
}

#[test]
fn test_print_array_float_type_returns_zero() {
    let m = vec![vec![1.5_f32, 2.5], vec![3.5, 4.5]];
    let rc = utils::print_array(&m, "test", "float");
    assert_eq!(rc, 0);
}

#[test]
fn test_print_array_unsupported_type_returns_zero() {
    // C: when type != "float", prints unsupported message but still returns 0.
    let m = vec![vec![1.0_f32]];
    let rc = utils::print_array(&m, "ints", "int");
    assert_eq!(rc, 0);
}

#[test]
fn test_print_array_empty_matrix_with_float_returns_zero() {
    let m: Vec<Vec<f32>> = Vec::new();
    let rc = utils::print_array(&m, "empty", "float");
    assert_eq!(rc, 0);
}

#[test]
fn test_print_array_empty_name_and_unsupported_type_returns_zero() {
    let m = vec![vec![0.0_f32]];
    let rc = utils::print_array(&m, "", "double");
    assert_eq!(rc, 0);
}

fn main() {}
