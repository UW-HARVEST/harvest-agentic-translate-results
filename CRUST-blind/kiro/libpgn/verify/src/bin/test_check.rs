use libpgn::check::PgnCheck;

#[test]
fn test_check_none() {
    assert_eq!(PgnCheck::from(""), PgnCheck::None);
    assert_eq!(PgnCheck::from("e4"), PgnCheck::None);
}

#[test]
fn test_check_single() {
    assert_eq!(PgnCheck::from("+"), PgnCheck::Single);
}

#[test]
fn test_check_double() {
    assert_eq!(PgnCheck::from("++"), PgnCheck::Double);
}

#[test]
fn test_check_mate() {
    assert_eq!(PgnCheck::from("#"), PgnCheck::Mate);
}

#[test]
fn test_check_from_string_consumed() {
    let mut consumed = 0;
    let result = PgnCheck::__pgn_check_from_string("+rest", &mut consumed);
    assert_eq!(result, PgnCheck::Single);
    assert_eq!(consumed, 1);
}

#[test]
fn test_check_double_consumed() {
    let mut consumed = 0;
    let result = PgnCheck::__pgn_check_from_string("++rest", &mut consumed);
    assert_eq!(result, PgnCheck::Double);
    assert_eq!(consumed, 2);
}

#[test]
fn test_check_mate_consumed() {
    let mut consumed = 0;
    let result = PgnCheck::__pgn_check_from_string("#rest", &mut consumed);
    assert_eq!(result, PgnCheck::Mate);
    assert_eq!(consumed, 1);
}

#[test]
fn test_check_none_consumed() {
    let mut consumed = 0;
    let result = PgnCheck::__pgn_check_from_string("abc", &mut consumed);
    assert_eq!(result, PgnCheck::None);
    assert_eq!(consumed, 0);
}

fn main() {}
