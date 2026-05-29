#![allow(unused_imports)]
use c_aces::channel::{Channel, Parameters};

#[test]
fn test_channel_init_pq_squared_less_than_q() {
    // C: init_channel(2, 33, 1) - p^2=4 < 33, gcd(2,33)=1 -> coprime, so q stays
    let c = Channel::new(2, 33, 1);
    assert_eq!(c.p, 2);
    assert_eq!(c.q, 33);
    assert_eq!(c.w, 1);
}

#[test]
fn test_channel_init_pq_not_squared_less_than_q() {
    // p^2 = 9 > 8, so q = (3+1)^2 = 16
    let c = Channel::new(3, 8, 1);
    assert_eq!(c.p, 3);
    assert_eq!(c.q, 16);
    assert_eq!(c.w, 1);
}

#[test]
fn test_channel_init_not_coprime() {
    // p^2 = 25 < 100, but gcd(5,100) = 5, not coprime, so q = 36
    let c = Channel::new(5, 100, 1);
    assert_eq!(c.p, 5);
    assert_eq!(c.q, 36);
    assert_eq!(c.w, 1);
}

#[test]
fn test_channel_init_valid() {
    // p^2 = 25 < 51, coprime(5,51): gcd=1 yes
    let c = Channel::new(5, 51, 1);
    assert_eq!(c.p, 5);
    assert_eq!(c.q, 51);
    assert_eq!(c.w, 1);
}

#[test]
fn test_channel_init_valid_2() {
    // p^2 = 49 < 50, coprime(7,50): gcd=1 yes
    let c = Channel::new(7, 50, 1);
    assert_eq!(c.p, 7);
    assert_eq!(c.q, 50);
    assert_eq!(c.w, 1);
}

#[test]
fn test_channel_init_method_ok() {
    let c = Channel::init(2, 33, 1).expect("p < q should succeed");
    assert_eq!(c.p, 2);
    assert_eq!(c.q, 33);
    assert_eq!(c.w, 1);
}

#[test]
fn test_channel_init_method_err() {
    // p >= q should fail
    let r = Channel::init(33, 2, 1);
    assert!(r.is_err());
    let r = Channel::init(5, 5, 1);
    assert!(r.is_err());
}

#[test]
fn test_parameters_struct() {
    let p = Parameters { dim: 10, N: 1 };
    assert_eq!(p.dim, 10);
    assert_eq!(p.N, 1);
}

fn main() {}
