use crate::aces_internal::{
    generate_error, generate_f0, generate_f1, generate_linear, generate_secret, generate_u,
    generate_vanisher,
};
use crate::channel::{Channel, Parameters};
use crate::error::Result;
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

impl Aces {
    /// Create a new Aces instance with allocated buffers for the given dimension.
    pub fn allocate(dim: usize) -> Self {
        let u = Polynomial::new(vec![0; dim + 1]);
        let lambda = Matrix3D::new(dim, dim);
        let x = PolyArray::new(vec![Polynomial::new(vec![0; dim]); dim]);
        let f0 = PolyArray::new(vec![Polynomial::new(vec![0; dim]); dim]);
        let f1 = Polynomial::new(vec![0; dim]);
        Aces {
            shared_info: SharedInfo {
                channel: Channel { p: 0, q: 0, w: 0 },
                param: Parameters { dim: 0, N: 0 },
                pk: PublicKey { u, lambda },
            },
            private_key: PrivateKey { x, f0, f1 },
        }
    }
}

/// Set up the ACES instance (using external memory, for example).
pub fn set_aces(aces: &mut Aces, dim: usize, _mem: &mut [u8]) -> Result<()> {
    // In Rust we don't need to use external memory; just allocate buffers.
    aces.shared_info.pk.u = Polynomial::new(vec![0; dim + 1]);
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);
    aces.private_key.x = PolyArray::new(vec![Polynomial::new(vec![0; dim]); dim]);
    aces.private_key.f0 = PolyArray::new(vec![Polynomial::new(vec![0; dim]); dim]);
    aces.private_key.f1 = Polynomial::new(vec![0; dim]);
    Ok(())
}
/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param.dim = dim;
    aces.shared_info.param.N = 1;

    // Ensure buffers exist for the given dim.
    let dim_us = dim as usize;
    aces.shared_info.pk.u = Polynomial::new(vec![0; dim_us + 1]);
    aces.shared_info.pk.lambda = Matrix3D::new(dim_us, dim_us);
    aces.private_key.x = PolyArray::new(vec![Polynomial::new(vec![0; dim_us]); dim_us]);
    aces.private_key.f0 = PolyArray::new(vec![Polynomial::new(vec![0; dim_us]); dim_us]);
    aces.private_key.f1 = Polynomial::new(vec![0; dim_us]);

    aces.shared_info.channel = Channel::new(p, q, 1);

    let channel = aces.shared_info.channel;
    let param = aces.shared_info.param;
    generate_u(&channel, &param, &mut aces.shared_info.pk.u)?;
    generate_secret(
        &channel,
        &param,
        &aces.shared_info.pk.u,
        &mut aces.private_key.x,
        &mut aces.shared_info.pk.lambda,
    )?;
    generate_f0(&channel, &param, &mut aces.private_key.f0)?;

    let u_clone = aces.shared_info.pk.u.clone();
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
        return Err(crate::error::AcesError::GenericError(
            "size > 1 not supported".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut r_m = Polynomial::new(vec![0i64; dim]);
    let mut e = Polynomial::new(vec![0i64; dim]);
    let mut b = Polynomial::new(vec![0i64; dim]);

    generate_error(q, message[0], &mut r_m)?;
    generate_vanisher(p, q, &mut e)?;
    generate_linear(p, q, &mut b)?;

    // Ensure result.c1 has dim entries
    if result.c1.polies.len() < dim {
        result
            .c1
            .polies
            .resize(dim, Polynomial::new(vec![0i64; dim]));
    }

    // C1
    for i in 0..dim {
        let mut tmp = b.mul(&aces.private_key.f0.polies[i], q)?;
        tmp.poly_mod(&aces.shared_info.pk.u, q)?;
        // memcpy result->c1.polies[i].coeffs, tmp.coeffs
        result.c1.polies[i].coeffs.resize(tmp.coeffs.len(), 0);
        for j in 0..tmp.coeffs.len() {
            result.c1.polies[i].coeffs[j] = tmp.coeffs[j];
        }
    }

    // C2
    // result.c2 = aces.private_key.f1 + e (in C, it's stored into result->c2 first)
    let mut c2 = aces.private_key.f1.add(&e, q)?;
    let mut tmp = c2.mul(&b, q)?;
    tmp = tmp.add(&r_m, q)?;
    tmp.poly_mod(&aces.shared_info.pk.u, q)?;
    // memcpy result->c2.coeffs, tmp.coeffs
    c2.coeffs = tmp.coeffs;
    result.c2 = c2;

    result.level = p;
    Ok(())
}
/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() > 1 {
        return Err(crate::error::AcesError::GenericError(
            "size > 1 not supported".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut c0t_x = Polynomial::new(vec![0i64; 2 * dim]);

    for i in 0..dim {
        let prod = message.c1.polies[i].mul(&aces.private_key.x.polies[i], q)?;
        // pad prod to size 2*dim
        let target_len = 2 * dim;
        let prod_padded = if prod.coeffs.len() < target_len {
            let pad = target_len - prod.coeffs.len();
            let mut v = vec![0i64; pad];
            v.extend_from_slice(&prod.coeffs);
            Polynomial::new(v)
        } else {
            prod
        };
        c0t_x = prod_padded.add(&c0t_x, q)?;
    }

    c0t_x.fit(q)?;
    c0t_x = message.c2.sub(&c0t_x, q)?;

    let csum: i64 = c0t_x.coef_sum();
    let q_i = q as i64;
    let p_i = p as i64;
    // (csum % q) % p, in C with possibly negative csum.
    let res = ((csum % q_i) % p_i + p_i) % p_i;
    result[0] = res as u64;
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

    // ensure result.c1 has dim entries
    if result.c1.polies.len() < dim {
        result
            .c1
            .polies
            .resize(dim, Polynomial::new(vec![0i64; dim]));
    }

    for i in 0..dim {
        let mut sum = a.c1.polies[i].add(&b.c1.polies[i], q)?;
        sum.poly_mod(&info.pk.u, q)?;
        result.c1.polies[i] = sum;
    }
    let mut c2 = a.c2.add(&b.c2, q)?;
    c2.poly_mod(&info.pk.u, q)?;
    result.c2 = c2;
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
    // C version is a stub that returns 0 without computing anything.
    Ok(())
}
/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    let q = info.channel.q;
    let p = info.channel.p;
    let scaler = level.wrapping_mul(p);
    let new_c2 = message.c2.sub_scaler(scaler, q)?;
    message.c2 = new_c2;
    message.level = message.level.wrapping_sub(level);
    let _ = q; // keep
    let _: Coeff = 0; // type sanity
    Ok(())
}
