use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, Matrix2D, Matrix3D};
use crate::polynomial::{Coeff, PolyArray, Polynomial};

const EPROB: f64 = 0.5;

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    let size = rm.coeffs.len();
    if size == 0 {
        return Ok(());
    }
    for i in 0..size {
        rm.coeffs[i] = randrange(0, q) as Coeff;
    }
    let qi = q as Coeff;
    let sum = rm.coef_sum();
    let last_idx = size - 1;
    let shift = (rm.coeffs[last_idx] + (message as Coeff - sum)) % qi;
    rm.coeffs[last_idx] = if shift > 0 { shift } else { shift + qi };
    Ok(())
}

/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    let size = e.coeffs.len();
    if size == 0 {
        return Ok(());
    }
    // C: int64_t k = randrange(0, 1) < EPROB ? 0 : 1;
    let r = randrange(0, 1);
    let k: i64 = if (r as f64) < EPROB { 0 } else { 1 };
    for i in 0..size {
        e.coeffs[i] = randrange(0, q) as Coeff;
    }
    let qi = q as Coeff;
    let sum = e.coef_sum();
    let last_idx = size - 1;
    let shift = (e.coeffs[last_idx] + (p as Coeff) * k - sum) % qi;
    e.coeffs[last_idx] = if shift > 0 { shift } else { shift + qi };
    Ok(())
}

/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    let size = b.coeffs.len();
    if size == 0 {
        return Ok(());
    }
    let k = randrange(0, p) as Coeff;
    for i in 0..size {
        b.coeffs[i] = randrange(0, q) as Coeff;
    }
    let qi = q as Coeff;
    let sum = b.coef_sum();
    let last_idx = size - 1;
    let shift = (b.coeffs[last_idx] + k - sum) % qi;
    b.coeffs[last_idx] = if shift > 0 { shift } else { shift + qi };
    Ok(())
}

/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    if u.coeffs.len() <= dim {
        // u must have dim + 1 coefficients
        return Ok(());
    }
    let mut nonzeros = randrange(param.dim / 2, param.dim - 1);
    let mut zeroes = param.dim - nonzeros;
    u.coeffs[0] = 1;

    let mut i: usize = 1;
    while i < dim {
        if nonzeros > 1 {
            u.coeffs[i] = randrange(0, channel.q) as Coeff;
            i += 1;
            nonzeros -= 1;
        } else {
            break;
        }

        if zeroes > 0 {
            let half = zeroes / 2;
            let samp = clamp(0, zeroes, normal_rand(half as f64, half as f64) as u64);
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
    let sum = u.coef_sum();
    u.coeffs[dim] = (channel.q as Coeff) - sum;
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

    // Set secret polynomials from columns of m: secret[i].coeffs[j] = m[j][i]
    for i in 0..dim {
        // Make sure secret has the appropriate dimensions.
        // Caller is supposed to provide PolyArray with `dim` polynomials of `dim` coeffs each.
        if i >= secret.polies.len() {
            break;
        }
        for j in 0..dim {
            if j < secret.polies[i].coeffs.len() {
                secret.polies[i].coeffs[j] = m.data[j * dim + i] as Coeff;
            }
        }
    }

    // For each i, build a matrix of products poly_i * poly_j mod u, then
    // multiply invm * arr to get lambda[i].
    for i in 0..dim {
        let mut arr = Matrix2D::new(dim);
        for j in 0..dim {
            let mut a_ij_poly = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            a_ij_poly.poly_mod(u, channel.q)?;
            // Store coefficients in column j of arr; row indices are reversed.
            // C: matrix2d_set(arr, row, j, a_ij_poly.coeffs[arr->dim - row - 1]);
            let len = a_ij_poly.coeffs.len();
            for row in 0..dim {
                // a_ij_poly.coeffs[arr->dim - row - 1] indexes into a_ij_poly's full
                // size, which after poly_fit may be less than dim. Out-of-range
                // access in C means undefined; we'll guard against it.
                let target_idx = dim - row - 1;
                let value = if target_idx < len {
                    a_ij_poly.coeffs[target_idx] as u64
                } else {
                    0
                };
                arr.data[row * dim + j] = value;
            }
        }
        let result = crate::matrix::matrix2d_multiply(&invm, &arr, channel.q)?;
        if let Some(lambda_i) = lambda.get_mut(i) {
            *lambda_i = result;
        }
    }

    Ok(())
}

/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, _param: &Parameters, f0: &mut PolyArray) -> Result<()> {
    for i in 0..f0.polies.len() {
        let len = f0.polies[i].coeffs.len();
        for j in 0..len {
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
    let target_size = dim * 2;
    let mut f_pre = Polynomial::new(vec![0; target_size]);

    for i in 0..dim {
        if i >= f0.polies.len() || i >= x.polies.len() {
            break;
        }
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        // Pad tmp to f_pre size to match C's add semantics.
        // C does poly_add(&f_pre, &tmp, &f_pre, q) where f_pre.size and tmp.size
        // may differ. Our add already handles sizes correctly but uses the larger size.
        // After the add, our result will have size = max(f_pre.size, tmp.size).
        // We need to maintain f_pre size invariant.
        let added = f_pre.add(&tmp, channel.q)?;
        // The result might be larger than f_pre.size; we keep added as f_pre.
        f_pre = added;
    }
    f_pre.poly_mod(u, channel.q)?;

    // f1 must be set to f_pre's contents (right-aligned).
    // Keep f1's allocated size; only change the contents.
    let f1_len = f1.coeffs.len();
    let f_pre_len = f_pre.coeffs.len();
    if f_pre_len <= f1_len {
        // Right-align: zero-fill the leading positions, copy f_pre into trailing.
        let diff = f1_len - f_pre_len;
        for i in 0..diff {
            f1.coeffs[i] = 0;
        }
        for i in 0..f_pre_len {
            f1.coeffs[diff + i] = f_pre.coeffs[i];
        }
    } else {
        // Truncate from leading positions if f_pre is larger than f1's size.
        let diff = f_pre_len - f1_len;
        for i in 0..f1_len {
            f1.coeffs[i] = f_pre.coeffs[diff + i];
        }
    }

    Ok(())
}
