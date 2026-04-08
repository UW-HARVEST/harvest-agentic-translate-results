use libpgn::pgn::Pgn;
use libpgn::score::PgnScore;
use libpgn::piece::PgnPiece;

#[test]
fn test_pgn_new() {
    let p = Pgn::new();
    assert!(p.metadata.is_none());
    assert!(p.moves.is_none());
    assert_eq!(p.score, PgnScore::Unknown);
}

#[test]
fn test_pgn_parse_metadata() {
    let m = Pgn::parse_metadata("[Event \"Test\"]\n");
    assert_eq!(m.get("Event"), Some("Test"));
}

#[test]
fn test_pgn_parse_move() {
    let m = Pgn::parse_move("e4");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(4));
}

#[test]
fn test_pgn_parse_moves() {
    let moves = Pgn::parse_moves("1. e4 e5 1-0");
    assert_eq!(moves.values.len(), 1);
}

#[test]
fn test_pgn_parse_score() {
    assert_eq!(Pgn::parse_score("1-0"), PgnScore::WhiteWon);
    assert_eq!(Pgn::parse_score("0-1"), PgnScore::BlackWon);
    assert_eq!(Pgn::parse_score("1/2-1/2"), PgnScore::Draw);
    assert_eq!(Pgn::parse_score("*"), PgnScore::Ongoing);
}

#[test]
fn test_pgn_parse_full() {
    let input = "[Event \"Test\"]\n[Site \"Here\"]\n\n1. e4 e5 1-0";
    let mut pgn = Pgn::new();
    let consumed = pgn.parse(input);
    assert!(consumed > 0);
    assert!(pgn.metadata.is_some());
    let meta = pgn.metadata.as_ref().unwrap();
    assert_eq!(meta.get("Event"), Some("Test"));
    assert_eq!(meta.get("Site"), Some("Here"));
    assert!(pgn.moves.is_some());
    let moves = pgn.moves.as_ref().unwrap();
    assert_eq!(moves.values.len(), 1);
    assert_eq!(pgn.score, PgnScore::WhiteWon);
}

#[test]
fn test_pgn_parse_no_metadata() {
    let input = "1. e4 e5 1-0";
    let mut pgn = Pgn::new();
    pgn.parse(input);
    assert!(pgn.metadata.is_none());
    assert!(pgn.moves.is_some());
    assert_eq!(pgn.score, PgnScore::WhiteWon);
}

#[test]
fn test_pgn_parse_draw() {
    let input = "1. d4 d5 1/2-1/2";
    let mut pgn = Pgn::new();
    pgn.parse(input);
    assert_eq!(pgn.score, PgnScore::Draw);
}

#[test]
fn test_pgn_parse_ongoing() {
    let input = "1. e4 e5 *";
    let mut pgn = Pgn::new();
    pgn.parse(input);
    assert_eq!(pgn.score, PgnScore::Ongoing);
}

fn main() {}
