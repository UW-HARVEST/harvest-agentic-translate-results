use c_aces::aces_internal::{
    generate_error, generate_f0, generate_f1, generate_linear, generate_secret, generate_u,
    generate_vanisher,
};
use c_aces::channel::{Channel, Parameters};
use c_aces::matrix::Matrix3D;
use c_aces::polynomial::{PolyArray, Polynomial};

#[test]
fn test_generate_u_coef_sum_equals_q() {
    // From C test generate_u_test: with channel(2, 33, 1) and dim=10,
    // coef_sum(u) must equal q = 33.
    let channel = Channel::init(2, 33, 1).unwrap();
    let param = Parameters { dim: 10, N: 1 };

    let mut u = Polynomial::new(vec![0i64; 11]);
    generate_u(&channel, &param, &mut u).unwrap();

    assert_eq!(u.coef_sum(), 33);
    // First coefficient should be 1 (set explicitly in generate_u).
    assert_eq!(u.coeffs[0], 1);
    // Length is dim+1 = 11.
    assert_eq!(u.coeffs.len(), 11);
}

#[test]
fn test_generate_u_multiple_runs() {
    let channel = Channel::init(2, 33, 1).unwrap();
    let param = Parameters { dim: 10, N: 1 };

    for _ in 0..10 {
        let mut u = Polynomial::new(vec![0i64; 11]);
        generate_u(&channel, &param, &mut u).unwrap();
        assert_eq!(u.coef_sum(), 33);
        assert_eq!(u.coeffs[0], 1);
    }
}

#[test]
fn test_generate_error_evaluates_to_message() {
    // The sum of coefficients of rm must be congruent to message mod q.
    // The C code's invariant: coef_sum(rm) ≡ message (mod q), since
    // shift = last + (message - csum), and final last = shift (or shift+q).
    let q: u64 = 100;
    for message in [0u64, 1, 5, 50, 99].iter() {
        let mut rm = Polynomial::new(vec![0i64; 5]);
        generate_error(q, *message, &mut rm).unwrap();
        let csum = rm.coef_sum();
        let csum_mod = ((csum % (q as i64)) + q as i64) % q as i64;
        assert_eq!(csum_mod as u64, *message);
        // every coefficient is in [0, q]
        for &c in &rm.coeffs {
            assert!(c >= 0 && c <= q as i64);
        }
    }
}

#[test]
fn test_generate_vanisher_evaluation() {
    // The sum of coefficients must be congruent to p*k (mod q) where k ∈ {0,1}.
    let p: u64 = 3;
    let q: u64 = 100;
    for _ in 0..10 {
        let mut e = Polynomial::new(vec![0i64; 5]);
        generate_vanisher(p, q, &mut e).unwrap();
        let csum = e.coef_sum();
        let csum_mod = ((csum % (q as i64)) + q as i64) % q as i64;
        // Should be either 0 or p
        assert!(csum_mod == 0 || csum_mod == p as i64);
    }
}

#[test]
fn test_generate_linear_in_range() {
    // The sum of coefficients evaluates to k where k ∈ [0, p].
    let p: u64 = 5;
    let q: u64 = 100;
    for _ in 0..10 {
        let mut b = Polynomial::new(vec![0i64; 5]);
        generate_linear(p, q, &mut b).unwrap();
        let csum = b.coef_sum();
        let csum_mod = ((csum % (q as i64)) + q as i64) % q as i64;
        // k is in [0, p]
        assert!(csum_mod >= 0 && csum_mod <= p as i64);
    }
}

#[test]
fn test_generate_f0_coeffs_in_range() {
    // f0 polynomials' coefficients should be in [0, q-1] (i.e. < q).
    let channel = Channel::init(2, 33, 1).unwrap();
    let param = Parameters { dim: 10, N: 1 };
    let mut f0 = PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]);
    generate_f0(&channel, &param, &mut f0).unwrap();

    assert_eq!(f0.polies.len(), 10);
    for poly in f0.polies.iter() {
        assert_eq!(poly.coeffs.len(), 10);
        for &c in poly.coeffs.iter() {
            assert!(c >= 0 && c < channel.q as i64);
        }
    }
}

#[test]
fn test_generate_secret_runs_and_sets_lambda() {
    // Just make sure generate_secret runs to completion and doesn't crash
    // with reasonable parameters.
    let channel = Channel::init(2, 33, 1).unwrap();
    let param = Parameters { dim: 10, N: 1 };

    let mut u = Polynomial::new(vec![0i64; 11]);
    generate_u(&channel, &param, &mut u).unwrap();

    let mut secret = PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]);
    let mut lambda = Matrix3D::new(10, 10);

    generate_secret(&channel, &param, &u, &mut secret, &mut lambda).unwrap();

    assert_eq!(secret.polies.len(), 10);
    for p in secret.polies.iter() {
        assert_eq!(p.coeffs.len(), 10);
    }
    assert_eq!(lambda.data.len(), 10);
    for sub in lambda.data.iter() {
        assert_eq!(sub.dim, 10);
    }
}

#[test]
fn test_generate_f1_runs() {
    let channel = Channel::init(2, 33, 1).unwrap();
    let param = Parameters { dim: 10, N: 1 };

    let mut u = Polynomial::new(vec![0i64; 11]);
    generate_u(&channel, &param, &mut u).unwrap();

    let mut secret = PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]);
    let mut lambda = Matrix3D::new(10, 10);
    generate_secret(&channel, &param, &u, &mut secret, &mut lambda).unwrap();

    let mut f0 = PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]);
    generate_f0(&channel, &param, &mut f0).unwrap();

    let mut f1 = Polynomial::new(vec![0i64; 10]);
    generate_f1(&channel, &param, &f0, &secret, &u, &mut f1).unwrap();
    // f1 should have been written to.
    assert!(!f1.coeffs.is_empty());
}

fn main() {}
