use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
use crate::polynomial::{Coeff, PolyArray, Polynomial};

const EPROB: u64 = 0; // EPROB = 0.5; randrange(0,1) returns 0 or 1.
                      // C: `randrange(0, 1) < EPROB ? 0 : 1`. With EPROB=0.5 and integer
                      // randrange returning 0 or 1, the comparison `< 0.5` is true only when
                      // the value is 0. We replicate that with a manual check below.

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    let n = rm.coeffs.len();
    if n == 0 {
        return Ok(());
    }
    for i in 0..n {
        rm.coeffs[i] = randrange(0, q) as Coeff;
    }
    let qm = q as i64;
    let last = rm.coeffs[n - 1];
    let sum = rm.coef_sum();
    let shift = (last + (message as i64) - sum) % qm;
    rm.coeffs[n - 1] = if shift > 0 { shift } else { shift + qm };
    Ok(())
}
/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    let n = e.coeffs.len();
    if n == 0 {
        return Ok(());
    }
    // C: `int64_t k = randrange(0, 1) < EPROB ? 0 : 1;`
    // EPROB = 0.5, randrange returns 0 or 1. Comparison `0 < 0.5` -> true (k=0), `1 < 0.5` -> false (k=1).
    let r = randrange(0, 1);
    let k: i64 = if (r as f64) < 0.5 { 0 } else { 1 };
    let _ = EPROB; // referenced for parity
    for i in 0..n {
        e.coeffs[i] = randrange(0, q) as Coeff;
    }
    let qm = q as i64;
    let last = e.coeffs[n - 1];
    let sum = e.coef_sum();
    let shift = (last + (p as i64) * k - sum) % qm;
    e.coeffs[n - 1] = if shift > 0 { shift } else { shift + qm };
    Ok(())
}
/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    let n = b.coeffs.len();
    if n == 0 {
        return Ok(());
    }
    let k = randrange(0, p) as i64;
    for i in 0..n {
        b.coeffs[i] = randrange(0, q) as Coeff;
    }
    let qm = q as i64;
    let last = b.coeffs[n - 1];
    let sum = b.coef_sum();
    let shift = (last + k - sum) % qm;
    b.coeffs[n - 1] = if shift > 0 { shift } else { shift + qm };
    Ok(())
}
/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    if u.coeffs.len() < dim + 1 {
        return Err(crate::error::AcesError::GenericError(
            "u polynomial too small".to_string(),
        ));
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
            let mean = (zeroes / 2) as f64;
            let stddev = (zeroes / 2) as f64;
            let raw = normal_rand(mean, stddev);
            // C performs `clamp(0, zeroes, normal_rand(...))` where the
            // truncation to uint64_t happens implicitly.
            let raw_u = if raw < 0.0 { 0u64 } else { raw as u64 };
            let samp = clamp(0, zeroes, raw_u);
            let zero = randrange(0, samp);
            for _ in 0..zero {
                if i < dim {
                    u.coeffs[i] = 0;
                    i += 1;
                } else {
                    break;
                }
            }
            zeroes -= zero;
        }
    }

    u.coeffs[dim] = 0;
    let s = u.coef_sum();
    u.coeffs[dim] = (channel.q as i64) - s;
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

    // secret->polies[i].coeffs[j] = matrix2d_get(&m, j, i);
    // We assume secret already has dim polynomials of length dim.
    for i in 0..dim {
        if secret.polies[i].coeffs.len() < dim {
            return Err(crate::error::AcesError::GenericError(
                "secret poly too small".to_string(),
            ));
        }
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.data[j * dim + i] as Coeff;
        }
    }

    // For each i, we build a Matrix2D `arr` whose column j (after the loop body)
    // contains coefficients from a_ij_poly = secret_i * secret_j mod u.
    let mut arr = Matrix2D::new(dim);

    for i in 0..dim {
        for j in 0..dim {
            // a_ij_poly = secret[i] * secret[j], as a polynomial of size 2*dim.
            // Then poly_mod by u.
            let mut a_ij_poly = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            // Pad/truncate so that we have exactly 2*dim coefficients (left-padded with 0s),
            // mirroring the C behavior where the result buffer was 2*dim.
            // After fit(), the polynomial may have fewer than 2*dim coefficients.
            if a_ij_poly.coeffs.len() < 2 * dim {
                let pad = 2 * dim - a_ij_poly.coeffs.len();
                let mut padded = vec![0i64; 2 * dim];
                for (idx, c) in a_ij_poly.coeffs.iter().enumerate() {
                    padded[pad + idx] = *c;
                }
                a_ij_poly.coeffs = padded;
            }
            a_ij_poly.poly_mod(u, channel.q)?;
            // After poly_mod, ensure we have arr->dim coefficients accessible.
            // C does: matrix2d_set(arr, row, j, a_ij_poly.coeffs[arr->dim - row - 1]);
            // The coefficient indexed at (dim - row - 1) is read.
            // If the polynomial has fewer coefficients than dim, fill missing high-index
            // positions with 0 (since the polynomial is implicitly left-padded).
            let len = a_ij_poly.coeffs.len();
            for row in 0..dim {
                let target_idx = dim as isize - row as isize - 1;
                let val = if target_idx < 0 {
                    0
                } else {
                    let idx = target_idx as usize;
                    // a_ij_poly.coeffs is conceptually right-aligned; coefficient with
                    // exponent k is at index (len - 1 - k). But the C code does direct
                    // indexing into `a_ij_poly.coeffs[arr->dim - row - 1]`, treating
                    // the buffer as having exactly dim entries. We need to mimic that.
                    // The original buffer was 2*dim long, and after poly_fit it's
                    // potentially smaller. The C code reads from index (dim - row - 1)
                    // of whatever size the polynomial currently has.
                    if idx < len {
                        a_ij_poly.coeffs[idx]
                    } else {
                        0
                    }
                };
                arr.data[row * dim + j] = val as u64;
            }
        }
        // lambda[i] = invm * arr (mod q)
        let result = matrix2d_multiply(&invm, &arr, channel.q)?;
        if let Some(target) = lambda.get_mut(i) {
            target.data = result.data;
            target.dim = result.dim;
        }
    }

    Ok(())
}
/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, _param: &Parameters, f0: &mut PolyArray) -> Result<()> {
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
    // f_pre is initially zero polynomial of size 2*dim.
    let mut f_pre = Polynomial {
        coeffs: vec![0i64; 2 * dim],
    };
    for i in 0..dim {
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        // Left-pad tmp to size 2*dim.
        let mut padded = vec![0i64; 2 * dim];
        let pad = 2 * dim - tmp.coeffs.len();
        for (idx, c) in tmp.coeffs.iter().enumerate() {
            padded[pad + idx] = *c;
        }
        let tmp_padded = Polynomial { coeffs: padded };
        f_pre = f_pre.add(&tmp_padded, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // C: f1->coeffs += diff; f1->size -= diff; memcpy(...)
    // This shifts the f1 buffer pointer forward and shrinks its size to match f_pre.
    // In Rust, mirror this by truncating f1 to f_pre.coeffs.len() and copying.
    if f1.coeffs.len() < f_pre.coeffs.len() {
        return Err(crate::error::AcesError::GenericError(
            "f1 buffer too small".to_string(),
        ));
    }
    let new_len = f_pre.coeffs.len();
    f1.coeffs.truncate(new_len);
    for i in 0..new_len {
        f1.coeffs[i] = f_pre.coeffs[i];
    }
    Ok(())
}
