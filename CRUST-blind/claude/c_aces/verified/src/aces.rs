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

fn required_size(dim: usize) -> usize {
    // sizeof(Coeff) * (2*dim + 1 + 2*dim*dim)
    //   + sizeof(Matrix2D) * dim
    //   + sizeof(Polynomial) * 2*dim
    //   + sizeof(uint64_t) * dim*dim*dim
    let coeff = std::mem::size_of::<i64>();
    let matrix2d_sz = std::mem::size_of::<crate::matrix::Matrix2D>();
    let poly_sz = std::mem::size_of::<Polynomial>();
    let u64_sz = std::mem::size_of::<u64>();
    coeff * (2 * dim + 1 + 2 * dim * dim)
        + matrix2d_sz * dim
        + poly_sz * 2 * dim
        + u64_sz * dim * dim * dim
}

/// Set up the ACES instance (using external memory, for example).
pub fn set_aces(aces: &mut Aces, dim: usize, mem: &mut [u8]) -> Result<()> {
    if mem.len() < required_size(dim) {
        return Err(AcesError::GenericError(
            "insufficient memory for ACES instance".to_string(),
        ));
    }
    // Rust does not need raw memory carving; we set up Vecs to match the
    // expected layout.
    aces.shared_info.pk.u = Polynomial {
        coeffs: vec![0i64; dim + 1],
    };
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);
    aces.private_key.x = PolyArray {
        polies: (0..dim)
            .map(|_| Polynomial {
                coeffs: vec![0i64; dim],
            })
            .collect(),
    };
    aces.private_key.f0 = PolyArray {
        polies: (0..dim)
            .map(|_| Polynomial {
                coeffs: vec![0i64; dim],
            })
            .collect(),
    };
    aces.private_key.f1 = Polynomial {
        coeffs: vec![0i64; dim],
    };
    Ok(())
}

/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param = Parameters { dim, N: 1 };

    aces.shared_info.channel = Channel::new(p, q, 1);

    let dim_usize = dim as usize;
    // Ensure structures are sized properly.
    if aces.shared_info.pk.u.coeffs.len() < dim_usize + 1 {
        aces.shared_info.pk.u = Polynomial {
            coeffs: vec![0i64; dim_usize + 1],
        };
    }
    if aces.shared_info.pk.lambda.data.len() < dim_usize {
        aces.shared_info.pk.lambda = Matrix3D::new(dim_usize, dim_usize);
    }
    if aces.private_key.x.polies.len() < dim_usize {
        aces.private_key.x = PolyArray {
            polies: (0..dim_usize)
                .map(|_| Polynomial {
                    coeffs: vec![0i64; dim_usize],
                })
                .collect(),
        };
    }
    if aces.private_key.f0.polies.len() < dim_usize {
        aces.private_key.f0 = PolyArray {
            polies: (0..dim_usize)
                .map(|_| Polynomial {
                    coeffs: vec![0i64; dim_usize],
                })
                .collect(),
        };
    }
    if aces.private_key.f1.coeffs.len() < dim_usize {
        aces.private_key.f1 = Polynomial {
            coeffs: vec![0i64; dim_usize],
        };
    }

    generate_u(
        &aces.shared_info.channel,
        &aces.shared_info.param,
        &mut aces.shared_info.pk.u,
    )?;

    let channel = aces.shared_info.channel;
    let param = aces.shared_info.param;
    let u = aces.shared_info.pk.u.clone();

    generate_secret(
        &channel,
        &param,
        &u,
        &mut aces.private_key.x,
        &mut aces.shared_info.pk.lambda,
    )?;
    generate_f0(&channel, &param, &mut aces.private_key.f0)?;
    generate_f1(
        &channel,
        &param,
        &aces.private_key.f0,
        &aces.private_key.x,
        &u,
        &mut aces.private_key.f1,
    )?;
    Ok(())
}

/// Encrypt a message.
pub fn aces_encrypt(aces: &Aces, message: &[u64], result: &mut CipherMessage) -> Result<()> {
    if message.len() > 1 {
        return Err(AcesError::GenericError(
            "only single-element messages supported".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let msg = if message.is_empty() { 0 } else { message[0] };

    let mut r_m = Polynomial {
        coeffs: vec![0i64; dim],
    };
    let mut e = Polynomial {
        coeffs: vec![0i64; dim],
    };
    let mut b = Polynomial {
        coeffs: vec![0i64; dim],
    };

    generate_error(q, msg, &mut r_m)?;
    generate_vanisher(p, q, &mut e)?;
    generate_linear(p, q, &mut b)?;

    // Ensure result.c1 has dim polies.
    if result.c1.polies.len() < dim {
        result.c1.polies.resize(
            dim,
            Polynomial {
                coeffs: vec![0i64; dim],
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
    let f1_plus_e = aces.private_key.f1.add(&e, q)?;
    let mut tmp = f1_plus_e.mul(&b, q)?;
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
            "only single-element results supported".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    // Compute c1 . x  (sum over i of c1[i] * x[i] mod q)
    let mut c0t_x = Polynomial { coeffs: vec![0i64] };

    for i in 0..dim {
        if i >= message.c1.polies.len() || i >= aces.private_key.x.polies.len() {
            break;
        }
        let tmp = message.c1.polies[i].mul(&aces.private_key.x.polies[i], q)?;
        c0t_x = tmp.add(&c0t_x, q)?;
    }

    c0t_x.fit(q)?;
    let diff_poly = message.c2.sub(&c0t_x, q)?;

    let csum = diff_poly.coef_sum();
    let cs_mod_q = ((csum % q as i64) + q as i64) % q as i64;
    let value = (cs_mod_q as u64) % p;
    if !result.is_empty() {
        result[0] = value;
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

    if result.c1.polies.len() < dim {
        result.c1.polies.resize(
            dim,
            Polynomial {
                coeffs: vec![0i64; dim],
            },
        );
    }
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
    // The C reference implementation has aces_mul as a TODO returning 0; we
    // mirror that behavior by returning Ok(()) without modifying result.
    Ok(())
}

/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    let new_c2 = message
        .c2
        .sub_scaler(level.wrapping_mul(info.channel.p), info.channel.q)?;
    message.c2 = new_c2;
    message.level = message.level.wrapping_sub(level);
    Ok(())
}
