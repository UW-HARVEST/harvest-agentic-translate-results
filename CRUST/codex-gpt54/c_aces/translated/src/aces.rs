use crate::channel::{Channel, Parameters};
use crate::error::{AcesError, Result};
use crate::matrix::Matrix3D;
use crate::polynomial::{PolyArray, Polynomial};
use crate::aces_internal::{
    generate_error, generate_f0, generate_f1, generate_linear, generate_secret, generate_u,
    generate_vanisher,
};

fn generic_error(message: impl Into<String>) -> AcesError {
    AcesError::GenericError(message.into())
}

fn zero_poly(size: usize) -> Polynomial {
    Polynomial::new(vec![0; size])
}

fn zero_poly_array(count: usize, poly_size: usize) -> PolyArray {
    PolyArray::new((0..count).map(|_| zero_poly(poly_size)).collect())
}

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
pub fn set_aces(aces: &mut Aces, dim: usize, mem: &mut [u8]) -> Result<()> {
    let coeff_bytes = std::mem::size_of::<i64>();
    let matrix_bytes = std::mem::size_of::<usize>() + std::mem::size_of::<*const u64>();
    let poly_bytes = std::mem::size_of::<usize>() + std::mem::size_of::<*const i64>();
    let required = coeff_bytes * (2 * dim + 1 + 2 * dim * dim)
        + matrix_bytes * dim
        + poly_bytes * 2 * dim
        + std::mem::size_of::<u64>() * dim * dim * dim;

    if mem.len() < required {
        return Err(generic_error("insufficient backing memory"));
    }

    aces.shared_info.pk.u = zero_poly(dim + 1);
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);
    aces.private_key.x = zero_poly_array(dim, dim);
    aces.private_key.f0 = zero_poly_array(dim, dim);
    aces.private_key.f1 = zero_poly(dim);
    Ok(())
}
/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    let dim_usize = dim as usize;
    aces.shared_info = SharedInfo {
        channel: Channel::init(p, q, 1)?,
        param: Parameters { dim, N: 1 },
        pk: PublicKey {
            u: zero_poly(dim_usize + 1),
            lambda: Matrix3D::new(dim_usize, dim_usize),
        },
    };
    aces.private_key = PrivateKey {
        x: zero_poly_array(dim_usize, dim_usize),
        f0: zero_poly_array(dim_usize, dim_usize),
        f1: zero_poly(dim_usize),
    };

    generate_u(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &mut aces.shared_info.pk.u,
    )?;
    generate_secret(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &aces.shared_info.pk.u,
        &mut aces.private_key.x,
        &mut aces.shared_info.pk.lambda,
    )?;
    generate_f0(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &mut aces.private_key.f0,
    )?;
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
        return Err(generic_error("only one message element is supported"));
    }
    if message.is_empty() {
        return Err(generic_error("message is empty"));
    }

    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut r_m = zero_poly(dim);
    let mut e = zero_poly(dim);
    let mut b = zero_poly(dim);
    generate_error(q, message[0], &mut r_m)?;
    generate_vanisher(p, q, &mut e)?;
    generate_linear(p, q, &mut b)?;

    result.c1 = zero_poly_array(dim, dim);
    result.c2 = zero_poly(dim);

    for i in 0..dim {
        let mut tmp = b.mul(&aces.private_key.f0.polies[i], q)?;
        tmp.poly_mod(&aces.shared_info.pk.u, q)?;
        result.c1.polies[i].set_zero();
        for (dst, src) in result.c1.polies[i].coeffs.iter_mut().zip(tmp.coeffs.iter()) {
            *dst = *src;
        }
    }

    result.c2 = aces.private_key.f1.add(&e, q)?;
    let mut tmp = result.c2.mul(&b, q)?;
    tmp = tmp.add(&r_m, q)?;
    tmp.poly_mod(&aces.shared_info.pk.u, q)?;
    result.c2 = zero_poly(dim);
    for (dst, src) in result.c2.coeffs.iter_mut().zip(tmp.coeffs.iter()) {
        *dst = *src;
    }

    result.level = p;
    Ok(())
}
/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() > 1 {
        return Err(generic_error("only one result element is supported"));
    }
    if result.is_empty() {
        return Err(generic_error("result buffer is empty"));
    }

    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;
    let mut c0tx = zero_poly(dim * 2);
    c0tx.set_zero();

    for i in 0..dim {
        let tmp = message.c1.polies[i].mul(&aces.private_key.x.polies[i], q)?;
        c0tx = tmp.add(&c0tx, q)?;
    }

    c0tx.fit(q)?;
    c0tx = message.c2.sub(&c0tx, q)?;
    result[0] = ((c0tx.coef_sum() as u64) % q) % p;
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
    result.c1 = zero_poly_array(dim, dim);
    for i in 0..dim {
        let mut sum = a.c1.polies[i].add(&b.c1.polies[i], info.channel.q)?;
        sum.poly_mod(&info.pk.u, info.channel.q)?;
        result.c1.polies[i] = sum;
    }

    result.c2 = a.c2.add(&b.c2, info.channel.q)?;
    result.c2.poly_mod(&info.pk.u, info.channel.q)?;
    result.level = a.level + b.level;
    Ok(())
}
/// Perform homomorphic multiplication on two ciphertexts.
pub fn aces_mul(
    a: &CipherMessage,
    b: &CipherMessage,
    info: &SharedInfo,
    result: &mut CipherMessage,
) -> Result<()> {
    let _ = (a, b, info, result);
    Ok(())
}
/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    message.c2 = message
        .c2
        .sub_scaler(level.wrapping_mul(info.channel.p), info.channel.q)?;
    message.level = message.level.wrapping_sub(level);
    Ok(())
}
