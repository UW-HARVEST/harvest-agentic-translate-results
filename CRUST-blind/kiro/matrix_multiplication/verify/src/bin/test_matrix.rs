use matrix_multiplication::matrix;

fn make_result(rows: usize, cols: usize) -> Vec<Vec<f32>> {
    vec![vec![0.0f32; cols]; rows]
}

// --- generate_matrix tests ---

#[test]
fn test_generate_matrix_4x4() {
    let m = matrix::generate_matrix(4, 4, false);
    assert_eq!(m, vec![
        vec![0.0, 1.0, 2.0, 3.0],
        vec![4.0, 5.0, 6.0, 7.0],
        vec![8.0, 9.0, 10.0, 11.0],
        vec![12.0, 13.0, 14.0, 15.0],
    ]);
}

#[test]
fn test_generate_matrix_3x4() {
    let m = matrix::generate_matrix(3, 4, false);
    assert_eq!(m, vec![
        vec![0.0, 1.0, 2.0, 3.0],
        vec![3.0, 4.0, 5.0, 6.0],
        vec![6.0, 7.0, 8.0, 9.0],
    ]);
}

#[test]
fn test_generate_matrix_4x3() {
    let m = matrix::generate_matrix(4, 3, false);
    assert_eq!(m, vec![
        vec![0.0, 1.0, 2.0],
        vec![4.0, 5.0, 6.0],
        vec![8.0, 9.0, 10.0],
        vec![12.0, 13.0, 14.0],
    ]);
}

#[test]
fn test_generate_matrix_2x2() {
    let m = matrix::generate_matrix(2, 2, false);
    assert_eq!(m, vec![
        vec![0.0, 1.0],
        vec![2.0, 3.0],
    ]);
}

#[test]
fn test_generate_matrix_0x0() {
    let m = matrix::generate_matrix(0, 0, false);
    assert!(m.is_empty());
}

// --- free_matrix tests ---

#[test]
fn test_free_matrix() {
    let mut m = matrix::generate_matrix(2, 2, false);
    let ret = matrix::free_matrix(&mut m);
    assert_eq!(ret, 0);
    assert!(m.is_empty());
}

// --- multiply with method 1 (algorithm1) ---

#[test]
fn test_multiply_4x4_method1() {
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(4, 4, false);
    let mut r = make_result(4, 4);
    let ret = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(ret, 0);
    assert_eq!(r, vec![
        vec![56.0, 62.0, 68.0, 74.0],
        vec![152.0, 174.0, 196.0, 218.0],
        vec![248.0, 286.0, 324.0, 362.0],
        vec![344.0, 398.0, 452.0, 506.0],
    ]);
}

#[test]
fn test_multiply_4x4_method3() {
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(4, 4, false);
    let mut r = make_result(4, 4);
    let ret = matrix::multiply(&m1, &m2, &mut r, 3);
    assert_eq!(ret, 0);
    assert_eq!(r, vec![
        vec![56.0, 62.0, 68.0, 74.0],
        vec![152.0, 174.0, 196.0, 218.0],
        vec![248.0, 286.0, 324.0, 362.0],
        vec![344.0, 398.0, 452.0, 506.0],
    ]);
}

#[test]
fn test_multiply_8x8_method1() {
    let m1 = matrix::generate_matrix(8, 8, false);
    let m2 = matrix::generate_matrix(8, 8, false);
    let mut r = make_result(8, 8);
    let ret = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(ret, 0);
    let expected = vec![
        vec![1120.0, 1148.0, 1176.0, 1204.0, 1232.0, 1260.0, 1288.0, 1316.0],
        vec![2912.0, 3004.0, 3096.0, 3188.0, 3280.0, 3372.0, 3464.0, 3556.0],
        vec![4704.0, 4860.0, 5016.0, 5172.0, 5328.0, 5484.0, 5640.0, 5796.0],
        vec![6496.0, 6716.0, 6936.0, 7156.0, 7376.0, 7596.0, 7816.0, 8036.0],
        vec![8288.0, 8572.0, 8856.0, 9140.0, 9424.0, 9708.0, 9992.0, 10276.0],
        vec![10080.0, 10428.0, 10776.0, 11124.0, 11472.0, 11820.0, 12168.0, 12516.0],
        vec![11872.0, 12284.0, 12696.0, 13108.0, 13520.0, 13932.0, 14344.0, 14756.0],
        vec![13664.0, 14140.0, 14616.0, 15092.0, 15568.0, 16044.0, 16520.0, 16996.0],
    ];
    assert_eq!(r, expected);
}

#[test]
fn test_multiply_8x8_method2() {
    let m1 = matrix::generate_matrix(8, 8, false);
    let m2 = matrix::generate_matrix(8, 8, false);
    let mut r = make_result(8, 8);
    let ret = matrix::multiply(&m1, &m2, &mut r, 2);
    assert_eq!(ret, 0);
    let expected = vec![
        vec![1120.0, 1148.0, 1176.0, 1204.0, 1232.0, 1260.0, 1288.0, 1316.0],
        vec![2912.0, 3004.0, 3096.0, 3188.0, 3280.0, 3372.0, 3464.0, 3556.0],
        vec![4704.0, 4860.0, 5016.0, 5172.0, 5328.0, 5484.0, 5640.0, 5796.0],
        vec![6496.0, 6716.0, 6936.0, 7156.0, 7376.0, 7596.0, 7816.0, 8036.0],
        vec![8288.0, 8572.0, 8856.0, 9140.0, 9424.0, 9708.0, 9992.0, 10276.0],
        vec![10080.0, 10428.0, 10776.0, 11124.0, 11472.0, 11820.0, 12168.0, 12516.0],
        vec![11872.0, 12284.0, 12696.0, 13108.0, 13520.0, 13932.0, 14344.0, 14756.0],
        vec![13664.0, 14140.0, 14616.0, 15092.0, 15568.0, 16044.0, 16520.0, 16996.0],
    ];
    assert_eq!(r, expected);
}

#[test]
fn test_multiply_8x8_method3() {
    let m1 = matrix::generate_matrix(8, 8, false);
    let m2 = matrix::generate_matrix(8, 8, false);
    let mut r = make_result(8, 8);
    let ret = matrix::multiply(&m1, &m2, &mut r, 3);
    assert_eq!(ret, 0);
    let expected = vec![
        vec![1120.0, 1148.0, 1176.0, 1204.0, 1232.0, 1260.0, 1288.0, 1316.0],
        vec![2912.0, 3004.0, 3096.0, 3188.0, 3280.0, 3372.0, 3464.0, 3556.0],
        vec![4704.0, 4860.0, 5016.0, 5172.0, 5328.0, 5484.0, 5640.0, 5796.0],
        vec![6496.0, 6716.0, 6936.0, 7156.0, 7376.0, 7596.0, 7816.0, 8036.0],
        vec![8288.0, 8572.0, 8856.0, 9140.0, 9424.0, 9708.0, 9992.0, 10276.0],
        vec![10080.0, 10428.0, 10776.0, 11124.0, 11472.0, 11820.0, 12168.0, 12516.0],
        vec![11872.0, 12284.0, 12696.0, 13108.0, 13520.0, 13932.0, 14344.0, 14756.0],
        vec![13664.0, 14140.0, 14616.0, 15092.0, 15568.0, 16044.0, 16520.0, 16996.0],
    ];
    assert_eq!(r, expected);
}

// --- 1x1 multiply ---

#[test]
fn test_multiply_1x1_method1() {
    let m1 = matrix::generate_matrix(1, 1, false);
    let m2 = matrix::generate_matrix(1, 1, false);
    let mut r = make_result(1, 1);
    let ret = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(ret, 0);
    assert_eq!(r, vec![vec![0.0]]);
}

#[test]
fn test_multiply_1x1_method3() {
    let m1 = matrix::generate_matrix(1, 1, false);
    let m2 = matrix::generate_matrix(1, 1, false);
    let mut r = make_result(1, 1);
    let ret = matrix::multiply(&m1, &m2, &mut r, 3);
    assert_eq!(ret, 0);
    assert_eq!(r, vec![vec![0.0]]);
}

// --- 1x2 * 2x1 multiply ---

#[test]
fn test_multiply_1x2_2x1_method1() {
    let m1 = matrix::generate_matrix(1, 2, false);
    let m2 = matrix::generate_matrix(2, 1, false);
    let mut r = make_result(1, 1);
    let ret = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(ret, 0);
    assert_eq!(r, vec![vec![2.0]]);
}

#[test]
fn test_multiply_1x2_2x1_method3() {
    let m1 = matrix::generate_matrix(1, 2, false);
    let m2 = matrix::generate_matrix(2, 1, false);
    let mut r = make_result(1, 1);
    let ret = matrix::multiply(&m1, &m2, &mut r, 3);
    assert_eq!(ret, 0);
    assert_eq!(r, vec![vec![2.0]]);
}

// --- dimension mismatch ---

#[test]
fn test_multiply_dimension_mismatch() {
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(1, 4, false);
    let mut r = make_result(4, 4);
    let ret = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(ret, -1);
}

// --- invalid method ---

#[test]
fn test_multiply_invalid_method() {
    let m1 = matrix::generate_matrix(1, 2, false);
    let m2 = matrix::generate_matrix(2, 1, false);
    let mut r = make_result(1, 1);
    let ret = matrix::multiply(&m1, &m2, &mut r, 0);
    assert_eq!(ret, -1);
}

// --- algorithm1 directly ---

#[test]
fn test_algorithm1_direct() {
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(4, 4, false);
    let mut r = make_result(4, 4);
    matrix::algorithm1(&m1, &m2, &mut r);
    assert_eq!(r, vec![
        vec![56.0, 62.0, 68.0, 74.0],
        vec![152.0, 174.0, 196.0, 218.0],
        vec![248.0, 286.0, 324.0, 362.0],
        vec![344.0, 398.0, 452.0, 506.0],
    ]);
}

// --- algorithm3 directly ---

#[test]
fn test_algorithm3_direct() {
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(4, 4, false);
    let mut r = make_result(4, 4);
    matrix::algorithm3(&m1, &m2, &mut r);
    assert_eq!(r, vec![
        vec![56.0, 62.0, 68.0, 74.0],
        vec![152.0, 174.0, 196.0, 218.0],
        vec![248.0, 286.0, 324.0, 362.0],
        vec![344.0, 398.0, 452.0, 506.0],
    ]);
}

// --- algorithm2 directly ---

#[test]
fn test_algorithm2_direct() {
    let m1 = matrix::generate_matrix(8, 8, false);
    let m2 = matrix::generate_matrix(8, 8, false);
    let mut r = make_result(8, 8);
    matrix::algorithm2(&m1, &m2, &mut r);
    let expected = vec![
        vec![1120.0, 1148.0, 1176.0, 1204.0, 1232.0, 1260.0, 1288.0, 1316.0],
        vec![2912.0, 3004.0, 3096.0, 3188.0, 3280.0, 3372.0, 3464.0, 3556.0],
        vec![4704.0, 4860.0, 5016.0, 5172.0, 5328.0, 5484.0, 5640.0, 5796.0],
        vec![6496.0, 6716.0, 6936.0, 7156.0, 7376.0, 7596.0, 7816.0, 8036.0],
        vec![8288.0, 8572.0, 8856.0, 9140.0, 9424.0, 9708.0, 9992.0, 10276.0],
        vec![10080.0, 10428.0, 10776.0, 11124.0, 11472.0, 11820.0, 12168.0, 12516.0],
        vec![11872.0, 12284.0, 12696.0, 13108.0, 13520.0, 13932.0, 14344.0, 14756.0],
        vec![13664.0, 14140.0, 14616.0, 15092.0, 15568.0, 16044.0, 16520.0, 16996.0],
    ];
    assert_eq!(r, expected);
}

fn main() {}
