use c_aces::polynomial::Polynomial;

#[test]
fn test_degree() {
    let p = Polynomial::new(vec![0, 0, 0, 0, 5]);
    assert_eq!(p.degree(), 0);

    let p = Polynomial::new(vec![0, 0, 0, 0, 0]);
    assert_eq!(p.degree(), 0);

    let p = Polynomial::new(vec![1, 0, 0, 0, 0]);
    assert_eq!(p.degree(), 4);

    let p = Polynomial::new(vec![0, 0, 5, 0, 0]);
    assert_eq!(p.degree(), 2);

    let p = Polynomial::new(vec![]);
    assert_eq!(p.degree(), 0);
}

#[test]
fn test_set_zero() {
    let mut p = Polynomial::new(vec![1, 2, 3, 4, 5]);
    p.set_zero();
    assert_eq!(p.coeffs, vec![0, 0, 0, 0, 0]);
}

#[test]
fn test_coef_sum() {
    let p = Polynomial::new(vec![1, 2, 3, 4, 5]);
    assert_eq!(p.coef_sum(), 15);

    let p = Polynomial::new(vec![1, -2, 3, -4]);
    assert_eq!(p.coef_sum(), -2);

    let p = Polynomial::new(vec![]);
    assert_eq!(p.coef_sum(), 0);
}

#[test]
fn test_fit() {
    let mut p = Polynomial::new(vec![0, 0, 3, 5, 7]);
    p.fit(10).unwrap();
    assert_eq!(p.coeffs, vec![3, 5, 7]);

    let mut p = Polynomial::new(vec![0, 0, 0, 0, 0]);
    p.fit(10).unwrap();
    assert_eq!(p.coeffs, vec![0]);
}

#[test]
fn test_fit_with_negative() {
    // C: poly_fit [0,0,-3,5,-7] mod 10 -> [3,5,9] (unsigned promotion)
    let mut p = Polynomial::new(vec![0, 0, -3, 5, -7]);
    p.fit(10).unwrap();
    assert_eq!(p.coeffs, vec![3, 5, 9]);
}

#[test]
fn test_poly_equal() {
    let a = Polynomial::new(vec![1, 2, 3]);
    let b = Polynomial::new(vec![1, 2, 3]);
    let c = Polynomial::new(vec![1, 2, 4]);
    let d = Polynomial::new(vec![2, 3]);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
}

#[test]
fn test_add_same_size() {
    // [1,2,3,4,5,6,7,8,9,0] + [9,8,7,6,5,4,3,2,1,0] mod 10 = [0,0,0,0,0,0,0,0,0,0]
    let a = Polynomial::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
    let b = Polynomial::new(vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    let r = a.add(&b, 10).unwrap();
    assert_eq!(r.coeffs, vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_add_same_size_2() {
    // [19,12,13,14,15,16,17,18,19,0] + [91,81,71,61,51,41,31,21,11,0] mod 10
    let a = Polynomial::new(vec![19, 12, 13, 14, 15, 16, 17, 18, 19, 0]);
    let b = Polynomial::new(vec![91, 81, 71, 61, 51, 41, 31, 21, 11, 0]);
    let r = a.add(&b, 10).unwrap();
    assert_eq!(r.coeffs, vec![0, 3, 4, 5, 6, 7, 8, 9, 0, 0]);
}

#[test]
fn test_add_same_size_3() {
    let a = Polynomial::new(vec![2023, 6886, 31098, 18163, 31707, 29601, 12607, 8388, 23470, 32705]);
    let b = Polynomial::new(vec![16451, 13256, 19216, 1619, 24245, 17402, 17194, 14084, 6779, 28573]);
    let r = a.add(&b, 10).unwrap();
    assert_eq!(r.coeffs, vec![4, 2, 4, 2, 2, 3, 1, 2, 9, 8]);
}

#[test]
fn test_add_different_sizes() {
    // [1,2,3] + [10,20,30,40,50] mod 100 = [10,20,31,42,53]
    let a = Polynomial::new(vec![1, 2, 3]);
    let b = Polynomial::new(vec![10, 20, 30, 40, 50]);
    let r = a.add(&b, 100).unwrap();
    assert_eq!(r.coeffs, vec![10, 20, 31, 42, 53]);
}

#[test]
fn test_add_zero_modulus() {
    let a = Polynomial::new(vec![1]);
    let b = Polynomial::new(vec![1]);
    assert!(a.add(&b, 0).is_err());
}

#[test]
fn test_sub_same_size() {
    // C: [5,3,1] - [1,2,3] mod 10 = [4,1,4] (unsigned promotion: (5-1)%10=4, (3-2)%10=1, (1-3)%10u=4)
    let a = Polynomial::new(vec![5, 3, 1]);
    let b = Polynomial::new(vec![1, 2, 3]);
    let r = a.sub(&b, 10).unwrap();
    assert_eq!(r.coeffs, vec![4, 1, 4]);
}

#[test]
fn test_sub_same_size_negative() {
    // C: [1,2,3] - [5,3,1] mod 10 = [2,5,2] (unsigned promotion)
    let a = Polynomial::new(vec![1, 2, 3]);
    let b = Polynomial::new(vec![5, 3, 1]);
    let r = a.sub(&b, 10).unwrap();
    assert_eq!(r.coeffs, vec![2, 5, 2]);
}

#[test]
fn test_sub_different_sizes() {
    // C: [1,2,3] - [10,20,30,40,50] mod 100 = [6,96,87,78,69]
    // (0-10)%100u=6, (0-20)%100u=96, (1-30)%100u=87, (2-40)%100u=78, (3-50)%100u=69
    let a = Polynomial::new(vec![1, 2, 3]);
    let b = Polynomial::new(vec![10, 20, 30, 40, 50]);
    let r = a.sub(&b, 100).unwrap();
    assert_eq!(r.coeffs, vec![6, 96, 87, 78, 69]);
}

#[test]
fn test_sub_zero_modulus() {
    let a = Polynomial::new(vec![1]);
    let b = Polynomial::new(vec![1]);
    assert!(a.sub(&b, 0).is_err());
}

#[test]
fn test_mul() {
    // C ground truth
    let a = Polynomial::new(vec![3456, 20394, 4075, 11783, 31701]);
    let b = Polynomial::new(vec![0, 22297, 668, 14130, 14859, 17349, 29965, 1383, 5818, 5889]);
    let r = a.mul(&b, 10).unwrap();
    assert_eq!(r.coeffs, vec![2, 6, 7, 5, 1, 9, 0, 1, 5, 0, 2, 5, 9]);
}

#[test]
fn test_lshift() {
    // C: poly_lshift [1,2,3,4] by [1,1,1] mod 10 = [1,2,4] (size 3)
    let a = Polynomial::new(vec![1, 2, 3, 4]);
    let b = Polynomial::new(vec![1, 1, 1]);
    let r = a.lshift(&b, 10).unwrap();
    assert_eq!(r.coeffs, vec![1, 2, 4]);
}

#[test]
fn test_lshift_leading_not_one() {
    let a = Polynomial::new(vec![1, 2, 3, 4]);
    let b = Polynomial::new(vec![2, 1, 1]);
    assert!(a.lshift(&b, 10).is_err());
}

#[test]
fn test_lshift_degree_too_small() {
    let a = Polynomial::new(vec![0, 0, 3]);
    let b = Polynomial::new(vec![1, 1, 1]);
    assert!(a.lshift(&b, 10).is_err());
}

#[test]
fn test_poly_mod() {
    // C: poly_mod [1,2,3,4] by [1,1,1] mod 10 = [1,3] (size 2)
    let mut a = Polynomial::new(vec![1, 2, 3, 4]);
    let b = Polynomial::new(vec![1, 1, 1]);
    a.poly_mod(&b, 10).unwrap();
    assert_eq!(a.coeffs, vec![1, 3]);
}

#[test]
fn test_sub_scaler() {
    // C: [5,10,15] - 3 mod 10 = [2,7,2]
    let p = Polynomial::new(vec![5, 10, 15]);
    let r = p.sub_scaler(3, 10).unwrap();
    assert_eq!(r.coeffs, vec![2, 7, 2]);
}

#[test]
fn test_sub_scaler_negative() {
    // C: [2,1,0] - 5 mod 10 = [3,2,1] (unsigned promotion)
    let p = Polynomial::new(vec![2, 1, 0]);
    let r = p.sub_scaler(5, 10).unwrap();
    assert_eq!(r.coeffs, vec![3, 2, 1]);
}

#[test]
fn test_add_scaler() {
    // C: [5,10,15] + 3 mod 10 = [8,3,8]
    let p = Polynomial::new(vec![5, 10, 15]);
    let r = p.add_scaler(3, 10).unwrap();
    assert_eq!(r.coeffs, vec![8, 3, 8]);
}

#[test]
fn test_add_scaler_overflow() {
    // C: [8,9,7] + 5 mod 10 = [3,4,2]
    let p = Polynomial::new(vec![8, 9, 7]);
    let r = p.add_scaler(5, 10).unwrap();
    assert_eq!(r.coeffs, vec![3, 4, 2]);
}

fn main() {}
