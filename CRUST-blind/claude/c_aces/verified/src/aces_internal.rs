use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
use crate::polynomial::{Coeff, PolyArray, Polynomial};

const EPROB: f64 = 0.5;

fn coeff_modulo(value: i64, modulus: u64) -> i64 {
    if modulus == 0 {
        return value;
    }
    let m = modulus as i64;
    let r = value % m;
    if r < 0 {
        r + m
    } else {
        r
    }
}

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    if rm.coeffs.is_empty() {
        return Ok(());
    }
    let size = rm.coeffs.len();
    for i in 0..size {
        rm.coeffs[i] = randrange(0, q.saturating_sub(1)) as i64;
    }
    let last = rm.coeffs[size - 1];
    let csum = rm.coef_sum();
    let shift = coeff_modulo(last + (message as i64) - csum, q);
    rm.coeffs[size - 1] = if shift > 0 { shift } else { shift + q as i64 };
    Ok(())
}

/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    if e.coeffs.is_empty() {
        return Ok(());
    }
    // Mirror C: int64_t k = randrange(0, 1) < EPROB ? 0 : 1;
    // randrange(0, 1) returns u64 (0 or 1), comparing < 0.5: if 0 -> true, if 1 -> false
    let k: i64 = if (randrange(0, 1) as f64) < EPROB { 0 } else { 1 };
    let size = e.coeffs.len();
    for i in 0..size {
        e.coeffs[i] = randrange(0, q.saturating_sub(1)) as i64;
    }
    let last = e.coeffs[size - 1];
    let csum = e.coef_sum();
    let shift = coeff_modulo(last + (p as i64) * k - csum, q);
    e.coeffs[size - 1] = if shift > 0 { shift } else { shift + q as i64 };
    Ok(())
}

/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    if b.coeffs.is_empty() {
        return Ok(());
    }
    let k: i64 = randrange(0, p) as i64;
    let size = b.coeffs.len();
    for i in 0..size {
        b.coeffs[i] = randrange(0, q.saturating_sub(1)) as i64;
    }
    let last = b.coeffs[size - 1];
    let csum = b.coef_sum();
    let shift = coeff_modulo(last + k - csum, q);
    b.coeffs[size - 1] = if shift > 0 { shift } else { shift + q as i64 };
    Ok(())
}

/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    if u.coeffs.len() < dim + 1 {
        u.coeffs.resize(dim + 1, 0);
    }
    // Reset
    for c in u.coeffs.iter_mut() {
        *c = 0;
    }

    if dim == 0 {
        return Ok(());
    }
    if dim == 1 {
        u.coeffs[0] = 1;
        u.coeffs[1] = (channel.q as i64) - u.coef_sum();
        return Ok(());
    }

    // Mimic randrange(dim/2, dim - 1)
    let lower = (param.dim / 2) as u64;
    let upper = (param.dim - 1) as u64;
    let mut nonzeros = randrange(lower, upper);
    let mut zeroes = (param.dim as u64) - nonzeros;
    u.coeffs[0] = 1;

    let mut i: usize = 1;
    while i < dim {
        if nonzeros > 1 {
            u.coeffs[i] = randrange(0, channel.q.saturating_sub(1)) as i64;
            i += 1;
            nonzeros -= 1;
        } else {
            break;
        }

        if zeroes > 0 {
            let mid = (zeroes / 2) as f64;
            let sample = normal_rand(mid, mid);
            // C casts double->uint64_t. Negative doubles produce undefined behavior in C
            // but typically end up as 0 / large. Use clamp(0, zeroes, sample) where sample
            // is interpreted as if cast to u64. Negative values -> 0.
            let sample_u64 = if sample.is_nan() || sample < 0.0 {
                0
            } else {
                sample as u64
            };
            let samp = clamp(0, zeroes, sample_u64);
            let zero = randrange(0, samp);
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
    u.coeffs[dim] = (channel.q as i64) - u.coef_sum();
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

    // Ensure secret has appropriate size.
    if secret.polies.len() < dim {
        secret
            .polies
            .resize(dim, Polynomial { coeffs: vec![0; dim] });
    }
    for i in 0..dim {
        if secret.polies[i].coeffs.len() < dim {
            secret.polies[i].coeffs.resize(dim, 0);
        }
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.data[j * dim + i] as i64;
        }
    }

    // Ensure lambda has dim 2D matrices of dim x dim.
    if lambda.data.len() < dim {
        lambda.data.resize(dim, Matrix2D::new(dim));
    }
    for k in 0..dim {
        if lambda.data[k].dim != dim {
            lambda.data[k] = Matrix2D::new(dim);
        }
    }

    // Reuse `m` as `arr` to hold per-column polynomial coefficients.
    let mut arr = Matrix2D::new(dim);

    for i in 0..dim {
        for j in 0..dim {
            // a_ij = secret[i] * secret[j] mod u
            let mut a_ij_poly = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            a_ij_poly.poly_mod(u, channel.q)?;

            // The C version writes a_ij_poly.coeffs[arr->dim - row - 1] into arr.
            // After poly_mod, a_ij_poly may have a smaller size; pad with zeros at the
            // high end to length `dim` for indexing.
            let mut padded: Vec<Coeff> = vec![0; dim];
            let csize = a_ij_poly.coeffs.len();
            // Polynomial layout has highest degree at index 0. Right-align (low degree
            // at the end) so coeffs[size-1] is constant term.
            if csize <= dim {
                let offset = dim - csize;
                for (k, &c) in a_ij_poly.coeffs.iter().enumerate() {
                    padded[offset + k] = c;
                }
            } else {
                // Should not happen after mod, but truncate in case.
                let offset = csize - dim;
                for k in 0..dim {
                    padded[k] = a_ij_poly.coeffs[offset + k];
                }
            }

            for row in 0..dim {
                let val = padded[dim - row - 1] as u64;
                arr.data[row * dim + j] = val;
            }
        }

        // lambda[i] = invm * arr (mod q)
        let result = matrix2d_multiply(&invm, &arr, channel.q)?;
        lambda.data[i] = result;
    }

    Ok(())
}

/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, _param: &Parameters, f0: &mut PolyArray) -> Result<()> {
    // randrange(0, q - 1) yields the range [0, q-1] inclusive.
    let upper = channel.q.saturating_sub(1);
    for i in 0..f0.polies.len() {
        let len = f0.polies[i].coeffs.len();
        for j in 0..len {
            f0.polies[i].coeffs[j] = randrange(0, upper) as i64;
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
    let mut f_pre = Polynomial {
        coeffs: vec![0i64; dim * 2],
    };

    for i in 0..dim {
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        f_pre = f_pre.add(&tmp, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // Replace f1 contents with f_pre.
    f1.coeffs = f_pre.coeffs;
    Ok(())
}
