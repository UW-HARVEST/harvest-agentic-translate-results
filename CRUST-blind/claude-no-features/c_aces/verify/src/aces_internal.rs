use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
use crate::polynomial::{Coeff, PolyArray, Polynomial};

const EPROB: f64 = 0.5;

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    if rm.coeffs.is_empty() {
        return Ok(());
    }
    for i in 0..rm.coeffs.len() {
        rm.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last = rm.coeffs.len() - 1;
    let qi = q as Coeff;
    let cs = rm.coef_sum();
    let shift = (rm.coeffs[last]
        .wrapping_add(message as Coeff)
        .wrapping_sub(cs))
        % qi;
    rm.coeffs[last] = if shift > 0 { shift } else { shift + qi };
    Ok(())
}

/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    if e.coeffs.is_empty() {
        return Ok(());
    }
    // C: int64_t k = randrange(0, 1) < EPROB ? 0 : 1;
    // randrange(0,1) returns either 0 or 1 (uint), and EPROB is 0.5; so k = 0 if randrange<0.5, else 1.
    let k: i64 = if (randrange(0, 1) as f64) < EPROB { 0 } else { 1 };
    for i in 0..e.coeffs.len() {
        e.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last = e.coeffs.len() - 1;
    let qi = q as Coeff;
    let pi = p as Coeff;
    let cs = e.coef_sum();
    let shift = (e.coeffs[last]
        .wrapping_add(pi.wrapping_mul(k))
        .wrapping_sub(cs))
        % qi;
    e.coeffs[last] = if shift > 0 { shift } else { shift + qi };
    Ok(())
}

/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    if b.coeffs.is_empty() {
        return Ok(());
    }
    let k = randrange(0, p) as i64;
    for i in 0..b.coeffs.len() {
        b.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last = b.coeffs.len() - 1;
    let qi = q as Coeff;
    let cs = b.coef_sum();
    let shift = (b.coeffs[last].wrapping_add(k).wrapping_sub(cs)) % qi;
    b.coeffs[last] = if shift > 0 { shift } else { shift + qi };
    Ok(())
}

/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    // The polynomial u has size param.dim + 1
    let dim = param.dim as usize;
    if u.coeffs.len() < dim + 1 {
        // Resize for safety: caller may not have set the correct size.
        u.coeffs.resize(dim + 1, 0);
    }
    // Reset coefficients
    for c in u.coeffs.iter_mut() {
        *c = 0;
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
            let half = (zeroes / 2) as f64;
            let samp = clamp(0, zeroes, normal_rand(half, half) as u64);
            let zero = randrange(0, samp);
            for _ in 0..zero {
                if i >= dim {
                    break;
                }
                u.coeffs[i] = 0;
                i += 1;
            }
            zeroes = zeroes.saturating_sub(zero);
        }
    }

    u.coeffs[dim] = 0;
    let cs = u.coef_sum();
    u.coeffs[dim] = (channel.q as i64).wrapping_sub(cs);
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

    // Set secret->size = dim implicitly via vector length.
    // Ensure secret.polies has at least `dim` polynomials, each with at least `dim` coeffs.
    if secret.polies.len() < dim {
        secret.polies.resize(dim, Polynomial::new(vec![0; dim]));
    }
    for i in 0..dim {
        if secret.polies[i].coeffs.len() < dim {
            secret.polies[i].coeffs.resize(dim, 0);
        }
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.get(j, i).unwrap_or(0) as Coeff;
        }
    }

    // For each i in [0, dim):
    //   build a Matrix2D `arr` (size dim) where for each j in [0, dim):
    //       compute a_ij_poly = (secret[i] * secret[j]) % u, mod channel.q
    //       and set arr[row][j] = a_ij_poly[arr.dim - row - 1] for row in [0, dim)
    //   lambda[i] = invm * arr (mod q)

    let mut arr = Matrix2D::new(dim);
    for i in 0..dim {
        for j in 0..dim {
            let mut a_ij_poly = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            a_ij_poly.poly_mod(u, channel.q)?;

            // a_ij_poly may be smaller than dim; treat missing leading entries as 0.
            // arr[row][j] = a_ij_poly.coeffs[arr.dim - row - 1] for row in 0..dim
            //
            // The C code reads from a_ij_poly.coeffs[arr->dim - row - 1] but a_ij_poly
            // might have size 2*secret->size = 2*dim. After poly_mod, the size has
            // shrunk. The index `arr->dim - row - 1` is in range [0, dim - 1].
            // We want to read the last `dim` coefficients of a_ij_poly (low-degree
            // terms; recall coefficients are stored from high-degree at index 0 to
            // low-degree at the last index).
            for row in 0..dim {
                let target_idx_from_end = row + 1; // dim - (dim - row - 1) = row + 1
                let coeffs_len = a_ij_poly.coeffs.len();
                let val = if target_idx_from_end <= coeffs_len {
                    a_ij_poly.coeffs[coeffs_len - target_idx_from_end]
                } else {
                    0
                };
                let val_u64 = ((val % channel.q as i64) + channel.q as i64) as u64 % channel.q;
                arr.set(row, j, val_u64)?;
            }
        }

        let result = matrix2d_multiply(&invm, &arr, channel.q)?;
        if let Some(target) = lambda.get_mut(i) {
            *target = result;
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
    let mut f_pre = Polynomial::new(vec![0; dim * 2]);

    for i in 0..dim {
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        // f_pre = f_pre + tmp  (with appropriate size handling)
        f_pre = f_pre.add(&tmp, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // Copy f_pre.coeffs into the tail of f1 — matching C's pointer math.
    if f1.coeffs.len() >= f_pre.coeffs.len() {
        let diff = f1.coeffs.len() - f_pre.coeffs.len();
        // C effectively shrinks f1's "size" to f_pre.size and copies in.
        // Translate to: write into f1.coeffs starting at index `diff`.
        for k in 0..f_pre.coeffs.len() {
            f1.coeffs[diff + k] = f_pre.coeffs[k];
        }
        // Then truncate f1 to start at `diff`:
        f1.coeffs.drain(0..diff);
    } else {
        // If f1 is smaller than f_pre, just copy what fits.
        f1.coeffs = f_pre.coeffs.clone();
    }
    Ok(())
}
