use c_aces::aces::*;
use c_aces::channel::{Channel, Parameters};
use c_aces::matrix::Matrix3D;
use c_aces::polynomial::{PolyArray, Polynomial};

fn make_aces_shell(dim: usize) -> Aces {
    Aces {
        shared_info: SharedInfo {
            channel: Channel::new(0, 0, 0),
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
fn test_set_aces() {
    let dim = 4;
    let mut aces = make_aces_shell(dim);
    let mut mem = vec![0u8; 65536];
    set_aces(&mut aces, dim, &mut mem).unwrap();
    assert_eq!(aces.shared_info.pk.u.coeffs.len(), dim + 1);
    assert_eq!(aces.shared_info.pk.lambda.data.len(), dim);
    assert_eq!(aces.private_key.x.polies.len(), dim);
    assert_eq!(aces.private_key.f0.polies.len(), dim);
    assert_eq!(aces.private_key.f1.coeffs.len(), dim);
}

#[test]
fn test_set_aces_insufficient_memory() {
    let mut aces = make_aces_shell(4);
    let mut mem = vec![0u8; 1];
    assert!(set_aces(&mut aces, 4, &mut mem).is_err());
}

#[test]
fn test_aces_mul_noop() {
    let dim = 4;
    let c1 = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0; dim]); dim]),
        c2: Polynomial::new(vec![0; dim]),
        level: 0,
    };
    let c2 = CipherMessage {
        c1: PolyArray::new(vec![Polynomial::new(vec![0; dim]); dim]),
        c2: Polynomial::new(vec![0; dim]),
        level: 0,
    };
    let info = SharedInfo {
        channel: Channel::new(2, 33, 1),
        param: Parameters { dim: dim as u64, N: 1 },
        pk: PublicKey {
            u: Polynomial::new(vec![0; dim + 1]),
            lambda: Matrix3D::new(dim, dim),
        },
    };
    let mut result = CipherMessage {
        c1: PolyArray::new(vec![]),
        c2: Polynomial::new(vec![]),
        level: 0,
    };
    // aces_mul is a no-op (TODO in C), should return Ok
    assert!(aces_mul(&c1, &c2, &info, &mut result).is_ok());
}

fn main() {}
