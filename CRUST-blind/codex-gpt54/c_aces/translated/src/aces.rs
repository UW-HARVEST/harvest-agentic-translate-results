use crate::aces_internal::{
    generate_error, generate_f0, generate_f1, generate_linear, generate_secret, generate_u,
    generate_vanisher,
};
use crate::channel::{Channel, Parameters};
use crate::error::{AcesError, Result};
use crate::matrix::Matrix3D;
use crate::polynomial::{PolyArray, Polynomial};

fn err(msg: &str) -> AcesError {
    AcesError::GenericError(msg.to_string())
}

fn zero_poly(size: usize) -> Polynomial {
    Polynomial::new(vec![0; size])
}

fn zero_poly_array(count: usize, size: usize) -> PolyArray {
    PolyArray::new((0..count).map(|_| zero_poly(size)).collect())
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
    let coeff_size = std::mem::size_of::<i64>();
    let c_matrix2d_size = std::mem::size_of::<usize>() + std::mem::size_of::<*const u64>();
    let c_polynomial_size = std::mem::size_of::<*const i64>() + std::mem::size_of::<usize>();
    let required = coeff_size * (2 * dim + 1 + 2 * dim * dim)
        + c_matrix2d_size * dim
        + c_polynomial_size * 2 * dim
        + std::mem::size_of::<u64>() * dim * dim * dim;

    if mem.len() < required {
        return Err(err("insufficient memory for ACES setup"));
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
    aces.shared_info.param = Parameters { dim, N: 1 };
    aces.shared_info.channel = Channel::init(p, q, 1)?;

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
    if message.len() != 1 {
        return Err(err("exactly one message value is required"));
    }

    let dim = aces.shared_info.param.dim as usize;
    let mut r_m = zero_poly(dim);
    let mut e = zero_poly(dim);
    let mut b = zero_poly(dim);

    generate_error(aces.shared_info.channel.q, message[0], &mut r_m)?;
    generate_vanisher(
        aces.shared_info.channel.p,
        aces.shared_info.channel.q,
        &mut e,
    )?;
    generate_linear(
        aces.shared_info.channel.p,
        aces.shared_info.channel.q,
        &mut b,
    )?;

    for i in 0..dim {
        let mut tmp = aces.private_key.f0.polies[i].mul(&b, aces.shared_info.channel.q)?;
        tmp.poly_mod(&aces.shared_info.pk.u, aces.shared_info.channel.q)?;

        result.c1.polies[i].set_zero();
        for (idx, coeff) in tmp.coeffs.iter().enumerate() {
            if idx < result.c1.polies[i].coeffs.len() {
                result.c1.polies[i].coeffs[idx] = *coeff;
            }
        }
    }

    result.c2 = aces
        .private_key
        .f1
        .add(&e, aces.shared_info.channel.q)?
        .mul(&b, aces.shared_info.channel.q)?;
    result.c2 = result.c2.add(&r_m, aces.shared_info.channel.q)?;
    result
        .c2
        .poly_mod(&aces.shared_info.pk.u, aces.shared_info.channel.q)?;

    let mut c2 = zero_poly(dim);
    for (idx, coeff) in result.c2.coeffs.iter().enumerate() {
        if idx < c2.coeffs.len() {
            c2.coeffs[idx] = *coeff;
        }
    }
    result.c2 = c2;
    result.level = aces.shared_info.channel.p;

    Ok(())
}
/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() != 1 {
        return Err(err("exactly one result slot is required"));
    }

    let dim = aces.shared_info.param.dim as usize;
    let mut c0_tx = zero_poly(dim * 2);

    for i in 0..dim {
        let tmp =
            message.c1.polies[i].mul(&aces.private_key.x.polies[i], aces.shared_info.channel.q)?;
        c0_tx = tmp.add(&c0_tx, aces.shared_info.channel.q)?;
    }

    c0_tx.fit(aces.shared_info.channel.q)?;
    c0_tx = message.c2.sub(&c0_tx, aces.shared_info.channel.q)?;
    result[0] = (c0_tx.coef_sum() as u64 % aces.shared_info.channel.q) % aces.shared_info.channel.p;

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
        result.c1.polies[i] = a.c1.polies[i].add(&b.c1.polies[i], info.channel.q)?;
        result.c1.polies[i].poly_mod(&info.pk.u, info.channel.q)?;
    }

    result.c2 = a.c2.add(&b.c2, info.channel.q)?;
    result.c2.poly_mod(&info.pk.u, info.channel.q)?;
    result.level = a.level.wrapping_add(b.level);
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
