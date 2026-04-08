use avalanche::avalanche::Matrix;

#[test]
fn test_macros() {
    let mut m = Matrix::matrix_alloc(2, 2);
    for r in 0..2 {
        for c in 0..2 {
            m.matrix_set(r, c, 1.0 / (r + c + 1) as f64);
        }
    }
    for r in 0..2 {
        for c in 0..2 {
            let expected = 1.0 / (r + c + 1) as f64;
            assert_eq!(expected, m.matrix_get(r, c));
        }
    }
    println!("test_macros PASS");
}

#[test]
fn print_hilbert() {
    let mut m = Matrix::matrix_alloc(3, 3);
    for r in 0..3 {
        for c in 0..3 {
            m.matrix_set(r, c, 1.0 / (r + c + 1) as f64);
        }
    }
    let mut out = Vec::new();
    m.matrix_fprintf(&mut out, "%8.4f");
    let output = String::from_utf8(out).unwrap();
    print!("{}", output);
}
