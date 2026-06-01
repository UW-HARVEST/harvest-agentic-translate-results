use libpgn::annotation::PgnAnnotation;

#[test]
fn test_annotation_from_str() {
    assert_eq!(PgnAnnotation::from("!"), PgnAnnotation::GoodMove);
    assert_eq!(PgnAnnotation::from("?"), PgnAnnotation::Mistake);
    assert_eq!(PgnAnnotation::from("!!"), PgnAnnotation::BrilliantMove);
    assert_eq!(PgnAnnotation::from("??"), PgnAnnotation::Blunder);
    assert_eq!(PgnAnnotation::from("!?"), PgnAnnotation::InterestingMove);
    assert_eq!(PgnAnnotation::from("?!"), PgnAnnotation::DubiousMove);
    assert_eq!(PgnAnnotation::from(""), PgnAnnotation::Unknown);
    assert_eq!(PgnAnnotation::from("x"), PgnAnnotation::Unknown);
}

#[test]
fn test_annotation_from_string_consumed() {
    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_from_string("!", &mut consumed);
    assert_eq!(a, PgnAnnotation::GoodMove);
    assert_eq!(consumed, 1);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_from_string("!!", &mut consumed);
    assert_eq!(a, PgnAnnotation::BrilliantMove);
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_from_string("??", &mut consumed);
    assert_eq!(a, PgnAnnotation::Blunder);
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_from_string("?!", &mut consumed);
    assert_eq!(a, PgnAnnotation::DubiousMove);
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_from_string("!?", &mut consumed);
    assert_eq!(a, PgnAnnotation::InterestingMove);
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_from_string("?", &mut consumed);
    assert_eq!(a, PgnAnnotation::Mistake);
    assert_eq!(consumed, 1);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_from_string("", &mut consumed);
    assert_eq!(a, PgnAnnotation::Unknown);
    assert_eq!(consumed, 0);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_from_string("x", &mut consumed);
    assert_eq!(a, PgnAnnotation::Unknown);
    assert_eq!(consumed, 0);
}

#[test]
fn test_annotation_nag_basic() {
    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_nag_from_string("$1", &mut consumed);
    assert_eq!(a, PgnAnnotation::GoodMove);
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_nag_from_string("$0", &mut consumed);
    assert_eq!(a, PgnAnnotation::Null);
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_nag_from_string("$2", &mut consumed);
    assert_eq!(a, PgnAnnotation::Mistake);
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_nag_from_string("$4", &mut consumed);
    assert_eq!(a, PgnAnnotation::Blunder);
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    let a = PgnAnnotation::pgn_annotation_nag_from_string("xyz", &mut consumed);
    assert_eq!(a, PgnAnnotation::Unknown);
    assert_eq!(consumed, 0);
}

#[test]
fn test_annotation_display() {
    assert_eq!(format!("{}", PgnAnnotation::GoodMove), "!");
    assert_eq!(format!("{}", PgnAnnotation::Mistake), "?");
    assert_eq!(format!("{}", PgnAnnotation::BrilliantMove), "!!");
    assert_eq!(format!("{}", PgnAnnotation::Blunder), "??");
    assert_eq!(format!("{}", PgnAnnotation::InterestingMove), "!?");
    assert_eq!(format!("{}", PgnAnnotation::DubiousMove), "?!");
    assert_eq!(format!("{}", PgnAnnotation::Unknown), "");
    assert_eq!(format!("{}", PgnAnnotation::Null), "");
}

#[test]
fn test_annotation_from_i8() {
    assert_eq!(PgnAnnotation::from(-1i8), PgnAnnotation::Unknown);
    assert_eq!(PgnAnnotation::from(0i8), PgnAnnotation::Null);
    assert_eq!(PgnAnnotation::from(1i8), PgnAnnotation::GoodMove);
    assert_eq!(PgnAnnotation::from(2i8), PgnAnnotation::Mistake);
    assert_eq!(PgnAnnotation::from(3i8), PgnAnnotation::BrilliantMove);
    assert_eq!(PgnAnnotation::from(4i8), PgnAnnotation::Blunder);
    assert_eq!(PgnAnnotation::from(5i8), PgnAnnotation::InterestingMove);
    assert_eq!(PgnAnnotation::from(6i8), PgnAnnotation::DubiousMove);
}

fn main() {}
