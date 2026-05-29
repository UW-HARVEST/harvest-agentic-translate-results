use matrix_multiplication::matrix;

#[test]
fn test_generate_matrix_3x3_pattern() {
    // From running C: generate_matrix(3,3,false) gives row-major [j + i*x]
    let m = matrix::generate_matrix(3, 3, false);
    assert_eq!(m.len(), 3);
    assert_eq!(m[0], vec![0.0_f32, 1.0, 2.0]);
    assert_eq!(m[1], vec![3.0_f32, 4.0, 5.0]);
    assert_eq!(m[2], vec![6.0_f32, 7.0, 8.0]);
}

#[test]
fn test_generate_matrix_4x4_pattern() {
    // C ground truth output for generate_matrix(4,4,false)
    let m = matrix::generate_matrix(4, 4, false);
    assert_eq!(m.len(), 4);
    for i in 0..4 {
        assert_eq!(m[i].len(), 4);
        for j in 0..4 {
            let expected = (j + i * 4) as f32;
            assert_eq!(m[i][j], expected, "mismatch at [{}][{}]", i, j);
        }
    }
    // Spot-check a few exact values from the C run.
    assert_eq!(m[0][0], 0.0);
    assert_eq!(m[1][1], 5.0);
    assert_eq!(m[3][3], 15.0);
}

#[test]
fn test_generate_matrix_1x1_pattern() {
    let m = matrix::generate_matrix(1, 1, false);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].len(), 1);
    assert_eq!(m[0][0], 0.0);
}

#[test]
fn test_generate_matrix_5x5_pattern() {
    let m = matrix::generate_matrix(5, 5, false);
    assert_eq!(m.len(), 5);
    let expected: Vec<Vec<f32>> = vec![
        vec![0.0, 1.0, 2.0, 3.0, 4.0],
        vec![5.0, 6.0, 7.0, 8.0, 9.0],
        vec![10.0, 11.0, 12.0, 13.0, 14.0],
        vec![15.0, 16.0, 17.0, 18.0, 19.0],
        vec![20.0, 21.0, 22.0, 23.0, 24.0],
    ];
    assert_eq!(m, expected);
}

#[test]
fn test_generate_matrix_random_shape_and_values() {
    // Random mode: shape must be x by y, all values must be finite and non-negative
    // (mirroring rand() returning a non-negative int cast to float).
    let m = matrix::generate_matrix(4, 7, true);
    assert_eq!(m.len(), 4);
    for row in &m {
        assert_eq!(row.len(), 7);
        for &v in row {
            assert!(v.is_finite());
            assert!(v >= 0.0, "expected non-negative, got {}", v);
        }
    }
}

#[test]
fn test_free_matrix_clears_and_returns_zero() {
    let mut m = matrix::generate_matrix(3, 3, false);
    assert_eq!(m.len(), 3);
    let rc = matrix::free_matrix(&mut m);
    assert_eq!(rc, 0);
    assert_eq!(m.len(), 0);
}

#[test]
fn test_free_matrix_on_empty() {
    let mut m: Vec<Vec<f32>> = Vec::new();
    let rc = matrix::free_matrix(&mut m);
    assert_eq!(rc, 0);
    assert_eq!(m.len(), 0);
}

fn make_m1(x: usize, y: usize) -> Vec<Vec<f32>> {
    let mut m = vec![vec![0.0_f32; y]; x];
    for i in 0..x {
        for j in 0..y {
            m[i][j] = (i + j + 1) as f32;
        }
    }
    m
}

fn make_m2(x: usize, y: usize) -> Vec<Vec<f32>> {
    let mut m = vec![vec![0.0_f32; y]; x];
    for i in 0..x {
        for j in 0..y {
            m[i][j] = ((i + 1) * (j + 1)) as f32;
        }
    }
    m
}

#[test]
fn test_algorithm1_2x3_times_3x4() {
    // C ground truth alg1(2,3,3,4)
    let m1 = make_m1(2, 3);
    let m2 = make_m2(3, 4);
    let mut r = vec![vec![0.0_f32; 4]; 2];
    matrix::algorithm1(&m1, &m2, &mut r);
    assert_eq!(r[0], vec![14.0, 28.0, 42.0, 56.0]);
    assert_eq!(r[1], vec![20.0, 40.0, 60.0, 80.0]);
}

#[test]
fn test_algorithm1_1x1() {
    let m1 = make_m1(1, 1);
    let m2 = make_m2(1, 1);
    let mut r = vec![vec![0.0_f32; 1]; 1];
    matrix::algorithm1(&m1, &m2, &mut r);
    assert_eq!(r[0][0], 1.0);
}

#[test]
fn test_algorithm1_3x2_times_2x5() {
    // C ground truth alg1(3,2,2,5)
    let m1 = make_m1(3, 2);
    let m2 = make_m2(2, 5);
    let mut r = vec![vec![0.0_f32; 5]; 3];
    matrix::algorithm1(&m1, &m2, &mut r);
    assert_eq!(r[0], vec![5.0, 10.0, 15.0, 20.0, 25.0]);
    assert_eq!(r[1], vec![8.0, 16.0, 24.0, 32.0, 40.0]);
    assert_eq!(r[2], vec![11.0, 22.0, 33.0, 44.0, 55.0]);
}

#[test]
fn test_algorithm1_4x4_square() {
    // C ground truth alg1(4,4,4,4)
    let m1 = make_m1(4, 4);
    let m2 = make_m2(4, 4);
    let mut r = vec![vec![0.0_f32; 4]; 4];
    matrix::algorithm1(&m1, &m2, &mut r);
    assert_eq!(r[0], vec![30.0, 60.0, 90.0, 120.0]);
    assert_eq!(r[1], vec![40.0, 80.0, 120.0, 160.0]);
    assert_eq!(r[2], vec![50.0, 100.0, 150.0, 200.0]);
    assert_eq!(r[3], vec![60.0, 120.0, 180.0, 240.0]);
}

#[test]
fn test_algorithm3_2x3_times_3x4() {
    let m1 = make_m1(2, 3);
    let m2 = make_m2(3, 4);
    let mut r = vec![vec![999.0_f32; 4]; 2]; // pre-fill to verify zero-init
    matrix::algorithm3(&m1, &m2, &mut r);
    assert_eq!(r[0], vec![14.0, 28.0, 42.0, 56.0]);
    assert_eq!(r[1], vec![20.0, 40.0, 60.0, 80.0]);
}

#[test]
fn test_algorithm3_1x1() {
    let m1 = make_m1(1, 1);
    let m2 = make_m2(1, 1);
    let mut r = vec![vec![0.0_f32; 1]; 1];
    matrix::algorithm3(&m1, &m2, &mut r);
    assert_eq!(r[0][0], 1.0);
}

#[test]
fn test_algorithm3_3x2_times_2x5() {
    let m1 = make_m1(3, 2);
    let m2 = make_m2(2, 5);
    let mut r = vec![vec![0.0_f32; 5]; 3];
    matrix::algorithm3(&m1, &m2, &mut r);
    assert_eq!(r[0], vec![5.0, 10.0, 15.0, 20.0, 25.0]);
    assert_eq!(r[1], vec![8.0, 16.0, 24.0, 32.0, 40.0]);
    assert_eq!(r[2], vec![11.0, 22.0, 33.0, 44.0, 55.0]);
}

#[test]
fn test_algorithm3_4x4_square() {
    let m1 = make_m1(4, 4);
    let m2 = make_m2(4, 4);
    let mut r = vec![vec![0.0_f32; 4]; 4];
    matrix::algorithm3(&m1, &m2, &mut r);
    assert_eq!(r[0], vec![30.0, 60.0, 90.0, 120.0]);
    assert_eq!(r[1], vec![40.0, 80.0, 120.0, 160.0]);
    assert_eq!(r[2], vec![50.0, 100.0, 150.0, 200.0]);
    assert_eq!(r[3], vec![60.0, 120.0, 180.0, 240.0]);
}

#[test]
fn test_algorithm2_size_8() {
    // C ground truth alg2(8) — square multiplication (size must be multiple of 8 in C
    // due to AVX intrinsics). The Rust version operates on any square size, but we use
    // multiples of 8 to match exactly the C ground truth output.
    let m1 = make_m1(8, 8);
    let m2 = make_m2(8, 8);
    let mut r = vec![vec![0.0_f32; 8]; 8];
    matrix::algorithm2(&m1, &m2, &mut r);
    let expected: Vec<Vec<f32>> = vec![
        vec![204.0, 408.0, 612.0, 816.0, 1020.0, 1224.0, 1428.0, 1632.0],
        vec![240.0, 480.0, 720.0, 960.0, 1200.0, 1440.0, 1680.0, 1920.0],
        vec![276.0, 552.0, 828.0, 1104.0, 1380.0, 1656.0, 1932.0, 2208.0],
        vec![312.0, 624.0, 936.0, 1248.0, 1560.0, 1872.0, 2184.0, 2496.0],
        vec![348.0, 696.0, 1044.0, 1392.0, 1740.0, 2088.0, 2436.0, 2784.0],
        vec![384.0, 768.0, 1152.0, 1536.0, 1920.0, 2304.0, 2688.0, 3072.0],
        vec![420.0, 840.0, 1260.0, 1680.0, 2100.0, 2520.0, 2940.0, 3360.0],
        vec![456.0, 912.0, 1368.0, 1824.0, 2280.0, 2736.0, 3192.0, 3648.0],
    ];
    assert_eq!(r, expected);
}

#[test]
fn test_algorithm2_size_16() {
    // C ground truth alg2(16). Compare a few exact rows for full precision.
    let m1 = make_m1(16, 16);
    let m2 = make_m2(16, 16);
    let mut r = vec![vec![0.0_f32; 16]; 16];
    matrix::algorithm2(&m1, &m2, &mut r);
    assert_eq!(
        r[0],
        vec![
            1496.0, 2992.0, 4488.0, 5984.0, 7480.0, 8976.0, 10472.0, 11968.0, 13464.0, 14960.0,
            16456.0, 17952.0, 19448.0, 20944.0, 22440.0, 23936.0,
        ]
    );
    assert_eq!(
        r[15],
        vec![
            3536.0, 7072.0, 10608.0, 14144.0, 17680.0, 21216.0, 24752.0, 28288.0, 31824.0, 35360.0,
            38896.0, 42432.0, 45968.0, 49504.0, 53040.0, 56576.0,
        ]
    );
    assert_eq!(
        r[7],
        vec![
            2448.0, 4896.0, 7344.0, 9792.0, 12240.0, 14688.0, 17136.0, 19584.0, 22032.0, 24480.0,
            26928.0, 29376.0, 31824.0, 34272.0, 36720.0, 39168.0,
        ]
    );
}

#[test]
fn test_multiply_method1_returns_zero_and_correct_values() {
    // C ground truth: multiply method=1 (2,3)x(3,2) rc=0; values [[14,28],[20,40]]
    let m1 = make_m1(2, 3);
    let m2 = make_m2(3, 2);
    let mut r = vec![vec![0.0_f32; 2]; 2];
    let rc = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(rc, 0);
    assert_eq!(r[0], vec![14.0, 28.0]);
    assert_eq!(r[1], vec![20.0, 40.0]);
}

#[test]
fn test_multiply_method3_returns_zero_and_correct_values() {
    let m1 = make_m1(2, 3);
    let m2 = make_m2(3, 2);
    let mut r = vec![vec![0.0_f32; 2]; 2];
    let rc = matrix::multiply(&m1, &m2, &mut r, 3);
    assert_eq!(rc, 0);
    assert_eq!(r[0], vec![14.0, 28.0]);
    assert_eq!(r[1], vec![20.0, 40.0]);
}

#[test]
fn test_multiply_method2_square() {
    // method 2 on size 8 square — same as alg2 ground truth
    let m1 = make_m1(8, 8);
    let m2 = make_m2(8, 8);
    let mut r = vec![vec![0.0_f32; 8]; 8];
    let rc = matrix::multiply(&m1, &m2, &mut r, 2);
    assert_eq!(rc, 0);
    assert_eq!(r[0][0], 204.0);
    assert_eq!(r[7][7], 3648.0);
    assert_eq!(r[3][4], 1560.0);
}

#[test]
fn test_multiply_dimension_mismatch_returns_neg1() {
    // C ground truth: y1=3, x2=4 mismatch → multiply returns -1
    let m1 = make_m1(2, 3);
    let m2 = make_m2(4, 2);
    let mut r = vec![vec![0.0_f32; 2]; 2];
    let rc = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(rc, -1);
}

#[test]
fn test_multiply_unknown_method_returns_neg1() {
    // C ground truth: invalid method → multiply returns -1
    let m1 = make_m1(2, 3);
    let m2 = make_m2(3, 2);
    let mut r = vec![vec![0.0_f32; 2]; 2];
    let rc = matrix::multiply(&m1, &m2, &mut r, 99);
    assert_eq!(rc, -1);
}

#[test]
fn test_multiply_method_zero_returns_neg1() {
    let m1 = make_m1(2, 3);
    let m2 = make_m2(3, 2);
    let mut r = vec![vec![0.0_f32; 2]; 2];
    let rc = matrix::multiply(&m1, &m2, &mut r, 0);
    assert_eq!(rc, -1);
}

fn main() {}
