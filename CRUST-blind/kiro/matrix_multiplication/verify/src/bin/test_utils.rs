use matrix_multiplication::utils;

#[test]
fn test_print_array_float() {
    let m = vec![
        vec![0.0f32, 1.0],
        vec![2.0, 3.0],
    ];
    let ret = utils::print_array(&m, "test", "float");
    assert_eq!(ret, 0);
}

#[test]
fn test_print_array_unsupported() {
    let m = vec![vec![1.0f32]];
    let ret = utils::print_array(&m, "test", "int");
    assert_eq!(ret, 0);
}

#[test]
fn test_print_float_array() {
    let m = vec![
        vec![1.0f32, 2.0, 3.0],
        vec![4.0, 5.0, 6.0],
    ];
    let ret = utils::print_float_array(&m);
    assert_eq!(ret, 0);
}

#[test]
fn test_print_float_array_empty() {
    let m: Vec<Vec<f32>> = vec![];
    let ret = utils::print_float_array(&m);
    assert_eq!(ret, 0);
}

fn main() {}
