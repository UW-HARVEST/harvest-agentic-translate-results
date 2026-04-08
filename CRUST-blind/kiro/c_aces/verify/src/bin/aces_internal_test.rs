use c_aces::aces_internal::*;
use c_aces::channel::Channel;
use c_aces::channel::Parameters;
use c_aces::polynomial::Polynomial;
use c_aces::matrix::{Matrix2D, Matrix3D, matrix2d_eye, matrix2d_multiply};

#[test]
fn test_generate_u_coef_sum() {
    let channel = Channel::init(2, 33, 1).unwrap();
    let param = Parameters { dim: 10, N: 1 };
    let mut u = Polynomial::new(vec![0i64; 11]);
    generate_u(&channel, &param, &mut u).unwrap();
    // u is constructed so coef_sum(u) == q
    assert_eq!(u.coef_sum(), 33);
    // leading coeff must be 1
    assert_eq!(u.coeffs[0], 1);
}

#[test]
fn test_generate_f0_fills_values() {
    let channel = Channel::init(2, 33, 1).unwrap();
    let param = Parameters { dim: 4, N: 1 };
    let mut f0 = c_aces::polynomial::PolyArray::new(
        (0..4).map(|_| Polynomial::new(vec![0i64; 4])).collect(),
    );
    generate_f0(&channel, &param, &mut f0).unwrap();
    // At least some coefficients should be non-zero (probabilistic but near-certain)
    let total: i64 = f0.polies.iter().flat_map(|p| p.coeffs.iter()).sum();
    assert!(total != 0);
}

#[test]
fn test_generate_secret_produces_valid_lambda() {
    let channel = Channel::init(2, 33, 1).unwrap();
    let param = Parameters { dim: 4, N: 1 };
    let mut u = Polynomial::new(vec![0i64; 5]);
    generate_u(&channel, &param, &mut u).unwrap();

    let mut secret = c_aces::polynomial::PolyArray::new(
        (0..4).map(|_| Polynomial::new(vec![0i64; 4])).collect(),
    );
    let mut lambda = Matrix3D::new(4, 4);
    generate_secret(&channel, &param, &u, &mut secret, &mut lambda).unwrap();

    // secret should have 4 polynomials
    assert_eq!(secret.polies.len(), 4);
    // lambda should have 4 matrices
    assert_eq!(lambda.data.len(), 4);
}

fn main() {}
