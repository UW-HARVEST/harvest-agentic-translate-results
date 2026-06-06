use crate::aces_internal::{
    generate_error, generate_f0, generate_f1, generate_linear, generate_secret, generate_u,
    generate_vanisher,
};
use crate::channel::{Channel, Parameters};
use crate::error::{AcesError, Result};
use crate::matrix::Matrix3D;
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

/// Build a fresh CipherMessage with allocated buffers matching the conventions
/// used by `aces_encrypt` (`c1` has `dim` polynomials of `dim` coeffs, and `c2`
/// has `dim` coefficients).
pub fn make_cipher_message(dim: usize) -> CipherMessage {
    let polies = (0..dim)
        .map(|_| Polynomial {
            coeffs: vec![0i64; dim],
        })
        .collect();
    CipherMessage {
        c1: PolyArray { polies },
        c2: Polynomial {
            coeffs: vec![0i64; dim],
        },
        level: 0,
    }
}

/// Set up the ACES instance (using external memory, for example).
pub fn set_aces(aces: &mut Aces, dim: usize, _mem: &mut [u8]) -> Result<()> {
    // Initialize the buffers expected by the rest of the algorithm. We don't
    // honor the external memory pointer in safe Rust; instead we allocate
    // appropriately-sized vectors that mirror the layout the C code sets up.
    aces.shared_info.pk.u = Polynomial {
        coeffs: vec![0i64; dim + 1],
    };
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);

    let x_polies = (0..dim)
        .map(|_| Polynomial {
            coeffs: vec![0i64; dim],
        })
        .collect();
    aces.private_key.x = PolyArray { polies: x_polies };

    let f0_polies = (0..dim)
        .map(|_| Polynomial {
            coeffs: vec![0i64; dim],
        })
        .collect();
    aces.private_key.f0 = PolyArray { polies: f0_polies };

    aces.private_key.f1 = Polynomial {
        coeffs: vec![0i64; dim],
    };
    Ok(())
}
/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param = Parameters { dim, N: 1 };

    aces.shared_info.channel = Channel::new(p, q, 1);

    // Make sure the buffers exist with the right sizes.
    set_aces(aces, dim as usize, &mut [])?;

    generate_u(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &mut aces.shared_info.pk.u,
    )?;

    // Build inputs for generate_secret.
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
            "aces_encrypt: only single-element messages are supported".to_string(),
        ));
    }
    if message.is_empty() {
        return Err(AcesError::GenericError(
            "aces_encrypt: message cannot be empty".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut r_m = Polynomial {
        coeffs: vec![0i64; dim],
    };
    let mut e = Polynomial {
        coeffs: vec![0i64; dim],
    };
    let mut b = Polynomial {
        coeffs: vec![0i64; dim],
    };

    generate_error(q, message[0], &mut r_m)?;
    generate_vanisher(p, q, &mut e)?;
    generate_linear(p, q, &mut b)?;

    // Make sure the result has the right shape.
    if result.c1.polies.len() != dim {
        result.c1.polies = (0..dim)
            .map(|_| Polynomial {
                coeffs: vec![0i64; dim],
            })
            .collect();
    }
    if result.c2.coeffs.len() != dim {
        result.c2.coeffs = vec![0i64; dim];
    }

    // C1
    for i in 0..dim {
        let mut tmp = b.mul(&aces.private_key.f0.polies[i], q)?;
        tmp.poly_mod(&aces.shared_info.pk.u, q)?;
        // Copy tmp.coeffs into result.c1.polies[i].coeffs (matching C's memcpy).
        let n = std::cmp::min(tmp.coeffs.len(), result.c1.polies[i].coeffs.len());
        for k in 0..n {
            result.c1.polies[i].coeffs[k] = tmp.coeffs[k];
        }
        // Truncate the buffer to the result polynomial size if the buffer is bigger.
        if tmp.coeffs.len() < result.c1.polies[i].coeffs.len() {
            result.c1.polies[i].coeffs.truncate(tmp.coeffs.len());
        }
    }

    // C2 = (f1 + e) * b + r_m mod u, mod q
    let c2_pre = aces.private_key.f1.add(&e, q)?;
    let mut tmp = c2_pre.mul(&b, q)?;
    tmp = tmp.add(&r_m, q)?;
    tmp.poly_mod(&aces.shared_info.pk.u, q)?;
    result.c2 = tmp;

    // Level
    result.level = p;

    Ok(())
}
/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() > 1 {
        return Err(AcesError::GenericError(
            "aces_decrypt: only single-element results supported".to_string(),
        ));
    }
    if result.is_empty() {
        return Err(AcesError::GenericError(
            "aces_decrypt: result cannot be empty".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    // c0Tx is computed using C polynomial buffers of size 2*dim. We instead
    // accumulate using the polynomial APIs.
    let mut c0_t_x = Polynomial {
        coeffs: vec![0i64; 2 * dim],
    };
    for i in 0..dim {
        let tmp = message.c1.polies[i].mul(&aces.private_key.x.polies[i], q)?;
        c0_t_x = c0_t_x.add(&tmp, q)?;
    }

    // poly_fit removes leading zeros and applies modulus.
    c0_t_x.fit(q)?;
    let c0_t_x = message.c2.sub(&c0_t_x, q)?;

    let q_i = q as i64;
    let p_i = p as i64;
    let raw_sum: i64 = c0_t_x.coeffs.iter().sum();
    let modq = ((raw_sum % q_i) + q_i) % q_i;
    let modp = ((modq % p_i) + p_i) % p_i;
    result[0] = modp as u64;
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

    // Make sure result is sized correctly.
    if result.c1.polies.len() != dim {
        result.c1.polies = (0..dim)
            .map(|_| Polynomial {
                coeffs: vec![0i64; dim],
            })
            .collect();
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
    // TODO: Mirroring C's aces_mul, this is currently a no-op.
    Ok(())
}
/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    let scaler = level.wrapping_mul(info.channel.p);
    message.c2 = message.c2.sub_scaler(scaler, info.channel.q)?;
    message.level = message.level.wrapping_sub(level);
    Ok(())
}
