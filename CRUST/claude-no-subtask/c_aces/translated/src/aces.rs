use crate::aces_internal::{
    generate_error, generate_f0, generate_f1, generate_linear, generate_secret, generate_u,
    generate_vanisher,
};
use crate::channel::{Channel, Parameters};
use crate::error::{AcesError, Result};
use crate::matrix::Matrix3D;
use crate::polynomial::{Coeff, PolyArray, Polynomial};
/// Represents the public key for the arithmetic channel.
#[derive(Debug, Clone)]
pub struct PublicKey {
    pub u: Polynomial,
    pub lambda: Matrix3D,
}
/// Represents the private key for the arithmetic channel.
#[derive(Debug, Clone)]
pub struct PrivateKey {
    pub x: PolyArray,
    pub f0: PolyArray,
    pub f1: Polynomial,
}
/// Shared information for the arithmetic channel.
#[derive(Debug, Clone)]
pub struct SharedInfo {
    pub channel: Channel,
    pub param: Parameters,
    pub pk: PublicKey,
}
/// An instance of the ACES encryption scheme.
#[derive(Debug, Clone)]
pub struct Aces {
    pub shared_info: SharedInfo,
    pub private_key: PrivateKey,
}
/// A ciphertext in the ACES framework.
#[derive(Debug, Clone)]
pub struct CipherMessage {
    pub c1: PolyArray,
    pub c2: Polynomial,
    pub level: u64,
}
/// Set up the ACES instance (using external memory, for example).
///
/// In the C version this carved an externally-supplied byte buffer into the
/// various polynomials and matrices.  In safe Rust we ignore the buffer and
/// allocate fresh storage of the right shape.
pub fn set_aces(aces: &mut Aces, dim: usize, _mem: &mut [u8]) -> Result<()> {
    // u has dim + 1 coefficients.
    aces.shared_info.pk.u = Polynomial {
        coeffs: vec![0i64; dim + 1],
    };
    // lambda is a Matrix3D with `dim` matrices each of dimension `dim`.
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);
    // x is a PolyArray with `dim` polynomials each of size `dim`.
    aces.private_key.x = PolyArray {
        polies: (0..dim)
            .map(|_| Polynomial {
                coeffs: vec![0i64; dim],
            })
            .collect(),
    };
    // f0 has the same shape.
    aces.private_key.f0 = PolyArray {
        polies: (0..dim)
            .map(|_| Polynomial {
                coeffs: vec![0i64; dim],
            })
            .collect(),
    };
    // f1 is a polynomial of size `dim`.
    aces.private_key.f1 = Polynomial {
        coeffs: vec![0i64; dim],
    };
    aces.shared_info.param.dim = dim as u64;
    Ok(())
}
/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param.dim = dim;
    aces.shared_info.param.N = 1;
    aces.shared_info.channel = Channel::new(p, q, 1);

    // Ensure the storage shape matches `dim`. If not initialized via
    // `set_aces`, allocate now.
    let dim_usize = dim as usize;
    if aces.shared_info.pk.u.coeffs.len() != dim_usize + 1 {
        set_aces(aces, dim_usize, &mut [])?;
    }

    generate_u(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &mut aces.shared_info.pk.u,
    )?;

    let channel = aces.shared_info.channel;
    let param = aces.shared_info.param;
    let u_clone = aces.shared_info.pk.u.clone();
    generate_secret(
        &channel,
        &param,
        &u_clone,
        &mut aces.private_key.x,
        &mut aces.shared_info.pk.lambda,
    )?;
    generate_f0(&channel, &param, &mut aces.private_key.f0)?;
    let f0_clone = aces.private_key.f0.clone();
    let x_clone = aces.private_key.x.clone();
    generate_f1(
        &channel,
        &param,
        &f0_clone,
        &x_clone,
        &u_clone,
        &mut aces.private_key.f1,
    )?;
    Ok(())
}
/// Encrypt a message.
pub fn aces_encrypt(aces: &Aces, message: &[u64], result: &mut CipherMessage) -> Result<()> {
    if message.len() > 1 {
        return Err(AcesError::GenericError(
            "only single-element messages are supported".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let channel = aces.shared_info.channel;
    let info_pk_u = &aces.shared_info.pk.u;

    let mut r_m = Polynomial {
        coeffs: vec![0i64; dim],
    };
    let mut e = Polynomial {
        coeffs: vec![0i64; dim],
    };
    let mut b = Polynomial {
        coeffs: vec![0i64; dim],
    };

    generate_error(channel.q, message[0], &mut r_m)?;
    generate_vanisher(channel.p, channel.q, &mut e)?;
    generate_linear(channel.p, channel.q, &mut b)?;

    // C1
    for i in 0..dim {
        let mut tmp = b.mul(&aces.private_key.f0.polies[i], channel.q)?;
        // Pad to size 2*dim to mirror C buffer sizing.
        if tmp.coeffs.len() < 2 * dim {
            let pad = 2 * dim - tmp.coeffs.len();
            let mut padded = vec![0i64; 2 * dim];
            for (idx, c) in tmp.coeffs.iter().enumerate() {
                padded[pad + idx] = *c;
            }
            tmp.coeffs = padded;
        }
        tmp.poly_mod(info_pk_u, channel.q)?;

        // memcpy(result->c1.polies[i].coeffs, tmp.coeffs, tmp.size * sizeof(Coeff));
        let dst = &mut result.c1.polies[i].coeffs;
        let copy_len = std::cmp::min(dst.len(), tmp.coeffs.len());
        for k in 0..copy_len {
            dst[k] = tmp.coeffs[k];
        }
    }

    // C2 = (f1 + e) * b + r_m, mod u, mod q
    let f1_plus_e = aces.private_key.f1.add(&e, channel.q)?;
    // Store this into result.c2 first as in the C code, then overwrite.
    let mut tmp = f1_plus_e.mul(&b, channel.q)?;
    if tmp.coeffs.len() < 2 * dim {
        let pad = 2 * dim - tmp.coeffs.len();
        let mut padded = vec![0i64; 2 * dim];
        for (idx, c) in tmp.coeffs.iter().enumerate() {
            padded[pad + idx] = *c;
        }
        tmp.coeffs = padded;
    }
    tmp = tmp.add(&r_m, channel.q)?;
    tmp.poly_mod(info_pk_u, channel.q)?;

    // memcpy(result->c2.coeffs, tmp.coeffs, tmp.size * sizeof(Coeff));
    let copy_len = std::cmp::min(result.c2.coeffs.len(), tmp.coeffs.len());
    for k in 0..copy_len {
        result.c2.coeffs[k] = tmp.coeffs[k];
    }

    result.level = channel.p;
    Ok(())
}
/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() > 1 {
        return Err(AcesError::GenericError(
            "only single-element messages are supported".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let channel = aces.shared_info.channel;

    let mut c0_t_x = Polynomial {
        coeffs: vec![0i64; 2 * dim],
    };

    for i in 0..dim {
        let mut tmp = message.c1.polies[i].mul(&aces.private_key.x.polies[i], channel.q)?;
        if tmp.coeffs.len() < 2 * dim {
            let pad = 2 * dim - tmp.coeffs.len();
            let mut padded = vec![0i64; 2 * dim];
            for (idx, c) in tmp.coeffs.iter().enumerate() {
                padded[pad + idx] = *c;
            }
            tmp.coeffs = padded;
        }
        c0_t_x = tmp.add(&c0_t_x, channel.q)?;
    }

    c0_t_x.fit(channel.q)?;
    let diff = message.c2.sub(&c0_t_x, channel.q)?;
    c0_t_x = diff;

    let qm = channel.q as i64;
    let pm = channel.p as i64;
    let s = c0_t_x.coef_sum() % qm;
    // Convert to unsigned, accounting for negative values.
    let s_u: u64 = if s < 0 {
        ((s + qm) as u64) % channel.q
    } else {
        (s as u64) % channel.q
    } % channel.p;
    let _ = (pm,); // silence unused
    if !result.is_empty() {
        result[0] = s_u;
    }
    Ok(())
}
/// Perform homomorphic addition on two ciphertexts.
pub fn aces_add(
    a: &CipherMessage,
    b: &CipherMessage,
    info: &SharedInfo,
    result: &mut CipherMessage,
) -> Result<()> {
    let dim = info.param.dim as usize;
    let q = info.channel.q;
    for i in 0..dim {
        let mut sum = a.c1.polies[i].add(&b.c1.polies[i], q)?;
        sum.poly_mod(&info.pk.u, q)?;
        let dst = &mut result.c1.polies[i].coeffs;
        let copy_len = std::cmp::min(dst.len(), sum.coeffs.len());
        // Reset dst then copy.
        for v in dst.iter_mut() {
            *v = 0;
        }
        let dst_len = dst.len();
        for k in 0..copy_len {
            // Right-align if the resulting polynomial is smaller.
            let dst_idx = dst_len - copy_len + k;
            let _ = dst_idx; // mirror C: copy starting at index 0
            dst[k] = sum.coeffs[k];
        }
    }
    let mut sum_c2 = a.c2.add(&b.c2, q)?;
    sum_c2.poly_mod(&info.pk.u, q)?;
    for v in result.c2.coeffs.iter_mut() {
        *v = 0;
    }
    let copy_len = std::cmp::min(result.c2.coeffs.len(), sum_c2.coeffs.len());
    for k in 0..copy_len {
        result.c2.coeffs[k] = sum_c2.coeffs[k];
    }
    result.level = a.level + b.level;
    Ok(())
}
/// Perform homomorphic multiplication on two ciphertexts.
pub fn aces_mul(
    _a: &CipherMessage,
    _b: &CipherMessage,
    _info: &SharedInfo,
    _result: &mut CipherMessage,
) -> Result<()> {
    // The C reference also leaves this as a stub (TODO). We mirror that behavior
    // with a no-op success result.
    Ok(())
}
/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    // poly_sub_scaler(&a->c2, k * info->channel.p, &a->c2, info->channel.q);
    let scaler = level.wrapping_mul(info.channel.p);
    let new_c2 = message.c2.sub_scaler(scaler, info.channel.q)?;
    message.c2 = new_c2;
    message.level = message.level.wrapping_sub(level);
    let _: Coeff = 0;
    Ok(())
}
