use libpgn::annotation::PgnAnnotation;

#[test]
fn test_annotation_from_string_good_move() {
    assert_eq!(PgnAnnotation::from("!"), PgnAnnotation::GoodMove);
}

#[test]
fn test_annotation_from_string_mistake() {
    assert_eq!(PgnAnnotation::from("?"), PgnAnnotation::Mistake);
}

#[test]
fn test_annotation_from_string_brilliant() {
    assert_eq!(PgnAnnotation::from("!!"), PgnAnnotation::BrilliantMove);
}

#[test]
fn test_annotation_from_string_blunder() {
    assert_eq!(PgnAnnotation::from("??"), PgnAnnotation::Blunder);
}

#[test]
fn test_annotation_from_string_interesting() {
    assert_eq!(PgnAnnotation::from("!?"), PgnAnnotation::InterestingMove);
}

#[test]
fn test_annotation_from_string_dubious() {
    assert_eq!(PgnAnnotation::from("?!"), PgnAnnotation::DubiousMove);
}

#[test]
fn test_annotation_from_string_empty() {
    assert_eq!(PgnAnnotation::from(""), PgnAnnotation::Unknown);
}

#[test]
fn test_annotation_from_string_unknown_char() {
    assert_eq!(PgnAnnotation::from("x"), PgnAnnotation::Unknown);
}

#[test]
fn test_annotation_display() {
    assert_eq!(PgnAnnotation::GoodMove.to_string(), "!");
    assert_eq!(PgnAnnotation::Mistake.to_string(), "?");
    assert_eq!(PgnAnnotation::BrilliantMove.to_string(), "!!");
    assert_eq!(PgnAnnotation::Blunder.to_string(), "??");
    assert_eq!(PgnAnnotation::InterestingMove.to_string(), "!?");
    assert_eq!(PgnAnnotation::DubiousMove.to_string(), "?!");
}

#[test]
fn test_annotation_unknown_display_empty() {
    // C returns 0 bytes for Unknown
    assert_eq!(PgnAnnotation::Unknown.to_string(), "");
}

#[test]
fn test_annotation_null_display() {
    // C annotation_to_string for Null falls through to sprintf($0)
    assert_eq!(PgnAnnotation::Null.to_string(), "$0");
}

#[test]
fn test_annotation_nag_from_string() {
    let mut consumed = 0;
    let ann = PgnAnnotation::pgn_annotation_nag_from_string("$1", &mut consumed);
    assert_eq!(ann, PgnAnnotation::GoodMove);
    assert_eq!(consumed, 2);
}

#[test]
fn test_annotation_nag_from_string_mistake() {
    let mut consumed = 0;
    let ann = PgnAnnotation::pgn_annotation_nag_from_string("$2", &mut consumed);
    assert_eq!(ann, PgnAnnotation::Mistake);
}

#[test]
fn test_annotation_nag_not_dollar() {
    let mut consumed = 0;
    let ann = PgnAnnotation::pgn_annotation_nag_from_string("abc", &mut consumed);
    assert_eq!(ann, PgnAnnotation::Unknown);
    assert_eq!(consumed, 0);
}

#[test]
fn test_annotation_consumed_single_char() {
    let mut consumed = 0;
    let ann = PgnAnnotation::pgn_annotation_from_string("!", &mut consumed);
    assert_eq!(ann, PgnAnnotation::GoodMove);
    assert_eq!(consumed, 1);
}

#[test]
fn test_annotation_consumed_two_char() {
    let mut consumed = 0;
    let ann = PgnAnnotation::pgn_annotation_from_string("!!", &mut consumed);
    assert_eq!(ann, PgnAnnotation::BrilliantMove);
    assert_eq!(consumed, 2);
}

fn main() {}
