use c_aces::channel::Channel;

#[test]
fn test_channel_new() {
    let ch = Channel::new(2, 33, 1);
    assert_eq!(ch.p, 2);
    assert_eq!(ch.q, 33);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_init_valid() {
    // p=2, q=33: p^2=4 < 33, gcd(2,33)=1 -> keep q=33
    let ch = Channel::init(2, 33, 1).unwrap();
    assert_eq!(ch.p, 2);
    assert_eq!(ch.q, 33);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_init_coprime_large_q() {
    // p=3, q=100: p^2=9 < 100, gcd(3,100)=1 -> keep q=100
    let ch = Channel::init(3, 100, 1).unwrap();
    assert_eq!(ch.p, 3);
    assert_eq!(ch.q, 100);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_init_not_coprime() {
    // p=5, q=26: p^2=25 < 26, gcd(5,26)=1 -> keep q=26
    let ch = Channel::init(5, 26, 1).unwrap();
    assert_eq!(ch.p, 5);
    assert_eq!(ch.q, 26);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_init_p_squared_not_less() {
    // p=6, q=36: p^2=36, 36 < 36 is false -> q = (6+1)^2 = 49
    let ch = Channel::init(6, 36, 1).unwrap();
    assert_eq!(ch.p, 6);
    assert_eq!(ch.q, 49);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_init_coprime_check() {
    // p=4, q=17: p^2=16 < 17, gcd(4,17)=1 -> keep q=17
    let ch = Channel::init(4, 17, 1).unwrap();
    assert_eq!(ch.p, 4);
    assert_eq!(ch.q, 17);
    assert_eq!(ch.w, 1);
}

fn main() {}
