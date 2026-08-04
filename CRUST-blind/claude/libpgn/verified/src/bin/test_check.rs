#[allow(unused_imports)]
use libpgn::check::PgnCheck;

#[test]
fn test_check_from_string_single() {
    let mut consumed = 0;
    let r = PgnCheck::__pgn_check_from_string("+", &mut consumed);
    assert_eq!(r, PgnCheck::Single);
    assert_eq!(consumed, 1);
}

#[test]
fn test_check_from_string_double() {
    let mut consumed = 0;
    let r = PgnCheck::__pgn_check_from_string("++", &mut consumed);
    assert_eq!(r, PgnCheck::Double);
    assert_eq!(consumed, 2);
}

#[test]
fn test_check_from_string_mate() {
    let mut consumed = 0;
    let r = PgnCheck::__pgn_check_from_string("#", &mut consumed);
    assert_eq!(r, PgnCheck::Mate);
    assert_eq!(consumed, 1);
}

#[test]
fn test_check_from_string_empty() {
    let mut consumed = 0;
    let r = PgnCheck::__pgn_check_from_string("", &mut consumed);
    assert_eq!(r, PgnCheck::None);
    assert_eq!(consumed, 0);
}

#[test]
fn test_check_from_string_no_check() {
    let mut consumed = 0;
    let r = PgnCheck::__pgn_check_from_string("abc", &mut consumed);
    assert_eq!(r, PgnCheck::None);
    assert_eq!(consumed, 0);

    let mut consumed = 0;
    let r = PgnCheck::__pgn_check_from_string("x", &mut consumed);
    assert_eq!(r, PgnCheck::None);
    assert_eq!(consumed, 0);
}

#[test]
fn test_check_from_string_with_trailing() {
    let mut consumed = 0;
    let r = PgnCheck::__pgn_check_from_string("+a", &mut consumed);
    assert_eq!(r, PgnCheck::Single);
    assert_eq!(consumed, 1);

    let mut consumed = 0;
    let r = PgnCheck::__pgn_check_from_string("#a", &mut consumed);
    assert_eq!(r, PgnCheck::Mate);
    assert_eq!(consumed, 1);

    let mut consumed = 0;
    let r = PgnCheck::__pgn_check_from_string("+#", &mut consumed);
    assert_eq!(r, PgnCheck::Single);
    assert_eq!(consumed, 1);
}

#[test]
fn test_check_from_str_alias() {
    assert_eq!(PgnCheck::from("+"), PgnCheck::Single);
    assert_eq!(PgnCheck::from("++"), PgnCheck::Double);
    assert_eq!(PgnCheck::from("#"), PgnCheck::Mate);
    assert_eq!(PgnCheck::from(""), PgnCheck::None);
}

#[test]
fn test_check_repr_values() {
    assert_eq!(PgnCheck::Mate as i8, -1);
    assert_eq!(PgnCheck::None as i8, 0);
    assert_eq!(PgnCheck::Single as i8, 1);
    assert_eq!(PgnCheck::Double as i8, 2);
}

fn main() {}
