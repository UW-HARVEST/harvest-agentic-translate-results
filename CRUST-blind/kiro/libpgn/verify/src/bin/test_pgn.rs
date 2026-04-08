use libpgn::pgn::Pgn;
use libpgn::piece::PgnPiece;
use libpgn::score::PgnScore;
use libpgn::annotation::PgnAnnotation;

#[test]
fn test_pgn_parse_full() {
    let mut pgn = Pgn::new();
    pgn.parse(
        "[Event \"Ch City (open)\"]\n\
         [Site \"Frankfurt (Germany)\"]\n\
         \n\
         1.e4 e5\n\
         2.Nc3 Nc6\n\
         3. g3 0-1"
    );

    let md = pgn.metadata.as_ref().unwrap();
    assert_eq!(md.get("Event"), Some("Ch City (open)"));
    assert_eq!(md.get("Site"), Some("Frankfurt (Germany)"));

    let moves = pgn.moves.as_ref().unwrap();
    assert_eq!(moves.values.len(), 3);

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
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::Unknown);

    assert_eq!(moves.values[1].white.notation, "Nc3");
    assert_eq!(moves.values[1].white.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].white.dest.file, Some('c'));
    assert_eq!(moves.values[1].white.dest.rank, Some(3));
    assert_eq!(moves.values[1].black.notation, "Nc6");
    assert_eq!(moves.values[1].black.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].black.dest.file, Some('c'));
    assert_eq!(moves.values[1].black.dest.rank, Some(6));

    assert_eq!(moves.values[2].white.notation, "g3");
    assert_eq!(moves.values[2].white.piece, PgnPiece::Pawn);
    assert_eq!(moves.values[2].white.dest.file, Some('g'));
    assert_eq!(moves.values[2].white.dest.rank, Some(3));

    assert_eq!(pgn.score, PgnScore::BlackWon);
}

#[test]
fn test_pgn_parse_helpers() {
    assert_eq!(Pgn::parse_score("1-0"), PgnScore::WhiteWon);
    assert_eq!(Pgn::parse_score("0-1"), PgnScore::BlackWon);
    assert_eq!(Pgn::parse_score("1/2-1/2"), PgnScore::Draw);

    let mv = Pgn::parse_move("e4");
    assert_eq!(mv.piece, PgnPiece::Pawn);
    assert_eq!(mv.dest.file, Some('e'));
    assert_eq!(mv.dest.rank, Some(4));

    let moves = Pgn::parse_moves("1.e4 e5");
    assert_eq!(moves.values.len(), 1);
    assert_eq!(moves.values[0].white.notation, "e4");
    assert_eq!(moves.values[0].black.notation, "e5");
}

#[test]
fn test_pgn_parse_metadata_helper() {
    let md = Pgn::parse_metadata(
        "[Event \"test\"]\n[Site \"here\"]\n"
    );
    assert_eq!(md.get("Event"), Some("test"));
    assert_eq!(md.get("Site"), Some("here"));
}

fn main() {}
