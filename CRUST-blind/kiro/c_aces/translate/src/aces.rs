use crate::aces_internal::*;
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
/// Set up the ACES instance.
pub fn set_aces(aces: &mut Aces, dim: usize, _mem: &mut [u8]) -> Result<()> {
    // In Rust we use Vec-based allocation, so we just initialize the structures
    // with the right dimensions. The C code uses a flat memory buffer; we don't need that.
    let required = std::mem::size_of::<Coeff>() * (2 * dim + 1 + 2 * dim * dim)
        + std::mem::size_of::<Polynomial>() * 2 * dim
        + std::mem::size_of::<u64>() * dim * dim * dim;
    if _mem.len() < required {
        return Err(AcesError::GenericError("insufficient memory".into()));
    }

    aces.shared_info.pk.u = Polynomial::new(vec![0i64; dim + 1]);
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);
    aces.private_key.x = PolyArray::new((0..dim).map(|_| Polynomial::new(vec![0i64; dim])).collect());
    aces.private_key.f0 = PolyArray::new((0..dim).map(|_| Polynomial::new(vec![0i64; dim])).collect());
    aces.private_key.f1 = Polynomial::new(vec![0i64; dim]);
    Ok(())
}
/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param.dim = dim;
    aces.shared_info.param.N = 1;
    aces.shared_info.channel = Channel::init(p, q, 1)?;
    generate_u(&aces.shared_info.channel, &aces.shared_info.param, &mut aces.shared_info.pk.u)?;
    generate_secret(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &aces.shared_info.pk.u,
        &mut aces.private_key.x,
        &mut aces.shared_info.pk.lambda,
    )?;
    generate_f0(&aces.shared_info.channel, &aces.shared_info.param, &mut aces.private_key.f0)?;
    generate_f1(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &aces.private_key.f0,
        &aces.private_key.x,
        &aces.shared_info.pk.u,
        &mut aces.private_key.f1,
    )?;
    Ok(())
}
/// Encrypt a message.
pub fn aces_encrypt(aces: &Aces, message: &[u64], result: &mut CipherMessage) -> Result<()> {
    if message.len() > 1 {
        return Err(AcesError::GenericError("size > 1".into()));
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
    result.c1.polies = Vec::with_capacity(dim);
    for i in 0..dim {
        let mut tmp = b.mul(&aces.private_key.f0.polies[i], q)?;
        tmp.poly_mod(&aces.shared_info.pk.u, q)?;
        // Pad/truncate to dim size
        let mut coeffs = vec![0i64; dim];
        let src_len = tmp.coeffs.len().min(dim);
        let offset = dim - src_len;
        coeffs[offset..].copy_from_slice(&tmp.coeffs[..src_len]);
        result.c1.polies.push(Polynomial::new(coeffs));
    }

    // C2
    let f1_plus_e = aces.private_key.f1.add(&e, q)?;
    let mut tmp = f1_plus_e.mul(&b, q)?;
    tmp = tmp.add(&r_m, q)?;
    tmp.poly_mod(&aces.shared_info.pk.u, q)?;
    let mut coeffs = vec![0i64; dim];
    let src_len = tmp.coeffs.len().min(dim);
    let offset = dim - src_len;
    coeffs[offset..].copy_from_slice(&tmp.coeffs[..src_len]);
    result.c2 = Polynomial::new(coeffs);

    result.level = p;
    Ok(())
}
/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() > 1 {
        return Err(AcesError::GenericError("size > 1".into()));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut c0tx = Polynomial::new(vec![0i64; dim * 2]);

    for i in 0..dim {
        let tmp = message.c1.polies[i].mul(&aces.private_key.x.polies[i], q)?;
        c0tx = tmp.add(&c0tx, q)?;
    }

    c0tx.fit(q)?;
    c0tx = message.c2.sub(&c0tx, q)?;

    result[0] = ((c0tx.coef_sum() % q as i64) % p as i64) as u64;
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

    result.c1.polies = Vec::with_capacity(dim);
    for i in 0..dim {
        let mut p = a.c1.polies[i].add(&b.c1.polies[i], q)?;
        p.poly_mod(&info.pk.u, q)?;
        result.c1.polies.push(p);
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
    // C code: TODO: Implement it (returns 0 with no-op)
    Ok(())
}
/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    let q = info.channel.q;
    let p = info.channel.p;
    message.c2 = message.c2.sub_scaler(level * p, q)?;
    message.level -= level;
    Ok(())
}
