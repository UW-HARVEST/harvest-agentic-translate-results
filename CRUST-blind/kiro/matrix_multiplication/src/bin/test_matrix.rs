use matrix_multiplication::matrix::*;
use matrix_multiplication::utils::*;

const ARRAY_TYPE_CHAR: &str = "float";

fn test_generate_matrix(x: usize, y: usize, name: &str, typ: &str) {
    let matrix = generate_matrix(x, y, false);
    print_array(&matrix, name, typ);
    // free_matrix equivalent: matrix is dropped
}

fn tests_generate_matrix() {
    test_generate_matrix(4, 4, "matrix", ARRAY_TYPE_CHAR);
    test_generate_matrix(3, 4, "matrix 2", ARRAY_TYPE_CHAR);
    test_generate_matrix(4, 3, "matrix 3", ARRAY_TYPE_CHAR);
    test_generate_matrix(0, 0, "matrix 4", ARRAY_TYPE_CHAR);
    test_generate_matrix(2, 2, "", ARRAY_TYPE_CHAR);
}

fn test_multiply(x1: usize, y1: usize, x2: usize, y2: usize, test_name: &str, method: i32) {
    let matrix1 = generate_matrix(x1, y1, false);
    let matrix2 = generate_matrix(x2, y2, false);
    let mut r = generate_matrix(x1, y2, true);
    let flag = multiply(&matrix1, &matrix2, &mut r, method);
    if flag != 0 {
        println!("x1:{}, y1:{}, x2:{}, y2:{}, method: {}: error", x1, y1, x2, y2, 1);
    } else {
        println!("====={}======", test_name);
        print_array(&matrix1, "result", ARRAY_TYPE_CHAR);
        print_array(&matrix2, "result", ARRAY_TYPE_CHAR);
        print_array(&r, "result", ARRAY_TYPE_CHAR);
        println!("=============");
    }
}

fn tests_multiply() {
    let (mut x1, mut y1, mut x2, mut y2) = (4, 4, 4, 4);
    test_multiply(x1, y1, x2, y2, "multiply 1-1", 1);
    test_multiply(x1, y1, x2, y2, "multiply 1-3", 3);

    x1 = 8; y1 = 8; x2 = 8; y2 = 8;
    test_multiply(x1, y1, x2, y2, "multiply 1-1", 1);
    test_multiply(x1, y1, x2, y2, "multiply 1-3", 3);
    test_multiply(x1, y1, x2, y2, "multiply 1-2", 2);

    x1 = 4; y1 = 4; x2 = 1; y2 = 4;
    test_multiply(x1, y1, x2, y2, "multiply 2-1", 1);
    test_multiply(x1, y1, x2, y2, "multiply 2-3", 3);

    x1 = 1; y1 = 1; x2 = 1; y2 = 1;
    test_multiply(x1, y1, x2, y2, "multiply 3-1", 1);
    test_multiply(x1, y1, x2, y2, "multiply 3-3", 3);

    x1 = 1; y1 = 2; x2 = 2; y2 = 1;
    test_multiply(x1, y1, x2, y2, "multiply 4-1", 1);
    test_multiply(x1, y1, x2, y2, "multiply 4-3", 3);

    x1 = 1; y1 = 2; x2 = 2; y2 = 1;
    test_multiply(x1, y1, x2, y2, "multiply 5-0", 0);
}

fn main() {
    tests_multiply();
}
