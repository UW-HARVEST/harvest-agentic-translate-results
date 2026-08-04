use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::{AcesError, Result};
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
use crate::polynomial::{PolyArray, Polynomial};

const EPROB: f64 = 0.5;

fn generic_error(message: impl Into<String>) -> AcesError {
    AcesError::GenericError(message.into())
}

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    for coeff in &mut rm.coeffs {
        *coeff = randrange(0, q) as i64;
    }

    let last = rm.coeffs.len() - 1;
    let shift = (rm.coeffs[last] + message as i64 - rm.coef_sum()) % q as i64;
    rm.coeffs[last] = if shift > 0 { shift } else { shift + q as i64 };
    Ok(())
}
/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    let k = if (randrange(0, 1) as f64) < EPROB { 0 } else { 1 };
    for coeff in &mut e.coeffs {
        *coeff = randrange(0, q) as i64;
    }

    let last = e.coeffs.len() - 1;
    let shift = (e.coeffs[last] + (p as i64 * k) - e.coef_sum()) % q as i64;
    e.coeffs[last] = if shift > 0 { shift } else { shift + q as i64 };
    Ok(())
}
/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    let k = randrange(0, p) as i64;
    for coeff in &mut b.coeffs {
        *coeff = randrange(0, q) as i64;
    }

    let last = b.coeffs.len() - 1;
    let shift = (b.coeffs[last] + k - b.coef_sum()) % q as i64;
    b.coeffs[last] = if shift > 0 { shift } else { shift + q as i64 };
    Ok(())
}
/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    if u.coeffs.len() <= dim {
        return Err(generic_error("polynomial storage too small"));
    }

    u.set_zero();

    let mut nonzeros = randrange(param.dim / 2, param.dim - 1);
    let mut zeroes = param.dim - nonzeros;
    u.coeffs[0] = 1;

    let mut i = 1_usize;
    while i < dim {
        if nonzeros > 1 {
            u.coeffs[i] = randrange(0, channel.q) as i64;
            i += 1;
            nonzeros -= 1;
        } else {
            break;
        }

        if zeroes > 0 {
            let sample = clamp(
                0,
                zeroes,
                normal_rand(zeroes as f64 / 2.0, zeroes as f64 / 2.0) as u64,
            );
            let zero = randrange(0, sample);
            for _ in 0..zero {
                if i >= dim {
                    break;
                }
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

    for i in 0..dim {
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m
                .get(j, i)
                .ok_or_else(|| generic_error("matrix index out of bounds"))? as i64;
        }
    }

    for i in 0..dim {
        for j in 0..dim {
            let mut a_ij_poly = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            a_ij_poly.poly_mod(u, channel.q)?;

            for row in 0..dim {
                let value = a_ij_poly
                    .coeffs
                    .get(dim - row - 1)
                    .copied()
                    .unwrap_or(0) as u64;
                m.set(row, j, value)?;
            }
        }

        let product = matrix2d_multiply(&invm, &m, channel.q)?;
        *lambda
            .get_mut(i)
            .ok_or_else(|| generic_error("matrix index out of bounds"))? = product;
    }

    Ok(())
}
/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, param: &Parameters, f0: &mut PolyArray) -> Result<()> {
    let _ = param;
    for poly in &mut f0.polies {
        for coeff in &mut poly.coeffs {
            *coeff = randrange(0, channel.q - 1) as i64;
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
    f_pre.set_zero();

    for i in 0..dim {
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        f_pre = f_pre.add(&tmp, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    f1.coeffs = f_pre.coeffs;
    Ok(())
}
