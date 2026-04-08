use libpgn::score::PgnScore;

#[test]
fn test_score_from_string_ongoing() {
    assert_eq!(PgnScore::from("*"), PgnScore::Ongoing);
}

#[test]
fn test_score_from_string_draw() {
    assert_eq!(PgnScore::from("1/2-1/2"), PgnScore::Draw);
}

#[test]
fn test_score_from_string_white_won() {
    assert_eq!(PgnScore::from("1-0"), PgnScore::WhiteWon);
}

#[test]
fn test_score_from_string_black_won() {
    assert_eq!(PgnScore::from("0-1"), PgnScore::BlackWon);
}

#[test]
fn test_score_from_string_forfeit() {
    assert_eq!(PgnScore::from("0-0"), PgnScore::Forfeit);
}

#[test]
fn test_score_from_string_white_forfeit() {
    assert_eq!(PgnScore::from("0-1/2"), PgnScore::WhiteForfeit);
}

#[test]
fn test_score_from_string_black_forfeit() {
    assert_eq!(PgnScore::from("1/2-0"), PgnScore::BlackForfeit);
}

#[test]
fn test_score_from_string_unknown() {
    assert_eq!(PgnScore::from(""), PgnScore::Unknown);
    assert_eq!(PgnScore::from("abc"), PgnScore::Unknown);
}

#[test]
fn test_score_to_string() {
    assert_eq!(PgnScore::Ongoing.to_string(), "*");
    assert_eq!(PgnScore::Draw.to_string(), "1/2-1/2");
    assert_eq!(PgnScore::WhiteWon.to_string(), "1-0");
    assert_eq!(PgnScore::BlackWon.to_string(), "0-1");
    assert_eq!(PgnScore::Forfeit.to_string(), "0-0");
    assert_eq!(PgnScore::WhiteForfeit.to_string(), "0-1/2");
    assert_eq!(PgnScore::BlackForfeit.to_string(), "1/2-0");
    assert_eq!(PgnScore::Unknown.to_string(), "");
}

#[test]
fn test_score_with_consumption() {
    let mut consumed = 0;
    let score = PgnScore::from_string_with_consumption_pub("1-0", &mut consumed);
    assert_eq!(score, PgnScore::WhiteWon);
    assert_eq!(consumed, 3);
}

#[test]
fn test_score_draw_consumption() {
    let mut consumed = 0;
    let score = PgnScore::from_string_with_consumption_pub("1/2-1/2", &mut consumed);
    assert_eq!(score, PgnScore::Draw);
    assert_eq!(consumed, 7);
}

#[test]
fn test_score_ongoing_consumption() {
    let mut consumed = 0;
    let score = PgnScore::from_string_with_consumption_pub("*", &mut consumed);
    assert_eq!(score, PgnScore::Ongoing);
    assert_eq!(consumed, 1);
}

#[test]
fn test_score_no_dash_unknown() {
    // "1x0" has no dash separator -> Unknown
    assert_eq!(PgnScore::from("1x0"), PgnScore::Unknown);
}

fn main() {}
