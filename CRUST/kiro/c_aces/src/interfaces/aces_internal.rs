use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::{AcesError, Result};
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
use crate::polynomial::{Coeff, PolyArray, Polynomial};

const EPROB: f64 = 0.5;

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    for i in 0..rm.coeffs.len() {
        rm.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last = rm.coeffs.len() - 1;
    let shift = (rm.coeffs[last] + message as Coeff - rm.coef_sum()) % q as Coeff;
    rm.coeffs[last] = if shift > 0 { shift } else { shift + q as Coeff };
    Ok(())
}
/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    let k: i64 = if (randrange(0, 1) as f64) < EPROB { 0 } else { 1 };
    for i in 0..e.coeffs.len() {
        e.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last = e.coeffs.len() - 1;
    let shift = (e.coeffs[last] + p as Coeff * k - e.coef_sum()) % q as Coeff;
    e.coeffs[last] = if shift > 0 { shift } else { shift + q as Coeff };
    Ok(())
}
/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    let k = randrange(0, p) as i64;
    for i in 0..b.coeffs.len() {
        b.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last = b.coeffs.len() - 1;
    let shift = (b.coeffs[last] + k - b.coef_sum()) % q as Coeff;
    b.coeffs[last] = if shift > 0 { shift } else { shift + q as Coeff };
    Ok(())
}
/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    let nonzeros_init = randrange(dim as u64 / 2, dim as u64 - 1);
    let mut nonzeros = nonzeros_init;
    let mut zeroes = dim as u64 - nonzeros;
    u.coeffs[0] = 1;
    let mut i = 1;
    while i < dim {
        if nonzeros > 1 {
            u.coeffs[i] = randrange(0, channel.q) as Coeff;
            i += 1;
            nonzeros -= 1;
        } else {
            break;
        }
        if zeroes > 0 {
            let samp = clamp(0, zeroes, normal_rand(zeroes as f64 / 2.0, zeroes as f64 / 2.0) as u64);
            let zero = randrange(0, samp);
            for _ in 0..zero {
                u.coeffs[i] = 0;
                i += 1;
            }
            zeroes -= zero;
        }
    }
    u.coeffs[dim] = 0;
    u.coeffs[dim] = channel.q as i64 - u.coef_sum();
    Ok(())
}
/// Generate the secret key for the arithmetic channel.
pub fn generate_secret(
    channel: &Channel,
    param: &Parameters,
    u: &Polynomial,
    secret: &mut PolyArray,
    lambda: &mut Matrix3D,
) -> Result<()> {
    let dim = param.dim as usize;
    let mut m = Matrix2D::new(dim);
    let mut invm = Matrix2D::new(dim);
    fill_random_invertible_pairs(&mut m, &mut invm, channel.q, 600)?;

    secret.polies.resize(dim, Polynomial::new(vec![0; dim]));
    for i in 0..dim {
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.data[j * dim + i] as Coeff;
        }
    }

    for i in 0..dim {
        let mut arr = Matrix2D::new(dim);
        let mut a_ij_poly = Polynomial::new(vec![0; 2 * dim]);
        for j in 0..dim {
            a_ij_poly = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            a_ij_poly.poly_mod(u, channel.q)?;
            // Pad a_ij_poly to dim if needed
            while a_ij_poly.coeffs.len() < dim {
                a_ij_poly.coeffs.insert(0, 0);
            }
            for row in 0..dim {
                arr.data[row * dim + j] = a_ij_poly.coeffs[dim - row - 1] as u64;
            }
        }
        let result = matrix2d_multiply(&invm, &arr, channel.q)?;
        lambda.data[i] = result;
    }
    Ok(())
}
/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, param: &Parameters, f0: &mut PolyArray) -> Result<()> {
    for i in 0..f0.polies.len() {
        for j in 0..f0.polies[i].coeffs.len() {
            f0.polies[i].coeffs[j] = randrange(0, channel.q - 1) as Coeff;
        }
    }
    Ok(())
}
/// Generate the polynomial `f1` for the public key.
pub fn generate_f1(
    channel: &Channel,
    param: &Parameters,
    f0: &PolyArray,
    x: &PolyArray,
    u: &Polynomial,
    f1: &mut Polynomial,
) -> Result<()> {
    let dim = param.dim as usize;
    let mut f_pre = Polynomial::new(vec![0; dim * 2]);
    for i in 0..dim {
        let mut tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        f_pre = f_pre.add(&tmp, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;
    // Copy f_pre into the tail of f1
    let diff = f1.coeffs.len().saturating_sub(f_pre.coeffs.len());
    for i in 0..f_pre.coeffs.len() {
        if diff + i < f1.coeffs.len() {
            f1.coeffs[diff + i] = f_pre.coeffs[i];
        }
    }
    Ok(())
}
