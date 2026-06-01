use libpgn::check::PgnCheck;

#[test]
fn test_check_from_str() {
    assert_eq!(PgnCheck::from(""), PgnCheck::None);
    assert_eq!(PgnCheck::from("+"), PgnCheck::Single);
    assert_eq!(PgnCheck::from("++"), PgnCheck::Double);
    assert_eq!(PgnCheck::from("#"), PgnCheck::Mate);
    assert_eq!(PgnCheck::from("x"), PgnCheck::None);
}

#[test]
fn test_check_consumed_count() {
    let mut consumed = 0usize;
    let c = PgnCheck::__pgn_check_from_string("", &mut consumed);
    assert_eq!(c, PgnCheck::None);
    assert_eq!(consumed, 0);

    consumed = 0;
    let c = PgnCheck::__pgn_check_from_string("+", &mut consumed);
    assert_eq!(c, PgnCheck::Single);
    assert_eq!(consumed, 1);

    consumed = 0;
    let c = PgnCheck::__pgn_check_from_string("++", &mut consumed);
    assert_eq!(c, PgnCheck::Double);
    assert_eq!(consumed, 2);

    consumed = 0;
    let c = PgnCheck::__pgn_check_from_string("#", &mut consumed);
    assert_eq!(c, PgnCheck::Mate);
    assert_eq!(consumed, 1);

    consumed = 0;
    let c = PgnCheck::__pgn_check_from_string("xyz", &mut consumed);
    assert_eq!(c, PgnCheck::None);
    assert_eq!(consumed, 0);

    // Cumulative consumed
    let mut consumed = 5usize;
    let c = PgnCheck::__pgn_check_from_string("+", &mut consumed);
    assert_eq!(c, PgnCheck::Single);
    assert_eq!(consumed, 6);
}

fn main() {}
