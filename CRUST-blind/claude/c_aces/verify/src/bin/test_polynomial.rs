#![allow(unused_imports)]
use c_aces::polynomial::{Coeff, PolyArray, Polynomial};

#[test]
fn test_poly_new() {
    let p = Polynomial::new(vec![1, 2, 3]);
    assert_eq!(p.coeffs, vec![1i64, 2, 3]);
}

#[test]
fn test_set_zero() {
    let mut p = Polynomial::new(vec![1, 2, 3, 4, 5]);
    p.set_zero();
    assert_eq!(p.coeffs, vec![0i64; 5]);
}

#[test]
fn test_degree_basic() {
    // C: degree of [0,0,0,0,0] = 0
    let p = Polynomial::new(vec![0, 0, 0, 0, 0]);
    assert_eq!(p.degree(), 0);

    // [0,0,0,0,5] -> only constant nonzero, degree 0
    let p = Polynomial::new(vec![0, 0, 0, 0, 5]);
    assert_eq!(p.degree(), 0);

    // [0,1,2,3,4] -> highest nonzero at index 1, degree = size - i - 1 = 5 - 1 - 1 = 3
    let p = Polynomial::new(vec![0, 1, 2, 3, 4]);
    assert_eq!(p.degree(), 3);

    // [3,0,0,0,0] -> first nonzero at index 0, degree = 5 - 0 - 1 = 4
    let p = Polynomial::new(vec![3, 0, 0, 0, 0]);
    assert_eq!(p.degree(), 4);
}

#[test]
fn test_coef_sum_basic() {
    let p = Polynomial::new(vec![1, 2, 3, 4]);
    assert_eq!(p.coef_sum(), 10);

    let p = Polynomial::new(vec![-1, -2, -3]);
    assert_eq!(p.coef_sum(), -6);

    let p = Polynomial::new(vec![]);
    assert_eq!(p.coef_sum(), 0);
}

#[test]
fn test_fit_with_leading_zeros() {
    // C: fit([0,0,7,12,15],5) -> [2, 2, 0]
    let mut p = Polynomial::new(vec![0, 0, 7, 12, 15]);
    p.fit(5).unwrap();
    assert_eq!(p.coeffs, vec![2i64, 2, 0]);
}

#[test]
fn test_fit_constant_only() {
    // C: fit([0,0,0,0,9],5) -> [4]
    let mut p = Polynomial::new(vec![0, 0, 0, 0, 9]);
    p.fit(5).unwrap();
    assert_eq!(p.coeffs, vec![4i64]);
}

#[test]
fn test_fit_all_zero() {
    // All zero -> degree=0, idx=size-1, retains last element only
    let mut p = Polynomial::new(vec![0, 0, 0, 0]);
    p.fit(5).unwrap();
    assert_eq!(p.coeffs, vec![0i64]);
}

#[test]
fn test_poly_add_same_size() {
    // C: add([1,2,3],[4,5,6],100) -> [5,7,9]
    let p1 = Polynomial::new(vec![1, 2, 3]);
    let p2 = Polynomial::new(vec![4, 5, 6]);
    let r = p1.add(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![5i64, 7, 9]);
}

#[test]
fn test_poly_add_diff_size() {
    // C: add([1,2,3,4,5],[10,20,30],100) -> [1, 2, 13, 24, 35]
    let p1 = Polynomial::new(vec![1, 2, 3, 4, 5]);
    let p2 = Polynomial::new(vec![10, 20, 30]);
    let r = p1.add(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![1i64, 2, 13, 24, 35]);
}

#[test]
fn test_poly_add_diff_size_reverse() {
    // C: add([1,2],[3,4,5],100) -> [3, 5, 7]
    let p1 = Polynomial::new(vec![1, 2]);
    let p2 = Polynomial::new(vec![3, 4, 5]);
    let r = p1.add(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![3i64, 5, 7]);
}

#[test]
fn test_poly_add_with_mod_wrapping() {
    // C: add_test_2: poly1=[19, 12, 13, ..., 19] poly2=[91,81,...,11] mod 10
    // Expected: [0, 3, 4, 5, 6, 7, 8, 9, 0]
    let p1 = Polynomial::new(vec![19, 12, 13, 14, 15, 16, 17, 18, 19]);
    let p2 = Polynomial::new(vec![91, 81, 71, 61, 51, 41, 31, 21, 11]);
    let r = p1.add(&p2, 10).unwrap();
    assert_eq!(r.coeffs, vec![0i64, 3, 4, 5, 6, 7, 8, 9, 0]);
}

#[test]
fn test_poly_sub_same_size() {
    // C: sub([10,20,30],[1,2,3],100) -> [9,18,27]
    let p1 = Polynomial::new(vec![10, 20, 30]);
    let p2 = Polynomial::new(vec![1, 2, 3]);
    let r = p1.sub(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![9i64, 18, 27]);
}

#[test]
fn test_poly_sub_equal() {
    // C: sub equal -> [0,0,0]
    let p1 = Polynomial::new(vec![3, 4, 5]);
    let p2 = Polynomial::new(vec![3, 4, 5]);
    let r = p1.sub(&p2, 7).unwrap();
    assert_eq!(r.coeffs, vec![0i64, 0, 0]);
}

#[test]
fn test_poly_sub_diff_size() {
    // C: sub([5,5,5,5,5],[1,2,3],7) -> [5, 5, 4, 3, 2]
    let p1 = Polynomial::new(vec![5, 5, 5, 5, 5]);
    let p2 = Polynomial::new(vec![1, 2, 3]);
    let r = p1.sub(&p2, 7).unwrap();
    assert_eq!(r.coeffs, vec![5i64, 5, 4, 3, 2]);
}

#[test]
fn test_poly_mul_basic() {
    // C: mul([1,1]*[1,2],100) = [1, 3, 2]
    let p1 = Polynomial::new(vec![1, 1]);
    let p2 = Polynomial::new(vec![1, 2]);
    let r = p1.mul(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![1i64, 3, 2]);
}

#[test]
fn test_poly_mul_with_zero_coeffs() {
    // C: mul([2,0,1]*[1,1],100) = [2, 2, 1, 1]
    let p1 = Polynomial::new(vec![2, 0, 1]);
    let p2 = Polynomial::new(vec![1, 1]);
    let r = p1.mul(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![2i64, 2, 1, 1]);
}

#[test]
fn test_poly_mul_simple() {
    // C: mul([1,2,3],[4,5,6],100) -> [4,13,28,27,18]
    let p1 = Polynomial::new(vec![1, 2, 3]);
    let p2 = Polynomial::new(vec![4, 5, 6]);
    let r = p1.mul(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![4i64, 13, 28, 27, 18]);
}

#[test]
fn test_poly_lshift() {
    // C: lshift([1,2,3,4],[1,5],100)=0 result=[97, 3, 4]
    let p1 = Polynomial::new(vec![1, 2, 3, 4]);
    let p2 = Polynomial::new(vec![1, 5]);
    let r = p1.lshift(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![97i64, 3, 4]);
}

#[test]
fn test_poly_lshift_simpler() {
    // C: lshift([2,3,4],[1,1],100) = [1, 4]
    let p1 = Polynomial::new(vec![2, 3, 4]);
    let p2 = Polynomial::new(vec![1, 1]);
    let r = p1.lshift(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![1i64, 4]);
}

#[test]
fn test_poly_lshift_non_monic_returns_error() {
    let p1 = Polynomial::new(vec![1, 2, 3]);
    let p2 = Polynomial::new(vec![2, 1]);
    let r = p1.lshift(&p2, 100);
    assert!(r.is_err());
}

#[test]
fn test_poly_mod_basic() {
    // C: mod([1,2,3,4],[1,5],100) -> [14]
    let mut p1 = Polynomial::new(vec![1, 2, 3, 4]);
    let p2 = Polynomial::new(vec![1, 5]);
    p1.poly_mod(&p2, 100).unwrap();
    assert_eq!(p1.coeffs, vec![14i64]);
}

#[test]
fn test_poly_mod_simpler() {
    // C: mod([1,0,5] mod [1,1] in 100) = [6]
    let mut p1 = Polynomial::new(vec![1, 0, 5]);
    let p2 = Polynomial::new(vec![1, 1]);
    p1.poly_mod(&p2, 100).unwrap();
    assert_eq!(p1.coeffs, vec![6i64]);
}

#[test]
fn test_poly_sub_scaler() {
    // C: sub_scaler([10,20,30], 5, 100) -> [5, 15, 25]
    let p = Polynomial::new(vec![10, 20, 30]);
    let r = p.sub_scaler(5, 100).unwrap();
    assert_eq!(r.coeffs, vec![5i64, 15, 25]);
}

#[test]
fn test_poly_sub_scaler_underflow_positive() {
    // Positive results case: (10-3)%7=0, (20-3)%7=3, (30-3)%7=6
    let p = Polynomial::new(vec![10, 20, 30]);
    let r = p.sub_scaler(3, 7).unwrap();
    assert_eq!(r.coeffs, vec![0i64, 3, 6]);
}

#[test]
fn test_poly_add_scaler() {
    // C: add_scaler([10,20,30], 5, 100) -> [15, 25, 35]
    let p = Polynomial::new(vec![10, 20, 30]);
    let r = p.add_scaler(5, 100).unwrap();
    assert_eq!(r.coeffs, vec![15i64, 25, 35]);
}

#[test]
fn test_poly_eq() {
    let p1 = Polynomial::new(vec![1, 2, 3]);
    let p2 = Polynomial::new(vec![1, 2, 3]);
    assert_eq!(p1, p2);
    let p3 = Polynomial::new(vec![1, 2]);
    assert_ne!(p1, p3);
}

#[test]
fn test_polyarray_new() {
    let polies = vec![Polynomial::new(vec![1, 2]), Polynomial::new(vec![3, 4])];
    let pa = PolyArray::new(polies.clone());
    assert_eq!(pa.polies.len(), 2);
    assert_eq!(pa.polies[0].coeffs, vec![1i64, 2]);
    assert_eq!(pa.polies[1].coeffs, vec![3i64, 4]);
}

#[test]
fn test_zero_modulus_errors() {
    let p1 = Polynomial::new(vec![1, 2, 3]);
    let p2 = Polynomial::new(vec![4, 5, 6]);
    assert!(p1.add(&p2, 0).is_err());
    assert!(p1.sub(&p2, 0).is_err());
    assert!(p1.mul(&p2, 0).is_err());
    assert!(p1.lshift(&p2, 0).is_err());
    assert!(p1.add_scaler(1, 0).is_err());
    assert!(p1.sub_scaler(1, 0).is_err());
}

fn main() {}
