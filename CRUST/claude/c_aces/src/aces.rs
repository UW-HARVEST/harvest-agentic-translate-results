use crate::aces_internal::{
    generate_error, generate_f0, generate_f1, generate_linear, generate_secret, generate_u,
    generate_vanisher,
};
use crate::channel::{Channel, Parameters};
use crate::error::Result;
use crate::matrix::{Matrix2D, Matrix3D};
use crate::polynomial::{PolyArray, Polynomial};

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
/// Set up the ACES instance using a fixed dimension.
///
/// In Rust we manage memory natively rather than using an external buffer, so
/// this allocates the appropriate sized polynomials and matrices.
pub fn set_aces(aces: &mut Aces, dim: usize, _mem: &mut [u8]) -> Result<()> {
    // u is dim+1 long
    aces.shared_info.pk.u = Polynomial::new(vec![0i64; dim + 1]);
    // lambda is dim matrices of dimension dim
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);
    // x: dim polynomials each of length dim
    aces.private_key.x = PolyArray::new(
        (0..dim)
            .map(|_| Polynomial::new(vec![0i64; dim]))
            .collect(),
    );
    // f0: dim polynomials of length dim
    aces.private_key.f0 = PolyArray::new(
        (0..dim)
            .map(|_| Polynomial::new(vec![0i64; dim]))
            .collect(),
    );
    // f1: a polynomial of length dim
    aces.private_key.f1 = Polynomial::new(vec![0i64; dim]);
    Ok(())
}
/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param.dim = dim;
    aces.shared_info.param.N = 1;
    aces.shared_info.channel = Channel::init(p, q, 1)?;

    // Allocate the storage for polynomials/matrices.
    let mut buf: Vec<u8> = Vec::new();
    set_aces(aces, dim as usize, &mut buf)?;

    let channel = aces.shared_info.channel;
    let param = aces.shared_info.param;
    generate_u(&channel, &param, &mut aces.shared_info.pk.u)?;

    // Take temporary ownership of u, x, lambda for the call.
    // We need to call generate_secret(channel, param, u, &mut secret, &mut lambda)
    // u is borrowed immutably, but x and lambda are mutable. We need to split borrows.
    // Use std::mem::take to swap out and back.
    let mut x = std::mem::replace(&mut aces.private_key.x, PolyArray::new(Vec::new()));
    let mut lambda = std::mem::replace(
        &mut aces.shared_info.pk.lambda,
        Matrix3D { data: Vec::new() },
    );
    generate_secret(&channel, &param, &aces.shared_info.pk.u, &mut x, &mut lambda)?;
    aces.private_key.x = x;
    aces.shared_info.pk.lambda = lambda;

    generate_f0(&channel, &param, &mut aces.private_key.f0)?;
    let mut f1 = std::mem::replace(&mut aces.private_key.f1, Polynomial::new(Vec::new()));
    generate_f1(
        &channel,
        &param,
        &aces.private_key.f0,
        &aces.private_key.x,
        &aces.shared_info.pk.u,
        &mut f1,
    )?;
    aces.private_key.f1 = f1;
    Ok(())
}
/// Encrypt a message.
pub fn aces_encrypt(aces: &Aces, message: &[u64], result: &mut CipherMessage) -> Result<()> {
    if message.len() > 1 {
        return Err(crate::error::AcesError::GenericError("size > 1".into()));
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

    // C1
    for i in 0..dim {
        let mut tmp = b.mul(&aces.private_key.f0.polies[i], q)?;
        tmp.poly_mod(&aces.shared_info.pk.u, q)?;
        // Pad/truncate to fit the result c1[i] size (which is dim)
        let target_len = result.c1.polies[i].coeffs.len();
        if tmp.coeffs.len() < target_len {
            let need = target_len - tmp.coeffs.len();
            let mut padded = vec![0i64; need];
            padded.extend_from_slice(&tmp.coeffs);
            result.c1.polies[i].coeffs = padded;
        } else {
            // Take last `target_len` if larger
            let start = tmp.coeffs.len() - target_len;
            result.c1.polies[i].coeffs.copy_from_slice(&tmp.coeffs[start..]);
        }
    }

    // C2: c2 = (f1 + e); tmp = c2 * b; tmp += r_m; tmp mod u; copy tmp -> c2
    let c2 = aces.private_key.f1.add(&e, q)?;
    let mut tmp = c2.mul(&b, q)?;
    tmp = tmp.add(&r_m, q)?;
    tmp.poly_mod(&aces.shared_info.pk.u, q)?;

    // Make the c2 result match tmp's size (or the target's)
    let target_len = result.c2.coeffs.len();
    if tmp.coeffs.len() < target_len {
        let need = target_len - tmp.coeffs.len();
        let mut padded = vec![0i64; need];
        padded.extend_from_slice(&tmp.coeffs);
        result.c2.coeffs = padded;
    } else {
        let start = tmp.coeffs.len() - target_len;
        result.c2.coeffs.copy_from_slice(&tmp.coeffs[start..]);
    }
    result.level = p;
    Ok(())
}
/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() > 1 {
        return Err(crate::error::AcesError::GenericError("size > 1".into()));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut c0_tx = Polynomial::new(vec![0i64; 2 * dim]);

    for i in 0..dim {
        let tmp = message.c1.polies[i].mul(&aces.private_key.x.polies[i], q)?;
        c0_tx = c0_tx.add(&tmp, q)?;
    }

    c0_tx.fit(q)?;
    let c0_tx = message.c2.sub(&c0_tx, q)?;
    let s = c0_tx.coef_sum();
    let q_i = q as i64;
    let p_i = p as i64;
    let v = s.rem_euclid(q_i).rem_euclid(p_i);
    result[0] = v as u64;
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
    for i in 0..dim {
        let mut sum = a.c1.polies[i].add(&b.c1.polies[i], info.channel.q)?;
        sum.poly_mod(&info.pk.u, info.channel.q)?;
        result.c1.polies[i] = sum;
    }
    let mut c2 = a.c2.add(&b.c2, info.channel.q)?;
    c2.poly_mod(&info.pk.u, info.channel.q)?;
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
    // C version is not implemented (TODO marked); keep behavior identical.
    Ok(())
}
/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    let new_c2 = message.c2.sub_scaler(level * info.channel.p, info.channel.q)?;
    message.c2 = new_c2;
    message.level -= level;
    Ok(())
}

// Silence unused warning for Matrix2D import (used elsewhere)
#[allow(dead_code)]
fn _unused_matrix(_m: Matrix2D) {}
