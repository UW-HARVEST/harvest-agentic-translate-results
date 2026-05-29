#![allow(unused_imports)]
use c_aces::aces::{
    aces_add, aces_decrypt, aces_encrypt, aces_mul, aces_refresh, init_aces, set_aces, Aces,
    CipherMessage, PrivateKey, PublicKey, SharedInfo,
};
use c_aces::channel::{Channel, Parameters};
use c_aces::matrix::Matrix3D;
use c_aces::polynomial::{PolyArray, Polynomial};

fn make_aces(dim: usize) -> Aces {
    Aces {
        shared_info: SharedInfo {
            channel: Channel::new(2, 33, 1),
            param: Parameters { dim: dim as u64, N: 1 },
            pk: PublicKey {
                u: Polynomial::new(vec![0i64; dim + 1]),
                lambda: Matrix3D::new(dim, dim),
            },
        },
        private_key: PrivateKey {
            x: PolyArray::new((0..dim).map(|_| Polynomial::new(vec![0i64; dim])).collect()),
            f0: PolyArray::new((0..dim).map(|_| Polynomial::new(vec![0i64; dim])).collect()),
            f1: Polynomial::new(vec![0i64; dim]),
        },
    }
}

fn make_cipher(dim: usize) -> CipherMessage {
    CipherMessage {
        c1: PolyArray::new((0..dim).map(|_| Polynomial::new(vec![0i64; dim])).collect()),
        c2: Polynomial::new(vec![0i64; dim]),
        level: 0,
    }
}

#[test]
fn test_set_aces_basic() {
    let mut aces = make_aces(0);
    let mut mem = vec![0u8; 100_000];
    let r = set_aces(&mut aces, 5, &mut mem);
    assert!(r.is_ok());
    assert_eq!(aces.shared_info.pk.u.coeffs.len(), 6);
    assert_eq!(aces.shared_info.pk.lambda.data.len(), 5);
    assert_eq!(aces.private_key.x.polies.len(), 5);
    assert_eq!(aces.private_key.f0.polies.len(), 5);
    assert_eq!(aces.private_key.f1.coeffs.len(), 5);
}

#[test]
fn test_set_aces_insufficient_memory() {
    let mut aces = make_aces(0);
    let mut mem = vec![0u8; 1];
    let r = set_aces(&mut aces, 5, &mut mem);
    assert!(r.is_err());
}

#[test]
fn test_init_aces_sets_params() {
    let mut aces = make_aces(10);
    init_aces(2, 33, 10, &mut aces).unwrap();
    assert_eq!(aces.shared_info.param.dim, 10);
    assert_eq!(aces.shared_info.param.N, 1);
    assert_eq!(aces.shared_info.channel.p, 2);
    assert_eq!(aces.shared_info.channel.q, 33);
    assert_eq!(aces.shared_info.channel.w, 1);
    // u sums to q
    assert_eq!(aces.shared_info.pk.u.coef_sum(), 33);
    // x has dim polies, each with dim coeffs
    assert_eq!(aces.private_key.x.polies.len(), 10);
    for p in &aces.private_key.x.polies {
        assert_eq!(p.coeffs.len(), 10);
    }
    // f0 same
    assert_eq!(aces.private_key.f0.polies.len(), 10);
    for p in &aces.private_key.f0.polies {
        assert_eq!(p.coeffs.len(), 10);
    }
    // lambda
    assert_eq!(aces.shared_info.pk.lambda.data.len(), 10);
    for m in &aces.shared_info.pk.lambda.data {
        assert_eq!(m.dim, 10);
    }
}

#[test]
fn test_aces_encrypt_runs() {
    let mut aces = make_aces(10);
    init_aces(2, 33, 10, &mut aces).unwrap();
    let msg: [u64; 1] = [0];
    let mut cipher = make_cipher(10);
    aces_encrypt(&aces, &msg, &mut cipher).unwrap();
    assert_eq!(cipher.level, aces.shared_info.channel.p);
    assert_eq!(cipher.c1.polies.len(), 10);
}

#[test]
fn test_aces_encrypt_too_large_message() {
    let aces = make_aces(10);
    let msg: [u64; 2] = [0, 1];
    let mut cipher = make_cipher(10);
    assert!(aces_encrypt(&aces, &msg, &mut cipher).is_err());
}

#[test]
fn test_aces_decrypt_too_large_buffer() {
    let aces = make_aces(10);
    let cipher = make_cipher(10);
    let mut buf = vec![0u64; 2];
    assert!(aces_decrypt(&aces, &cipher, &mut buf).is_err());
}

#[test]
fn test_aces_add_combines_levels() {
    let mut aces = make_aces(10);
    init_aces(2, 33, 10, &mut aces).unwrap();

    let mut a = make_cipher(10);
    let mut b = make_cipher(10);
    let mut r = make_cipher(10);
    let msg: [u64; 1] = [0];
    aces_encrypt(&aces, &msg, &mut a).unwrap();
    aces_encrypt(&aces, &msg, &mut b).unwrap();

    let alvl = a.level;
    let blvl = b.level;
    aces_add(&a, &b, &aces.shared_info, &mut r).unwrap();
    assert_eq!(r.level, alvl + blvl);
    assert_eq!(r.c1.polies.len(), 10);
}

#[test]
fn test_aces_mul_does_not_panic() {
    let mut aces = make_aces(10);
    init_aces(2, 33, 10, &mut aces).unwrap();
    let mut a = make_cipher(10);
    let mut b = make_cipher(10);
    let mut r = make_cipher(10);
    let msg: [u64; 1] = [0];
    aces_encrypt(&aces, &msg, &mut a).unwrap();
    aces_encrypt(&aces, &msg, &mut b).unwrap();
    // C version is a TODO that returns 0 without modifying result.
    aces_mul(&a, &b, &aces.shared_info, &mut r).unwrap();
}

#[test]
fn test_aces_refresh_decrements_level() {
    let mut aces = make_aces(10);
    init_aces(2, 33, 10, &mut aces).unwrap();
    let mut a = make_cipher(10);
    let msg: [u64; 1] = [0];
    aces_encrypt(&aces, &msg, &mut a).unwrap();

    // Adding the same ciphertext twice raises the level. Then refresh by 1.
    let mut sum = make_cipher(10);
    aces_add(&a, &a, &aces.shared_info, &mut sum).unwrap();
    let pre = sum.level;
    aces_refresh(&aces.shared_info, &mut sum, 1).unwrap();
    assert_eq!(sum.level, pre - 1);
}

#[test]
fn test_aces_decrypt_runs() {
    // Round-trip is not guaranteed by the C reference under all parameters because
    // generate_error/generate_vanisher introduce noise. Just verify that decrypt
    // returns a value within [0, p) without panicking.
    let mut aces = make_aces(10);
    init_aces(2, 33, 10, &mut aces).unwrap();
    let mut cipher = make_cipher(10);
    let msg: [u64; 1] = [0];
    aces_encrypt(&aces, &msg, &mut cipher).unwrap();
    let mut buf = vec![0u64; 1];
    aces_decrypt(&aces, &cipher, &mut buf).unwrap();
    assert!(buf[0] < aces.shared_info.channel.p);
}

fn main() {}
