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
/// Set up the ACES instance using a given dimension. The `_mem` argument is
/// kept for API parity with the C version; in this safe Rust port we own
/// memory directly.
pub fn set_aces(aces: &mut Aces, dim: usize, _mem: &mut [u8]) -> Result<()> {
    // u: dim + 1 coefficients
    aces.shared_info.pk.u = Polynomial::new(vec![0 as Coeff; dim + 1]);
    // lambda: dim Matrix2D, each of dim x dim
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);
    // x: dim polynomials with dim coefficients each
    aces.private_key.x = PolyArray::new(
        (0..dim)
            .map(|_| Polynomial::new(vec![0 as Coeff; dim]))
            .collect(),
    );
    // f0: dim polynomials with dim coefficients each
    aces.private_key.f0 = PolyArray::new(
        (0..dim)
            .map(|_| Polynomial::new(vec![0 as Coeff; dim]))
            .collect(),
    );
    // f1: dim coefficients
    aces.private_key.f1 = Polynomial::new(vec![0 as Coeff; dim]);
    Ok(())
}

/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param.dim = dim;
    aces.shared_info.param.N = 1;

    aces.shared_info.channel = Channel::new(p, q, 1);

    // Move u out so we can call generate_u without overlapping borrows.
    let mut u = std::mem::replace(&mut aces.shared_info.pk.u, Polynomial::new(Vec::new()));
    generate_u(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &mut u,
    )?;
    aces.shared_info.pk.u = u;

    // Generate secret: needs the read-only u and writable x and lambda.
    let mut x =
        std::mem::replace(&mut aces.private_key.x, PolyArray::new(Vec::new()));
    let mut lambda = std::mem::replace(
        &mut aces.shared_info.pk.lambda,
        Matrix3D::new(0, 0),
    );
    generate_secret(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &aces.shared_info.pk.u,
        &mut x,
        &mut lambda,
    )?;
    aces.private_key.x = x;
    aces.shared_info.pk.lambda = lambda;

    // Generate f0
    let mut f0 = std::mem::replace(&mut aces.private_key.f0, PolyArray::new(Vec::new()));
    generate_f0(&aces.shared_info.channel, &aces.shared_info.param, &mut f0)?;
    aces.private_key.f0 = f0;

    // Generate f1
    let mut f1 = std::mem::replace(
        &mut aces.private_key.f1,
        Polynomial::new(Vec::new()),
    );
    generate_f1(
        &aces.shared_info.channel,
        &aces.shared_info.param,
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
        return Err(crate::error::AcesError::GenericError(
            "message size > 1".to_string(),
        ));
    }
    if message.is_empty() {
        return Err(crate::error::AcesError::GenericError(
            "empty message".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;
    let u = &aces.shared_info.pk.u;

    let mut r_m = Polynomial::new(vec![0 as Coeff; dim]);
    let mut e = Polynomial::new(vec![0 as Coeff; dim]);
    let mut b = Polynomial::new(vec![0 as Coeff; dim]);

    generate_error(q, message[0], &mut r_m)?;
    generate_vanisher(p, q, &mut e)?;
    generate_linear(p, q, &mut b)?;

    // C1: for each i, c1[i] = (b * f0[i]) mod u
    for i in 0..dim {
        if i >= aces.private_key.f0.polies.len() {
            break;
        }
        let mut tmp = b.mul(&aces.private_key.f0.polies[i], q)?;
        tmp.poly_mod(u, q)?;
        // Copy tmp into result.c1.polies[i] keeping result's allocated size.
        copy_into_polynomial(&mut result.c1.polies[i], &tmp);
    }

    // C2: c2 = (((f1 + e) * b) + r_m) mod u
    let f1_plus_e = aces.private_key.f1.add(&e, q)?;
    let mut tmp = f1_plus_e.mul(&b, q)?;
    tmp = tmp.add(&r_m, q)?;
    tmp.poly_mod(u, q)?;
    copy_into_polynomial(&mut result.c2, &tmp);

    result.level = p;
    Ok(())
}

fn copy_into_polynomial(dst: &mut Polynomial, src: &Polynomial) {
    // The C version uses memcpy with src.size, but the Rust dst should retain
    // its allocated size. We simply replace dst's contents with src's coeffs.
    dst.coeffs = src.coeffs.clone();
}

/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.is_empty() {
        return Err(crate::error::AcesError::GenericError(
            "result slice empty".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut c0_t_x = Polynomial::new(vec![0 as Coeff; 2 * dim]);

    for i in 0..dim {
        if i >= message.c1.polies.len() || i >= aces.private_key.x.polies.len() {
            break;
        }
        let tmp = message.c1.polies[i].mul(&aces.private_key.x.polies[i], q)?;
        c0_t_x = tmp.add(&c0_t_x, q)?;
    }

    c0_t_x.fit(q)?;
    let diff = message.c2.sub(&c0_t_x, q)?;
    let sum = diff.coef_sum();
    // C: *result = (coef_sum(c0Tx) % q) % p
    let m = q as Coeff;
    let s = sum % m;
    let s = if s < 0 { s + m } else { s };
    result[0] = (s as u64) % p;
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
    let u = &info.pk.u;

    for i in 0..dim {
        if i >= a.c1.polies.len() || i >= b.c1.polies.len() || i >= result.c1.polies.len() {
            break;
        }
        let mut sum = a.c1.polies[i].add(&b.c1.polies[i], q)?;
        sum.poly_mod(u, q)?;
        result.c1.polies[i] = sum;
    }

    let mut c2_sum = a.c2.add(&b.c2, q)?;
    c2_sum.poly_mod(u, q)?;
    result.c2 = c2_sum;
    result.level = a.level + b.level;
    Ok(())
}

/// Perform homomorphic multiplication on two ciphertexts.
/// The C implementation is currently a TODO that simply returns 0; we keep
/// the same behavior here.
pub fn aces_mul(
    _a: &CipherMessage,
    _b: &CipherMessage,
    _info: &SharedInfo,
    _result: &mut CipherMessage,
) -> Result<()> {
    Ok(())
}

/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    let q = info.channel.q;
    let p = info.channel.p;
    let new_c2 = message.c2.sub_scaler(level * p, q)?;
    message.c2 = new_c2;
    message.level -= level;
    Ok(())
}
