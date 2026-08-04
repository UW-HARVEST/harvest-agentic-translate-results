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
/// In this Rust port, this function allocates the internal buffers needed by
/// the various components of the Aces structure for the given dimension. The
/// `mem` parameter is accepted to mirror the C interface but is not used since
/// Rust manages memory via owned `Vec`s.
pub fn set_aces(aces: &mut Aces, dim: usize, _mem: &mut [u8]) -> Result<()> {
    // u is a polynomial of size dim+1
    aces.shared_info.pk.u = Polynomial {
        coeffs: vec![0 as Coeff; dim + 1],
    };

    // lambda: 3D matrix of size `dim`, each 2D matrix is dim x dim
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);

    // x: PolyArray of `dim` polynomials each of size `dim`
    let mut x_polies = Vec::with_capacity(dim);
    for _ in 0..dim {
        x_polies.push(Polynomial {
            coeffs: vec![0 as Coeff; dim],
        });
    }
    aces.private_key.x = PolyArray { polies: x_polies };

    // f0: PolyArray of `dim` polynomials each of size `dim`
    let mut f0_polies = Vec::with_capacity(dim);
    for _ in 0..dim {
        f0_polies.push(Polynomial {
            coeffs: vec![0 as Coeff; dim],
        });
    }
    aces.private_key.f0 = PolyArray { polies: f0_polies };

    // f1: Polynomial of size `dim`
    aces.private_key.f1 = Polynomial {
        coeffs: vec![0 as Coeff; dim],
    };

    Ok(())
}
/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param.dim = dim;
    aces.shared_info.param.N = 1;

    aces.shared_info.channel = Channel::new(p, q, 1);

    // We need backing storage for the polynomials and matrices.
    let mut empty: Vec<u8> = Vec::new();
    set_aces(aces, dim as usize, empty.as_mut_slice())?;

    // Borrow split: extract intermediate values to avoid borrow conflicts.
    let channel = aces.shared_info.channel;
    let param = aces.shared_info.param;

    generate_u(&channel, &param, &mut aces.shared_info.pk.u)?;

    {
        let u_clone = aces.shared_info.pk.u.clone();
        generate_secret(
            &channel,
            &param,
            &u_clone,
            &mut aces.private_key.x,
            &mut aces.shared_info.pk.lambda,
        )?;
    }
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
        return Err(AcesError::GenericError(
            "aces_encrypt: only one message supported".to_string(),
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

    let mut r_m = Polynomial {
        coeffs: vec![0 as Coeff; dim],
    };
    let mut e = Polynomial {
        coeffs: vec![0 as Coeff; dim],
    };
    let mut b = Polynomial {
        coeffs: vec![0 as Coeff; dim],
    };

    generate_error(q, message[0], &mut r_m)?;
    generate_vanisher(p, q, &mut e)?;
    generate_linear(p, q, &mut b)?;

    // Make sure result.c1 has dim polynomials sized for the result.
    if result.c1.polies.len() < dim {
        result.c1.polies.resize(
            dim,
            Polynomial {
                coeffs: vec![0 as Coeff; dim],
            },
        );
    }

    // C1
    for i in 0..dim {
        let mut tmp = b.mul(&aces.private_key.f0.polies[i], q)?;
        tmp.poly_mod(&aces.shared_info.pk.u, q)?;
        result.c1.polies[i] = tmp;
    }

    // C2
    let mut c2 = aces.private_key.f1.add(&e, q)?;
    c2 = c2.mul(&b, q)?;
    c2 = c2.add(&r_m, q)?;
    c2.poly_mod(&aces.shared_info.pk.u, q)?;
    result.c2 = c2;

    result.level = p;
    Ok(())
}
/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() > 1 {
        return Err(AcesError::GenericError(
            "aces_decrypt: only one message supported".to_string(),
        ));
    }
    if result.is_empty() {
        return Err(AcesError::GenericError(
            "aces_decrypt: empty result buffer".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut c0_t_x = Polynomial {
        coeffs: vec![0 as Coeff; 2 * dim],
    };

    for i in 0..dim {
        let tmp = message.c1.polies[i].mul(&aces.private_key.x.polies[i], q)?;
        c0_t_x = c0_t_x.add(&tmp, q)?;
    }

    c0_t_x.fit(q)?;
    c0_t_x = message.c2.sub(&c0_t_x, q)?;

    let s = c0_t_x.coef_sum();
    let q_i = q as i64;
    let p_i = p as i64;
    let val = ((s % q_i) % p_i) as u64;
    result[0] = val;
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
        result.c1.polies.resize(
            dim,
            Polynomial {
                coeffs: vec![0 as Coeff; dim],
            },
        );
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
    // TODO: Implement it (matches the C version which also stubs this out).
    Ok(())
}
/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    let new_c2 = message.c2.sub_scaler(level * info.channel.p, info.channel.q)?;
    message.c2 = new_c2;
    message.level -= level;
    Ok(())
}
