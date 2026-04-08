use libpgn::annotation::PgnAnnotation;

#[test]
fn test_annotation_from_string() {
    let mut consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_from_string("!", &mut consumed), PgnAnnotation::GoodMove);
    assert_eq!(consumed, 1);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_from_string("?", &mut consumed), PgnAnnotation::Mistake);
    assert_eq!(consumed, 1);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_from_string("!!", &mut consumed), PgnAnnotation::BrilliantMove);
    assert_eq!(consumed, 2);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_from_string("??", &mut consumed), PgnAnnotation::Blunder);
    assert_eq!(consumed, 2);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_from_string("!?", &mut consumed), PgnAnnotation::InterestingMove);
    assert_eq!(consumed, 2);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_from_string("?!", &mut consumed), PgnAnnotation::DubiousMove);
    assert_eq!(consumed, 2);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_from_string("", &mut consumed), PgnAnnotation::Unknown);
    assert_eq!(consumed, 0);
}

#[test]
fn test_annotation_nag_from_string() {
    let mut consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_nag_from_string("$0", &mut consumed), PgnAnnotation::Null);
    assert_eq!(consumed, 2);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_nag_from_string("$1", &mut consumed), PgnAnnotation::GoodMove);
    assert_eq!(consumed, 2);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_nag_from_string("$4", &mut consumed), PgnAnnotation::Blunder);
    assert_eq!(consumed, 2);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_nag_from_string("$6", &mut consumed), PgnAnnotation::DubiousMove);
    assert_eq!(consumed, 2);

    // NAG > 6 maps to Null in Rust (C stores raw int)
    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_nag_from_string("$69", &mut consumed), PgnAnnotation::Null);
    assert_eq!(consumed, 3);

    consumed = 0;
    assert_eq!(PgnAnnotation::pgn_annotation_nag_from_string("$420", &mut consumed), PgnAnnotation::Null);
    assert_eq!(consumed, 4);
}

#[test]
fn test_annotation_nag_multi() {
    // "$0 $19" -> takes last NAG, which is 19 -> Null in Rust
    let mut consumed = 0;
    let ann = PgnAnnotation::pgn_annotation_nag_from_string("$0 $19", &mut consumed);
    assert_eq!(ann, PgnAnnotation::Null);
    assert_eq!(consumed, 6);
}

#[test]
fn test_annotation_display() {
    assert_eq!(format!("{}", PgnAnnotation::Unknown), "");
    assert_eq!(format!("{}", PgnAnnotation::Null), "$0");
    assert_eq!(format!("{}", PgnAnnotation::GoodMove), "!");
    assert_eq!(format!("{}", PgnAnnotation::Mistake), "?");
    assert_eq!(format!("{}", PgnAnnotation::BrilliantMove), "!!");
    assert_eq!(format!("{}", PgnAnnotation::Blunder), "??");
    assert_eq!(format!("{}", PgnAnnotation::InterestingMove), "!?");
    assert_eq!(format!("{}", PgnAnnotation::DubiousMove), "?!");
}

#[test]
fn test_annotation_from_str_trait() {
    assert_eq!(PgnAnnotation::from("!"), PgnAnnotation::GoodMove);
    assert_eq!(PgnAnnotation::from("??"), PgnAnnotation::Blunder);
    assert_eq!(PgnAnnotation::from(""), PgnAnnotation::Unknown);
}

fn main() {}
