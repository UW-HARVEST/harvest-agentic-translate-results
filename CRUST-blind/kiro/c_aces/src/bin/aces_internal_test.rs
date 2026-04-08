use c_aces::aces_internal::{generate_error, generate_linear, generate_u, generate_vanisher};
use c_aces::channel::{Channel, Parameters};
use c_aces::polynomial::Polynomial;

#[test]
fn test_channel_init_valid() {
    let ch = Channel::init(2, 33, 1).unwrap();
    assert_eq!(ch.p, 2);
    assert_eq!(ch.q, 33);
    assert_eq!(ch.w, 1);
}

#[test]
fn test_channel_init_p_squared_not_less_than_q() {
    // p=5, q=10: p^2=25 >= 10, so q becomes (5+1)^2 = 36
    let ch = Channel::init(5, 10, 1).unwrap();
    assert_eq!(ch.p, 5);
    assert_eq!(ch.q, 36);
}

#[test]
fn test_channel_init_coprime_check() {
    // p=5, q=26: p^2=25 < 26 and gcd(5,26)=1, so q stays 26
    let ch = Channel::init(5, 26, 1).unwrap();
    assert_eq!(ch.p, 5);
    assert_eq!(ch.q, 26);
}

#[test]
fn test_generate_u_coef_sum_equals_q() {
    let ch = Channel::init(2, 33, 1).unwrap();
    let param = Parameters { dim: 10, N: 1 };
    let mut u = Polynomial::new(vec![0; 11]);
    generate_u(&ch, &param, &mut u).unwrap();
    assert_eq!(u.coef_sum(), 33);
    assert_eq!(u.coeffs[0], 1); // leading coeff is 1
}

#[test]
fn test_generate_error_coef_sum_equals_message() {
    let q = 33u64;
    let message = 7u64;
    let mut rm = Polynomial::new(vec![0; 10]);
    generate_error(q, message, &mut rm).unwrap();
    // coef_sum(rm) mod q == message
    let sum = rm.coef_sum();
    let result = ((sum % q as i64) + q as i64) % q as i64;
    assert_eq!(result as u64, message);
}

#[test]
fn test_generate_vanisher_coef_sum_is_multiple_of_p() {
    let p = 5u64;
    let q = 33u64;
    let mut e = Polynomial::new(vec![0; 10]);
    generate_vanisher(p, q, &mut e).unwrap();
    // coef_sum(e) mod q should be 0 or p (i.e., p*k where k in {0,1})
    let sum = e.coef_sum();
    let val = ((sum % q as i64) + q as i64) % q as i64;
    assert!(val as u64 == 0 || val as u64 == p);
}

#[test]
fn test_generate_linear_coef_sum_in_range() {
    let p = 5u64;
    let q = 33u64;
    let mut b = Polynomial::new(vec![0; 10]);
    generate_linear(p, q, &mut b).unwrap();
    // coef_sum(b) mod q should be in [0, p]
    let sum = b.coef_sum();
    let val = ((sum % q as i64) + q as i64) % q as i64;
    assert!(val as u64 <= p);
}

fn main() {}
