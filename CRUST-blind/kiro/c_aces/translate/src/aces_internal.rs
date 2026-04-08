use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
use crate::polynomial::{Coeff, PolyArray, Polynomial};

const EPROB: f64 = 0.5;

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    for i in 0..rm.coeffs.len() {
        rm.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last = rm.coeffs.len() - 1;
    let shift = (rm.coeffs[last] + message as Coeff - rm.coef_sum()) % (q as Coeff);
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
    let shift = (e.coeffs[last] + p as Coeff * k - e.coef_sum()) % (q as Coeff);
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
    let shift = (b.coeffs[last] + k - b.coef_sum()) % (q as Coeff);
    b.coeffs[last] = if shift > 0 { shift } else { shift + q as Coeff };
    Ok(())
}
/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    let mut nonzeros = randrange(param.dim / 2, param.dim - 1);
    let mut zeroes = param.dim - nonzeros;
    u.coeffs[0] = 1;

    let mut i = 1usize;
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
                if i < dim {
                    u.coeffs[i] = 0;
                    i += 1;
                }
            }
            zeroes -= zero;
        }
    }

    u.coeffs[dim] = 0;
    u.coeffs[dim] = channel.q as Coeff - u.coef_sum();
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
        secret.polies[i].coeffs.resize(dim, 0);
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.data[j * dim + i] as Coeff;
        }
    }

    for i in 0..dim {
        // Build arr matrix: for each j, multiply secret[i] * secret[j], mod u, then fill column j
        let mut arr = Matrix2D::new(dim);
        for j in 0..dim {
            let mut a_ij_poly = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            a_ij_poly.poly_mod(u, channel.q)?;

            for row in 0..dim {
                let val = if (dim - row - 1) < a_ij_poly.coeffs.len() {
                    a_ij_poly.coeffs[a_ij_poly.coeffs.len() - 1 - (dim - row - 1)] as u64
                } else {
                    0
                };
                arr.set(row, j, val)?;
            }
        }

        let result = matrix2d_multiply(&invm, &arr, channel.q)?;
        if let Some(lam) = lambda.get_mut(i) {
            lam.data = result.data;
            lam.dim = result.dim;
        }
    }

    Ok(())
}
/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, _param: &Parameters, f0: &mut PolyArray) -> Result<()> {
    for poly in f0.polies.iter_mut() {
        for c in poly.coeffs.iter_mut() {
            *c = randrange(0, channel.q - 1) as Coeff;
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
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        f_pre = f_pre.add(&tmp, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // Copy f_pre into the tail of f1 (matching C's pointer adjustment)
    let f1_len = f1.coeffs.len();
    let f_pre_len = f_pre.coeffs.len();
    if f_pre_len <= f1_len {
        let diff = f1_len - f_pre_len;
        for i in 0..diff {
            f1.coeffs[i] = 0;
        }
        for i in 0..f_pre_len {
            f1.coeffs[diff + i] = f_pre.coeffs[i];
        }
    } else {
        // f_pre is larger, just copy the tail
        let offset = f_pre_len - f1_len;
        for i in 0..f1_len {
            f1.coeffs[i] = f_pre.coeffs[offset + i];
        }
    }

    Ok(())
}
