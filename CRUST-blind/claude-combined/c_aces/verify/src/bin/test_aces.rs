use c_aces::aces::{
    aces_add, aces_decrypt, aces_encrypt, aces_mul, aces_refresh, init_aces, set_aces, Aces,
    CipherMessage, PrivateKey, PublicKey, SharedInfo,
};
use c_aces::channel::{Channel, Parameters};
use c_aces::matrix::Matrix3D;
use c_aces::polynomial::{PolyArray, Polynomial};

fn make_empty_aces() -> Aces {
    Aces {
        shared_info: SharedInfo {
            channel: Channel { p: 0, q: 0, w: 0 },
            param: Parameters { dim: 0, N: 0 },
            pk: PublicKey {
                u: Polynomial::new(vec![]),
                lambda: Matrix3D::new(0, 0),
            },
        },
        private_key: PrivateKey {
            x: PolyArray::new(vec![]),
            f0: PolyArray::new(vec![]),
            f1: Polynomial::new(vec![]),
        },
    }
}

#[test]
fn test_set_aces_allocates_buffers() {
    let mut aces = make_empty_aces();
    let mut mem = vec![0u8; 4096];
    set_aces(&mut aces, 5, &mut mem).unwrap();

    // u has dim+1 coefficients
    assert_eq!(aces.shared_info.pk.u.coeffs.len(), 6);
    // lambda is dim x dim x dim
    assert_eq!(aces.shared_info.pk.lambda.data.len(), 5);
    for sub in aces.shared_info.pk.lambda.data.iter() {
        assert_eq!(sub.dim, 5);
    }
    // x has dim polynomials of size dim
    assert_eq!(aces.private_key.x.polies.len(), 5);
    for p in aces.private_key.x.polies.iter() {
        assert_eq!(p.coeffs.len(), 5);
    }
    // f0 has dim polynomials of size dim
    assert_eq!(aces.private_key.f0.polies.len(), 5);
    for p in aces.private_key.f0.polies.iter() {
        assert_eq!(p.coeffs.len(), 5);
    }
    // f1 has size dim
    assert_eq!(aces.private_key.f1.coeffs.len(), 5);
}

#[test]
fn test_init_aces_sets_channel_and_params() {
    let mut aces = make_empty_aces();
    init_aces(2, 33, 10, &mut aces).unwrap();

    // Channel was initialized with given p,q (with w=1) since 4<33 && coprime
    assert_eq!(aces.shared_info.channel.p, 2);
    assert_eq!(aces.shared_info.channel.q, 33);
    assert_eq!(aces.shared_info.channel.w, 1);

    // Parameters
    assert_eq!(aces.shared_info.param.dim, 10);
    assert_eq!(aces.shared_info.param.N, 1);

    // u was generated and has coef_sum == q
    assert_eq!(aces.shared_info.pk.u.coef_sum(), 33);

    // f0 coefficients are in [0, q-1]
    for poly in aces.private_key.f0.polies.iter() {
        for &c in poly.coeffs.iter() {
            assert!(c >= 0 && c < 33);
        }
    }
}

#[test]
fn test_init_aces_with_failed_channel_check() {
    // p=3, q=4 -> q gets replaced with 16 = (3+1)^2
    let mut aces = make_empty_aces();
    init_aces(3, 4, 8, &mut aces).unwrap();
    assert_eq!(aces.shared_info.channel.p, 3);
    assert_eq!(aces.shared_info.channel.q, 16);
    assert_eq!(aces.shared_info.channel.w, 1);
    assert_eq!(aces.shared_info.param.dim, 8);
    assert_eq!(aces.shared_info.param.N, 1);
}

#[test]
fn test_aces_encrypt_returns_proper_level_and_size() {
    let mut aces = make_empty_aces();
    init_aces(2, 33, 10, &mut aces).unwrap();

    let mut result = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![0i64; 10]),
        level: 0,
    };
    aces_encrypt(&aces, &[1u64], &mut result).unwrap();

    assert_eq!(result.level, aces.shared_info.channel.p);
    // c1 has dim entries
    assert_eq!(result.c1.polies.len(), 10);
}

#[test]
fn test_aces_encrypt_too_large_size() {
    let mut aces = make_empty_aces();
    init_aces(2, 33, 10, &mut aces).unwrap();

    let mut result = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![0i64; 10]),
        level: 0,
    };
    let big_msg = vec![1u64; 5];
    assert!(aces_encrypt(&aces, &big_msg, &mut result).is_err());
}

#[test]
fn test_aces_add_levels_and_dimensions() {
    let mut aces = make_empty_aces();
    init_aces(2, 33, 10, &mut aces).unwrap();

    let mut a = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![0i64; 10]),
        level: 0,
    };
    let mut b = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![0i64; 10]),
        level: 0,
    };
    aces_encrypt(&aces, &[0u64], &mut a).unwrap();
    aces_encrypt(&aces, &[0u64], &mut b).unwrap();

    let mut sum = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![0i64; 10]),
        level: 0,
    };
    aces_add(&a, &b, &aces.shared_info, &mut sum).unwrap();

    // Levels should add
    assert_eq!(sum.level, a.level + b.level);
    // c1 has dim entries
    assert_eq!(sum.c1.polies.len(), 10);
}

#[test]
fn test_aces_decrypt_size_check() {
    let mut aces = make_empty_aces();
    init_aces(2, 33, 10, &mut aces).unwrap();

    let cipher = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![0i64; 10]),
        level: 2,
    };
    let mut result = vec![0u64; 5];
    assert!(aces_decrypt(&aces, &cipher, &mut result).is_err());
}

#[test]
fn test_aces_refresh_decrements_level_and_subtracts_scaler() {
    // aces_refresh: c2 = c2 - k * p (mod q); level -= k.
    let mut aces = make_empty_aces();
    init_aces(2, 33, 10, &mut aces).unwrap();

    // Create a known c2 with all zeros, level=10
    let mut msg = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![10i64, 10, 10, 10, 10]),
        level: 10,
    };

    let original_level = msg.level;
    aces_refresh(&aces.shared_info, &mut msg, 3).unwrap();
    assert_eq!(msg.level, original_level - 3);

    // Check c2 was reduced by 3*p = 6 (mod q=33), so coeffs become (10-6) % 33 = 4
    for &c in msg.c2.coeffs.iter() {
        assert_eq!(c, 4);
    }
}

#[test]
fn test_aces_mul_stub_returns_ok() {
    // aces_mul is a stub in C - it should return 0 (Ok in Rust) without computing.
    let mut aces = make_empty_aces();
    init_aces(2, 33, 10, &mut aces).unwrap();

    let a = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![0i64; 10]),
        level: 1,
    };
    let b = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![0i64; 10]),
        level: 1,
    };
    let mut result = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0i64; 10]); 10]),
        c2: Polynomial::new(vec![0i64; 10]),
        level: 99,
    };
    // Should succeed (stub).
    aces_mul(&a, &b, &aces.shared_info, &mut result).unwrap();
}

#[test]
fn test_ciphermessage_struct() {
    // Verify CipherMessage struct fields.
    let cipher = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![1, 2, 3])]),
        c2: Polynomial::new(vec![4, 5, 6]),
        level: 7,
    };
    assert_eq!(cipher.c1.polies.len(), 1);
    assert_eq!(cipher.c1.polies[0].coeffs, vec![1, 2, 3]);
    assert_eq!(cipher.c2.coeffs, vec![4, 5, 6]);
    assert_eq!(cipher.level, 7);
}

#[test]
fn test_publickey_privatekey_struct() {
    let pk = PublicKey {
        u: Polynomial::new(vec![1, 2]),
        lambda: Matrix3D::new(2, 2),
    };
    assert_eq!(pk.u.coeffs, vec![1, 2]);
    assert_eq!(pk.lambda.data.len(), 2);

    let sk = PrivateKey {
        x: PolyArray::new(vec![Polynomial::new(vec![1])]),
        f0: PolyArray::new(vec![Polynomial::new(vec![2])]),
        f1: Polynomial::new(vec![3]),
    };
    assert_eq!(sk.x.polies.len(), 1);
    assert_eq!(sk.f0.polies.len(), 1);
    assert_eq!(sk.f1.coeffs, vec![3]);
}

fn main() {}
