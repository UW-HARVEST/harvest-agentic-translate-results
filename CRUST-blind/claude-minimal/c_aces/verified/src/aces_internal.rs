use crate::channel::{Channel, Parameters};
use crate::common::{clamp, normal_rand, randrange};
use crate::error::Result;
use crate::matrix::{fill_random_invertible_pairs, Matrix2D, Matrix3D};
use crate::polynomial::{Coeff, PolyArray, Polynomial};

pub const EPROB: f64 = 0.5;

/// Generate an error element `rm` over Zq[X]₍u₎.
pub fn generate_error(q: u64, message: u64, rm: &mut Polynomial) -> Result<()> {
    if rm.coeffs.is_empty() {
        return Ok(());
    }
    for i in 0..rm.coeffs.len() {
        rm.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last = rm.coeffs.len() - 1;
    let sum = rm.coef_sum();
    let shift = (rm.coeffs[last] + (message as Coeff) - sum) % (q as Coeff);
    rm.coeffs[last] = if shift > 0 { shift } else { shift + q as Coeff };
    Ok(())
}
/// Generate a vanisher vector `e` over Zq[X]₍u₎.
pub fn generate_vanisher(p: u64, q: u64, e: &mut Polynomial) -> Result<()> {
    if e.coeffs.is_empty() {
        return Ok(());
    }
    let k: i64 = if (randrange(0, 1) as f64) < EPROB { 0 } else { 1 };
    for i in 0..e.coeffs.len() {
        e.coeffs[i] = randrange(0, q) as Coeff;
    }
    let last = e.coeffs.len() - 1;
    let sum = e.coef_sum();
    let shift = (e.coeffs[last] + (p as Coeff) * k - sum) % (q as Coeff);
    e.coeffs[last] = if shift > 0 { shift } else { shift + q as Coeff };
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
    let last = b.coeffs.len() - 1;
    let sum = b.coef_sum();
    let shift = (b.coeffs[last] + k - sum) % (q as Coeff);
    b.coeffs[last] = if shift > 0 { shift } else { shift + q as Coeff };
    Ok(())
}
/// Generate the polynomial `u` for the arithmetic channel.
pub fn generate_u(channel: &Channel, param: &Parameters, u: &mut Polynomial) -> Result<()> {
    let dim = param.dim as usize;
    if u.coeffs.len() < dim + 1 {
        u.coeffs.resize(dim + 1, 0);
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
            let samp = clamp(0, zeroes, normal_rand((zeroes / 2) as f64, (zeroes / 2) as f64) as u64);
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

    // Build secret polynomial array from columns of m.
    if secret.polies.len() < dim {
        secret.polies.resize(
            dim,
            Polynomial {
                coeffs: vec![0 as Coeff; dim],
            },
        );
    }
    for i in 0..dim {
        if secret.polies[i].coeffs.len() < dim {
            secret.polies[i].coeffs.resize(dim, 0);
        }
        for j in 0..dim {
            secret.polies[i].coeffs[j] = m.data[j * dim + i] as Coeff;
        }
    }

    // arr = &m, then for each i, build a_ij polynomial = secret[i] * secret[j] mod u (mod q)
    // and store its coefficients into a temp matrix arr (= m), then lambda[i] = invm * arr.
    let mut arr = Matrix2D::new(dim);

    for i in 0..dim {
        for j in 0..dim {
            let mut a_ij = secret.polies[i].mul(&secret.polies[j], channel.q)?;
            // Pad/extend to size 2*dim if needed for poly_mod stability?
            // The C version allocates a fresh polynomial of size 2*secret->size and
            // mul writes into it; mul output size after our `fit` is appropriate.
            a_ij.poly_mod(u, channel.q)?;

            // Make sure a_ij has at least `dim` coefficients to read from
            // Pad on the LEFT with zeros so coefficient layout matches expected.
            if a_ij.coeffs.len() < dim {
                let pad = dim - a_ij.coeffs.len();
                let mut new_coeffs = vec![0 as Coeff; dim];
                for (k, c) in a_ij.coeffs.iter().enumerate() {
                    new_coeffs[k + pad] = *c;
                }
                a_ij.coeffs = new_coeffs;
            }

            for row in 0..dim {
                arr.data[row * dim + j] = a_ij.coeffs[dim - row - 1] as u64;
            }
        }

        let lam_i = matrix_multiply_mod(&invm, &arr, channel.q)?;
        if let Some(target) = lambda.get_mut(i) {
            target.dim = lam_i.dim;
            target.data = lam_i.data;
        }
    }

    Ok(())
}

fn matrix_multiply_mod(m: &Matrix2D, invm: &Matrix2D, modulus: u64) -> Result<Matrix2D> {
    crate::matrix::matrix2d_multiply(m, invm, modulus)
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
    let mut f_pre = Polynomial {
        coeffs: vec![0 as Coeff; dim * 2],
    };

    for i in 0..dim {
        let tmp = f0.polies[i].mul(&x.polies[i], channel.q)?;
        f_pre = f_pre.add(&tmp, channel.q)?;
    }
    f_pre.poly_mod(u, channel.q)?;

    // Copy f_pre (which has been trimmed by fit) into f1.
    f1.coeffs = f_pre.coeffs;
    Ok(())
}
