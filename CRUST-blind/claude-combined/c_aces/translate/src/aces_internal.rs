use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
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
    let q_i = q as Coeff;
    let last = rm.coeffs[size - 1];
    let csum: i64 = rm.coeffs.iter().sum();
    let shift = (last + (message as Coeff - csum)) % q_i;
    rm.coeffs[size - 1] = if shift > 0 { shift } else { shift + q_i };
    Ok(())
}
/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    // C: int64_t k = randrange(0, 1) < EPROB ? 0 : 1;
    let k: i64 = if (randrange(0, 1) as f64) < EPROB { 0 } else { 1 };
    let size = e.coeffs.len();
    for i in 0..size {
        e.coeffs[i] = randrange(0, q) as Coeff;
    }
    if size == 0 {
        return Ok(());
    }
    let q_i = q as Coeff;
    let last = e.coeffs[size - 1];
    let csum: i64 = e.coeffs.iter().sum();
    let shift = (last + (p as Coeff) * k - csum) % q_i;
    e.coeffs[size - 1] = if shift > 0 { shift } else { shift + q_i };
    Ok(())
}
/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    let k: i64 = randrange(0, p) as i64;
    let size = b.coeffs.len();
    for i in 0..size {
        b.coeffs[i] = randrange(0, q) as Coeff;
    }
    if size == 0 {
        return Ok(());
    }
    let q_i = q as Coeff;
    let last = b.coeffs[size - 1];
    let csum: i64 = b.coeffs.iter().sum();
    let shift = (last + k - csum) % q_i;
    b.coeffs[size - 1] = if shift > 0 { shift } else { shift + q_i };
    Ok(())
}
/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    // u must have size = param.dim + 1
    if u.coeffs.len() < (param.dim as usize) + 1 {
        // ensure size
        u.coeffs.resize((param.dim as usize) + 1, 0);
    }
    // Reset coefficients to 0
    for c in u.coeffs.iter_mut() {
        *c = 0;
    }

    let dim = param.dim;
    let mut nonzeros = randrange(dim / 2, dim - 1);
    let mut zeroes = dim - nonzeros;
    u.coeffs[0] = 1;

    let mut i: usize = 1;
    while i < dim as usize {
        if nonzeros > 1 {
            u.coeffs[i] = randrange(0, channel.q) as Coeff;
            i += 1;
            nonzeros -= 1;
        } else {
            break;
        }

        if zeroes > 0 {
            // C: clamp(0, zeroes, normal_rand(zeroes / 2, zeroes / 2));
            let half = (zeroes / 2) as f64;
            let nr = normal_rand(half, half);
            // The C casts the double to uint64_t before clamp.
            let nr_u = if nr < 0.0 { 0u64 } else { nr as u64 };
            let samp = clamp(0, zeroes, nr_u);
            let zero = randrange(0, samp);
            for _ in 0..zero {
                u.coeffs[i] = 0;
                i += 1;
            }
            zeroes -= zero;
        }
    }

    let last_idx = dim as usize;
    u.coeffs[last_idx] = 0;
    let csum: i64 = u.coeffs.iter().sum();
    u.coeffs[last_idx] = (channel.q as i64) - csum;
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

    // secret should have `dim` polynomials each of size `dim`.
    if secret.polies.len() < dim {
        secret.polies.resize(dim, Polynomial::new(vec![0; dim]));
    }
    for i in 0..dim {
        if secret.polies[i].coeffs.len() < dim {
            secret.polies[i].coeffs.resize(dim, 0);
        }
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.data[j * dim + i] as Coeff;
        }
    }

    // arr is a working matrix2d, like &m in the C code (writes columns into m)
    let mut arr = Matrix2D::new(dim);

    for i in 0..secret.polies.len() {
        for j in 0..secret.polies.len() {
            // Compute a_ij_poly = secret[i] * secret[j], modulo u, modulo q.
            let prod = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            let mut a_ij_poly = Polynomial::new(prod.coeffs.clone());
            // Pad to size 2*secret_size (matches C's behavior of reusing buffer)
            let target_len = 2 * secret.polies.len();
            if a_ij_poly.coeffs.len() < target_len {
                let pad = target_len - a_ij_poly.coeffs.len();
                let mut new_coeffs = vec![0i64; pad];
                new_coeffs.extend_from_slice(&a_ij_poly.coeffs);
                a_ij_poly.coeffs = new_coeffs;
            }
            a_ij_poly.poly_mod(u, channel.q)?;

            // C iterates row 0..arr->dim and reads `a_ij_poly.coeffs[arr->dim - row - 1]`
            // a_ij_poly may have shrunk via poly_fit; if it's smaller than arr->dim,
            // we need to access "virtual" leading zeros. The C code unconditionally
            // accesses these slots even after poly_fit shrinks the size. To mimic the
            // semantics safely, treat out-of-range accesses as 0.
            for row in 0..arr.dim {
                let col_idx = arr.dim - row - 1;
                let value = if col_idx < a_ij_poly.coeffs.len() {
                    // a_ij_poly is fitted, so coeffs are stored compactly
                    // We need to map: the highest-degree term is at index 0 in coeffs
                    // The original poly buffer was 2*dim entries with leading zeros
                    // After fit, the poly is left-trimmed.
                    // C reads buf[dim - row - 1], where buf is original size 2*dim.
                    // After fit, the layout in buf is the same logical poly value.
                    // However, fit() in our Rust impl truncates, so we lose those zeros.
                    // To preserve C semantics, reconstruct as if it had size 2*dim:
                    let logical_size = 2 * secret.polies.len();
                    let leading_zeros = logical_size - a_ij_poly.coeffs.len();
                    if (arr.dim - row - 1) < leading_zeros {
                        0i64
                    } else {
                        a_ij_poly.coeffs[(arr.dim - row - 1) - leading_zeros]
                    }
                } else {
                    0i64
                };
                arr.set(row, j, value as u64)?;
            }
        }

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
    let target_len = dim * 2;

    // f_pre: zero-initialized poly of size dim*2
    let mut f_pre = Polynomial::new(vec![0i64; target_len]);

    for i in 0..dim {
        let prod = f0.polies[i].mul(&x.polies[i], channel.q)?;
        // Pad prod to target_len with leading zeros
        let mut prod_padded = if prod.coeffs.len() < target_len {
            let pad = target_len - prod.coeffs.len();
            let mut v = vec![0i64; pad];
            v.extend_from_slice(&prod.coeffs);
            Polynomial::new(v)
        } else {
            prod
        };
        // f_pre = f_pre + prod, mod q
        // ensure prod_padded is exactly target_len for the in-place add
        if prod_padded.coeffs.len() != target_len {
            prod_padded.coeffs.resize(target_len, 0);
        }
        f_pre = f_pre.add(&prod_padded, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // The C code: f1->coeffs += diff; f1->size -= diff; memcpy(f1->coeffs, f_pre.coeffs, ...)
    // i.e., it shifts f1's "view" so that f1 contains exactly f_pre's coefficients.
    // In Rust, we just replace f1's coeffs with f_pre's coeffs.
    f1.coeffs = f_pre.coeffs;
    Ok(())
}
