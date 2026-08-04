use c_aces::polynomial::{PolyArray, Polynomial};

#[test]
fn test_new_and_struct() {
    let p = Polynomial::new(vec![1, 2, 3, 4]);
    assert_eq!(p.coeffs, vec![1, 2, 3, 4]);
}

#[test]
fn test_set_zero() {
    let mut p = Polynomial::new(vec![5, 6, 7, 8, 9, 10]);
    p.set_zero();
    assert_eq!(p.coeffs, vec![0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_set_zero_empty_keeps_size() {
    let mut p = Polynomial::new(vec![]);
    p.set_zero();
    assert_eq!(p.coeffs, Vec::<i64>::new());
}

#[test]
fn test_degree_all_zero_except_last() {
    // Leading-degree convention: coeffs[0] is highest degree.
    // [0,0,0,0,0,0,0,0,0,5] -> degree 0
    let p = Polynomial::new(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 5]);
    assert_eq!(p.degree(), 0);
}

#[test]
fn test_degree_all_zero() {
    let p = Polynomial::new(vec![0; 10]);
    assert_eq!(p.degree(), 0);
}

#[test]
fn test_degree_full() {
    let p = Polynomial::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(p.degree(), 9);
}

#[test]
fn test_degree_some_leading_zeros() {
    let p = Polynomial::new(vec![0, 0, 7, 1, 0]);
    assert_eq!(p.degree(), 2);
}

#[test]
fn test_coef_sum() {
    let p = Polynomial::new(vec![1, 2, 3, 4, 5]);
    assert_eq!(p.coef_sum(), 15);

    let p = Polynomial::new(vec![]);
    assert_eq!(p.coef_sum(), 0);
}

#[test]
fn test_fit_trims_leading_zeros() {
    let mut p = Polynomial::new(vec![0, 0, 12, 7, 9]);
    p.fit(5).unwrap();
    // Expected: [2, 2, 4]
    assert_eq!(p.coeffs, vec![2, 2, 4]);
}

#[test]
fn test_poly_add_same_size_mod10_zeros() {
    let p1 = Polynomial::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
    let p2 = Polynomial::new(vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    let r = p1.add(&p2, 10).unwrap();
    assert_eq!(r.coeffs, vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_poly_add_diff_size() {
    // C test poly_add_test_2: m1 has 9 entries with leading 19; treating size 10
    // means index 0 is "extra". To preserve C semantics, we reproduce the actual
    // test by passing same-size 10 vectors with last position 0.
    let p1 = Polynomial::new(vec![19, 12, 13, 14, 15, 16, 17, 18, 19, 0]);
    let p2 = Polynomial::new(vec![91, 81, 71, 61, 51, 41, 31, 21, 11, 0]);
    let r = p1.add(&p2, 10).unwrap();
    assert_eq!(r.coeffs, vec![0, 3, 4, 5, 6, 7, 8, 9, 0, 0]);
}

#[test]
fn test_poly_add_diff_size_actual() {
    // Test the behavior when sizes are different - the smaller poly is treated
    // as if right-aligned (low-degree terms shared).
    let p1 = Polynomial::new(vec![1, 2, 3, 4]);
    let p2 = Polynomial::new(vec![10, 20]);
    // result size = 4; diff1 = 0, diff2 = 2
    // r[0] = (1 + 0) % 100 = 1
    // r[1] = (2 + 0) % 100 = 2
    // r[2] = (3 + 10) % 100 = 13
    // r[3] = (4 + 20) % 100 = 24
    let r = p1.add(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![1, 2, 13, 24]);
}

#[test]
fn test_poly_sub_same_size() {
    let p1 = Polynomial::new(vec![10, 5, 8, 3, 4]);
    let p2 = Polynomial::new(vec![3, 4, 5, 7, 1]);
    let r = p1.sub(&p2, 7).unwrap();
    // mod 7 gives [0, 1, 3, 5, 3] per C scratch run
    assert_eq!(r.coeffs, vec![0, 1, 3, 5, 3]);
}

#[test]
fn test_poly_mul_mod10() {
    let p1 = Polynomial::new(vec![3456, 20394, 4075, 11783, 31701]);
    let p2 = Polynomial::new(vec![0, 22297, 668, 14130, 14859, 17349, 29965, 1383, 5818, 5889]);
    let r = p1.mul(&p2, 10).unwrap();
    // Expected from C: [2, 6, 7, 5, 1, 9, 0, 1, 5, 0, 2, 5, 9]
    assert_eq!(r.coeffs, vec![2, 6, 7, 5, 1, 9, 0, 1, 5, 0, 2, 5, 9]);
}

#[test]
fn test_poly_lshift_mod10() {
    let p1 = Polynomial::new(vec![27137, 32221, 1765, 27880, 27111, 14656, 290, 21614, 11518, 24573]);
    let p2 = Polynomial::new(vec![1, 28927, 14714, 29356, 3805]);
    let r = p1.lshift(&p2, 10).unwrap();
    // From C scratch: lshift result mod10 [size=9]: 2 7 8 6 6 0 4 8 3
    assert_eq!(r.coeffs, vec![2, 7, 8, 6, 6, 0, 4, 8, 3]);
}

#[test]
fn test_poly_lshift_invalid_leading_coeff() {
    let p1 = Polynomial::new(vec![5, 1, 2]);
    let p2 = Polynomial::new(vec![2, 3, 4]); // leading coeff != 1
    assert!(p1.lshift(&p2, 7).is_err());
}

#[test]
fn test_poly_lshift_dividend_smaller() {
    let p1 = Polynomial::new(vec![1, 2]);
    let p2 = Polynomial::new(vec![1, 3, 4]); // higher degree
    assert!(p1.lshift(&p2, 7).is_err());
}

#[test]
fn test_poly_sub_scaler() {
    let p = Polynomial::new(vec![10, 5, 8, 3, 4]);
    let r = p.sub_scaler(2, 7).unwrap();
    // From C scratch: sub_scaler mod7 by 2: 1 3 6 1 2
    assert_eq!(r.coeffs, vec![1, 3, 6, 1, 2]);
}

#[test]
fn test_poly_add_scaler() {
    let p = Polynomial::new(vec![1, 2, 3, 4, 5]);
    let r = p.add_scaler(5, 7).unwrap();
    // From C scratch: 6 0 1 2 3
    assert_eq!(r.coeffs, vec![6, 0, 1, 2, 3]);
}

#[test]
fn test_poly_mod() {
    let mut p1 = Polynomial::new(vec![1, 0, 0, 5, 3, 2, 1, 4, 7, 9]);
    let p2 = Polynomial::new(vec![1, 2, 3, 4, 5]);
    p1.poly_mod(&p2, 11).unwrap();
    // From C scratch: poly_mod result [size=4]: 5 8 3 7
    assert_eq!(p1.coeffs, vec![5, 8, 3, 7]);
}

#[test]
fn test_polyarray_new() {
    let p1 = Polynomial::new(vec![1, 2]);
    let p2 = Polynomial::new(vec![3, 4]);
    let arr = PolyArray::new(vec![p1.clone(), p2.clone()]);
    assert_eq!(arr.polies.len(), 2);
    assert_eq!(arr.polies[0], p1);
    assert_eq!(arr.polies[1], p2);
}

fn main() {}
