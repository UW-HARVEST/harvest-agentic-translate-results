use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
use crate::polynomial::{PolyArray, Polynomial};

const EPROB: f64 = 0.5;

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    for c in rm.coeffs.iter_mut() {
        *c = randrange(0, q) as i64;
    }
    let len = rm.coeffs.len();
    let coef_sum = rm.coef_sum();
    let q_i = q as i64;
    let last = rm.coeffs[len - 1];
    let shift = (last + (message as i64) - coef_sum) % q_i;
    rm.coeffs[len - 1] = if shift > 0 { shift } else { shift + q_i };
    Ok(())
}
/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    let r = randrange(0, 1) as f64;
    let k: i64 = if r < EPROB { 0 } else { 1 };
    for c in e.coeffs.iter_mut() {
        *c = randrange(0, q) as i64;
    }
    let len = e.coeffs.len();
    let coef_sum = e.coef_sum();
    let q_i = q as i64;
    let last = e.coeffs[len - 1];
    let shift = (last + (p as i64) * k - coef_sum) % q_i;
    e.coeffs[len - 1] = if shift > 0 { shift } else { shift + q_i };
    Ok(())
}
/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    let k = randrange(0, p) as i64;
    for c in b.coeffs.iter_mut() {
        *c = randrange(0, q) as i64;
    }
    let len = b.coeffs.len();
    let coef_sum = b.coef_sum();
    let q_i = q as i64;
    let last = b.coeffs[len - 1];
    let shift = (last + k - coef_sum) % q_i;
    b.coeffs[len - 1] = if shift > 0 { shift } else { shift + q_i };
    Ok(())
}
/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    // Polynomial size is dim + 1 in the C code (since u stores dim+1 coeffs).
    let mut nonzeros = randrange((param.dim / 2) as u64, (param.dim - 1) as u64);
    let mut zeroes = param.dim - nonzeros;
    u.coeffs[0] = 1;

    let mut i: usize = 1;
    while i < dim {
        if nonzeros > 1 {
            u.coeffs[i] = randrange(0, channel.q) as i64;
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
    let q_i = channel.q as i64;
    let sum = u.coef_sum();
    u.coeffs[dim] = q_i - sum;
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

    // Fill secret with columns of m: secret[i].coeffs[j] = m[j][i]
    for i in 0..dim {
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.data[j * dim + i] as i64;
        }
    }

    // For each i, j compute polynomial = secret[i] * secret[j] mod u, mod q
    // and arrange into a column in `arr` (which is &m in C). Then
    // lambda[i] = invm * arr.
    let secret_size = secret.polies.len();
    for i in 0..secret_size {
        // arr is reused across j-iterations within an i.
        let mut arr = Matrix2D::new(dim);
        for j in 0..secret_size {
            let prod = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            let mut a_ij = prod;
            a_ij.poly_mod(u, channel.q)?;
            // Place coefficients into column j of arr: arr[row][j] = a_ij.coeffs[arr.dim - row - 1]
            // The polynomial after poly_mod may be smaller; we need to align it
            // to the right of the dim-length column.
            // C code uses a_ij.coeffs[arr->dim - row - 1] directly. After poly_mod
            // a_ij is fitted; if its size is dim, we use as-is. If smaller, the
            // leading coefficients are conceptually 0 (left of array).
            // To mirror C, we need to ensure a_ij has length >= dim. Pad if smaller.
            let pad = if a_ij.coeffs.len() < dim {
                let need = dim - a_ij.coeffs.len();
                let mut padded = vec![0i64; need];
                padded.extend_from_slice(&a_ij.coeffs);
                padded
            } else {
                a_ij.coeffs.clone()
            };
            for row in 0..dim {
                arr.data[row * dim + j] = pad[dim - row - 1] as u64;
            }
        }

        let result = matrix2d_multiply(&invm, &arr, channel.q)?;
        let target = lambda
            .get_mut(i)
            .ok_or_else(|| crate::error::AcesError::GenericError("lambda index oob".into()))?;
        target.dim = result.dim;
        target.data = result.data;
    }
    Ok(())
}
/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, _param: &Parameters, f0: &mut PolyArray) -> Result<()> {
    for poly in f0.polies.iter_mut() {
        for c in poly.coeffs.iter_mut() {
            *c = randrange(0, channel.q - 1) as i64;
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
    // Build f_pre starting as zero polynomial of size dim*2
    let mut f_pre = Polynomial::new(vec![0i64; dim * 2]);

    for i in 0..dim {
        let prod = f0.polies[i].mul(&x.polies[i], channel.q)?;
        // Sum: pad prod to f_pre size then add.
        f_pre = f_pre.add(&prod, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // Mirror C: f1->coeffs += diff; f1->size -= diff; copy f_pre into f1.
    // In Rust, replace f1.coeffs with the f_pre.coeffs.
    f1.coeffs = f_pre.coeffs.clone();
    Ok(())
}
