#[allow(unused_imports)]
use libpgn::annotation::PgnAnnotation;
#[allow(unused_imports)]
use libpgn::pgn::Pgn;
#[allow(unused_imports)]
use libpgn::piece::PgnPiece;
#[allow(unused_imports)]
use libpgn::score::PgnScore;

#[test]
fn test_pgn_init_default() {
    let p = Pgn::new();
    assert!(p.metadata.is_none());
    assert!(p.moves.is_none());
    assert_eq!(p.score, PgnScore::Unknown);
}

#[test]
fn test_pgn_parse_metadata() {
    let m = Pgn::parse_metadata("[Event \"Foo\"]\n[Site \"Bar\"]\n");
    assert_eq!(m.get("Event"), Some("Foo"));
    assert_eq!(m.get("Site"), Some("Bar"));
}

#[test]
fn test_pgn_parse_score() {
    assert_eq!(Pgn::parse_score("1-0"), PgnScore::WhiteWon);
    assert_eq!(Pgn::parse_score("0-1"), PgnScore::BlackWon);
    assert_eq!(Pgn::parse_score("1/2-1/2"), PgnScore::Draw);
    assert_eq!(Pgn::parse_score("*"), PgnScore::Ongoing);
    assert_eq!(Pgn::parse_score(""), PgnScore::Unknown);
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
    let mvs = Pgn::parse_moves("1.e4 e5");
    assert_eq!(mvs.values.len(), 1);
    assert_eq!(mvs.values[0].white.notation, "e4");
    assert_eq!(mvs.values[0].black.notation, "e5");
}

#[test]
fn test_pgn_parse_full() {
    // Mirrors C tests/pgn.c
    let mut pgn = Pgn::new();
    pgn.parse(
        "[Event \"Ch City (open)\"]\n\
         [Site \"Frankfurt (Germany)\"]\n\
         \n\
         1.e4 e5\n\
         2.Nc3 Nc6\n\
         3. g3 0-1",
    );

    let metadata = pgn.metadata.as_ref().unwrap();
    assert_eq!(metadata.get("Event"), Some("Ch City (open)"));
    assert_eq!(metadata.get("Site"), Some("Frankfurt (Germany)"));

    let moves = pgn.moves.as_ref().unwrap();
    assert_eq!(moves.values[0].white.notation, "e4");
    assert_eq!(moves.values[0].white.piece, PgnPiece::Pawn);
    assert_eq!(moves.values[0].white.dest.file, Some('e'));
    assert_eq!(moves.values[0].white.dest.rank, Some(4));
    assert_eq!(moves.values[0].white.captures, false);
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::Unknown);
    assert_eq!(moves.values[0].black.notation, "e5");
    assert_eq!(moves.values[0].black.piece, PgnPiece::Pawn);
    assert_eq!(moves.values[0].black.dest.file, Some('e'));
    assert_eq!(moves.values[0].black.dest.rank, Some(5));
    assert_eq!(moves.values[0].black.captures, false);

    assert_eq!(moves.values[1].white.notation, "Nc3");
    assert_eq!(moves.values[1].white.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].white.dest.file, Some('c'));
    assert_eq!(moves.values[1].white.dest.rank, Some(3));
    assert_eq!(moves.values[1].black.notation, "Nc6");
    assert_eq!(moves.values[1].black.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].black.dest.file, Some('c'));
    assert_eq!(moves.values[1].black.dest.rank, Some(6));

    assert_eq!(pgn.score, PgnScore::BlackWon);
}

fn main() {}
