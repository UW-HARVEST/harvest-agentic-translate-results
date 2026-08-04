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
pub fn set_aces(aces: &mut Aces, dim: usize, _mem: &mut [u8]) -> Result<()> {
    // In Rust we don't manage memory manually. Instead, we resize all of the
    // owned data structures so they have the right shape for the given `dim`.

    // Initialize u: polynomial of size (dim + 1)
    aces.shared_info.pk.u = Polynomial::new(vec![0 as Coeff; dim + 1]);

    // Initialize lambda: 3D matrix of size `dim`, each Matrix2D of dim×dim
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);

    // Initialize x: PolyArray of `dim` polynomials each of size `dim`
    let mut x_polies = Vec::with_capacity(dim);
    for _ in 0..dim {
        x_polies.push(Polynomial::new(vec![0 as Coeff; dim]));
    }
    aces.private_key.x = PolyArray::new(x_polies);

    // Initialize f0: PolyArray of `dim` polynomials each of size `dim`
    let mut f0_polies = Vec::with_capacity(dim);
    for _ in 0..dim {
        f0_polies.push(Polynomial::new(vec![0 as Coeff; dim]));
    }
    aces.private_key.f0 = PolyArray::new(f0_polies);

    // Initialize f1: polynomial of size `dim`
    aces.private_key.f1 = Polynomial::new(vec![0 as Coeff; dim]);

    Ok(())
}

/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param = Parameters { dim, N: 1 };

    // C calls init_channel which sets w = 1 always; we mirror that.
    aces.shared_info.channel = Channel::new(p, q, 1);

    // Make sure our owned data structures are correctly sized.
    set_aces(aces, dim as usize, &mut [])?;

    // Generate u
    let mut u = std::mem::replace(&mut aces.shared_info.pk.u, Polynomial::new(vec![]));
    generate_u(&aces.shared_info.channel, &aces.shared_info.param, &mut u)?;
    aces.shared_info.pk.u = u;

    // Generate secret (x) and lambda
    let channel = aces.shared_info.channel;
    let param = aces.shared_info.param;
    let u_poly = aces.shared_info.pk.u.clone();
    let mut x = std::mem::replace(&mut aces.private_key.x, PolyArray::new(vec![]));
    let mut lambda = std::mem::replace(&mut aces.shared_info.pk.lambda, Matrix3D::new(0, 0));
    generate_secret(&channel, &param, &u_poly, &mut x, &mut lambda)?;
    aces.private_key.x = x;
    aces.shared_info.pk.lambda = lambda;

    // Generate f0
    let mut f0 = std::mem::replace(&mut aces.private_key.f0, PolyArray::new(vec![]));
    generate_f0(&channel, &param, &mut f0)?;
    aces.private_key.f0 = f0;

    // Generate f1
    let mut f1 = std::mem::replace(&mut aces.private_key.f1, Polynomial::new(vec![]));
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
        return Err(AcesError::GenericError(
            "aces_encrypt: only single-element messages supported".to_string(),
        ));
    }
    if message.is_empty() {
        return Err(AcesError::GenericError(
            "aces_encrypt: empty message".to_string(),
        ));
    }

    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut r_m = Polynomial::new(vec![0 as Coeff; dim]);
    let mut e = Polynomial::new(vec![0 as Coeff; dim]);
    let mut b = Polynomial::new(vec![0 as Coeff; dim]);

    generate_error(q, message[0], &mut r_m)?;
    generate_vanisher(p, q, &mut e)?;
    generate_linear(p, q, &mut b)?;

    // Make sure result.c1 has dim polynomials.
    if result.c1.polies.len() < dim {
        result
            .c1
            .polies
            .resize(dim, Polynomial::new(vec![0 as Coeff; dim]));
    }

    // C1: For each i, c1[i] = (b * f0[i]) mod u.
    for i in 0..dim {
        let mut tmp = b.mul(&aces.private_key.f0.polies[i], q)?;
        tmp.poly_mod(&aces.shared_info.pk.u, q)?;

        // Copy into result.c1.polies[i].coeffs (matching C memcpy of tmp.size bytes
        // into the front of result.c1.polies[i].coeffs).
        if result.c1.polies[i].coeffs.len() < tmp.coeffs.len() {
            result.c1.polies[i].coeffs.resize(tmp.coeffs.len(), 0);
        }
        for (k, c) in tmp.coeffs.iter().enumerate() {
            result.c1.polies[i].coeffs[k] = *c;
        }
    }

    // C2: c2 = (f1 + e) ; c2 = (c2 * b) + r_m ; mod u
    let c2_inter = aces.private_key.f1.add(&e, q)?;
    let mut tmp = c2_inter.mul(&b, q)?;
    tmp = tmp.add(&r_m, q)?;
    tmp.poly_mod(&aces.shared_info.pk.u, q)?;

    // Copy tmp into result.c2.coeffs (front portion).
    if result.c2.coeffs.len() < tmp.coeffs.len() {
        result.c2.coeffs.resize(tmp.coeffs.len(), 0);
    }
    for (k, c) in tmp.coeffs.iter().enumerate() {
        result.c2.coeffs[k] = *c;
    }

    // Level
    result.level = p;

    Ok(())
}

/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() > 1 {
        return Err(AcesError::GenericError(
            "aces_decrypt: only single-element messages supported".to_string(),
        ));
    }
    if result.is_empty() {
        return Err(AcesError::GenericError(
            "aces_decrypt: result buffer is empty".to_string(),
        ));
    }

    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut c0_t_x = Polynomial::new(vec![0 as Coeff; 2 * dim]);

    for i in 0..dim {
        let tmp = message.c1.polies[i].mul(&aces.private_key.x.polies[i], q)?;
        c0_t_x = c0_t_x.add(&tmp, q)?;
    }

    c0_t_x.fit(q)?;
    c0_t_x = message.c2.sub(&c0_t_x, q)?;

    let cs = c0_t_x.coef_sum();
    let qi = q as i64;
    let pi = p as i64;
    let val = ((cs % qi) + qi) % qi;
    let val = ((val % pi) + pi) % pi;
    result[0] = val as u64;

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

    if result.c1.polies.len() < dim {
        result
            .c1
            .polies
            .resize(dim, Polynomial::new(vec![0 as Coeff; dim]));
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
    // C version is a TODO that does nothing.
    Ok(())
}

/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    let q = info.channel.q;
    let p = info.channel.p;
    let new_c2 = message.c2.sub_scaler(level.wrapping_mul(p), q)?;
    message.c2 = new_c2;
    message.level = message.level.wrapping_sub(level);
    Ok(())
}
