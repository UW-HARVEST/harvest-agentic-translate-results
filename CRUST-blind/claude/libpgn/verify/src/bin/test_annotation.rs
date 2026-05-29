#[allow(unused_imports)]
use libpgn::annotation::PgnAnnotation;

#[test]
fn test_annotation_from_string_simple() {
    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_from_string("!", &mut consumed),
        PgnAnnotation::GoodMove
    );
    assert_eq!(consumed, 1);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_from_string("?", &mut consumed),
        PgnAnnotation::Mistake
    );
    assert_eq!(consumed, 1);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_from_string("!!", &mut consumed),
        PgnAnnotation::BrilliantMove
    );
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_from_string("??", &mut consumed),
        PgnAnnotation::Blunder
    );
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_from_string("!?", &mut consumed),
        PgnAnnotation::InterestingMove
    );
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_from_string("?!", &mut consumed),
        PgnAnnotation::DubiousMove
    );
    assert_eq!(consumed, 2);
}

#[test]
fn test_annotation_from_empty_or_unknown() {
    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_from_string("", &mut consumed),
        PgnAnnotation::Unknown
    );
    assert_eq!(consumed, 0);
}

#[test]
fn test_annotation_with_trailing() {
    // C: input "!a" -> result=1 consumed=1
    let mut consumed = 0;
    let r = PgnAnnotation::pgn_annotation_from_string("!a", &mut consumed);
    assert_eq!(r, PgnAnnotation::GoodMove);
    assert_eq!(consumed, 1);

    // C: input "?b" -> result=2 consumed=1
    let mut consumed = 0;
    let r = PgnAnnotation::pgn_annotation_from_string("?b", &mut consumed);
    assert_eq!(r, PgnAnnotation::Mistake);
    assert_eq!(consumed, 1);

    // C: input "!?b" -> result=5 consumed=2
    let mut consumed = 0;
    let r = PgnAnnotation::pgn_annotation_from_string("!?b", &mut consumed);
    assert_eq!(r, PgnAnnotation::InterestingMove);
    assert_eq!(consumed, 2);

    // C: input "??x" -> result=4 consumed=2
    let mut consumed = 0;
    let r = PgnAnnotation::pgn_annotation_from_string("??x", &mut consumed);
    assert_eq!(r, PgnAnnotation::Blunder);
    assert_eq!(consumed, 2);

    // C: input "!!?" -> result=3 consumed=2
    let mut consumed = 0;
    let r = PgnAnnotation::pgn_annotation_from_string("!!?", &mut consumed);
    assert_eq!(r, PgnAnnotation::BrilliantMove);
    assert_eq!(consumed, 2);
}

#[test]
fn test_annotation_nag_basic() {
    // C: $0 -> 0, consumed=2
    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$0", &mut consumed),
        PgnAnnotation::Null
    );
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$1", &mut consumed),
        PgnAnnotation::GoodMove
    );
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$2", &mut consumed),
        PgnAnnotation::Mistake
    );
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$3", &mut consumed),
        PgnAnnotation::BrilliantMove
    );
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$4", &mut consumed),
        PgnAnnotation::Blunder
    );
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$5", &mut consumed),
        PgnAnnotation::InterestingMove
    );
    assert_eq!(consumed, 2);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$6", &mut consumed),
        PgnAnnotation::DubiousMove
    );
    assert_eq!(consumed, 2);
}

#[test]
fn test_annotation_nag_arbitrary_values() {
    // From C: $19 -> 19 consumed=3
    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$19", &mut consumed),
        PgnAnnotation(19)
    );
    assert_eq!(consumed, 3);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$69", &mut consumed),
        PgnAnnotation(69)
    );
    assert_eq!(consumed, 3);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$420", &mut consumed),
        PgnAnnotation(420)
    );
    assert_eq!(consumed, 4);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$255", &mut consumed),
        PgnAnnotation(255)
    );
    assert_eq!(consumed, 4);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$256", &mut consumed),
        PgnAnnotation(256)
    );
    assert_eq!(consumed, 4);
}

#[test]
fn test_annotation_nag_chained() {
    // From C: "$0 $19" -> result=19 consumed=6
    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$0 $19", &mut consumed),
        PgnAnnotation(19)
    );
    assert_eq!(consumed, 6);

    // C: "$0 $1" -> result=1 consumed=5
    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$0 $1", &mut consumed),
        PgnAnnotation::GoodMove
    );
    assert_eq!(consumed, 5);

    // C: "$1 $0" -> result=0 consumed=5
    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("$1 $0", &mut consumed),
        PgnAnnotation::Null
    );
    assert_eq!(consumed, 5);
}

#[test]
fn test_annotation_nag_no_dollar() {
    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("19", &mut consumed),
        PgnAnnotation::Unknown
    );
    assert_eq!(consumed, 0);

    let mut consumed = 0;
    assert_eq!(
        PgnAnnotation::pgn_annotation_nag_from_string("", &mut consumed),
        PgnAnnotation::Unknown
    );
    assert_eq!(consumed, 0);
}

#[test]
fn test_annotation_to_string() {
    // From C output, calling pgn_annotation_to_string:
    // -1 -> ""  bytes=0
    assert_eq!(format!("{}", PgnAnnotation::Unknown), "");
    // 0 -> "$0" bytes=2
    assert_eq!(format!("{}", PgnAnnotation::Null), "$0");
    // 1 -> "!"
    assert_eq!(format!("{}", PgnAnnotation::GoodMove), "!");
    // 2 -> "?"
    assert_eq!(format!("{}", PgnAnnotation::Mistake), "?");
    // 3 -> "!!"
    assert_eq!(format!("{}", PgnAnnotation::BrilliantMove), "!!");
    // 4 -> "??"
    assert_eq!(format!("{}", PgnAnnotation::Blunder), "??");
    // 5 -> "!?"
    assert_eq!(format!("{}", PgnAnnotation::InterestingMove), "!?");
    // 6 -> "?!"
    assert_eq!(format!("{}", PgnAnnotation::DubiousMove), "?!");
    // 7 -> "$7"
    assert_eq!(format!("{}", PgnAnnotation(7)), "$7");
    // 9 -> "$9"
    assert_eq!(format!("{}", PgnAnnotation(9)), "$9");
    // 19 -> "$19"
    assert_eq!(format!("{}", PgnAnnotation(19)), "$19");
    // 100 -> "$100"
    assert_eq!(format!("{}", PgnAnnotation(100)), "$100");
}

fn main() {}
