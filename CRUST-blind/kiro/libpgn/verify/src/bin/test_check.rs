use libpgn::check::PgnCheck;

#[test]
fn test_check_from_string() {
    let mut consumed = 0;
    assert_eq!(PgnCheck::__pgn_check_from_string("+", &mut consumed), PgnCheck::Single);
    assert_eq!(consumed, 1);

    consumed = 0;
    assert_eq!(PgnCheck::__pgn_check_from_string("++", &mut consumed), PgnCheck::Double);
    assert_eq!(consumed, 2);

    consumed = 0;
    assert_eq!(PgnCheck::__pgn_check_from_string("#", &mut consumed), PgnCheck::Mate);
    assert_eq!(consumed, 1);

    consumed = 0;
    assert_eq!(PgnCheck::__pgn_check_from_string("e4", &mut consumed), PgnCheck::None);
    assert_eq!(consumed, 0);
}

#[test]
fn test_check_from_str_trait() {
    assert_eq!(PgnCheck::from("+"), PgnCheck::Single);
    assert_eq!(PgnCheck::from("++"), PgnCheck::Double);
    assert_eq!(PgnCheck::from("#"), PgnCheck::Mate);
    assert_eq!(PgnCheck::from("e4"), PgnCheck::None);
}

fn main() {}
