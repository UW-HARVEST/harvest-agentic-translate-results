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
/// Set up the ACES instance (using external memory, for example).
pub fn set_aces(aces: &mut Aces, dim: usize, _mem: &mut [u8]) -> Result<()> {
    aces.shared_info.pk.u = Polynomial {
        coeffs: vec![0 as Coeff; dim + 1],
    };
    aces.shared_info.pk.lambda = Matrix3D::new(dim, dim);

    let x_polies: Vec<Polynomial> = (0..dim)
        .map(|_| Polynomial {
            coeffs: vec![0 as Coeff; dim],
        })
        .collect();
    aces.private_key.x = PolyArray { polies: x_polies };

    let f0_polies: Vec<Polynomial> = (0..dim)
        .map(|_| Polynomial {
            coeffs: vec![0 as Coeff; dim],
        })
        .collect();
    aces.private_key.f0 = PolyArray { polies: f0_polies };

    aces.private_key.f1 = Polynomial {
        coeffs: vec![0 as Coeff; dim],
    };

    Ok(())
}
/// Initialize an instance of ACES.
pub fn init_aces(p: u64, q: u64, dim: u64, aces: &mut Aces) -> Result<()> {
    aces.shared_info.param.dim = dim;
    aces.shared_info.param.N = 1;

    aces.shared_info.channel = Channel::init(p, q, 1)?;

    // Make sure structures are sized correctly
    let dim_usize = dim as usize;
    if aces.shared_info.pk.u.coeffs.len() < dim_usize + 1 {
        aces.shared_info.pk.u = Polynomial {
            coeffs: vec![0 as Coeff; dim_usize + 1],
        };
    }
    if aces.shared_info.pk.lambda.data.len() < dim_usize {
        aces.shared_info.pk.lambda = Matrix3D::new(dim_usize, dim_usize);
    }
    if aces.private_key.x.polies.len() < dim_usize {
        let x_polies: Vec<Polynomial> = (0..dim_usize)
            .map(|_| Polynomial {
                coeffs: vec![0 as Coeff; dim_usize],
            })
            .collect();
        aces.private_key.x = PolyArray { polies: x_polies };
    }
    if aces.private_key.f0.polies.len() < dim_usize {
        let f0_polies: Vec<Polynomial> = (0..dim_usize)
            .map(|_| Polynomial {
                coeffs: vec![0 as Coeff; dim_usize],
            })
            .collect();
        aces.private_key.f0 = PolyArray { polies: f0_polies };
    }
    if aces.private_key.f1.coeffs.len() < dim_usize {
        aces.private_key.f1 = Polynomial {
            coeffs: vec![0 as Coeff; dim_usize],
        };
    }

    let channel = aces.shared_info.channel;
    let param = aces.shared_info.param;
    generate_u(&channel, &param, &mut aces.shared_info.pk.u)?;

    // Generate secret key (separately to avoid double-borrow issues)
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
        return Err(crate::error::AcesError::GenericError(
            "message size > 1 unsupported".to_string(),
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

    let msg = if message.is_empty() { 0 } else { message[0] };

    generate_error(q, msg, &mut r_m)?;
    generate_vanisher(p, q, &mut e)?;
    generate_linear(p, q, &mut b)?;

    // C1: for each i, c1[i] = (b * f0[i]) mod u (mod q)
    for i in 0..dim {
        let mut tmp = poly_mul_buf(&b, &aces.private_key.f0.polies[i], 2 * dim, q)?;
        tmp.poly_mod(&aces.shared_info.pk.u, q)?;
        // Copy into result.c1.polies[i]
        copy_poly_to_buf(&tmp, &mut result.c1.polies[i]);
    }

    // C2: c2 = ((f1 + e) * b + r_m) mod u
    let f1_plus_e = aces.private_key.f1.add(&e, q)?;
    let mut tmp = poly_mul_buf(&f1_plus_e, &b, 2 * dim, q)?;
    let tmp_with_rm = tmp.add(&r_m, q)?;
    tmp = tmp_with_rm;
    tmp.poly_mod(&aces.shared_info.pk.u, q)?;
    copy_poly_to_buf(&tmp, &mut result.c2);

    result.level = p;
    Ok(())
}
/// Decrypt a ciphertext.
pub fn aces_decrypt(aces: &Aces, message: &CipherMessage, result: &mut [u64]) -> Result<()> {
    if result.len() > 1 {
        return Err(crate::error::AcesError::GenericError(
            "result size > 1 unsupported".to_string(),
        ));
    }
    let dim = aces.shared_info.param.dim as usize;
    let q = aces.shared_info.channel.q;
    let p = aces.shared_info.channel.p;

    let mut c0_t_x = Polynomial {
        coeffs: vec![0 as Coeff; 2 * dim],
    };
    for i in 0..dim {
        let tmp = poly_mul_buf(&message.c1.polies[i], &aces.private_key.x.polies[i], 2 * dim, q)?;
        c0_t_x = c0_t_x.add(&tmp, q)?;
    }
    c0_t_x.fit(q)?;
    let c0_final = message.c2.sub(&c0_t_x, q)?;
    let sum = c0_final.coef_sum();
    let val = ((sum % (q as i64)).rem_euclid(p as i64)) as u64;
    if !result.is_empty() {
        result[0] = val;
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
    let q = info.channel.q;
    for i in 0..dim {
        let mut sum = a.c1.polies[i].add(&b.c1.polies[i], q)?;
        sum.poly_mod(&info.pk.u, q)?;
        copy_poly_to_buf(&sum, &mut result.c1.polies[i]);
    }
    let mut c2 = a.c2.add(&b.c2, q)?;
    c2.poly_mod(&info.pk.u, q)?;
    copy_poly_to_buf(&c2, &mut result.c2);
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
    // Not implemented in C either (TODO comment)
    Ok(())
}
/// Refresh a ciphertext to mitigate level increase.
pub fn aces_refresh(info: &SharedInfo, message: &mut CipherMessage, level: u64) -> Result<()> {
    let q = info.channel.q;
    let p = info.channel.p;
    let scaler = level.wrapping_mul(p);
    let new_c2 = message.c2.sub_scaler(scaler, q)?;
    message.c2 = new_c2;
    message.level = message.level.saturating_sub(level);
    Ok(())
}

/// Poly multiplication that produces a result with the specified buffer size
/// and applies poly_fit to remove leading zeros.
fn poly_mul_buf(
    poly1: &Polynomial,
    poly2: &Polynomial,
    result_size: usize,
    modulus: u64,
) -> Result<Polynomial> {
    let mut result = vec![0 as Coeff; result_size];
    if poly1.coeffs.is_empty() || poly2.coeffs.is_empty() {
        return Ok(Polynomial { coeffs: result });
    }
    let m = modulus as i64;
    let deg1 = poly1.coeffs.len() - 1;
    let deg2 = poly2.coeffs.len() - 1;
    if result_size < deg1 + deg2 + 1 {
        return Err(crate::error::AcesError::GenericError(
            "result too small".to_string(),
        ));
    }
    let res_deg = result_size - deg2 - deg1 - 1;
    for i in 0..poly1.coeffs.len() {
        for j in 0..poly2.coeffs.len() {
            let prod = (poly1.coeffs[i] * poly2.coeffs[j]) % m;
            result[res_deg + i + j] = result[res_deg + i + j] + prod;
        }
    }
    let mut p = Polynomial { coeffs: result };
    p.fit(modulus)?;
    Ok(p)
}

/// Copy polynomial coefficients into target's existing buffer (preserving its size).
fn copy_poly_to_buf(src: &Polynomial, dst: &mut Polynomial) {
    let dst_len = dst.coeffs.len();
    let src_len = src.coeffs.len();
    let copy_len = src_len.min(dst_len);
    // Copy src into the trailing portion of dst
    if dst_len >= src_len {
        let offset = dst_len - src_len;
        for i in 0..offset {
            dst.coeffs[i] = 0;
        }
        for i in 0..copy_len {
            dst.coeffs[offset + i] = src.coeffs[i];
        }
    } else {
        // src bigger than dst: just take the most significant portion
        let offset = src_len - dst_len;
        for i in 0..dst_len {
            dst.coeffs[i] = src.coeffs[offset + i];
        }
    }
}
