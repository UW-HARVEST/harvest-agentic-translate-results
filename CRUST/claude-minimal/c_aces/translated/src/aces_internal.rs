use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, Matrix2D, Matrix3D};
use crate::polynomial::{Coeff, PolyArray, Polynomial};

pub const EPROB: f64 = 0.5;

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    let n = rm.coeffs.len();
    if n == 0 {
        return Ok(());
    }
    for i in 0..n {
        rm.coeffs[i] = randrange(0, q) as i64;
    }

    let qc = q as i64;
    let shift = (rm.coeffs[n - 1] + (message as i64 - rm.coef_sum())) % qc;
    rm.coeffs[n - 1] = if shift > 0 { shift } else { shift + qc };
    Ok(())
}

/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    let k: i64 = if (randrange(0, 1) as f64) < EPROB { 0 } else { 1 };
    let n = e.coeffs.len();
    if n == 0 {
        return Ok(());
    }
    for i in 0..n {
        e.coeffs[i] = randrange(0, q) as i64;
    }
    let qc = q as i64;
    let shift = (e.coeffs[n - 1] + (p as i64) * k - e.coef_sum()) % qc;
    e.coeffs[n - 1] = if shift > 0 { shift } else { shift + qc };
    Ok(())
}

/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    let k = randrange(0, p) as i64;
    let n = b.coeffs.len();
    if n == 0 {
        return Ok(());
    }
    for i in 0..n {
        b.coeffs[i] = randrange(0, q) as i64;
    }
    let qc = q as i64;
    let shift = (b.coeffs[n - 1] + k - b.coef_sum()) % qc;
    b.coeffs[n - 1] = if shift > 0 { shift } else { shift + qc };
    Ok(())
}

/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    if u.coeffs.len() < dim + 1 {
        // Resize so we can place the leading and trailing coefficients.
        u.coeffs.resize(dim + 1, 0);
    }
    let mut nonzeros = randrange(param.dim / 2, param.dim - 1);
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
            let mean = (zeroes / 2) as f64;
            let std = (zeroes / 2) as f64;
            let sample = normal_rand(mean, std);
            // sample may be negative; clamp it via the same logic as C: cast to u64.
            let samp_u64 = if sample < 0.0 { 0u64 } else { sample as u64 };
            let samp = clamp(0, zeroes, samp_u64);
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

    // Ensure secret has dim polynomials of length dim.
    if secret.polies.len() < dim {
        secret
            .polies
            .resize(dim, Polynomial::new(vec![0 as Coeff; dim]));
    }
    for i in 0..dim {
        if secret.polies[i].coeffs.len() < dim {
            secret.polies[i].coeffs.resize(dim, 0);
        }
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.data[j * dim + i] as i64;
        }
    }

    // Reuse `m` as scratch matrix arr.
    let mut arr = m;

    for i in 0..dim {
        for j in 0..dim {
            let mut a_ij_poly = Polynomial::new(vec![0 as Coeff; 2 * dim]);
            // poly_mul places result based on result_size logic. In our impl, mul produces
            // a poly of size deg1+deg2+1 then poly_fit. We then poly_mod by u.
            let prod = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            // Pad/truncate prod to size 2*dim with leading zeros (left), to match C semantics.
            // Actually, in our poly impl we don't need to do this; just use prod directly.
            a_ij_poly.coeffs = prod.coeffs;
            a_ij_poly.poly_mod(u, channel.q)?;

            // Pad poly result with leading zeros so it's size dim.
            let pad = if arr.dim > a_ij_poly.coeffs.len() {
                arr.dim - a_ij_poly.coeffs.len()
            } else {
                0
            };
            for row in 0..arr.dim {
                // a_ij_poly.coeffs[arr.dim - row - 1] when accounting for padded leading zeros
                let idx = arr.dim - row - 1;
                let val: i64 = if idx < pad {
                    0
                } else {
                    a_ij_poly.coeffs[idx - pad]
                };
                let dim_arr = arr.dim;
                arr.data[row * dim_arr + j] = val as u64;
            }
        }

        // Multiply invm * arr and store in lambda[i].
        let result_mat = crate::matrix::matrix2d_multiply(&invm, &arr, channel.q)?;
        if let Some(target) = lambda.get_mut(i) {
            *target = result_mat;
        }
    }
    Ok(())
}

/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, _param: &Parameters, f0: &mut PolyArray) -> Result<()> {
    for i in 0..f0.polies.len() {
        for j in 0..f0.polies[i].coeffs.len() {
            f0.polies[i].coeffs[j] = randrange(0, channel.q - 1) as i64;
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
    let mut f_pre = Polynomial::new(vec![0 as Coeff; dim * 2]);

    for i in 0..dim {
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        // Pad to dim * 2 if needed
        let mut tmp_padded = vec![0 as Coeff; dim * 2];
        let offset = (dim * 2).saturating_sub(tmp.coeffs.len());
        for (idx, c) in tmp.coeffs.iter().enumerate() {
            tmp_padded[offset + idx] = *c;
        }
        let tmp_poly = Polynomial::new(tmp_padded);
        f_pre = f_pre.add(&tmp_poly, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // Replicate C: copy the trailing portion of f_pre into f1 (right-aligned).
    let n = f1.coeffs.len();
    if f_pre.coeffs.len() <= n {
        let diff = n - f_pre.coeffs.len();
        // Match C: shifts pointer right by `diff`, and reduces size accordingly.
        // The Rust equivalent: only the last `f_pre.coeffs.len()` slots of f1 are written, but the
        // resulting polynomial is conceptually f1 with reduced size. We'll truncate from the front
        // instead so the polynomial degree matches.
        f1.coeffs.drain(0..diff);
        for (i, c) in f_pre.coeffs.iter().enumerate() {
            f1.coeffs[i] = *c;
        }
    } else {
        // f_pre is larger - just take the trailing `n` coefficients.
        let start = f_pre.coeffs.len() - n;
        for i in 0..n {
            f1.coeffs[i] = f_pre.coeffs[start + i];
        }
    }

    Ok(())
}
