use c_aces::channel::{Channel, Parameters};

#[test]
fn test_channel_p_squared_lt_q_coprime() {
    // p=2, q=33: p^2 = 4 < 33 and gcd(2,33)=1 -> q stays 33
    let ch = Channel::new(2, 33, 1);
    assert_eq!(ch.p, 2);
    assert_eq!(ch.q, 33);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_failed_squared_check() {
    // p=3, q=4: p^2=9 not less than 4, so q replaced with (3+1)^2=16
    let ch = Channel::new(3, 4, 1);
    assert_eq!(ch.p, 3);
    assert_eq!(ch.q, 16);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_coprime_okay() {
    // p=5, q=27: 25<27, gcd(5,27)=1 -> stays 27
    let ch = Channel::new(5, 27, 1);
    assert_eq!(ch.p, 5);
    assert_eq!(ch.q, 27);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_not_coprime() {
    // p=5, q=30: 25<30 but gcd(5,30)=5, so q replaced with (5+1)^2=36
    let ch = Channel::new(5, 30, 1);
    assert_eq!(ch.p, 5);
    assert_eq!(ch.q, 36);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_init() {
    let ch = Channel::init(2, 33, 1).unwrap();
    assert_eq!(ch.p, 2);
    assert_eq!(ch.q, 33);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_parameters_struct() {
    let param = Parameters { dim: 10, N: 1 };
    assert_eq!(param.dim, 10);
    assert_eq!(param.N, 1);
}

fn main() {}
