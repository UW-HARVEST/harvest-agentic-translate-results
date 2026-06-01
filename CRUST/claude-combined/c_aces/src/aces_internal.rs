use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, Matrix2D, Matrix3D};
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
    let last_idx = rm.coeffs.len() - 1;
    let sum = rm.coef_sum();
    let shift = (rm.coeffs[last_idx] + (message as Coeff - sum)) % (q as Coeff);
    rm.coeffs[last_idx] = if shift > 0 { shift } else { shift + q as Coeff };
    Ok(())
}
/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    if e.coeffs.is_empty() {
        return Ok(());
    }
    let r = randrange(0, 1);
    let k: i64 = if (r as f64) < EPROB { 0 } else { 1 };
    for i in 0..e.coeffs.len() {
        e.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last_idx = e.coeffs.len() - 1;
    let sum = e.coef_sum();
    let shift = (e.coeffs[last_idx] + p as Coeff * k - sum) % (q as Coeff);
    e.coeffs[last_idx] = if shift > 0 { shift } else { shift + q as Coeff };
    Ok(())
}
/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    if b.coeffs.is_empty() {
        return Ok(());
    }
    let k = randrange(0, p) as Coeff;
    for i in 0..b.coeffs.len() {
        b.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last_idx = b.coeffs.len() - 1;
    let sum = b.coef_sum();
    let shift = (b.coeffs[last_idx] + k - sum) % (q as Coeff);
    b.coeffs[last_idx] = if shift > 0 { shift } else { shift + q as Coeff };
    Ok(())
}
/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    if u.coeffs.len() < dim + 1 {
        return Ok(());
    }
    // Init u to zero first
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
    u.coeffs[dim] = channel.q as i64 - sum;
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

    // Set secret polynomial coefficients from m (column-wise)
    for i in 0..dim {
        for j in 0..dim {
            // secret->polies[i].coeffs[j] = matrix2d_get(&m, j, i);
            if i < secret.polies.len() && j < secret.polies[i].coeffs.len() {
                secret.polies[i].coeffs[j] = m.data[j * dim + i] as Coeff;
            }
        }
    }

    // For each i, compute lambda[i] = invm * arr where arr[row][j] is built from
    // poly_mul(secret[i], secret[j]) % u
    for i in 0..dim {
        // Build arr matrix (use a temporary Matrix2D)
        let mut arr = Matrix2D::new(dim);
        for j in 0..dim {
            // a_ij_poly = secret[i] * secret[j]  with size 2*secret.size
            // The C code computes the full multiplication and then poly_mod by u
            let secret_i = &secret.polies[i];
            let secret_j = &secret.polies[j];
            // poly_mul in C produces a polynomial up to size 2*dim, then trims via fit
            let mut a_ij_poly = poly_mul_secret(secret_i, secret_j, 2 * dim, channel.q)?;
            // Apply poly_mod
            a_ij_poly.poly_mod(u, channel.q)?;

            // arr->data[row][j] = a_ij_poly.coeffs[arr->dim - row - 1]
            // But the polynomial may have been trimmed, so we need to handle padding
            // a_ij_poly.coeffs[arr->dim - row - 1]: in C, a_ij_poly was size 2*dim before the trim
            // and after fit, size could be smaller. The C code accesses
            // a_ij_poly.coeffs[arr->dim - row - 1] which assumes size at least dim.
            // Actually, looking at the C code: it accesses a_ij_poly.coeffs[arr->dim - row - 1]
            // for row in 0..dim. So index ranges from dim-1 down to 0.
            // We need indices 0..dim into the polynomial. If poly is smaller, treat missing
            // (high-order, leftmost) coefficients as 0.
            let poly_size = a_ij_poly.coeffs.len();
            for row in 0..dim {
                let idx = dim.saturating_sub(row + 1);
                let val: u64 = if idx < poly_size {
                    a_ij_poly.coeffs[idx] as u64
                } else {
                    0
                };
                arr.data[row * dim + j] = val;
            }
        }

        // lambda[i] = invm * arr
        let result = crate::matrix::matrix2d_multiply(&invm, &arr, channel.q)?;
        if let Some(lambda_i) = lambda.get_mut(i) {
            *lambda_i = result;
        }
    }

    Ok(())
}

/// Poly multiplication that produces a result with the specified buffer size
/// and applies poly_fit to remove leading zeros, matching the C implementation
/// of poly_mul which writes into a pre-allocated buffer.
fn poly_mul_secret(
    poly1: &Polynomial,
    poly2: &Polynomial,
    result_size: usize,
    modulus: u64,
) -> Result<Polynomial> {
    // Reproduce C's poly_mul with the explicit result size.
    let mut result = vec![0 as Coeff; result_size];
    if poly1.coeffs.is_empty() || poly2.coeffs.is_empty() {
        return Ok(Polynomial { coeffs: result });
    }
    let m = modulus as i64;
    let deg1 = poly1.coeffs.len() - 1;
    let deg2 = poly2.coeffs.len() - 1;
    if result_size < deg1 + deg2 + 1 {
        return Err(crate::error::AcesError::GenericError(
            "result too small".to_string(),
        ));
    }
    let res_deg = result_size - deg2 - deg1 - 1;
    for i in 0..poly1.coeffs.len() {
        for j in 0..poly2.coeffs.len() {
            let prod = (poly1.coeffs[i] * poly2.coeffs[j]) % m;
            result[res_deg + i + j] = result[res_deg + i + j] + prod;
        }
    }
    let mut p = Polynomial { coeffs: result };
    p.fit(modulus)?;
    Ok(p)
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
    let buffer_size = dim * 2;

    let mut f_pre = Polynomial {
        coeffs: vec![0 as Coeff; buffer_size],
    };

    for i in 0..dim {
        let tmp = poly_mul_secret(&f0.polies[i], &x.polies[i], buffer_size, channel.q)?;
        f_pre = poly_add_into(&f_pre, &tmp, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // f1 = f_pre but trimmed to f_pre.size
    // The C code does:
    //   diff = f1->size - f_pre.size;
    //   f1->coeffs += diff;
    //   f1->size -= diff;
    //   memcpy(f1->coeffs, f_pre.coeffs, f_pre.size * sizeof(Coeff));
    //
    // Since Rust uses Vec, we just resize and copy.
    *f1 = f_pre;
    Ok(())
}

/// Helper: poly add producing a result of the larger size.
fn poly_add_into(p1: &Polynomial, p2: &Polynomial, modulus: u64) -> Result<Polynomial> {
    p1.add(p2, modulus)
}
