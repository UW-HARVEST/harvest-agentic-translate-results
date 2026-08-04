#![allow(unused_imports)]
use c_aces::aces_internal::{
    generate_error, generate_f0, generate_f1, generate_linear, generate_secret, generate_u,
    generate_vanisher,
};
use c_aces::channel::{Channel, Parameters};
use c_aces::matrix::{matrix2d_multiply, Matrix2D, Matrix3D};
use c_aces::polynomial::{PolyArray, Polynomial};

// generate_u: per C test, after calling generate_u(channel{p=2,q=33,w=1}, dim=10),
// coef_sum(u) == 33 (= q).
#[test]
fn test_generate_u_coef_sum_equals_q() {
    let channel = Channel::new(2, 33, 1);
    let param = Parameters { dim: 10, N: 1 };
    for _ in 0..10 {
        let mut u = Polynomial::new(vec![0i64; 11]);
        generate_u(&channel, &param, &mut u).unwrap();
        assert_eq!(u.coef_sum(), 33);
    }
}

#[test]
fn test_generate_u_other_q() {
    let channel = Channel::new(2, 33, 1);
    let param = Parameters { dim: 8, N: 1 };
    for _ in 0..5 {
        let mut u = Polynomial::new(vec![0i64; 9]);
        generate_u(&channel, &param, &mut u).unwrap();
        assert_eq!(u.coef_sum(), channel.q as i64);
        // First coefficient should be 1 (per C: u->coeffs[0] = 1 in main loop).
        assert_eq!(u.coeffs[0], 1);
    }
}

#[test]
fn test_generate_u_dim_one() {
    // dim=1: u has size 2; u[0]=1, u[1]=q-1, sum should be q.
    let channel = Channel::new(2, 33, 1);
    let param = Parameters { dim: 1, N: 1 };
    let mut u = Polynomial::new(vec![0i64; 2]);
    generate_u(&channel, &param, &mut u).unwrap();
    assert_eq!(u.coeffs[0], 1);
    assert_eq!(u.coef_sum(), 33);
}

// generate_error: rm.coeffs sum mod q should equal message mod q.
#[test]
fn test_generate_error_coef_sum_matches_message() {
    let q: u64 = 100;
    for msg in [0u64, 1, 5, 50, 99].iter() {
        for _ in 0..5 {
            let mut rm = Polynomial::new(vec![0i64; 6]);
            generate_error(q, *msg, &mut rm).unwrap();
            // sum mod q should equal message
            let s = rm.coef_sum();
            let smod = ((s % q as i64) + q as i64) % q as i64;
            assert_eq!(smod as u64, *msg);
        }
    }
}

#[test]
fn test_generate_error_size_zero_no_panic() {
    let mut rm = Polynomial::new(vec![]);
    assert!(generate_error(100, 0, &mut rm).is_ok());
}

// generate_vanisher: produces e such that coef_sum(e) mod q is either 0 or p.
#[test]
fn test_generate_vanisher_coef_sum() {
    let p: u64 = 5;
    let q: u64 = 100;
    for _ in 0..30 {
        let mut e = Polynomial::new(vec![0i64; 8]);
        generate_vanisher(p, q, &mut e).unwrap();
        let s = e.coef_sum();
        let smod = ((s % q as i64) + q as i64) % q as i64;
        assert!(
            smod as u64 == 0 || smod as u64 == p,
            "vanisher coef_sum mod q ({}) is not 0 or p ({})",
            smod,
            p
        );
    }
}

// generate_linear: produces b such that coef_sum(b) mod q is in [0, p].
#[test]
fn test_generate_linear_coef_sum() {
    let p: u64 = 5;
    let q: u64 = 100;
    for _ in 0..50 {
        let mut b = Polynomial::new(vec![0i64; 8]);
        generate_linear(p, q, &mut b).unwrap();
        let s = b.coef_sum();
        let smod = ((s % q as i64) + q as i64) % q as i64;
        assert!(smod as u64 <= p);
    }
}

// generate_f0: fills polynomials with values in [0, q-1].
#[test]
fn test_generate_f0_in_range() {
    let channel = Channel::new(2, 33, 1);
    let param = Parameters { dim: 10, N: 1 };
    let mut f0 = PolyArray::new(
        (0..10)
            .map(|_| Polynomial::new(vec![0i64; 10]))
            .collect(),
    );
    generate_f0(&channel, &param, &mut f0).unwrap();
    for poly in &f0.polies {
        for &c in &poly.coeffs {
            assert!(c >= 0 && (c as u64) < channel.q);
        }
    }
}

// generate_secret: fills secret and lambda; runs without panicking on dim=10.
#[test]
fn test_generate_secret_runs() {
    let channel = Channel::new(2, 33, 1);
    let param = Parameters { dim: 10, N: 1 };

    let mut u = Polynomial::new(vec![0i64; 11]);
    generate_u(&channel, &param, &mut u).unwrap();

    let mut secret = PolyArray::new(
        (0..10)
            .map(|_| Polynomial::new(vec![0i64; 10]))
            .collect(),
    );
    let mut lambda = Matrix3D::new(10, 10);

    generate_secret(&channel, &param, &u, &mut secret, &mut lambda).unwrap();

    assert_eq!(secret.polies.len(), 10);
    for p in &secret.polies {
        assert_eq!(p.coeffs.len(), 10);
    }
    assert_eq!(lambda.data.len(), 10);
    for m in &lambda.data {
        assert_eq!(m.dim, 10);
        assert_eq!(m.data.len(), 100);
    }
}

#[test]
fn test_generate_secret_smaller_dim() {
    let channel = Channel::new(2, 33, 1);
    let param = Parameters { dim: 4, N: 1 };

    let mut u = Polynomial::new(vec![0i64; 5]);
    generate_u(&channel, &param, &mut u).unwrap();

    let mut secret = PolyArray::new(
        (0..4)
            .map(|_| Polynomial::new(vec![0i64; 4]))
            .collect(),
    );
    let mut lambda = Matrix3D::new(4, 4);

    generate_secret(&channel, &param, &u, &mut secret, &mut lambda).unwrap();
    assert_eq!(secret.polies.len(), 4);
    assert_eq!(lambda.data.len(), 4);
}

// generate_f1: doesn't panic and returns Ok.
#[test]
fn test_generate_f1_runs() {
    let channel = Channel::new(2, 33, 1);
    let param = Parameters { dim: 4, N: 1 };

    let mut u = Polynomial::new(vec![0i64; 5]);
    generate_u(&channel, &param, &mut u).unwrap();

    let mut secret = PolyArray::new(
        (0..4)
            .map(|_| Polynomial::new(vec![0i64; 4]))
            .collect(),
    );
    let mut lambda = Matrix3D::new(4, 4);
    generate_secret(&channel, &param, &u, &mut secret, &mut lambda).unwrap();

    let mut f0 = PolyArray::new(
        (0..4)
            .map(|_| Polynomial::new(vec![0i64; 4]))
            .collect(),
    );
    generate_f0(&channel, &param, &mut f0).unwrap();

    let mut f1 = Polynomial::new(vec![0i64; 4]);
    generate_f1(&channel, &param, &f0, &secret, &u, &mut f1).unwrap();

    // f1 coefficients should be within [0, q-1]
    for &c in &f1.coeffs {
        assert!(c >= 0 && (c as u64) < channel.q);
    }
}

fn main() {}
