#[allow(unused_imports)]
use libpgn::score::PgnScore;

#[test]
fn test_pgn_score_valid() {
    assert_eq!(PgnScore::from("1/2-1/2"), PgnScore::Draw);
    assert_eq!(PgnScore::from("1-0"), PgnScore::WhiteWon);
    assert_eq!(PgnScore::from("0-1"), PgnScore::BlackWon);
    assert_eq!(PgnScore::from("0-0"), PgnScore::Forfeit);
    assert_eq!(PgnScore::from("0-1/2"), PgnScore::WhiteForfeit);
    assert_eq!(PgnScore::from("1/2-0"), PgnScore::BlackForfeit);
    assert_eq!(PgnScore::from("*"), PgnScore::Ongoing);
}

#[test]
fn test_pgn_score_invalid() {
    assert_eq!(PgnScore::from("1-1/2"), PgnScore::Unknown);
    assert_eq!(PgnScore::from("1/2-1"), PgnScore::Unknown);
    assert_eq!(PgnScore::from("2-0"), PgnScore::Unknown);
    assert_eq!(PgnScore::from("0-2"), PgnScore::Unknown);
    assert_eq!(PgnScore::from(""), PgnScore::Unknown);
    assert_eq!(PgnScore::from("-"), PgnScore::Unknown);
    assert_eq!(PgnScore::from("0-"), PgnScore::Unknown);
    assert_eq!(PgnScore::from("-0"), PgnScore::Unknown);
}

#[test]
fn test_pgn_score_consumed() {
    // For valid scores, consumed reflects # of characters read
    let mut consumed = 0;
    assert_eq!(PgnScore::from_string_with_consumption("1/2-1/2", &mut consumed), PgnScore::Draw);
    assert_eq!(consumed, 7);

    let mut consumed = 0;
    assert_eq!(PgnScore::from_string_with_consumption("1-0", &mut consumed), PgnScore::WhiteWon);
    assert_eq!(consumed, 3);

    let mut consumed = 0;
    assert_eq!(PgnScore::from_string_with_consumption("0-1", &mut consumed), PgnScore::BlackWon);
    assert_eq!(consumed, 3);

    let mut consumed = 0;
    assert_eq!(PgnScore::from_string_with_consumption("0-0", &mut consumed), PgnScore::Forfeit);
    assert_eq!(consumed, 3);

    let mut consumed = 0;
    assert_eq!(PgnScore::from_string_with_consumption("0-1/2", &mut consumed), PgnScore::WhiteForfeit);
    assert_eq!(consumed, 5);

    let mut consumed = 0;
    assert_eq!(PgnScore::from_string_with_consumption("1/2-0", &mut consumed), PgnScore::BlackForfeit);
    assert_eq!(consumed, 5);

    let mut consumed = 0;
    assert_eq!(PgnScore::from_string_with_consumption("*", &mut consumed), PgnScore::Ongoing);
    assert_eq!(consumed, 1);
}

#[test]
fn test_pgn_score_to_string() {
    assert_eq!(format!("{}", PgnScore::Draw), "1/2-1/2");
    assert_eq!(format!("{}", PgnScore::WhiteWon), "1-0");
    assert_eq!(format!("{}", PgnScore::BlackWon), "0-1");
    assert_eq!(format!("{}", PgnScore::Forfeit), "0-0");
    assert_eq!(format!("{}", PgnScore::WhiteForfeit), "0-1/2");
    assert_eq!(format!("{}", PgnScore::BlackForfeit), "1/2-0");
    assert_eq!(format!("{}", PgnScore::Ongoing), "*");
    assert_eq!(format!("{}", PgnScore::Unknown), "");
}

fn main() {}
