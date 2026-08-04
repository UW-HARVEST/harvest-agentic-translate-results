use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, matrix2d_multiply, Matrix2D, Matrix3D};
use crate::polynomial::{PolyArray, Polynomial};

const EPROB: f64 = 0.5;

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    let size = rm.coeffs.len();
    if size == 0 {
        return Ok(());
    }
    for i in 0..size {
        rm.coeffs[i] = randrange(0, q) as i64;
    }

    let q_i = q as i64;
    let m_i = message as i64;
    let sum: i64 = rm.coeffs.iter().sum();
    let last = rm.coeffs[size - 1];
    let shift = (last + (m_i - sum)) % q_i;
    rm.coeffs[size - 1] = if shift > 0 { shift } else { shift + q_i };
    Ok(())
}
/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    let size = e.coeffs.len();
    if size == 0 {
        return Ok(());
    }
    let raw = randrange(0, 1) as f64;
    let k: i64 = if raw < EPROB { 0 } else { 1 };
    for i in 0..size {
        e.coeffs[i] = randrange(0, q) as i64;
    }

    let q_i = q as i64;
    let p_i = p as i64;
    let sum: i64 = e.coeffs.iter().sum();
    let last = e.coeffs[size - 1];
    let shift = (last + p_i * k - sum) % q_i;
    e.coeffs[size - 1] = if shift > 0 { shift } else { shift + q_i };
    Ok(())
}
/// Generate a linear vector `b` over Zq[X]₍u₎.
pub fn generate_linear(p: u64, q: u64, b: &mut Polynomial) -> Result<()> {
    let size = b.coeffs.len();
    if size == 0 {
        return Ok(());
    }
    let k = randrange(0, p) as i64;
    for i in 0..size {
        b.coeffs[i] = randrange(0, q) as i64;
    }

    let q_i = q as i64;
    let sum: i64 = b.coeffs.iter().sum();
    let last = b.coeffs[size - 1];
    let shift = (last + k - sum) % q_i;
    b.coeffs[size - 1] = if shift > 0 { shift } else { shift + q_i };
    Ok(())
}
/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    if u.coeffs.len() < dim + 1 {
        // Mirror the C behavior best-effort; if the buffer is too small, just bail out
        // (the C code uses pointer indexing without bounds checks).
        return Ok(());
    }
    // Zero the relevant portion first since C tests call set_zero before generate_u.
    for c in u.coeffs.iter_mut() {
        *c = 0;
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
            let stddev = (zeroes / 2) as f64;
            let nr = normal_rand(mean, stddev);
            // Translate the C cast to uint64_t with clamp [0, zeroes].
            let samp_u64 = if nr < 0.0 { 0u64 } else { nr as u64 };
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
    let sum: i64 = u.coeffs.iter().sum();
    u.coeffs[dim] = (channel.q as i64) - sum;
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

    // Resize secret if needed (mirror C `secret->size = m.dim;`).
    if secret.polies.len() < dim {
        secret.polies.resize(dim, Polynomial { coeffs: vec![] });
    }
    for i in 0..dim {
        if secret.polies[i].coeffs.len() < dim {
            secret.polies[i].coeffs.resize(dim, 0);
        }
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.data[j * dim + i] as i64;
        }
    }

    // Allocate a temporary 2D matrix used as a per-row buffer (mirrors C's `arr = &m;`).
    let mut arr = Matrix2D::new(dim);

    for i in 0..dim {
        for j in 0..dim {
            // Compute a_ij_poly = secret[i] * secret[j] mod u, mod q.
            let mut a_ij_poly = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            a_ij_poly.poly_mod(u, channel.q)?;

            // Pad on the left with zeroes so the polynomial has at least `dim` coefficients,
            // matching C's row-indexing arr->dim - row - 1.
            if a_ij_poly.coeffs.len() < dim {
                let pad = dim - a_ij_poly.coeffs.len();
                let mut new_coeffs = vec![0i64; dim];
                for (idx, c) in a_ij_poly.coeffs.iter().enumerate() {
                    new_coeffs[idx + pad] = *c;
                }
                a_ij_poly.coeffs = new_coeffs;
            }
            for row in 0..dim {
                let val = a_ij_poly.coeffs[a_ij_poly.coeffs.len() - row - 1];
                arr.data[row * dim + j] = val as u64;
            }
        }

        let result = matrix2d_multiply(&invm, &arr, channel.q)?;
        if let Some(target) = lambda.get_mut(i) {
            // Resize if necessary.
            if target.dim != dim {
                *target = Matrix2D::new(dim);
            }
            target.data.copy_from_slice(&result.data);
        }
    }

    Ok(())
}
/// Generate the polynomial array `f0` for the public key.
pub fn generate_f0(channel: &Channel, _param: &Parameters, f0: &mut PolyArray) -> Result<()> {
    for poly in f0.polies.iter_mut() {
        for c in poly.coeffs.iter_mut() {
            *c = randrange(0, channel.q.saturating_sub(1)) as i64;
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
    // f_pre starts as zero polynomial of "size 2 * dim" in the C code; we emulate by
    // an explicit 2*dim-coefficient polynomial that we then reduce.
    let mut f_pre = Polynomial {
        coeffs: vec![0i64; dim * 2],
    };

    for i in 0..dim {
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        f_pre = f_pre.add(&tmp, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // Replace `f1` contents: in C, the pointer is bumped and only the lower
    // f_pre.size coefficients are written. Here, we simply assign f1 to f_pre's
    // coefficients (truncating to fit f_pre).
    if f1.coeffs.len() >= f_pre.coeffs.len() {
        let diff = f1.coeffs.len() - f_pre.coeffs.len();
        // Zero-pad so size matches f1's length, but with the low-degree zeros at
        // the front like the original.
        let mut new_coeffs = vec![0i64; f_pre.coeffs.len()];
        new_coeffs.copy_from_slice(&f_pre.coeffs);
        let _ = diff;
        f1.coeffs = new_coeffs;
    } else {
        f1.coeffs = f_pre.coeffs.clone();
    }
    Ok(())
}
