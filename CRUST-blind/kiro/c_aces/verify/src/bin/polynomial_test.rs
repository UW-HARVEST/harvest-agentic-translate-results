use c_aces::polynomial::Polynomial;

#[test]
fn test_degree_all_zeros() {
    let p = Polynomial::new(vec![0; 10]);
    assert_eq!(p.degree(), 0);
}

#[test]
fn test_degree_last_nonzero() {
    let mut c = vec![0i64; 10];
    c[9] = 5;
    let p = Polynomial::new(c);
    assert_eq!(p.degree(), 0);
}

#[test]
fn test_degree_two_nonzero() {
    let mut c = vec![0i64; 10];
    c[8] = 3;
    c[9] = 5;
    let p = Polynomial::new(c);
    assert_eq!(p.degree(), 1);
}

#[test]
fn test_degree_first_nonzero() {
    let mut c = vec![0i64; 10];
    c[0] = 1;
    c[8] = 3;
    c[9] = 5;
    let p = Polynomial::new(c);
    assert_eq!(p.degree(), 9);
}

#[test]
fn test_degree_empty() {
    let p = Polynomial::new(vec![]);
    assert_eq!(p.degree(), 0);
}

#[test]
fn test_coef_sum() {
    let p = Polynomial::new(vec![1, 2, 3, 4, 5]);
    assert_eq!(p.coef_sum(), 15);
}

#[test]
fn test_coef_sum_with_negatives() {
    let p = Polynomial::new(vec![-1, 0, 1]);
    assert_eq!(p.coef_sum(), 0);
}

#[test]
fn test_set_zero() {
    let mut p = Polynomial::new(vec![1, 2, 3, 4, 5]);
    p.set_zero();
    assert_eq!(p.coeffs, vec![0, 0, 0, 0, 0]);
}

#[test]
fn test_add_same_size() {
    let p1 = Polynomial::new(vec![1, 2, 3, 4, 5]);
    let p2 = Polynomial::new(vec![5, 4, 3, 2, 1]);
    let r = p1.add(&p2, 10).unwrap();
    assert_eq!(r.coeffs, vec![6, 6, 6, 6, 6]);
}

#[test]
fn test_add_different_size() {
    let p1 = Polynomial::new(vec![1, 2, 3]);
    let p2 = Polynomial::new(vec![10, 20, 30, 40, 50]);
    let r = p1.add(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![10, 20, 31, 42, 53]);
}

#[test]
fn test_add_large_values() {
    let p1 = Polynomial::new(vec![2023, 6886, 31098, 18163, 31707, 29601, 12607, 8388, 23470, 32705]);
    let p2 = Polynomial::new(vec![16451, 13256, 19216, 1619, 24245, 17402, 17194, 14084, 6779, 28573]);
    let r = p1.add(&p2, 10).unwrap();
    assert_eq!(r.coeffs, vec![4, 2, 4, 2, 2, 3, 1, 2, 9, 8]);
}

#[test]
fn test_add_mod_zero_error() {
    let p1 = Polynomial::new(vec![1]);
    let p2 = Polynomial::new(vec![1]);
    assert!(p1.add(&p2, 0).is_err());
}

#[test]
fn test_sub_same_size() {
    let p1 = Polynomial::new(vec![10, 20, 30, 40, 50]);
    let p2 = Polynomial::new(vec![1, 2, 3, 4, 5]);
    let r = p1.sub(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![9, 18, 27, 36, 45]);
}

#[test]
fn test_sub_different_size() {
    let p1 = Polynomial::new(vec![5, 10, 15]);
    let p2 = Polynomial::new(vec![1, 2, 3, 4, 5]);
    let r = p1.sub(&p2, 100).unwrap();
    // C ground truth: 15 14 2 6 10
    // In C: (0-1)%100 = 99 (unsigned promotion), (0-2)%100 = 98, (5-3)%100 = 2, (10-4)%100 = 6, (15-5)%100 = 10
    // Wait, C output was: 15 14 2 6 10
    // positions: [0]=0-1=-1, [1]=0-2=-2, [2]=5-3=2, [3]=10-4=6, [4]=15-5=10
    // But C gave: 15 14 2 6 10 which means diff is reversed
    // Actually p1 is smaller (3 elems), p2 is larger (5 elems)
    // diff1 = 5-3 = 2, diff2 = 5-5 = 0
    // i=0: (i>=2? no -> 0) - (p2[0]=1) = -1 % 100 = 99 in C (unsigned) ... but C says 15?
    // Wait, the C test was sub([5,10,15]-[1,2,3,4,5]) where p1=[5,10,15] size=3, p2=[1,2,3,4,5] size=5
    // diff1=5-3=2, diff2=5-5=0
    // i=0: (0>=2? no -> 0) - p2[0]=1 = -1. In C: (-1) % (uint64_t)100 = huge_number % 100 = 99
    // But C output was 15! Let me re-read...
    // Oh wait, the C output was: sub([5,10,15]-[1,2,3,4,5] mod100)=15 14 2 6 10
    // Hmm, that doesn't match -1%100=99 either. Let me re-check the C code path.
    // Actually wait - in the C poly_sub for different sizes, the result size is result->size
    // which was set to 5. But the subtraction is p1 - p2.
    // Hmm, I think I misread. Let me re-check the ground truth output.
    // The output was: "sub([5,10,15]-[1,2,3,4,5] mod100)=15 14 2 6 10"
    // Hmm that's weird. Let me reconsider: maybe the output is wrong because I set mr to {0}
    // and the result polynomial was not fully written? No, the C code writes all positions.
    // Actually: diff1=2, diff2=0
    // i=0: a=0, b=p2[0]=1 -> (0-1)%100 = -1 as int64_t, but mod is uint64_t
    // In C: (int64_t)(-1) % (uint64_t)(100) -> -1 converted to uint64_t = 18446744073709551615
    // 18446744073709551615 % 100 = 15. YES! That's 15.
    // i=1: a=0, b=p2[1]=2 -> (0-2) -> -2 as uint64 = 18446744073709551614 % 100 = 14
    // i=2: a=p1[0]=5, b=p2[2]=3 -> (5-3)%100 = 2
    // i=3: a=p1[1]=10, b=p2[3]=4 -> (10-4)%100 = 6
    // i=4: a=p1[2]=15, b=p2[4]=5 -> (15-5)%100 = 10
    // So C gives: [15, 14, 2, 6, 10]
    assert_eq!(r.coeffs, vec![15, 14, 2, 6, 10]);
}

#[test]
fn test_sub_negative_result() {
    // C: (1-4)%10 with uint64 promotion: -3 as u64 % 10 = 3
    let p1 = Polynomial::new(vec![1, 2, 3]);
    let p2 = Polynomial::new(vec![4, 5, 6]);
    let r = p1.sub(&p2, 10).unwrap();
    assert_eq!(r.coeffs, vec![3, 3, 3]);
}

#[test]
fn test_sub_mod_zero_error() {
    let p1 = Polynomial::new(vec![1]);
    let p2 = Polynomial::new(vec![1]);
    assert!(p1.sub(&p2, 0).is_err());
}

#[test]
fn test_mul() {
    let p1 = Polynomial::new(vec![1, 2, 3]);
    let p2 = Polynomial::new(vec![4, 5, 6]);
    let r = p1.mul(&p2, 1000).unwrap();
    assert_eq!(r.coeffs, vec![4, 13, 28, 27, 18]);
}

#[test]
fn test_mul_from_c_test() {
    let p1 = Polynomial::new(vec![3456, 20394, 4075, 11783, 31701]);
    let p2 = Polynomial::new(vec![0, 22297, 668, 14130, 14859, 17349, 29965, 1383, 5818, 5889]);
    let r = p1.mul(&p2, 10).unwrap();
    assert_eq!(r.coeffs, vec![2, 6, 7, 5, 1, 9, 0, 1, 5, 0, 2, 5, 9]);
}

#[test]
fn test_fit_leading_zeros() {
    let mut p = Polynomial::new(vec![0, 0, 0, 5, 10, 15]);
    p.fit(100).unwrap();
    assert_eq!(p.coeffs, vec![5, 10, 15]);
}

#[test]
fn test_fit_all_zeros() {
    let mut p = Polynomial::new(vec![0, 0, 0, 0, 0]);
    p.fit(100).unwrap();
    assert_eq!(p.coeffs, vec![0]);
}

#[test]
fn test_fit_with_modulus() {
    let mut p = Polynomial::new(vec![0, 150, 200, 50]);
    p.fit(100).unwrap();
    assert_eq!(p.coeffs, vec![50, 0, 50]);
}

#[test]
fn test_fit_empty_error() {
    let mut p = Polynomial::new(vec![]);
    assert!(p.fit(100).is_err());
}

#[test]
fn test_lshift() {
    let p1 = Polynomial::new(vec![1, 3, 5, 7]);
    let p2 = Polynomial::new(vec![1, 2, 3]);
    let r = p1.lshift(&p2, 100).unwrap();
    assert_eq!(r.coeffs, vec![1, 2, 7]);
}

#[test]
fn test_lshift_leading_not_one() {
    let p1 = Polynomial::new(vec![1, 3, 5, 7]);
    let p2 = Polynomial::new(vec![2, 2, 3]);
    assert!(p1.lshift(&p2, 100).is_err());
}

#[test]
fn test_lshift_degree_too_small() {
    let p1 = Polynomial::new(vec![0, 5]);
    let p2 = Polynomial::new(vec![1, 2, 3, 4]);
    assert!(p1.lshift(&p2, 100).is_err());
}

#[test]
fn test_poly_mod() {
    let mut p1 = Polynomial::new(vec![1, 0, 3, 2, 1]);
    let p2 = Polynomial::new(vec![1, 1, 1]);
    p1.poly_mod(&p2, 100).unwrap();
    assert_eq!(p1.coeffs, vec![98]);
}

#[test]
fn test_sub_scaler() {
    let p = Polynomial::new(vec![10, 20, 30, 40]);
    let r = p.sub_scaler(5, 100).unwrap();
    assert_eq!(r.coeffs, vec![5, 15, 25, 35]);
}

#[test]
fn test_sub_scaler_underflow() {
    // C: (1-5)%10 with uint64 promotion: -4 as u64 % 10 = 2
    // (2-5)%10: -3 as u64 % 10 = 3
    // (3-5)%10: -2 as u64 % 10 = 4
    let p = Polynomial::new(vec![1, 2, 3]);
    let r = p.sub_scaler(5, 10).unwrap();
    assert_eq!(r.coeffs, vec![2, 3, 4]);
}

#[test]
fn test_add_scaler() {
    let p = Polynomial::new(vec![10, 20, 30, 40]);
    let r = p.add_scaler(5, 100).unwrap();
    assert_eq!(r.coeffs, vec![15, 25, 35, 45]);
}

#[test]
fn test_add_scaler_overflow() {
    // C: (8+5)%10 = 3, (9+5)%10 = 4, (10+5)%10 = 5
    let p = Polynomial::new(vec![8, 9, 10]);
    let r = p.add_scaler(5, 10).unwrap();
    assert_eq!(r.coeffs, vec![3, 4, 5]);
}

fn main() {}
