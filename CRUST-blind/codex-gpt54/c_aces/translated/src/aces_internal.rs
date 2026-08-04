use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::{AcesError, Result};
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
use crate::polynomial::{PolyArray, Polynomial};

const EPROB: f64 = 0.5;

fn err(msg: &str) -> AcesError {
    AcesError::GenericError(msg.to_string())
}

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    if rm.coeffs.is_empty() || q == 0 {
        return Err(err("invalid polynomial or modulus"));
    }

    for coeff in &mut rm.coeffs {
        *coeff = randrange(0, q) as i64;
    }

    let last = rm.coeffs.len() - 1;
    let shift = (rm.coeffs[last] + (message as i64 - rm.coef_sum())) % q as i64;
    rm.coeffs[last] = if shift > 0 { shift } else { shift + q as i64 };
    Ok(())
}
/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    if e.coeffs.is_empty() || q == 0 {
        return Err(err("invalid polynomial or modulus"));
    }

    let k = if (randrange(0, 1) as f64) < EPROB {
        0
    } else {
        1
    };
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
    if b.coeffs.is_empty() || q == 0 {
        return Err(err("invalid polynomial or modulus"));
    }

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
    if u.coeffs.len() <= dim || dim == 0 {
        return Err(err("u polynomial has invalid size"));
    }

    u.set_zero();
    let mut nonzeros = randrange(param.dim / 2, param.dim - 1) as usize;
    let mut zeroes = dim - nonzeros;
    u.coeffs[0] = 1;

    let mut i = 1usize;
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
                zeroes as u64,
                normal_rand((zeroes / 2) as f64, (zeroes / 2) as f64) as u64,
            ) as usize;
            let zero = randrange(0, sample as u64) as usize;
            for _ in 0..zero {
                if i >= dim {
                    break;
                }
                u.coeffs[i] = 0;
                i += 1;
            }
            zeroes -= zero.min(zeroes);
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
    if secret.polies.len() != dim || lambda.data.len() != dim {
        return Err(err("secret or lambda dimensions do not match"));
    }

    let mut m = Matrix2D::new(dim);
    let mut invm = Matrix2D::new(dim);
    fill_random_invertible_pairs(&mut m, &mut invm, channel.q, 600)?;

    for i in 0..dim {
        if secret.polies[i].coeffs.len() < dim {
            return Err(err("secret polynomial has invalid size"));
        }
        for j in 0..dim {
            secret.polies[i].coeffs[j] =
                m.get(j, i)
                    .ok_or_else(|| err("matrix index out of bounds"))? as i64;
        }
    }

    let mut arr = m.clone();
    for i in 0..dim {
        for j in 0..dim {
            let mut a_ij_poly = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            a_ij_poly.poly_mod(u, channel.q)?;

            let pad = dim.saturating_sub(a_ij_poly.coeffs.len());
            for row in 0..dim {
                let idx = dim - row - 1;
                let coeff = if idx < pad {
                    0
                } else {
                    a_ij_poly.coeffs[idx - pad]
                };
                arr.set(row, j, coeff as u64)?;
            }
        }

        let product = matrix2d_multiply(&invm, &arr, channel.q)?;
        let target = lambda
            .get_mut(i)
            .ok_or_else(|| err("lambda index out of bounds"))?;
        *target = product;
    }

    Ok(())
}
/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, param: &Parameters, f0: &mut PolyArray) -> Result<()> {
    let dim = param.dim as usize;
    if f0.polies.len() != dim {
        return Err(err("f0 dimension does not match parameters"));
    }

    for poly in &mut f0.polies {
        for coeff in &mut poly.coeffs {
            *coeff = randrange(0, channel.q.saturating_sub(1)) as i64;
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
    if f0.polies.len() != dim || x.polies.len() != dim || f1.coeffs.len() < dim {
        return Err(err("invalid polynomial dimensions"));
    }

    let mut f_pre = Polynomial::new(vec![0; dim * 2]);
    for i in 0..dim {
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        f_pre = f_pre.add(&tmp, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    f1.set_zero();
    let diff = f1.coeffs.len().saturating_sub(f_pre.coeffs.len());
    for (idx, coeff) in f_pre.coeffs.iter().enumerate() {
        if diff + idx < f1.coeffs.len() {
            f1.coeffs[diff + idx] = *coeff;
        }
    }

    Ok(())
}
